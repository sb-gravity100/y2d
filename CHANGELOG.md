# y2d Changelog

All notable changes to the desktop application are recorded here.

## [0.1.2] - 2026-10-27

- Fixed CUTV and application-update links being blocked by a missing opener URL scope.

## [0.1.1] - 2026-10-27

- Added a compact rounded custom Windows installer matching the application, with install location, shortcut choice, installation status, and launch.
- Added Settings checks for new stable application releases and a download-page action.
- Bundled private FFmpeg and ffprobe alongside yt-dlp; no separate tool setup or system PATH dependency.
- Suppressed downloader and version-check console windows in Windows builds.
- Kept a small CUTV companion promotion linking to its repository.

## [0.1.0] - 2026-10-27 (Early Access)

- Scaffolded the Tauri 2 desktop app with Svelte, TypeScript, and Vite.
- Added the initial downloader UI and Rust process-management commands.
- Added app-data tool management for yt-dlp and FFmpeg with SHA-256 release digest checks.
- Reshaped the app into a fixed rounded dark window with a draggable top strip and sliding history drawer.
- Simplified the Downloads screen and added clipboard URL auto-fill with a manual Paste button.
- Added configurable desktop download notifications, missing-tool warnings, and a CUTV release link.
