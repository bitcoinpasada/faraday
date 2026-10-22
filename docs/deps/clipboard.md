# The clipboard: the platform's own tool, no crate

Decision: the desktop shell reads and writes the clipboard by running
`wl-paste`/`wl-copy`, `xclip` or `pbpaste`/`pbcopy`, in that order. No
clipboard crate is used.

The read path is used by the scanner's Paste on the desktop
(`faraday/shells/window/src/clipboard.rs`). The owner chose this
over `arboard` on 2026-10-21, as this page had decided.

## How the read runs

- Linux: with `WAYLAND_DISPLAY` set, `wl-paste --no-newline --type
  text/plain`, otherwise `xclip -selection clipboard -o`. If the first
  tool is not installed the other is tried. With neither the answer is
  `ClipboardUnavailable`.
- macOS: `pbpaste`.
- The tool runs only on a press of Paste, never at start and never kept
  open, with standard input and error null and no shell. After about two
  seconds it is killed and the answer is `ClipboardUnavailable`. More
  than 1 MiB of output, a failing exit or empty text is
  `ClipboardUnavailable` too. The text is moved to the core, not copied.

## Windows

No tool and no crate. The shell calls the API through `windows-sys`,
the release the Starter already pins (`=0.52.0`), with the features
`Win32_System_DataExchange`, `Win32_System_Memory` and `Win32_System_Ole`
added to `Win32_Foundation`: `OpenClipboard`,
`GetClipboardData(CF_UNICODETEXT)`, `GlobalLock`, `GlobalUnlock`,
`CloseClipboard`, the UTF-16 converted to a string. The clipboard is
closed on every path. A clipboard that cannot be opened, holds no
Unicode text, is empty or is over 1 MiB gives `ClipboardUnavailable`.

## What was considered

`arboard` and `copypasta` are the two maintained Rust clipboard crates.
Both were considered and declined.

- Both link a window-system stack of their own: `arboard` pulls in
  `x11rb` and `wl-clipboard-rs` on Linux and `objc2-app-kit` on macOS.
  That is a second X11/Wayland client in a process that holds seed
  words, alongside the one winit already owns, and it is code this
  project would then be vouching for under `supply-chain/`.
- Both keep a clipboard connection alive inside the process. This shell
  reads the clipboard only when the person taps "Paste" and writes it
  only when they tap "Copy"; a connection that outlives the tap is more
  than the contract asks for.
- The tools are on the machine already. A desktop running Wayland has
  `wl-clipboard`, one running X11 has `xclip`, and macOS ships
  `pbpaste`/`pbcopy` in the base system.

## What the choice costs

A Linux session with neither `wl-clipboard` nor `xclip` installed has no
clipboard as far as this shell is concerned: the core is answered
`ClipboardUnavailable` and `ClipboardNotWritten`, and the "Paste" and
"Copy" rows are dimmed with "no clipboard". That is the same state a
shell with no clipboard at all reports, and the screens already say it.

The Android shell does not go through a tool: it uses the platform's
`ClipboardManager` on the main thread.
