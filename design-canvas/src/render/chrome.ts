import { el, esc } from "../catalog";
import { SRC } from "../srcmap";

type Item = { id: string; label: string; kbd?: string; system?: boolean; sep?: boolean };

const MENUS: Record<string, { label: string; left: number; items: Item[] }> = {
  app: {
    label: "Visper",
    left: 48,
    items: [
      { id: "about", label: "关于 Visper" },
      { sep: true, id: "sep-1", label: "" },
      { id: "services", label: "服务", system: true },
      { sep: true, id: "sep-2", label: "" },
      { id: "hide", label: "隐藏 Visper", kbd: "⌘H" },
      { id: "hide-others", label: "隐藏其他", kbd: "⌥⌘H" },
      { id: "show-all", label: "全部显示" },
      { sep: true, id: "sep-3", label: "" },
      { id: "quit", label: "退出 Visper", kbd: "⌘Q" },
    ],
  },
  file: {
    label: "文件",
    left: 108,
    items: [
      { id: "add", label: "加入录音…", kbd: "⌘O" },
      { id: "export", label: "导出 Markdown…" },
      { sep: true, id: "sep-1", label: "" },
      { id: "close", label: "关闭窗口", kbd: "⌘W" },
    ],
  },
  edit: {
    label: "编辑",
    left: 156,
    items: [{ id: "copy", label: "拷贝", kbd: "⌘C" }],
  },
  window: {
    label: "窗口",
    left: 204,
    items: [
      { id: "minimize", label: "最小化", kbd: "⌘M" },
      { id: "zoom", label: "缩放" },
      { sep: true, id: "sep-1", label: "" },
      { id: "fullscreen", label: "进入全屏幕", system: true },
      { sep: true, id: "sep-2", label: "" },
      { id: "window-list", label: "Visper", system: true },
    ],
  },
  help: {
    label: "帮助",
    left: 252,
    items: [
      { id: "search", label: "搜索", system: true },
      { sep: true, id: "sep-1", label: "" },
      { id: "home", label: "Visper 项目主页" },
    ],
  },
};

function item(menu: string, entry: Item): string {
  if (entry.sep) {
    return el(`menu.${menu}.${entry.id}`, "分隔线", SRC.menu, "<i></i>", { class: "menu-sep" });
  }
  return el(
    `menu.${menu}.${entry.id}`,
    entry.label,
    entry.system ? "src/menu.rs:78" : SRC.menu,
    `<span>${esc(entry.label)}</span>${entry.kbd ? `<span class="kbd">${esc(entry.kbd)}</span>` : ""}`,
    { class: entry.system ? "menu-item is-system" : "menu-item" },
  );
}

export function renderMenu(open: keyof typeof MENUS): string {
  const bar = (Object.keys(MENUS) as (keyof typeof MENUS)[])
    .map((id) =>
      el(`menu.bar.${id}`, MENUS[id].label, SRC.menu, esc(MENUS[id].label), {
        extra: open === id ? 'style="font-weight:700"' : "",
      }),
    )
    .join("");
  const menu = MENUS[open];
  const drop = el(
    `menu.${open}`,
    menu.label,
    SRC.menu,
    menu.items.map((entry) => item(open, entry)).join(""),
    { class: "menu-drop", extra: `style="left:${menu.left}px"` },
  );
  return el(
    "menubar",
    "菜单栏",
    SRC.menu,
    `<div class="menubar">${el("menu.apple", "苹果菜单", "src/menu.rs:78", "", { class: "apple" })}${bar}</div>${drop}`,
    { class: "desktop", extra: 'style="width:860px;height:420px;position:relative"' },
  );
}

export function renderAbout(): string {
  return el(
    "about",
    "关于 Visper",
    SRC.about,
    el(
      "about.panel",
      "Visper",
      SRC.about,
      `${el("about.icon", "Visper", SRC.about, `<img src="./app-icon-512.png" alt="" width="72" height="72">`)}${el("about.name", "Visper", SRC.about, "Visper", { tag: "h3" })}${el("about.version", "版本 0.1.0", SRC.about, "版本 0.1.0", { tag: "p" })}${el("about.copyright", "© 2026 Liu Jun. MIT or Apache-2.0.", SRC.copyright, "© 2026 Liu Jun. MIT or Apache-2.0.", { tag: "p" })}`,
      { class: "about-panel" },
    ),
    { class: "desktop", extra: 'style="width:640px;height:420px"' },
  );
}

export function renderOpenPanel(kind: "add" | "export"): string {
  const prompt = kind === "add" ? "加入" : "导出";
  const src = kind === "add" ? SRC.pick : SRC.exportAsk;
  const files =
    kind === "add"
      ? ["产品讨论.m4a", "访谈记录.wav", "课程录音.mp3", "笔记.txt"]
      : ["文稿", "下载", "录音"];
  const list = files
    .map((name, index) =>
      el(`panel.file.${index}`, name, src, esc(name), {
        class: index === 0 ? "file on" : "file",
        extra: index === 0 ? 'style="background:#e7e7e7"' : "",
      }),
    )
    .join("");
  return el(
    "panel",
    prompt,
    src,
    el(
      "panel.window",
      prompt,
      src,
      `<div class="open-title">${el("panel.title", prompt, src, esc(prompt))}<div class="lights" style="left:16px;top:20px" aria-hidden="true"><i></i><i></i><i></i></div></div><div class="open-body"><div class="open-side">${["最近使用", "桌面", "文稿", "下载", "录音"].map((name, index) => el(`panel.place.${index}`, name, src, esc(name), { class: index === 4 ? "on" : "" })).join("")}</div><div class="open-files">${list}</div></div><div class="open-foot">${el("panel.cancel", "取消", src, "取消", { class: "sys-btn", extra: 'role="button"' })}${el("panel.confirm", prompt, src, esc(prompt), { class: "sys-btn default", extra: 'role="button"' })}</div>`,
      { class: "open-panel" },
    ),
    { class: "desktop", extra: 'style="width:860px;height:560px"' },
  );
}
