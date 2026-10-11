//! The desktop shell's clipboard, through the platform's own tool.
//!
//! There is no clipboard crate here (`docs/deps/clipboard.md`): every
//! one of them pulls in a window-system stack this shell already has
//! through winit, and a crate that talks X11 or Wayland itself is more
//! code in the process that holds a seed than the feature is worth. The
//! tools below are what a person on this platform already has, they run
//! outside this process, and a session without them answers "no
//! clipboard", which the rows state.
//!
//! Order is the platform's own: Wayland first, then X11, then macOS.
//! The first tool that runs answers; a tool that is not installed, or
//! that fails, is passed over.

use std::io::Write;
use std::process::{Command, Stdio};

/// The readers, in the order they are tried.
const READERS: [(&str, &[&str]); 3] = [
    ("wl-paste", &["--no-newline"]),
    ("xclip", &["-o", "-selection", "clipboard"]),
    ("pbpaste", &[]),
];

/// The writers, in the same order.
const WRITERS: [(&str, &[&str]); 3] = [
    ("wl-copy", &[]),
    ("xclip", &["-selection", "clipboard"]),
    ("pbcopy", &[]),
];

/// What the clipboard holds, or `None` where no tool ran, the tool
/// failed, or what came back is not text.
pub fn read() -> Option<String> {
    for (tool, args) in READERS {
        let Ok(out) = Command::new(tool).args(args).output() else {
            continue;
        };
        if !out.status.success() {
            continue;
        }
        return String::from_utf8(out.stdout).ok();
    }
    None
}

/// Puts `text` on the clipboard. `false` where no tool ran or the tool
/// failed.
pub fn write(text: &str) -> bool {
    for (tool, args) in WRITERS {
        let Ok(mut child) = Command::new(tool)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        else {
            continue;
        };
        let written = child
            .stdin
            .take()
            .is_some_and(|mut pipe| pipe.write_all(text.as_bytes()).is_ok());
        // `wl-copy` forks a process that owns the selection until
        // something else claims it, and exits at once; the others exit
        // when the pipe closes. Either way the status is the answer.
        let ok = child.wait().map(|s| s.success()).unwrap_or(false);
        if written && ok {
            return true;
        }
    }
    false
}
