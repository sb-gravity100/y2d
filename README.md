# y2d Desktop

A Windows-first download manager built with Tauri 2, Rust, Svelte 5, TypeScript, and Vite. The Rust backend runs yt-dlp as a child process and streams progress to the UI.

## Current scope

The `0.1.0` early-access UI uses a fixed, rounded dark window with one Downloads screen, a Settings view, and a sliding History drawer. It supports video or MP3 downloads, an output-folder picker, progress, cancellation, and revealing completed files. Clipboard URLs can auto-fill the URL field. Download completion and failure notifications can be enabled separately in Settings. Missing yt-dlp or FFmpeg appears as a warning on Downloads. The installer includes yt-dlp `2026.07.04` as a fallback. Settings can download or update yt-dlp and FFmpeg into y2d's app-data folder. Release asset hashes are checked before installation; the app does not change system PATH. The Downloads screen also links to [CUTV](https://github.com/sb-gravity100/cutv_rust/releases/latest), a companion Windows video trimmer with GIF export.

FFmpeg is an optional managed download using BtbN's static LGPL Windows build (about 170 MiB). An existing FFmpeg on system PATH is also detected. Queue/history remain in memory. Advanced download flags, browser handoff, installer signing, and app auto-updates are later milestones.

## Prerequisites

- Node.js 22 or another version supported by the current Vite toolchain.
- Rust stable with the MSVC Windows target and Tauri's Windows build prerequisites.
- Network access to GitHub to check or download current tool versions.

## Development

```powershell
npm install
npm run tauri dev
```

Frontend checks and build:

```powershell
npm run build
```

Create a desktop bundle:

```powershell
npm run tauri build
```

## Versioning

The release version must match in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`. Use SemVer and record user-facing changes in `CHANGELOG.md`.

See `PLAN.md`, `PHASES.md`, `FILE_INDEX.md`, and `THIRD_PARTY_NOTICES.md` for architecture, milestones, and tool provenance.

## Install

Download the Windows installer from [GitHub Releases](https://github.com/sb-gravity100/y2d/releases/latest). The early-access installer is currently unsigned; Windows may display a SmartScreen prompt.
