# Whisper large-v3 用法

官方名称是 Whisper，模型 id 是 `large-v3`（也写作 Whisper large-v3）。“Wisper”不是这个模型的名字。它是 OpenAI 的编码器–解码器语音模型，large 系列参数量是 1550 M，做多语言语音识别，也可以把语音翻译成英文。[README 的模型表](https://github.com/openai/whisper#available-models-and-languages)把 1550 M 的多语言模型标成 `large`；[当前 `whisper/__init__.py`](https://github.com/openai/whisper/blob/main/whisper/__init__.py) 里 `large` 和 `large-v3` 指向同一个权重文件。

这里的 prompt 是解码器里的上文，用来偏向拼写、标点和用词。它不是会执行指令的对话系统提示。OpenAI 的语音转写指南写的是：“Whisper doesn't follow instructions like a general-purpose text model”。见 [Improving reliability](https://platform.openai.com/docs/guides/speech-to-text#improving-reliability)。

## 模型身份

`large`、`large-v2`、`large-v3` 在 [Hugging Face 模型卡](https://huggingface.co/openai/whisper-large-v3) 的表里都是 1550 M、仅多语言。large-v3 和前两个 large 的架构相同，模型卡和 [2023-11-06 的发布说明](https://github.com/openai/whisper/discussions/1762) 只列出两处差别：频谱用 128 个 Mel 频带，不再是 80；增加了粤语语言 token。发布说明还写明，`large-v3` 从 `openai-whisper==20231106` 起可用，名字 `"large"` 从此指向 `"large-v3"`。仓库里的 [model card](https://github.com/openai/whisper/blob/main/model-card.md) 把发布日期写成：原系列 2022 年 9 月，`large-v2` 2022 年 12 月，`large-v3` 2023 年 11 月，`large-v3-turbo` 2024 年 9 月。

large-v3 的训练数据，模型卡和上述发布说明写的是同一句：100 万小时弱标注音频，加上用 large-v2 打出的 400 万小时伪标注音频，在这个混合集上训练 2.0 个 epoch。发布说明和模型卡都写，相对 large-v2，多种语言的错误降低了 10% 到 20%。仓库 model card 里 68 万小时、98 种非英语语言的数字，描述的是论文那一批模型，不要当成 large-v3 这次训练的规模。

[已发布的 `config.json`](https://huggingface.co/openai/whisper-large-v3/raw/main/config.json) 里 `num_mel_bins` 是 128，`decoder_layers` 和 `encoder_layers` 都是 32，`max_target_positions` 是 448，`vocab_size` 是 51866。

粤语代码在当前 [tokenizer.py](https://github.com/openai/whisper/blob/main/whisper/tokenizer.py) 里是 `yue`（`cantonese`），它是 `LANGUAGES` 的第 100 个代码。语言数量三处不一致，不要合成一个数字：论文写训练集里有 99 个语言 token（[第 2.3 节](https://arxiv.org/abs/2212.04356)）；[语音转写指南](https://platform.openai.com/docs/guides/speech-to-text#supported-languages) 写 Whisper 支持 98 种语言；`model.py` 用 `n_vocab - 51765 - 1` 计算 `num_languages`，按上面的 51866 得到 100，`transcribe()` 会把这个数传给 tokenizer，因此这个 checkpoint 会用到 `yue`。

`large-v3-turbo` 不是完整的 large-v3。[OpenAI README](https://github.com/openai/whisper#available-models-and-languages) 写它是 large-v3 的加速版，转写更快，精度损失很小，表里是 809 M、约 6 GB 显存、相对 large 约 8 倍速度；同一页写明它没有为翻译训练，指定 `--task translate` 仍会返回源语言。 [Hugging Face 的 turbo 模型卡](https://huggingface.co/openai/whisper-large-v3-turbo) 写它是剪过解码层再微调的 large-v3，解码层从 32 减到 4，更快，质量略降；[它的 config](https://huggingface.co/openai/whisper-large-v3-turbo/raw/main/config.json) 里 `decoder_layers` 是 4，`encoder_layers` 仍是 32，`num_mel_bins` 仍是 128。参数量有冲突：README 和 turbo 模型卡的表是 809 M，仓库 [model card](https://github.com/openai/whisper/blob/main/model-card.md) 的表把 `turbo` 写成 798 M。

许可证也有冲突。 [OpenAI README](https://github.com/openai/whisper#license) 写代码和权重是 MIT，[LICENSE](https://github.com/openai/whisper/blob/main/LICENSE) 文件是 MIT。Hugging Face 模型卡文首的 `license` 字段是 `apache-2.0`。两处都没有解释对方。

README 里 `large` 这一行的显存大约 10 GB，相对速度 1 倍，测的是 A100 上的英语转写，实际速度会随语言、语速和硬件变化。

## 能力边界

它把语音转成文字。翻译任务的目标是英文：`DecodingOptions` 的注释写的是 X→X 的 `transcribe`，或 X→英文的 `translate`（[decoding.py](https://github.com/openai/whisper/blob/main/whisper/decoding.py)）。CLI 的 `--task` 帮助文字是同样的划分（[transcribe.py](https://github.com/openai/whisper/blob/main/whisper/transcribe.py)）。[翻译 API](https://platform.openai.com/docs/guides/speech-to-text#translations) 写 “This endpoint supports translation into English only.” 模型卡写，识别时输出与音频相同的语言，翻译时输出与音频不同的语言；被评估的翻译任务是译成英文。

prompt 不能用来下命令。指南里那句是：“Whisper doesn't follow instructions like a general-purpose text model and accepts prompts of up to 224 tokens.” 因此 “总结这段话”“输出 JSON”“忽略噪声”“只输出简体中文” 这类话不会被当成要执行的指令。要改拼写，应给出已经写对的用词，见下一节。指南把真正的指令放在转写之后的文本模型里，标题是 “Post-processing with a text model”。

幻觉是模型卡和仓库 model card 都写了的行为：弱监督的噪声数据会让预测里出现音频中没有说过的文字。模型卡的解释是，模型既在猜下一个词，也在转写。同一节写，序列到序列的结构容易生成重复文本，beam search 和 temperature 调度能减轻，但不能消掉。论文 [第 4.5 节](https://arxiv.org/abs/2212.04356) 把长音频的失败写成重复循环、漏掉窗口开头或结尾的词，以及整段与音频无关的幻觉，并给出解码上的缓解办法，见「质量与幻觉」。静音由 `<|nospeech|>` 和 `no_speech_threshold` 处理。音乐上的幻觉，README、模型卡和这份指南都没有单独写成一条规格；发布讨论里的用户回复不能当成规格。

## 三种调用方式

### openai-whisper

README 的命令行示例用的是 `turbo`。要指定 large-v3，模型名在 `available_models()` 里，CLI 的 `--model` 接受这个名字：

```bash
whisper audio.flac --model large-v3
```

指定语言、或把非英语译成英语（README 的写法，模型换成 large-v3）：

```bash
whisper audio.flac --model large-v3 --language Chinese --task translate
```

Python 侧，发布说明的加载方式是：

```python
import whisper

model = whisper.load_model("large-v3")
result = model.transcribe("audio.mp3")
print(result["text"])
```

README 写，`transcribe()` 读完整文件，用滑动的 30 秒窗口做自回归解码。`transcribe()` 文档里实际列出的、会改变结果的参数包括 `language`、`task`、`temperature`、`compression_ratio_threshold`、`logprob_threshold`、`no_speech_threshold`、`condition_on_previous_text`、`initial_prompt`、`carry_initial_prompt`、`word_timestamps`、`fp16`。未指定 `language` 时，源码会打印 “Detecting language using up to the first 30 seconds.”

### Hugging Face Transformers

模型卡要求用 `openai/whisper-large-v3` 的 processor。这个 checkpoint 的 `num_mel_bins` 是 128，不能沿用 large / large-v2 的 80 维 Mel。模型卡没有单独的警告框，差别写在文首。长音频的感受野是 30 秒。

```python
import torch
from transformers import AutoModelForSpeechSeq2Seq, AutoProcessor, pipeline

device = "cuda:0" if torch.cuda.is_available() else "cpu"
torch_dtype = torch.float16 if torch.cuda.is_available() else torch.float32
model_id = "openai/whisper-large-v3"
model = AutoModelForSpeechSeq2Seq.from_pretrained(
    model_id, torch_dtype=torch_dtype, low_cpu_mem_usage=True, use_safetensors=True
)
model.to(device)
processor = AutoProcessor.from_pretrained(model_id)
pipe = pipeline(
    "automatic-speech-recognition",
    model=model,
    tokenizer=processor.tokenizer,
    feature_extractor=processor.feature_extractor,
    torch_dtype=torch_dtype,
    device=device,
)
result = pipe("audio.mp3", generate_kwargs={"language": "chinese", "task": "transcribe"})
```

模型卡写：默认识别，源语言和文字语言相同；要译成英文，把 `task` 设为 `"translate"`。句子时间戳是 `return_timestamps=True`，词时间戳是 `return_timestamps="word"`。模型卡示例里的温度回退是 `(0.0, 0.2, 0.4, 0.6, 0.8, 1.0)`，`logprob_threshold` 是 `-1.0`，`no_speech_threshold` 是 `0.6`。同一示例把 `compression_ratio_threshold` 设成 `1.35`，注释写这是 token 空间里的 zlib 压缩比。这不是 openai-whisper 的默认 2.4，见下一节。

### whisper.cpp

本仓库走的是这条路径。官方示例是 `whisper-cli`，权重文件名在 [models/README.md](https://github.com/ggml-org/whisper.cpp/blob/master/models/README.md) 里是 `ggml-large-v3.bin`（表中磁盘占用 2.9 GiB），预转换文件在 `https://huggingface.co/ggerganov/whisper.cpp/tree/main`。README 用 `make -j large-v3` 下载并跑这个模型。

```bash
./build/bin/whisper-cli -m models/ggml-large-v3.bin -f audio.wav -l zh
```

对应开关：`-l` / `--language`（`auto` 表示自动检测），`-tr` / `--translate`（帮助文字是 “translate from source language to english”），`--prompt`（帮助文字是 “initial prompt (max n_text_ctx/2 tokens)”），`--carry-initial-prompt`，`-bs` / `--beam-size`，`-bo` / `--best-of`，`-tp` / `--temperature`，`-tpi` / `--temperature-inc`，`-lpt` / `--logprob-thold`，`-nth` / `--no-speech-thold`，`-nt` / `--no-timestamps`。词级时间戳在 README 的 “Word-level timestamp (experimental)” 里用 `-ml 1`。这些字符串来自 [examples/cli/cli.cpp](https://github.com/ggml-org/whisper.cpp/blob/master/examples/cli/cli.cpp) 的帮助文本和 README。

> **托管 API 的 `whisper-1` 不是 self-hosted 的 large-v3。** [Create transcription 的模型字段](https://platform.openai.com/docs/api-reference/audio/createTranscription) 写的是：`whisper-1`（which is powered by our open source Whisper V2 model）。这句没有把它写成 `large-v3`，也没有使用仓库里的 id `large-v2`。同一份当前的[语音转写指南](https://platform.openai.com/docs/guides/speech-to-text)把新的通用转写推荐为 `gpt-transcribe`，并把 `whisper-1` 留给出词或分段时间戳、字幕和译成英文。在本机加载 `large-v3` 或 `ggml-large-v3.bin` 用的是 2023-11 的权重，不是这个托管 id。OpenAPI 的 `model` 枚举里还有 `gpt-4o-transcribe` 等，没有 `gpt-transcribe`，和指南首页不一致；以你要调用的那一页为准。

## 关键参数

下面每个默认值都来自读过的源码或帮助文本。Hugging Face 示例里的 `1.35` 只属于那个示例。

- `language`（CLI `--language`，whisper.cpp `-l`）。openai-whisper 接受 [tokenizer.py](https://github.com/openai/whisper/blob/main/whisper/tokenizer.py) 里的代码或英文名，例如 `zh` / `chinese`，粤语是 `yue` / `cantonese`；不指定时只用开头最多 30 秒做一次检测，并固定到整次 `transcribe()`。whisper.cpp 的帮助文字是 spoken language，`auto` 为自动检测；CLI 结构体的初值是 `"en"`。
- `task`（`transcribe` 或 `translate`；whisper.cpp 用 `-tr` 打开翻译）。识别保持源语言，翻译输出英文。不要把 “请翻译成英文” 写进 prompt。
- `initial_prompt`（API 和 whisper.cpp 都叫 `prompt`）。openai-whisper 的文档写：这是给第一个窗口的可选文本，用来提供自定义词表或专名，让这些词更容易被预测对。API 写：可选文本，用来引导风格，或接上一段音频；prompt 应与音频语言一致（[请求体](https://platform.openai.com/docs/api-reference/audio/createTranscription)）。
- `carry_initial_prompt`（whisper.cpp `--carry-initial-prompt`）。为真时，每次内部 `decode()` 都在 prompt 前加上 `initial_prompt`；空间不够就从左边截掉。默认是假。whisper.cpp 头文件写，这可能减弱对上文的条件。
- `temperature` 和回退。`transcribe()` 的默认是 `(0.0, 0.2, 0.4, 0.6, 0.8, 1.0)`，温度高于 0 时依次使用，直到压缩比或对数概率不再判定失败。CLI 的 `--temperature` 默认 0，`--temperature_increment_on_fallback` 默认 0.2，因此命令行会组成同一条阶梯。温度大于 0 时关掉 `beam_size`；温度为 0 时关掉 `best_of`。whisper.cpp 是 `-tp`（默认 0）和 `-tpi`（默认 0.2）；`--no-fallback` 把增量设为 0。API 只写：温度为 0 时，模型会按对数概率自动升温，直到碰到某些阈值。API 没有公开那些阈值的名字。
- `beam_size` / `best_of`。openai-whisper CLI 默认都是 5：温度为 0 时用 beam，非零温度时 `best_of` 是候选条数。whisper.cpp 的 beam search 默认 `beam_size` 也是 5，greedy 的 `best_of` 默认是 5；CLI 在 beam size 大于 1 时改用 beam search。
- `condition_on_previous_text`。默认真。文档写：把上一窗口的输出当作下一窗口的 prompt；关掉之后，窗口之间的文字可能不一致，但更不容易陷进重复循环或时间戳错位。源码里，若它为假，或这次温度高于 0.5，就不再把这些 prompt token 送进后面的窗口。whisper.cpp 没有同名开关。库默认 `no_context = true`，[whisper.h](https://github.com/ggml-org/whisper.cpp/blob/master/include/whisper.h) 的注释是 “do not use past transcription (if any) as initial prompt for the decoder”。实现上，这句话对应每次 `whisper_full_with_state` 开始时清空已有的 prompt 状态；同一次调用内部，温度低于 0.5 时仍会把本调用已经解出的 token 放进后续窗口。CLI 帮助文本里没有 `--no-context`，调用时沿用这个默认值。
- `compression_ratio_threshold`。默认 2.4。gzip 压缩比高于此值就当作解码失败（源码注释写的是 “too repetitive”）。论文第 4.5 节用的也是 gzip 压缩率高于 2.4。whisper.cpp 的 CLI 帮助里没有这个名字；它有 `-et` / `--entropy-thold`，帮助文字是 “entropy threshold for decoder fail”，结构体初值 2.40。不要把这两个阈值当成同一个量。
- `logprob_threshold`。默认 -1.0。采样 token 的平均对数概率低于此值就当作失败。whisper.cpp 是 `-lpt` / `--logprob-thold`，初值 -1.0。
- `no_speech_threshold`。默认 0.6。`<|nospeech|>` 的概率高于此值，并且平均对数概率低于 `logprob_threshold` 时，把这段当作静音。whisper.cpp 是 `-nth` / `--no-speech-thold`，初值 0.6。
- `word_timestamps`。默认假。文档写这是实验功能，用交叉注意力和动态时间规整抽取词时间戳。翻译任务上，源码会警告词时间戳可能不可靠。whisper.cpp README 用 `-ml 1` 得到实验性的词级时间戳；`-wt` 是 “word timestamp probability threshold”；`-dtw` 是 “compute token-level timestamps”。
- `fp16`。默认真。CPU 上源码会警告 “FP16 is not supported on CPU; using FP32 instead” 并改用 FP32。whisper.cpp CLI 没有同名开关；README 写的是库内 F16 / F32 混合精度。
- `without_timestamps`。`DecodingOptions` 的注释是用 `<|notimestamps|>` 只采样文字 token，默认假。whisper.cpp 的 `-nt` 帮助文字是 “do not print timestamps”；头文件里 `no_timestamps` 的注释是 “do not generate timestamps”，解码时会放入 `<|notimestamps|>`。

## Prompt 最佳实践

机制来自三处，说的是同一类东西。论文第 2.3 节写：解码器是以音频为条件的语言模型，训练时以一定概率把当前音频段之前的转写放进解码器上下文，损失不计算在这段上文上；任务本身靠 `<|startoftranscript|>`、语言 token、`<|transcribe|>` 或 `<|translate|>`、以及是否带时间戳来指定。API 把 `prompt` 定义成引导风格或延续上一段音频的文本，并要求与音频语言一致。openai-whisper 把 `initial_prompt` 定义成第一个窗口的上下文，例如词表和专名。

因此 prompt 里应该是你希望看到的那种转写，而不是对模型下的命令。英文指令配中文音频，同时违反 “应与音频语言一致” 和 “不执行通用指令”。把 “只输出简体中文” 写进 prompt，属于后者。

专名、产品名、缩写和标点，用已经写成目标样子的例句或词表去偏。指南给出的官方例子是一份拼写表，模型是 `whisper-1`：

```text
ZyntriQix, Digique Plus, CynapseFive, VortiQore V8, EchoNix Array, OrbitalLink Seven, DigiFractal Matrix, PULSE, RAPT, B.R.I.C.K., Q.U.A.R.T.Z., F.L.I.N.T.
```

指南写，这可以提高对不常见词和缩写的识别，但列表必须短。`initial_prompt` 的文档用的是同一机制：custom vocabularies or proper nouns。指南列出的适用场景还有：接上较长录音的上一段上下文；保留标点、大小写和填充词；为一种语言选择文字系统。large-v3 发布说明写，评测时把中文都转成了简体，以免简繁不一致被算成错误；并写 “the model can be steered to produce outputs in either form”。页面上的示例没有以文字形式出现在抓到的正文里，所以这里不引用那张图。下面这句只是在演示 “用已经写好的文字去偏”，不是官方示例：`张伟在会上讲了 Visper 和 API，记录使用简体中文。`

长度上限三处不一致：

- API 指南和 “Improving reliability” 写的是最多 224 个 token，对象是托管的 `whisper-1`。
- openai-whisper 没有写 “224” 这个句子。`decoding.py` 只保留 prompt 的最后 `n_text_ctx // 2 - 1` 个 token；`transcribe.py` 用同一个式子做 `remaining_prompt_length`。large-v3 的 `max_target_positions` 是 448。若检查点里的 `n_text_ctx` 就是 448，这个式子等于 223。223 是公式算出来的，源码里没有单独写成 “223 tokens”。
- whisper.cpp 的帮助文字是 “max n_text_ctx/2 tokens”。`whisper.cpp` 里文本上下文的结构体初值是 448，加载模型时会被文件里的 `n_text_ctx` 覆盖；预算是 `min(n_max_text_ctx, n_text_ctx/2)`。拼 prompt 时还会先放一个 previous-token 标记，动态上文占用的是 `max_prompt_ctx - 1`。`--carry-initial-prompt` 过长时，日志写会只保留最后若干 token。

长音频要分开看。当前语音转写指南没有写 “prompt 只作用于第一段 30 秒”。它写的是：文件上限 25 MB，更大就要压缩或切开，不要在句子中间切开，否则会丢掉上下文；prompt 的用途之一是带上较长录音里上一个切片的上下文。`chunking_strategy` 的说明是另一件事：不设置时，音频按一整块转写；超过 30 秒且使用 `gpt-4o-transcribe-diarize` 时必须设置。不要把这句当成 large-v3 的窗口规则。

openai-whisper 自己的规则是：`initial_prompt` 只供给第一个 30 秒窗口，除非 `carry_initial_prompt=True`。`condition_on_previous_text` 默认为真，后续窗口拿到的是模型自己上一窗口的输出；温度高于 0.5 时这段上文会被丢掉。论文第 4.5 节的策略与此相符：温度低于 0.5 时，把前一窗口的转写作为 previous-text conditioning。

翻译用 `task` / `--task translate` / `-tr`，不要写进 prompt。`turbo` 即使指定了翻译任务也返回源语言，需要翻译时用 `large` 或 `large-v3`，不要用 turbo。

中英夹杂：打开的 README、模型卡、API 指南和论文都没有给出一种 prompt 写法来处理一句里的中英切换。语言检测在 openai-whisper 里是整次调用只看开头最多 30 秒。这和下面的产品决定不是一回事。

文档支持的做法：带上一段转写的结尾；用一小组已经写对的专名或缩写；想要某种文字，就在 prompt 里使用那种文字（简体汉字、你要的标点）。文档不支持的做法：角色说明、 “输出 markdown”、 “输出 JSON”、否定指令（“不要重复”“忽略背景音乐”）。这些都落在 “不执行通用指令” 那一句上。

## 质量与幻觉

openai-whisper 和论文第 4.5 节对齐的默认旋钮是：

- beam search，5 条 beam（温度为 0 时）。
- 温度从 0 开始，每次加 0.2，直到 1.0。平均对数概率低于 -1，或 gzip 压缩比高于 2.4，就换下一个温度。
- 温度低于 0.5 时才把上一窗口文字当作条件。
- `<|nospeech|>` 概率高于 0.6，并且平均对数概率低于 -1，就把该段当作没有语音，而不是再升温硬解。

`hallucination_silence_threshold` 没有默认数值。CLI 帮助写：需要同时打开 `word_timestamps`，并跳过长于该秒数的静音，当一次可能的幻觉被检测到时。模型卡写，beam search 和温度调度不能完全去掉重复。论文写，这些启发式是在绕开噪声预测，不能当成已经消掉幻觉。

## 在本仓库里

Visper 把用户已经拥有的录音在本机做成转写稿。权重是完整的 `ggml-large-v3.bin`，从 `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin` 下载；whisper.cpp 的模型表把这个文件标成 2.9 GiB，ADR 0001 写大约 3 GB。推理通过 whisper-rs 调用 whisper.cpp。GPU 在 macOS 上是 Metal，在 Windows 上是 Vulkan（ADR 0003），开启 flash attention；这一次设备起不来，这一次转写就用 CPU。

自动模式下，录音按静音切开，每一段只判断为简体中文路径或英文路径，再按该语言转写。英文保持英文，中文保持中文。转写稿里的中文是简体中文，繁体在显示和保存前被转成简体。没有翻译。用户不能提供自定义 prompt 或词表，这是 ADR 0002 的产品决定，不是 Whisper 缺少 prompt 参数。实现上每个窗口调用一次解码，`set_translate(false)`，`set_language` 为 `zh` 或 `en`，`set_no_context(true)`，beam size 为 5。模型有粤语 token，本应用没有粤语选项；这类录音走中文路径再转成简体，不保证质量。

## 来源

下面是这次实际打开并据此写了事实的页面。

- https://github.com/openai/whisper
- https://raw.githubusercontent.com/openai/whisper/main/README.md
- https://github.com/openai/whisper/blob/main/model-card.md
- https://github.com/openai/whisper/blob/main/LICENSE
- https://github.com/openai/whisper/blob/main/whisper/__init__.py
- https://github.com/openai/whisper/blob/main/whisper/transcribe.py
- https://github.com/openai/whisper/blob/main/whisper/decoding.py
- https://github.com/openai/whisper/blob/main/whisper/model.py
- https://github.com/openai/whisper/blob/main/whisper/tokenizer.py
- https://github.com/openai/whisper/releases
- https://github.com/openai/whisper/discussions/1762
- https://huggingface.co/openai/whisper-large-v3
- https://huggingface.co/openai/whisper-large-v3/raw/main/README.md
- https://huggingface.co/openai/whisper-large-v3/raw/main/config.json
- https://huggingface.co/openai/whisper-large-v3-turbo
- https://huggingface.co/openai/whisper-large-v3-turbo/raw/main/config.json
- https://platform.openai.com/docs/guides/speech-to-text
- https://platform.openai.com/docs/api-reference/audio/createTranscription
- https://platform.openai.com/docs/static/api-definition.yaml
- https://github.com/ggml-org/whisper.cpp
- https://raw.githubusercontent.com/ggml-org/whisper.cpp/master/README.md
- https://github.com/ggml-org/whisper.cpp/blob/master/models/README.md
- https://github.com/ggml-org/whisper.cpp/blob/master/examples/cli/cli.cpp
- https://github.com/ggml-org/whisper.cpp/blob/master/include/whisper.h
- https://github.com/ggml-org/whisper.cpp/blob/master/src/whisper.cpp
- https://arxiv.org/abs/2212.04356
