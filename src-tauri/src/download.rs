use std::{
    collections::HashMap,
    path::PathBuf,
    process::{Command as StdCommand, Stdio as StdStdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    sync::{oneshot, Mutex},
};
use url::Url;
use uuid::Uuid;

const PROGRESS_PREFIX: &str = "Y2D_PROGRESS|";
const TITLE_PREFIX: &str = "Y2D_TITLE|";
const FILE_PREFIX: &str = "Y2D_FILE|";

#[derive(Clone, Default)]
pub struct DownloadManager {
    jobs: Arc<Mutex<HashMap<String, RunningJob>>>,
}

struct RunningJob {
    task: tauri::async_runtime::JoinHandle<()>,
    cancelled: Arc<AtomicBool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    url: String,
    output_dir: String,
    mode: DownloadMode,
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
#[serde(rename_all = "lowercase")]
enum DownloadMode {
    Video,
    Audio,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadEvent {
    id: String,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<DownloadMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    progress: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    speed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    eta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

impl DownloadEvent {
    fn new(id: &str, status: &'static str) -> Self {
        Self {
            id: id.to_owned(),
            status,
            url: None,
            mode: None,
            progress: None,
            speed: None,
            eta: None,
            title: None,
            file_path: None,
            message: None,
        }
    }
}

#[tauri::command]
pub async fn default_download_dir() -> Result<String, String> {
    eprintln!("[y2d] resolving default download directory");
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .ok_or_else(|| "Could not locate the user home directory".to_string())?;
    Ok(PathBuf::from(home).join("Downloads").to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    manager: State<'_, DownloadManager>,
    request: DownloadRequest,
) -> Result<String, String> {
    let parsed_url = Url::parse(request.url.trim()).map_err(|_| "Enter a valid http or https URL".to_string())?;
    if !matches!(parsed_url.scheme(), "http" | "https") || parsed_url.host_str().is_none() {
        return Err("Only http and https URLs are supported".into());
    }

    let output_dir = PathBuf::from(request.output_dir.trim());
    if !output_dir.is_dir() {
        return Err("Choose an existing download folder".into());
    }

    let id = Uuid::new_v4().to_string();
    eprintln!("[y2d] starting download job {id} ({:?})", request.mode);
    let downloader = resolve_yt_dlp(&app)?;
    let mut command = Command::new(downloader);
    command
        .arg("--ignore-config")
        .arg("--no-colors")
        .arg("--no-simulate")
        .arg("--newline")
        .arg("--progress")
        .arg("--progress-template")
        .arg("download:Y2D_PROGRESS|%(progress.status)s|%(progress._percent_str)s|%(progress._speed_str)s|%(progress._eta_str)s")
        .arg("--print")
        .arg("before_dl:Y2D_TITLE|%(title)s")
        .arg("--print")
        .arg("after_move:Y2D_FILE|%(filepath)s")
        .arg("--paths")
        .arg(output_dir)
        .arg("--output")
        .arg("%(title)s [%(id)s].%(ext)s")
        .arg("--embed-metadata")
        .arg("--embed-thumbnail")
        .arg("--convert-thumbnails")
        .arg("jpg");

    match request.mode {
        DownloadMode::Video => {
            command.arg("--format").arg("bestvideo+bestaudio/best");
        }
        DownloadMode::Audio => {
            command
                .arg("--extract-audio")
                .arg("--audio-format")
                .arg("mp3")
                .arg("--format")
                .arg("bestaudio/best");
        }
    }

    if let Some(ffmpeg_dir) = crate::tools::managed_ffmpeg_dir(&app) {
        let mut path_entries = vec![ffmpeg_dir];
        if let Some(existing_path) = std::env::var_os("PATH") {
            path_entries.extend(std::env::split_paths(&existing_path));
        }
        let path = std::env::join_paths(path_entries)
            .map_err(|error| format!("Could not prepare the managed FFmpeg path: {error}"))?;
        command.env("PATH", path);
    }

    command
        .arg("--")
        .arg(parsed_url.as_str())
        .stdin(StdStdio::null())
        .stdout(StdStdio::piped())
        .stderr(StdStdio::piped())
        .kill_on_drop(true);

    let mut child = command.spawn().map_err(|error| {
        eprintln!("[y2d] failed to start yt-dlp for {id}: {error}");
        format!("Could not start yt-dlp: {error}. Install yt-dlp or restart the app to use its bundled copy.")
    })?;
    let stdout = child.stdout.take().ok_or_else(|| "Could not capture yt-dlp output".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "Could not capture yt-dlp errors".to_string())?;
    let cancelled = Arc::new(AtomicBool::new(false));
    let jobs = manager.jobs.clone();
    let task_jobs = jobs.clone();
    let task_app = app.clone();
    let task_id = id.clone();
    let task_cancelled = cancelled.clone();
    let (start_tx, start_rx) = oneshot::channel::<()>();

    let task = tauri::async_runtime::spawn(async move {
        if start_rx.await.is_err() {
            return;
        }
        supervise_download(task_app, task_id.clone(), child, stdout, stderr, task_cancelled, task_jobs).await;
    });
    jobs.lock().await.insert(id.clone(), RunningJob { task, cancelled });
    let mut event = DownloadEvent::new(&id, "downloading");
    event.title = Some("Preparing download".into());
    event.url = Some(request.url.trim().to_string());
    event.mode = Some(request.mode);
    emit_event(&app, event);
    let _ = start_tx.send(());
    Ok(id)
}

#[tauri::command]
pub async fn cancel_download(app: AppHandle, manager: State<'_, DownloadManager>, id: String) -> Result<(), String> {
    eprintln!("[y2d] cancel requested for job {id}");
    let job = manager.jobs.lock().await.remove(&id).ok_or_else(|| "Download is no longer active".to_string())?;
    job.cancelled.store(true, Ordering::Relaxed);
    job.task.abort();
    let mut event = DownloadEvent::new(&id, "cancelled");
    event.message = Some("Download cancelled".into());
    emit_event(&app, event);
    Ok(())
}

#[tauri::command]
pub async fn reveal_file(file_path: String) -> Result<(), String> {
    eprintln!("[y2d] reveal requested for a completed file");
    let path = PathBuf::from(file_path);
    if !path.is_file() {
        return Err("The downloaded file could not be found".into());
    }

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = StdCommand::new("explorer.exe");
        command.arg(format!("/select,{}", path.display()));
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = StdCommand::new("open");
        command.arg("-R").arg(&path);
        command
    };
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let mut command = {
        let mut command = StdCommand::new("xdg-open");
        command.arg(path.parent().unwrap_or(&path));
        command
    };

    command
        .stdin(StdStdio::null())
        .stdout(StdStdio::null())
        .stderr(StdStdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open the file location: {error}"))
}

fn resolve_yt_dlp(app: &AppHandle) -> Result<PathBuf, String> {
    crate::tools::resolve_yt_dlp(app)
}

async fn supervise_download(
    app: AppHandle,
    id: String,
    mut child: Child,
    stdout: tokio::process::ChildStdout,
    stderr: tokio::process::ChildStderr,
    cancelled: Arc<AtomicBool>,
    jobs: Arc<Mutex<HashMap<String, RunningJob>>>,
) {
    let app_for_stdout = app.clone();
    let id_for_stdout = id.clone();
    let stdout_task = tauri::async_runtime::spawn(async move {
        read_progress(app_for_stdout, id_for_stdout, stdout).await;
    });
    let stderr_lines = Arc::new(Mutex::new(Vec::<String>::new()));
    let stderr_task = tauri::async_runtime::spawn(read_errors(stderr, stderr_lines.clone()));

    let exit = child.wait().await;
    let _ = stdout_task.await;
    let _ = stderr_task.await;
    jobs.lock().await.remove(&id);

    if cancelled.load(Ordering::Relaxed) {
        return;
    }

    match exit {
        Ok(status) if status.success() => {
            eprintln!("[y2d] download job {id} completed");
            let mut event = DownloadEvent::new(&id, "completed");
            event.progress = Some(100.0);
            emit_event(&app, event);
        }
        Ok(status) => {
            let details = stderr_lines.lock().await.join("\n");
            let summary = if details.is_empty() {
                format!("yt-dlp exited with code {}", status.code().map_or_else(|| "unknown".into(), |code| code.to_string()))
            } else {
                details.lines().rev().take(3).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join(" · ")
            };
            eprintln!("[y2d] download job {id} failed: {summary}");
            let mut event = DownloadEvent::new(&id, "failed");
            event.message = Some(summary);
            emit_event(&app, event);
        }
        Err(error) => {
            eprintln!("[y2d] download job {id} process wait failed: {error}");
            let mut event = DownloadEvent::new(&id, "failed");
            event.message = Some(format!("Could not read download result: {error}"));
            emit_event(&app, event);
        }
    }
}

async fn read_progress(app: AppHandle, id: String, stdout: tokio::process::ChildStdout) {
    let mut lines = BufReader::new(stdout).lines();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) if line.starts_with(TITLE_PREFIX) => {
                let mut event = DownloadEvent::new(&id, "downloading");
                event.title = Some(line[TITLE_PREFIX.len()..].trim().to_string());
                emit_event(&app, event);
            }
            Ok(Some(line)) if line.starts_with(FILE_PREFIX) => {
                let mut event = DownloadEvent::new(&id, "downloading");
                event.file_path = Some(line[FILE_PREFIX.len()..].trim().to_string());
                emit_event(&app, event);
            }
            Ok(Some(line)) if line.starts_with(PROGRESS_PREFIX) => {
                let fields = line[PROGRESS_PREFIX.len()..].split('|').collect::<Vec<_>>();
                if fields.len() >= 4 {
                    let mut event = DownloadEvent::new(&id, "downloading");
                    event.progress = fields[1].trim().trim_end_matches('%').parse::<f64>().ok();
                    event.speed = non_empty(fields[2]);
                    event.eta = non_empty(fields[3]);
                    emit_event(&app, event);
                }
            }
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(error) => {
                eprintln!("[y2d] stdout read failed for {id}: {error}");
                break;
            }
        }
    }
}

async fn read_errors(stderr: tokio::process::ChildStderr, lines: Arc<Mutex<Vec<String>>>) {
    let mut reader = BufReader::new(stderr).lines();
    loop {
        match reader.next_line().await {
            Ok(Some(line)) => {
                let mut recent = lines.lock().await;
                if recent.len() == 20 {
                    recent.remove(0);
                }
                recent.push(line);
            }
            Ok(None) => break,
            Err(error) => {
                eprintln!("[y2d] stderr read failed: {error}");
                break;
            }
        }
    }
}

fn emit_event(app: &AppHandle, event: DownloadEvent) {
    if let Err(error) = app.emit("download-event", event) {
        eprintln!("[y2d] failed to emit download event: {error}");
    }
}

fn non_empty(value: &str) -> Option<String> {
    let cleaned = value.trim();
    (!cleaned.is_empty() && cleaned != "NA").then(|| cleaned.to_string())
}
