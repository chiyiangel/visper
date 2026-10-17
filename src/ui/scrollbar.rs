//! The overlay scrollbar of the Transcript and the Library list (visual spec
//! §7.7). GPUI 0.2 scrolls with the wheel but draws no bar, so this paints a
//! thin thumb over the right edge of a scroll area, reads and sets the area's
//! `ScrollHandle`, and fades in and out on its own.
use super::motion::ease_out;
use crate::theme::{Palette, tint};
use gpui::prelude::*;
use gpui::{
    App, Bounds, DispatchPhase, Div, Hitbox, HitboxBehavior, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, ScrollHandle, SharedString, Stateful, Window, canvas,
    div, fill, point, px, size,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// The width of the strip that takes the pointer, inside the padding that
/// both scroll areas keep on their right.
const STRIP: f32 = 12.0;
/// The gap between the thumb and the window edge, and between the thumb and
/// either end of the strip.
const INSET: f32 = 2.0;
const THIN: f32 = 6.0;
const WIDE: f32 = 8.0;
const MIN_THUMB: f32 = 24.0;
/// How long the bar stays after the last scroll or the pointer leaving.
const HOLD: Duration = Duration::from_secs(1);
const FADE: Duration = Duration::from_millis(120);

/// Where the thumb sits in the strip, in pixels from the strip's top.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Thumb {
    top: f32,
    len: f32,
}

/// The thumb for a viewport of `viewport` px over content that scrolls
/// `max_scroll` px further, scrolled `scrolled` px down. `None` when the
/// content fits and there is nothing to show.
fn thumb(viewport: f32, max_scroll: f32, scrolled: f32) -> Option<Thumb> {
    if viewport <= 0.0 || max_scroll <= 0.5 {
        return None;
    }
    let track = (viewport - 2.0 * INSET).max(0.0);
    let content = viewport + max_scroll;
    let len = (track * viewport / content).clamp(MIN_THUMB.min(track), track);
    let progress = (scrolled / max_scroll).clamp(0.0, 1.0);
    Some(Thumb {
        top: INSET + (track - len) * progress,
        len,
    })
}

/// The scroll distance after the thumb, grabbed when the area was scrolled
/// `start` px, has moved `dy` px. The thumb's travel maps onto the whole
/// scroll range.
fn dragged(start: f32, dy: f32, viewport: f32, max_scroll: f32) -> f32 {
    let Some(grabbed) = thumb(viewport, max_scroll, 0.0) else {
        return start;
    };
    let travel = viewport - 2.0 * INSET - grabbed.len;
    if travel <= 0.0 {
        return start;
    }
    (start + dy * max_scroll / travel).clamp(0.0, max_scroll)
}

/// The scroll distance after a click on the track: one viewport toward it.
fn paged(scrolled: f32, viewport: f32, max_scroll: f32, down: bool) -> f32 {
    let to = if down {
        scrolled + viewport
    } else {
        scrolled - viewport
    };
    to.clamp(0.0, max_scroll)
}

/// Whether the bar should be on screen: while the pointer is over the area
/// or a drag is under way, and for a second after the last of them or the
/// last scroll.
fn wanted(pointer: bool, since_active: Option<Duration>) -> bool {
    pointer || since_active.is_some_and(|elapsed| elapsed < HOLD)
}

/// A level between 0 and 1 that eases toward its target, or jumps there when
/// motion is reduced.
#[derive(Clone, Copy)]
struct Fade {
    target: bool,
    from: f32,
    since: Instant,
}

impl Fade {
    fn at_rest(now: Instant) -> Self {
        Self {
            target: false,
            from: 0.0,
            since: now,
        }
    }

    fn level(&self, now: Instant, reduce_motion: bool) -> f32 {
        let to = if self.target { 1.0 } else { 0.0 };
        if reduce_motion {
            return to;
        }
        let t = ease_out(now.saturating_duration_since(self.since), FADE);
        self.from + (to - self.from) * t
    }

    fn set(&mut self, target: bool, now: Instant, reduce_motion: bool) {
        if target != self.target {
            self.from = self.level(now, reduce_motion);
            self.target = target;
            self.since = now;
        }
    }

    fn settled(&self, now: Instant, reduce_motion: bool) -> bool {
        reduce_motion || now.saturating_duration_since(self.since) >= FADE
    }
}

struct Drag {
    /// The pointer's y when the thumb was grabbed, and how far the area was
    /// scrolled then.
    from_y: Pixels,
    scrolled: f32,
}

struct State {
    /// A preview holds the bar on screen, hot or not, for review.
    #[cfg(debug_assertions)]
    pinned: Option<bool>,
    area_hover: bool,
    strip_hover: bool,
    /// Either of the two, as of the last frame.
    pointer: bool,
    drag: Option<Drag>,
    last_scrolled: Option<f32>,
    /// The last scroll, or the pointer arriving or leaving.
    last_active: Option<Instant>,
    shown: Fade,
    /// Thumb hovered or dragged: wider and darker.
    hot: Fade,
}

impl State {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            #[cfg(debug_assertions)]
            pinned: None,
            area_hover: false,
            strip_hover: false,
            pointer: false,
            drag: None,
            last_scrolled: None,
            last_active: None,
            shown: Fade::at_rest(now),
            hot: Fade::at_rest(now),
        }
    }

    /// The content fits: nothing is shown and nothing is held.
    fn clear(&mut self) {
        let now = Instant::now();
        self.drag = None;
        self.strip_hover = false;
        self.pointer = false;
        self.last_scrolled = None;
        self.last_active = None;
        self.shown = Fade::at_rest(now);
        self.hot = Fade::at_rest(now);
    }
}

/// What the bar of one scroll area remembers between frames. Cheap to clone;
/// every clone is the same bar.
#[derive(Clone)]
pub(super) struct Scrollbar {
    name: &'static str,
    state: Rc<RefCell<State>>,
}

impl Scrollbar {
    /// `name` is the debug selector of the bar's strip.
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            state: Rc::new(RefCell::new(State::new())),
        }
    }

    /// Keeps the bar on screen, drawn as if hovered when `hot`, so that a
    /// preview can show it without a pointer.
    #[cfg(debug_assertions)]
    pub fn pin(&self, hot: bool) {
        self.state.borrow_mut().pinned = Some(hot);
    }

    /// `area` scrolls vertically and fills its parent's height; the result
    /// is that area with the bar laid over its right edge, which takes no
    /// layout width. `handle` is how the bar reads and sets the scroll.
    pub fn wrap(&self, area: Stateful<Div>, handle: &ScrollHandle, palette: Palette) -> Div {
        let state = self.state.clone();
        let area = area
            .track_scroll(handle)
            .on_hover(move |hovered, window, _| {
                state.borrow_mut().area_hover = *hovered;
                window.refresh();
            });
        div()
            .relative()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(area)
            .child(self.strip(handle.clone(), palette))
    }

    fn strip(&self, handle: ScrollHandle, palette: Palette) -> Div {
        let state = self.state.clone();
        let name = SharedString::from(self.name);
        let prepaint_handle = handle.clone();
        div()
            .absolute()
            .top_0()
            .right_0()
            .bottom_0()
            .w(px(STRIP))
            .debug_selector(move || name.to_string())
            .child(
                canvas(
                    move |bounds, window, _| {
                        let (viewport, max_scroll, scrolled) = measure(&prepaint_handle);
                        thumb(viewport, max_scroll, scrolled).map(|_| {
                            window.insert_hitbox(bounds, HitboxBehavior::BlockMouseExceptScroll)
                        })
                    },
                    move |bounds, hitbox, window, _| {
                        let Some(hitbox) = hitbox else {
                            state.borrow_mut().clear();
                            return;
                        };
                        paint(&state, &handle, palette, bounds, &hitbox, window);
                    },
                )
                .size_full(),
            )
    }
}

/// The viewport height, how far the area can scroll, and how far it has.
fn measure(handle: &ScrollHandle) -> (f32, f32, f32) {
    (
        f32::from(handle.bounds().size.height),
        f32::from(handle.max_offset().height),
        -f32::from(handle.offset().y),
    )
}

fn set_scrolled(handle: &ScrollHandle, scrolled: f32) {
    handle.set_offset(point(handle.offset().x, px(-scrolled)));
}

fn paint(
    state: &Rc<RefCell<State>>,
    handle: &ScrollHandle,
    palette: Palette,
    bounds: Bounds<Pixels>,
    hitbox: &Hitbox,
    window: &mut Window,
) {
    let now = Instant::now();
    let reduce_motion = crate::macos::reduce_motion();
    let (viewport, max_scroll, scrolled) = measure(handle);
    let Some(thumb) = thumb(viewport, max_scroll, scrolled) else {
        return;
    };
    let (shown, hot, animating) = {
        let mut state = state.borrow_mut();
        if state
            .last_scrolled
            .is_some_and(|before| (before - scrolled).abs() > 0.5)
        {
            state.last_active = Some(now);
        }
        state.last_scrolled = Some(scrolled);
        state.strip_hover = hitbox.is_hovered(window);
        let pointer = state.area_hover || state.strip_hover || state.drag.is_some();
        #[cfg(debug_assertions)]
        let pointer = pointer || state.pinned.is_some();
        if pointer != state.pointer {
            state.pointer = pointer;
            state.last_active = Some(now);
        }
        let since_active = state
            .last_active
            .map(|active| now.saturating_duration_since(active));
        let hot = state.strip_hover || state.drag.is_some();
        #[cfg(debug_assertions)]
        let hot = hot || state.pinned == Some(true);
        state
            .shown
            .set(wanted(pointer, since_active), now, reduce_motion);
        state.hot.set(hot, now, reduce_motion);
        let holding = !pointer && since_active.is_some_and(|elapsed| elapsed < HOLD);
        (
            state.shown.level(now, reduce_motion),
            state.hot.level(now, reduce_motion),
            holding
                || !state.shown.settled(now, reduce_motion)
                || !state.hot.settled(now, reduce_motion),
        )
    };
    if animating {
        window.request_animation_frame();
    }
    if shown > 0.0 {
        let width = THIN + (WIDE - THIN) * hot;
        let opacity =
            (palette.scrollbar + (palette.scrollbar_hot - palette.scrollbar) * hot) * shown;
        let rect = Bounds::new(
            point(
                bounds.right() - px(INSET + width),
                bounds.top() + px(thumb.top),
            ),
            size(px(width), px(thumb.len)),
        );
        window.paint_quad(fill(rect, tint(palette.ink, opacity)).corner_radii(px(width / 2.0)));
    }
    listen(state, handle, bounds, hitbox, window);
}

fn listen(
    state: &Rc<RefCell<State>>,
    handle: &ScrollHandle,
    bounds: Bounds<Pixels>,
    hitbox: &Hitbox,
    window: &mut Window,
) {
    window.on_mouse_event({
        let (state, handle, hitbox) = (state.clone(), handle.clone(), hitbox.clone());
        move |event: &MouseDownEvent, phase, window: &mut Window, cx: &mut App| {
            if phase != DispatchPhase::Bubble
                || event.button != MouseButton::Left
                || !hitbox.is_hovered(window)
            {
                return;
            }
            let (viewport, max_scroll, scrolled) = measure(&handle);
            let Some(thumb) = thumb(viewport, max_scroll, scrolled) else {
                return;
            };
            let y = f32::from(event.position.y - bounds.top());
            if y >= thumb.top && y <= thumb.top + thumb.len {
                state.borrow_mut().drag = Some(Drag {
                    from_y: event.position.y,
                    scrolled,
                });
            } else {
                let down = y > thumb.top;
                set_scrolled(&handle, paged(scrolled, viewport, max_scroll, down));
            }
            window.refresh();
            cx.stop_propagation();
        }
    });
    window.on_mouse_event({
        let (state, handle, hitbox) = (state.clone(), handle.clone(), hitbox.clone());
        move |event: &MouseMoveEvent, phase, window: &mut Window, _: &mut App| {
            if phase != DispatchPhase::Capture {
                return;
            }
            let mut state = state.borrow_mut();
            if let Some(drag) = &state.drag {
                if event.pressed_button == Some(MouseButton::Left) {
                    let (viewport, max_scroll, _) = measure(&handle);
                    let dy = f32::from(event.position.y - drag.from_y);
                    set_scrolled(&handle, dragged(drag.scrolled, dy, viewport, max_scroll));
                } else {
                    // The button went up where this window did not see it.
                    state.drag = None;
                }
                window.refresh();
            } else {
                let over = hitbox.is_hovered(window);
                if over != state.strip_hover {
                    state.strip_hover = over;
                    window.refresh();
                }
            }
        }
    });
    window.on_mouse_event({
        let state = state.clone();
        move |event: &MouseUpEvent, phase, window: &mut Window, _: &mut App| {
            if phase == DispatchPhase::Capture
                && event.button == MouseButton::Left
                && state.borrow_mut().drag.take().is_some()
            {
                window.refresh();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRACK: f32 = 400.0 - 2.0 * INSET;

    #[test]
    fn content_that_fits_has_no_thumb() {
        assert_eq!(thumb(400.0, 0.0, 0.0), None);
        assert_eq!(thumb(400.0, 0.3, 0.0), None);
        assert_eq!(thumb(0.0, 100.0, 0.0), None);
    }

    #[test]
    fn the_thumb_is_as_long_as_the_viewport_is_of_the_content() {
        // Content twice the viewport: the thumb covers half the track.
        let half = thumb(400.0, 400.0, 0.0).unwrap();
        assert!((half.len - TRACK / 2.0).abs() < 0.01);
        let quarter = thumb(400.0, 1200.0, 0.0).unwrap();
        assert!((quarter.len - TRACK / 4.0).abs() < 0.01);
    }

    #[test]
    fn the_thumb_never_gets_shorter_than_a_minimum() {
        let tiny = thumb(400.0, 400_000.0, 0.0).unwrap();
        assert_eq!(tiny.len, MIN_THUMB);
        // And never longer than a track that is itself short.
        let short = thumb(20.0, 400.0, 0.0).unwrap();
        assert_eq!(short.len, 20.0 - 2.0 * INSET);
    }

    #[test]
    fn the_thumb_moves_from_the_top_to_the_bottom_of_the_track() {
        let start = thumb(400.0, 1200.0, 0.0).unwrap();
        assert_eq!(start.top, INSET);
        let end = thumb(400.0, 1200.0, 1200.0).unwrap();
        assert!((end.top + end.len - (400.0 - INSET)).abs() < 0.01);
        let middle = thumb(400.0, 1200.0, 600.0).unwrap();
        assert!((middle.top - (INSET + (TRACK - middle.len) / 2.0)).abs() < 0.01);
        // An overscrolled offset stays on the track.
        assert_eq!(thumb(400.0, 1200.0, 5000.0).unwrap(), end);
        assert_eq!(thumb(400.0, 1200.0, -50.0).unwrap(), start);
    }

    #[test]
    fn dragging_the_thumb_to_the_end_of_the_track_scrolls_to_the_end() {
        let (viewport, max) = (400.0, 1200.0);
        let len = thumb(viewport, max, 0.0).unwrap().len;
        let travel = viewport - 2.0 * INSET - len;
        assert!((dragged(0.0, travel, viewport, max) - max).abs() < 0.01);
        assert!((dragged(0.0, travel / 2.0, viewport, max) - max / 2.0).abs() < 0.01);
        // The thumb keeps up with the pointer, wherever it was grabbed.
        let grabbed = dragged(300.0, 10.0, viewport, max);
        let before = thumb(viewport, max, 300.0).unwrap().top;
        let after = thumb(viewport, max, grabbed).unwrap().top;
        assert!((after - before - 10.0).abs() < 0.01);
    }

    #[test]
    fn dragging_past_either_end_stops_at_it() {
        assert_eq!(dragged(100.0, 100_000.0, 400.0, 1200.0), 1200.0);
        assert_eq!(dragged(100.0, -100_000.0, 400.0, 1200.0), 0.0);
        assert_eq!(dragged(7.0, 50.0, 400.0, 0.0), 7.0);
    }

    #[test]
    fn a_click_on_the_track_pages_by_one_viewport() {
        assert_eq!(paged(0.0, 400.0, 1200.0, true), 400.0);
        assert_eq!(paged(400.0, 400.0, 1200.0, true), 800.0);
        assert_eq!(paged(800.0, 400.0, 1200.0, false), 400.0);
        assert_eq!(paged(1000.0, 400.0, 1200.0, true), 1200.0);
        assert_eq!(paged(100.0, 400.0, 1200.0, false), 0.0);
    }

    #[test]
    fn the_bar_stays_for_a_second_after_the_last_activity() {
        let ms = Duration::from_millis;
        assert!(wanted(true, None));
        assert!(wanted(true, Some(ms(5000))));
        assert!(!wanted(false, None));
        assert!(wanted(false, Some(ms(0))));
        assert!(wanted(false, Some(ms(999))));
        assert!(!wanted(false, Some(ms(1000))));
    }

    #[test]
    fn the_bar_fades_in_and_out_over_the_motion_length() {
        let start = Instant::now();
        let mut fade = Fade::at_rest(start);
        assert_eq!(fade.level(start, false), 0.0);
        fade.set(true, start, false);
        let half = fade.level(start + FADE / 2, false);
        assert!(half > 0.5 && half < 1.0);
        assert_eq!(fade.level(start + FADE, false), 1.0);
        assert!(fade.settled(start + FADE, false));
        // Turned around halfway, it fades back from where it was.
        let turn = start + FADE / 2;
        fade.set(false, turn, false);
        assert!((fade.level(turn, false) - half).abs() < 0.001);
        assert_eq!(fade.level(turn + FADE, false), 0.0);
    }

    #[test]
    fn with_reduced_motion_the_bar_appears_and_vanishes_at_once() {
        let start = Instant::now();
        let mut fade = Fade::at_rest(start);
        fade.set(true, start, true);
        assert_eq!(fade.level(start, true), 1.0);
        assert!(fade.settled(start, true));
        fade.set(false, start, true);
        assert_eq!(fade.level(start, true), 0.0);
    }

    /// The `long` preview: 1000 Segments and 100 Recordings, in a window the
    /// test platform draws. Starts at the top, with no bar pinned.
    #[cfg(debug_assertions)]
    mod on_screen {
        use super::*;
        use crate::ui::VisperWindow;
        use gpui::{Modifiers, TestAppContext, VisualTestContext, WindowHandle};

        const LEFT: Option<MouseButton> = Some(MouseButton::Left);

        fn open_long(
            cx: &mut TestAppContext,
        ) -> (WindowHandle<VisperWindow>, &mut VisualTestContext) {
            let window = cx.update(|cx| crate::ui::preview::open_preview("long", cx));
            let cx = VisualTestContext::from_window(window.into(), cx).into_mut();
            window
                .update(cx, |view, _, _| {
                    view.transcript_scroll.set_offset(point(px(0.0), px(0.0)));
                    view.library_scroll.set_offset(point(px(0.0), px(0.0)));
                })
                .unwrap();
            cx.run_until_parked();
            (window, cx)
        }

        /// The area's handle: how far it is scrolled and can scroll, and its
        /// viewport.
        fn scroll(
            window: WindowHandle<VisperWindow>,
            cx: &mut VisualTestContext,
            library: bool,
        ) -> (f32, f32, f32) {
            window
                .read_with(cx, |view, _| {
                    measure(if library {
                        &view.library_scroll
                    } else {
                        &view.transcript_scroll
                    })
                })
                .map(|(viewport, max, scrolled)| (scrolled, max, viewport))
                .unwrap()
        }

        fn thumb_center(
            window: WindowHandle<VisperWindow>,
            cx: &mut VisualTestContext,
            selector: &'static str,
            library: bool,
        ) -> gpui::Point<Pixels> {
            let strip = cx.debug_bounds(selector).expect("the bar is on screen");
            let (scrolled, max, viewport) = scroll(window, cx, library);
            let thumb = thumb(viewport, max, scrolled).expect("the content overflows");
            point(
                strip.right() - px(INSET + 3.0),
                strip.top() + px(thumb.top + thumb.len / 2.0),
            )
        }

        fn drag_thumb(
            window: WindowHandle<VisperWindow>,
            cx: &mut VisualTestContext,
            selector: &'static str,
            library: bool,
            dy: f32,
        ) {
            let from = thumb_center(window, cx, selector, library);
            cx.simulate_mouse_move(from, None, Modifiers::default());
            cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
            for step in 1..=4 {
                let to = point(from.x, from.y + px(dy * step as f32 / 4.0));
                cx.simulate_mouse_move(to, LEFT, Modifiers::default());
            }
            cx.simulate_mouse_up(
                point(from.x, from.y + px(dy)),
                MouseButton::Left,
                Modifiers::default(),
            );
        }

        #[gpui::test]
        fn dragging_the_thumb_scrolls_in_proportion(cx: &mut TestAppContext) {
            for (selector, library) in
                [("transcript-scrollbar", false), ("library-scrollbar", true)]
            {
                let (window, cx) = open_long(cx);
                let (scrolled, max, viewport) = scroll(window, cx, library);
                assert_eq!(scrolled, 0.0);
                assert!(max > viewport, "{selector}: a long list");

                let len = thumb(viewport, max, 0.0).unwrap().len;
                let travel = viewport - 2.0 * INSET - len;
                drag_thumb(window, cx, selector, library, travel / 2.0);
                let (scrolled, ..) = scroll(window, cx, library);
                assert!(
                    (scrolled - max / 2.0).abs() < 2.0,
                    "{selector}: half the travel is half the range, got {scrolled} of {max}"
                );

                // Dragging on past the end stops at the end.
                drag_thumb(window, cx, selector, library, travel * 2.0);
                let (scrolled, ..) = scroll(window, cx, library);
                assert!(
                    (scrolled - max).abs() < 0.5,
                    "{selector}: {scrolled} of {max}"
                );
            }
        }

        #[gpui::test]
        fn clicking_the_track_pages_by_one_viewport(cx: &mut TestAppContext) {
            let (window, cx) = open_long(cx);
            let strip = cx.debug_bounds("transcript-scrollbar").unwrap();
            let (_, max, viewport) = scroll(window, cx, false);

            let below = point(strip.right() - px(INSET + 3.0), strip.bottom() - px(8.0));
            cx.simulate_mouse_move(below, None, Modifiers::default());
            cx.simulate_click(below, Modifiers::default());
            let (scrolled, ..) = scroll(window, cx, false);
            assert!(
                (scrolled - viewport).abs() < 0.5,
                "paged down once: {scrolled}"
            );

            // The thumb has moved down; the track above it pages back up.
            let above = point(strip.right() - px(INSET + 3.0), strip.top() + px(4.0));
            cx.simulate_mouse_move(above, None, Modifiers::default());
            cx.simulate_click(above, Modifiers::default());
            let (scrolled, ..) = scroll(window, cx, false);
            assert!(scrolled.abs() < 0.5, "paged back up: {scrolled} of {max}");
        }

        /// The Transcript follows new Segments only from its end, and a
        /// thumb dragged to the end puts it there.
        #[gpui::test]
        fn a_thumb_dragged_to_the_end_leaves_the_transcript_following(cx: &mut TestAppContext) {
            let (window, cx) = open_long(cx);
            let (_, _, viewport) = scroll(window, cx, false);
            drag_thumb(window, cx, "transcript-scrollbar", false, viewport * 10.0);
            window
                .read_with(cx, |view, _| {
                    let handle = &view.transcript_scroll;
                    assert!(handle.max_offset().height + handle.offset().y <= px(24.0));
                })
                .unwrap();
        }

        /// Dragging across the text still selects it, and clicking the text
        /// does not touch the bar.
        #[gpui::test]
        fn text_selection_still_works_beside_the_bar(cx: &mut TestAppContext) {
            let (window, cx) = open_long(cx);
            let area = cx
                .debug_bounds("transcript")
                .expect("the Transcript is on screen");
            // The first Segment's text: 4 px of padding, 3 of the row's, half
            // a line down; after the 32 px padding, the row's 10, the
            // timestamp column and its gap.
            let from = point(
                area.left() + px(32.0 + 10.0 + 64.0 + 8.0 + 20.0),
                area.top() + px(4.0 + 3.0 + 12.0),
            );
            cx.simulate_mouse_move(from, None, Modifiers::default());
            cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
            let to = point(from.x + px(160.0), from.y);
            cx.simulate_mouse_move(to, LEFT, Modifiers::default());
            cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
            window
                .read_with(cx, |view, _| {
                    let segments = view.library.selected().unwrap().visible_segments();
                    let text = view.selection.borrow().text(segments);
                    assert!(text.chars().count() > 3, "selected {text:?}");
                    assert!(text.contains("个分段"), "selected {text:?}");
                })
                .unwrap();
            assert_eq!(scroll(window, cx, false).0, 0.0);
        }
    }
}
