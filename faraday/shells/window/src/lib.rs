//! What the desktop app (`faraday/shells/desktop`) and the Starter
//! (`faraday/shells/starter`) share to run in a window, so the two behave
//! alike: the frame pacing ([`pace`]), the input ([`input`]), the copy of
//! a frame into the window ([`view`]), and the camera with the scanner
//! that reads its frames ([`camera`], [`lens`]). Each shell keeps its own
//! loop and what only it does (the desktop's stick folders, the Starter's
//! disks).

pub mod camera;
pub mod input;
pub mod lens;
pub mod pace;
pub mod view;

use osk_shell_api::{EntropyBytes, Event};

/// Fresh entropy for the app, when the system has it.
pub fn entropy() -> Option<Event> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)
        .ok()
        .map(|()| Event::Entropy(EntropyBytes::new(bytes)))
}
