#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{path::PathBuf, process::Stdio, sync::{atomic::{AtomicBool, Ordering}, Mutex}};
use tauri::{Manager, State};
use tokio::process::Command;
use winreg::{enums::HKEY_CURRENT_USER, RegKey};
use std::os::windows::process::CommandExt;

const PAYLOAD: &[u8] = include_bytes!(env!("Y2D_INSTALLER_PAYLOAD"));

#[derive(Default)]
struct SetupState {
    busy: AtomicBool,
    installed: Mutex<Option<PathBuf>>,
}

#[tauri::command]
fn setup_destination(app: tauri::AppHandle) -> Result<String, String> {
    eprintln!("[y2d-setup] reading installation destination");
    let existing = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\y2d")
        .ok().and_then(|key| key.get_value::<String, _>("InstallLocation").ok().map(|path| path.trim_matches('"').to_string()))
        .filter(|path| PathBuf::from(path).join("y2d.exe").is_file());
    if let Some(path) = existing { return Ok(path); }
    app.path().local_data_dir().map(|root| root.join("y2d").to_string_lossy().into_owned())
        .map_err(|error| format!("Could not find the installation folder: {error}"))
}

fn validate_destination(destination: &str) -> Result<PathBuf, String> {
    eprintln!("[y2d-setup] validating selected installation directory");
    if destination.chars().any(|c| c.is_control() || c == '"') {
        return Err("Choose a valid local installation folder".into());
    }
    let path = PathBuf::from(destination.trim());
    if !path.is_absolute() || path.parent().is_none() || destination.starts_with("\\\\") {
        return Err("Choose a local folder below a drive root".into());
    }
    if path.is_file() { return Err("The installation location is a file".into()); }
    if path.is_dir() && !path.join("y2d.exe").is_file() {
        let mut entries = std::fs::read_dir(&path).map_err(|error| error.to_string())?;
        if entries.next().is_some() { return Err("Choose an empty folder or an existing y2d installation".into()); }
    }
    Ok(path)
}

#[tauri::command]
async fn setup_install(destination: String, shortcuts: bool, state: State<'_, SetupState>) -> Result<(), String> {
    eprintln!("[y2d-setup] installation requested; shortcuts={shortcuts}");
    if state.busy.swap(true, Ordering::SeqCst) {
        return Err("Installation is already running".into());
    }
    let result = async {
        let destination = validate_destination(&destination)?;
        *state.installed.lock().map_err(|e| e.to_string())? = None;
        let temp = std::env::temp_dir().join(format!("y2d-setup-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir(&temp).await.map_err(|e| e.to_string())?;
        let payload = temp.join("install.exe");
        let install_result = async {
            tokio::fs::write(&payload, PAYLOAD).await.map_err(|e| e.to_string())?;
            eprintln!("[y2d-setup] starting silent per-user Windows installation");
            let mut command = Command::new(&payload);
            command.arg("/S");
            if !shortcuts { command.arg("/NS"); }
            // NSIS requires /D last, without quotes, including paths containing spaces.
            command.as_std_mut().raw_arg(format!("/D={}", destination.display()));
            command.creation_flags(0x08000000).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
            let status = command.status().await.map_err(|e| format!("Could not start installation: {e}"))?;
            if !status.success() {
                return Err(format!("Installation failed (code {:?}). Close y2d, check folder permissions, and retry.", status.code()));
            }
            if !destination.join("y2d.exe").is_file() {
                return Err("Installation did not create y2d in the selected folder. Retry after closing any running copy.".into());
            }
            *state.installed.lock().map_err(|e| e.to_string())? = Some(destination);
            Ok(())
        }.await;
        if let Err(error) = tokio::fs::remove_dir_all(&temp).await {
            eprintln!("[y2d-setup] warning: temporary payload cleanup failed: {error}");
        }
        install_result
    }.await;
    state.busy.store(false, Ordering::SeqCst);
    match &result {
        Ok(()) => eprintln!("[y2d-setup] installation completed"),
        Err(error) => eprintln!("[y2d-setup] installation failed: {error}"),
    }
    result
}

#[tauri::command]
fn setup_launch(state: State<'_, SetupState>) -> Result<(), String> {
    eprintln!("[y2d-setup] launching installed application");
    let folder = state.installed.lock().map_err(|e| e.to_string())?.clone()
        .ok_or("Complete installation before launching y2d")?;
    std::process::Command::new(folder.join("y2d.exe"))
        .current_dir(folder).creation_flags(0x08000000)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .spawn().map(|_| ()).map_err(|e| {
            eprintln!("[y2d-setup] launch failed: {e}");
            format!("Could not launch y2d: {e}")
        })
}

fn main() {
    eprintln!("[y2d-setup] opening custom installer");
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(SetupState::default())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.state::<SetupState>().busy.load(Ordering::SeqCst) {
                    eprintln!("[y2d-setup] postponing close until installation finishes");
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![setup_destination, setup_install, setup_launch])
        .run(tauri::generate_context!())
        .expect("Could not open y2d Setup. Microsoft Edge WebView2 Runtime is required.");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsafe_installation_destinations_are_rejected() {
        for path in ["relative", "C:\\", "\\\\server\\share", "C:\\bad\"path", "C:\\bad\npath"] {
            assert!(validate_destination(path).is_err(), "{path}");
        }
    }
}
