use std::path::PathBuf;
use tauri::{AppHandle, command};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::engine::batch::execute_batch;
use crate::engine::detector::{SystemInfo, get_system_info};
use crate::engine::formats::{
    ImageItemInfo, inspect_image, scan_folder_for_images, scan_folder_for_jxl,
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
    for path in paths {
        if let Ok(info) = inspect_image(&path) {
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
pub fn cmd_convert_batch(
    app: AppHandle,
    items: Vec<ImageItemInfo>,
    profile: EncodingProfile,
) -> Vec<ConversionResult> {
    execute_batch(&app, items, profile)
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
pub fn cmd_convert_legacy_batch(
    app: AppHandle,
    items: Vec<ImageItemInfo>,
    profile: LegacyExportProfile,
) -> Vec<ConversionResult> {
    execute_legacy_batch(&app, items, profile)
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

    let image_paths = scan_folder_for_images(&folder_path);
    let mut total_bytes = 0u64;
    for p in &image_paths {
        if let Ok(meta) = std::fs::metadata(p) {
            total_bytes += meta.len();
        }
    }

    Ok(FolderScanSummary {
        folder_path: path,
        folder_name,
        image_count: image_paths.len(),
        total_bytes,
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

    let jxl_paths = scan_folder_for_jxl(&folder_path);
    let mut total_bytes = 0u64;
    for p in &jxl_paths {
        if let Ok(meta) = std::fs::metadata(p) {
            total_bytes += meta.len();
        }
    }

    Ok(FolderScanSummary {
        folder_path: path,
        folder_name,
        image_count: jxl_paths.len(),
        total_bytes,
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
                let child_paths = if is_jxl_mode {
                    scan_folder_for_jxl(&path)
                } else {
                    scan_folder_for_images(&path)
                };
                let mut total_bytes = 0u64;
                for p in &child_paths {
                    if let Ok(m) = std::fs::metadata(p) {
                        total_bytes += m.len();
                    }
                }
                folder_summary = Some(FolderScanSummary {
                    folder_path: path_str.clone(),
                    folder_name,
                    image_count: child_paths.len(),
                    total_bytes,
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

            if is_supported
                && let Ok(info) = inspect_image(&path_str)
            {
                items.push(info);
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
pub fn cmd_convert_folder_batch(
    app: AppHandle,
    folder_path: String,
    profile: EncodingProfile,
) -> Result<crate::engine::batch::BatchSummary, String> {
    crate::engine::batch::execute_folder_batch(&app, &folder_path, profile)
}

#[command]
pub fn cmd_convert_legacy_folder_batch(
    app: AppHandle,
    folder_path: String,
    profile: LegacyExportProfile,
) -> Result<crate::engine::batch::BatchSummary, String> {
    crate::engine::legacy::execute_legacy_folder_batch(&app, &folder_path, profile)
}


