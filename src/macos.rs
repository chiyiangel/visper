//! The AppKit pieces GPUI leaves out: the Dock icon of an unbundled run, the
//! standard About panel, the Window and Help menus, and a window frame that
//! survives relaunch. Every function is a no-op on other platforms.

// objc 0.2's `msg_send!` tests `feature = "cargo-clippy"`, which Cargo no
// longer declares.
#![allow(unexpected_cfgs)]

/// The name the window frame is saved under in the user defaults.
#[cfg(target_os = "macos")]
const FRAME_NAME: &str = "Visper";

/// `cargo run` starts a bare executable, so macOS has no icon file to show;
/// set it from the embedded PNG. Inside `Visper.app` the bundle's `.icns`,
/// with its hand-tuned small sizes, is already in the Dock and is kept.
#[cfg(target_os = "macos")]
pub fn set_icon() {
    use cocoa::appkit::{NSApp, NSApplication, NSImage};
    use cocoa::base::{id, nil};
    use cocoa::foundation::NSData;

    const ICON: &[u8] = include_bytes!("../assets/brand/app-icon-512.png");

    if bundled() {
        return;
    }
    // SAFETY: called on the main thread inside the GPUI run closure, so
    // `NSApp` exists. `dataWithBytes:length:` copies ICON into an autoreleased
    // NSData; the NSImage is owned and deliberately kept for the app's life.
    unsafe {
        let data = NSData::dataWithBytes_length_(nil, ICON.as_ptr().cast(), ICON.len() as u64);
        let image: id = NSImage::initWithData_(NSImage::alloc(nil), data);
        if image != nil {
            NSApp().setApplicationIconImage_(image);
        }
    }
}

/// Whether this process runs from an `.app` bundle with an Info.plist.
#[cfg(target_os = "macos")]
fn bundled() -> bool {
    use cocoa::base::{id, nil};
    use cocoa::foundation::NSBundle;

    // SAFETY: `mainBundle` always exists; an unbundled executable's has no
    // identifier.
    unsafe {
        let bundle: id = NSBundle::mainBundle();
        bundle.bundleIdentifier() != nil
    }
}

/// AppKit only fills in the window list, the full-screen item and the Help
/// search field for menus it has been told about, and GPUI only tells it
/// about a menu named "Window". Ours are named in Chinese.
#[cfg(target_os = "macos")]
pub fn adopt_menus(window: &str, help: &str) {
    use cocoa::appkit::{NSApp, NSApplication};
    use cocoa::base::{id, nil};
    use cocoa::foundation::{NSAutoreleasePool, NSString};
    use objc::{msg_send, sel, sel_impl};

    // SAFETY: main thread, after `set_menus` built the main menu. The
    // NSStrings are autoreleased; the menus stay owned by the main menu.
    unsafe {
        let app = NSApp();
        let main = app.mainMenu();
        if main == nil {
            return;
        }
        let submenu = |title: &str| -> id {
            let title = NSString::alloc(nil).init_str(title).autorelease();
            let item: id = msg_send![main, itemWithTitle: title];
            if item == nil {
                nil
            } else {
                msg_send![item, submenu]
            }
        };
        let window = submenu(window);
        if window != nil {
            app.setWindowsMenu_(window);
        }
        let help = submenu(help);
        if help != nil {
            let _: () = msg_send![app, setHelpMenu: help];
        }
    }
}

/// The standard About panel. Inside the bundle it reads the Info.plist;
/// unbundled it would show the executable's lowercase name and no version.
#[cfg(target_os = "macos")]
pub fn show_about() {
    use cocoa::appkit::NSApp;
    use cocoa::base::{id, nil};
    use cocoa::foundation::{NSArray, NSAutoreleasePool, NSDictionary, NSString};
    use objc::{msg_send, sel, sel_impl};

    // SAFETY: main thread (a menu action). All objects are autoreleased.
    unsafe {
        let string = |s: &str| -> id { NSString::alloc(nil).init_str(s).autorelease() };
        let keys = NSArray::arrayWithObjects(
            nil,
            &[string("ApplicationName"), string("ApplicationVersion")],
        );
        let values =
            NSArray::arrayWithObjects(nil, &[string("Visper"), string(env!("CARGO_PKG_VERSION"))]);
        let options = NSDictionary::dictionaryWithObjects_forKeys_(nil, values, keys);
        let _: () = msg_send![NSApp(), orderFrontStandardAboutPanelWithOptions: options];
    }
}

/// Puts the window back where the user last left it, and keeps saving its
/// frame as it moves, the way AppKit windows remember themselves.
#[cfg(target_os = "macos")]
pub fn remember_frame(window: &gpui::Window) {
    use cocoa::base::{BOOL, id, nil};
    use cocoa::foundation::{NSAutoreleasePool, NSString};
    use objc::{msg_send, sel, sel_impl};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    // `Window::window_handle` is GPUI's own handle, not the raw one.
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return;
    };
    // SAFETY: main thread; `ns_view` is GPUI's live content view, whose
    // window outlives this call. The name is autoreleased.
    unsafe {
        let view = appkit.ns_view.as_ptr() as id;
        let ns_window: id = msg_send![view, window];
        if ns_window == nil {
            return;
        }
        let name = NSString::alloc(nil).init_str(FRAME_NAME).autorelease();
        let _: BOOL = msg_send![ns_window, setFrameUsingName: name];
        let _: BOOL = msg_send![ns_window, setFrameAutosaveName: name];
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_icon() {}

#[cfg(not(target_os = "macos"))]
pub fn adopt_menus(_window: &str, _help: &str) {}

#[cfg(not(target_os = "macos"))]
pub fn show_about() {}

#[cfg(not(target_os = "macos"))]
pub fn remember_frame(_window: &gpui::Window) {}
