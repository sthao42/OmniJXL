use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tauri::{AppHandle, Emitter};

use super::formats::ImageItemInfo;
use super::jxl::{ConversionResult, EncodingProfile, convert_single_image};

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
    let start_all = std::time::Instant::now();
    let completed_count = Arc::new(AtomicUsize::new(0));

    // Emit initial 'converting' signal for items
    for item in &items {
        let _ = app.emit(
            "conversion-progress",
            ProgressUpdate {
                id: item.id.clone(),
                status: "converting".to_string(),
                progress: 10,
                target_size: None,
                savings_bytes: None,
                savings_percent: None,
                time_ms: None,
                error_message: None,
            },
        );
    }

    // Run parallel conversions via Rayon
    let results: Vec<ConversionResult> = items
        .par_iter()
        .map(|item| {
            // Emit progress 40% before heavy SIMD pass
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

            completed_count.fetch_add(1, Ordering::SeqCst);

            // Emit completion progress event
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

            res
        })
        .collect();

    let _total_elapsed = start_all.elapsed().as_millis() as u64;

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
    let dir = std::path::PathBuf::from(folder_path);
    if !dir.is_dir() {
        return Err(format!("Not a valid directory: {}", folder_path));
    }

    let image_paths = super::formats::scan_folder_for_images(&dir);
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
    let completed_count = Arc::new(AtomicUsize::new(0));
    let failed_count = Arc::new(AtomicUsize::new(0));
    let orig_bytes = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let target_bytes = Arc::new(std::sync::atomic::AtomicU64::new(0));

    image_paths.par_iter().for_each(|path| {
        let path_str = path.to_string_lossy();
        let file_name = path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("image")
            .to_string();

        if let Ok(item) = super::formats::inspect_image_without_thumbnail(&path_str) {
            let res = convert_single_image(&item, &profile);
            if res.status == "done" {
                completed_count.fetch_add(1, Ordering::SeqCst);
                if let Some(target) = res.target_size {
                    target_bytes.fetch_add(target, Ordering::SeqCst);
                }
                orig_bytes.fetch_add(item.orig_size, Ordering::SeqCst);
            } else {
                failed_count.fetch_add(1, Ordering::SeqCst);
            }
        } else {
            failed_count.fetch_add(1, Ordering::SeqCst);
        }

        let done = completed_count.load(Ordering::SeqCst);
        let fails = failed_count.load(Ordering::SeqCst);
        let _ = app.emit(
            "folder-conversion-progress",
            FolderBatchProgress {
                total: total_count,
                completed: done,
                failed: fails,
                current_file: file_name,
                orig_bytes_completed: orig_bytes.load(Ordering::SeqCst),
                target_bytes_completed: target_bytes.load(Ordering::SeqCst),
                is_done: false,
            },
        );
    });

    let total_elapsed = start_all.elapsed().as_millis() as u64;
    let final_orig = orig_bytes.load(Ordering::SeqCst);
    let final_target = target_bytes.load(Ordering::SeqCst);
    let final_completed = completed_count.load(Ordering::SeqCst);
    let final_failed = failed_count.load(Ordering::SeqCst);

    let savings_bytes = if final_orig > final_target {
        (final_orig - final_target) as i64
    } else {
        0
    };
    let savings_percent = if final_orig > 0 {
        ((savings_bytes as f64 / final_orig as f64) * 100.0) as i32
    } else {
        0
    };

    let _ = app.emit(
        "folder-conversion-progress",
        FolderBatchProgress {
            total: total_count,
            completed: final_completed,
            failed: final_failed,
            current_file: "Completed".to_string(),
            orig_bytes_completed: final_orig,
            target_bytes_completed: final_target,
            is_done: true,
        },
    );

    Ok(BatchSummary {
        total_items: total_count,
        completed_items: final_completed,
        failed_items: final_failed,
        orig_total_bytes: final_orig,
        target_total_bytes: final_target,
        total_savings_bytes: savings_bytes,
        total_savings_percent: savings_percent,
        total_duration_ms: total_elapsed,
        throughput_mp_per_sec: 0.0,
    })
}

