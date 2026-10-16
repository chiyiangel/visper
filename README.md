<p align="center">
  <img src="assets/brand/app-icon-512.png" width="112" alt="Visper icon">
</p>

<h1 align="center">Visper</h1>

<p align="center">
  Turn the recordings you already have into transcripts, entirely on your own Mac.
</p>

<p align="center">
  English · <a href="README.zh-CN.md">简体中文</a>
</p>

---

Visper is a native desktop app, written in Rust with [GPUI](https://www.gpui.rs/), that transcribes local audio files with OpenAI's Whisper **large-v3** model through [whisper.cpp](https://github.com/ggml-org/whisper.cpp). Nothing is uploaded: the model, the audio and the transcripts all stay on your machine.

It is built for meetings, lectures and interviews in **Simplified Chinese, English, or a mix of both**.

> **Status:** early (v0.1). Runs on macOS and Windows. The interface is in Simplified Chinese.

## Features

- **Fully offline.** After the one-time model download, transcription needs no network.
- **Mixed Chinese and English.** In *Auto* mode the recording is split on silence and every chunk is detected as Chinese or English on its own, so a switch halfway through does not drag the rest off course. You can also force 中文 or 英文 for a whole recording. See [ADR 0002](docs/adr/0002-zh-en-per-chunk.md).
- **Simplified Chinese only.** Traditional characters are converted to Simplified (via OpenCC) before anything is shown or saved.
- **Full-quality model.** Uses the unquantized `ggml-large-v3.bin` (about 3.1 GB), downloaded by the app with progress on first launch.
- **GPU when available.** Inference runs on Metal on macOS and on Vulkan on Windows, and falls back to the CPU if the GPU cannot be set up. See [ADR 0003](docs/adr/0003-metal-on-macos.md).
- **A queue, not a wizard.** Add recordings with the file picker or by dragging them onto the window; they are transcribed one at a time, in order. Segments appear live while a recording is being transcribed.
- **Listen back.** Click any segment to play the recording from that point.
- **Markdown export.** One `[HH:MM:SS] text` block per segment.
- **A library that remembers.** Recordings and transcripts persist between launches. A recording whose file has moved is marked as missing, and its transcript stays readable.
- Light and dark appearance follow the system.

Supported audio formats: `m4a`, `mp3`, `wav`, `flac`, `aac`.

### Out of scope

Visper deliberately does not do live microphone capture, translation, summaries, speaker labels, transcript editing, custom prompts or vocabularies, or languages other than Chinese and English. Cantonese is transcribed as Chinese with no quality guarantee.

## Requirements

- macOS (Apple Silicon recommended), or Windows 10/11 on x86_64
- About 3.1 GB of free disk space for the model, plus enough memory to load it
- On Windows, a GPU driver that provides Vulkan (`vulkan-1.dll`; current NVIDIA, AMD and Intel drivers all do) and the [Microsoft Visual C++ Redistributable](https://learn.microsoft.com/cpp/windows/latest-supported-vc-redist). Without `vulkan-1.dll` Visper does not start.
- To build: Rust stable (installed automatically through `rust-toolchain.toml`), plus the platform tools below

## Build and run

There are no prebuilt releases yet, so build from source. ### macOS

Needs Xcode Command Line Tools and [Homebrew](https://brew.sh).

1. Install CMake, which compiles whisper.cpp:

   ```bash
   brew install cmake
   ```

2. Clone and run:

   ```bash
   git clone https://github.com/chiyiangel/visper.git
   ```

   ```bash
   cd visper && cargo run --release
   ```

The first build compiles whisper.cpp and takes several minutes. On first launch Visper downloads the model from Hugging Face ([`ggerganov/whisper.cpp`](https://huggingface.co/ggerganov/whisper.cpp)); transcription starts once it has finished.

### Windows

1. Install the build dependencies:
   - [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/) with the **Desktop development with C++** workload (the MSVC compiler and Windows SDK)
   - [CMake](https://cmake.org/download/)
   - The [LunarG Vulkan SDK](https://vulkan.lunarg.com/sdk/home#windows). Its installer sets `VULKAN_SDK`, which the build needs; open a new terminal afterwards.

2. Point Cargo at a short target directory. MSBuild cannot handle paths over 260 characters, and whisper.cpp's build nests deep enough that a target directory longer than about 20 characters fails with `FTK1011` or `MSB8066`:

   ```powershell
   setx CARGO_TARGET_DIR C:\t
   ```

3. Clone and run, in a new terminal:

   ```bash
   git clone https://github.com/chiyiangel/visper.git
   ```

   ```bash
   cd visper && cargo run --release
   ```

Only `x86_64-pc-windows-msvc` is supported. Every push and pull request also builds `visper.exe` in [CI](.github/workflows/ci.yml); it is attached to the run as the `visper-windows-x86_64` artifact.

### Where data lives

Everything is in one folder: `~/Library/Application Support/Visper/` on macOS, `%LOCALAPPDATA%\Visper\` on Windows.

| File | What it is |
| --- | --- |
| `ggml-large-v3.bin` | The Whisper model (`.part` while downloading) |
| `library.json` | Your library: recording paths, status and transcripts |
| `instance.sock` | macOS only: keeps a second launch from opening a second window (Windows uses a named pipe instead) |

Your audio files are never copied. To reset Visper, quit it and delete that folder.

## Development

```bash
cargo test
```

```bash
cargo clippy --all-targets
```

Debug builds can open isolated preview windows that use sample data instead of your library and never download the model. Scenarios are `empty`, `transcribing`, `completed`, `failure`, `missing` and `long`:

```bash
cargo run -- --preview completed
```

After changing the SVGs in `assets/brand/`, regenerate the app icon with `scripts/make-app-icon.sh` (needs `brew install librsvg`).

### Project layout

| Path | Contents |
| --- | --- |
| `src/library.rs` | The library model: recordings, statuses, queue rules. No GUI dependency, fully unit-tested |
| `src/transcribe.rs` | Decoding, resampling, silence chunking and whisper.cpp inference |
| `src/download.rs` | Model download |
| `src/store.rs` | Saving and loading `library.json` |
| `src/playback.rs` | Audio playback |
| `src/ui.rs`, `src/ui/` | GPUI window, rendering and motion |
| `GLOSSARY.md` | Domain terms (in Chinese) used in code and UI |
| `docs/adr/` | Architecture decision records |
| `docs/design/` | Visual and interaction spec |

## Contributing

Issues and pull requests are welcome; please read [CONTRIBUTING.md](CONTRIBUTING.md) first. Issues may be written in English or Chinese.

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
