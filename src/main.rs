// Release builds are a GUI app on Windows: no console window behind it.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod assets;
mod download;
mod file_url;
mod instance;
mod macos;
mod markdown;
mod menu;
mod playback;
mod store;
mod theme;
mod transcribe;
mod ui;

use std::sync::mpsc;
use std::time::Duration;

use gpui::{App, Application, Timer};

fn main() {
    #[cfg(debug_assertions)]
    if let Some(scenario) = preview_scenario() {
        Application::new()
            .with_assets(assets::Assets)
            .run(move |cx| {
                macos::set_icon();
                menu::install(cx);
                ui::open_preview(&scenario, cx);
            });
        return;
    }
    let support = ui::support_dir();
    let _ = std::fs::create_dir_all(&support);
    let Some(rx) = instance::claim(&support) else {
        return;
    };

    // Finder can hand over Recordings before the window exists, at launch.
    let (opened, opened_rx) = mpsc::channel();
    let app = Application::new().with_assets(assets::Assets);
    app.on_open_urls(move |urls| {
        for path in urls.iter().filter_map(|url| file_url::to_path(url)) {
            let _ = opened.send(path);
        }
    });
    app.run(move |cx: &mut App| {
        macos::set_icon();
        menu::install(cx);
        let window = ui::open(
            store::Store::new(store::library_path(&support)),
            download::weight_path(&support),
            cx,
        );
        // Not in previews, which would share and overwrite the saved frame.
        let _ = window.update(cx, |_, window, _| macos::remember_frame(window));
        cx.spawn(async move |cx| {
            loop {
                Timer::after(Duration::from_millis(250)).await;
                let mut activate = false;
                while rx.try_recv().is_ok() {
                    activate = true;
                }
                let paths: Vec<_> = opened_rx.try_iter().collect();
                if activate || !paths.is_empty() {
                    let _ = cx.update(|cx| {
                        cx.activate(true);
                        let _ = window.update(cx, |view, window, cx| {
                            window.activate_window();
                            view.open_files(paths, cx);
                        });
                    });
                }
            }
        })
        .detach();
    });
}

#[cfg(debug_assertions)]
fn preview_scenario() -> Option<String> {
    if std::env::args().nth(1).as_deref() == Some("--preview") {
        return Some(std::env::args().nth(2).unwrap_or_else(|| "empty".into()));
    }
    // A locally packaged preview executable always enters the isolated path,
    // including when launched through Finder without command-line arguments.
    std::env::current_exe()
        .ok()?
        .file_name()?
        .to_str()?
        .strip_prefix("visper-preview-")
        .map(str::to_owned)
}
