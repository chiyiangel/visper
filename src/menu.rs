//! The menu bar and the standard shortcuts behind it. Only shortcuts a Mac
//! user already knows are bound; Visper adds none of its own.

use gpui::{App, Context, KeyBinding, Menu, MenuItem, OsAction, SystemMenuType, Window, actions};

use crate::macos;
use crate::ui::VisperWindow;

actions!(
    visper,
    [
        About,
        Hide,
        HideOthers,
        ShowAll,
        Quit,
        AddRecording,
        Export,
        CloseWindow,
        Copy,
        Minimize,
        Zoom,
        Homepage,
    ]
);

const WINDOW_MENU: &str = "窗口";
const HELP_MENU: &str = "帮助";

pub fn install(cx: &mut App) {
    // `secondary` is ⌘ on macOS and Ctrl elsewhere.
    cx.bind_keys([
        KeyBinding::new("secondary-c", Copy, None),
        KeyBinding::new("secondary-o", AddRecording, None),
    ]);
    #[cfg(target_os = "macos")]
    cx.bind_keys([
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-h", Hide, None),
        KeyBinding::new("alt-cmd-h", HideOthers, None),
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("cmd-m", Minimize, None),
    ]);

    cx.on_action(|_: &About, _| macos::show_about());
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    cx.on_action(|_: &Homepage, cx| cx.open_url(env!("CARGO_PKG_HOMEPAGE")));
    cx.on_action(|_: &Quit, cx| {
        in_window(
            cx,
            |view, window, cx| view.request_quit(window, cx),
            |cx| cx.quit(),
        )
    });
    cx.on_action(|_: &CloseWindow, cx| {
        in_window(
            cx,
            |view, window, cx| view.request_close(window, cx),
            |_| {},
        )
    });
    cx.on_action(|_: &AddRecording, cx| in_window(cx, |view, _, cx| view.pick_file(cx), |_| {}));
    cx.on_action(|_: &Export, cx| in_window(cx, |view, _, cx| view.export_selected(cx), |_| {}));
    cx.on_action(|_: &Copy, cx| in_window(cx, |view, _, cx| view.copy_selection(cx), |_| {}));
    cx.on_action(|_: &Minimize, cx| in_window(cx, |_, window, _| window.minimize_window(), |_| {}));
    cx.on_action(|_: &Zoom, cx| in_window(cx, |_, window, _| window.zoom_window(), |_| {}));

    cx.set_menus(menus());
    macos::adopt_menus(WINDOW_MENU, HELP_MENU);
}

/// The first menu is titled with the app's name by macOS, whatever it is
/// called here. AppKit adds 进入全屏幕 and the window list to 窗口, and the
/// search field to 帮助.
fn menus() -> Vec<Menu> {
    vec![
        Menu {
            name: "Visper".into(),
            items: vec![
                MenuItem::action("关于 Visper", About),
                MenuItem::separator(),
                MenuItem::os_submenu("服务", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("隐藏 Visper", Hide),
                MenuItem::action("隐藏其他", HideOthers),
                MenuItem::action("全部显示", ShowAll),
                MenuItem::separator(),
                MenuItem::action("退出 Visper", Quit),
            ],
        },
        Menu {
            name: "文件".into(),
            items: vec![
                MenuItem::action("加入录音…", AddRecording),
                MenuItem::action("导出 Markdown…", Export),
                MenuItem::separator(),
                MenuItem::action("关闭窗口", CloseWindow),
            ],
        },
        Menu {
            name: "编辑".into(),
            items: vec![MenuItem::os_action("拷贝", Copy, OsAction::Copy)],
        },
        Menu {
            name: WINDOW_MENU.into(),
            items: vec![
                MenuItem::action("最小化", Minimize),
                MenuItem::action("缩放", Zoom),
            ],
        },
        Menu {
            name: HELP_MENU.into(),
            items: vec![MenuItem::action("Visper 项目主页", Homepage)],
        },
    ]
}

/// Runs `act` on the Visper window in front, or `otherwise` when there is
/// none. Global action listeners run while GPUI still holds the window that
/// dispatched them, so the update waits until it has been handed back.
fn in_window(
    cx: &mut App,
    act: impl FnOnce(&mut VisperWindow, &mut Window, &mut Context<VisperWindow>) + 'static,
    otherwise: impl FnOnce(&mut App) + 'static,
) {
    cx.defer(move |cx| {
        let window = cx
            .active_window()
            .or_else(|| cx.windows().first().copied())
            .and_then(|window| window.downcast::<VisperWindow>());
        match window {
            Some(window) => {
                let _ = window.update(cx, act);
            }
            None => otherwise(cx),
        }
    });
}
