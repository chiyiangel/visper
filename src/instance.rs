//! One Visper per user. The first launch becomes the primary instance; a later
//! launch knocks on it, so the primary can bring its window forward, and exits.

use std::path::Path;
use std::sync::mpsc::{self, Receiver};

/// Becomes the primary instance and returns a channel that yields one `()` per
/// later launch, or knocks on the running primary and returns `None`.
pub fn claim(support: &Path) -> Option<Receiver<()>> {
    let (tx, rx) = mpsc::channel();
    let listen = platform::claim(support)?;
    std::thread::spawn(move || {
        listen(&mut || {
            let _ = tx.send(());
        })
    });
    Some(rx)
}

#[cfg(unix)]
mod platform {
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::Path;

    pub fn claim(support: &Path) -> Option<impl FnOnce(&mut dyn FnMut()) + Send + 'static> {
        let socket = support.join("instance.sock");
        if UnixStream::connect(&socket).is_ok() {
            return None;
        }
        let _ = std::fs::remove_file(&socket);
        let listener = match UnixListener::bind(&socket) {
            Ok(listener) => listener,
            Err(_) => {
                let _ = UnixStream::connect(&socket);
                return None;
            }
        };
        Some(move |knock: &mut dyn FnMut()| {
            for incoming in listener.incoming() {
                if incoming.is_ok() {
                    knock();
                }
            }
        })
    }
}

#[cfg(windows)]
mod platform {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows_sys::Win32::Foundation::{
        ERROR_NO_DATA, ERROR_PIPE_CONNECTED, GetLastError, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_INBOUND,
    };
    use windows_sys::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_TYPE_BYTE,
        PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
    };

    /// Named pipes share one namespace per machine, so the user name keeps two
    /// accounts from finding each other's instance.
    fn pipe_name() -> String {
        let user = std::env::var("USERNAME").unwrap_or_default();
        format!(r"\\.\pipe\visper-{user}")
    }

    pub fn claim(_support: &Path) -> Option<impl FnOnce(&mut dyn FnMut()) + Send + 'static> {
        let name = pipe_name();
        let wide: Vec<u16> = std::ffi::OsStr::new(&name)
            .encode_wide()
            .chain(Some(0))
            .collect();
        // SAFETY: `wide` is NUL-terminated and outlives the call. The first
        // pipe instance flag makes creation fail when another process owns it.
        let pipe = unsafe {
            CreateNamedPipeW(
                wide.as_ptr(),
                PIPE_ACCESS_INBOUND | FILE_FLAG_FIRST_PIPE_INSTANCE,
                PIPE_TYPE_BYTE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                0,
                0,
                0,
                std::ptr::null(),
            )
        };
        if pipe == INVALID_HANDLE_VALUE {
            // Opening the pipe is the knock.
            let _ = std::fs::OpenOptions::new().write(true).open(&name);
            return None;
        }
        // A raw handle is not `Send`; the listening thread is its only user.
        let pipe = pipe as usize;
        Some(move |knock: &mut dyn FnMut()| {
            let pipe = pipe as HANDLE;
            loop {
                // SAFETY: `pipe` is a valid server handle owned by this thread
                // for the life of the process.
                let connected = unsafe { ConnectNamedPipe(pipe, std::ptr::null_mut()) } != 0
                    || matches!(
                        unsafe { GetLastError() },
                        ERROR_PIPE_CONNECTED | ERROR_NO_DATA
                    );
                // Anything else would fail again at once; stop rather than spin.
                if !connected {
                    return;
                }
                knock();
                unsafe { DisconnectNamedPipe(pipe) };
            }
        })
    }
}
