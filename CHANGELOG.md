# Changelog

All notable changes to this project are documented here. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Windows support (x86_64, MSVC): the same Vulkan inference with CPU fallback, data under `%LOCALAPPDATA%\Visper`, and one instance per user. The exe needs a GPU driver that provides `vulkan-1.dll` and the Visual C++ runtime.
- GitHub Actions CI on Windows and macOS. The Windows job uploads `visper.exe` as an artifact, the macOS job a disk image.
- `scripts/bundle-macos.sh` builds `Visper.app` and a `.dmg`: ad hoc signed by default, or signed with a Developer ID, hardened runtime and notarization when configured. The bundle is localized in Simplified Chinese and opens `m4a`, `mp3`, `wav`, `flac` and `aac` files as an alternate app.
- A menu bar (Visper, 文件, 编辑, 窗口, 帮助) with the standard macOS shortcuts: ⌘O adds a recording, ⌘C copies the selected transcript text, ⌘W and ⌘Q quit, plus ⌘H, ⌥⌘H and ⌘M. On Windows, Ctrl+O and Ctrl+C.
- Recordings opened from Finder (Open With) or dropped on the Dock icon are added to the library, and show up in the Dock icon's recent items.
- The standard About panel, and a window that reopens at its last size and position.
- Quitting or closing the window while a recording is transcribing asks first, since it cancels that transcription.
- Choose Large v3 or Large v3 Turbo from the Library footer. Turbo is faster and slightly less accurate; picking a Model that is not on disk yet downloads it, and picking back one that is abandons the download.
- On first launch Visper asks which Model to download instead of fetching one on its own.
- Each transcript shows the Model that produced it: a Large or Turbo tag on its row, and the full name in the info line.
- A scrollbar on the Transcript and on the Library list: a thin overlay thumb that shows while scrolling or hovering, fades out after a second, can be dragged, and pages when the track is clicked. It honours Reduce motion.
- Recordings can only be added once a Model is on disk; until then 加入录音, 选择录音, ⌘O, drops and files opened from Finder are refused.

### Changed

- On macOS, inference runs on whisper.cpp's native Metal backend instead of Vulkan through MoltenVK, about 1.6× faster on an M3 Pro and with about 0.5 GB less memory ([ADR 0003](docs/adr/0003-metal-on-macos.md)). Building on macOS now needs only CMake; the Vulkan loader, MoltenVK, shaderc and `VULKAN_SDK` are no longer used.
- Flash attention is on for GPU inference. Without it, the Vulkan path dropped words.

### Fixed

- On macOS, quitting after a model had been loaded crashed with a Metal assertion from ggml. Quitting now frees the model first.

## [0.1.0]

First public version. macOS only.

### Added

- Local transcription of `m4a`, `mp3`, `wav`, `flac` and `aac` recordings with Whisper large-v3 through whisper.cpp, on Vulkan (MoltenVK) with CPU fallback.
- Automatic download of `ggml-large-v3.bin` with progress and retry.
- Auto language mode that detects Chinese or English per silence-separated chunk, plus forced 中文 and 英文 modes.
- Traditional-to-Simplified Chinese conversion of every transcript.
- A persistent library with a one-at-a-time transcription queue, live segments, cancel, re-run and missing-file detection.
- Segment playback: click a segment to play from that point.
- Markdown export.
- Light and dark appearance.

[Unreleased]: https://github.com/chiyiangel/visper/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/chiyiangel/visper/releases/tag/v0.1.0
