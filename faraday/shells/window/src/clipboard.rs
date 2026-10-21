//! The clipboard's text, as the scanner's Paste asks for it: read when
//! the core sends `RequestClipboard`, never watched and never written
//! (the desktop does not answer `WriteClipboard`). The system clipboard
//! is read through the platform's own tool, no clipboard crate
//! (`docs/deps/clipboard.md`); a [`Source`] stands in for it where there
//! is no display.

use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use osk_shell_api::{Event, FileKind};

/// The most text a read returns, in bytes. More is unavailable.
const CAP: usize = 1 << 20;

/// How long a tool runs before it is killed.
const LIMIT: Duration = Duration::from_secs(2);

/// Where the clipboard's text comes from.
pub trait Source {
    /// The clipboard's text, or `None` when there is none: it is empty,
    /// it holds something that is not text, it is over the cap, or it
    /// cannot be reached. Costs running another program (up to two
    /// seconds) or a Windows API call, so it is called only for a press
    /// of Paste.
    fn text(&mut self) -> Option<String>;
}

/// A program that prints the clipboard's text on its standard output.
#[derive(Clone, Debug)]
pub struct Tool {
    /// The program's name, looked up on `PATH`.
    pub program: &'static str,
    /// Its arguments.
    pub args: &'static [&'static str],
}

/// What running a [`Tool`] came to.
#[cfg_attr(windows, allow(dead_code))]
#[derive(Debug, PartialEq, Eq)]
enum Run {
    /// The program is not installed (or cannot be started): the next
    /// tool may be tried.
    Missing,
    /// It ran and printed this; or it failed, timed out or printed more
    /// than the cap, and there is nothing.
    Done(Option<String>),
}

/// Runs `tool` and returns its standard output as text. Costs a process
/// and up to `limit` of waiting; the tool is killed after that. Standard
/// input and error are null, no shell is involved, and the output is read
/// on a thread of its own so that a tool that prints and then hangs is
/// still cut off at `limit`.
#[cfg_attr(windows, allow(dead_code))]
fn run(tool: &Tool, limit: Duration, cap: usize) -> Run {
    let spawned = Command::new(tool.program)
        .args(tool.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = spawned else {
        return Run::Missing;
    };
    let Some(mut out) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Run::Done(None);
    };
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        // One byte past the cap says "more than the cap".
        let read = out.by_ref().take(cap as u64 + 1).read_to_end(&mut bytes);
        let _ = tx.send(read.map(|_| bytes));
    });
    let bytes = rx.recv_timeout(limit);
    // Over the cap, late or failed: stop the tool. Otherwise it has
    // closed its output and is reaped here.
    let in_time = matches!(&bytes, Ok(Ok(b)) if b.len() <= cap);
    if !in_time {
        let _ = child.kill();
    }
    let succeeded = child.wait().map(|s| s.success()).unwrap_or(false);
    match bytes {
        Ok(Ok(bytes)) if in_time && succeeded => Run::Done(String::from_utf8(bytes).ok()),
        _ => Run::Done(None),
    }
}

/// The tools to try, in order, on this platform and session.
#[cfg_attr(windows, allow(dead_code))]
fn tools() -> Vec<Tool> {
    let wl = Tool {
        program: "wl-paste",
        args: &["--no-newline", "--type", "text/plain"],
    };
    let xclip = Tool {
        program: "xclip",
        args: &["-selection", "clipboard", "-o"],
    };
    if cfg!(target_os = "macos") {
        vec![Tool {
            program: "pbpaste",
            args: &[],
        }]
    } else if std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty()) {
        vec![wl, xclip]
    } else {
        vec![xclip, wl]
    }
}

/// The system clipboard, read on each request and not kept open.
#[cfg_attr(windows, allow(dead_code))]
struct System {
    /// The tools to try in order; the first one installed is the one.
    tools: Vec<Tool>,
    /// The time each gets.
    limit: Duration,
    /// The most bytes taken.
    cap: usize,
}

impl Source for System {
    fn text(&mut self) -> Option<String> {
        #[cfg(windows)]
        {
            windows::text(self.cap)
        }
        #[cfg(not(windows))]
        {
            for tool in &self.tools {
                match run(tool, self.limit, self.cap) {
                    Run::Missing => continue,
                    Run::Done(text) => return text,
                }
            }
            None
        }
    }
}

/// The Windows clipboard through the API: no tool.
#[cfg(windows)]
#[allow(unsafe_code)]
mod windows {
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, OpenClipboard,
    };
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
    use windows_sys::Win32::System::Ole::CF_UNICODETEXT;

    /// The clipboard's Unicode text, or `None` when there is none, it is
    /// over `cap` bytes or the clipboard cannot be opened. Costs a
    /// round trip to the clipboard's owner. The clipboard is closed and
    /// the memory unlocked on every path.
    pub fn text(cap: usize) -> Option<String> {
        // SAFETY: the clipboard is opened for this call only and closed
        // before it returns, whatever `read_open` finds.
        unsafe {
            if OpenClipboard(0) == 0 {
                return None;
            }
            let text = read_open(cap);
            CloseClipboard();
            text
        }
    }

    /// Reads the text while the clipboard is open.
    ///
    /// # Safety
    /// The caller holds the clipboard open.
    unsafe fn read_open(cap: usize) -> Option<String> {
        // SAFETY: the handle is the clipboard's own and is not freed here;
        // it is read only while locked, within the `GlobalSize` bytes it
        // reports, and unlocked after.
        unsafe {
            let handle = GetClipboardData(u32::from(CF_UNICODETEXT));
            if handle == 0 {
                return None;
            }
            let hglobal = handle as *mut core::ffi::c_void;
            let size = GlobalSize(hglobal);
            if size == 0 || size > cap.saturating_mul(2) {
                return None;
            }
            let ptr = GlobalLock(hglobal) as *const u16;
            if ptr.is_null() {
                return None;
            }
            let units = std::slice::from_raw_parts(ptr, size / 2);
            let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
            let text = String::from_utf16(&units[..end]).ok();
            GlobalUnlock(hglobal);
            text.filter(|t| t.len() <= cap)
        }
    }
}

/// Answers the core's requests for the clipboard's text.
pub struct Clipboard {
    source: Box<dyn Source>,
}

impl Clipboard {
    /// The system clipboard. Cheap: nothing is run until a press of
    /// Paste asks.
    pub fn system() -> Clipboard {
        Clipboard::from_source(Box::new(System {
            tools: tools(),
            limit: LIMIT,
            cap: CAP,
        }))
    }

    /// A clipboard that reads from `source`.
    pub fn from_source(source: Box<dyn Source>) -> Clipboard {
        Clipboard { source }
    }

    /// The event that answers `RequestClipboard { kind }`: the text
    /// unmodified, or `ClipboardUnavailable` when there is none or it is
    /// empty. Costs what [`Source::text`] costs. The text is moved into
    /// the event, not copied, and the core wipes it once routed.
    pub fn answer(&mut self, kind: FileKind) -> Event {
        match self.source.text() {
            Some(text) if !text.is_empty() => Event::Clipboard { kind, text },
            _ => Event::ClipboardUnavailable { kind },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake(Option<&'static str>);

    impl Source for Fake {
        fn text(&mut self) -> Option<String> {
            self.0.map(str::to_owned)
        }
    }

    fn answer(held: Option<&'static str>) -> Event {
        Clipboard::from_source(Box::new(Fake(held))).answer(FileKind::Text)
    }

    #[test]
    fn text_on_the_clipboard_is_answered_as_it_is() {
        match answer(Some("  tb1qexample \n")) {
            Event::Clipboard { kind, text } => {
                assert_eq!(kind, FileKind::Text);
                assert_eq!(text, "  tb1qexample \n");
            }
            _ => panic!("expected the text"),
        }
    }

    #[test]
    fn an_empty_clipboard_or_one_without_text_is_unavailable() {
        for held in [None, Some("")] {
            assert!(matches!(
                answer(held),
                Event::ClipboardUnavailable {
                    kind: FileKind::Text
                }
            ));
        }
    }

    const MISSING: Tool = Tool {
        program: "faraday-no-such-clipboard-tool",
        args: &[],
    };

    #[test]
    fn a_missing_tool_is_unavailable() {
        assert_eq!(run(&MISSING, LIMIT, CAP), Run::Missing);
        let mut board = System {
            tools: vec![MISSING],
            limit: LIMIT,
            cap: CAP,
        };
        assert_eq!(board.text(), None);
    }

    #[cfg(unix)]
    mod stand_in {
        use super::*;

        fn board(tools: Vec<Tool>, limit: Duration, cap: usize) -> Clipboard {
            Clipboard::from_source(Box::new(System { tools, limit, cap }))
        }

        fn printing(script: &'static str) -> Tool {
            Tool {
                program: "sh",
                args: Box::leak(Box::new(["-c", script])),
            }
        }

        #[test]
        fn a_tool_that_prints_text_is_answered_with_it_unmodified() {
            let mut b = board(vec![printing("printf ' tb1q\\n'")], LIMIT, CAP);
            match b.answer(FileKind::Text) {
                Event::Clipboard { text, .. } => assert_eq!(text, " tb1q\n"),
                _ => panic!("expected the text"),
            }
        }

        #[test]
        fn a_tool_that_prints_nothing_or_fails_is_unavailable() {
            for script in ["true", "printf nope; exit 1"] {
                let mut b = board(vec![printing(script)], LIMIT, CAP);
                assert!(matches!(
                    b.answer(FileKind::Text),
                    Event::ClipboardUnavailable { .. }
                ));
            }
        }

        #[test]
        fn output_over_the_cap_is_refused_and_output_at_it_is_taken() {
            let mut over = board(vec![printing("printf 0123456789A")], LIMIT, 10);
            assert!(matches!(
                over.answer(FileKind::Text),
                Event::ClipboardUnavailable { .. }
            ));
            let mut at = board(vec![printing("printf 0123456789")], LIMIT, 10);
            assert!(matches!(at.answer(FileKind::Text), Event::Clipboard { .. }));
        }

        #[test]
        fn a_tool_that_hangs_is_cut_off() {
            let started = std::time::Instant::now();
            let mut b = board(
                vec![printing("printf partial; exec sleep 30")],
                Duration::from_millis(300),
                CAP,
            );
            assert!(matches!(
                b.answer(FileKind::Text),
                Event::ClipboardUnavailable { .. }
            ));
            assert!(started.elapsed() < Duration::from_secs(10));
        }

        #[test]
        fn the_next_tool_is_tried_when_the_first_is_missing() {
            let mut b = board(vec![MISSING, printing("printf hello")], LIMIT, CAP);
            assert!(matches!(b.answer(FileKind::Text), Event::Clipboard { .. }));
        }
    }
}
