# Inference is whisper.cpp on Vulkan

Visper transcribes local recordings with Whisper large-v3. The first release is macOS only, but the GPU path has to be the same one a later Windows or Linux build can reuse. Inference goes through whisper.cpp via whisper-rs. The only GPU backend is Vulkan, which on macOS means MoltenVK. If Vulkan device setup fails, that transcription runs on the CPU. v1 does not add a Metal or Core ML backend. The app downloads the full `ggml-large-v3.bin` from Hugging Face (`ggerganov/whisper.cpp`) by itself and shows download progress. That file is about 3GB.

Metal would be faster on a Mac, and a quantized or turbo weight would be smaller. Both were rejected: Metal would be a second compute backend, and the smaller weights give up the full large-v3 quality this app is for. A pure-Rust runtime was rejected because it does not have a real Vulkan Whisper path.

On macOS this is superseded by [ADR 0003](0003-metal-on-macos.md): inference there runs on Metal. Its rejection of the turbo weight is superseded by [ADR 0004](0004-large-v3-or-turbo.md): the user can pick large-v3-turbo.
