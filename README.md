<p align="center">
  <img src="assets/brand/app-icon-512.png" width="112" alt="Visper icon">
</p>

<h1 align="center">Visper</h1>

<p align="center">
  <strong>Private, offline transcription for the recordings you already have.</strong><br>
  Chinese, English, or both in the same recording. Whisper large-v3 running on your own GPU.
</p>

<p align="center">
  English · <a href="README.zh-CN.md">简体中文</a> · <a href="https://visper-site.vercel.app">Website</a>
</p>

<p align="center">
  <img alt="Platforms: macOS (Apple Silicon) and Windows x86_64" src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows-426650">
  <img alt="Written in Rust" src="https://img.shields.io/badge/built%20with-Rust%20%2B%20GPUI-426650">
  <img alt="License: MIT OR Apache-2.0" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-426650">
</p>

---

Visper is a native desktop app for turning audio files (meetings, lectures, interviews) into timestamped transcripts. It runs OpenAI's Whisper **large-v3** or **large-v3-turbo** model locally through [whisper.cpp](https://github.com/ggml-org/whisper.cpp), on Metal on macOS and Vulkan on Windows. Nothing is uploaded: the model, your audio and your transcripts stay on your machine.

It is written in Rust with [GPUI](https://www.gpui.rs/), the UI framework behind the Zed editor.

> [!NOTE]
> **Status: early (v0.1).** Visper runs on macOS (Apple Silicon) and Windows 10/11 (x86_64). The interface is in Simplified Chinese.
> Prebuilt downloads are **coming soon**. For now, [build it from source](#build-from-source).

## Contents

- [Features](#features)
- [What it looks like](#what-it-looks-like)
- [How it works](#how-it-works)
- [Requirements](#requirements)
- [Build from source](#build-from-source)
- [Using Visper](#using-visper)
- [Settings and data](#settings-and-data)
- [Development](#development)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [License](#license)

## Features

- **Fully offline.** After a one-time model download, transcription never touches the network. The only network access Visper makes is fetching the model weights from Hugging Face over HTTPS.
- **Mixed Chinese and English.** In *自动 (Auto)* mode the recording is split on silence and each stretch is detected as Chinese or English on its own, so a switch halfway through does not drag the rest of the transcript into the wrong language. You can also lock a recording to *中文* or *英文*. See [ADR 0002](docs/adr/0002-zh-en-per-chunk.md).
- **Simplified Chinese output.** Traditional characters are converted to Simplified with OpenCC before anything is shown or saved.
- **Two models, your choice.**
  | Model | Weight file | Size | Best for |
  | --- | --- | --- | --- |
  | Large v3 | `ggml-large-v3.bin` | ≈ 3.1 GB | Highest accuracy: interviews full of names and terms |
  | Large v3 Turbo | `ggml-large-v3-turbo.bin` | ≈ 1.6 GB | Much faster, slightly less accurate: everyday meetings |

  Both are unquantized. You pick one on first launch and can switch from the Library footer at any time; each transcript records which model produced it. See [ADR 0004](docs/adr/0004-large-v3-or-turbo.md).
- **GPU acceleration with a safety net.** Metal on macOS, Vulkan on Windows, flash attention on the GPU path, and an automatic fallback to the CPU when the GPU cannot be set up. See [ADR 0003](docs/adr/0003-metal-on-macos.md) and the [Vulkan vs. Metal benchmark](docs/whisper-vulkan-vs-metal-bench.md).
- **A queue, not a wizard.** Drop recordings on the window or pick them with ⌘O / Ctrl+O. They are transcribed one at a time, in order. Segments appear live, with progress, speed and an estimated time remaining.
- **Listen back.** Click any segment to play the recording from that point.
- **Markdown export.** One `[HH:MM:SS] text` block per segment, saved as `<recording name>.md` into a folder you choose.
- **A library that remembers.** Recordings and transcripts persist between launches. Re-run a finished recording, cancel a running one, or remove one from the library. If a file has moved, it is marked *文件缺失 (missing)* and its transcript stays readable and exportable. Your audio files are never copied.
- **A real desktop app.** On macOS: a native menu bar with the standard shortcuts, Open With from Finder, dropping files on the Dock icon, the About panel, and a window that reopens where you left it. On Windows: themed window controls and a single instance per user. On both: light and dark appearance that follows the system, and a prompt before quitting would cancel a running transcription.

Supported audio formats: `m4a`, `mp3`, `wav`, `flac`, `aac`.

### Out of scope

Visper is intentionally small. It does **not** do live microphone capture, translation, summaries, speaker labels, transcript editing, custom prompts or vocabularies, or languages other than Chinese and English. Cantonese is transcribed through the Chinese path with no quality guarantee.

## What it looks like

There are no screenshots in the repository yet. The [design canvas](design-canvas/) recreates every screen of the app (first launch, model download, the queue, every status, playback, export, light and dark) as live HTML drawn from the GPUI code, and you can browse it online at **[visper-chi.vercel.app](https://visper-chi.vercel.app)**.

<!-- TODO: add real screenshots of the app (light and dark) under docs/screenshots/ and show them here. -->

## How it works

```text
 audio file ──► decode ──► resample ──► split on silence ──► detect language ──► whisper.cpp ──► Simplified ──► Library
 m4a/mp3/wav   Symphonia   16 kHz mono  stretches ≤ 30 s     zh or en per         beam search 5   OpenCC t2s     library.json
 flac/aac                  Rubato       (20 ms RMS frames)   stretch (Auto mode)  Metal / Vulkan                 + live UI
                                                                                  / CPU fallback
```

1. **Decode and resample.** [Symphonia](https://github.com/pdeljanov/Symphonia) decodes the file and [Rubato](https://github.com/HEnquist/rubato) resamples it to 16 kHz mono, the rate Whisper expects.
2. **Split on silence.** Loudness is measured in 20 ms frames against an adaptive noise floor. Pauses of 400 ms or more separate stretches of speech, and a stretch longer than one 30-second Whisper window is cut at the longest breath inside it. Stretches with almost no voiced audio are skipped.
3. **Pick a language per stretch.** In Auto mode each stretch is run through Whisper's language detector and judged as only Chinese or English (Cantonese counts toward Chinese). Neighbouring stretches in the same language are merged into one window, so a long recording does not pay one pass per sentence.
4. **Transcribe.** Each window is decoded by whisper.cpp (via [whisper-rs](https://codeberg.org/tazz4843/whisper-rs)) with beam search, no translation and no carried-over context. The GPU context is created once and reused for later recordings; if it cannot be created, that one transcription runs on the CPU.
5. **Store.** Segments are converted to Simplified Chinese, streamed to the window as they arrive, and saved to `library.json`. A failed or cancelled transcription keeps no partial result.

### Code layout

| Path | Contents |
| --- | --- |
| `src/library.rs` | The domain model: recordings, statuses, the queue, model choice, export. No GUI dependency, fully unit-tested |
| `src/transcribe.rs` | Decoding, resampling, silence splitting, language detection and whisper.cpp inference on a worker thread |
| `src/download.rs` | Model download into a `.part` file with progress, retry and stall detection |
| `src/store.rs` | Atomic saving and loading of `library.json` |
| `src/playback.rs` | Audio playback with [Rodio](https://github.com/RustAudio/rodio) |
| `src/ui.rs`, `src/ui/` | The GPUI window, rendering, motion, scrollbars and debug previews |
| `src/menu.rs` | Menu bar and keyboard shortcuts |
| `src/instance.rs` | One Visper per user: a Unix socket on macOS, a named pipe on Windows |
| `src/macos.rs` | AppKit calls GPUI does not make: About panel, Window and Help menus, saved window frame, Dock icon |
| `packaging/macos/`, `scripts/` | `Info.plist`, the `Visper.app` / `.dmg` bundler, and the icon generator |
| `design-canvas/` | A static site that recreates every screen for design review |
| `docs/adr/` | Architecture decision records |
| `docs/design/` | Visual and interaction spec |
| `GLOSSARY.md` | Domain terms (in Chinese) used in code and UI |

## Requirements

**To run**

- macOS on Apple Silicon, or Windows 10/11 on x86_64
- About 3.1 GB of free disk space for Large v3 (1.6 GB for Turbo), plus enough memory to load it
- On Windows: a GPU driver that provides Vulkan (`vulkan-1.dll`; current NVIDIA, AMD and Intel drivers all do) and the [Microsoft Visual C++ Redistributable](https://learn.microsoft.com/cpp/windows/latest-supported-vc-redist). Visper does not start without `vulkan-1.dll`.

**To build**

- Rust stable. [`rust-toolchain.toml`](rust-toolchain.toml) pins the stable channel with `rustfmt` and `clippy`, so `rustup` installs it automatically.
- CMake, which compiles whisper.cpp, plus the platform tools below.

Linux is not a supported target yet.

## Build from source

### macOS

Needs the Xcode Command Line Tools and [Homebrew](https://brew.sh).

```bash
brew install cmake
git clone https://github.com/chiyiangel/visper.git
cd visper
cargo run --release
```

The first build compiles whisper.cpp with Metal and takes several minutes.

#### Build `Visper.app` and a disk image

`cargo run` starts a bare executable. For an app you can keep in Applications, with its icon and Finder's Open With:

```bash
scripts/bundle-macos.sh --dmg
```

This writes `Visper.app` and `Visper-<version>-arm64.dmg` to `target/release/bundle/`. The bundle is signed ad hoc, so it opens on the Mac that built it; on another Mac it has to be allowed under System Settings → Privacy & Security first. To sign and notarize for distribution, set:

| Variable | Value |
| --- | --- |
| `VISPER_SIGN_IDENTITY` | A *Developer ID Application* certificate in your keychain. Signs with the hardened runtime. |
| `VISPER_NOTARY_PROFILE` | A profile saved with `xcrun notarytool store-credentials`. Notarizes and staples the disk image. |

### Windows (x86_64)

1. Install the build dependencies:
   - [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/) with the **Desktop development with C++** workload (MSVC and the Windows SDK)
   - [CMake](https://cmake.org/download/)
   - The [LunarG Vulkan SDK](https://vulkan.lunarg.com/sdk/home#windows). Its installer sets `VULKAN_SDK`, which the build needs; open a new terminal afterwards.
2. Use a short Cargo target directory. MSBuild cannot handle paths over 260 characters, and whisper.cpp's build nests deep enough that a target directory longer than about 20 characters fails with `FTK1011` or `MSB8066`:

   ```powershell
   setx CARGO_TARGET_DIR C:\t
   ```

3. In a new terminal:

   ```powershell
   git clone https://github.com/chiyiangel/visper.git
   cd visper
   cargo run --release
   ```

Only `x86_64-pc-windows-msvc` is supported. Release builds are a GUI-subsystem executable with no console window.

## Using Visper

1. **Choose a model.** On first launch Visper asks which model to download (Large v3 or Large v3 Turbo) and shows the download progress. Recordings can be added once a model is on disk.
2. **Add recordings.** Drag files onto the window, click 选择录音 / 加入录音, or press ⌘O (Ctrl+O on Windows). On macOS you can also use Finder's Open With or drop files on the Dock icon.
3. **Pick a language** for a queued recording if you want: 自动 (Auto, the default), 中文 or 英文.
4. **Watch it transcribe.** Segments appear as they are recognised, with progress, speed and time remaining. Queued recordings wait their turn, and pick up again the next time you open Visper if you quit.
5. **Read, listen and export.** Click a segment to play from there, copy text with ⌘C / Ctrl+C, and export the transcript as Markdown. Use 重新转写 (re-run) to transcribe a finished recording again, for example after switching models, or remove it from the library from the ⋯ menu.

An exported file looks like this:

```markdown
[00:00:04] 我们先看一下这次产品讨论的目标。

[00:00:18] 界面需要更清晰，让主要操作自然出现。

[00:00:36] We should keep the workflow simple and predictable.
```

Switching the model only affects transcriptions that start afterwards; existing transcripts keep the model that produced them.

## Settings and data

Visper has no configuration file. The two choices it remembers are the current model (chosen in the app) and each recording's language. Everything else lives in one folder:

| Platform | Folder |
| --- | --- |
| macOS | `~/Library/Application Support/Visper/` |
| Windows | `%LOCALAPPDATA%\Visper\` (local, not roaming, so the weights do not follow a domain account) |

| File | What it is |
| --- | --- |
| `ggml-large-v3.bin`, `ggml-large-v3-turbo.bin` | The downloaded model weights (`.part` while downloading) |
| `library.json` | Your library: recording paths, statuses and transcripts |
| `instance.sock` | macOS only: keeps a second launch from opening a second window (Windows uses a named pipe) |

Weights are downloaded from [`ggerganov/whisper.cpp`](https://huggingface.co/ggerganov/whisper.cpp) on Hugging Face. To reset Visper, quit it and delete that folder. On macOS the window position is stored separately in the user defaults (`io.github.chiyiangel.visper`, or `visper` when run with `cargo run`).

## Development

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
```

CI runs the same checks on Windows and macOS for every push and pull request. The domain model in `src/library.rs` has no GUI dependency and can be checked on its own, on any OS:

```bash
cargo check --lib --no-default-features
```

### Preview scenarios

Debug builds can open isolated preview windows that use sample data instead of your library and never download a model:

```bash
cargo run -- --preview completed
```

Scenarios: `empty`, `transcribing`, `completed`, `failure`, `missing`, `long`, `model-popover`, `model-download`, `model-download-failed`, `first-launch`, `first-launch-download`, `first-launch-failed`. Append `-dark` for dark mode, e.g. `completed-dark`.

### Design canvas

[`design-canvas/`](design-canvas/) is a Vite site that redraws every screen from the strings and sizes in `src/ui/`. Click any control to copy a reference (`frame`, element id, `file:line`) you can paste into a review. See its [README](design-canvas/README.md).

### App icon

After changing the SVGs in `assets/brand/`, regenerate the icon with `scripts/make-app-icon.sh` (needs `brew install librsvg`).

## Roadmap

- **Prebuilt downloads** (coming soon): an Apple Silicon `.dmg` and a Windows x86_64 `.zip` on GitHub Releases. The [release workflow](.github/workflows/release.yml) already builds them from `v*` tags.
- **Signed and notarized macOS builds.** The bundler supports Developer ID signing and notarization; release builds are ad hoc signed for now.

The scope stays deliberately narrow (see [Out of scope](#out-of-scope)). Ideas and bug reports are welcome in [Issues](https://github.com/chiyiangel/visper/issues).

## Contributing

Issues and pull requests are welcome, in English or Chinese. Please read [CONTRIBUTING.md](CONTRIBUTING.md) first: it covers the scope, the domain vocabulary in [GLOSSARY.md](GLOSSARY.md), the [visual spec](docs/design/visual-spec-v1.md) and the checks a pull request must pass. Changes go into [CHANGELOG.md](CHANGELOG.md).

To report a security problem, follow [SECURITY.md](SECURITY.md) instead of opening a public issue.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

The Whisper model weights are not part of this repository. They are published by OpenAI under the MIT license and downloaded at runtime from Hugging Face.

## Acknowledgements

- [OpenAI Whisper](https://github.com/openai/whisper) for the model
- [whisper.cpp](https://github.com/ggml-org/whisper.cpp) and [whisper-rs](https://codeberg.org/tazz4843/whisper-rs) for inference
- [GPUI](https://www.gpui.rs/) by Zed Industries for the interface
- [Symphonia](https://github.com/pdeljanov/Symphonia), [Rubato](https://github.com/HEnquist/rubato) and [Rodio](https://github.com/RustAudio/rodio) for audio
- [ferrous-opencc](https://crates.io/crates/ferrous-opencc) for Traditional-to-Simplified conversion
