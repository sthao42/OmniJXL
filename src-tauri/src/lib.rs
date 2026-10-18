pub mod commands;
pub mod engine;

use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            cmd_get_system_info,
            cmd_inspect_images,
            cmd_convert_single,
            cmd_convert_batch,
            cmd_select_folder,
            cmd_select_files,
            cmd_open_folder,
            cmd_scan_folder_summary,
            cmd_scan_folder_jxl_summary,
            cmd_select_jxl_files,
            cmd_convert_legacy_single,
            cmd_convert_legacy_batch,
            cmd_handle_dropped_paths,
            cmd_convert_folder_batch,
            cmd_convert_legacy_folder_batch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running OmniJXL application");
}
