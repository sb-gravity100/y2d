# y2d Implementation Phases

## Phase 0 — Foundation (0.1.0)

- [x] Create a Tauri 2 + TypeScript/Vite desktop project.
- [x] Move the frontend to Svelte 5 and simplify typography/layout.
- [x] Set a fixed rounded dark window, custom draggable top strip, and sliding history drawer.
- [x] Add initial Rust commands and a least-privilege capability boundary.
- [x] Set app, Cargo, and frontend package versions to `0.1.0`.
- [ ] Build and open the desktop window; resolve scaffold issues.
- **Done when:** Svelte and Rust compile, Tauri opens, and the file index matches the source tree.

## Phase 1 — Download GUI and tools (0.1.x)

- [x] URL input, output-folder picker, video/audio selection, queue view.
- [x] Start yt-dlp with argument arrays and emit progress events.
- [x] Cancel active downloads and reveal completed files.
- [x] Add y2d-managed downloads for yt-dlp and FFmpeg, with version status, progress, digest checks, and app-data installation.
- [x] Auto-fill an empty URL field from a valid clipboard link on launch/focus, with a manual Paste button and a reduced-copy Downloads screen.
- [x] Add missing-tool warnings, links to install from Settings, configurable native completion/failure notifications, and a CUTV release link.
- [ ] Exercise a video and an MP3 download and verify both installed tools.
- **Done when:** the UI can run a download using bundled or app-managed tools, show progress, cancel, and reveal the result.

## Phase 2 — Existing y2d workflows (0.2.0)

- [ ] Port Japanese audio, timestamp/section download, playlist behavior, GIF conversion, and `cb` video+audio muxing.
- [ ] Normalize downloader errors and retain useful yt-dlp diagnostics.
- **Done when:** existing core workflows are available through typed app options and job state.

## Phase 3 — Persistence (0.3.0)

- [ ] Add SQLite schema migrations for jobs/history and settings.
- [ ] Restore recent jobs safely after restart and expose searchable history.
- [ ] Add tool cleanup and rollback controls for old managed versions.
- **Done when:** settings and history persist across restarts, and tool updates are recoverable.

## Phase 4 — Browser integration (0.4.0)

- [ ] Adapt the native messaging host to submit URLs to the running app.
- [ ] Keep extension version independent and bump its manifest for extension or dependent host changes.
- [ ] Display integration status and setup instructions.
- **Done when:** supported browser actions enqueue URLs and report useful errors when the app is unavailable.

## Phase 5 — Release readiness (1.0.0)

- [ ] Create the Windows installer and establish release automation/signing.
- [ ] Add signed app updates and recovery guidance.
- [ ] Finalize privacy, supported-site limitations, third-party notices, and troubleshooting documentation.
- **Done when:** a clean Windows install can update without losing user settings or history.
