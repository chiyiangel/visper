//! The Dock icon. Visper runs as a bare executable, not a `.app` bundle, so
//! macOS has no icon file to show; set it from the embedded PNG instead.

/// Harmless when launched from a real bundle: it just sets the same icon.
#[cfg(target_os = "macos")]
pub fn set_icon() {
    use cocoa::appkit::{NSApp, NSApplication, NSImage};
    use cocoa::base::{id, nil};
    use cocoa::foundation::NSData;

    const ICON: &[u8] = include_bytes!("../assets/brand/app-icon-512.png");

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

#[cfg(not(target_os = "macos"))]
pub fn set_icon() {}
