# y2d Desktop App Plan

## Objective

Build a Windows-first desktop GUI for the existing y2d downloader in this workspace, using Tauri 2 with a Rust backend and Svelte/TypeScript frontend. Preserve useful workflows from `C:\cli_tools\y2d_dl`: URL downloads, audio/video modes, GIF conversion, section downloads, video/audio combination, progress, completion reveal, and browser-extension handoff in later releases.

## Stack and boundaries

- **Desktop runtime:** Tauri 2.
- **Backend:** stable Rust with Tokio for process supervision and job events; serde for IPC payloads.
- **Frontend:** Svelte 5 + TypeScript + Vite. Svelte components compile to optimized client code; keep the UI dependency-light.
- **Downloader:** Ship the bundled yt-dlp executable as a private fallback. Allow downloading the official yt-dlp Windows release into y2d's app-data tools folder.
- **FFmpeg:** Bundle a pinned static LGPL Windows build (ffmpeg and ffprobe) from BtbN/FFmpeg-Builds and offer app-managed updates. Install it in a versioned app-data folder without changing system PATH or requiring administrator access.
- **Integrity:** Read release metadata from GitHub's official REST API, accept only the pinned repository and exact asset names, require its SHA-256 digest, verify size and hash before installation, and stage files before marking a version installed.
- **Persistence:** SQLite migrations for queue/history/settings remain a later milestone.
- **Versioning:** SemVer starting at `0.1.0`, synchronized across `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`. Extension version remains independent.

The renderer may call only narrow Tauri commands for downloads, tool status/install, folder selection, and file reveal. Keep HTTP clients, archive extraction, process execution, and filesystem operations in Rust. Validate URLs, paths, asset hosts, file sizes, release asset names, and hashes in Rust.

## Initial UX

Use a fixed 1120×790 borderless dark window with rounded corners. The top strip is draggable and contains only the app mark and a close button at the upper-right. Keep one Downloads screen, with Settings as a separate view and History in a right-side sliding drawer. Use a large URL field, video/MP3 choice, output folder, current jobs, and progress. Detect an HTTP(S) URL from the clipboard on launch/window focus only when the field is empty, with a manual Paste button. Show warnings and a Settings shortcut when yt-dlp or FFmpeg is unavailable. Keep the screen copy minimal and include a compact CUTV repository link. Settings contains the destination, yt-dlp/FFmpeg version controls, and per-event native desktop notification preferences for completed or failed downloads. Use Cascadia Code with Consolas fallback and readable type sizes.

## Current limitations

Queue/history are currently in memory. Advanced yt-dlp workflows, browser-extension handoff, installer signing, and signed automatic app updates remain later milestones. Manual application update checks and the custom Windows setup are available in 0.1.1. FFmpeg is bundled; optional tool updates show the exact release asset size before install. The custom setup requires Microsoft Edge WebView2 Runtime to display its interface.

## Release 0.1.1 — Desktop distribution

- Add an explicit Settings app-update check against `sb-gravity100/y2d` latest stable GitHub release. Compare SemVer and open the pinned release page; do not silently install unsigned app updates.
- Ship FFmpeg and ffprobe alongside yt-dlp; resolve bundled or app-managed tools without requiring system PATH. Hide Windows child processes during downloads and version probes.
- Deliver a standalone custom setup executable: a compact 560×440 rounded dark Tauri window matching the app (Cascadia Code, #0d1216 base, #141b20 surfaces, #84d5bd actions, #e3b17b mark). Left-align installation details with destination selection, busy state, errors, completion and launch. Embed a per-user NSIS payload and run it silently for reliable Windows registration, shortcuts and uninstall support.
- Keep the existing compact CUTV companion promotion and point it to the requested repository.
- Build the custom setup through a reproducible script; record bundled tool digests/provenance and include third-party licensing material.

## Fix 0.1.2 - External links

Allow only the CUTV repository and y2d latest-release URLs in the opener capability. The command permission alone does not grant a URL scope. Rebuild the custom setup with the corrected capability.
