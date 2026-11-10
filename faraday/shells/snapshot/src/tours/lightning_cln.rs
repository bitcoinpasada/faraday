//! The Core Lightning source of the Lightning node key tool
//! (`docs/DESIGN.md` §5).

use crate::tour::{Tour, scroll_to, type_text};
use faraday_core::testkit;
use faraday_core::{Action, Code, Screen};
use faraday_shell_api::{App, Event, Key};

/// Core Lightning's node from a loaded key, and its `hsm_secret` from
/// Files: the key's node, the source with no file in
/// the Inbox, an encrypted file chosen and asking for its passphrase, a
/// wrong passphrase's line, and the node it opens to.
pub(crate) fn lightning_cln_tour(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Entry(None));
    t.type_key(0);
    let fp = t
        .app
        .session
        .keys
        .last()
        .ok_or("test key 1 did not load")?
        .fingerprint
        .0;
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::Lightning);
    t.press(Action::LCln(Some(fp)));
    t.shot("cln-key")?;
    t.press(Action::LCln(None));
    t.shot("cln-no-file")?;
    testkit::inbox_file(
        &mut t.app,
        "hsm_secret",
        include_bytes!("../../../../faraday-samples/data/hsm-secret-encrypted").to_vec(),
    );
    t.press(Action::LCln(None));
    t.shot("cln-files")?;
    let at = t
        .app
        .inbox()
        .iter()
        .position(|i| i.name == "hsm_secret")
        .ok_or("the sample file is not in Files")?;
    t.press(Action::LFile(at));
    scroll_to(t, Action::LResolve);
    t.shot("cln-asks")?;
    t.press(Action::LPassphrase);
    type_text(t, "faraday1");
    t.press(Action::LResolve);
    testkit::settle(&mut t.app);
    scroll_to(t, Action::LResolve);
    t.shot("cln-wrong")?;
    t.app.event(Event::Key(Key::Backspace));
    t.shot("cln-typed")?;
    t.press(Action::LResolve);
    testkit::settle(&mut t.app);
    scroll_to(t, Action::LOut);
    t.shot("cln-node")?;
    t.press(Action::LShow);
    t.press(Action::ShowCode(Code::NodeId));
    t.shot("cln-code")
}
