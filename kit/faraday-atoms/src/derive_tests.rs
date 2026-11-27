//! `#[derive(Pressed)]` and `#[hides]` as a feature's press type uses them (04 §11.1).

use crate::Pressed;

/// A page's presses: some hide a secret.
#[derive(Clone, PartialEq, crate::Pressed)]
#[allow(dead_code)]
enum Press {
    Back,
    #[hides]
    Hide,
    /// Show pressed again.
    #[hides]
    Show(bool),
    Pick {
        at: usize,
    },
    #[hides]
    Wipe {
        all: bool,
    },
    Done,
}

/// Invariant: A press marked `#[hides]` reports that it hides a secret, whatever it holds, and no other press does.
/// Regression: The derive marks the wrong variant, or skips one with a payload, so a hiding press leaves the secret on the next frame.
/// Owner: kit
#[test]
fn only_the_marked_presses_hide() {
    assert!(Press::Hide.hides());
    assert!(Press::Show(true).hides());
    assert!(Press::Wipe { all: false }.hides());
    assert!(!Press::Back.hides());
    assert!(!Press::Pick { at: 1 }.hides());
    assert!(!Press::Done.hides());
}
