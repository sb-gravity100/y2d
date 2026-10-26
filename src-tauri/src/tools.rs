use std::{
    cmp::Ordering,
    fs::File,
    io,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use reqwest::{redirect::Policy, Client};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};
use tokio::{io::AsyncWriteExt, process::Command};
use zip::ZipArchive;

const GITHUB_API: &str = "https://api.github.com/repos";
const YTDLP_REPO: &str = "yt-dlp/yt-dlp";
const FFMPEG_REPO: &str = "BtbN/FFmpeg-Builds";
const YTDLP_ASSET: &str = "yt-dlp.exe";
const FFMPEG_ASSET: &str = "ffmpeg-master-latest-win64-lgpl.zip";
const MAX_YTDLP_BYTES: u64 = 50 * 1024 * 1024;
const MAX_FFMPEG_BYTES: u64 = 300 * 1024 * 1024;
const MAX_EXTRACTED_FFMPEG_BYTES: u64 = 300 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ToolName {
    #[serde(rename = "yt-dlp")]
    YtDlp,
    #[serde(rename = "ffmpeg")]
    Ffmpeg,
}

impl ToolName {
    fn key(self) -> &'static str {
        match self {
            Self::YtDlp => "yt-dlp",
            Self::Ffmpeg => "ffmpeg",
        }
    }

    fn repo(self) -> &'static str {
        match self {
            Self::YtDlp => YTDLP_REPO,
            Self::Ffmpeg => FFMPEG_REPO,
        }
    }

    fn asset_name(self) -> &'static str {
        match self {
            Self::YtDlp => YTDLP_ASSET,
            Self::Ffmpeg => FFMPEG_ASSET,
        }
    }

    fn executable_name(self) -> &'static str {
        match self {
            Self::YtDlp => "yt-dlp.exe",
            Self::Ffmpeg => "ffmpeg.exe",
        }
    }

    fn max_asset_size(self) -> u64 {
        match self {
            Self::YtDlp => MAX_YTDLP_BYTES,
            Self::Ffmpeg => MAX_FFMPEG_BYTES,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsStatus {
    pub yt_dlp: ToolStatus,
    pub ffmpeg: ToolStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub installed: bool,
    pub installed_version: Option<String>,
    pub installed_source: String,
    pub latest_version: Option<String>,
    pub latest_size_bytes: Option<u64>,
    pub update_available: Option<bool>,
    pub error: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolDownloadProgress {
    tool: ToolName,
    state: &'static str,
    progress: f64,
    downloaded_bytes: u64,
    total_bytes: u64,
    message: Option<String>,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    published_at: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    digest: Option<String>,
}

struct InstalledTool {
    path: PathBuf,
    version: Option<String>,
    source: &'static str,
    version_key: Option<String>,
}

struct ReleaseAsset {
    version: String,
    version_key: String,
    display_version: String,
    size: u64,
    digest: String,
    url: String,
}

#[tauri::command]
pub async fn get_tool_status(app: AppHandle) -> Result<ToolsStatus, String> {
    eprintln!("[y2d] reading tool status and checking latest releases");
    let client = github_client()?;
    let (yt_release, ffmpeg_release) = tokio::join!(
        fetch_release(&client, YTDLP_REPO),
        fetch_release(&client, FFMPEG_REPO)
    );

    let yt_dlp = build_tool_status(&app, ToolName::YtDlp, yt_release).await;
    let ffmpeg = build_tool_status(&app, ToolName::Ffmpeg, ffmpeg_release).await;
    Ok(ToolsStatus { yt_dlp, ffmpeg })
}

#[tauri::command]
pub async fn install_tool(app: AppHandle, tool: ToolName) -> Result<ToolsStatus, String> {
    eprintln!("[y2d] tool install requested: {}", tool.key());
    let client = github_client()?;
    let release = fetch_release(&client, tool.repo()).await?;
    let asset = release_asset(tool, &release)?;
    let root = tool_root(&app, tool)?;
    tokio::fs::create_dir_all(&root)
        .await
        .map_err(|error| format!("Could not create the y2d tools folder: {error}"))?;

    let version_folder = root.join(&asset.version_key);
    let installed_executable = version_folder.join(tool.executable_name());
    if installed_executable.is_file() {
        if read_version(&installed_executable, tool).await.is_some() {
            eprintln!("[y2d] requested tool version is already installed: {}", tool.key());
            return get_tool_status(app).await;
        }
        eprintln!("[y2d] removing invalid managed executable for {}", tool.key());
        tokio::fs::remove_dir_all(&version_folder)
            .await
            .map_err(|error| format!("Could not replace the invalid {} install: {error}", tool.key()))?;
    }

    let downloaded_asset = download_and_verify(&app, &client, tool, &asset).await?;
    let staging = root.join(format!(".staging-{}", uuid::Uuid::new_v4()));
    let setup_result = async {
        tokio::fs::create_dir_all(&staging)
            .await
            .map_err(|error| format!("Could not prepare the tool install folder: {error}"))?;

        match tool {
            ToolName::YtDlp => {
                tokio::fs::copy(&downloaded_asset, staging.join("yt-dlp.exe"))
                    .await
                    .map_err(|error| format!("Could not prepare yt-dlp: {error}"))?;
            }
            ToolName::Ffmpeg => {
                let archive = downloaded_asset.clone();
                let destination = staging.clone();
                tauri::async_runtime::spawn_blocking(move || extract_ffmpeg(&archive, &destination))
                    .await
                    .map_err(|error| format!("FFmpeg extraction task failed: {error}"))??;
            }
        }

        let staged_executable = staging.join(tool.executable_name());
        let actual_version = read_version(&staged_executable, tool)
            .await
            .ok_or_else(|| format!("The downloaded {} executable could not be started", tool.key()))?;
        eprintln!("[y2d] verified staged {} version {actual_version}", tool.key());

        tokio::fs::rename(&staging, &version_folder)
            .await
            .map_err(|error| format!("Could not activate the new {} version: {error}", tool.key()))?;
        Ok::<(), String>(())
    }
    .await;

    let _ = tokio::fs::remove_file(&downloaded_asset).await;
    if setup_result.is_err() {
        let _ = tokio::fs::remove_dir_all(&staging).await;
        emit_progress(
            &app,
            ToolDownloadProgress {
                tool,
                state: "failed",
                progress: 0.0,
                downloaded_bytes: 0,
                total_bytes: asset.size,
                message: setup_result.as_ref().err().cloned(),
            },
        );
        return setup_result.map(|_| unreachable!());
    }

    eprintln!("[y2d] installed {} version {}", tool.key(), asset.display_version);
    emit_progress(
        &app,
        ToolDownloadProgress {
            tool,
            state: "complete",
            progress: 100.0,
            downloaded_bytes: asset.size,
            total_bytes: asset.size,
            message: Some(format!("Installed {}", asset.display_version)),
        },
    );
    get_tool_status(app).await
}

pub fn resolve_yt_dlp(app: &AppHandle) -> Result<PathBuf, String> {
    if let Some(installed) = latest_managed_tool(app, ToolName::YtDlp) {
        return Ok(installed.path);
    }

    if let Ok(resources) = app.path().resource_dir() {
        let bundled = resources.join("binaries").join("yt-dlp.exe");
        if bundled.is_file() {
            return Ok(bundled);
        }
    }

    let development_copy = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join("yt-dlp.exe");
    if development_copy.is_file() {
        return Ok(development_copy);
    }

    Ok(PathBuf::from("yt-dlp.exe"))
}

pub fn managed_ffmpeg_dir(app: &AppHandle) -> Option<PathBuf> {
    latest_managed_tool(app, ToolName::Ffmpeg)
        .and_then(|tool| tool.path.parent().map(Path::to_path_buf))
}

async fn build_tool_status(
    app: &AppHandle,
    tool: ToolName,
    release_result: Result<GitHubRelease, String>,
) -> ToolStatus {
    let installed = latest_managed_tool(app, tool).or_else(|| fallback_tool(app, tool));
    let installed_version = match &installed {
        Some(tool_info) if tool_info.version.is_some() => tool_info.version.clone(),
        Some(tool_info) => read_version(&tool_info.path, tool).await,
        None => None,
    };
    let installed_source = installed
        .as_ref()
        .map(|tool_info| tool_info.source.to_string())
        .unwrap_or_else(|| "missing".to_string());
    let installed_key = installed.as_ref().and_then(|tool_info| tool_info.version_key.clone());

    match release_result {
        Ok(release) => match release_asset(tool, &release) {
            Ok(asset) => {
                let update_available = if installed.is_none() {
                    Some(true)
                } else if tool == ToolName::Ffmpeg && installed_source != "managed" {
                    Some(true)
                } else if let Some(current_key) = installed_key.as_deref() {
                    Some(compare_versions(current_key, &asset.version_key) == Ordering::Less)
                } else if let Some(current_version) = installed_version.as_deref() {
                    compare_versions_optional(current_version, &asset.version)
                } else {
                    None
                };

                ToolStatus {
                    installed: installed_version.is_some(),
                    installed_version,
                    installed_source,
                    latest_version: Some(asset.display_version),
                    latest_size_bytes: Some(asset.size),
                    update_available,
                    error: None,
                }
            }
            Err(error) => ToolStatus {
                installed: installed_version.is_some(),
                installed_version,
                installed_source,
                latest_version: None,
                latest_size_bytes: None,
                update_available: None,
                error: Some(error),
            },
        },
        Err(error) => ToolStatus {
            installed: installed_version.is_some(),
            installed_version,
            installed_source,
            latest_version: None,
            latest_size_bytes: None,
            update_available: None,
            error: Some(error),
        },
    }
}

fn fallback_tool(app: &AppHandle, tool: ToolName) -> Option<InstalledTool> {
    match tool {
        ToolName::YtDlp => {
            if let Ok(resources) = app.path().resource_dir() {
                let bundled = resources.join("binaries").join("yt-dlp.exe");
                if bundled.is_file() {
                    return Some(InstalledTool {
                        path: bundled,
                        version: None,
                        source: "bundled",
                        version_key: None,
                    });
                }
            }
            let development_copy = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("binaries")
                .join("yt-dlp.exe");
            if development_copy.is_file() {
                return Some(InstalledTool {
                    path: development_copy,
                    version: None,
                    source: "bundled",
                    version_key: None,
                });
            }
            Some(InstalledTool {
                path: PathBuf::from(tool.executable_name()),
                version: None,
                source: "system",
                version_key: None,
            })
        }
        ToolName::Ffmpeg => Some(InstalledTool {
            path: PathBuf::from(tool.executable_name()),
            version: None,
            source: "system",
            version_key: None,
        }),
    }
}

fn latest_managed_tool(app: &AppHandle, tool: ToolName) -> Option<InstalledTool> {
    let root = tool_root(app, tool).ok()?;
    let mut candidates = std::fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .filter_map(|entry| {
            let version_key = entry.file_name().to_string_lossy().into_owned();
            if version_key.starts_with('.') {
                return None;
            }
            let executable = entry.path().join(tool.executable_name());
            executable.is_file().then_some((version_key, executable))
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| compare_versions(&left.0, &right.0));
    let (version_key, path) = candidates.pop()?;
    Some(InstalledTool {
        path,
        version: None,
        source: "managed",
        version_key: Some(version_key),
    })
}

fn tool_root(app: &AppHandle, tool: ToolName) -> Result<PathBuf, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not locate y2d's application data folder: {error}"))?;
    Ok(app_data.join("tools").join(tool.key()))
}

async fn fetch_release(client: &Client, repo: &str) -> Result<GitHubRelease, String> {
    let url = format!("{GITHUB_API}/{repo}/releases/latest");
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("Could not reach GitHub release metadata: {error}"))?
        .error_for_status()
        .map_err(|error| format!("GitHub did not return a latest release: {error}"))?;
    response
        .json::<GitHubRelease>()
        .await
        .map_err(|error| format!("Could not read GitHub release metadata: {error}"))
}

fn release_asset(tool: ToolName, release: &GitHubRelease) -> Result<ReleaseAsset, String> {
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == tool.asset_name())
        .ok_or_else(|| format!("The latest {} release has no supported Windows asset", tool.key()))?;
    if asset.size == 0 || asset.size > tool.max_asset_size() {
        return Err(format!("The {} release asset has an unexpected size", tool.key()));
    }

    let expected_prefix = format!("https://github.com/{}/releases/download/", tool.repo());
    if !asset.browser_download_url.starts_with(&expected_prefix) {
        return Err(format!("The {} release asset URL was not from its pinned GitHub repository", tool.key()));
    }

    let digest = asset
        .digest
        .as_deref()
        .and_then(|value| value.strip_prefix("sha256:"))
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| format!("The latest {} release does not provide a valid SHA-256 digest", tool.key()))?
        .to_ascii_lowercase();
    let version_key = match tool {
        ToolName::YtDlp => safe_version_component(&release.tag_name)?,
        ToolName::Ffmpeg => {
            let digits = release.published_at.chars().filter(char::is_ascii_digit).collect::<String>();
            if digits.len() < 14 {
                return Err("The latest FFmpeg release has an invalid timestamp".into());
            }
            digits[..14].to_string()
        }
    };

    let display_version = match tool {
        ToolName::YtDlp => release.tag_name.clone(),
        ToolName::Ffmpeg => release
            .name
            .clone()
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| format!("Build {}", &version_key)),
    };

    Ok(ReleaseAsset {
        version: release.tag_name.clone(),
        version_key,
        display_version,
        size: asset.size,
        digest,
        url: asset.browser_download_url.clone(),
    })
}

fn safe_version_component(version: &str) -> Result<String, String> {
    let cleaned = version
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_'))
        .take(64)
        .collect::<String>();
    if cleaned.is_empty() || cleaned != version {
        return Err("The release version contains unsupported path characters".into());
    }
    Ok(cleaned)
}

async fn download_and_verify(
    app: &AppHandle,
    client: &Client,
    tool: ToolName,
    asset: &ReleaseAsset,
) -> Result<PathBuf, String> {
    let root = tool_root(app, tool)?;
    let temp_dir = root.join(".downloads");
    tokio::fs::create_dir_all(&temp_dir)
        .await
        .map_err(|error| format!("Could not prepare a temporary download folder: {error}"))?;
    let temp_path = temp_dir.join(format!("{}.{}.download", tool.key(), uuid::Uuid::new_v4()));

    let result = async {
        emit_progress(
            app,
            ToolDownloadProgress {
                tool,
                state: "downloading",
                progress: 0.0,
                downloaded_bytes: 0,
                total_bytes: asset.size,
                message: None,
            },
        );
        let mut response = client
            .get(&asset.url)
            .send()
            .await
            .map_err(|error| format!("Could not download {}: {error}", tool.key()))?
            .error_for_status()
            .map_err(|error| format!("GitHub could not provide {}: {error}", tool.key()))?;
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .await
            .map_err(|error| format!("Could not create the temporary tool file: {error}"))?;

        let mut hasher = Sha256::new();
        let mut downloaded = 0_u64;
        let mut last_emitted_percent = -1_i32;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| format!("The {} download was interrupted: {error}", tool.key()))?
        {
            downloaded = downloaded.saturating_add(chunk.len() as u64);
            if downloaded > asset.size || downloaded > tool.max_asset_size() {
                return Err(format!("The {} download exceeded its declared size", tool.key()));
            }
            file.write_all(&chunk)
                .await
                .map_err(|error| format!("Could not write the {} download: {error}", tool.key()))?;
            hasher.update(&chunk);
            let percent = ((downloaded as f64 / asset.size as f64) * 100.0).floor() as i32;
            if percent > last_emitted_percent {
                last_emitted_percent = percent;
                emit_progress(
                    app,
                    ToolDownloadProgress {
                        tool,
                        state: "downloading",
                        progress: percent as f64,
                        downloaded_bytes: downloaded,
                        total_bytes: asset.size,
                        message: None,
                    },
                );
            }
        }
        file.flush()
            .await
            .map_err(|error| format!("Could not finish writing the {} download: {error}", tool.key()))?;
        drop(file);

        if downloaded != asset.size {
            return Err(format!(
                "The {} download was incomplete ({downloaded} of {} bytes)",
                tool.key(),
                asset.size
            ));
        }

        let actual_digest = hex::encode(hasher.finalize());
        if actual_digest != asset.digest {
            return Err(format!("The {} download failed its SHA-256 check", tool.key()));
        }
        eprintln!("[y2d] verified {} release asset digest", tool.key());
        Ok::<PathBuf, String>(temp_path.clone())
    }
    .await;

    if result.is_err() {
        let _ = tokio::fs::remove_file(&temp_path).await;
    }
    result
}

fn extract_ffmpeg(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let source = File::open(archive_path)
        .map_err(|error| format!("Could not open the verified FFmpeg archive: {error}"))?;
    let mut archive = ZipArchive::new(source)
        .map_err(|error| format!("The FFmpeg download is not a valid ZIP archive: {error}"))?;
    let mut found_ffmpeg = false;
    let mut found_ffprobe = false;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("Could not inspect the FFmpeg archive: {error}"))?;
        let Some(path) = entry.enclosed_name() else {
            continue;
        };
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let is_bin_file = path.components().any(|component| component.as_os_str() == "bin");
        if !is_bin_file {
            continue;
        }

        let target_name = if file_name.eq_ignore_ascii_case("ffmpeg.exe") {
            found_ffmpeg = true;
            Some("ffmpeg.exe")
        } else if file_name.eq_ignore_ascii_case("ffprobe.exe") {
            found_ffprobe = true;
            Some("ffprobe.exe")
        } else {
            None
        };
        let Some(target_name) = target_name else {
            continue;
        };
        if entry.size() > MAX_EXTRACTED_FFMPEG_BYTES {
            return Err(format!("{target_name} in the FFmpeg archive is unexpectedly large"));
        }

        let target = destination.join(target_name);
        let mut output = File::create(&target)
            .map_err(|error| format!("Could not extract {target_name}: {error}"))?;
        io::copy(&mut entry, &mut output)
            .map_err(|error| format!("Could not extract {target_name}: {error}"))?;
        output
            .sync_all()
            .map_err(|error| format!("Could not flush {target_name}: {error}"))?;
    }

    if !found_ffmpeg {
        return Err("The FFmpeg archive did not contain bin/ffmpeg.exe".into());
    }
    if !found_ffprobe {
        eprintln!("[y2d] FFmpeg archive contains no ffprobe.exe; continuing with ffmpeg only");
    }
    Ok(())
}

async fn read_version(path: &Path, tool: ToolName) -> Option<String> {
    let argument = match tool {
        ToolName::YtDlp => "--version",
        ToolName::Ffmpeg => "-version",
    };
    let output = Command::new(path)
        .arg(argument)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let first_line = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()?
        .trim()
        .to_string();
    match tool {
        ToolName::YtDlp => (!first_line.is_empty()).then_some(first_line),
        ToolName::Ffmpeg => first_line.split_whitespace().nth(2).map(ToString::to_string),
    }
}

fn github_client() -> Result<Client, String> {
    Client::builder()
        .user_agent("y2d-tool-manager/0.1.0")
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(1800))
        .redirect(Policy::custom(|attempt| {
            if attempt.url().scheme() == "https" && attempt.previous().len() < 5 {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .map_err(|error| format!("Could not initialize secure downloads: {error}"))
}

fn compare_versions(left: &str, right: &str) -> Ordering {
    let left_parts = left.split('.').collect::<Vec<_>>();
    let right_parts = right.split('.').collect::<Vec<_>>();
    for index in 0..left_parts.len().max(right_parts.len()) {
        let left_part = left_parts.get(index).and_then(|part| part.parse::<u64>().ok()).unwrap_or(0);
        let right_part = right_parts.get(index).and_then(|part| part.parse::<u64>().ok()).unwrap_or(0);
        match left_part.cmp(&right_part) {
            Ordering::Equal => {}
            ordering => return ordering,
        }
    }
    left.cmp(right)
}

fn compare_versions_optional(current: &str, latest: &str) -> Option<bool> {
    let current_numeric = current
        .split('.')
        .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()));
    let latest_numeric = latest
        .split('.')
        .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()));
    (current_numeric && latest_numeric).then(|| compare_versions(current, latest) == Ordering::Less)
}

fn emit_progress(app: &AppHandle, progress: ToolDownloadProgress) {
    if let Err(error) = app.emit("tool-download-progress", progress) {
        eprintln!("[y2d] failed to emit tool download progress: {error}");
    }
}
