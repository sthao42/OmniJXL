use std::path::PathBuf;
use tauri::{AppHandle, command};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::engine::batch::execute_batch;
use crate::engine::detector::{SystemInfo, get_system_info};
use crate::engine::formats::{
    ImageItemInfo, inspect_image, inspect_image_without_thumbnail,
    scan_folder_for_images_with_size, scan_folder_for_jxl_with_size,
};
use crate::engine::jxl::{
    ConversionResult, EncodingProfile, convert_single_image, resolve_destination_folder,
};
use crate::engine::legacy::{
    LegacyExportProfile, convert_single_jxl_to_legacy, execute_legacy_batch,
};

#[command]
pub fn cmd_get_system_info() -> SystemInfo {
    get_system_info()
}

#[command]
pub fn cmd_inspect_images(paths: Vec<String>) -> Vec<ImageItemInfo> {
    let mut items = Vec::new();
    let generate_thumbnails = paths.len() <= 64;
    for path in paths {
        let res = if generate_thumbnails {
            inspect_image(&path)
        } else {
            inspect_image_without_thumbnail(&path)
        };
        if let Ok(info) = res {
            items.push(info);
        }
    }
    items
}

#[command]
pub fn cmd_convert_single(item: ImageItemInfo, profile: EncodingProfile) -> ConversionResult {
    convert_single_image(&item, &profile)
}

#[command]
pub async fn cmd_convert_batch(
    app: AppHandle,
    items: Vec<ImageItemInfo>,
    profile: EncodingProfile,
) -> Result<Vec<ConversionResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        execute_batch(&app, items, profile)
    })
    .await
    .map_err(|e| format!("Batch execution thread error: {}", e))
}

#[command]
pub async fn cmd_select_folder(app: AppHandle) -> Result<Option<String>, String> {
    let folder = app.dialog().file().blocking_pick_folder();
    Ok(folder.map(|f| f.to_string()))
}

#[command]
pub async fn cmd_select_files(app: AppHandle) -> Result<Vec<String>, String> {
    let files = app
        .dialog()
        .file()
        .add_filter(
            "Images",
            &[
                "png", "jpg", "jpeg", "webp", "avif", "tiff", "tif", "bmp", "gif", "exr", "hdr",
                "ico", "tga", "cr2", "nef", "arw", "dng", "ppm", "pam", "pfm", "pgx",
            ],
        )
        .blocking_pick_files();

    match files {
        Some(file_list) => Ok(file_list.into_iter().map(|f| f.to_string()).collect()),
        None => Ok(Vec::new()),
    }
}

#[command]
pub fn cmd_open_folder(app: AppHandle, path: Option<String>) -> Result<(), String> {
    let resolved_path = if let Some(p) = path {
        let p_buf = PathBuf::from(&p);
        if p_buf.is_dir() {
            p_buf
        } else if p_buf.is_file() {
            p_buf
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .to_path_buf()
        } else {
            resolve_destination_folder(&None, std::path::Path::new("."))
        }
    } else {
        resolve_destination_folder(&None, std::path::Path::new("."))
    };

    let path_str = resolved_path.to_string_lossy().to_string();
    app.opener()
        .open_path(path_str, None::<&str>)
        .map_err(|e| format!("Failed to open folder: {}", e))
}

#[command]
pub async fn cmd_select_jxl_files(app: AppHandle) -> Result<Vec<String>, String> {
    let files = app
        .dialog()
        .file()
        .add_filter("JPEG XL Images", &["jxl"])
        .blocking_pick_files();

    match files {
        Some(file_list) => Ok(file_list.into_iter().map(|f| f.to_string()).collect()),
        None => Ok(Vec::new()),
    }
}

#[command]
pub fn cmd_convert_legacy_single(
    item: ImageItemInfo,
    profile: LegacyExportProfile,
) -> ConversionResult {
    convert_single_jxl_to_legacy(&item, &profile)
}

#[command]
pub async fn cmd_convert_legacy_batch(
    app: AppHandle,
    items: Vec<ImageItemInfo>,
    profile: LegacyExportProfile,
) -> Result<Vec<ConversionResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        execute_legacy_batch(&app, items, profile)
    })
    .await
    .map_err(|e| format!("Legacy batch execution thread error: {}", e))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FolderScanSummary {
    pub folder_path: String,
    pub folder_name: String,
    pub image_count: usize,
    pub total_bytes: u64,
}

#[command]
pub fn cmd_scan_folder_summary(path: String) -> Result<FolderScanSummary, String> {
    let folder_path = PathBuf::from(&path);
    if !folder_path.is_dir() {
        return Err(format!("Not a valid directory: {}", path));
    }
    let folder_name = folder_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Folder")
        .to_string();

    let scanned = scan_folder_for_images_with_size(&folder_path);

    Ok(FolderScanSummary {
        folder_path: path,
        folder_name,
        image_count: scanned.paths.len(),
        total_bytes: scanned.total_bytes,
    })
}

#[command]
pub fn cmd_scan_folder_jxl_summary(path: String) -> Result<FolderScanSummary, String> {
    let folder_path = PathBuf::from(&path);
    if !folder_path.is_dir() {
        return Err(format!("Not a valid directory: {}", path));
    }
    let folder_name = folder_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Folder")
        .to_string();

    let scanned = scan_folder_for_jxl_with_size(&folder_path);

    Ok(FolderScanSummary {
        folder_path: path,
        folder_name,
        image_count: scanned.paths.len(),
        total_bytes: scanned.total_bytes,
    })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProcessedDropResult {
    pub items: Vec<ImageItemInfo>,
    pub folder_summary: Option<FolderScanSummary>,
    pub total_folders: usize,
    pub total_files: usize,
}

#[command]
pub fn cmd_handle_dropped_paths(paths: Vec<String>, is_jxl_mode: bool) -> ProcessedDropResult {
    let mut items = Vec::new();
    let mut folder_summary = None;
    let mut total_folders = 0;
    let mut total_files = 0;
    let generate_thumbnails = paths.len() <= 64;

    for path_str in paths {
        let path = PathBuf::from(&path_str);
        if path.is_dir() {
            total_folders += 1;
            if folder_summary.is_none() {
                let folder_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Folder")
                    .to_string();
                let scanned = if is_jxl_mode {
                    scan_folder_for_jxl_with_size(&path)
                } else {
                    scan_folder_for_images_with_size(&path)
                };
                folder_summary = Some(FolderScanSummary {
                    folder_path: path_str.clone(),
                    folder_name,
                    image_count: scanned.paths.len(),
                    total_bytes: scanned.total_bytes,
                });
            }
        } else if path.is_file() {
            total_files += 1;
            let is_supported = if is_jxl_mode {
                path.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("jxl"))
                    .unwrap_or(false)
            } else {
                true
            };

            if is_supported {
                let res = if generate_thumbnails {
                    inspect_image(&path_str)
                } else {
                    inspect_image_without_thumbnail(&path_str)
                };
                if let Ok(info) = res {
                    items.push(info);
                }
            }
        }
    }

    ProcessedDropResult {
        items,
        folder_summary,
        total_folders,
        total_files,
    }
}

#[command]
pub async fn cmd_convert_folder_batch(
    app: AppHandle,
    folder_path: String,
    profile: EncodingProfile,
) -> Result<crate::engine::batch::BatchSummary, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::engine::batch::execute_folder_batch(&app, &folder_path, profile)
    })
    .await
    .map_err(|e| format!("Folder batch task failed: {}", e))?
}

#[command]
pub async fn cmd_convert_legacy_folder_batch(
    app: AppHandle,
    folder_path: String,
    profile: LegacyExportProfile,
) -> Result<crate::engine::batch::BatchSummary, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::engine::legacy::execute_legacy_folder_batch(&app, &folder_path, profile)
    })
    .await
    .map_err(|e| format!("Legacy folder batch task failed: {}", e))?
}

#[command]
pub fn cmd_cancel_batch() {
    crate::engine::batch::cancel_batch();
}


