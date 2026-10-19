use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Instant;
use tauri::{AppHandle, Emitter};

use super::detector::find_djxl_executable;
use super::formats::ImageItemInfo;
use super::jxl::{ConversionResult, sanitize_filename};
use super::metadata::{extract_metadata, inject_comfy_metadata_into_png};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyExportProfile {
    pub target_format: String, // "png" | "jpg"
    pub jpeg_quality: u32,     // 60..100
    pub reconstruct_jpeg: bool,
    pub dest_folder: Option<String>,
}

impl Default for LegacyExportProfile {
    fn default() -> Self {
        Self {
            target_format: "png".to_string(),
            jpeg_quality: 95,
            reconstruct_jpeg: true,
            dest_folder: None,
        }
    }
}

pub fn convert_single_jxl_to_legacy(
    item: &ImageItemInfo,
    profile: &LegacyExportProfile,
) -> ConversionResult {
    let start_time = Instant::now();
    let djxl_path = match find_djxl_executable() {
        Some(p) => p,
        None => {
            return ConversionResult {
                id: item.id.clone(),
                status: "error".to_string(),
                target_path: None,
                target_size: None,
                savings_bytes: None,
                savings_percent: None,
                time_ms: None,
                error_message: Some(
                    "djxl decoder executable not found. Please install libjxl or check PATH."
                        .to_string(),
                ),
            };
        }
    };

    let source_path = Path::new(&item.path);
    if !source_path.is_file() {
        return ConversionResult {
            id: item.id.clone(),
            status: "error".to_string(),
            target_path: None,
            target_size: None,
            savings_bytes: None,
            savings_percent: None,
            time_ms: None,
            error_message: Some(format!("Source JXL image not found: {}", item.path)),
        };
    }

    // Defensive Gatekeeper: Sniff magic bytes to verify genuine JXL bitstream
    if let Ok(magic) = super::limits::sniff_file_magic(source_path)
        && magic != super::limits::MagicFormat::JxlContainer
        && magic != super::limits::MagicFormat::JxlCodestream
    {
        return ConversionResult {
            id: item.id.clone(),
            status: "error".to_string(),
            target_path: None,
            target_size: None,
            savings_bytes: None,
            savings_percent: None,
            time_ms: None,
            error_message: Some(format!(
                "File does not contain valid JPEG XL container or codestream magic bytes (detected {:?})",
                magic
            )),
        };
    }

    // Defensive Gatekeeper: Validate dimensions ceiling
    if let Err(e) = super::limits::validate_image_dimensions(item.width, item.height, 4) {
        return ConversionResult {
            id: item.id.clone(),
            status: "error".to_string(),
            target_path: None,
            target_size: None,
            savings_bytes: None,
            savings_percent: None,
            time_ms: None,
            error_message: Some(e.to_string()),
        };
    }

    let dest_dir = super::jxl::resolve_destination_folder(&profile.dest_folder, source_path);
    let raw_stem = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let safe_stem = sanitize_filename(raw_stem);

    let ext = profile.target_format.to_lowercase();
    let target_ext = if ext.contains("jpg") || ext.contains("jpeg") {
        "jpg"
    } else {
        "png"
    };

    let target_path = dest_dir.join(format!("{}.{}", safe_stem, target_ext));

    let abs_input = std::path::absolute(source_path).unwrap_or_else(|_| source_path.to_path_buf());
    let abs_target = std::path::absolute(&target_path).unwrap_or_else(|_| target_path.clone());

    let mut cmd = Command::new(&djxl_path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW (0x08000000) | BELOW_NORMAL_PRIORITY_CLASS (0x00004000)
        cmd.creation_flags(0x08000000 | 0x00004000);
    }
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.arg(&abs_input);
    cmd.arg(&abs_target);

    let sys_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let worker_threads = (sys_threads / 2).clamp(1, 4);
    cmd.arg(format!("--num_threads={}", worker_threads));

    if target_ext == "jpg" && !profile.reconstruct_jpeg {
        let q = profile.jpeg_quality.clamp(60, 100);
        cmd.arg("-q").arg(q.to_string());
    }

    cmd.arg("--quiet");

    let timeout = std::time::Duration::from_secs(120);
    match super::jxl::run_command_with_timeout(cmd, timeout, super::batch::is_batch_cancelled) {
        Ok(output) => {
            let elapsed_ms = start_time.elapsed().as_millis() as u64;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                let msg = if !stderr.trim().is_empty() {
                    stderr.trim().to_string()
                } else if !stdout.trim().is_empty() {
                    stdout.trim().to_string()
                } else {
                    format!("djxl exited with error code: {:?}", output.status.code())
                };

                return ConversionResult {
                    id: item.id.clone(),
                    status: "error".to_string(),
                    target_path: None,
                    target_size: None,
                    savings_bytes: None,
                    savings_percent: None,
                    time_ms: Some(elapsed_ms),
                    error_message: Some(msg),
                };
            }

            // If target is PNG, extract any ComfyUI workflow metadata from the source JXL container and restore it into the decoded PNG
            if target_ext == "png" {
                let extracted = extract_metadata(source_path);
                if extracted.comfy.prompt.is_some()
                    || extracted.comfy.workflow.is_some()
                    || extracted.comfy.parameters.is_some()
                {
                    let _ = inject_comfy_metadata_into_png(&target_path, &extracted.comfy);
                }
            }

            if let Ok(metadata) = fs::metadata(&target_path) {
                let target_size = metadata.len();
                let orig_size = item.orig_size;
                let size_diff = (target_size as i64) - (orig_size as i64);

                ConversionResult {
                    id: item.id.clone(),
                    status: "done".to_string(),
                    target_path: Some(target_path.to_string_lossy().to_string()),
                    target_size: Some(target_size),
                    savings_bytes: Some(size_diff),
                    savings_percent: None,
                    time_ms: Some(elapsed_ms),
                    error_message: None,
                }
            } else {
                ConversionResult {
                    id: item.id.clone(),
                    status: "error".to_string(),
                    target_path: None,
                    target_size: None,
                    savings_bytes: None,
                    savings_percent: None,
                    time_ms: Some(elapsed_ms),
                    error_message: Some("Target image file was not created by the decoder".to_string()),
                }
            }
        }
        Err(e) => {
            let elapsed_ms = start_time.elapsed().as_millis() as u64;
            ConversionResult {
                id: item.id.clone(),
                status: "error".to_string(),
                target_path: None,
                target_size: None,
                savings_bytes: None,
                savings_percent: None,
                time_ms: Some(elapsed_ms),
                error_message: Some(format!("Failed to spawn djxl process: {}", e)),
            }
        }
    }
}

pub fn execute_legacy_batch(
    app: &AppHandle,
    items: Vec<ImageItemInfo>,
    profile: LegacyExportProfile,
) -> Vec<ConversionResult> {
    super::batch::reset_cancel_flag();
    let mut results = Vec::with_capacity(items.len());

    for item in items {
        if super::batch::is_batch_cancelled() {
            break;
        }

        let _ = app.emit(
            "legacy-conversion-progress",
            serde_json::json!({
                "id": item.id,
                "status": "converting",
                "progress": 30,
                "target_size": null,
                "time_ms": null,
                "error_message": null
            }),
        );

        let res = convert_single_jxl_to_legacy(&item, &profile);

        let _ = app.emit(
            "legacy-conversion-progress",
            serde_json::json!({
                "id": item.id,
                "status": res.status,
                "progress": 100,
                "target_size": res.target_size,
                "time_ms": res.time_ms,
                "error_message": res.error_message
            }),
        );

        results.push(res);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    results
}

pub fn execute_legacy_folder_batch(
    app: &AppHandle,
    folder_path: &str,
    profile: LegacyExportProfile,
) -> Result<super::batch::BatchSummary, String> {
    super::batch::reset_cancel_flag();
    use std::path::PathBuf;

    let dir = PathBuf::from(folder_path);
    if !dir.is_dir() {
        return Err(format!("Not a valid directory: {}", folder_path));
    }

    let scanned = super::formats::scan_folder_for_jxl_with_size(&dir);
    let jxl_paths = scanned.paths;
    let total_count = jxl_paths.len();
    if total_count == 0 {
        return Ok(super::batch::BatchSummary {
            total_items: 0,
            completed_items: 0,
            failed_items: 0,
            orig_total_bytes: 0,
            target_total_bytes: 0,
            total_savings_bytes: 0,
            total_savings_percent: 0,
            total_duration_ms: 0,
            throughput_mp_per_sec: 0.0,
        });
    }

    let start_all = Instant::now();
    let mut completed_count = 0usize;
    let mut failed_count = 0usize;
    let mut orig_bytes = 0u64;
    let mut target_bytes = 0u64;

    for (idx, path) in jxl_paths.iter().enumerate() {
        if super::batch::is_batch_cancelled() {
            let _ = app.emit(
                "legacy-folder-conversion-progress",
                super::batch::FolderBatchProgress {
                    total: total_count,
                    completed: completed_count,
                    failed: failed_count,
                    current_file: "Conversion stopped by user".to_string(),
                    orig_bytes_completed: orig_bytes,
                    target_bytes_completed: target_bytes,
                    is_done: true,
                },
            );
            break;
        }

        let path_str = path.to_string_lossy();
        let file_name = path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("image.jxl")
            .to_string();

        let _ = app.emit(
            "legacy-folder-conversion-progress",
            super::batch::FolderBatchProgress {
                total: total_count,
                completed: completed_count,
                failed: failed_count,
                current_file: format!("Unpacking ({}/{}): {}", idx + 1, total_count, file_name),
                orig_bytes_completed: orig_bytes,
                target_bytes_completed: target_bytes,
                is_done: false,
            },
        );

        if let Ok(item) = super::formats::inspect_image_without_thumbnail(&path_str) {
            let res = convert_single_jxl_to_legacy(&item, &profile);
            if res.status == "done" {
                completed_count += 1;
                if let Some(target) = res.target_size {
                    target_bytes += target;
                }
                orig_bytes += item.orig_size;
            } else {
                failed_count += 1;
            }
        } else {
            failed_count += 1;
        }

        let _ = app.emit(
            "legacy-folder-conversion-progress",
            super::batch::FolderBatchProgress {
                total: total_count,
                completed: completed_count,
                failed: failed_count,
                current_file: file_name,
                orig_bytes_completed: orig_bytes,
                target_bytes_completed: target_bytes,
                is_done: false,
            },
        );

        // Cooperative cooling & OS scheduling yield
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    let total_elapsed = start_all.elapsed().as_millis() as u64;
    let savings_bytes = if orig_bytes > target_bytes {
        (orig_bytes - target_bytes) as i64
    } else {
        0
    };
    let savings_percent = if orig_bytes > 0 {
        ((savings_bytes as f64 / orig_bytes as f64) * 100.0) as i32
    } else {
        0
    };

    let _ = app.emit(
        "legacy-folder-conversion-progress",
        super::batch::FolderBatchProgress {
            total: total_count,
            completed: completed_count,
            failed: failed_count,
            current_file: if super::batch::is_batch_cancelled() {
                "Stopped".to_string()
            } else {
                "Completed".to_string()
            },
            orig_bytes_completed: orig_bytes,
            target_bytes_completed: target_bytes,
            is_done: true,
        },
    );

    Ok(super::batch::BatchSummary {
        total_items: total_count,
        completed_items: completed_count,
        failed_items: failed_count,
        orig_total_bytes: orig_bytes,
        target_total_bytes: target_bytes,
        total_savings_bytes: savings_bytes,
        total_savings_percent: savings_percent,
        total_duration_ms: total_elapsed,
        throughput_mp_per_sec: 0.0,
    })
}

