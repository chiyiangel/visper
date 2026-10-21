import { renderAbout, renderMenu, renderOpenPanel } from "./render/chrome";
import { renderFlow } from "./render/flow";
import { renderWindow } from "./render/window";
import type { FrameSpec, Group, LibRow, Recording, Segment, ThemeName, WindowModel } from "./types";

const PATH = "/Users/lin/录音/产品讨论.m4a";

const PREVIEW: Segment[] = [
  { id: "000004", time: "00:00:04", text: "我们先看一下这次产品讨论的目标。" },
  { id: "000018", time: "00:00:18", text: "界面需要更清晰，让主要操作自然出现。" },
  { id: "000036", time: "00:00:36", text: "We should keep the workflow simple and predictable." },
  { id: "000052", time: "00:00:52", text: "语言自动判断，中文和英文都保留原文。" },
  { id: "000116", time: "00:01:16", text: "接下来讨论导入和转写的反馈。" },
  { id: "000142", time: "00:01:42", text: "完成之后，可以按分段回听，再导出 Markdown。" },
];

function playingSegs(): Segment[] {
  return PREVIEW.map((seg) => ({ ...seg, playing: seg.id === "000036" }));
}

function doneRows(selected: string | null): LibRow[] {
  const rows: LibRow[] = [
    { id: "product", name: "产品讨论.m4a", status: "completed", extra: "· 18:05 · 用时 04:08", tag: "Turbo" },
    { id: "interview", name: "访谈记录.wav", status: "completed", extra: "· 40:00 · 用时 09:37", tag: "Large" },
    { id: "lecture", name: "课程录音.mp3", status: "completed", extra: "· 40:00 · 用时 09:37", tag: "Turbo" },
    { id: "weekly", name: "团队周会.m4a", status: "completed", extra: "· 40:00 · 用时 09:37", tag: "Large" },
    { id: "retro", name: "项目回顾.flac", status: "completed", extra: "· 40:00 · 用时 09:37", tag: "Large" },
  ];
  return rows.map((row) => ({ ...row, selected: row.id === selected }));
}

function finished(selected: string | null): Group[] {
  return [{ id: "finished", label: "已结束", rows: doneRows(selected) }];
}

function runningRows(extra: string, progress?: number): Group[] {
  return [
    {
      id: "running",
      label: "转写中",
      rows: [
        {
          id: "product",
          name: "产品讨论.m4a",
          status: "transcribing",
          selected: true,
          extra,
          progress,
        },
      ],
    },
    {
      id: "queued",
      label: "等待中",
      rows: [
        { id: "interview", name: "访谈记录.wav", status: "queued", extra: "· 40:00" },
        { id: "lecture", name: "课程录音.mp3", status: "queued", extra: "· 40:00" },
      ],
    },
    {
      id: "finished",
      label: "已结束",
      rows: [
        { id: "weekly", name: "团队周会.m4a", status: "completed", extra: "· 40:00 · 用时 09:37", tag: "Large" },
        { id: "retro", name: "项目回顾.flac", status: "completed", extra: "· 40:00 · 用时 09:37", tag: "Large" },
      ],
    },
  ];
}

function product(over: Partial<Recording> = {}): Recording {
  return {
    name: "产品讨论.m4a",
    path: PATH,
    status: "completed",
    duration: "18:05",
    language: "auto",
    took: "04:08",
    modelName: "Large v3 Turbo",
    segments: PREVIEW.map((seg) => ({ ...seg })),
    hint: "点击分段，从此处回听",
    playback: { enabled: true, mode: "idle", jump: true },
    exportEnabled: true,
    showRerun: true,
    rerunEnabled: true,
    ...over,
  };
}

function shell(theme: ThemeName, view: WindowModel["view"], over: Partial<WindowModel> = {}): WindowModel {
  return {
    theme,
    width: 1120,
    height: 760,
    canAdd: true,
    groups: finished("product"),
    footer: "large",
    view,
    ...over,
  };
}

function win(
  partial: Omit<FrameSpec, "themeable" | "render" | "width" | "height"> & {
    width?: number;
    height?: number;
    scrolled?: boolean;
    model: (theme: ThemeName) => WindowModel;
  },
): FrameSpec {
  const sample = partial.model("light");
  return {
    id: partial.id,
    number: partial.number,
    title: partial.title,
    section: partial.section,
    caption: partial.caption,
    assumption: partial.assumption,
    play: partial.play,
    width: partial.width ?? sample.width,
    height: partial.height ?? sample.height,
    themeable: true,
    render: (theme) => renderWindow(partial.model(theme), { scrolled: partial.scrolled }),
  };
}

const settled = {
  headline: "还需约 2 分钟",
  settled: true,
  finishAt: "预计 14:32 完成",
  elapsed: "04:12",
  processed: "12:18 / 18:05",
  speed: "2.9×",
  fraction: 738000 / 1085000,
  percent: "68%",
};

const SAMPLE = "示例数据来自 src/ui/preview.rs 的预览场景，时间与百分比按 src/library.rs 的进度公式算出。";

export function allFrames(): FrameSpec[] {
  const light = defineFrames();
  const dark: FrameSpec[] = light
    .filter((frame) => frame.themeable)
    .map((frame) => ({
      ...frame,
      id: `${frame.id}-dark`,
      number: `${frame.number}d`,
      section: "深色模式",
      caption: `深色外观，与浅色「${frame.number} · ${frame.title}」同一状态。${frame.caption}`,
    }));
  return [...light, ...dark];
}

function defineFrames(): FrameSpec[] {
  return [
    {
      id: "0-flow",
      number: "0",
      title: "交互流程总图",
      section: "交互流程",
      caption: "屏幕之间怎么走。编号能在画布上找到对应的帧。",
      width: 1180,
      height: 680,
      themeable: false,
      render: () => renderFlow(),
    },
    win({
      id: "1-first-choose",
      number: "1",
      title: "首次使用 · 选择模型",
      section: "首次使用与模型",
      caption: "资料库是空的，还没有选过模型。两张卡片都不预选。加入录音处于禁用，悬停提示见 27b。",
      play: "mark",
      model: (theme) =>
        shell(theme, { kind: "choose-model" }, { canAdd: false, groups: [], footer: "none" }),
    }),
    win({
      id: "2-first-download",
      number: "2",
      title: "正在准备 Large v3",
      section: "首次使用与模型",
      caption: "点了 Large v3 之后。进度 1.8 / 3.1 GB · 57%，与预览场景 first-launch-download 一致。选择录音仍禁用。",
      model: (theme) =>
        shell(theme, { kind: "ready-empty" }, {
          canAdd: false,
          groups: [],
          footer: "large",
          download: {
            state: "running",
            title: "正在准备 Large v3",
            note: "首次使用需下载一次，之后可离线转写",
            amount: "1.8 / 3.1 GB · 57%",
            fraction: 1795000000 / 3095033483,
          },
        }),
    }),
    win({
      id: "3-first-connecting",
      number: "3",
      title: "正在连接",
      section: "首次使用与模型",
      caption: "下载已经开始，服务器还没给出总大小。数值处写「正在连接…」，进度条停在 0。",
      model: (theme) =>
        shell(theme, { kind: "ready-empty" }, {
          canAdd: false,
          groups: [],
          footer: "large",
          download: {
            state: "running",
            title: "正在准备 Large v3",
            note: "首次使用需下载一次，之后可离线转写",
            amount: "正在连接…",
            fraction: 0,
          },
        }),
    }),
    win({
      id: "4-first-download-failed",
      number: "4",
      title: "首次下载失败",
      section: "首次使用与模型",
      caption: "本机还没有另一个模型，所以只有「重试」，没有「切回」。错误原文来自预览场景。",
      model: (theme) =>
        shell(theme, { kind: "ready-empty" }, {
          canAdd: false,
          groups: [],
          footer: "large",
          download: {
            state: "failed",
            title: "Large v3 下载失败",
            error: "连接下载地址失败：网络中断",
          },
        }),
    }),
    win({
      id: "5-empty-ready",
      number: "5",
      title: "空资料库 · 可以加入",
      section: "首次使用与模型",
      caption: "至少有一个模型已下载。空状态恢复成拖入或选择文件，选择录音可用。",
      play: "mark",
      model: (theme) => shell(theme, { kind: "ready-empty" }, { groups: [], footer: "large" }),
    }),
    win({
      id: "6-drag-over",
      number: "6",
      title: "拖入落点",
      section: "首次使用与模型",
      caption: "把合法录音拖到窗口上。主栏变成播放定位的底色，虚线框变成强调色。",
      model: (theme) =>
        shell(theme, { kind: "ready-empty" }, { groups: [], footer: "large", dragOver: true }),
    }),
    win({
      id: "7-notice-no-model",
      number: "7",
      title: "模型未就绪时打开文件",
      section: "首次使用与模型",
      caption: "从 Finder 或 Dock 丢进来的文件不会加入。通知条说明原因，点「关闭」收起。",
      model: (theme) =>
        shell(theme, { kind: "choose-model" }, {
          canAdd: false,
          groups: [],
          footer: "none",
          notice: "模型下载完成后才能加入录音",
        }),
    }),
    win({
      id: "8-completed-playing",
      number: "8",
      title: "完成 · 正在播放",
      section: "资料库与阅读",
      caption: `完成并回听。第三段高亮，按钮是暂停。页脚当前模型是 Large v3，这条稿的标签是 Turbo。${SAMPLE}`,
      model: (theme) =>
        shell(theme, {
          kind: "recording",
          rec: product({
            segments: playingSegs(),
            playback: { enabled: true, mode: "playing", time: "00:41 / 18:05", jump: true },
          }),
        }),
    }),
    win({
      id: "9-paused",
      number: "9",
      title: "已暂停",
      section: "资料库与阅读",
      caption: "暂停后留在原来的位置。按钮回到播放，状态写成「已暂停」。",
      model: (theme) =>
        shell(theme, {
          kind: "recording",
          rec: product({
            segments: playingSegs(),
            playback: { enabled: true, mode: "paused", time: "00:41 / 18:05", jump: true },
          }),
        }),
    }),
    win({
      id: "10-completed-idle",
      number: "10",
      title: "完成 · 尚未播放",
      section: "资料库与阅读",
      caption: "还没开始回听。底部只写「播放」，不显示时间。",
      model: (theme) => shell(theme, { kind: "recording", rec: product() }),
    }),
    win({
      id: "11-completed-empty",
      number: "11",
      title: "完成 · 空转写稿",
      section: "资料库与阅读",
      caption: "转写成功但没有可写的文字。仍是完成，不是失败。原文件在就可以播放，导出不可用。",
      model: (theme) =>
        shell(theme, {
          kind: "recording",
          rec: product({
            segments: [],
            emptyCopy: "录音中未得到可转写的文字。",
            exportEnabled: false,
            playback: { enabled: true, mode: "idle", jump: false },
          }),
        }),
    }),
    win({
      id: "12-queued-first",
      number: "12",
      title: "等待中 · 首次转写",
      section: "资料库与阅读",
      caption: "还没有稿。语言可以改，默认停在「自动」。播放、导出不可用，也没有重新转写。",
      model: (theme) =>
        shell(
          theme,
          {
            kind: "recording",
            rec: product({
              status: "queued",
              duration: undefined,
              took: undefined,
              modelName: undefined,
              segments: [],
              emptyCopy: "还没有转写稿。",
              hint: "",
              exportEnabled: false,
              showRerun: false,
              rerunEnabled: false,
              playback: { enabled: false, mode: "idle" },
            }),
          },
          {
            groups: [
              {
                id: "queued",
                label: "等待中",
                rows: [{ id: "product", name: "产品讨论.m4a", status: "queued", selected: true }],
              },
              {
                id: "finished",
                label: "已结束",
                rows: doneRows(null).slice(1, 3),
              },
            ],
          },
        ),
    }),
    win({
      id: "13-queued-previous",
      number: "13",
      title: "等待中 · 已有转写稿",
      section: "资料库与阅读",
      caption: "重新排队之前的稿还在，可以播放和导出。语言滑块停在「中文」。还没结束，所以行上不显示模型标签。",
      model: (theme) =>
        shell(
          theme,
          {
            kind: "recording",
            rec: product({
              status: "queued",
              language: "zh",
              took: undefined,
              modelName: undefined,
              showRerun: false,
              rerunEnabled: false,
            }),
          },
          {
            groups: [
              {
                id: "queued",
                label: "等待中",
                rows: [{ id: "product", name: "产品讨论.m4a", status: "queued", selected: true, extra: "· 18:05" }],
              },
              { id: "finished", label: "已结束", rows: doneRows(null).slice(1, 3) },
            ],
          },
        ),
    }),
    win({
      id: "14-queued-model",
      number: "14",
      title: "等待中 · 模型未就绪",
      section: "资料库与阅读",
      caption: "当前模型还在下载，排队的录音写明会自动开始。已有别的模型时仍可加入新录音。",
      model: (theme) =>
        shell(
          theme,
          {
            kind: "recording",
            rec: product({
              status: "queued",
              took: undefined,
              modelName: undefined,
              segments: [],
              emptyCopy: "还没有转写稿。",
              hint: "",
              waitingForModel: true,
              exportEnabled: false,
              showRerun: false,
              rerunEnabled: false,
              playback: { enabled: false, mode: "idle" },
            }),
          },
          {
            footer: "turbo",
            download: {
              state: "running",
              title: "正在准备 Large v3 Turbo",
              note: "首次使用需下载一次，之后可离线转写",
              amount: "0.6 / 1.6 GB · 39%",
              fraction: 640000000 / 1624555275,
            },
            groups: [
              {
                id: "queued",
                label: "等待中",
                rows: [{ id: "product", name: "产品讨论.m4a", status: "queued", selected: true }],
              },
            ],
          },
        ),
    }),
    win({
      id: "15-preparing",
      number: "15",
      title: "正在准备录音",
      section: "转写中",
      caption: "还没有任何进度。详情标题是「正在准备录音…」。行上因为时钟已经开始、又算不出剩余，写成「正在估算…」。",
      model: (theme) =>
        shell(
          theme,
          {
            kind: "recording",
            rec: product({
              status: "transcribing",
              took: undefined,
              modelName: undefined,
              segments: [],
              emptyCopy: "正在转写，分段将在这里显示。",
              hint: "分段实时显示",
              showRerun: false,
              exportEnabled: false,
              playback: { enabled: false, mode: "idle" },
              progress: { headline: "正在准备录音…", settled: false, elapsed: "00:01" },
            }),
          },
          { groups: runningRows("· 正在估算…") },
        ),
    }),
    win({
      id: "16-estimating",
      number: "16",
      title: "正在估算剩余时间",
      section: "转写中",
      caption: "已用时不到 30 秒或进度不到 3%。有进度条，没有剩余分钟，也没有速度。",
      model: (theme) =>
        shell(
          theme,
          {
            kind: "recording",
            rec: product({
              status: "transcribing",
              took: undefined,
              modelName: undefined,
              segments: [],
              emptyCopy: "正在转写，分段将在这里显示。",
              hint: "分段实时显示",
              showRerun: false,
              exportEnabled: false,
              playback: { enabled: false, mode: "idle" },
              progress: {
                headline: "正在估算剩余时间…",
                settled: false,
                elapsed: "00:20",
                processed: "00:20 / 18:05",
                fraction: 20000 / 1085000,
                percent: "1%",
              },
            }),
          },
          { groups: runningRows("· 正在估算…", 0.018) },
        ),
    }),
    win({
      id: "17-transcribing",
      number: "17",
      title: "转写中 · 实时分段",
      section: "转写中",
      caption: `首次转写，分段已经出现。播放可用，导出不可用。书写标记会循环，这是界面里唯一的循环动画。${SAMPLE}`,
      assumption: "「预计 14:32 完成」只示范格式。应用里这是本地时间，等于现在加上剩余时间。",
      play: "writing",
      model: (theme) =>
        shell(
          theme,
          {
            kind: "recording",
            rec: product({
              status: "transcribing",
              took: undefined,
              modelName: undefined,
              hint: "分段实时显示",
              writing: true,
              showRerun: false,
              exportEnabled: false,
              progress: settled,
              playback: { enabled: true, mode: "idle", jump: true },
            }),
          },
          { groups: runningRows("· 还需约 2 分钟", 0.68) },
        ),
    }),
    win({
      id: "18-rerun",
      number: "18",
      title: "重新转写中",
      section: "转写中",
      caption: "新的分段在往外长，导出仍写上次保存的稿，旁边有一行说明。播放走已有的稿。",
      play: "writing",
      model: (theme) =>
        shell(
          theme,
          {
            kind: "recording",
            rec: product({
              status: "transcribing",
              took: undefined,
              modelName: undefined,
              exportsPrevious: true,
              hint: "分段实时显示",
              writing: true,
              showRerun: false,
              segments: [
                { id: "live-0", time: "00:00:00", text: "这次重跑的新分段。" },
                { id: "live-1", time: "00:00:12", text: "导出仍使用上次已保存的转写稿。" },
              ],
              progress: settled,
              playback: { enabled: true, mode: "idle", jump: true },
            }),
          },
          { groups: runningRows("· 还需约 2 分钟", 0.68) },
        ),
    }),
    win({
      id: "19-long",
      number: "19",
      title: "长列表与长转写稿",
      section: "资料库与阅读",
      caption:
        "预览场景 long 有 100 条录音和 1000 个分段。这里只画出滚到中段的样子和两条滚动条，避免把整份稿铺开。文件名提示是悬停 500 毫秒后出现的。",
      assumption: "滚动条的位置是示意，不是按 1000 段精确换算的滑块长度。",
      scrolled: true,
      model: (theme) =>
        shell(
          theme,
          {
            kind: "recording",
            rec: product({
              name: "这是一份包含很长文件名称与多层路径的产品讨论记录用于检查布局是否发生重叠_2026-10-14.m4a",
              path: "/Users/lin/文稿/年度计划/项目资料/产品研究/转写测试/这是一份包含很长文件名称与多层路径的产品讨论记录用于检查布局是否发生重叠_2026-10-14.m4a",
              segments: Array.from({ length: 12 }, (_, index) => ({
                id: `long-${index}`,
                time: `00:${String(Math.floor((index * 18) / 60)).padStart(2, "0")}:${String((index * 18) % 60).padStart(2, "0")}`,
                text:
                  index === 2
                    ? "第 3 个分段。我们使用真实的中英文混排检查长内容的换行与阅读布局。The layout should remain stable while the transcript grows. 一个很长的连续单词用于检验换行：abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz。"
                    : `第 ${index + 1} 个分段。我们使用真实的中英文混排检查长内容的换行与阅读布局。`,
              })),
            }),
          },
          {
            groups: [
              {
                id: "finished",
                label: "已结束",
                rows: [
                  {
                    id: "long-name",
                    name: "这是一份包含很长文件名称与多层路径的产品讨论记录用于检查布局是否发生重叠_2026-10-14.m4a",
                    status: "completed",
                    selected: true,
                    extra: "· 18:05 · 用时 04:08",
                    tag: "Turbo",
                  },
                  ...Array.from({ length: 9 }, (_, index) => ({
                    id: `n${index + 5}`,
                    name: `第${index + 5}份录音_包含较长名称的产品讨论与技术评审记录.m4a`,
                    status: "completed" as const,
                    extra: "· 40:00 · 用时 09:37",
                    tag: "Large" as const,
                  })),
                ],
              },
            ],
            tooltip: "filename",
            tooltipText:
              "这是一份包含很长文件名称与多层路径的产品讨论记录用于检查布局是否发生重叠_2026-10-14.m4a",
          },
        ),
    }),
    win({
      id: "20-narrow",
      number: "20",
      title: "窄窗口",
      section: "资料库与阅读",
      caption: "最小内容区 920 × 620。资料库宽 264，详情左右内边距 24，底部播放区高 72。",
      width: 920,
      height: 620,
      model: (theme) =>
        shell(
          theme,
          {
            kind: "recording",
            rec: product({
              segments: playingSegs(),
              playback: { enabled: true, mode: "playing", time: "00:41 / 18:05", jump: true },
            }),
          },
          { width: 920, height: 620, narrow: true },
        ),
    }),
    win({
      id: "21-unselected",
      number: "21",
      title: "未选择录音",
      section: "资料库与阅读",
      caption: "资料库里有录音，但没有选中。右侧是淡色标记和「选择左边的一条录音」。头部仍是 52 像素的拖动区。",
      model: (theme) =>
        shell(theme, { kind: "unselected" }, { groups: finished(null) }),
    }),
    win({
      id: "22-more",
      number: "22",
      title: "更多操作",
      section: "弹层与焦点",
      caption: "「从资料库移除」放在更多里，用危险色，不和导出并列。",
      model: (theme) =>
        shell(theme, { kind: "recording", rec: product({ moreOpen: true }) }),
    }),
    win({
      id: "23-model-popover",
      number: "23",
      title: "转写模型",
      section: "弹层与焦点",
      caption: "从页脚的模型名打开。当前是 Large v3，Turbo 还要下载 1.6 GB。点一项就切换并收起。",
      model: (theme) =>
        shell(theme, { kind: "recording", rec: product() }, {
          popover: { current: "large", large: "已下载", turbo: "需下载 1.6 GB" },
        }),
    }),
    win({
      id: "24-exported",
      number: "24",
      title: "已导出",
      section: "弹层与焦点",
      caption: "导出成功后，按钮旁出现「已导出」，大约 3 秒后消失。不打开文件，也不打开访达。",
      model: (theme) =>
        shell(theme, { kind: "recording", rec: product({ justExported: true }) }),
    }),
    win({
      id: "25-text-selection",
      number: "25",
      title: "选中转写文字",
      section: "弹层与焦点",
      caption: "拖动选择正文后，⌘C 拷贝选中的文字。高亮用选中底色，不显示编辑工具。",
      model: (theme) =>
        shell(theme, {
          kind: "recording",
          rec: product({
            segments: PREVIEW.map((seg) =>
              seg.id === "000036" ? { ...seg, select: [10, 27] as [number, number] } : { ...seg },
            ),
          }),
        }),
    }),
    win({
      id: "26-focus",
      number: "26",
      title: "键盘焦点环",
      section: "弹层与焦点",
      caption: "只有用 Tab 移动焦点时才出现。独立控件的环在外侧 2 像素，圆角跟着放大。鼠标点一下就收起。",
      model: (theme) =>
        shell(theme, { kind: "recording", rec: product() }, { focus: "detail.export" }),
    }),
    win({
      id: "27-tooltip-more",
      number: "27",
      title: "更多操作的提示",
      section: "弹层与焦点",
      caption: "纯图标按钮悬停 500 毫秒后出现提示。底色是文字色，字是底色，圆角 6。",
      model: (theme) =>
        shell(theme, { kind: "recording", rec: product() }, { tooltip: "more", tooltipText: "更多操作" }),
    }),
    win({
      id: "27b-tooltip-add",
      number: "27b",
      title: "还不能加入录音",
      section: "弹层与焦点",
      caption: "模型还没下载完时，加入录音整行禁用，悬停提示「模型下载完成后可以加入录音」。",
      model: (theme) =>
        shell(theme, { kind: "choose-model" }, {
          canAdd: false,
          groups: [],
          footer: "none",
          tooltip: "add",
          tooltipText: "模型下载完成后可以加入录音",
        }),
    }),
    win({
      id: "28-notice-export-empty",
      number: "28",
      title: "不能导出",
      section: "通知",
      caption: "菜单里的导出在没有已保存分段时给出这条通知。空转写稿的导出按钮本身是禁用的。",
      model: (theme) =>
        shell(
          theme,
          {
            kind: "recording",
            rec: product({
              segments: [],
              emptyCopy: "录音中未得到可转写的文字。",
              exportEnabled: false,
              playback: { enabled: true, mode: "idle", jump: false },
            }),
          },
          { notice: "不能导出。没有已保存的分段。" },
        ),
    }),
    win({
      id: "29-notice-export-failed",
      number: "29",
      title: "导出失败",
      section: "通知",
      caption: "写出文件失败。前缀「导出失败：」是固定的，后面是当时的错误。",
      assumption: "「没有写入权限」是举例，源码把底层错误原样接在前缀后面。",
      model: (theme) =>
        shell(theme, { kind: "recording", rec: product() }, { notice: "导出失败：没有写入权限。" }),
    }),
    win({
      id: "30-notice-playback",
      number: "30",
      title: "无法播放",
      section: "通知",
      caption: "没有音频输出时的通知。跳到某一段失败会写成「无法跳到这一段：…」。",
      model: (theme) =>
        shell(theme, { kind: "recording", rec: product() }, { notice: "没有可用的音频输出。" }),
    }),
    win({
      id: "31-notice-rejected",
      number: "31",
      title: "格式不能加入",
      section: "通知",
      caption: "第一期只收 m4a、mp3、wav、flac、aac。",
      model: (theme) =>
        shell(theme, { kind: "recording", rec: product() }, {
          notice: "不能加入「录像.mp4」。第一期只收 m4a、mp3、wav、flac、aac。",
        }),
    }),
    win({
      id: "32-notice-unreadable",
      number: "32",
      title: "文件打不开",
      section: "通知",
      caption: "路径打不开时不加入资料库。",
      model: (theme) =>
        shell(theme, { kind: "recording", rec: product() }, {
          notice: "打不开「锁住.m4a」，没有加入资料库。",
        }),
    }),
    win({
      id: "33-notice-duplicate-queue",
      number: "33",
      title: "已经在等待中",
      section: "通知",
      caption: "同一条路径已经在队列里，不会再加一条。",
      model: (theme) =>
        shell(
          theme,
          {
            kind: "recording",
            rec: product({
              status: "transcribing",
              took: undefined,
              modelName: undefined,
              hint: "分段实时显示",
              writing: true,
              showRerun: false,
              exportEnabled: false,
              progress: settled,
            }),
          },
          {
            notice: "「访谈记录.wav」已经在等待中，没有再加一条。",
            groups: runningRows("· 还需约 2 分钟", 0.68),
          },
        ),
    }),
    win({
      id: "34-notice-duplicate-running",
      number: "34",
      title: "已经在转写",
      section: "通知",
      caption: "正在转写的那一条再被加入时，只提示正在转写。",
      model: (theme) =>
        shell(
          theme,
          {
            kind: "recording",
            rec: product({
              status: "transcribing",
              took: undefined,
              modelName: undefined,
              hint: "分段实时显示",
              writing: true,
              showRerun: false,
              exportEnabled: false,
              progress: settled,
            }),
          },
          {
            groups: runningRows("· 还需约 2 分钟", 0.68),
            notice: "「产品讨论.m4a」正在转写。",
          },
        ),
    }),
    win({
      id: "35-failed-kept",
      number: "35",
      title: "转写失败 · 稿还在",
      section: "异常与文件缺失",
      caption: "失败提示用危险色，说明上次转写稿已保留。可以重新转写，也可以导出上次的稿。",
      model: (theme) =>
        shell(
          theme,
          {
            kind: "recording",
            rec: product({
              status: "failed",
              took: undefined,
              failure: {
                title: "转写失败",
                body: "无法解码录音。上次成功保存的转写稿仍然可用。\n上次转写稿已保留。",
              },
            }),
          },
          {
            groups: [
              {
                id: "finished",
                label: "已结束",
                rows: doneRows("product").map((row) =>
                  row.id === "product"
                    ? { ...row, status: "failed", extra: "· 18:05", tag: "Turbo" }
                    : row,
                ),
              },
            ],
          },
        ),
    }),
    win({
      id: "36-failed-empty",
      number: "36",
      title: "转写失败 · 没有稿",
      section: "异常与文件缺失",
      caption: "没有上次的稿就不留半成品。默认说明是「转写未能完成。」，有具体错误时换成那句。",
      model: (theme) =>
        shell(
          theme,
          {
            kind: "recording",
            rec: product({
              status: "failed",
              took: undefined,
              modelName: undefined,
              segments: [],
              emptyCopy: "这次转写没有保存分段。",
              hint: "",
              exportEnabled: false,
              playback: { enabled: false, mode: "idle" },
              failure: { title: "转写失败", body: "音频在 00:06:12 处解码失败。" },
            }),
          },
          {
            groups: [
              {
                id: "finished",
                label: "已结束",
                rows: [
                  { id: "product", name: "产品讨论.m4a", status: "failed", selected: true, extra: "· 18:05" },
                  ...doneRows(null).slice(1, 3),
                ],
              },
            ],
          },
        ),
    }),
    win({
      id: "37-cancelled-kept",
      number: "37",
      title: "已取消 · 稿还在",
      section: "异常与文件缺失",
      caption: "取消丢掉进行中的分段，已保存的稿留着。没有失败色的提示条。",
      model: (theme) =>
        shell(
          theme,
          {
            kind: "recording",
            rec: product({ status: "cancelled", took: undefined }),
          },
          {
            groups: [
              {
                id: "finished",
                label: "已结束",
                rows: doneRows("product").map((row) =>
                  row.id === "product" ? { ...row, status: "cancelled", extra: "· 18:05", tag: "Turbo" } : row,
                ),
              },
            ],
          },
        ),
    }),
    win({
      id: "38-cancelled-empty",
      number: "38",
      title: "已取消 · 没有分段",
      section: "异常与文件缺失",
      caption: "第一次转写被取消。正文是「已取消，没有分段。」播放和导出都不可用，可以重新转写。",
      model: (theme) =>
        shell(
          theme,
          {
            kind: "recording",
            rec: product({
              status: "cancelled",
              took: undefined,
              modelName: undefined,
              segments: [],
              emptyCopy: "已取消，没有分段。",
              hint: "",
              exportEnabled: false,
              playback: { enabled: false, mode: "idle" },
            }),
          },
          {
            groups: [
              {
                id: "finished",
                label: "已结束",
                rows: [
                  { id: "product", name: "产品讨论.m4a", status: "cancelled", selected: true },
                ],
              },
            ],
          },
        ),
    }),
    win({
      id: "39-missing-kept",
      number: "39",
      title: "文件缺失 · 稿还在",
      section: "异常与文件缺失",
      caption: "状态仍是完成。行上用琥珀色写「文件缺失」，不再写时长。稿可以读、可以导出，不能播放，也不能重新转写。",
      model: (theme) =>
        shell(
          theme,
          {
            kind: "recording",
            rec: product({
              rerunEnabled: false,
              playback: { enabled: false, mode: "idle", missingNote: true },
              missing: { title: "找不到原录音文件", body: "转写稿已保留，可继续阅读和导出。" },
            }),
          },
          {
            groups: [
              {
                id: "finished",
                label: "已结束",
                rows: doneRows("product").map((row) =>
                  row.id === "product"
                    ? { ...row, extra: "· 文件缺失", extraWarning: true, tag: "Turbo" }
                    : row,
                ),
              },
            ],
          },
        ),
    }),
    win({
      id: "40-missing-empty",
      number: "40",
      title: "文件缺失 · 空稿",
      section: "异常与文件缺失",
      caption: "保存过一份空转写稿。说明换成不能播放、不能重新转写。导出仍然不可用。",
      model: (theme) =>
        shell(
          theme,
          {
            kind: "recording",
            rec: product({
              segments: [],
              emptyCopy: "录音中未得到可转写的文字。",
              exportEnabled: false,
              rerunEnabled: false,
              playback: { enabled: false, mode: "idle", missingNote: true, jump: false },
              missing: {
                title: "找不到原录音文件",
                body: "空转写稿已保留，原录音文件缺失，无法播放或重新转写。",
              },
            }),
          },
          {
            groups: [
              {
                id: "finished",
                label: "已结束",
                rows: [
                  {
                    id: "product",
                    name: "产品讨论.m4a",
                    status: "completed",
                    selected: true,
                    extra: "· 文件缺失",
                    extraWarning: true,
                    tag: "Large",
                  },
                ],
              },
            ],
          },
        ),
    }),
    win({
      id: "41-missing-none",
      number: "41",
      title: "文件缺失 · 没有稿",
      section: "异常与文件缺失",
      caption: "失败之后文件又不见了。缺失提示和失败提示叠在一起。重新转写按钮在，但是禁用。",
      model: (theme) =>
        shell(
          theme,
          {
            kind: "recording",
            rec: product({
              status: "failed",
              took: undefined,
              modelName: undefined,
              segments: [],
              emptyCopy: "这次转写没有保存分段。",
              hint: "",
              exportEnabled: false,
              rerunEnabled: false,
              playback: { enabled: false, mode: "idle", missingNote: true },
              missing: { title: "找不到原录音文件", body: "无法读取原录音文件，转写和播放不可用。" },
              failure: { title: "转写失败", body: "音频在 00:06:12 处解码失败。" },
            }),
          },
          {
            groups: [
              {
                id: "finished",
                label: "已结束",
                rows: [
                  {
                    id: "product",
                    name: "产品讨论.m4a",
                    status: "failed",
                    selected: true,
                    extra: "· 文件缺失",
                    extraWarning: true,
                  },
                ],
              },
            ],
          },
        ),
    }),
    win({
      id: "41b-notice-missing-rerun",
      number: "41b",
      title: "缺失时重新转写",
      section: "异常与文件缺失",
      caption: "文件在点击的间隙打不开。重新转写不会开始，通知写「文件缺失，不能重新转写。」",
      model: (theme) =>
        shell(
          theme,
          {
            kind: "recording",
            rec: product({
              rerunEnabled: false,
              playback: { enabled: false, mode: "idle", missingNote: true },
              missing: { title: "找不到原录音文件", body: "转写稿已保留，可继续阅读和导出。" },
            }),
          },
          { notice: "文件缺失，不能重新转写。" },
        ),
    }),
    win({
      id: "42-switch-download",
      number: "42",
      title: "下载另一个模型",
      section: "异常与文件缺失",
      caption: "阅读一份已完成的稿时，页脚切到了还没下载的 Turbo。下载条只跨主栏。已有模型，所以仍可加入录音。",
      model: (theme) =>
        shell(theme, { kind: "recording", rec: product() }, {
          footer: "turbo",
          download: {
            state: "running",
            title: "正在准备 Large v3 Turbo",
            note: "首次使用需下载一次，之后可离线转写",
            amount: "0.6 / 1.6 GB · 39%",
            fraction: 640000000 / 1624555275,
          },
        }),
    }),
    win({
      id: "43-switch-failed",
      number: "43",
      title: "下载失败 · 可切回",
      section: "异常与文件缺失",
      caption: "Turbo 下载失败，Large v3 还在本机，所以多一个「切回 Large v3」。",
      model: (theme) =>
        shell(theme, { kind: "recording", rec: product() }, {
          footer: "turbo",
          download: {
            state: "failed",
            title: "Large v3 Turbo 下载失败",
            error: "连接下载地址失败：网络中断",
            switchBack: "Large v3",
          },
        }),
    }),
    {
      id: "44-menu-app",
      number: "44",
      title: "菜单 · Visper",
      section: "系统菜单与对话框",
      caption: "应用菜单。macOS 会把第一项菜单显示成应用名。",
      assumption: "菜单的白底、阴影和「服务」子菜单是系统画的。Visper 只提供条目和快捷键。",
      width: 860,
      height: 420,
      themeable: false,
      render: () => renderMenu("app"),
    },
    {
      id: "45-menu-file",
      number: "45",
      title: "菜单 · 文件",
      section: "系统菜单与对话框",
      caption: "加入录音始终可点；没模型时点了没有效果。导出没有单独的快捷键。",
      assumption: "菜单外观是系统的。",
      width: 860,
      height: 420,
      themeable: false,
      render: () => renderMenu("file"),
    },
    {
      id: "46-menu-edit",
      number: "46",
      title: "菜单 · 编辑",
      section: "系统菜单与对话框",
      caption: "只有拷贝，快捷键 ⌘C，拷贝的是转写稿里选中的文字。",
      assumption: "菜单外观是系统的。",
      width: 860,
      height: 420,
      themeable: false,
      render: () => renderMenu("edit"),
    },
    {
      id: "47-menu-window",
      number: "47",
      title: "菜单 · 窗口",
      section: "系统菜单与对话框",
      caption: "最小化 ⌘M，缩放没有快捷键。进入全屏幕和窗口列表是 AppKit 加上的。",
      assumption: "「进入全屏幕」和窗口列表的字是按源码注释里的系统行为画的，不是 Visper 自己的字符串。",
      width: 860,
      height: 420,
      themeable: false,
      render: () => renderMenu("window"),
    },
    {
      id: "48-menu-help",
      number: "48",
      title: "菜单 · 帮助",
      section: "系统菜单与对话框",
      caption: "帮助菜单里只有项目主页。搜索框是系统加在帮助菜单上的。",
      assumption: "搜索框的位置和「搜索」二字是系统帮助菜单的常见样子，源码没有写这个字符串。",
      width: 860,
      height: 420,
      themeable: false,
      render: () => renderMenu("help"),
    },
    {
      id: "49-about",
      number: "49",
      title: "关于 Visper",
      section: "系统菜单与对话框",
      caption: "标准关于面板。名称和版本由应用传入，版权来自 Info.plist。版本号是 Cargo.toml 里的 0.1.0。",
      assumption: "面板的尺寸、图标圆角和字距按 macOS 关于面板示意，源码没有自绘这个窗口。",
      width: 640,
      height: 420,
      themeable: false,
      render: () => renderAbout(),
    },
    win({
      id: "50-quit",
      number: "50",
      title: "退出确认",
      section: "系统菜单与对话框",
      caption: "正在转写时退出或关闭窗口，会先问。确认后取消当前转写，等待中的录音下次打开再继续。",
      assumption: "询问是系统对话框，贴在窗口上的位置和按钮样式按 macOS 警告框示意。按钮文案是源码里的。",
      model: (theme) =>
        shell(
          theme,
          {
            kind: "recording",
            rec: product({
              status: "transcribing",
              took: undefined,
              modelName: undefined,
              hint: "分段实时显示",
              writing: true,
              showRerun: false,
              exportEnabled: false,
              progress: settled,
            }),
          },
          {
            groups: runningRows("· 还需约 2 分钟", 0.68),
            sheet: {
              title: "正在转写，要退出 Visper 吗？",
              body: "「产品讨论.m4a」的转写会被取消。等待中的录音会在下次打开时继续。",
              ok: "退出",
              cancel: "继续转写",
              src: "src/ui.rs:325",
            },
          },
        ),
    }),
    win({
      id: "51-remove",
      number: "51",
      title: "移除确认",
      section: "系统菜单与对话框",
      caption: "已保存过转写稿（包括空稿）时，移除前要确认。没有稿则直接移除。原录音文件保留。",
      assumption: "对话框外框是系统的，按警告框示意。标题、说明和按钮文案来自源码。",
      model: (theme) =>
        shell(theme, { kind: "recording", rec: product() }, {
          sheet: {
            title: "从资料库移除",
            body: "已保存的转写稿将被移除，原录音文件会保留。",
            ok: "移除",
            cancel: "取消",
            src: "src/ui.rs:487",
          },
        }),
    }),
    {
      id: "52-open-panel",
      number: "52",
      title: "加入录音",
      section: "系统菜单与对话框",
      caption: "选择文件。Visper 把确认按钮设成「加入」，只选文件，不选文件夹，不多选。",
      assumption: "面板里的位置、边栏和文件列表是 macOS 打开面板的示意。源码只设置了 prompt。",
      width: 860,
      height: 560,
      themeable: false,
      render: () => renderOpenPanel("add"),
    },
    {
      id: "53-export-panel",
      number: "53",
      title: "导出到文件夹",
      section: "系统菜单与对话框",
      caption: "导出只选一个目录，确认按钮是「导出」。文件名用录音的名字加上 .md。",
      assumption: "面板外观是系统的，按 macOS 选择文件夹示意。",
      width: 860,
      height: 560,
      themeable: false,
      render: () => renderOpenPanel("export"),
    },
  ];
}
