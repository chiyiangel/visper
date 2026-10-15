mod assets;
mod dock;
mod download;
mod markdown;
mod playback;
mod store;
mod theme;
mod transcribe;
mod ui;

use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::time::Duration;

use gpui::{App, Application, Timer};

fn main() {
    #[cfg(debug_assertions)]
    if let Some(scenario) = preview_scenario() {
        Application::new()
            .with_assets(assets::Assets)
            .run(move |cx| {
                dock::set_icon();
                ui::open_preview(&scenario, cx);
            });
        return;
    }
    let support = ui::support_dir();
    let _ = std::fs::create_dir_all(&support);
    let socket = support.join("instance.sock");
    let listener = match claim_primary(&socket) {
        Some(listener) => listener,
        None => return,
    };
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for incoming in listener.incoming() {
            if incoming.is_ok() {
                let _ = tx.send(());
            }
        }
    });

    Application::new()
        .with_assets(assets::Assets)
        .run(move |cx: &mut App| {
            dock::set_icon();
            let window = ui::open(
                store::Store::new(store::library_path(&support)),
                download::weight_path(&support),
                cx,
            );
            cx.spawn(async move |cx| {
                loop {
                    Timer::after(Duration::from_millis(250)).await;
                    let mut activate = false;
                    while rx.try_recv().is_ok() {
                        activate = true;
                    }
                    if activate {
                        let _ = cx.update(|cx| {
                            cx.activate(true);
                            let _ = window.update(cx, |_, window, _| window.activate_window());
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

fn claim_primary(socket: &Path) -> Option<UnixListener> {
    if UnixStream::connect(socket).is_ok() {
        return None;
    }
    let _ = std::fs::remove_file(socket);
    match UnixListener::bind(socket) {
        Ok(listener) => Some(listener),
        Err(_) => {
            let _ = UnixStream::connect(socket);
            None
        }
    }
}
