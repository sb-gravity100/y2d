use std::time::Duration;

use semver::Version;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdate {
    current_version: String,
    latest_version: String,
    update_available: bool,
}

fn stable_version(tag: &str) -> Result<Version, String> {
    eprintln!("[y2d] validating application release version");
    let version = Version::parse(tag.trim_start_matches('v'))
        .map_err(|_| "The latest release has an invalid version number".to_string())?;
    if !version.pre.is_empty() {
        return Err("The latest release is not a stable version".into());
    }
    Ok(version)
}

#[tauri::command]
pub async fn check_app_update(app: AppHandle) -> Result<AppUpdate, String> {
    eprintln!("[y2d] checking sb-gravity100/y2d for app updates");
    let result = async {
        let client = reqwest::Client::builder()
            .user_agent(concat!("y2d/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|error| format!("Could not prepare update check: {error}"))?;
        let mut response = client
            .get("https://api.github.com/repos/sb-gravity100/y2d/releases/latest")
            .send().await
            .map_err(|error| format!("Could not reach GitHub. Check the connection and try again: {error}"))?
            .error_for_status()
            .map_err(|error| format!("GitHub update check failed. Try again later: {error}"))?;
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
            if body.len() + chunk.len() > 1024 * 1024 {
                return Err("GitHub release metadata was unexpectedly large".into());
            }
            body.extend_from_slice(&chunk);
        }
        let release: Release = serde_json::from_slice(&body)
            .map_err(|error| format!("Could not read GitHub release information: {error}"))?;
        if release.draft || release.prerelease {
            return Err("No stable release is available".into());
        }
        let latest = stable_version(&release.tag_name)?;
        let current = &app.package_info().version;
        let available = latest > *current;
        eprintln!("[y2d] app update check complete: current={current}, latest={latest}, available={available}");
        Ok(AppUpdate { current_version: current.to_string(), latest_version: latest.to_string(), update_available: available })
    }.await;
    if let Err(error) = &result {
        eprintln!("[y2d] app update check failed: {error}");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_versions_use_semver_ordering() {
        assert!(stable_version("v0.1.10").unwrap() > stable_version("v0.1.9").unwrap());
        assert!(stable_version("v0.1.0").unwrap() < stable_version("0.1.1").unwrap());
        assert_eq!(stable_version("v0.1.1").unwrap(), stable_version("0.1.1").unwrap());
    }

    #[test]
    fn malformed_or_preview_versions_are_rejected() {
        assert!(stable_version("latest").is_err());
        assert!(stable_version("v1.0.0-beta.1").is_err());
    }
}
