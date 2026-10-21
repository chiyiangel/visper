import { el, esc, setFocus } from "../catalog";
import { icon, mark } from "../icons";
import { SRC } from "../srcmap";
import type { Group, LibRow, Recording, Segment, StatusName, ThemeName, WindowModel } from "../types";

const STATUS: Record<StatusName, { label: string; icon: string; tone: string }> = {
  queued: { label: "等待中", icon: "waiting", tone: "var(--secondary)" },
  transcribing: { label: "转写中", icon: "waiting", tone: "var(--accent)" },
  completed: { label: "完成", icon: "check", tone: "var(--accent)" },
  failed: { label: "失败", icon: "warning", tone: "var(--danger)" },
  cancelled: { label: "已取消", icon: "cancelled", tone: "var(--secondary)" },
};

const LANGS = [
  { id: "auto", label: "自动" },
  { id: "zh", label: "中文" },
  { id: "en", label: "英文" },
] as const;

const LANG_NOTE = { auto: "自动识别语言", zh: "中文", en: "英文" } as const;

const MODELS = {
  large: { id: "large-v3", full: "Large v3", short: "Large", blurb: "更准，较慢", note: "准确度最高，适合专有名词多的访谈", size: "3.1 GB" },
  turbo: { id: "large-v3-turbo", full: "Large v3 Turbo", short: "Turbo", blurb: "更快，略不准", note: "快得多，准确度略低，适合日常会议", size: "1.6 GB" },
} as const;

function statusEl(id: string, status: StatusName): string {
  const spec = STATUS[status];
  return el(
    id,
    spec.label,
    SRC.status,
    `<span style="color:${spec.tone};display:flex">${icon(spec.icon, 14)}</span><span class="small">${esc(spec.label)}</span>`,
    { class: "status" },
  );
}

function button(
  id: string,
  label: string,
  src: string,
  kind: "primary" | "secondary" | "quiet" | "icon",
  enabled: boolean,
  glyph?: string,
  extraClass = "",
): string {
  const inner =
    kind === "icon"
      ? icon(glyph ?? "more", 16)
      : `${glyph ? icon(glyph, 16) : ""}<span>${esc(label)}</span>`;
  const cls = [
    "btn",
    kind === "primary" ? "btn-primary" : "",
    kind === "secondary" ? "btn-secondary" : "",
    kind === "quiet" ? "btn-quiet" : "",
    kind === "icon" ? "btn-icon" : "",
    kind === "primary" && id === "empty.choose" ? "btn-lg" : "",
    enabled ? "" : "is-disabled",
    extraClass,
  ]
    .filter(Boolean)
    .join(" ");
  return el(id, label, src, inner, {
    class: cls,
    extra: enabled ? 'role="button"' : 'role="button" aria-disabled="true"',
  });
}

function segmentBody(seg: Segment, id: string): string {
  if (!seg.select) {
    return el(`${id}.body`, seg.text, SRC.segment, esc(seg.text), { class: "body" });
  }
  const [from, to] = seg.select;
  const html = `${esc(seg.text.slice(0, from))}${el(
    `${id}.selection`,
    seg.text.slice(from, to),
    SRC.segment,
    esc(seg.text.slice(from, to)),
    { tag: "span", class: "sel" },
  )}${esc(seg.text.slice(to))}`;
  return el(`${id}.body`, seg.text, SRC.segment, html, { class: "body" });
}

function renderSegment(seg: Segment): string {
  const id = `transcript.segment.${seg.id}`;
  return el(
    id,
    `${seg.time} ${seg.text}`,
    SRC.segment,
    `${el(id + ".stamp", seg.time, SRC.segment, esc(seg.time), { class: "stamp" })}${segmentBody(seg, id)}`,
    { class: seg.playing ? "segment playing" : "segment" },
  );
}

function renderRows(group: Group): string {
  return group.rows
    .map((row) => {
      const id = `library.row.${row.id}`;
      const spec = STATUS[row.status];
      const bits = [
        el(id + ".icon", row.name, SRC.row, icon("audio", 20), {
          extra: `style="color:${row.selected ? "var(--accent)" : "var(--secondary)"};margin-top:1px"`,
        }),
        `<div class="row-copy">${el(id + ".name", row.name, SRC.row, esc(row.name), { class: "row-name" })}${renderMeta(id, row, spec.label)}${row.progress != null ? el(id + ".progress", `${Math.floor(row.progress * 100)}%`, SRC.rowProgress, `<div class="bar"><span style="width:${Math.round(row.progress * 100)}%"></span></div>`, { class: "row-progress" }) : ""}</div>`,
      ].join("");
      return el(id, row.name, SRC.row, bits, {
        class: `row${row.status === "transcribing" ? " running" : ""}${row.selected ? " is-selected" : ""}`,
      });
    })
    .join("");
}

function renderMeta(id: string, row: LibRow, statusLabel: string): string {
  const extra = row.extra
    ? el(id + ".extra", row.extra, SRC.row, esc(row.extra), {
        class: "small extra",
        extra: row.extraWarning ? 'style="color:var(--warning)"' : "",
      })
    : "";
  const tag = row.tag
    ? el(id + ".tag", row.tag, SRC.tag, esc(row.tag), { class: "tag" })
    : "";
  return `<div class="meta">${statusEl(id + ".status", row.status)}${extra}${tag}</div>`.replace(
    statusLabel,
    statusLabel,
  );
}

function renderList(groups: Group[], scrolled: boolean): string {
  const body = groups
    .map(
      (group) =>
        el(`library.group.${group.id}`, group.label, SRC.group, esc(group.label), { class: "group" }) +
        renderRows(group),
    )
    .join("");
  const thumb = scrolled
    ? el("library.scrollbar", "资料库滚动条", SRC.scrollbar, "", {
        class: "thumb",
        extra: 'style="top:90px;height:72px"',
      })
    : "";
  return `<div class="list">${scrolled ? `<div class="scroll-shift">${body}</div>` : body}${thumb}</div>`;
}

function renderPopover(model: NonNullable<WindowModel["popover"]>): string {
  const option = (key: "large" | "turbo", state: string) => {
    const spec = MODELS[key];
    const current = model.current === key;
    const id = `model.${spec.id}`;
    return el(
      id,
      spec.full,
      SRC.popOption,
      `<div class="tick">${current ? icon("tick", 14) : ""}</div><div class="pop-copy"><div class="pop-name">${el(`${id}.name`, spec.full, SRC.modelName, esc(spec.full))}${el(`${id}.tag`, spec.short, SRC.tag, esc(spec.short), { class: "tag" })}</div>${el(`${id}.state`, `${spec.blurb} · ${state}`, SRC.blurb, esc(`${spec.blurb} · ${state}`), { class: "small pop-sub" })}</div>`,
      { class: current ? "pop-option is-current" : "pop-option" },
    );
  };
  return el(
    "model.popover",
    "转写模型",
    SRC.popover,
    `${el("model.popover.title", "转写模型", SRC.popTitle, "转写模型", { class: "pop-title" })}${option("large", model.large)}${option("turbo", model.turbo)}${el("model.popover.note", "只影响之后开始的转写，已有转写稿不变", SRC.popFoot, "只影响之后开始的转写，已有转写稿不变", { class: "pop-foot" })}`,
    { class: "popover" },
  );
}

function renderFooter(model: WindowModel): string {
  const label = model.footer === "none" ? "未选模型" : model.footer === "turbo" ? "Large v3 Turbo" : "Large v3";
  const name = el(
    "library.footer.model",
    label,
    SRC.footerButton,
    model.footer === "none" ? esc(label) : `<strong>${esc(label)}</strong>`,
    { class: model.footer === "none" ? "small" : "small" },
  );
  const button = el(
    "library.footer.button",
    label,
    SRC.footerButton,
    `${name}${icon("chevron-up", 10)}`,
    { class: "model-btn", extra: 'role="button"' },
  );
  return el(
    "library.footer",
    "本机转写",
    SRC.footer,
    `<div class="footer-line"></div><div class="footer-row">${el("library.footer.local", "本机转写", SRC.local, `${icon("lock", 14)}<span class="small">本机转写</span>`, { class: "local" })}${button}</div>${model.popover ? renderPopover(model.popover) : ""}`,
    { class: "footer" },
  );
}

function renderSidebar(model: WindowModel, count: number, scrolled: boolean): string {
  const empty = count === 0;
  const add = el(
    "library.add",
    "加入录音",
    SRC.add,
    `<span class="add-circle">${icon("plus-bold", 12)}</span><span>加入录音</span>`,
    { class: model.canAdd ? "add-row" : "add-row is-disabled", extra: 'role="button"' },
  );
  const addTip =
    model.tooltip === "add"
      ? el("library.add.tooltip", model.tooltipText ?? "模型下载完成后可以加入录音", SRC.addTip, esc(model.tooltipText ?? "模型下载完成后可以加入录音"), { class: "tip below" })
      : "";
  const lights = el(
    "window.traffic-lights",
    "关闭、最小化、缩放",
    SRC.lights,
    "<i></i><i></i><i></i>",
    { class: "lights", extra: 'aria-hidden="true"' },
  );
  return el(
    "library",
    "资料库",
    SRC.sidebar,
    `${el("library.drag", "拖动窗口", SRC.drag, lights, { class: "drag" })}<div class="add-wrap">${add}${addTip}</div><div class="lib-title">${el("library.title", "资料库", SRC.libraryTitle, "资料库", { class: "small med" })}${el("library.count", `${count} 份录音`, SRC.count, `${count} 份录音`, { class: "small ter" })}</div>${empty ? el("library.empty", "还没有录音", SRC.emptyNote, "还没有录音", { class: "small empty-note" }) : renderList(model.groups, scrolled)}${renderFooter(model)}`,
    { class: "sidebar" },
  );
}

function renderDownload(model: NonNullable<WindowModel["download"]>): string {
  if (model.state === "running") {
    return el(
      "download",
      model.title,
      SRC.download,
      `${el("download.icon", model.title, SRC.download, icon("download", 24), { extra: 'style="color:var(--accent)"' })}<div class="strip-main"><div class="strip-copy">${el("download.title", model.title, SRC.download, esc(model.title), { class: "med" })}${el("download.note", model.note, SRC.download, esc(model.note), { class: "small" })}</div><div class="bar"><span style="width:${Math.round(model.fraction * 100)}%"></span></div></div>${el("download.amount", model.amount, SRC.download, esc(model.amount), { class: "small mono" })}`,
      { class: "strip" },
    );
  }
  const back = model.switchBack
    ? button("download.switch-back", `切回 ${model.switchBack}`, SRC.switchBack, "secondary", true)
    : "";
  return el(
    "download",
    model.title,
    SRC.download,
    `${el("download.icon", model.title, SRC.download, icon("warning", 20), { extra: 'style="color:var(--danger)"' })}<div class="strip-copy" style="flex:1">${el("download.title", model.title, SRC.download, esc(model.title), { class: "med", extra: 'style="color:var(--danger)"' })}${el("download.error", model.error, SRC.download, esc(model.error), { class: "small" })}</div>${button("download.retry", "重试", SRC.retry, "secondary", true, "retry")}${back}`,
    { class: "strip" },
  );
}

function renderNotice(text: string): string {
  return el(
    "notice",
    text,
    SRC.notice,
    `${el("notice.icon", text, SRC.notice, icon("warning", 16), { extra: 'style="color:var(--warning)"' })}${el("notice.text", text, SRC.notice, esc(text), { class: "small" })}${button("notice.dismiss", "关闭", SRC.dismiss, "quiet", true)}`,
    { class: "notice" },
  );
}

function renderLanguage(rec: Recording): string {
  const index = LANGS.findIndex((lang) => lang.id === rec.language);
  const pills = LANGS.map((lang) =>
    el(`language.${lang.id}`, lang.label, SRC.language, esc(lang.label), {
      class: lang.id === rec.language ? "on" : "",
      extra: 'role="button"',
    }),
  ).join("");
  return el(
    "language",
    LANGS[index]?.label ?? "自动",
    SRC.language,
    `<div class="pill" style="left:calc(2px + (100% - 4px) / 3 * ${index})"></div>${pills}`,
    { class: "lang" },
  );
}

function renderProgress(rec: Recording): string {
  const progress = rec.progress;
  if (!progress) return "";
  const title = el(
    "progress.title",
    progress.headline,
    SRC.remaining,
    esc(progress.headline),
    { class: progress.settled ? "card-title settled" : "card-title" },
  );
  const finish = progress.finishAt
    ? el("progress.finish", progress.finishAt, "src/ui/render.rs:1788", esc(progress.finishAt), { class: "small" })
    : "";
  const stats = [
    progress.elapsed ? el("progress.elapsed", `已用时 ${progress.elapsed}`, SRC.stat, `<span class="small">已用时</span><b>${esc(progress.elapsed)}</b>`, { class: "stat" }) : "",
    progress.processed ? el("progress.processed", `已处理 ${progress.processed}`, SRC.stat, `<span class="small">已处理</span><b>${esc(progress.processed)}</b>`, { class: "stat" }) : "",
    progress.speed ? el("progress.speed", `速度 ${progress.speed}`, SRC.stat, `<span class="small">速度</span><b>${esc(progress.speed)}</b>`, { class: "stat" }) : "",
  ].join("");
  const meter =
    progress.fraction != null
      ? `<div class="meter"><div class="bar"><span style="width:${Math.round(progress.fraction * 100)}%"></span></div>${el("progress.percent", progress.percent ?? "", SRC.progress, esc(progress.percent ?? ""), { class: "small percent" })}</div>`
      : "";
  return el(
    "progress",
    progress.headline,
    SRC.progress,
    `<div class="card-top"><div style="display:flex;align-items:baseline;gap:8px;min-width:0">${title}${finish}</div>${button("progress.cancel", "取消转写", SRC.cancel, "secondary", true)}</div>${meter}<div class="stats">${stats}</div>`,
    { class: "card" },
  );
}

function renderAlert(id: string, kind: "warn" | "bad", title: string, body: string, src: string): string {
  return el(
    id,
    title,
    src,
    `${icon("warning", 18)}${el(`${id}.copy`, body, src, `${el(`${id}.title`, title, src, esc(title), { tag: "strong" })}${el(`${id}.body`, body, src, esc(body).replaceAll("\n", "<br>"), { class: "small" })}`, { class: "copy" })}`,
    { class: kind === "warn" ? "alert warn" : "alert bad" },
  );
}

function renderActions(rec: Recording, tooltip?: WindowModel["tooltip"], tooltipText?: string): string {
  const exported = rec.justExported
    ? el("detail.exported", "已导出", SRC.exported, "已导出", { class: "small" })
    : "";
  const rerun = rec.showRerun
    ? button("detail.rerun", "重新转写", SRC.rerun, "secondary", rec.rerunEnabled, "retry")
    : "";
  const more = button("detail.more", "更多操作", SRC.more, "icon", true, "more");
  const menu = rec.moreOpen
    ? el(
        "detail.more.menu",
        "从资料库移除",
        SRC.remove,
        button("detail.remove", "从资料库移除", SRC.remove, "quiet", true, "remove", "danger"),
        { class: "menu-pop" },
      )
    : "";
  const tip =
    tooltip === "more"
      ? el("detail.more.tooltip", tooltipText ?? "更多操作", SRC.moreTip, esc(tooltipText ?? "更多操作"), {
          class: "tip under-more",
        })
      : "";
  return el(
    "detail.actions",
    "操作",
    SRC.actions,
    `${exported}${button("detail.export", "导出 Markdown", SRC.export, "secondary", rec.exportEnabled, "export")}${rerun}<div style="position:relative">${more}${menu}${tip}</div>`,
    { class: "actions" },
  );
}

function renderRecording(rec: Recording, model: WindowModel): string {
  const info = [
    statusEl("detail.status", rec.status),
    rec.duration ? el("detail.duration", rec.duration, SRC.info, esc(rec.duration), { class: "small" }) : "",
    rec.status === "queued"
      ? renderLanguage(rec)
      : el("detail.language", LANG_NOTE[rec.language], SRC.languageNote, esc(LANG_NOTE[rec.language]), { class: "small" }),
    rec.took ? el("detail.took", `转写用时 ${rec.took}`, SRC.info, `转写用时 ${esc(rec.took)}`, { class: "small" }) : "",
    rec.modelName ? el("detail.model", rec.modelName, SRC.modelName, esc(rec.modelName), { class: "small" }) : "",
  ].join("");
  const head = [
    el("detail.path", rec.path, SRC.path, esc(rec.path), { class: "small path" }),
    el("detail.info", rec.name, SRC.info, info, { class: "info" }),
    rec.exportsPrevious
      ? el("detail.previous", "导出使用上次已保存的转写稿", SRC.previous, "导出使用上次已保存的转写稿", { class: "small" })
      : "",
    rec.missing ? renderAlert("detail.missing", "warn", rec.missing.title, rec.missing.body, SRC.missing) : "",
    rec.failure ? renderAlert("detail.failure", "bad", rec.failure.title, rec.failure.body, SRC.failure) : "",
    rec.waitingForModel
      ? el("detail.waiting", "模型准备完成后自动开始", SRC.waiting, "模型准备完成后自动开始", { class: "small" })
      : "",
    rec.progress ? renderProgress(rec) : "",
  ].join("");
  const segments = rec.segments.map(renderSegment).join("");
  const empty = rec.segments.length
    ? ""
    : el("transcript.empty", rec.emptyCopy ?? "", SRC.transcript, esc(rec.emptyCopy ?? ""), {
        class: "small",
        extra: 'style="padding:16px 0"',
      });
  const writing = rec.writing
    ? el(
        "transcript.writing",
        "正在转写后续内容…",
        SRC.live,
        `${el("transcript.writing.icon", "正在转写后续内容…", SRC.writing, `<i class="a"></i><i class="b"></i>`, { class: "pen" })}${el("transcript.writing.label", "正在转写后续内容…", SRC.live, "正在转写后续内容…", { class: "small" })}`,
        { class: "live" },
      )
    : "";
  const scrolled = rec.segments.length > 8;
  const thumb = scrolled
    ? el("transcript.scrollbar", "转写稿滚动条", SRC.scrollbar, "", {
        class: "thumb",
        extra: 'style="top:40px;height:84px"',
      })
    : "";
  const transcript = el(
    "transcript",
    "转写稿",
    SRC.transcript,
    `<div class="${scrolled ? "reading scroll-shift" : "reading"}">${empty}${segments}${writing}</div>${thumb}`,
    { class: "transcript" },
  );
  const modeLabel = rec.playback.mode === "playing" ? "正在播放" : rec.playback.mode === "paused" ? "已暂停" : "播放";
  const playLabel = rec.playback.mode === "playing" ? "暂停" : "播放";
  const play = el(
    "playback.play",
    playLabel,
    SRC.play,
    icon(rec.playback.mode === "playing" ? "pause" : "play", 16),
    {
      class: rec.playback.enabled ? "play" : "play is-disabled",
      extra: 'role="button"',
    },
  );
  const playback = el(
    "playback",
    modeLabel,
    SRC.playback,
    `<div class="play-side">${play}${el("playback.state", modeLabel, SRC.playback, esc(modeLabel), { extra: rec.playback.enabled ? "" : 'style="color:var(--disabled)"' })}${rec.playback.missingNote ? el("playback.missing", "原录音文件缺失，无法播放", SRC.playback, "原录音文件缺失，无法播放", { class: "small" }) : ""}${rec.playback.time ? el("playback.time", rec.playback.time, SRC.playback, esc(rec.playback.time), { class: "small mono" }) : ""}</div>${rec.playback.jump ? el("playback.jump", "点击分段跳转", SRC.playback, "点击分段跳转", { class: "small" }) : ""}`,
    { class: "playback" },
  );
  return el(
    "detail",
    rec.name,
    SRC.detail,
    `<div class="detail-head">${head}</div><div class="rule">${el("transcript.heading", "转写稿", SRC.heading, "转写稿", { class: "heading" })}${rec.hint ? el("transcript.hint", rec.hint, SRC.heading, esc(rec.hint), { class: "small" }) : ""}</div>${transcript}${playback}`,
    { class: "detail" },
  );
}

function renderEmpty(model: WindowModel, belowBar: boolean): string {
  if (model.view.kind === "unselected") {
    return el(
      "empty",
      "选择左边的一条录音",
      SRC.empty,
      `${el("empty.mark", "标记", SRC.empty, mark(72, "var(--mark-faint)", "var(--mark-lines-faint)", false))}${el("empty.prompt", "选择左边的一条录音", SRC.empty, "选择左边的一条录音", { class: "small" })}`,
      { class: "empty-pane plain" },
    );
  }
  const choosing = model.view.kind === "choose-model";
  const cards = (["large", "turbo"] as const)
    .map((key) => {
      const spec = MODELS[key];
      const id = `first-model.${spec.id}`;
      return el(
        id,
        spec.full,
        SRC.cards,
        `<div class="name-line">${el(`${id}.name`, spec.full, SRC.modelName, esc(spec.full))}${el(`${id}.tag`, spec.short, SRC.tag, spec.short, { class: "tag" })}</div>${el(`${id}.note`, spec.note, SRC.cardNote, esc(spec.note), { class: "formats" })}${el(`${id}.size`, `下载 ${spec.size}`, SRC.cards, `下载 ${spec.size}`, { class: "small size" })}`,
        { class: "model-card", extra: 'role="button"' },
      );
    })
    .join("");
  const middle = choosing
    ? `${el("empty.lead", "先选一个转写模型，点一下就开始下载", SRC.cards, "先选一个转写模型，点一下就开始下载")}<div class="cards">${cards}</div>${el("empty.after", "模型下载完成后就能加入录音", SRC.cards, "模型下载完成后就能加入录音", { class: "formats" })}`
    : `${el("empty.lead", model.canAdd ? "拖入录音，或选择本机文件" : "模型下载完成后，拖入录音或选择本机文件", SRC.welcome, model.canAdd ? "拖入录音，或选择本机文件" : "模型下载完成后，拖入录音或选择本机文件")}${button("empty.choose", "选择录音", SRC.choose, "primary", model.canAdd)}${el("empty.formats", "M4A · MP3 · WAV · FLAC · AAC", SRC.welcome, "M4A · MP3 · WAV · FLAC · AAC", { class: "formats" })}`;
  const inner = el(
    "empty.hero",
    "把录音变成文字",
    SRC.welcome,
    `${el("empty.mark", "标记", SRC.welcome, mark(132, "var(--mark)", "var(--mark-lines)", choosing || model.view.kind === "ready-empty"))}${el("empty.title", "把录音变成文字", SRC.welcome, "把录音变成文字", { class: "empty-title" })}${middle}${el("empty.local", "录音与转写稿保留在本机", SRC.welcome, `${icon("lock", 14)}<span class="small">录音与转写稿保留在本机</span>`, { class: "keep" })}`,
    { class: "empty-inner" },
  );
  return el(
    "empty",
    "把录音变成文字",
    SRC.empty,
    el("empty.drop", model.dragOver ? "拖入落点" : "投放区域", SRC.empty, inner, {
      class: model.dragOver ? "drop is-hot" : "drop",
    }),
    { class: `empty-pane${belowBar ? " below" : ""}` },
  );
}

function renderSheet(sheet: NonNullable<WindowModel["sheet"]>): string {
  return el(
    "dialog.scrim",
    sheet.title,
    sheet.src,
    el(
      "dialog",
      sheet.title,
      sheet.src,
      `${el("dialog.title", sheet.title, sheet.src, esc(sheet.title), { tag: "h3" })}${el("dialog.body", sheet.body, sheet.src, esc(sheet.body), { tag: "p" })}<div class="sheet-actions">${el("dialog.cancel", sheet.cancel, sheet.src, esc(sheet.cancel), { class: "sys-btn", extra: 'role="button"' })}${el("dialog.ok", sheet.ok, sheet.src, esc(sheet.ok), { class: "sys-btn default", extra: 'role="button"' })}</div>`,
      { class: "sheet" },
    ),
    { class: "sheet-scrim" },
  );
}

export function renderWindow(model: WindowModel, opts?: { scrolled?: boolean }): string {
  setFocus(model.focus);
  const count = model.groups.reduce((sum, group) => sum + group.rows.length, 0);
  const below = Boolean(model.download || model.notice);
  const filename =
    model.view.kind === "recording"
      ? el("detail.filename", model.view.rec.name, SRC.filename, esc(model.view.rec.name), { class: "filename" })
      : "";
  const actions =
    model.view.kind === "recording" ? renderActions(model.view.rec, model.tooltip, model.tooltipText) : "";
  const mainBody =
    model.view.kind === "recording" ? renderRecording(model.view.rec, model) : renderEmpty(model, below);
  const fileTip =
    model.tooltip === "filename"
      ? el("library.filename.tooltip", model.tooltipText ?? "", SRC.rowTip, esc(model.tooltipText ?? ""), {
          class: "tip",
          extra: 'style="top:168px;left:28px"',
        })
      : "";
  const inner = `${renderSidebar(model, count, Boolean(opts?.scrolled))}${el(
    "main",
    "详情",
    SRC.header,
    `${el("detail.header", filename ? "详情头部" : "拖动窗口", SRC.header, `${filename}${actions}`, { class: "pane-header" })}${model.download ? renderDownload(model.download) : ""}${model.notice ? renderNotice(model.notice) : ""}${mainBody}`,
    { class: model.dragOver ? "main is-dragover" : "main" },
  )}${fileTip}${model.sheet ? renderSheet(model.sheet) : ""}`;
  return el("window", "Visper", SRC.window, inner, {
    class: `product${model.theme === "dark" ? " theme-dark" : ""}${model.narrow ? " narrow" : ""}`,
    extra: `style="width:${model.width}px;height:${model.height}px"`,
  });
}
