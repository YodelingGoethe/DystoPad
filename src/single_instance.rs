//! One window: a second start hands its files to the running DystoPad and quits.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use interprocess::local_socket::prelude::*;
use interprocess::local_socket::{GenericNamespaced, ListenerOptions, Name, Stream};

/// One name per user, so two people on one computer each get their own window.
fn name() -> std::io::Result<Name<'static>> {
    let user = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_default();
    format!("dystopad-{user}.sock").to_ns_name::<GenericNamespaced>()
}

/// Sends the files (none just brings the window up) to a running DystoPad. False if there is none.
pub fn hand_over(files: &[PathBuf]) -> bool {
    let Ok(mut stream) = name().and_then(Stream::connect) else {
        return false;
    };
    let message = files
        .iter()
        .map(|f| f.to_string_lossy())
        .collect::<Vec<_>>()
        .join("\n");
    stream.write_all(message.as_bytes()).is_ok()
}

/// Waits for later starts in the background; their files go to `send`, then `wake` runs.
pub fn listen(send: Sender<Vec<PathBuf>>, wake: impl Fn() + Send + 'static) {
    let Ok(listener) = name().and_then(|name| ListenerOptions::new().name(name).create_sync())
    else {
        return;
    };
    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let mut message = String::new();
            if stream.read_to_string(&mut message).is_ok() {
                let files = message
                    .lines()
                    .filter(|l| !l.is_empty())
                    .map(PathBuf::from)
                    .collect();
                if send.send(files).is_err() {
                    return;
                }
                wake();
            }
        }
    });
}
