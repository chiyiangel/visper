//! Interaction feedback GPUI does not animate on its own: hover fades, the
//! language pill sliding, and focus rings that only keyboard navigation shows.
use gpui::{App, FocusHandle, SharedString, Window};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

const HOVER: Duration = Duration::from_millis(120);
const PILL: Duration = Duration::from_millis(180);

#[derive(Clone, Default)]
pub(super) struct Motion(Rc<RefCell<State>>);

#[derive(Default)]
struct State {
    /// The last input was Tab, so the focused control shows its ring.
    keyboard: bool,
    handles: HashMap<SharedString, FocusHandle>,
    focused: Option<SharedString>,
    /// The hovered control fading in and the one it left fading out, each
    /// with the level it started from.
    entering: Option<(SharedString, f32)>,
    leaving: Option<(SharedString, f32)>,
    hover_since: Option<Instant>,
    /// The language option the pill slides away from.
    pill_from: Option<(usize, Instant)>,
}

fn ease_out(elapsed: Duration, length: Duration) -> f32 {
    let t = (elapsed.as_secs_f32() / length.as_secs_f32()).clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

impl State {
    fn hover_progress(&self) -> f32 {
        self.hover_since
            .map_or(1.0, |since| ease_out(since.elapsed(), HOVER))
    }

    fn level(&self, id: &str) -> f32 {
        let t = self.hover_progress();
        match (&self.entering, &self.leaving) {
            (Some((current, start)), _) if current.as_ref() == id => start + (1.0 - start) * t,
            (_, Some((previous, start))) if previous.as_ref() == id => start * (1.0 - t),
            _ => 0.0,
        }
    }
}

impl Motion {
    /// Call once per frame before building controls.
    pub fn begin_frame(&self, window: &mut Window) {
        let mut state = self.0.borrow_mut();
        state.focused = state
            .handles
            .iter()
            .find(|(_, handle)| handle.is_focused(window))
            .map(|(id, _)| id.clone());
        let hovering = state
            .hover_since
            .is_some_and(|since| since.elapsed() < HOVER);
        let sliding = state
            .pill_from
            .is_some_and(|(_, since)| since.elapsed() < PILL);
        if hovering || sliding {
            window.request_animation_frame();
        }
    }

    /// Returns whether the mode changed and the window needs a redraw.
    pub fn use_keyboard(&self, keyboard: bool) -> bool {
        let mut state = self.0.borrow_mut();
        let changed = state.keyboard != keyboard;
        state.keyboard = keyboard;
        changed
    }

    pub fn handle(&self, id: &SharedString, cx: &mut App) -> FocusHandle {
        self.0
            .borrow_mut()
            .handles
            .entry(id.clone())
            .or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone()
    }

    pub fn ring(&self, id: &str) -> bool {
        let state = self.0.borrow();
        state.keyboard
            && state
                .focused
                .as_ref()
                .is_some_and(|focused| focused.as_ref() == id)
    }

    /// 0 at rest, 1 fully hovered, eased in between.
    pub fn hover(&self, id: &str) -> f32 {
        self.0.borrow().level(id)
    }

    pub fn on_hover(&self, id: SharedString) -> impl Fn(&bool, &mut Window, &mut App) + 'static {
        let motion = self.clone();
        move |hovered, window, _| {
            let mut state = motion.0.borrow_mut();
            let level = state.level(&id);
            if *hovered {
                if let Some((current, start)) = state.entering.take()
                    && current != id
                {
                    let t = state.hover_progress();
                    state.leaving = Some((current, start + (1.0 - start) * t));
                }
                state.entering = Some((id.clone(), level));
            } else if state
                .entering
                .as_ref()
                .is_some_and(|(current, _)| *current == id)
            {
                state.entering = None;
                state.leaving = Some((id.clone(), level));
            } else {
                return;
            }
            state.hover_since = Some(Instant::now());
            drop(state);
            window.refresh();
        }
    }

    pub fn move_pill(&self, from: usize) {
        self.0.borrow_mut().pill_from = Some((from, Instant::now()));
    }

    /// The pill's position in option widths, sliding toward `to`.
    pub fn pill(&self, to: usize) -> f32 {
        match self.0.borrow().pill_from {
            Some((from, since)) if since.elapsed() < PILL => {
                let t = ease_out(since.elapsed(), PILL);
                from as f32 + (to as f32 - from as f32) * t
            }
            _ => to as f32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_eases_out_and_settles() {
        assert_eq!(ease_out(Duration::ZERO, HOVER), 0.0);
        assert!(ease_out(HOVER / 2, HOVER) > 0.5);
        assert_eq!(ease_out(HOVER * 2, HOVER), 1.0);
    }

    #[test]
    fn a_control_left_mid_fade_fades_out_from_where_it_was() {
        let state = State {
            leaving: Some(("export".into(), 0.4)),
            hover_since: Some(Instant::now() - HOVER),
            ..Default::default()
        };
        assert_eq!(state.level("export"), 0.0);
        let state = State {
            entering: Some(("more".into(), 0.4)),
            hover_since: Some(Instant::now() - HOVER),
            ..Default::default()
        };
        assert_eq!(state.level("more"), 1.0);
        assert_eq!(state.level("export"), 0.0);
    }
}
