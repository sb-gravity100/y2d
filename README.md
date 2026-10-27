# y2d Desktop

A Windows-first download manager built with Tauri 2, Rust, Svelte 5, TypeScript, and Vite. The Rust backend runs yt-dlp as a child process and streams progress to the UI.

## Current scope

The `0.1.2` early-access UI uses a fixed, rounded dark window with one Downloads screen, Settings, and a sliding History drawer. It supports video or MP3 downloads, an output folder, progress, cancellation, and revealing completed files. Clipboard URLs can auto-fill the URL field. Completion and failure notifications can be enabled separately.

The installer includes private yt-dlp, FFmpeg, and ffprobe executables. No command-line installation or system PATH configuration is required. Windows download and tool-version subprocesses run without console windows. Settings can update private tools with SHA-256 verification and check for new stable y2d releases. Application updates open the GitHub release page for manual download; signed automatic installation is a later milestone. A small [CUTV](https://github.com/sb-gravity100/cutv_rust) promotion links to the companion video trimmer.

The custom setup is a compact rounded window matching the app. Choose a destination and shortcuts, install, then open y2d. Windows Apps settings provides uninstall support. Close y2d before upgrading. Queue/history remain in memory. Advanced download flags, browser handoff, and signing remain later milestones.

## Prerequisites

- Node.js 22 or another version supported by the current Vite toolchain.
- Rust stable with the MSVC Windows target and Tauri's Windows build prerequisites.
- Network access to GitHub to check or download current tool versions.

## Development

```powershell
npm install
npm run prepare:tools
npm run tauri dev
```

Frontend checks and build:

```powershell
npm run build
```

Build the custom Windows installer (x64):

```powershell
npm run build:setup
```

The build prepares a pinned, hash-verified FFmpeg archive, builds a per-user NSIS payload, and embeds it in the custom setup. Deliver `artifacts/y2d_0.1.2_x64-setup.exe` and its `.sha256` file. `npm run tauri build` alone produces the underlying standard installer; use `build:setup` for the matching custom interface.

The setup and application require Microsoft Edge WebView2 Runtime (normally present on supported Windows installations). The custom setup cannot display without it; install the runtime first on a machine where it is missing. Building requires network access for pinned tool assets and Rust/npm dependencies. FFmpeg resources are generated locally rather than committed to Git.

## Versioning

The release version must match in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`. Use SemVer and record user-facing changes in `CHANGELOG.md`.

See `PLAN.md`, `PHASES.md`, `FILE_INDEX.md`, and `THIRD_PARTY_NOTICES.md` for architecture, milestones, and tool provenance.

## Install

Download the Windows installer from [GitHub Releases](https://github.com/sb-gravity100/y2d/releases/latest). The early-access installer is currently unsigned; Windows may display a SmartScreen prompt.
