use super::motion::Motion;
use super::{PlaybackState, Store, VisperWindow};
use crate::theme::{Palette, metrics as m, mix, tint};
use gpui::prelude::*;
use gpui::{
    Animation, AnimationExt, App, Bounds, Context, Div, ExternalPaths, FontWeight, HighlightStyle,
    IntoElement, MouseButton, ParentElement, Pixels, Point, Render, SharedString, Stateful, Styled,
    StyledText, Svg, TitlebarOptions, Window, WindowBounds, WindowControlArea, WindowHandle,
    WindowOptions, deferred, div, ease_in_out, ease_out_quint, point, px, rgb, size, svg,
};
use std::ops::Range;
use std::time::Duration;

#[derive(Clone, Copy)]
struct Layout {
    narrow: bool,
    sidebar: f32,
    padding: f32,
}

impl Layout {
    fn for_window(window: &Window) -> Self {
        let narrow = window.viewport_size().width < px(1024.0);
        Self {
            narrow,
            sidebar: if narrow {
                m::NARROW_SIDEBAR
            } else {
                m::SIDEBAR
            },
            padding: if narrow {
                m::NARROW_CONTENT
            } else {
                m::CONTENT
            },
        }
    }
}

fn icon(name: &str, color: u32, width: f32) -> Svg {
    svg()
        .path(format!("icons/{name}.svg"))
        .size(px(width))
        .flex_none()
        .text_color(rgb(color))
}

/// The Mark's artwork is 910 x 520, and its two lines start 70% of the way across.
const MARK_ASPECT: f32 = 520.0 / 910.0;
const MARK_LINES_FROM: f32 = 0.7;

/// The Mark's box; its vee and lines are stacked inside, as GPUI tints a
/// whole svg one colour.
fn mark_frame(width: f32) -> Div {
    div()
        .relative()
        .flex_none()
        .w(px(width))
        .h(px(width * MARK_ASPECT))
}

fn mark_vee(width: f32, color: u32) -> Div {
    div().absolute().top_0().left_0().child(
        svg()
            .path("icons/mark-vee.svg")
            .w(px(width))
            .h(px(width * MARK_ASPECT))
            .text_color(rgb(color)),
    )
}

/// The lines sit in a clipping window as wide as they are, so narrowing it
/// writes them out from the left.
fn mark_lines(width: f32, color: u32) -> Div {
    let from = width * MARK_LINES_FROM;
    div()
        .absolute()
        .top_0()
        .left(px(from))
        .w(px(width - from))
        .h(px(width * MARK_ASPECT))
        .overflow_hidden()
        .child(
            svg()
                .path("icons/mark-lines.svg")
                .absolute()
                .top_0()
                .left(px(-from))
                .w(px(width))
                .h(px(width * MARK_ASPECT))
                .text_color(rgb(color)),
        )
}

fn mark(width: f32, vee: u32, lines: u32) -> Div {
    mark_frame(width)
        .child(mark_lines(width, lines))
        .child(mark_vee(width, vee))
}

/// The Mark as the first launch greets with it: the vee fades in, then the
/// lines are written out after it. Plays once, when the screen appears.
fn welcome_mark(width: f32, palette: Palette) -> Div {
    const VEE: Duration = Duration::from_millis(360);
    const LINES_AFTER: Duration = Duration::from_millis(280);
    const LINES: Duration = Duration::from_millis(520);
    let total = LINES_AFTER + LINES;
    let written = width * (1.0 - MARK_LINES_FROM);
    mark_frame(width)
        .child(mark_lines(width, palette.mark_lines).with_animation(
            "mark-lines",
            Animation::new(total),
            move |lines, delta| {
                let t =
                    (delta * total.as_secs_f32() - LINES_AFTER.as_secs_f32()) / LINES.as_secs_f32();
                lines.w(px(written * ease_out_quint()(t.clamp(0.0, 1.0))))
            },
        ))
        .child(mark_vee(width, palette.mark).with_animation(
            "mark-vee",
            Animation::new(VEE).with_easing(ease_out_quint()),
            |vee, delta| vee.opacity(delta),
        ))
}

/// Two strokes of a pen: the bars write out, hold, then fade, the second
/// bar a beat behind the first.
fn writing(palette: Palette) -> Div {
    const CYCLE: Duration = Duration::from_millis(1400);
    const BEHIND: f32 = 0.18 / 1.4;
    let stroke = |id: &'static str, width: f32, behind: f32, color: u32| {
        div()
            .h(px(3.0))
            .w(px(0.0))
            .rounded_full()
            .bg(rgb(color))
            .with_animation(id, Animation::new(CYCLE).repeat(), move |bar, delta| {
                let phase = (delta - behind).rem_euclid(1.0);
                let grown = ease_in_out((phase / 0.45).min(1.0));
                let fade = ((1.0 - phase) / 0.3).clamp(0.0, 1.0);
                bar.w(px(width * grown)).opacity(fade)
            })
    };
    div()
        .w(px(14.0))
        .flex_none()
        .flex()
        .flex_col()
        .gap(px(3.0))
        .child(stroke("writing-top", 14.0, 0.0, palette.accent))
        .child(
            div()
                .ml(px(3.0))
                .child(stroke("writing-bottom", 11.0, BEHIND, palette.mark_lines)),
        )
}

fn label(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(m::LABEL_SIZE))
        .line_height(px(m::LABEL_LINE))
        .child(text.into())
}

fn small(text: impl Into<SharedString>, palette: Palette) -> Div {
    div()
        .text_size(px(m::SMALL_SIZE))
        .line_height(px(m::SMALL_LINE))
        .text_color(rgb(palette.secondary))
        .child(text.into())
}

fn heading(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(m::BODY_SIZE))
        .line_height(px(22.0))
        .font_weight(FontWeight::SEMIBOLD)
        .child(text.into())
}

#[derive(Clone, Copy)]
enum Emphasis {
    Primary,
    Secondary,
    Quiet,
}

/// Where a keyboard focus ring sits: around a free-standing control, or
/// inside one packed among its neighbours.
#[derive(Clone, Copy)]
enum Ring {
    Outside,
    Inside,
}

/// What every control needs to draw its feedback.
#[derive(Clone)]
struct Ui {
    palette: Palette,
    motion: Motion,
}

impl Ui {
    /// Pointer, focus handle, hover tracking and, when Tab put it in focus,
    /// a ring of even width whose corners follow the control's own.
    fn interactive(
        &self,
        control: Stateful<Div>,
        id: &SharedString,
        radius: f32,
        ring: Ring,
        cx: &mut App,
    ) -> Stateful<Div> {
        let handle = self.motion.handle(id, cx);
        let control = control
            .relative()
            .cursor_pointer()
            .track_focus(&handle)
            .on_hover(self.motion.on_hover(id.clone()));
        if !self.motion.ring(id) {
            return control;
        }
        let (offset, radius) = match ring {
            Ring::Outside => (-4.0, radius + 4.0),
            Ring::Inside => (0.0, radius),
        };
        control.child(
            div()
                .absolute()
                .top(px(offset))
                .left(px(offset))
                .right(px(offset))
                .bottom(px(offset))
                .rounded(px(radius))
                .border_2()
                .border_color(rgb(self.palette.accent)),
        )
    }
}

fn button(
    id: &str,
    text: &'static str,
    glyph: Option<&str>,
    emphasis: Emphasis,
    enabled: bool,
    ui: &Ui,
    cx: &mut App,
) -> Stateful<Div> {
    let palette = ui.palette;
    let id = SharedString::from(id.to_owned());
    let mut control = div()
        .id(id.clone())
        .h(px(m::BUTTON))
        .pl(px(if glyph.is_some() { 10.0 } else { 12.0 }))
        .pr(px(if matches!(emphasis, Emphasis::Primary) {
            14.0
        } else {
            12.0
        }))
        .gap(px(6.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(m::BUTTON_RADIUS))
        .text_size(px(m::LABEL_SIZE))
        .line_height(px(m::LABEL_LINE))
        .font_weight(FontWeight::MEDIUM)
        .border_1();
    let foreground = if !enabled {
        palette.disabled
    } else if matches!(emphasis, Emphasis::Primary) {
        palette.on_accent
    } else {
        palette.ink
    };
    if let Some(glyph) = glyph {
        control = control.child(icon(glyph, foreground, 16.0));
    }
    control = control.child(text).text_color(rgb(foreground));
    if !enabled {
        return control.border_color(rgb(palette.line));
    }
    let level = ui.motion.hover(&id);
    control = ui.interactive(control, &id, m::BUTTON_RADIUS, Ring::Outside, cx);
    match emphasis {
        Emphasis::Primary => {
            let fill = mix(palette.accent, palette.accent_hover, level);
            control
                .bg(rgb(fill))
                .border_color(rgb(fill))
                .shadow(lift(0.10))
                .active(move |style| {
                    style
                        .bg(rgb(palette.accent_pressed))
                        .border_color(rgb(palette.accent_pressed))
                })
        }
        Emphasis::Secondary => control
            .bg(rgb(mix(palette.raised, palette.ink, 0.04 * level)))
            .border_color(rgb(mix(palette.line, palette.ink, 0.06 * level)))
            .shadow(lift(0.05))
            .active(move |style| style.bg(rgb(mix(palette.raised, palette.ink, 0.08)))),
        Emphasis::Quiet => control
            .bg(tint(palette.ink, 0.06 * level))
            .border_color(gpui::transparent_black())
            .active(move |style| style.bg(tint(palette.ink, 0.11))),
    }
}

/// A square button holding only its glyph, named by a tooltip.
fn icon_button(id: &str, glyph: &str, name: &'static str, ui: &Ui, cx: &mut App) -> Stateful<Div> {
    let palette = ui.palette;
    let id = SharedString::from(id.to_owned());
    let level = ui.motion.hover(&id);
    let control = div()
        .id(id.clone())
        .size(px(m::BUTTON))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(m::BUTTON_RADIUS))
        .bg(tint(palette.ink, 0.06 * level))
        .active(move |style| style.bg(tint(palette.ink, 0.11)))
        .child(icon(
            glyph,
            mix(palette.secondary, palette.ink, level),
            16.0,
        ))
        .tooltip(move |_, cx| {
            cx.new(|_| Tooltip {
                text: name.into(),
                palette,
            })
            .into()
        });
    ui.interactive(control, &id, m::BUTTON_RADIUS, Ring::Outside, cx)
}

fn play_button(playing: bool, enabled: bool, ui: &Ui, cx: &mut App) -> Stateful<Div> {
    let palette = ui.palette;
    let id = SharedString::from("play");
    let glyph = if playing { "pause" } else { "play" };
    let control = div()
        .id(id.clone())
        .size(px(36.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full();
    if !enabled {
        return control
            .border_1()
            .border_color(rgb(palette.line))
            .child(icon(glyph, palette.disabled, 16.0));
    }
    let fill = mix(palette.accent, palette.accent_hover, ui.motion.hover(&id));
    ui.interactive(control, &id, 18.0, Ring::Outside, cx)
        .bg(rgb(fill))
        .shadow(lift(0.10))
        .active(move |style| style.bg(rgb(palette.accent_pressed)))
        .child(icon(glyph, palette.on_accent, 16.0))
}

/// A one-pixel drop shadow that lifts a control off the page.
fn lift(alpha: f32) -> Vec<gpui::BoxShadow> {
    vec![gpui::BoxShadow {
        color: gpui::hsla(0.0, 0.0, 0.0, alpha),
        offset: point(px(0.0), px(1.0)),
        blur_radius: px(1.0),
        spread_radius: px(0.0),
    }]
}

fn progress_bar(fraction: f32, palette: Palette) -> Div {
    div()
        .h(px(4.0))
        .w_full()
        .rounded_full()
        .overflow_hidden()
        .bg(rgb(palette.line))
        .child(
            div()
                .h_full()
                .w(gpui::relative(fraction.clamp(0.0, 1.0)))
                .rounded_full()
                .bg(rgb(palette.accent)),
        )
}

/// A pair is ordered in reading order: segment, then UTF-8 offset.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
struct TextPosition {
    segment: usize,
    byte: usize,
}

#[derive(Default)]
pub(super) struct TextSelection {
    anchor: Option<TextPosition>,
    head: Option<TextPosition>,
    origin: Option<Point<Pixels>>,
    selecting: bool,
    dragged: bool,
}

impl TextSelection {
    fn range(&self, segment: usize, text: &str) -> Option<Range<usize>> {
        let (a, b) = (self.anchor?, self.head?);
        let (start, end) = if a <= b { (a, b) } else { (b, a) };
        if segment < start.segment || segment > end.segment {
            return None;
        }
        let boundary = |mut byte: usize| {
            byte = byte.min(text.len());
            while !text.is_char_boundary(byte) {
                byte -= 1;
            }
            byte
        };
        let from = if segment == start.segment {
            boundary(start.byte)
        } else {
            0
        };
        let to = if segment == end.segment {
            boundary(end.byte)
        } else {
            text.len()
        };
        (from < to).then_some(from..to)
    }

    pub(super) fn text(&self, segments: &[visper::Segment]) -> String {
        segments
            .iter()
            .enumerate()
            .filter_map(|(index, segment)| {
                self.range(index, &segment.text)
                    .map(|range| segment.text[range].to_owned())
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

impl Render for VisperWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let appearance = window.appearance();
        #[cfg(debug_assertions)]
        let appearance = self.preview_appearance.unwrap_or(appearance);
        let palette = Palette::from_appearance(appearance);
        self.motion.begin_frame(window);
        let ui = Ui {
            palette,
            motion: self.motion.clone(),
        };
        let pointer = self.motion.clone();
        let timing = self.timing();
        let layout = Layout::for_window(window);
        let library = self.library.clone();
        let model_open = self.model_open;
        let playback = library
            .selected()
            .and_then(|recording| self.player.state(&recording.path));
        #[cfg(debug_assertions)]
        let playback = self.preview_playback.or(playback);
        let view = SelectionView {
            just_exported: library
                .selected()
                .is_some_and(|recording| self.exported.as_ref() == Some(&recording.path)),
            more_open: self.more_open,
            playback,
            selection: self.selection.clone(),
            focus: self.selection_focus.clone(),
            scroll: self.transcript_scroll.clone(),
        };
        div()
            .size_full()
            .flex()
            .bg(rgb(palette.main))
            .text_color(rgb(palette.ink))
            .font_family(m::UI_FONT)
            .text_size(px(m::LABEL_SIZE))
            .line_height(px(m::LABEL_LINE))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                let stroke = &event.keystroke;
                if stroke.key == "escape" && this.model_open {
                    this.model_open = false;
                    cx.stop_propagation();
                    cx.notify();
                }
                if stroke.key == "tab" {
                    this.motion.use_keyboard(true);
                    if stroke.modifiers.shift {
                        window.focus_prev();
                    } else {
                        window.focus_next();
                    }
                    window.prevent_default();
                    cx.stop_propagation();
                }
            }))
            .capture_any_mouse_down(move |_, window, _| {
                if pointer.use_keyboard(false) {
                    window.refresh();
                }
            })
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
                for path in paths.paths() {
                    this.admit(path.clone(), cx);
                }
            }))
            .child(sidebar(
                &library,
                model_open,
                &ui,
                layout,
                timing.as_ref(),
                cx,
            ))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .bg(rgb(palette.main))
                    .drag_over::<ExternalPaths>(move |style, _, _, _| {
                        style.bg(rgb(palette.playing))
                    })
                    .child(pane_header(library.selected(), &view, &ui, layout, cx))
                    .children(download_strip(library.download(), &ui, layout, cx))
                    .children(self.notice.clone().map(|text| notice_bar(text, &ui, cx)))
                    .child(detail(&library, &view, &ui, layout, timing.as_ref(), cx)),
            )
    }
}

/// The strip across the top of the main column, where the window is dragged
/// by; it names the selected Recording and holds its actions.
fn pane_header(
    recording: Option<&visper::Recording>,
    view: &SelectionView,
    ui: &Ui,
    layout: Layout,
    cx: &mut Context<VisperWindow>,
) -> Div {
    div()
        .h(px(m::TITLEBAR))
        .flex_none()
        .pl(px(layout.padding))
        .pr(px(16.0))
        .gap(px(16.0))
        .flex()
        .items_center()
        .window_control_area(WindowControlArea::Drag)
        .children(recording.map(|recording| {
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(14.0))
                .line_height(px(20.0))
                .font_weight(FontWeight::SEMIBOLD)
                .truncate()
                .child(recording.file_name())
        }))
        .children(recording.map(|recording| detail_actions(recording, view, ui, cx)))
}

fn download_strip(
    download: &visper::Download,
    ui: &Ui,
    layout: Layout,
    cx: &mut Context<VisperWindow>,
) -> Option<Div> {
    let palette = ui.palette;
    let strip = div()
        .min_h(px(if layout.narrow { 80.0 } else { 64.0 }))
        .flex_none()
        .px(px(24.0))
        .py(px(12.0))
        .gap(px(16.0))
        .flex()
        .items_center()
        .border_b_1()
        .border_color(rgb(palette.line));
    match download {
        visper::Download::Running {
            model,
            received,
            total,
        } => {
            let fraction = if *total > 0 {
                *received as f32 / *total as f32
            } else {
                0.0
            };
            let amount = if *total > 0 {
                format!(
                    "{:.1} / {:.1} GB · {}%",
                    *received as f64 / 1e9,
                    *total as f64 / 1e9,
                    (fraction * 100.0).floor()
                )
            } else {
                "正在连接…".into()
            };
            let copy = div()
                .flex_none()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    label(format!("正在准备 {}", model.full_name()))
                        .font_weight(FontWeight::MEDIUM),
                )
                .child(small("首次使用需下载一次，之后可离线转写", palette));
            let progress = div()
                .flex_1()
                .min_w(px(100.0))
                .child(progress_bar(fraction, palette));
            Some(
                strip
                    .child(icon("download", palette.accent, 24.0))
                    .child(if layout.narrow {
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(8.0))
                            .child(copy)
                            .child(progress)
                    } else {
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(24.0))
                            .child(copy)
                            .child(progress)
                    })
                    .child(small(amount, palette).flex_none().font_family(m::MONO_FONT)),
            )
        }
        visper::Download::Failed { error, .. } => Some(
            strip
                .child(icon("warning", palette.danger, 20.0))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            label("模型下载失败")
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(palette.danger)),
                        )
                        .child(small(error.clone(), palette)),
                )
                .child(
                    button(
                        "retry-download",
                        "重试",
                        Some("retry"),
                        Emphasis::Secondary,
                        true,
                        ui,
                        cx,
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.retry_download(cx))),
                ),
        ),
        visper::Download::Idle => None,
    }
}

fn notice_bar(text: String, ui: &Ui, cx: &mut Context<VisperWindow>) -> Div {
    let palette = ui.palette;
    div()
        .px(px(24.0))
        .py(px(8.0))
        .flex_none()
        .flex()
        .items_center()
        .gap(px(12.0))
        .bg(rgb(palette.warning_surface))
        .border_b_1()
        .border_color(rgb(palette.line))
        .child(icon("warning", palette.warning, 16.0))
        .child(small(text, palette).flex_1().min_w_0())
        .child(
            button(
                "dismiss-notice",
                "关闭",
                None,
                Emphasis::Quiet,
                true,
                ui,
                cx,
            )
            .on_click(cx.listener(|this, _, _, cx| {
                this.notice = None;
                cx.notify();
            })),
        )
}

fn status(recording: &visper::Recording, palette: Palette) -> Div {
    let (glyph, color) = match recording.status {
        visper::Status::Transcribing => ("waiting", palette.accent),
        visper::Status::Queued => ("waiting", palette.secondary),
        visper::Status::Completed => ("check", palette.accent),
        visper::Status::Failed => ("warning", palette.danger),
        visper::Status::Cancelled => ("cancelled", palette.secondary),
    };
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(icon(glyph, color, 14.0))
        .child(small(recording.status.label(), palette))
}

struct Tooltip {
    text: SharedString,
    palette: Palette,
}
impl Render for Tooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(px(8.0))
            .py(px(3.0))
            .max_w(px(400.0))
            .rounded(px(6.0))
            .bg(rgb(self.palette.ink))
            .text_size(px(m::SMALL_SIZE))
            .line_height(px(m::SMALL_LINE))
            .text_color(rgb(self.palette.main))
            .child(self.text.clone())
    }
}

/// The sidebar's way to add a Recording: a row, not a button, since it sits
/// among the Library's own rows.
fn add_recording(ui: &Ui, cx: &mut Context<VisperWindow>) -> Stateful<Div> {
    let palette = ui.palette;
    let id = SharedString::from("add-recording");
    let level = ui.motion.hover(&id);
    let row = div()
        .id(id.clone())
        .w_full()
        .h(px(36.0))
        .px(px(10.0))
        .gap(px(10.0))
        .rounded(px(m::ROW_RADIUS))
        .flex()
        .items_center()
        .text_size(px(m::LABEL_SIZE))
        .line_height(px(m::LABEL_LINE))
        .font_weight(FontWeight::MEDIUM)
        .bg(tint(palette.ink, 0.05 * level))
        .active(move |style| style.bg(tint(palette.ink, 0.09)))
        .child(
            div()
                .size(px(22.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(rgb(mix(palette.accent, palette.accent_hover, level)))
                .child(icon("plus-bold", palette.on_accent, 12.0)),
        )
        .child("加入录音");
    ui.interactive(row, &id, m::ROW_RADIUS, Ring::Inside, cx)
        .on_click(cx.listener(|this, _, _, cx| this.pick_file(cx)))
}

fn sidebar(
    library: &visper::Library,
    model_open: bool,
    ui: &Ui,
    layout: Layout,
    timing: Option<&Timing>,
    cx: &mut Context<VisperWindow>,
) -> Div {
    let palette = ui.palette;
    let mut list = div()
        .id("library-list")
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .px(px(12.0))
        .pb(px(16.0))
        .flex()
        .flex_col()
        .gap(px(2.0));
    if library.is_empty() {
        list = list.child(
            small("还没有录音", palette)
                .mt(px(32.0))
                .w_full()
                .text_center(),
        );
    }
    for (group, recordings) in library.groups() {
        if recordings.is_empty() {
            continue;
        }
        list = list.child(
            div()
                .mt(px(10.0))
                .mb(px(4.0))
                .px(px(8.0))
                .text_size(px(11.0))
                .line_height(px(16.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(palette.tertiary))
                .child(group.label()),
        );
        for recording in recordings {
            let selected = library
                .selected()
                .is_some_and(|current| current.path == recording.path);
            let path = recording.path.clone();
            let name: SharedString = recording.file_name().into();
            let tooltip_name = name.clone();
            let id = SharedString::from(format!("recording-{}", path.display()));
            let level = ui.motion.hover(&id);
            let mut row = div()
                .id(id.clone())
                .w_full()
                .h(px(if recording.status == visper::Status::Transcribing {
                    76.0
                } else {
                    60.0
                }))
                .flex_none()
                .px(px(12.0))
                .py(px(10.0))
                .rounded(px(m::ROW_RADIUS))
                .flex()
                .items_start()
                .gap(px(10.0))
                .tooltip(move |_, cx| {
                    cx.new(|_| Tooltip {
                        text: tooltip_name.clone(),
                        palette,
                    })
                    .into()
                })
                .on_click(cx.listener(move |this, _, _, cx| this.select(&path, cx)))
                .child(
                    icon(
                        "audio",
                        if selected {
                            palette.accent
                        } else {
                            palette.secondary
                        },
                        20.0,
                    )
                    .mt(px(1.0)),
                );
            row = if selected {
                row.bg(rgb(palette.selected))
            } else {
                row.bg(tint(palette.ink, 0.05 * level))
                    .active(move |style| style.bg(tint(palette.ink, 0.09)))
            };
            let mut copy = div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    label(name)
                        .w(px(layout.sidebar - 80.0))
                        .font_weight(FontWeight::MEDIUM)
                        .truncate(),
                );
            let mut meta = div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(status(recording, palette));
            let running = timing.filter(|timing| timing.path == recording.path);
            if recording.is_missing() {
                meta = meta.child(small("· 文件缺失", palette).text_color(rgb(palette.warning)));
            } else if let Some(timing) = running {
                meta = meta.child(
                    small(format!("· {}", remaining_label(timing)), palette)
                        .min_w_0()
                        .truncate(),
                );
            } else if let Some(duration) = recording.duration_ms() {
                let mut text = format!("· {}", duration_label(duration));
                if let Some(took) = recording.transcription_ms() {
                    text.push_str(&format!(" · 用时 {}", duration_label(took)));
                }
                meta = meta.child(small(text, palette).min_w_0().truncate());
            }
            copy = copy.child(meta);
            if let Some(progress) = recording.progress() {
                copy = copy.child(
                    div()
                        .mt(px(4.0))
                        .child(progress_bar(progress.fraction(), palette)),
                );
            }
            list =
                list.child(ui.interactive(row.child(copy), &id, m::ROW_RADIUS, Ring::Inside, cx));
        }
    }
    div()
        .w(px(layout.sidebar))
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .bg(rgb(palette.sidebar))
        .border_r_1()
        .border_color(rgb(palette.line))
        .child(
            div()
                .h(px(m::TITLEBAR))
                .flex_none()
                .window_control_area(WindowControlArea::Drag),
        )
        .child(
            div()
                .px(px(12.0))
                .pb(px(8.0))
                .flex_none()
                .child(add_recording(ui, cx)),
        )
        .child(
            div()
                .px(px(20.0))
                .pt(px(8.0))
                .flex_none()
                .flex()
                .items_baseline()
                .justify_between()
                .child(small("资料库", palette).font_weight(FontWeight::MEDIUM))
                .child(
                    small(format!("{} 份录音", library.recordings().len()), palette)
                        .text_color(rgb(palette.tertiary)),
                ),
        )
        .child(list)
        .child(model_footer(library, model_open, ui, cx))
}

/// The Model on each line of the popover and the footer.
fn model_selector(model: visper::Model) -> &'static str {
    match model {
        visper::Model::LargeV3 => "model-large-v3",
        visper::Model::LargeV3Turbo => "model-large-v3-turbo",
    }
}

fn footer_label_selector(model: Option<visper::Model>) -> &'static str {
    match model {
        Some(visper::Model::LargeV3) => "footer-label-large-v3",
        Some(visper::Model::LargeV3Turbo) => "footer-label-large-v3-turbo",
        None => "footer-label-none",
    }
}

fn model_blurb(model: visper::Model) -> &'static str {
    match model {
        visper::Model::LargeV3 => "更准，较慢",
        visper::Model::LargeV3Turbo => "更快，略不准",
    }
}

/// 11 / 16 Medium on a faint wash, the same over a row, a selected row and
/// a selected option.
fn model_tag(model: visper::Model, palette: Palette) -> Div {
    div()
        .flex_none()
        .px(px(6.0))
        .rounded(px(4.0))
        .bg(tint(palette.ink, 0.08))
        .whitespace_nowrap()
        .text_size(px(11.0))
        .line_height(px(16.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(palette.secondary))
        .child(model.short_name())
}

/// The 48 px strip at the bottom of the Library: one button showing the
/// current Model, with the Model popover hanging above it.
fn model_footer(
    library: &visper::Library,
    open: bool,
    ui: &Ui,
    cx: &mut Context<VisperWindow>,
) -> Div {
    let palette = ui.palette;
    let id = SharedString::from("model-footer");
    let level = ui.motion.hover(&id);
    let current = library.model();
    let button = div()
        .id(id.clone())
        .debug_selector(|| "model-footer".into())
        .w_full()
        .h(px(47.0))
        .px(px(8.0))
        .gap(px(8.0))
        .rounded(px(m::ROW_RADIUS))
        .flex()
        .items_center()
        .bg(tint(palette.ink, 0.06 * level))
        .active(move |style| style.bg(tint(palette.ink, 0.10)))
        .child(icon("lock", palette.secondary, 14.0))
        .child(small("本机转写", palette).flex_none().whitespace_nowrap())
        .child(div().flex_1())
        .child(
            match current {
                Some(model) => small(model.full_name(), palette)
                    .text_color(rgb(palette.ink))
                    .font_weight(FontWeight::MEDIUM),
                None => small("未选模型", palette),
            }
            .flex_none()
            .whitespace_nowrap()
            .debug_selector(move || footer_label_selector(current).into()),
        )
        .child(icon("chevron-up", palette.secondary, 10.0));
    let button = ui
        .interactive(button, &id, m::ROW_RADIUS, Ring::Inside, cx)
        .on_click(cx.listener(|this, _, _, cx| {
            this.model_open = !this.model_open;
            cx.notify();
        }));
    let mut footer = div()
        .relative()
        .flex_none()
        .w_full()
        .px(px(12.0))
        .child(div().mx(px(8.0)).h(px(1.0)).bg(rgb(palette.line)))
        .child(button);
    if open {
        // The popover hangs above this anchor, so a press on an option is
        // outside the anchor's box; close on mouse up, as the 更多 menu does,
        // so the option's click runs first.
        footer = footer
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.model_open = false;
                    cx.notify();
                }),
            )
            .child(model_popover(library, ui, cx));
    }
    footer
}

fn model_popover(library: &visper::Library, ui: &Ui, cx: &mut Context<VisperWindow>) -> Div {
    let palette = ui.palette;
    let mut popover = div()
        .occlude()
        .absolute()
        .bottom(px(48.0))
        .left(px(12.0))
        .right(px(12.0))
        .p(px(6.0))
        .rounded(px(10.0))
        .border_1()
        .border_color(rgb(palette.line))
        .bg(rgb(palette.raised))
        .shadow(vec![gpui::BoxShadow {
            color: gpui::hsla(0.0, 0.0, 0.0, 0.08),
            offset: point(px(0.0), px(4.0)),
            blur_radius: px(12.0),
            spread_radius: px(0.0),
        }])
        .child(
            div()
                .pt(px(6.0))
                .px(px(10.0))
                .pb(px(4.0))
                .text_size(px(11.0))
                .line_height(px(16.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(palette.tertiary))
                .child("转写模型"),
        );
    for model in visper::Model::ALL {
        let id = SharedString::from(model_selector(model));
        let level = ui.motion.hover(&id);
        let chosen = library.model() == Some(model);
        let state = if library.is_on_disk(model) {
            "已下载".to_owned()
        } else if matches!(
            library.download(),
            visper::Download::Running { model: running, .. } if *running == model
        ) {
            "下载中".to_owned()
        } else {
            format!("需下载 {:.1} GB", model.bytes() as f64 / 1e9)
        };
        let option = div()
            .id(id.clone())
            .debug_selector(move || model_selector(model).into())
            .w_full()
            .px(px(10.0))
            .py(px(8.0))
            .gap(px(8.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .bg(if chosen {
                rgb(mix(palette.playing, palette.ink, 0.06 * level))
            } else {
                tint(palette.ink, 0.06 * level)
            })
            .active(move |style| style.bg(tint(palette.ink, 0.10)))
            .child(
                div()
                    .w(px(16.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .children(chosen.then(|| icon("tick", palette.accent, 14.0))),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                label(model.full_name())
                                    .flex_none()
                                    .whitespace_nowrap()
                                    .font_weight(FontWeight::MEDIUM),
                            )
                            .child(model_tag(model, palette)),
                    )
                    .child(small(format!("{} · {state}", model_blurb(model)), palette).truncate()),
            );
        popover = popover.child(
            ui.interactive(option, &id, 6.0, Ring::Inside, cx)
                .on_click(cx.listener(move |this, _, _, cx| this.choose_model(model, cx))),
        );
    }
    popover.child(
        div()
            .mt(px(4.0))
            .px(px(10.0))
            .pt(px(6.0))
            .pb(px(4.0))
            .border_t_1()
            .border_color(rgb(palette.line))
            .text_size(px(11.0))
            .line_height(px(16.0))
            .text_color(rgb(palette.tertiary))
            .truncate()
            .child("只影响之后开始的转写，已有转写稿不变"),
    )
}

struct SelectionView {
    just_exported: bool,
    more_open: bool,
    playback: Option<PlaybackState>,
    selection: std::rc::Rc<std::cell::RefCell<TextSelection>>,
    focus: gpui::FocusHandle,
    scroll: gpui::ScrollHandle,
}

fn detail(
    library: &visper::Library,
    view: &SelectionView,
    ui: &Ui,
    layout: Layout,
    timing: Option<&Timing>,
    cx: &mut Context<VisperWindow>,
) -> gpui::AnyElement {
    let palette = ui.palette;
    let Some(recording) = library.selected() else {
        return empty_detail(library.is_empty(), ui, layout, cx).into_any_element();
    };
    let header = div()
        .px(px(layout.padding))
        .pt(px(4.0))
        .pb(px(20.0))
        .flex_none()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(small(recording.path.display().to_string(), palette))
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(12.0))
                .child(status(recording, palette))
                .children(
                    recording
                        .duration_ms()
                        .map(|duration| small(duration_label(duration), palette)),
                )
                .child(if recording.status == visper::Status::Queued {
                    language_control(recording, ui, cx).into_any_element()
                } else {
                    small(language_note(recording.language), palette).into_any_element()
                })
                .children(
                    recording
                        .transcription_ms()
                        .map(|took| small(format!("转写用时 {}", duration_label(took)), palette)),
                ),
        )
        .children(
            recording
                .exports_previous_transcript()
                .then(|| small("导出使用上次已保存的转写稿", palette)),
        )
        .children(missing_file(recording, palette))
        .children(failure(recording, palette))
        .children(
            (recording.status == visper::Status::Queued && !library.is_ready())
                .then(|| small("模型准备完成后自动开始", palette)),
        )
        .children(
            (recording.status == visper::Status::Transcribing)
                .then(|| transcription_progress(recording, timing, ui, cx)),
        );
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(header)
        .child(
            div()
                .mx(px(layout.padding))
                .flex_none()
                .border_t_1()
                .border_color(rgb(palette.line))
                .pt(px(20.0))
                .pb(px(12.0))
                .flex()
                .items_baseline()
                .justify_between()
                .child(heading("转写稿"))
                .child(small(
                    if recording.status == visper::Status::Transcribing {
                        "分段实时显示"
                    } else if recording.can_play() {
                        "点击分段，从此处回听"
                    } else {
                        ""
                    },
                    palette,
                )),
        )
        .child(transcript(recording, view, ui, layout, cx))
        .child(playback_bar(recording, view, ui, layout, cx))
        .into_any_element()
}

fn detail_actions(
    recording: &visper::Recording,
    view: &SelectionView,
    ui: &Ui,
    cx: &mut Context<VisperWindow>,
) -> Div {
    let palette = ui.palette;
    let mut export = button(
        "export",
        "导出 Markdown",
        Some("export"),
        Emphasis::Secondary,
        recording.can_export(),
        ui,
        cx,
    );
    if recording.can_export() {
        let path = recording.path.clone();
        export = export.on_click(cx.listener(move |this, _, _, cx| this.export(&path, cx)));
    }
    let mut rerun = button(
        "rerun",
        "重新转写",
        Some("retry"),
        Emphasis::Secondary,
        recording.can_rerun(),
        ui,
        cx,
    );
    if recording.can_rerun() {
        let path = recording.path.clone();
        rerun = rerun.on_click(cx.listener(move |this, _, _, cx| this.rerun(&path, cx)));
    }
    let mut more = div().relative().flex_none().child(
        icon_button("more", "more", "更多操作", ui, cx)
            .debug_selector(|| "more-actions".into())
            .on_click(cx.listener(|this, _, _, cx| {
                this.more_open = !this.more_open;
                cx.notify();
            })),
    );
    if view.more_open {
        let path = recording.path.clone();
        // The menu hangs outside this anchor, so a press on 从资料库移除 is
        // outside the anchor's box. GPUI fires the click on mouse up. Closing
        // on mouse down unmounts the item before that, and the press does
        // nothing. Close on mouse up instead: the click still runs in that
        // event, and the menu unmounts only after it.
        more = more.on_mouse_up_out(
            MouseButton::Left,
            cx.listener(|this, _, _, cx| {
                this.more_open = false;
                cx.notify();
            }),
        );
        let menu = div()
            .occlude()
            .absolute()
            .top(px(40.0))
            .right_0()
            .w(px(196.0))
            .p(px(4.0))
            .rounded(px(10.0))
            .border_1()
            .border_color(rgb(palette.line))
            .bg(rgb(palette.raised))
            .shadow(vec![gpui::BoxShadow {
                color: gpui::hsla(0.0, 0.0, 0.0, 0.08),
                offset: point(px(0.0), px(4.0)),
                blur_radius: px(12.0),
                spread_radius: px(0.0),
            }])
            .child(
                button(
                    "remove",
                    "从资料库移除",
                    Some("remove"),
                    Emphasis::Quiet,
                    true,
                    ui,
                    cx,
                )
                .debug_selector(|| "remove-recording".into())
                .w_full()
                .justify_start()
                .text_color(rgb(palette.danger))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.more_open = false;
                    this.remove(&path, window, cx);
                })),
            );
        more = more.child(deferred(menu).priority(1));
    }
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(8.0))
        .flex_none()
        .children(view.just_exported.then(|| small("已导出", palette)))
        .child(export)
        .children((recording.status.group() == visper::Group::Finished).then_some(rerun))
        .child(more)
}

fn language_control(recording: &visper::Recording, ui: &Ui, cx: &mut Context<VisperWindow>) -> Div {
    const WIDTH: f32 = 216.0;
    const INSET: f32 = 2.0;
    let palette = ui.palette;
    let languages = [
        visper::Language::Auto,
        visper::Language::Chinese,
        visper::Language::English,
    ];
    let chosen = languages
        .iter()
        .position(|language| *language == recording.language)
        .unwrap_or(0);
    let option_width = (WIDTH - INSET * 2.0) / languages.len() as f32;
    let mut control = div()
        .relative()
        .h(px(32.0))
        .w(px(WIDTH))
        .flex_none()
        .flex()
        .p(px(INSET))
        .rounded(px(m::BUTTON_RADIUS))
        .bg(rgb(palette.track))
        .child(
            div()
                .absolute()
                .top(px(INSET))
                .bottom(px(INSET))
                .left(px(INSET + option_width * ui.motion.pill(chosen)))
                .w(px(option_width))
                .rounded(px(6.0))
                .bg(rgb(palette.raised))
                .shadow(vec![
                    gpui::BoxShadow {
                        color: gpui::hsla(0.0, 0.0, 0.0, 0.12),
                        offset: point(px(0.0), px(1.0)),
                        blur_radius: px(2.0),
                        spread_radius: px(0.0),
                    },
                    gpui::BoxShadow {
                        color: gpui::hsla(0.0, 0.0, 0.0, 0.06),
                        offset: point(px(0.0), px(0.0)),
                        blur_radius: px(0.0),
                        spread_radius: px(0.5),
                    },
                ]),
        );
    for (index, language) in languages.into_iter().enumerate() {
        let id = SharedString::from(format!("language-{}", language.label()));
        let foreground = if index == chosen {
            palette.ink
        } else {
            mix(palette.secondary, palette.ink, ui.motion.hover(&id))
        };
        let option = div()
            .id(id.clone())
            .flex_1()
            .min_w_0()
            .h_full()
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(13.0))
            .line_height(px(20.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(foreground))
            .child(language.label());
        let path = recording.path.clone();
        let motion = ui.motion.clone();
        let option = ui
            .interactive(option, &id, 6.0, Ring::Inside, cx)
            .on_click(cx.listener(move |this, _, _, cx| {
                if index != chosen {
                    motion.move_pill(chosen);
                }
                this.set_language(&path, language, cx)
            }));
        control = control.child(option);
    }
    control
}

fn transcription_progress(
    recording: &visper::Recording,
    timing: Option<&Timing>,
    ui: &Ui,
    cx: &mut Context<VisperWindow>,
) -> Div {
    let palette = ui.palette;
    let path = recording.path.clone();
    let progress = recording.progress();
    let timing = timing.filter(|timing| timing.path == recording.path);
    let (headline, settled) = match (progress, timing) {
        (None, _) => ("正在准备录音…".to_owned(), false),
        (Some(_), Some(timing)) if timing.remaining_ms.is_some() => (remaining_label(timing), true),
        _ => ("正在估算剩余时间…".to_owned(), false),
    };
    let finish_at = timing
        .and_then(|timing| timing.remaining_ms)
        .filter(|remaining| *remaining >= 60_000)
        .map(|remaining| format!("预计 {} 完成", local_time_after(remaining)));
    let mut title = div().flex().items_baseline().gap(px(8.0)).min_w_0().child(
        div()
            .text_size(px(m::BODY_SIZE))
            .line_height(px(22.0))
            .font_weight(if settled {
                FontWeight::SEMIBOLD
            } else {
                FontWeight::MEDIUM
            })
            .text_color(rgb(if settled {
                palette.ink
            } else {
                palette.secondary
            }))
            .child(headline),
    );
    if let Some(finish_at) = finish_at {
        title = title.child(small(finish_at, palette));
    }
    let mut stats = div().flex().flex_wrap().gap_x(px(20.0)).gap_y(px(4.0));
    if let Some(timing) = timing {
        stats = stats.child(stat("已用时", duration_label(timing.elapsed_ms), palette));
    }
    if let Some(progress) = progress {
        let hours = progress.duration_ms >= 3_600_000;
        stats = stats.child(stat(
            "已处理",
            format!(
                "{} / {}",
                time_label(progress.processed_ms, hours),
                time_label(progress.duration_ms, hours)
            ),
            palette,
        ));
    }
    if let Some(speed) = timing.and_then(|timing| timing.speed) {
        stats = stats.child(stat("速度", format!("{speed:.1}×"), palette));
    }
    let mut block = div()
        .p(px(16.0))
        .pt(px(14.0))
        .pb(px(12.0))
        .rounded(px(10.0))
        .border_1()
        .border_color(rgb(palette.line))
        .bg(rgb(palette.raised))
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(16.0))
                .child(title)
                .child(
                    button(
                        "cancel",
                        "取消转写",
                        None,
                        Emphasis::Secondary,
                        true,
                        ui,
                        cx,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| this.cancel(&path, cx))),
                ),
        );
    if let Some(progress) = progress {
        block = block.child(
            div()
                .flex()
                .items_center()
                .gap(px(12.0))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(progress_bar(progress.fraction(), palette)),
                )
                .child(
                    small(
                        format!("{}%", (progress.fraction() * 100.0).floor()),
                        palette,
                    )
                    .w(px(36.0))
                    .flex_none()
                    .text_right()
                    .font_family(m::MONO_FONT),
                ),
        );
    }
    block.child(stats)
}

/// A label and its figure, the figure in the text colour.
fn stat(name: &'static str, value: String, palette: Palette) -> Div {
    div().flex().gap(px(4.0)).child(small(name, palette)).child(
        small(value, palette)
            .font_family(m::MONO_FONT)
            .text_color(rgb(palette.ink)),
    )
}

/// How the running Transcription is timed, measured by the window.
pub(super) struct Timing {
    pub path: std::path::PathBuf,
    pub elapsed_ms: u64,
    /// None while the pace is not yet steady enough to estimate.
    pub remaining_ms: Option<u64>,
    pub speed: Option<f64>,
}

fn remaining_label(timing: &Timing) -> String {
    let Some(remaining) = timing.remaining_ms else {
        return "正在估算…".into();
    };
    let minutes = remaining.div_ceil(60_000);
    match minutes {
        0 | 1 if remaining < 60_000 => "还需不到 1 分钟".into(),
        0..60 => format!("还需约 {minutes} 分钟"),
        _ if minutes % 60 == 0 => format!("还需约 {} 小时", minutes / 60),
        _ => format!("还需约 {} 小时 {} 分钟", minutes / 60, minutes % 60),
    }
}

fn language_note(language: visper::Language) -> &'static str {
    match language {
        visper::Language::Auto => "自动识别语言",
        visper::Language::Chinese => "中文",
        visper::Language::English => "英文",
    }
}

/// The local wall-clock time `ms` from now, as HH:MM.
fn local_time_after(ms: u64) -> String {
    let at = std::time::SystemTime::now() + std::time::Duration::from_millis(ms);
    let seconds = at
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs()) as libc::time_t;
    // SAFETY: localtime_r and localtime_s read `seconds` and write only into
    // `local`.
    let mut local: libc::tm = unsafe { std::mem::zeroed() };
    #[cfg(unix)]
    let _ = unsafe { libc::localtime_r(&seconds, &mut local) };
    #[cfg(windows)]
    let _ = unsafe { libc::localtime_s(&mut local, &seconds) };
    format!("{:02}:{:02}", local.tm_hour, local.tm_min)
}

fn alert(
    title: &'static str,
    explanation: impl Into<SharedString>,
    foreground: u32,
    surface: u32,
    palette: Palette,
) -> Div {
    div()
        .p(px(16.0))
        .rounded(px(8.0))
        .bg(rgb(surface))
        .flex()
        .items_start()
        .gap(px(12.0))
        .child(icon("warning", foreground, 18.0).mt(px(1.0)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    label(title)
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(foreground)),
                )
                .child(small(explanation, palette)),
        )
}

fn missing_file(recording: &visper::Recording, palette: Palette) -> Option<Div> {
    if !recording.is_missing() {
        return None;
    }
    let explanation = if recording.can_export() {
        "转写稿已保留，可继续阅读和导出。"
    } else if recording.has_stored_transcript() {
        "空转写稿已保留，原录音文件缺失，无法播放或重新转写。"
    } else {
        "无法读取原录音文件，转写和播放不可用。"
    };
    Some(alert(
        "找不到原录音文件",
        explanation,
        palette.warning,
        palette.warning_surface,
        palette,
    ))
}

fn failure(recording: &visper::Recording, palette: Palette) -> Option<Div> {
    if recording.status != visper::Status::Failed {
        return None;
    }
    let mut explanation = recording.error().unwrap_or("转写未能完成。").to_owned();
    if recording.has_stored_transcript() {
        explanation.push_str("\n上次转写稿已保留。");
    }
    Some(alert(
        "转写失败",
        explanation,
        palette.danger,
        palette.danger_surface,
        palette,
    ))
}

fn empty_detail(empty: bool, ui: &Ui, layout: Layout, cx: &mut Context<VisperWindow>) -> Div {
    let palette = ui.palette;
    let pane = div()
        .flex_1()
        .min_h_0()
        .px(px(layout.padding))
        .pb(px(layout.padding))
        .flex()
        .flex_col();
    if !empty {
        return pane
            .items_center()
            .justify_center()
            .gap(px(16.0))
            .child(mark(72.0, palette.mark_faint, palette.mark_lines_faint))
            .child(small("选择左边的一条录音", palette));
    }
    let mut pick = button(
        "choose-recording",
        "选择录音",
        None,
        Emphasis::Primary,
        true,
        ui,
        cx,
    )
    .h(px(40.0))
    .px(px(20.0));
    pick = pick.on_click(cx.listener(|this, _, _, cx| this.pick_file(cx)));
    pane.child(
        div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .border_1()
            .border_dashed()
            .border_color(rgb(palette.line))
            .rounded(px(8.0))
            .drag_over::<ExternalPaths>(move |style, _, _, _| {
                style
                    .border_color(rgb(palette.accent))
                    .bg(rgb(palette.playing))
            })
            .child(
                div()
                    .max_w(px(440.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(16.0))
                    .pb(px(32.0))
                    .child(welcome_mark(132.0, palette))
                    .child(
                        div()
                            .mt(px(8.0))
                            .text_size(px(24.0))
                            .line_height(px(32.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("把录音变成文字"),
                    )
                    .child(label("拖入录音，或选择本机文件").text_color(rgb(palette.secondary)))
                    .child(div().mt(px(8.0)).child(pick))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .line_height(px(16.0))
                            .text_color(rgb(palette.tertiary))
                            .child("M4A · MP3 · WAV · FLAC · AAC"),
                    )
                    .child(
                        div()
                            .mt(px(32.0))
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(icon("lock", palette.secondary, 14.0))
                            .child(small("录音与转写稿保留在本机", palette)),
                    ),
            ),
    )
}

fn transcript(
    recording: &visper::Recording,
    view: &SelectionView,
    ui: &Ui,
    layout: Layout,
    cx: &mut Context<VisperWindow>,
) -> Stateful<Div> {
    let palette = ui.palette;
    let playing_segment = view
        .playback
        .and_then(|playback| recording.segment_at(playback.position_ms));
    let segments = recording.visible_segments();
    let selection = view.selection.clone();
    let focus = view.focus.clone();
    let scroll = view.scroll.clone();
    let mut content = div()
        .max_w(px(m::READING_WIDTH))
        .w_full()
        .flex()
        .flex_col()
        .pt(px(4.0))
        .pb(px(24.0));
    if segments.is_empty() {
        let copy = match recording.status {
            visper::Status::Queued => "还没有转写稿。",
            visper::Status::Transcribing => "正在转写，分段将在这里显示。",
            visper::Status::Completed => "录音中未得到可转写的文字。",
            visper::Status::Cancelled => "已取消，没有分段。",
            visper::Status::Failed => "这次转写没有保存分段。",
        };
        content = content.child(small(copy, palette).py(px(16.0)));
    }
    for (index, segment) in segments.iter().enumerate() {
        let highlights = selection.borrow().range(index, &segment.text).map(|range| {
            (
                range,
                HighlightStyle {
                    background_color: Some(rgb(palette.selected).into()),
                    ..Default::default()
                },
            )
        });
        let text = StyledText::new(segment.text.clone()).with_highlights(highlights);
        let text_layout = text.layout().clone();
        let start_layout = text_layout.clone();
        let move_layout = text_layout;
        let down_selection = selection.clone();
        let move_selection = selection.clone();
        let up_selection = selection.clone();
        let focus = focus.clone();
        let path = recording.path.clone();
        let start_ms = segment.start_ms;
        let can_play = recording.can_play();
        let hover_id = SharedString::from(format!("segment-{index}"));
        let level = ui.motion.hover(&hover_id);
        let playing = playing_segment == Some(index);
        let mut row = div()
            .id(("segment", index))
            .flex()
            .items_start()
            .gap(px(8.0))
            .px(px(10.0))
            .py(px(3.0))
            .rounded(px(m::ROW_RADIUS))
            .bg(tint(palette.ink, 0.04 * level))
            .on_hover(ui.motion.on_hover(hover_id))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |_, event: &gpui::MouseDownEvent, window, cx| {
                    let byte = start_layout
                        .index_for_position(event.position)
                        .unwrap_or_else(|index| index);
                    let position = TextPosition {
                        segment: index,
                        byte,
                    };
                    *down_selection.borrow_mut() = TextSelection {
                        anchor: Some(position),
                        head: Some(position),
                        origin: Some(event.position),
                        selecting: true,
                        dragged: false,
                    };
                    window.focus(&focus);
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(move |_, event: &gpui::MouseMoveEvent, _, cx| {
                if event.pressed_button != Some(MouseButton::Left) {
                    return;
                }
                let mut state = move_selection.borrow_mut();
                if !state.selecting {
                    return;
                }
                let byte = move_layout
                    .index_for_position(event.position)
                    .unwrap_or_else(|index| index);
                state.head = Some(TextPosition {
                    segment: index,
                    byte,
                });
                if let Some(origin) = state.origin {
                    state.dragged |= (event.position.x - origin.x).abs() > px(3.0)
                        || (event.position.y - origin.y).abs() > px(3.0);
                }
                cx.notify();
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    let play = {
                        let mut state = up_selection.borrow_mut();
                        let play = state.selecting && !state.dragged;
                        state.selecting = false;
                        play
                    };
                    if play && can_play {
                        this.play_from(&path, start_ms, cx);
                    } else {
                        cx.notify();
                    }
                }),
            );
        if playing {
            row = row.bg(rgb(palette.playing));
        }
        content = content.child(
            row.child(
                div()
                    .w(px(m::TIMESTAMP))
                    .flex_none()
                    .font_family(m::MONO_FONT)
                    .text_size(px(m::SMALL_SIZE))
                    .line_height(px(m::BODY_LINE))
                    .when(playing, |stamp| stamp.font_weight(FontWeight::SEMIBOLD))
                    .text_color(rgb(if playing {
                        palette.accent
                    } else {
                        mix(palette.secondary, palette.accent, level)
                    }))
                    .child(segment.start_label()),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .cursor_text()
                    .text_size(px(m::BODY_SIZE))
                    .line_height(px(m::BODY_LINE))
                    .child(text),
            ),
        );
    }
    if recording.status == visper::Status::Transcribing && !segments.is_empty() {
        content = content.child(
            div()
                .mt(px(8.0))
                .px(px(10.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(writing(palette))
                .child(small("正在转写后续内容…", palette)),
        );
    }
    div()
        .id("transcript")
        .flex_1()
        .min_h_0()
        .px(px(layout.padding))
        .overflow_y_scroll()
        .track_scroll(&scroll)
        .track_focus(&focus)
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|this, _, _, _| this.selection.borrow_mut().selecting = false),
        )
        .on_mouse_up_out(
            MouseButton::Left,
            cx.listener(|this, _, _, _| this.selection.borrow_mut().selecting = false),
        )
        .child(content)
}

fn playback_bar(
    recording: &visper::Recording,
    view: &SelectionView,
    ui: &Ui,
    layout: Layout,
    cx: &mut Context<VisperWindow>,
) -> Div {
    let palette = ui.palette;
    let playing = view.playback.is_some_and(|playback| playback.playing);
    let mut control = play_button(playing, recording.can_play(), ui, cx);
    if recording.can_play() {
        let path = recording.path.clone();
        control =
            control.on_click(cx.listener(move |this, _, _, cx| this.toggle_playback(&path, cx)));
    }
    let state = match view.playback {
        Some(playback) if playback.playing => "正在播放",
        Some(_) => "已暂停",
        None => "播放",
    };
    let mut left = div()
        .flex()
        .items_center()
        .gap(px(12.0))
        .child(control)
        .child(label(state).text_color(rgb(if recording.can_play() {
            palette.ink
        } else {
            palette.disabled
        })));
    if recording.is_missing() {
        left = left.child(small("原录音文件缺失，无法播放", palette));
    } else if let Some(playback) = view.playback {
        let total = playback.duration_ms.or(recording.duration_ms());
        let hours = total.unwrap_or(playback.position_ms) >= 3_600_000;
        let time = match total {
            Some(duration) => format!(
                "{} / {}",
                time_label(playback.position_ms, hours),
                time_label(duration, hours)
            ),
            None => time_label(playback.position_ms, hours),
        };
        left = left.child(small(time, palette).font_family(m::MONO_FONT));
    }
    div()
        .mx(px(layout.padding))
        .h(px(if layout.narrow { 72.0 } else { 64.0 }))
        .flex_none()
        .border_t_1()
        .border_color(rgb(palette.line))
        .flex()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .child(left)
        .children(
            (recording.can_play() && !recording.visible_segments().is_empty())
                .then(|| small("点击分段跳转", palette)),
        )
}

fn time_label(ms: u64, hours: bool) -> String {
    if hours {
        visper::clock(ms)
    } else {
        format!("{:02}:{:02}", ms / 60_000, ms / 1000 % 60)
    }
}

fn duration_label(ms: u64) -> String {
    time_label(ms, ms >= 3_600_000)
}

pub fn open(store: Store, support: std::path::PathBuf, cx: &mut App) -> WindowHandle<VisperWindow> {
    open_with(cx, move |cx| VisperWindow::new(store, support, cx))
}

pub(super) fn open_with(
    cx: &mut App,
    create: impl FnOnce(&mut Context<VisperWindow>) -> VisperWindow + 'static,
) -> WindowHandle<VisperWindow> {
    let bounds = Bounds::centered(None, size(px(m::WINDOW_WIDTH), px(m::WINDOW_HEIGHT)), cx);
    let window = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(m::MIN_WIDTH), px(m::MIN_HEIGHT))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Visper".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(16.0), px(18.0))),
                }),
                ..Default::default()
            },
            move |window, cx| {
                window.set_window_title("Visper");
                let view = cx.new(create);
                let observed = view.clone();
                let subscription = window.observe_window_appearance(move |_, cx| {
                    observed.update(cx, |_, cx| cx.notify());
                });
                view.update(cx, |this, _| this.appearance = Some(subscription));
                let closing = view.downgrade();
                window.on_window_should_close(cx, move |window, cx| {
                    closing
                        .update(cx, |this, cx| this.should_close(window, cx))
                        .unwrap_or(true)
                });
                view
            },
        )
        .expect("打开 Visper 窗口");
    cx.on_window_closed(|cx| cx.quit()).detach();
    cx.activate(true);
    window
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remaining(ms: Option<u64>) -> String {
        remaining_label(&Timing {
            path: "/Users/me/录音/周会.m4a".into(),
            elapsed_ms: 0,
            remaining_ms: ms,
            speed: None,
        })
    }

    #[test]
    fn remaining_time_reads_in_whole_minutes_rounded_up() {
        assert_eq!(remaining(None), "正在估算…");
        assert_eq!(remaining(Some(0)), "还需不到 1 分钟");
        assert_eq!(remaining(Some(59_999)), "还需不到 1 分钟");
        assert_eq!(remaining(Some(60_000)), "还需约 1 分钟");
        assert_eq!(remaining(Some(659_091)), "还需约 11 分钟");
        assert_eq!(remaining(Some(3_600_000)), "还需约 1 小时");
        assert_eq!(remaining(Some(4_200_000)), "还需约 1 小时 10 分钟");
    }

    #[test]
    fn reverse_drag_copies_chinese_and_english_across_segments() {
        let segments = vec![
            visper::Segment {
                start_ms: 0,
                end_ms: 1,
                text: "中文 English".into(),
            },
            visper::Segment {
                start_ms: 1,
                end_ms: 2,
                text: "API 与 Rust".into(),
            },
        ];
        let selection = TextSelection {
            anchor: Some(TextPosition {
                segment: 1,
                byte: 3,
            }),
            head: Some(TextPosition {
                segment: 0,
                byte: 3,
            }),
            ..Default::default()
        };
        assert_eq!(selection.text(&segments), "文 English\n\nAPI");
    }

    #[test]
    fn replaced_transcript_cannot_leave_selection_on_an_invalid_utf8_boundary() {
        let selection = TextSelection {
            anchor: Some(TextPosition {
                segment: 0,
                byte: 1,
            }),
            head: Some(TextPosition {
                segment: 0,
                byte: 100,
            }),
            ..Default::default()
        };
        assert_eq!(selection.range(0, "中文"), Some(0..6));
    }

    fn recording_count(library_file: &std::path::Path) -> usize {
        let saved = std::fs::read_to_string(library_file).unwrap();
        let saved: serde_json::Value = serde_json::from_str(&saved).unwrap();
        saved["recordings"].as_array().unwrap().len()
    }

    fn click(cx: &mut gpui::VisualTestContext, selector: &'static str) {
        let bounds = cx
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("{selector} was not on screen"));
        cx.simulate_click(bounds.center(), gpui::Modifiers::default());
    }

    /// The menu hangs below the title bar. Closing it on mouse down used to
    /// unmount 从资料库移除 before the click, so the press did nothing.
    #[gpui::test]
    fn remove_from_library_confirms_then_drops_the_recording(cx: &mut gpui::TestAppContext) {
        let dir = std::env::temp_dir().join(format!(
            "visper-remove-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let library_file = dir.join("library.json");
        let recording = "/tmp/visper-remove-test.m4a";
        std::fs::write(
            &library_file,
            serde_json::json!({
                "recordings": [{
                    "path": recording,
                    "status": "Completed",
                    "segments": [{"start_ms": 0, "end_ms": 1200, "text": "你好"}],
                    "transcript_stored": true,
                    "finished_order": 1
                }],
                "selected": recording,
                "finished_count": 1
            })
            .to_string(),
        )
        .unwrap();
        std::fs::write(
            crate::download::weight_path(&dir, visper::Model::LargeV3),
            b"weight",
        )
        .unwrap();

        let window = cx.update(|cx| {
            open(
                crate::store::Store::new(library_file.clone()),
                dir.clone(),
                cx,
            )
        });
        let cx = gpui::VisualTestContext::from_window(window.into(), cx).into_mut();

        click(cx, "more-actions");
        click(cx, "remove-recording");
        let (message, detail) = cx
            .pending_prompt()
            .expect("从资料库移除 should ask before dropping a stored transcript");
        assert_eq!(message, "从资料库移除");
        assert_eq!(detail, "已保存的转写稿将被移除，原录音文件会保留。");

        cx.simulate_prompt_answer("取消");
        cx.run_until_parked();
        assert!(!cx.has_pending_prompt());
        assert_eq!(recording_count(&library_file), 1);

        click(cx, "more-actions");
        click(cx, "remove-recording");
        cx.simulate_prompt_answer("移除");
        cx.run_until_parked();
        assert_eq!(recording_count(&library_file), 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[gpui::test]
    fn picking_the_other_model_from_the_footer_changes_the_label_and_the_saved_model(
        cx: &mut gpui::TestAppContext,
    ) {
        let dir = std::env::temp_dir().join(format!(
            "visper-model-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let library_file = dir.join("library.json");
        std::fs::write(&library_file, r#"{"recordings":[],"selected":null}"#).unwrap();
        // Both weights are stored, so choosing one starts no download.
        for model in visper::Model::ALL {
            std::fs::write(crate::download::weight_path(&dir, model), b"weight").unwrap();
        }
        let saved_model = || {
            let saved = std::fs::read_to_string(&library_file).unwrap();
            let saved: serde_json::Value = serde_json::from_str(&saved).unwrap();
            saved["model"].as_str().map(str::to_owned)
        };

        let window = cx.update(|cx| {
            open(
                crate::store::Store::new(library_file.clone()),
                dir.clone(),
                cx,
            )
        });
        let cx = gpui::VisualTestContext::from_window(window.into(), cx).into_mut();

        // GPUI keeps an element's debug bounds after it stops being drawn, so
        // only an element's first appearance is checked from them.
        let popover_open = |cx: &mut gpui::VisualTestContext| {
            window.read_with(cx, |view, _| view.model_open).unwrap()
        };

        // A library from before the choice comes up as Large v3.
        assert!(cx.debug_bounds("footer-label-large-v3").is_some());
        assert!(cx.debug_bounds("footer-label-large-v3-turbo").is_none());
        assert_eq!(saved_model().as_deref(), Some("LargeV3"));
        assert!(!popover_open(cx));

        click(cx, "model-footer");
        assert!(popover_open(cx));
        assert!(cx.debug_bounds("model-large-v3").is_some());
        assert!(cx.debug_bounds("model-large-v3-turbo").is_some());

        cx.simulate_keystrokes("escape");
        assert!(!popover_open(cx));

        click(cx, "model-footer");
        assert!(popover_open(cx));
        let option = cx.debug_bounds("model-large-v3-turbo").unwrap();
        cx.simulate_click(
            gpui::point(option.right() + gpui::px(200.0), option.center().y),
            gpui::Modifiers::default(),
        );
        assert!(!popover_open(cx));
        assert_eq!(saved_model().as_deref(), Some("LargeV3"));

        click(cx, "model-footer");
        click(cx, "model-large-v3-turbo");
        cx.run_until_parked();

        assert!(!popover_open(cx));
        assert!(cx.debug_bounds("footer-label-large-v3-turbo").is_some());
        assert_eq!(saved_model().as_deref(), Some("LargeV3Turbo"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
