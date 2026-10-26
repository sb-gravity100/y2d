# AGENTS.md — General Session Rules

General-purpose session rules, independent of any specific project or tech stack. Copy this into any project's `AGENTS.md` as a base.

---

## ⚡ START EVERY SESSION HERE

Before writing any code or reading any source file, always do this first:

1. **Ask the user to provide `PLAN.md`, `PHASES.md`, and `FILE_INDEX.md`** if they are not already in context. Do not proceed until at least `FILE_INDEX.md` is available.
2. Read `FILE_INDEX.md` (repo root) — it maps every file to its purpose and tags. Use it to find the exact file you need without scanning the tree.
3. Read `AGENTS.md` in full.

**Do not scan directories or read source files until you have identified the target files via `FILE_INDEX.md`.**

---

## Token efficiency — CRITICAL

Every file read and every output costs money. Minimize both aggressively.

- **Read only what you need.** Use `FILE_INDEX.md` to identify the exact file before opening anything. Never open a file speculatively.
- **Read only the relevant section.** Use `view_range` when you need one function or block, not the whole file.
- **No unnecessary confirmations.** Don't summarize what you're about to do before doing it. Don't recap what you just did after doing it. Act, then move on.
- **No padding.** No filler phrases ("Great question!", "Sure, I'll help with that", "Here's what I did"). Responses should contain only information the user needs.
- **Prefer targeted edits.** Use `str_replace` on the specific lines that change. Never rewrite a whole file to change a few lines.
- **One read per file per task.** Read a file once, make all needed changes, move on. Don't re-read files you already have in context.

---

## Session saves — CRITICAL

**The user's PC crashes unpredictably, anywhere between 10 minutes and 1 hour into a session.**

- After every logical unit of work: finish the change → `git commit` → update `commits.md`.
- A "logical unit" is: one file created, one feature completed, one test passing, one doc updated.
- Never leave more than one uncommitted logical change in the working tree.
- Every commit gets its own entry prepended to `commits.md` (format below).

---

## Logging rules

**Every function, endpoint, event handler, service call, and error path must be logged.**

Log at:
- Entry point (params/body — never credentials)
- Service/handler entry
- DB or external API queries
- Branch decisions
- Exception raises
- Significant state transitions

**Never log passwords, PINs, or raw tokens of any kind.**

Use appropriate log levels:
- `debug` — fine-grained flow
- `info` — significant events
- `warning` — recoverable anomalies
- `error` — failures

---

## Key workflow rules

### commits.md rule
After every commit, **prepend** a new entry to `commits.md`:
```
### `<short-hash>` · <YYYY-MM-DD> · <commit subject>
- `affected/file1`
- `affected/file2`
```
`commits.md` itself gets its own commit: `docs: update commits.md`.

### Commit granularity
Every distinguishable change gets its own commit. Never bundle unrelated changes. If you need "and" in the commit message, it should be two commits.

### Straggler sweep
After any rename, removal, or refactor — grep for old references across all code and doc files before committing.

### Planning doc sync
When any change affects design, behaviour, schema, or architecture — update the relevant planning docs in the same task:
- `PLAN.md` — tech stack, modules, API endpoints, DB schema
- `PHASES.md` — per-phase task lists
- `FILE_INDEX.md` — file system index (update when files are added, moved, or removed)

All planning docs must stay consistent with each other at all times.

---

## Reference documents (adapt per project)
- `FILE_INDEX.md` — **file system index with tags and descriptions — read this first**
- `PLAN.md` — full tech stack, API endpoints, schema
- `PHASES.md` — detailed per-phase tasks and done criteria
- `commits.md` — commit log (prepend after every commit)
---

# y2d Project Context

## Purpose and current state

`y2d` is a Windows-first desktop download manager being rebuilt from the existing Node/Electron project at `C:\cli_tools\y2d_dl`. The current workspace is a Tauri 2 desktop scaffold using a Svelte/TypeScript/Vite frontend and Rust backend. Version is `0.1.0`. The app has a fixed, borderless dark window with a Downloads view, separate Settings view, and sliding History drawer; the frontend build and Rust cargo check pass; the desktop window and live downloads have not yet been exercised.

## Architecture

- `src/App.svelte` and `src/app.css` provide the download, history, settings, and tool manager UI.
- `src-tauri/src/lib.rs` configures Tauri and registers commands.
- `src-tauri/src/download.rs` validates URL and destination input, starts/cancels yt-dlp child processes, emits progress events, and reveals downloaded files.
- `src-tauri/src/tools.rs` checks GitHub releases, verifies asset SHA-256 digests, and installs versioned yt-dlp/FFmpeg copies under app data.
- `src-tauri/binaries/yt-dlp.exe` is the bundled yt-dlp executable (version `2026.07.04`). FFmpeg can run from `PATH` or from y2d-managed app-data installs. Tool downloads do not modify system PATH.
- Tauri capabilities grant only the dialog folder picker, clipboard text reads, notification controls, and external URL opening needed by the renderer; downloader process execution stays in Rust.
- Keep `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` versions aligned. Bump the app version using SemVer and record user-facing changes in `CHANGELOG.md`.

## Commands

- `npm install` — install frontend dependencies.
- `npm run tauri dev` — launch the desktop app in development.
- `npm run build` — type-check and build the frontend.
- `npm run tauri build` — create a desktop bundle.

## Project documents

- `PLAN.md` — architecture and scope.
- `PHASES.md` — milestone checklist.
- `FILE_INDEX.md` — file map; read before exploring source files.
- `README.md` — setup and runtime requirements.

The existing browser extension and native messaging host remain in `C:\cli_tools\y2d_dl`; browser handoff is a later milestone for this workspace. Keep that integration separately versioned and bump its manifest version whenever extension code or dependent host behavior changes.
