<p align="center">
  <img src="assets/brand/app-icon-512.png" width="112" alt="Visper 图标">
</p>

<h1 align="center">Visper</h1>

<p align="center">
  <strong>把你已有的录音，在自己的电脑上离线转成文字。</strong><br>
  中文、英文，或同一份录音里中英混杂都行。Whisper large-v3 跑在你自己的 GPU 上。
</p>

<p align="center">
  <a href="README.md">English</a> · 简体中文 · <a href="https://visper-site.vercel.app">项目主页</a>
</p>

<p align="center">
  <img alt="平台：macOS（Apple Silicon）和 Windows x86_64" src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows-426650">
  <img alt="Rust + GPUI" src="https://img.shields.io/badge/built%20with-Rust%20%2B%20GPUI-426650">
  <img alt="许可：MIT OR Apache-2.0" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-426650">
</p>

---

Visper 是一个原生桌面应用，把会议、课程、访谈这类音频文件转成带时间戳的转写稿。它通过 [whisper.cpp](https://github.com/ggml-org/whisper.cpp) 在本机运行 OpenAI 的 Whisper **large-v3** 或 **large-v3-turbo** 模型，macOS 上走 Metal，Windows 上走 Vulkan。不上传任何东西：模型、录音和转写稿都留在你的电脑上。

它用 Rust 编写，界面基于 Zed 编辑器背后的 UI 框架 [GPUI](https://www.gpui.rs/)。

> [!NOTE]
> **状态：早期版本（v0.1）。** 支持 macOS（Apple Silicon）和 Windows 10/11（x86_64），界面语言为简体中文。
> 预编译安装包**即将提供**，目前请[从源码构建](#从源码构建)。

## 目录

- [功能](#功能)
- [界面预览](#界面预览)
- [工作原理](#工作原理)
- [运行与构建要求](#运行与构建要求)
- [从源码构建](#从源码构建)
- [使用方法](#使用方法)
- [设置与数据](#设置与数据)
- [开发](#开发)
- [路线图](#路线图)
- [参与贡献](#参与贡献)
- [许可](#许可)

## 功能

- **完全离线。** 首次下载模型之后，转写完全不需要网络。Visper 唯一的联网行为，是通过 HTTPS 从 Hugging Face 下载模型权重。
- **中英混杂。** “自动”模式按静音把录音切成若干段，每一段单独判断是中文还是英文，中途换语言不会把后面整段带偏。也可以把一份录音固定为“中文”或“英文”。见 [ADR 0002](docs/adr/0002-zh-en-per-chunk.md)。
- **只输出简体中文。** 繁体字在显示和保存之前用 OpenCC 转成简体。
- **两个模型，自己选。**
  | 模型 | 权重文件 | 大小 | 适合 |
  | --- | --- | --- | --- |
  | Large v3 | `ggml-large-v3.bin` | 约 3.1 GB | 准确度最高，适合专有名词多的访谈 |
  | Large v3 Turbo | `ggml-large-v3-turbo.bin` | 约 1.6 GB | 快得多，准确度略低，适合日常会议 |

  两者都是未量化的完整权重。首次启动时选一个下载，之后随时可以在资料库底部切换；每份转写稿都会记下产生它的模型。见 [ADR 0004](docs/adr/0004-large-v3-or-turbo.md)。
- **GPU 加速，带兜底。** macOS 用 Metal，Windows 用 Vulkan，GPU 路径开启 flash attention；GPU 初始化失败时自动退回 CPU。见 [ADR 0003](docs/adr/0003-metal-on-macos.md) 和 [Vulkan 与 Metal 对比测试](docs/whisper-vulkan-vs-metal-bench.md)。
- **队列式转写。** 把录音拖进窗口，或用 ⌘O / Ctrl+O 选择文件，按顺序一次转写一份。分段实时出现，同时显示进度、速度和预计剩余时间。
- **分段回听。** 点击任意分段，从那里开始播放录音。
- **导出 Markdown。** 每个分段一块 `[HH:MM:SS] 文字`，以 `<录音文件名>.md` 保存到你选的文件夹。
- **资料库。** 录音和转写稿在重启后仍在。可以重新转写已完成的录音、取消正在进行的转写，或把录音从资料库移除。录音文件被移走时标记为“文件缺失”，转写稿照样可读、可导出。Visper 从不复制你的音频文件。
- **地道的桌面应用。** macOS 上有原生菜单栏和标准快捷键、访达“打开方式”、拖到程序坞图标加入、“关于”面板，窗口下次打开仍在原处。Windows 上有跟随主题的窗口按钮，每个用户只运行一个实例。两个平台都跟随系统的浅色/深色外观；正在转写时退出会先确认，因为退出会取消这次转写。

支持的音频格式：`m4a`、`mp3`、`wav`、`flac`、`aac`。

### 不做的事

Visper 有意保持小而专注。它**不做**：麦克风实时录音、翻译、摘要、说话人标注、编辑转写稿、自定义提示词或词表、中英文以外的语言。粤语会按中文路径转写，不保证质量。

## 界面预览

仓库里暂时还没有截图。[设计画布](design-canvas/)用 GPUI 代码里的文案和尺寸，以实时 HTML 重绘了应用的每一个界面（首次启动、模型下载、队列、所有状态、播放、导出，浅色和深色），可以在线浏览：**[visper-chi.vercel.app](https://visper-chi.vercel.app)**。

<!-- TODO: 在 docs/screenshots/ 下补充真实截图（浅色、深色各一）并在这里展示。 -->

## 工作原理

```text
 音频文件 ──► 解码 ──► 重采样 ──► 按静音切段 ──► 判断语言 ──► whisper.cpp ──► 转简体 ──► 资料库
 m4a/mp3/wav  Symphonia  16 kHz 单声道  每段 ≤ 30 秒   每段 zh 或 en   beam search 5   OpenCC t2s  library.json
 flac/aac                Rubato     （20 ms RMS 帧）  （自动模式）    Metal / Vulkan              + 实时界面
                                                                   / CPU 兜底
```

1. **解码与重采样。** [Symphonia](https://github.com/pdeljanov/Symphonia) 解码文件，[Rubato](https://github.com/HEnquist/rubato) 重采样成 Whisper 需要的 16 kHz 单声道。
2. **按静音切段。** 以 20 ms 为一帧测响度，阈值随底噪自适应。400 ms 以上的停顿把语音分成若干段；超过一个 30 秒 Whisper 窗口的段，在段内最长的换气处切开。几乎没有人声的段直接跳过。
3. **逐段判断语言。** 自动模式下，每一段先跑一次 Whisper 的语言检测，只判为中文或英文（粤语计入中文）。相邻的同语言段会合并进同一个窗口，长录音不必每句话都单独跑一遍。
4. **转写。** 每个窗口交给 whisper.cpp（通过 [whisper-rs](https://codeberg.org/tazz4843/whisper-rs)）解码：beam search、不翻译、不沿用上一窗口的上下文。GPU 上下文只创建一次，后续录音复用；创建失败时，这一次转写改用 CPU。
5. **保存。** 分段转成简体后实时推到窗口，并写入 `library.json`。失败或取消的转写不保留半成品。

### 代码结构

| 路径 | 内容 |
| --- | --- |
| `src/library.rs` | 领域模型：录音、状态、队列、模型选择、导出。不依赖 GUI，完整单元测试 |
| `src/transcribe.rs` | 解码、重采样、静音切段、语言检测，以及在工作线程上的 whisper.cpp 推理 |
| `src/download.rs` | 模型下载：写入 `.part` 文件，带进度、重试和卡死检测 |
| `src/store.rs` | `library.json` 的原子保存与读取 |
| `src/playback.rs` | 用 [Rodio](https://github.com/RustAudio/rodio) 播放音频 |
| `src/ui.rs`、`src/ui/` | GPUI 窗口、渲染、动效、滚动条和调试预览 |
| `src/menu.rs` | 菜单栏和快捷键 |
| `src/instance.rs` | 每个用户只运行一个 Visper：macOS 用 Unix socket，Windows 用命名管道 |
| `src/macos.rs` | GPUI 没有提供的 AppKit 调用：关于面板、窗口和帮助菜单、窗口位置记忆、程序坞图标 |
| `packaging/macos/`、`scripts/` | `Info.plist`、`Visper.app` / `.dmg` 打包脚本、图标生成脚本 |
| `design-canvas/` | 重绘所有界面、用于设计评审的静态网站 |
| `docs/adr/` | 架构决策记录 |
| `docs/design/` | 视觉与交互规范 |
| `GLOSSARY.md` | 代码和界面里使用的领域术语 |

## 运行与构建要求

**运行**

- Apple Silicon 的 macOS，或 x86_64 的 Windows 10/11
- Large v3 需要约 3.1 GB 磁盘空间（Turbo 约 1.6 GB），以及足够加载模型的内存
- Windows 上需要提供 Vulkan 的显卡驱动（`vulkan-1.dll`，目前的 NVIDIA、AMD、Intel 驱动都带）和 [Microsoft Visual C++ 运行库](https://learn.microsoft.com/cpp/windows/latest-supported-vc-redist)。没有 `vulkan-1.dll` 时 Visper 无法启动。

**构建**

- Rust stable。[`rust-toolchain.toml`](rust-toolchain.toml) 固定了 stable 通道以及 `rustfmt`、`clippy`，`rustup` 会自动安装。
- CMake（用来编译 whisper.cpp），以及下面各平台的工具。

暂不支持 Linux。

## 从源码构建

### macOS

需要 Xcode Command Line Tools 和 [Homebrew](https://brew.sh)。

```bash
brew install cmake
git clone https://github.com/chiyiangel/visper.git
cd visper
cargo run --release
```

第一次构建会用 Metal 编译 whisper.cpp，需要几分钟。

#### 打包 `Visper.app` 和磁盘映像

`cargo run` 启动的是一个裸可执行文件。想要一个能放进“应用程序”、带图标、支持访达“打开方式”的应用：

```bash
scripts/bundle-macos.sh --dmg
```

它会在 `target/release/bundle/` 下生成 `Visper.app` 和 `Visper-<版本>-arm64.dmg`。默认是 ad hoc 签名，只能在构建它的那台 Mac 上直接打开；在别的 Mac 上需要先到“系统设置 → 隐私与安全性”里允许。要签名并公证后分发，设置：

| 变量 | 值 |
| --- | --- |
| `VISPER_SIGN_IDENTITY` | 钥匙串里的 *Developer ID Application* 证书，使用 hardened runtime 签名 |
| `VISPER_NOTARY_PROFILE` | 用 `xcrun notarytool store-credentials` 保存的配置，对磁盘映像公证并装订 |

### Windows（x86_64）

1. 安装构建依赖：
   - [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/)，勾选 **使用 C++ 的桌面开发**（MSVC 和 Windows SDK）
   - [CMake](https://cmake.org/download/)
   - [LunarG Vulkan SDK](https://vulkan.lunarg.com/sdk/home#windows)。安装程序会设置构建需要的 `VULKAN_SDK`，装完请新开一个终端。
2. 使用较短的 Cargo 目标目录。MSBuild 处理不了超过 260 个字符的路径，whisper.cpp 的构建目录嵌套很深，目标目录超过约 20 个字符就会报 `FTK1011` 或 `MSB8066`：

   ```powershell
   setx CARGO_TARGET_DIR C:\t
   ```

3. 在新终端里：

   ```powershell
   git clone https://github.com/chiyiangel/visper.git
   cd visper
   cargo run --release
   ```

只支持 `x86_64-pc-windows-msvc`。Release 构建是 GUI 子系统程序，不会带出控制台窗口。

## 使用方法

1. **选择模型。** 首次启动时，Visper 会让你选择下载 Large v3 还是 Large v3 Turbo，并显示下载进度。模型下载完成后才能加入录音。
2. **加入录音。** 把文件拖进窗口，点“选择录音”/“加入录音”，或按 ⌘O（Windows 上 Ctrl+O）。macOS 上也可以用访达的“打开方式”，或把文件拖到程序坞图标上。
3. **选择语言（可选）。** 等待中的录音可以选“自动”（默认）、“中文”或“英文”。
4. **查看转写。** 分段边识别边出现，同时显示进度、速度和剩余时间。其他录音按顺序等待；中途退出的话，等待中的录音会在下次打开时继续。
5. **阅读、回听、导出。** 点分段从那里播放，用 ⌘C / Ctrl+C 拷贝文字，导出为 Markdown。点“重新转写”可以把已完成的录音再转一遍（比如切换模型之后），在“⋯”菜单里可以把录音从资料库移除。

导出文件的格式如下：

```markdown
[00:00:04] 我们先看一下这次产品讨论的目标。

[00:00:18] 界面需要更清晰，让主要操作自然出现。

[00:00:36] We should keep the workflow simple and predictable.
```

切换模型只影响之后开始的转写，已有转写稿保持原来的模型。

## 设置与数据

Visper 没有配置文件。它记住的只有两样：当前模型（在应用里选）和每份录音的语言。其他数据都在一个文件夹里：

| 平台 | 文件夹 |
| --- | --- |
| macOS | `~/Library/Application Support/Visper/` |
| Windows | `%LOCALAPPDATA%\Visper\`（本地而非漫游目录，几 GB 的权重不会跟着域账户同步） |

| 文件 | 内容 |
| --- | --- |
| `ggml-large-v3.bin`、`ggml-large-v3-turbo.bin` | 下载的模型权重（下载中为 `.part`） |
| `library.json` | 资料库：录音路径、状态和转写稿 |
| `instance.sock` | 仅 macOS：防止第二次启动再开一个窗口（Windows 用命名管道） |

权重从 Hugging Face 的 [`ggerganov/whisper.cpp`](https://huggingface.co/ggerganov/whisper.cpp) 下载。想重置 Visper，退出后删除这个文件夹即可。macOS 上的窗口位置另存在用户偏好设置里（`io.github.chiyiangel.visper`，用 `cargo run` 运行时为 `visper`）。

## 开发

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
```

CI 在每次推送和 PR 时都会在 Windows 和 macOS 上跑同样的检查。`src/library.rs` 里的领域模型不依赖 GUI，可以在任何系统上单独检查：

```bash
cargo check --lib --no-default-features
```

### 预览场景

Debug 构建可以打开隔离的预览窗口，使用示例数据，不读你的资料库，也不下载模型：

```bash
cargo run -- --preview completed
```

场景有：`empty`、`transcribing`、`completed`、`failure`、`missing`、`long`、`model-popover`、`model-download`、`model-download-failed`、`first-launch`、`first-launch-download`、`first-launch-failed`。加上 `-dark` 后缀即为深色模式，例如 `completed-dark`。

### 设计画布

[`design-canvas/`](design-canvas/) 是一个 Vite 网站，按 `src/ui/` 里的文案和尺寸重绘所有界面。点击任意控件会复制一条引用（`frame`、元素 id、`文件:行号`），可以直接贴进评审意见。详见它的 [README](design-canvas/README.md)。

### 应用图标

修改 `assets/brand/` 里的 SVG 之后，用 `scripts/make-app-icon.sh` 重新生成图标（需要 `brew install librsvg`）。

## 路线图

- **预编译安装包**（即将提供）：在 GitHub Releases 上发布 Apple Silicon 的 `.dmg` 和 Windows x86_64 的 `.zip`。[发布工作流](.github/workflows/release.yml)已经可以从 `v*` 标签构建它们。
- **签名并公证的 macOS 版本。** 打包脚本已支持 Developer ID 签名和公证，目前发布的是 ad hoc 签名版本。

功能范围会继续保持克制（见[不做的事](#不做的事)）。欢迎在 [Issues](https://github.com/chiyiangel/visper/issues) 里提想法和问题。

## 参与贡献

欢迎提 Issue 和 PR，中英文均可。请先阅读 [CONTRIBUTING.md](CONTRIBUTING.md)：里面说明了功能范围、[GLOSSARY.md](GLOSSARY.md) 里的领域术语、[视觉规范](docs/design/visual-spec-v1.md)，以及 PR 必须通过的检查。用户能感知到的改动请记入 [CHANGELOG.md](CHANGELOG.md)。

安全问题请按 [SECURITY.md](SECURITY.md) 私下报告，不要开公开 Issue。

## 许可

本项目可任选以下任一许可证使用：

- Apache License 2.0（[LICENSE-APACHE](LICENSE-APACHE)）
- MIT 许可证（[LICENSE-MIT](LICENSE-MIT)）

除非你另有明确声明，你有意提交并被本项目收录的任何贡献（按 Apache-2.0 许可证的定义），都按上述双许可证授权，不附加任何其他条款或条件。

Whisper 模型权重不包含在本仓库中。它们由 OpenAI 以 MIT 许可证发布，在运行时从 Hugging Face 下载。

## 致谢

- [OpenAI Whisper](https://github.com/openai/whisper)：模型
- [whisper.cpp](https://github.com/ggml-org/whisper.cpp) 与 [whisper-rs](https://codeberg.org/tazz4843/whisper-rs)：推理
- Zed Industries 的 [GPUI](https://www.gpui.rs/)：界面
- [Symphonia](https://github.com/pdeljanov/Symphonia)、[Rubato](https://github.com/HEnquist/rubato)、[Rodio](https://github.com/RustAudio/rodio)：音频
- [ferrous-opencc](https://crates.io/crates/ferrous-opencc)：繁简转换
