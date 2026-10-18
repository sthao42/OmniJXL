use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use super::detector::find_cjxl_executable;
use super::formats::{FormatType, ImageItemInfo, detect_format_type, prepare_extended_format};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodingProfile {
    pub is_lossless: bool,
    pub distance: f32,
    pub effort: u32,
    pub is_progressive: bool,
    pub preserve_metadata: bool,
    pub reversible_jpeg: bool,
    pub dest_folder: Option<String>,
}

impl Default for EncodingProfile {
    fn default() -> Self {
        Self {
            is_lossless: true,
            distance: 1.0,
            effort: 7,
            is_progressive: true,
            preserve_metadata: true,
            reversible_jpeg: true,
            dest_folder: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionResult {
    pub id: String,
    pub status: String, // "done" | "error"
    pub target_path: Option<String>,
    pub target_size: Option<u64>,
    pub savings_bytes: Option<i64>,
    pub savings_percent: Option<i32>,
    pub time_ms: Option<u64>,
    pub error_message: Option<String>,
}

pub fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0'..='\x1f' => '_',
            _ => c,
        })
        .collect();
    let no_traversal = cleaned.replace("..", "__");
    let trimmed = no_traversal.trim_matches(|c| c == '.' || c == ' ' || c == '_');
    if trimmed.is_empty() {
        "output".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn resolve_destination_folder(custom_dest: &Option<String>, source_path: &Path) -> PathBuf {
    if let Some(dest) = custom_dest {
        let trimmed = dest.trim();
        if !trimmed.is_empty() {
            // Expand ~ on Windows/Unix if present
            let expanded = if trimmed.starts_with("~/") || trimmed.starts_with("~\\") {
                if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
                    PathBuf::from(home).join(&trimmed[2..])
                } else {
                    PathBuf::from(trimmed)
                }
            } else {
                PathBuf::from(trimmed)
            };
            let _ = fs::create_dir_all(&expanded);
            return expanded;
        }
    }

    // Default: use ~/Pictures/JXL_Exports or source parent
    if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        let default_dir = PathBuf::from(home).join("Pictures").join("JXL_Exports");
        let _ = fs::create_dir_all(&default_dir);
        default_dir
    } else {
        source_path.parent().unwrap_or(Path::new(".")).to_path_buf()
    }
}

pub fn convert_single_image(item: &ImageItemInfo, profile: &EncodingProfile) -> ConversionResult {
    let start_time = Instant::now();
    let cjxl_path = match find_cjxl_executable() {
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
                    "cjxl encoder executable not found. Please install libjxl or restart the application."
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
            error_message: Some(format!("Source image not found: {}", item.path)),
        };
    }

    // Defensive Gatekeeper: Validate dimension ceilings and memory allocation safety
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

    let dest_dir = resolve_destination_folder(&profile.dest_folder, source_path);
    let raw_stem = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let safe_stem = sanitize_filename(raw_stem);

    let target_path = dest_dir.join(format!("{}.jxl", safe_stem));

    // Handle temporary directory for extended formats (WebP, TIFF, BMP, etc.)
    let temp_dir_handle = tempfile::Builder::new().prefix("omnijxl_").tempdir();

    let (fmt_type, _) = detect_format_type(source_path);

    let input_path = if fmt_type == FormatType::NativeCjxl {
        source_path.to_path_buf()
    } else if let Ok(ref temp_dir) = temp_dir_handle {
        match prepare_extended_format(source_path, temp_dir.path()) {
            Ok(p) => p,
            Err(e) => {
                return ConversionResult {
                    id: item.id.clone(),
                    status: "error".to_string(),
                    target_path: None,
                    target_size: None,
                    savings_bytes: None,
                    savings_percent: None,
                    time_ms: None,
                    error_message: Some(format!("Failed to prepare extended format: {}", e)),
                };
            }
        }
    } else {
        source_path.to_path_buf()
    };

    // Extract all source metadata (EXIF, ICC, XMP, ComfyUI workflow/prompt)
    let extracted = super::metadata::extract_metadata(source_path);

    let mut sidecar_exif: Option<PathBuf> = None;
    let mut sidecar_icc: Option<PathBuf> = None;
    let mut sidecar_xmp: Option<PathBuf> = None;

    if profile.preserve_metadata
        && let Ok(ref temp_dir) = temp_dir_handle
    {
        // 1. ICC Color Profile (Color space preservation)
        if let Some(ref icc_bytes) = extracted.raw_icc {
            let p = temp_dir.path().join(format!("{}_icc.icc", safe_stem));
            if fs::write(&p, icc_bytes).is_ok() {
                sidecar_icc = Some(p);
            }
        }

        // 2. EXIF (GPS, Dates, Camera, Lens, and ComfyUI fallback tags)
        if let Some(ref exif_bytes) = extracted.raw_exif {
            let p = temp_dir.path().join(format!("{}_exif.bin", safe_stem));
            if fs::write(&p, exif_bytes).is_ok() {
                sidecar_exif = Some(p);
            }
        } else if extracted.comfy.prompt.is_some() || extracted.comfy.workflow.is_some() {
            let gen_exif = super::metadata::build_comfy_exif(
                extracted.comfy.workflow.as_deref(),
                extracted.comfy.prompt.as_deref(),
                extracted.summary.date_time.as_deref(),
            );
            let p = temp_dir.path().join(format!("{}_exif.bin", safe_stem));
            if fs::write(&p, gen_exif).is_ok() {
                sidecar_exif = Some(p);
            }
        }

        // 3. XMP Metadata
        if let Some(ref xmp_bytes) = extracted.raw_xmp {
            let p = temp_dir.path().join(format!("{}_xmp.xml", safe_stem));
            if fs::write(&p, xmp_bytes).is_ok() {
                sidecar_xmp = Some(p);
            }
        } else if extracted.comfy.prompt.is_some() || extracted.comfy.workflow.is_some() {
            let gen_xmp = super::metadata::build_comfy_xmp(
                extracted.comfy.workflow.as_deref(),
                extracted.comfy.prompt.as_deref(),
                extracted.summary.date_time.as_deref(),
            );
            let p = temp_dir.path().join(format!("{}_xmp.xml", safe_stem));
            if fs::write(&p, gen_xmp).is_ok() {
                sidecar_xmp = Some(p);
            }
        }
    }

    // Construct cjxl command arguments with sanitized absolute paths to prevent flag injection
    let abs_input = std::path::absolute(&input_path).unwrap_or_else(|_| input_path.clone());
    let abs_target = std::path::absolute(&target_path).unwrap_or_else(|_| target_path.clone());

    let mut cmd = Command::new(&cjxl_path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd.arg(&abs_input);
    cmd.arg(&abs_target);

    // Compression Mode & Distance
    if profile.is_lossless {
        cmd.arg("-d").arg("0");
        cmd.arg("-m").arg("1"); // Modular mode for exact lossless
        cmd.arg("--keep_invisible=1"); // Preserve color values in transparent pixels & mask layers
    } else {
        let dist = profile.distance.clamp(0.1, 15.0);
        cmd.arg("-d").arg(format!("{:.1}", dist));
        cmd.arg("-a").arg("0.0"); // Keep alpha channel mathematically lossless
        cmd.arg("--keep_invisible=1"); // Do not clear transparent RGB channels
    }

    // Encoding Effort
    let effort = profile.effort.clamp(1, 9);
    cmd.arg("-e").arg(effort.to_string());

    // Progressive pass
    if profile.is_progressive {
        cmd.arg("-p");
    }

    // Container & Metadata preservation
    if profile.preserve_metadata {
        cmd.arg("--container=1");
        if let Some(ref exif_p) = sidecar_exif {
            cmd.arg("-x").arg(format!("exif={}", exif_p.display()));
        }
        if let Some(ref icc_p) = sidecar_icc {
            cmd.arg("-x")
                .arg(format!("icc_pathname={}", icc_p.display()));
        }
        if let Some(ref xmp_p) = sidecar_xmp {
            cmd.arg("-x").arg(format!("xmp={}", xmp_p.display()));
        }
    }

    // Reversible JPEG transcode handling
    let is_jpeg =
        item.format.to_uppercase().contains("JPEG") || item.format.to_uppercase().contains("JPG");
    if is_jpeg {
        if profile.reversible_jpeg {
            cmd.arg("-j").arg("1");
        } else {
            cmd.arg("-j").arg("0");
        }
    }

    cmd.arg("--quiet");

    // Execute command
    match cmd.output() {
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
                    format!("cjxl exited with error code: {:?}", output.status.code())
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

            // Post-process: If ComfyUI workflow/prompt is present, inject brob box for ComfyUI drag-and-drop
            if profile.preserve_metadata
                && (extracted.comfy.prompt.is_some()
                    || extracted.comfy.workflow.is_some()
                    || extracted.comfy.parameters.is_some())
                && let Ok(jxl_bytes) = fs::read(&target_path)
                && let Some(augmented_jxl) =
                    super::metadata::inject_comfy_brob_to_jxl(&jxl_bytes, &extracted.comfy)
            {
                let _ = fs::write(&target_path, augmented_jxl);
            }

            // Read generated target size
            if let Ok(metadata) = fs::metadata(&target_path) {
                let target_size = metadata.len();
                let orig_size = item.orig_size;
                let savings_bytes = (orig_size as i64) - (target_size as i64);
                let savings_percent = if orig_size > 0 {
                    ((savings_bytes as f64 / orig_size as f64) * 100.0).round() as i32
                } else {
                    0
                };

                ConversionResult {
                    id: item.id.clone(),
                    status: "done".to_string(),
                    target_path: Some(target_path.to_string_lossy().to_string()),
                    target_size: Some(target_size),
                    savings_bytes: Some(savings_bytes),
                    savings_percent: Some(savings_percent),
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
                    error_message: Some(
                        "Target JXL file was not created by the encoder".to_string(),
                    ),
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
                error_message: Some(format!("Failed to spawn cjxl process: {}", e)),
            }
        }
    }
}
