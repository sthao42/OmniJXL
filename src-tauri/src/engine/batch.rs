use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};

use super::formats::ImageItemInfo;
use super::jxl::{ConversionResult, EncodingProfile, convert_single_image};

pub static BATCH_CANCELLED: AtomicBool = AtomicBool::new(false);

pub fn cancel_batch() {
    BATCH_CANCELLED.store(true, Ordering::SeqCst);
}

pub fn reset_cancel_flag() {
    BATCH_CANCELLED.store(false, Ordering::SeqCst);
}

pub fn is_batch_cancelled() -> bool {
    BATCH_CANCELLED.load(Ordering::SeqCst)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    pub id: String,
    pub status: String, // "converting" | "done" | "error"
    pub progress: u32,
    pub target_size: Option<u64>,
    pub savings_bytes: Option<i64>,
    pub savings_percent: Option<i32>,
    pub time_ms: Option<u64>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSummary {
    pub total_items: usize,
    pub completed_items: usize,
    pub failed_items: usize,
    pub orig_total_bytes: u64,
    pub target_total_bytes: u64,
    pub total_savings_bytes: i64,
    pub total_savings_percent: i32,
    pub total_duration_ms: u64,
    pub throughput_mp_per_sec: f64,
}

pub fn execute_batch(
    app: &AppHandle,
    items: Vec<ImageItemInfo>,
    profile: EncodingProfile,
) -> Vec<ConversionResult> {
    reset_cancel_flag();
    let mut results = Vec::with_capacity(items.len());

    // Process each queued image in sequence without launching all threads concurrently
    for item in &items {
        if is_batch_cancelled() {
            break;
        }

        let _ = app.emit(
            "conversion-progress",
            ProgressUpdate {
                id: item.id.clone(),
                status: "converting".to_string(),
                progress: 40,
                target_size: None,
                savings_bytes: None,
                savings_percent: None,
                time_ms: None,
                error_message: None,
            },
        );

        let res = convert_single_image(item, &profile);

        let _ = app.emit(
            "conversion-progress",
            ProgressUpdate {
                id: res.id.clone(),
                status: res.status.clone(),
                progress: 100,
                target_size: res.target_size,
                savings_bytes: res.savings_bytes,
                savings_percent: res.savings_percent,
                time_ms: res.time_ms,
                error_message: res.error_message.clone(),
            },
        );

        results.push(res);
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    results
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderBatchProgress {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub current_file: String,
    pub orig_bytes_completed: u64,
    pub target_bytes_completed: u64,
    pub is_done: bool,
}

pub fn execute_folder_batch(
    app: &AppHandle,
    folder_path: &str,
    profile: EncodingProfile,
) -> Result<BatchSummary, String> {
    reset_cancel_flag();
    let dir = std::path::PathBuf::from(folder_path);
    if !dir.is_dir() {
        return Err(format!("Not a valid directory: {}", folder_path));
    }

    let scanned = super::formats::scan_folder_for_images_with_size(&dir);
    let image_paths = scanned.paths;
    let total_count = image_paths.len();
    if total_count == 0 {
        return Ok(BatchSummary {
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

    let start_all = std::time::Instant::now();
    let mut completed_count = 0usize;
    let mut failed_count = 0usize;
    let mut orig_bytes = 0u64;
    let mut target_bytes = 0u64;

    // Process each individual image in sequence without loading or running the whole batch concurrently
    for (idx, path) in image_paths.iter().enumerate() {
        if is_batch_cancelled() {
            let _ = app.emit(
                "folder-conversion-progress",
                FolderBatchProgress {
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
            .unwrap_or("image")
            .to_string();

        // 1. Emit progress showing which specific image is currently being encoded
        let _ = app.emit(
            "folder-conversion-progress",
            FolderBatchProgress {
                total: total_count,
                completed: completed_count,
                failed: failed_count,
                current_file: format!("Processing ({}/{}): {}", idx + 1, total_count, file_name),
                orig_bytes_completed: orig_bytes,
                target_bytes_completed: target_bytes,
                is_done: false,
            },
        );

        // 2. Load & inspect only this single image into memory
        if let Ok(item) = super::formats::inspect_image_without_thumbnail(&path_str) {
            // 3. Convert this single image
            let res = convert_single_image(&item, &profile);
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

        // 4. Emit updated progress after this single image completes
        let _ = app.emit(
            "folder-conversion-progress",
            FolderBatchProgress {
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
        "folder-conversion-progress",
        FolderBatchProgress {
            total: total_count,
            completed: completed_count,
            failed: failed_count,
            current_file: if is_batch_cancelled() {
                "Stopped".to_string()
            } else {
                "Completed".to_string()
            },
            orig_bytes_completed: orig_bytes,
            target_bytes_completed: target_bytes,
            is_done: true,
        },
    );

    Ok(BatchSummary {
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

