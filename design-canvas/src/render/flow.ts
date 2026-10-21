import { el, esc } from "../catalog";

const NODES: { id: string; num: string; title: string; note: string; src: string }[] = [
  { id: "launch", num: "启动", title: "打开 Visper", note: "一个窗口。没有模型则进入选择。", src: "src/ui/render.rs:2329" },
  { id: "choose", num: "1", title: "选择转写模型", note: "Large v3 或 Large v3 Turbo，点一下开始下载。", src: "src/ui/render.rs:1985" },
  { id: "download", num: "2", title: "下载模型", note: "主栏顶部进度条。失败则重试，或切回已有模型。", src: "src/ui/render.rs:607" },
  { id: "empty", num: "5", title: "空资料库", note: "模型在本机之后才能加入录音。", src: "src/ui/render.rs:1872" },
  { id: "queued", num: "12", title: "等待中", note: "可改自动 / 中文 / 英文。模型未就绪则等下载。", src: "src/ui/render.rs:1551" },
  { id: "running", num: "17", title: "转写中", note: "实时分段。可取消。同一时间只有一条。", src: "src/ui/render.rs:1634" },
  { id: "done", num: "8", title: "完成", note: "阅读、点分段回听、导出 Markdown、重新转写。", src: "src/ui/render.rs:2069" },
  { id: "failed", num: "35", title: "失败或已取消", note: "不留半成品。有上次稿则恢复，并可重新转写。", src: "src/ui/render.rs:1855" },
  { id: "missing", num: "39", title: "文件缺失", note: "原状态还在。稿可阅读、可导出，不能播放。", src: "src/ui/render.rs:1835" },
  { id: "export", num: "53", title: "导出", note: "系统目录面板，确认按钮标题是「导出」。", src: "src/ui.rs:529" },
  { id: "remove", num: "51", title: "从资料库移除", note: "有已保存稿时先确认。录音文件留在原处。", src: "src/ui.rs:487" },
  { id: "quit", num: "50", title: "退出", note: "正在转写时询问。确认后取消当前条，队列下次继续。", src: "src/ui.rs:325" },
];

export function renderFlow(): string {
  const nodes = NODES.map((node) =>
    el(
      `flow.${node.id}`,
      node.title,
      node.src,
      `<em>${esc(node.num)}</em> <b>${esc(node.title)}</b><span>${esc(node.note)}</span>`,
      { class: "flow-node" },
    ),
  ).join("");
  return el(
    "flow",
    "交互流程",
    "src/ui/render.rs:463",
    `<h3>Visper 怎么在这些界面之间走</h3><p class="lead">从左到右、从上到下。编号对应画布上的帧。点一块可以复制它的元素引用。</p><div class="flow-grid">${nodes}</div><p class="lead" style="margin-top:16px">加入录音：文件菜单、⌘O、资料库「加入录音」、空状态「选择录音」，或把文件拖进窗口。没有模型时这些入口都不生效，并可能出现通知条。菜单栏、关于面板和文件面板是系统界面，文案来自 Visper。</p>`,
    { class: "flow" },
  );
}
