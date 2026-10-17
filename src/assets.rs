use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

/// Icons are embedded so the executable also works outside the repository.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        let bytes: &'static [u8] = match path {
            "icons/audio.svg" => include_bytes!("../assets/icons/audio.svg"),
            "icons/plus.svg" => include_bytes!("../assets/icons/plus.svg"),
            "icons/plus-bold.svg" => include_bytes!("../assets/icons/plus-bold.svg"),
            "icons/mark-vee.svg" => include_bytes!("../assets/icons/mark-vee.svg"),
            "icons/mark-lines.svg" => include_bytes!("../assets/icons/mark-lines.svg"),
            "icons/download.svg" => include_bytes!("../assets/icons/download.svg"),
            "icons/export.svg" => include_bytes!("../assets/icons/export.svg"),
            "icons/lock.svg" => include_bytes!("../assets/icons/lock.svg"),
            "icons/check.svg" => include_bytes!("../assets/icons/check.svg"),
            "icons/waiting.svg" => include_bytes!("../assets/icons/waiting.svg"),
            "icons/cancelled.svg" => include_bytes!("../assets/icons/cancelled.svg"),
            "icons/warning.svg" => include_bytes!("../assets/icons/warning.svg"),
            "icons/retry.svg" => include_bytes!("../assets/icons/retry.svg"),
            "icons/more.svg" => include_bytes!("../assets/icons/more.svg"),
            "icons/play.svg" => include_bytes!("../assets/icons/play.svg"),
            "icons/pause.svg" => include_bytes!("../assets/icons/pause.svg"),
            "icons/remove.svg" => include_bytes!("../assets/icons/remove.svg"),
            "icons/chevron-up.svg" => include_bytes!("../assets/icons/chevron-up.svg"),
            "icons/tick.svg" => include_bytes!("../assets/icons/tick.svg"),
            _ => return Ok(None),
        };
        Ok(Some(Cow::Borrowed(bytes)))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        if path != "icons" {
            return Ok(Vec::new());
        }
        Ok(vec![
            "icons/audio.svg".into(),
            "icons/plus.svg".into(),
            "icons/plus-bold.svg".into(),
            "icons/mark-vee.svg".into(),
            "icons/mark-lines.svg".into(),
            "icons/download.svg".into(),
            "icons/export.svg".into(),
            "icons/lock.svg".into(),
            "icons/check.svg".into(),
            "icons/waiting.svg".into(),
            "icons/cancelled.svg".into(),
            "icons/warning.svg".into(),
            "icons/retry.svg".into(),
            "icons/more.svg".into(),
            "icons/play.svg".into(),
            "icons/pause.svg".into(),
            "icons/remove.svg".into(),
            "icons/chevron-up.svg".into(),
            "icons/tick.svg".into(),
        ])
    }
}
