# Visper 设计画布

A static canvas for talking about Visper’s interface. Every frame is live HTML drawn from the GPUI implementation in `src/ui/`, not a screenshot. Open the site, pan around, and click a control to copy a reference you can paste into a review.

## Run

```bash
cd design-canvas
npm install
npm run dev
```

`npm run build` writes `element-index.json` and a static site in `dist/`.

## Vercel

Create a project with **Root Directory** `design-canvas`. The included `vercel.json` sets:

| Setting | Value |
| --- | --- |
| Build command | `npm run build` |
| Output directory | `dist` |

No environment variables. The site is static.

## How to move around

- Scroll pans. Hold Space, or switch to 平移 (H), and drag. Middle-drag always pans.
- ⌘+scroll, Ctrl+scroll, and a trackpad pinch zoom toward the cursor.
- 选择 (V) is the default. Hover outlines a control; click copies its reference.
- 适应 fits the whole canvas. 100% (or the `1` key) returns to actual size. `0` fits.
- A frame’s expand button zooms to that frame. The hash keeps the frame id and the camera, so a link can point at one:

  `#f=8-completed-playing&z=1.000&x=0.0&y=0.0`

  A bare `#8-completed-playing` also works.

- PLAY restarts that frame’s motion (the welcome mark, or the writing indicator while a transcript is growing).
- Download saves that frame as a self-contained HTML file.

## Element references

A click copies one line:

```text
[visper-design] frame=8-completed-playing > el=playback.play | label="暂停" | src=src/ui/render.rs:343
```

| Field | Meaning |
| --- | --- |
| `frame` | Stable frame id (`design-canvas/element-index.json` lists them). |
| `el` | Stable id, unique inside that frame. The same control keeps the same id in every state. |
| `label` | The words on screen. |
| `src` | `file:line` in this repo for the code that draws it. |

Ids are short ascii paths: `library.add`, `detail.export`, `language.zh`, `transcript.segment.000036.body`, `model.large-v3`. They are not the GPUI element ids (those are sometimes Chinese, like `language-自动`).

`element-index.json` is the machine-readable copy of every id. `npm run build` regenerates it. When a pasted reference comes back, look up `frame` + `el` in that file.

## Adding or updating a frame

The app’s screens live in `src/frames.ts`. Each one is a `WindowModel` (or a menu, dialog, or the flow diagram) passed to `renderWindow` in `src/render/window.ts`.

1. Add a frame in `defineFrames()`. Give it a new `id` (`12-queued-first`) and `number`. Reuse an existing model and override only the state that changed: status, segments, whether export is enabled, the notice string.
2. Dark mode is generated. Any frame with `themeable: true` gets a twin in 深色模式 (`id` plus `-dark`, number plus `d`). System menus and file panels are not twinned; they follow the operating system.
3. Put visible copy in the strings the Rust code already uses (`src/ui/render.rs`, `src/ui.rs`, `src/menu.rs`, `src/library.rs`). If a pixel size or a system chrome detail is not in the source, set `assumption` on the frame. The caption shows it in the amber note.
4. New controls need a stable id in the renderer (`el("detail.export", …)` in `src/render/window.ts`) and a `src` line in `src/srcmap.ts`.
5. Run `npm run build`. That refreshes `element-index.json` and fails typecheck only when you run `npx tsc --noEmit` (the Vite build itself bundles with esbuild).

Sample recordings and sentences come from the debug previews in `src/ui/preview.rs`. Durations and the “还需约 2 分钟” estimate follow `Progress` in `src/library.rs`.

## What the frames are

Light frames are numbered in `src/frames.ts`. They cover first launch and model download, the empty library, drag-and-drop, every library status (queued, transcribing, completed, failed, cancelled, missing), playback, export, the model popover, focus, tooltips, notices, the menu bar, the About panel, and the system file and confirmation dialogs. 深色模式 repeats each in-app window.

Traffic lights are the system controls. The app only sets their position, `(16, 18)`, in `src/ui/render.rs`. The 12px circles are the macOS size, noted once here rather than on every frame.
