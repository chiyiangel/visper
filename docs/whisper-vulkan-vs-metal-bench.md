# whisper.cpp on macOS: Vulkan (MoltenVK) vs Metal

Measured 2026-10-16 to size the cost of [ADR 0001](adr/0001-whisper-cpp-vulkan.md) on Apple silicon.

## Setup

- **Machine:** Apple M3 Pro (12 cores, 18 GB), macOS 15.7.9, Apple clang 17.
- **whisper.cpp:** v1.8.3, commit `2eeeba56e9edd762b4b38467bab96c2517163158`, ggml 0.9.5. This is the copy vendored by whisper-rs-sys 0.15.0 (pulled in by whisper-rs 0.16); its `src/`, `include/` and `ggml/src/` are byte-identical to the upstream tag. `whisper-cli` and `whisper-bench` come from the upstream tag, because the crate does not ship `examples/`.
- **Model:** `~/Library/Application Support/Visper/ggml-large-v3.bin` (f16, 3.1 GB).
- **Common CMake flags:** `-DCMAKE_BUILD_TYPE=Release -DGGML_BLAS=ON -DGGML_OPENMP=OFF`. These match whisper-rs-sys on macOS, which leaves the Apple default (BLAS through Accelerate) on and turns OpenMP off.
  - **Vulkan:** `-DGGML_VULKAN=ON -DGGML_METAL=OFF`, `VULKAN_SDK=/opt/homebrew` (molten-vk 1.4.2, vulkan-loader 1.4.363, shaderc 2026.4).
  - **Metal:** `-DGGML_METAL=ON -DGGML_METAL_NDEBUG=ON -DGGML_METAL_EMBED_LIBRARY=ON -DGGML_VULKAN=OFF` (the flags whisper-rs-sys's `metal` feature sets).
- **Vulkan device report:** `Apple M3 Pro (MoltenVK) | uma: 1 | fp16: 1 | bf16: 0 | int dot: 0 | matrix cores: none`. glslc supports `GL_KHR_cooperative_matrix`, but MoltenVK exposes no cooperative-matrix extension, so the Vulkan backend runs only its scalar/fp16 shaders.
- **Flash attention:** whisper-rs 0.16's `WhisperContextParameters::default()` sets `flash_attn: false`, and Visper does not override it. whisper-cli and whisper-bench 1.8.3 default to `true`. **FA off** is what Visper runs today; FA on is shown as the alternative.
- **End-to-end settings:** these mirror `transcribe_window`: `whisper-cli -t 8 -l en -bs 5 -mc 0 -sns`, with 8 threads = `min(cores, 8)`. The clip is 213.6 s of English TTS (`say -v Samantha` reading README.md), 16 kHz mono, giving about 8 thirty-second windows.

## Results

`whisper-bench -w 0` gives mean of 3 runs. End-to-end gives mean of 2 runs for the FA-off GPU rows and 1 run for the others. Repeat runs varied by less than 3 %.

| Backend | FA | Encoder (bench) | Decode/token (bench, batchd) | Wall time, end-to-end | of which load | Inference (wall − load) | Encode per window (cli) | batchd total (cli) | Peak memory footprint | Peak RSS |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| **Vulkan (MoltenVK)** — *Visper today* | off | 879 ms | 33.0 ms (9.9 ms) | **80.4 s** | 25.6 s | **54.8 s** | 932 ms | 41.8 s (11.0 ms × 3810) | 4.85 GB | 0.32 GB ¹ |
| Vulkan (MoltenVK) | on | 1150 ms | 27.7 ms (7.7 ms) | 75.5 s | 25.7 s | 49.8 s | 1163 ms | 33.1 s (8.0 ms × 4150) | 4.66 GB | 0.30 GB ¹ |
| **Metal** | off | 785 ms | 22.7 ms (6.0 ms) | **66.7 s** | 24.9 s | **41.7 s** | 829 ms | 28.5 s (6.8 ms × 4198) | 4.17 GB | 4.03 GB |
| Metal | on | **619 ms** | 20.3 ms (4.6 ms) | **55.6 s** | 24.7 s | **30.9 s** | 625 ms | 20.0 s (4.75 ms × 4203) | 4.18 GB | 4.06 GB |
| CPU only (`-ng`, Metal build) | off | — | — | 1825.7 s | 1.4 s | 1824 s | 140 s | 557 s (133 ms × 4199) | 4.39 GB | 3.41 GB |

Throughput relative to the clip's 213.6 s of audio, inference only: Vulkan FA-off runs at 3.9× real time, Metal FA-on at 6.9×, and CPU at 0.12×.

¹ MoltenVK's device allocations are not counted in RSS, so `peak memory footprint` (from `/usr/bin/time -l`) is the comparable number.

### Other observations

- **Vulkan with FA off loses words.** Across two runs it deterministically gave 505 words. Metal (FA on or off), CPU and Vulkan FA-on all gave the same 525-word transcript, byte for byte. Vulkan FA-off dropped the opening "Turn the recordings", "or languages" and "there are no pre-built", and it emitted unpunctuated runs. That is also why its cli row shows fewer encode and batchd runs. The test is one TTS clip, but the result points to a numerical problem in the non-FA attention path on MoltenVK, and that is the path Visper uses today.
- **Load time is about 25 s on both GPU backends, but 1.4 s on CPU.** The cost is uploading the f16 weights to the GPU, not disk I/O, and it does not depend on the backend.
- **Visper does more encoder work than whisper-cli.** It runs `lang_detect` on every window, which costs one extra encoder pass per window. On Visper's current path that adds about 0.9 s per window, and with Metal FA it would be about 0.6 s. This widens the gap a little beyond the table.
- **Flash attention helps Metal and hurts the Vulkan encoder.** Metal's encoder drops 785 → 619 ms with FA on. On MoltenVK the encoder rises 879 → 1150 ms, though decoding still gets faster.

## Conclusion

On the M3 Pro, Visper's current configuration (Vulkan through MoltenVK, flash attention off) is about **1.3× slower than native Metal with the same settings**, at 54.8 s vs 41.7 s of inference for 3.6 min of audio. It is **1.8× slower than Metal with flash attention**, at 30.9 s. Most of the gap is in decoding (batchd 11.0 vs 4.75 ms per step), because MoltenVK exposes no cooperative matrix and runs the generic shaders. The encoder gap is smaller, at 879 vs 619 ms. Vulkan also uses about 0.7 GB more memory, and with FA off it produced a measurably worse transcript, so enabling ggml's Metal backend on macOS (with `flash_attn = true`) looks worth a new ADR. Separately, turning on `flash_attn` alone already removes the word drops on the Vulkan path and is about 9 % faster there.
