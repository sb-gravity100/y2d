# Third-party notices

## yt-dlp

The bundled `src-tauri/binaries/yt-dlp.exe` reports version `2026.07.04` and was copied from the existing `ytdlp-nodejs` dependency in `C:\cli_tools\y2d_dl`. The upstream yt-dlp project distributes under the Unlicense; see [yt-dlp/LICENSE](https://github.com/yt-dlp/yt-dlp/blob/master/LICENSE). New versions are fetched from the pinned `yt-dlp/yt-dlp` GitHub repository and their SHA-256 digest is checked against GitHub release metadata before installation.

## FFmpeg

The bundled and optional managed FFmpeg tools use the static LGPL Windows build from [BtbN/FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds/releases). y2d extracts only `ffmpeg.exe` and `ffprobe.exe` into its per-user app-data directory. The release asset's GitHub SHA-256 digest is checked before extraction. The installer includes FFmpeg and ffprobe from BtbN autobuild `2026-10-26-14-25`, FFmpeg `N-128100-gceb466cbcd`, static LGPL variant. Archive SHA-256: `a46195b14ea18385570a9b154868813495195f199b9c886d67b16d474c883b34`. `scripts/prepare-tools.ps1` downloads and verifies that pinned build, extracts both binaries and upstream license/README material into `binaries/ffmpeg-notices/`, which ships in the installer. Upstream [source and build scripts](https://github.com/BtbN/FFmpeg-Builds/tree/autobuild-2026-10-26-14-25) and [FFmpeg source](https://github.com/FFmpeg/FFmpeg/commit/ceb466cbcd) are available for rebuilding; FFmpeg is under LGPL v2.1 or later for this variant. See the bundled license files for dependency terms.
