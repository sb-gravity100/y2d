# Third-party notices

## yt-dlp

The bundled `src-tauri/binaries/yt-dlp.exe` reports version `2026.07.04` and was copied from the existing `ytdlp-nodejs` dependency in `C:\cli_tools\y2d_dl`. The upstream yt-dlp project distributes under the Unlicense; see [yt-dlp/LICENSE](https://github.com/yt-dlp/yt-dlp/blob/master/LICENSE). New versions are fetched from the pinned `yt-dlp/yt-dlp` GitHub repository and their SHA-256 digest is checked against GitHub release metadata before installation.

## FFmpeg

The optional managed FFmpeg download uses the static LGPL Windows build from [BtbN/FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds/releases). y2d extracts only `ffmpeg.exe` and `ffprobe.exe` into its per-user app-data directory. The release asset's GitHub SHA-256 digest is checked before extraction. The downloaded FFmpeg build is not included in y2d's installer.
