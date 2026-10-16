<p align="center">
  <img src="assets/brand/app-icon-512.png" width="112" alt="Visper 图标">
</p>

<h1 align="center">Visper</h1>

<p align="center">
  把你已有的录音，在自己的 Mac 上变成转写稿。
</p>

<p align="center">
  <a href="README.md">English</a> · 简体中文
</p>

---

Visper 是一个用 Rust 和 [GPUI](https://www.gpui.rs/) 写的原生桌面应用，通过 [whisper.cpp](https://github.com/ggml-org/whisper.cpp) 调用 OpenAI 的 Whisper **large-v3** 模型转写本地音频文件。不上传任何东西：模型、录音和转写稿都留在你的电脑上。

适合会议、课程、访谈这类**简体中文、英文或中英混杂**的录音。

> **状态：** 早期版本（v0.1）。支持 macOS 和 Windows。界面语言为简体中文。

## 功能

- **完全离线。** 首次下载模型之后，转写不需要网络。
- **中英混杂。** “自动”模式按静音把录音切开，每一段单独判断是中文还是英文，中途换语言不会把后面带偏。也可以对整份录音固定为“中文”或“英文”。见 [ADR 0002](docs/adr/0002-zh-en-per-chunk.md)。
- **只出简体中文。** 繁体字在显示和保存之前用 OpenCC 转成简体。
- **完整质量的模型。** 使用未量化的 `ggml-large-v3.bin`（约 3.1 GB），首次启动时由应用自动下载并显示进度。
- **优先用 GPU。** 推理走 Vulkan（macOS 上即 MoltenVK，Windows 上由显卡驱动提供），无法初始化 Vulkan 设备时退回 CPU。见 [ADR 0001](docs/adr/0001-whisper-cpp-vulkan.md)。
- **队列式转写。** 用文件选择器或直接拖进窗口加入录音，按顺序一次转写一份；转写过程中分段实时出现。
- **分段回听。** 点击任意分段，从那里开始播放录音。
- **导出 Markdown。** 每个分段一行 `[HH:MM:SS] 文字`。
- **资料库。** 录音和转写稿在重启后仍在。录音文件被移走时标记为“文件缺失”，转写稿照样可读。
- 浅色、深色外观跟随系统。

支持的音频格式：`m4a`、`mp3`、`wav`、`flac`、`aac`。

### 不做的事

Visper 有意不做：麦克风实时录音、翻译、摘要、说话人标注、编辑转写稿、自定义提示词或词表，以及中英文以外的语言。粤语按中文转写，不保证质量。

## 环境要求

- macOS（推荐 Apple Silicon；Intel 机型见下方说明），或 x86_64 的 Windows 10/11
- 约 3.1 GB 磁盘空间存放模型，以及足够加载它的内存
- Windows 上需要提供 Vulkan 的显卡驱动（`vulkan-1.dll`，新版 NVIDIA、AMD、Intel 驱动都带）和 [Microsoft Visual C++ 运行库](https://learn.microsoft.com/cpp/windows/latest-supported-vc-redist)。没有 `vulkan-1.dll` 时 Visper 无法启动。
- 构建需要：Rust stable（`rust-toolchain.toml` 会自动安装），以及下面各平台的工具

## 构建与运行

目前还没有预编译的发布包，需要从源码构建。### macOS

需要 Xcode Command Line Tools 和 [Homebrew](https://brew.sh)。

1. 安装构建依赖。whisper.cpp 由 CMake 编译，它的 Vulkan 后端需要 Vulkan loader、头文件、MoltenVK 和 `glslc`（来自 shaderc）：

   ```bash
   brew install cmake vulkan-loader vulkan-headers molten-vk shaderc
   ```

2. 克隆并运行：

   ```bash
   git clone https://github.com/chiyiangel/visper.git
   ```

   ```bash
   cd visper && cargo run --release
   ```

第一次构建要编译 whisper.cpp，需要几分钟。首次启动时 Visper 会从 Hugging Face（[`ggerganov/whisper.cpp`](https://huggingface.co/ggerganov/whisper.cpp)）下载模型，下载完成后才开始转写。

**Intel Mac 或 LunarG Vulkan SDK。** [`.cargo/config.toml`](.cargo/config.toml) 把 `VULKAN_SDK` 默认设为 Apple Silicon 上 Homebrew 的前缀 `/opt/homebrew`。自己设置这个环境变量即可覆盖，例如：

```bash
VULKAN_SDK=/usr/local cargo run --release
```

### Windows

1. 安装构建依赖：
   - [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/)，勾选 **使用 C++ 的桌面开发** 工作负载（MSVC 编译器和 Windows SDK）
   - [CMake](https://cmake.org/download/)
   - [LunarG Vulkan SDK](https://vulkan.lunarg.com/sdk/home#windows)。安装器会设置 `VULKAN_SDK`，覆盖 `.cargo/config.toml` 里给 macOS 的默认值；装完后请新开一个终端。

2. 给 Cargo 一个短的 target 目录。MSBuild 处理不了超过 260 个字符的路径，而 whisper.cpp 的构建目录嵌套很深，target 目录路径超过大约 20 个字符就会报 `FTK1011` 或 `MSB8066`：

   ```powershell
   setx CARGO_TARGET_DIR C:\t
   ```

3. 在新终端里克隆并运行：

   ```bash
   git clone https://github.com/chiyiangel/visper.git
   ```

   ```bash
   cd visper && cargo run --release
   ```

只支持 `x86_64-pc-windows-msvc`。每次 push 和 pull request 都会在 [CI](.github/workflows/ci.yml) 里构建 `visper.exe`，作为 `visper-windows-x86_64` artifact 附在那次运行上。

### 数据存放位置

全部在一个文件夹里：macOS 上是 `~/Library/Application Support/Visper/`，Windows 上是 `%LOCALAPPDATA%\Visper\`。

| 文件 | 内容 |
| --- | --- |
| `ggml-large-v3.bin` | Whisper 模型（下载中为 `.part`） |
| `library.json` | 资料库：录音路径、状态和转写稿 |
| `instance.sock` | 仅 macOS：防止重复启动时打开第二个窗口（Windows 上改用命名管道） |

录音文件本身不会被复制。想重置 Visper，退出应用后删除这个文件夹即可。

## 开发

```bash
cargo test
```

```bash
cargo clippy --all-targets
```

Debug 构建可以打开隔离的预览窗口：使用示例数据，不读写你的资料库，也不下载模型。可用场景有 `empty`、`transcribing`、`completed`、`failure`、`missing`、`long`：

```bash
cargo run -- --preview completed
```

修改 `assets/brand/` 下的 SVG 后，用 `scripts/make-app-icon.sh` 重新生成应用图标（需要 `brew install librsvg`）。

### 目录结构

| 路径 | 内容 |
| --- | --- |
| `src/library.rs` | 资料库模型：录音、状态、队列规则。不依赖 GUI，单元测试覆盖 |
| `src/transcribe.rs` | 解码、重采样、按静音切分和 whisper.cpp 推理 |
| `src/download.rs` | 模型下载 |
| `src/store.rs` | 读写 `library.json` |
| `src/playback.rs` | 音频播放 |
| `src/ui.rs`、`src/ui/` | GPUI 窗口、渲染和动效 |
| `GLOSSARY.md` | 代码和界面使用的领域术语 |
| `docs/adr/` | 架构决策记录 |
| `docs/design/` | 视觉与交互规范 |

## 参与贡献

欢迎提交 Issue 和 Pull Request，请先阅读 [CONTRIBUTING.md](CONTRIBUTING.md)。Issue 用中文或英文都可以。

## 许可证

本项目以下列任一许可证授权，由你选择：

- Apache License 2.0（[LICENSE-APACHE](LICENSE-APACHE)）
- MIT 许可证（[LICENSE-MIT](LICENSE-MIT)）

除非你另有明确声明，你有意提交给本项目的任何贡献（按 Apache-2.0 的定义）都将按上述方式双重授权，不附加其他条款或条件。

Whisper 模型权重不包含在本仓库中。它由 OpenAI 以 MIT 许可证发布，应用运行时从 Hugging Face 下载。

## 致谢

- [OpenAI Whisper](https://github.com/openai/whisper)：模型
- [whisper.cpp](https://github.com/ggml-org/whisper.cpp) 与 [whisper-rs](https://codeberg.org/tazz4843/whisper-rs)：推理
- Zed Industries 的 [GPUI](https://www.gpui.rs/)：界面
- [Symphonia](https://github.com/pdeljanov/Symphonia)、[Rubato](https://github.com/HEnquist/rubato)、[Rodio](https://github.com/RustAudio/rodio)：音频
- [ferrous-opencc](https://crates.io/crates/ferrous-opencc)：繁简转换
