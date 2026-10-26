mod download;
mod tools;

use download::{cancel_download, default_download_dir, reveal_file, start_download, DownloadManager};
use tools::{get_tool_status, install_tool};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .manage(DownloadManager::default())
        .invoke_handler(tauri::generate_handler![
            start_download,
            cancel_download,
            default_download_dir,
            reveal_file,
            get_tool_status,
            install_tool
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
