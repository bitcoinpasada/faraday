//! The keys tour.

use crate::tour::{Tour, scroll, type_text};
use faraday_core::{Action, Screen};

/// Keys in no wallet on Wallets: a key beside the test kit's wallets,
/// with the column scrolled to its buttons; then one key added to an
/// empty session, the wallet made from it, and two keys.
pub(crate) fn keys_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::seeds::SeedsAction as S;
    let abandon = "abandon abandon abandon abandon abandon abandon \
                   abandon abandon abandon abandon abandon about";
    t.load_kit()?;
    t.press(Action::Entry(None));
    type_text(t, abandon);
    t.press(Action::EntryAdd);
    t.press(Action::Nav(Screen::Wallets));
    t.shot("wallets-kit-and-key")?;
    scroll(t, 1200);
    t.shot("wallets-kit-and-key-foot")?;
    t.press(Action::Lock);
    t.sticks(false);
    t.tick();
    t.press(Action::Entry(None));
    type_text(t, abandon);
    t.press(Action::EntryAdd);
    t.shot("wallets-one-key")?;
    let fp = t
        .app
        .session
        .keys
        .first()
        .map(|k| k.fingerprint.0)
        .ok_or("no key added")?;
    t.press(Action::KeyWallet(fp, 1));
    t.shot("from-key-shape")?;
    let steps = if t.size().0 < 700 { 5 } else { 2 };
    for k in 1..=steps {
        scroll(t, 300);
        t.shot(&format!("from-key-shape-{k}"))?;
    }
    t.press(Action::Nav(Screen::Wallets));
    t.press(Action::KeyWallet(fp, 2));
    t.shot("from-key-two")?;
    t.press(Action::Nav(Screen::Wallets));
    t.press(Action::Entry(None));
    t.type_key(1);
    t.press(Action::Nav(Screen::Wallets));
    t.shot("wallets-two-keys")?;
    t.press(Action::KeyWallet(fp, 1));
    t.press(Action::Seeds(S::Make));
    t.press(Action::Nav(Screen::Wallets));
    t.shot("wallets-one-wallet-one-key")?;
    // The small panel's wallet list, where the loose key has its row.
    t.press(Action::Nav(Screen::WalletsStart));
    t.shot("wallet-list-one-wallet-one-key")?;
    child_seed(t, fp)
}

/// BIP-85 child words from key `fp` loaded as a key: Record it as a
/// child seed?, then a wallet made over the child, recorded, and A BIP-85
/// child seed (F166).
fn child_seed(t: &mut Tour, fp: [u8; 4]) -> Result<(), String> {
    use faraday_core::cstep;
    t.press(Action::Bip85);
    t.press(Action::PKey(fp));
    t.press(Action::PApp(0));
    t.press(Action::PNext);
    t.press(Action::PNext);
    t.press(Action::PLoad);
    t.shot("bip85-record")?;
    let before = t.app.session.keys.len();
    t.press(Action::PRecord(true));
    let child = t
        .app
        .session
        .keys
        .all()
        .nth(before)
        .map(|k| k.fingerprint.0)
        .ok_or("no child loaded")?;
    t.press(Action::CreateWallet);
    t.press(Action::CKind(0));
    t.press(Action::CNext(cstep::KIND));
    t.press(Action::CVaults(false));
    t.press(Action::CNext(cstep::WHERE));
    t.press(Action::CSlotHere(0, child));
    t.press(Action::CNext(cstep::KEYS));
    t.shot("child-notice")?;
    t.press(Action::Cancel);
    Ok(())
}
