//! Sessions other tours start from: a stick session on Mainnet, with the
//! sample dice key added or not, and Faraday Demo as it opens.

use crate::tour::{Tour, display, laptop, type_text};
use faraday_core::create::NewKind;
use faraday_core::{Action, Faraday, Medium, Screen, StorageEvent};
use faraday_crypto::work::Compute;
use faraday_shell_api::App;

/// A fresh stick session on Mainnet, as the guide's pictures show it: a
/// laptop named for no maker, Secure Boot off at start, no stick in.
pub(crate) fn stick_session(t: &mut Tour) {
    let size = t.size();
    t.app = Faraday::new();
    t.app.medium = Medium::Stick;
    t.app.devices = laptop();
    t.app.secure_boot = faraday_core::SecureBootAtStart::Off;
    let _ = faraday_core::testkit::move_to(&mut t.app.session, faraday_core::testkit::net());
    t.app.event(crate::tour::display(size, t.dpi));
    faraday_core::testkit::vault_speed(&mut t.app, Some(180));
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
}

/// A fresh stick session on Mainnet with the guide's key (3dca6805)
/// typed into Add a key and added.
pub(crate) fn key_session(t: &mut Tour) {
    stick_session(t);
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::Entry(None));
    let _ = t.app.frame();
    type_text(t, DICE_WORDS);
    t.press(Action::EntryAdd);
}

/// Faraday Demo as it opens: Practice on testnet, online, no stick in.
pub(crate) fn demo_session(t: &mut Tour) {
    let size = t.size();
    t.app = Faraday::new();
    t.app.event(display(size, t.dpi));
    t.app.online = true;
    t.app.seed_line = true;
    t.app.session = faraday_core::wallet::Session::on(faraday_bip::keys::Network::Testnet);
    t.app.session.set_demo(&faraday_samples::SAMPLES);
    t.app.samples = Some(&faraday_samples::SAMPLES);
    t.app.set_modes(faraday_core::DESKTOP_MODES);
    t.press(Action::Nav(Screen::Home));
}

/// Costs a key's account derived. The guide's single-key wallet over its
/// key (loaded first in this session) as a test kit's wallet, its spend
/// paying `fee`: the wallet a coordinator such as Sparrow holds.
pub(crate) fn single_kit(
    c: &Compute,
    t: &Tour,
    fee: u64,
) -> Result<faraday_core::testkit::Kit, String> {
    let key = crate::tour::key_text_of(c, &t.app.session.keys, 0, NewKind::NativeSegwit)?;
    Ok(faraday_core::testkit::Kit {
        id: "guide",
        name: "New wallet 1",
        descriptor: format!("wpkh({key}/<0;1>/*)"),
        cosigner: None,
        fee,
    })
}

/// The Inbox as it is, `files` added after it, and `outbox` For the
/// stick, as a stick visit or a lock leaves them.
pub(crate) fn files_in(t: &mut Tour, files: &[(&str, Vec<u8>)], outbox: &[(&str, Vec<u8>)]) {
    let own = |f: &[(&str, Vec<u8>)]| -> Vec<(String, Vec<u8>)> {
        f.iter()
            .map(|(n, b)| ((*n).to_string(), b.clone()))
            .collect()
    };
    let inbox = t
        .app
        .inbox()
        .iter()
        .map(|i| (i.name.clone(), i.bytes.to_vec()))
        .chain(own(files))
        .collect();
    t.app.storage(StorageEvent::Restored {
        inbox,
        outbox: own(outbox),
        kept: Vec::new(),
    });
    t.pump();
}

/// The dice key's words, as §3 makes them.
pub(crate) const DICE_WORDS: &str =
    "brand hobby ranch shoulder brass hockey ranch short brand hockey random torch";

/// `kind`'s number, as `Action::CKind` carries it.
pub(crate) fn kind_at(kind: NewKind) -> Result<u8, String> {
    NewKind::ALL
        .iter()
        .position(|k| *k == kind)
        .map(|i| i as u8)
        .ok_or_else(|| format!("{kind:?} is not offered"))
}
