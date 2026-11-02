//! What the desktop app (`faraday/shells/desktop`) and the Starter
//! (`faraday/shells/starter`) share to run in a window, so the two behave
//! alike: the frame pacing ([`pace`]), the input ([`input`]), the copy of
//! a frame into the window ([`view`]), and the camera with the scanner
//! that reads its frames ([`camera`], [`lens`]), the clipboard's text
//! for Paste ([`clipboard`]), and moving the window by its top strip
//! ([`drag`]). Each shell keeps its own
//! loop and what only it does (the desktop's stick folders, the Starter's
//! disks).

pub mod camera;
pub mod clipboard;
pub mod drag;
pub mod input;
pub mod lens;
pub mod pace;
pub mod view;

use faraday_shell_api::{EntropyBytes, Event};

/// What `entropy` reads from on this system: the call the `getrandom`
/// crate makes here.
pub const ENTROPY_SOURCE: &str = if cfg!(windows) {
    "ProcessPrng"
} else if cfg!(target_vendor = "apple") {
    "getentropy(2)"
} else if cfg!(target_arch = "wasm32") {
    "crypto.getRandomValues"
} else {
    "getrandom(2)"
};

/// Fresh entropy for the app, when the system has it: the source's name
/// (`Event::EntropySource`), then the bytes. Nothing when the system has
/// none.
pub fn entropy() -> Vec<Event> {
    let mut bytes = [0u8; 32];
    match getrandom::fill(&mut bytes) {
        Ok(()) => vec![
            Event::EntropySource(ENTROPY_SOURCE),
            Event::Entropy(EntropyBytes::new(bytes)),
        ],
        Err(_) => Vec::new(),
    }
}

/// Has `app` call the window's loop when its worker has an answer
/// (`docs/WORKERS.md` §7.2): the loop's next pass sends the tick that
/// applies it ([`pace::Pacer::woken`]).
pub fn wake_on_work(
    app: &mut faraday_core::Faraday,
    proxy: &winit::event_loop::EventLoopProxy<()>,
) {
    let proxy = proxy.clone();
    app.set_wake(Box::new(move || {
        // A loop that has ended has no use for the note.
        let _ = proxy.send_event(());
    }));
}
