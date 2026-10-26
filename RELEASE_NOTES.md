# y2d 0.1.0 Early Access

Initial Windows release of y2d, a lightweight downloader built with Tauri 2.

## Included

- Video and MP3 downloads with progress, cancellation, and file reveal.
- Clipboard URL detection and a manual Paste button.
- Optional verified yt-dlp and FFmpeg downloads.
- Configurable desktop notifications for completed and failed downloads.
- Missing-tool warnings and links to CUTV, the companion video trimmer.

## Known limitations

- Queue and history are kept in memory and reset when the app closes.
- Advanced downloader workflows and browser handoff are not included yet.
- Video and MP3 downloads have not yet been exercised as part of release verification.
- The installer is unsigned; Windows may display a SmartScreen prompt.
