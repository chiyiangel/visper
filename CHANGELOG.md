# Changelog

All notable changes to this project are documented here. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Windows support (x86_64, MSVC): the same Vulkan inference with CPU fallback, data under `%LOCALAPPDATA%\Visper`, and one instance per user. The exe needs a GPU driver that provides `vulkan-1.dll` and the Visual C++ runtime.
- GitHub Actions CI on Windows and macOS. The Windows job uploads `visper.exe` as an artifact.

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
