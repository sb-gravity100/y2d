# y2d Desktop App Plan

## Objective

Build a Windows-first desktop GUI for the existing y2d downloader in this workspace, using Tauri 2 with a Rust backend and Svelte/TypeScript frontend. Preserve useful workflows from `C:\cli_tools\y2d_dl`: URL downloads, audio/video modes, GIF conversion, section downloads, video/audio combination, progress, completion reveal, and browser-extension handoff in later releases.

## Stack and boundaries

- **Desktop runtime:** Tauri 2.
- **Backend:** stable Rust with Tokio for process supervision and job events; serde for IPC payloads.
- **Frontend:** Svelte 5 + TypeScript + Vite. Svelte components compile to optimized client code; keep the UI dependency-light.
- **Downloader:** Keep the bundled yt-dlp executable as a fallback. Allow downloading the official yt-dlp Windows release into y2d's app-data tools folder.
- **FFmpeg:** Offer an app-managed static LGPL Windows build from BtbN/FFmpeg-Builds. Install it in a versioned app-data folder without changing system PATH or requiring administrator access.
- **Integrity:** Read release metadata from GitHub's official REST API, accept only the pinned repository and exact asset names, require its SHA-256 digest, verify size and hash before installation, and stage files before marking a version installed.
- **Persistence:** SQLite migrations for queue/history/settings remain a later milestone.
- **Versioning:** SemVer starting at `0.1.0`, synchronized across `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`. Extension version remains independent.

The renderer may call only narrow Tauri commands for downloads, tool status/install, folder selection, and file reveal. Keep HTTP clients, archive extraction, process execution, and filesystem operations in Rust. Validate URLs, paths, asset hosts, file sizes, release asset names, and hashes in Rust.

## Initial UX

Use a fixed 1120×790 borderless dark window with rounded corners. The top strip is draggable and contains only the app mark and a close button at the upper-right. Keep one Downloads screen, with Settings as a separate view and History in a right-side sliding drawer. Use a large URL field, video/MP3 choice, output folder, current jobs, and progress. Detect an HTTP(S) URL from the clipboard on launch/window focus only when the field is empty, with a manual Paste button. Show warnings and a Settings shortcut when yt-dlp or FFmpeg is unavailable. Keep the screen copy minimal and include a compact CUTV link to its latest release. Settings contains the destination, yt-dlp/FFmpeg version controls, and per-event native desktop notification preferences for completed or failed downloads. Use Cascadia Code with Consolas fallback and readable type sizes.

## Current limitations

Queue/history are currently in memory. Advanced yt-dlp workflows, browser-extension handoff, installer signing, and auto-updates remain later milestones. FFmpeg download size is approximately 170 MiB; show the exact release asset size before install.
