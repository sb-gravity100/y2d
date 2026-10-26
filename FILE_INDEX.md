# File Index

This index maps workspace files to their purpose. Update it when files are added, moved, or removed.

## Planning and project guidance

- `AGENTS.md` — general session rules and project-specific Codex context.
- `PLAN.md` — app architecture, scope, constraints, and version targets.
- `PHASES.md` — milestones and completion criteria.
- `FILE_INDEX.md` — this file map.
- `README.md` — setup and runtime requirements.
- `CHANGELOG.md` — user-facing release notes.
- `RELEASE_NOTES.md` — notes for the current GitHub release.
- `commits.md` — repository commit log.
- `THIRD_PARTY_NOTICES.md` — provenance and licenses for bundled/downloaded tools.
- `.gitignore`, `.gitattributes` — ignored build outputs and line-ending policy.
- `.vscode/extensions.json` — recommended editor extensions.

## Frontend

- `package.json`, `package-lock.json` — frontend scripts, dependencies, and version.
- `index.html` — Vite SPA entry document.
- `vite.config.ts`, `svelte.config.js` — Svelte/Vite development, preprocessing, and build configuration.
- `tsconfig.json`, `tsconfig.node.json` — TypeScript compiler configuration.
- `src/main.ts` — Svelte application entry point.
- `src/App.svelte` — download queue, settings, and tool manager UI.
- `src/app.css` — global typography and application styling.
- `src/vite-env.d.ts` — Vite client type declarations.

## Tauri backend

- `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` — Rust package metadata and locked dependencies.
- `src-tauri/build.rs` — Tauri build integration.
- `src-tauri/src/main.rs` — Windows desktop entry point.
- `src-tauri/src/lib.rs` — Tauri builder, command registration, and managed state.
- `src-tauri/src/download.rs` — URL validation, yt-dlp process lifecycle, progress events, cancellation, and file reveal.
- `src-tauri/src/tools.rs` — GitHub release lookup, SHA-256 verification, managed tool installation, and tool status.
- `src-tauri/tauri.conf.json` — app identity, version, window, CSP, build, and bundle settings.
- `src-tauri/capabilities/default.json` — main window permissions.
- `src-tauri/binaries/yt-dlp.exe` — bundled yt-dlp fallback executable.
- `src-tauri/icons/y2d.svg` — editable source for generated app/installer icons.
- `src-tauri/icons/` — generated platform app and installer icons.

## Generated and local-only

- `node_modules/`, `src-tauri/target/`, `dist/` — generated dependencies/build outputs; do not hand-edit.
- `src-tauri/gen/` — generated Tauri schemas; regenerate through Tauri tooling.
- App-data `tools/` — per-user managed binaries, installed outside the source tree.
