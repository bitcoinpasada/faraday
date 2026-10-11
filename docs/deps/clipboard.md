# The clipboard: the platform's own tool, no crate

Decision: the desktop shell reads and writes the clipboard by running
`wl-paste`/`wl-copy`, `xclip` or `pbpaste`/`pbcopy`, in that order. No
clipboard crate is used.

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
