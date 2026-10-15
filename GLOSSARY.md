# 转写

把用户已经拥有的录音，在本机变成一份转写稿。

## Language

**Recording** (录音):
用户已经拥有的一个音频文件。不是当场从麦克风录下来的声音。
_Avoid_: 录音过程, 片段, 音轨, clip

**Transcription** (转写):
把一份录音变成一份转写稿的一次任务。
_Avoid_: 翻译, 识别, job

**Transcript** (转写稿):
一份录音对应的文字结果，由若干分段组成。文字只含简体中文、英文，或两者。在应用里只读。
_Avoid_: 翻译, 字幕, 纪要, 文稿, 繁体中文

**Chinese** (简体中文):
转写稿里中文部分使用的文字。只指简体中文。繁体汉字在显示和保存之前转成简体，不进入转写稿。
_Avoid_: 繁体中文, 正体中文, 粤语

**Segment** (分段):
转写稿里的一段连续文字，带有开始时间和结束时间。一段不代表一个说话人。
_Avoid_: 字幕条, 说话人轮次, cue

**Library** (资料库):
应用持有的录音与转写稿。每份录音由它的绝对路径标识，并带有一种状态。音频文件本身不放进资料库。
_Avoid_: 历史, 项目, 播放列表, 副本

## Status

**Queued** (等待中):
已经在资料库里、还没开始的转写。
_Avoid_: pending, backlog

**Transcribing** (转写中):
正在进行的转写。同一时间只有一条。
_Avoid_: running, processing

**Completed** (完成):
已经成功写进资料库的转写。
_Avoid_: done, success

**Failed** (失败):
没有成功的转写。半成品不保留。
_Avoid_: error

**Cancelled** (已取消):
被用户取消的转写。半成品不保留，录音还在。
_Avoid_: aborted, stopped

**Missing** (文件缺失):
录音的路径已经打不开。这不是一种状态，原来的状态还在。
_Avoid_: deleted
