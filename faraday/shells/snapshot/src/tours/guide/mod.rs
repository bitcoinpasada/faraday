//! The guide tour: the pictures of `docs/guide/walk/`, one module for
//! each walkthrough, each shot named as its picture's file
//! (`just guide-pictures`). A shot that marks the control pressed next
//! writes its rectangle beside the picture (`Tour::shot_marking`).

mod back_up;
mod check_and_decode;
mod codex32;
mod faraday_demo;
mod first_start;
mod gpg;
mod make_a_wallet;
mod messages;
mod more_from_a_key;
mod multisig_with_others;
mod new_key;
mod receive;
mod restore;
mod secure_boot_keys;
mod spend;
mod spend_from_a_backup;
mod vaults_and_passwords;

pub(crate) use faraday_demo::demo_session;
pub(crate) use more_from_a_key::on_the_key;

use crate::tour::{Tour, type_text};
use faraday_core::create::NewKind;
use faraday_core::{Action, Screen, StickInfo, StorageEvent};
use faraday_crypto::work::Compute;
use faraday_shell_api::App;

/// Costs what the parts it runs cost. A fresh stick session as a person
/// meets it (first start), then New key made from the dice transcript;
/// a single-key wallet from that key, backed up; a 2-of-3 over it and
/// two test keys, and a key scanned for it; an address checked, and that
/// wallet's spend by QR, with two warnings; then a lock for a stick, the
/// wallet restored from the description on it, and the PSBT on it signed
/// after the second key is scanned in; a single key backed up in codex32
/// shares and Add a key's codex32 form, each in a session of its own;
/// a vault made, filled, written and unlocked again; a spend from what a
/// backup's vault holds; a GPG key and Secure Boot keys made in the
/// backup flow, a file signed and a boot program looked at before
/// signing; last, Faraday Demo's Home and Practice's codex32 sample.
pub(crate) fn guide_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    first_start::run(t)?;
    new_key::run(t)?;
    back_up::run(c, t)?;
    make_a_wallet::multisig(t)?;
    make_a_wallet::multisig_board(t)?;
    multisig_with_others::scan_key(c, t)?;
    // Each part from here is tried, and every one that fails is named.
    let parts: [(&str, Part); 16] = [
        ("receive", receive::receive),
        ("spend by QR", spend::spend_by_qr),
        ("warnings", spend::spend_warnings),
        ("stick", spend::stick_in),
        ("restore", restore::restore_wallet),
        ("spend from the stick", spend::spend_from_stick),
        ("backup in shares", codex32::shares_backup),
        ("codex32 form", make_a_wallet::codex32_form),
        ("corrected key", make_a_wallet::corrected_key),
        ("more from a key", |_, t| more_from_a_key::run(t)),
        ("check and decode", |_, t| check_and_decode::run(t)),
        ("messages", |_, t| messages::run(t)),
        ("vaults and passwords", |_, t| vaults_and_passwords::run(t)),
        ("spend from a backup", |_, t| spend_from_a_backup::run(t)),
        ("gpg key", |_, t| gpg::run(t)),
        ("secure boot keys", |_, t| secure_boot_keys::run(t)),
    ];
    let failed: Vec<String> = parts
        .into_iter()
        .filter_map(|(name, part)| part(c, t).err().map(|e| format!("{name}: {e}")))
        .collect();
    if !failed.is_empty() {
        return Err(failed.join("; "));
    }
    faraday_demo::desktop_home(t)
}

/// BIP 93's vector 2 (`NAME`, 2 of 3): shares A and C, and D made from
/// them (`tools/vectors/bip93/vectors.json`).
const NAME: [&str; 3] = [
    "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm",
    "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn",
    "ms12namedll4f8jlh4e5vdvuldlfxu2jhdnlsm97xvenrxeg",
];

/// A fresh stick session on Mainnet with the guide's key (3dca6805)
/// typed into Add a key and added.
fn key_session(t: &mut Tour) {
    crate::tours::screens::stick_session(t);
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::Entry(None));
    let _ = t.app.frame();
    type_text(t, DICE_WORDS);
    t.press(Action::EntryAdd);
}

/// A fresh session with no stick in, on Wallets.
fn fresh_session(t: &mut Tour) {
    t.restart();
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.press(Action::Nav(Screen::WalletsStart));
}

/// One part of the guide's §6 and §7.
type Part = fn(&Compute, &mut Tour) -> Result<(), String>;

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

/// Create a vault's passphrase typed twice, the vault made, and unlocked
/// with it, back on the checklist.
fn vault_made(t: &mut Tour) {
    use faraday_core::vault::VaultAction as V;
    for second in [false, true] {
        t.press(Action::Vault(V::CFocus(0, second)));
        type_text(t, "guide vault phrase");
    }
    t.press(Action::Vault(V::CGo));
    for _ in 0..10 {
        if t.app.screen == Screen::Unlock {
            break;
        }
        t.tick();
    }
    type_text(t, "guide vault phrase");
    t.press(Action::Vault(V::Unlock));
    for _ in 0..10 {
        if !t.app.vaults().open.is_empty() && t.app.screen == Screen::Backup {
            break;
        }
        t.tick();
    }
}

/// The checklist's item open now.
fn open_item(t: &Tour) -> Option<u8> {
    t.app.backup().and_then(|b| b.open)
}

/// The one lock, then each stick the plan names, as the flow asks; the
/// shots named from `name` (`<name>-insert-0`, `<name>-read-0`,
/// `<name>-written-0`, `<name>-done`), or by the picture `pictures`
/// gives such a name.
fn write_sticks(t: &mut Tour, name: &str, pictures: &[(&str, &str)]) -> Result<(), String> {
    use faraday_core::backup_flow::steps::FlowAction as F;
    let shot = |what: String| {
        pictures
            .iter()
            .find(|(n, _)| *n == what)
            .map_or(what.clone(), |(_, p)| (*p).to_string())
    };
    t.press(Action::Flow(F::WalletLock));
    let (inbox, outbox, kept) = t.boxes.clone().ok_or("nothing was kept by the lock")?;
    t.app.storage(StorageEvent::Restored {
        inbox,
        outbox,
        kept,
    });
    t.pump();
    t.shot(&shot(format!("{name}-after-lock")))?;
    for n in 0..6 {
        let slot = t
            .app
            .flow()
            .and_then(|f| f.slots.iter().position(|s| s.written.is_none()));
        let Some(slot) = slot else { break };
        t.shot(&shot(format!("{name}-insert-{n}")))?;
        let stick = StickInfo {
            serial: Some(0x2000 + slot as u32),
            id: format!("s{slot}"),
            label: "KEYSTICK".into(),
            boot: false,
            files: Vec::new(),
        };
        t.app.storage(StorageEvent::Sticks(vec![stick]));
        t.pump();
        let read = shot(format!("{name}-read-{n}"));
        t.shot(&read)?;
        for k in 0..2 {
            let hold = t
                .app
                .flow()
                .and_then(|f| f.live.reading.as_ref())
                .is_some_and(|r| r.hold);
            let press = Action::Flow(if hold { F::Hold } else { F::Tap });
            if k == 0 {
                t.mark(&read, press)?;
            }
            t.app.press(press);
            t.pump();
        }
        t.shot(&shot(format!("{name}-written-{n}")))?;
        t.app.storage(StorageEvent::Sticks(Vec::new()));
        t.pump();
    }
    for _ in 0..5 {
        t.tick();
    }
    t.shot(&shot(format!("{name}-done")))
}

/// A press on Make codex32 shares.
fn c32(t: &mut Tour, p: faraday_core::codex32::C32) {
    t.press(Action::Codex32(p));
}

/// `text` typed on Make codex32 shares' keyboard.
fn c32_type(t: &mut Tour, text: &str) {
    use faraday_core::codex32::{C32, OskPress};
    for ch in text.chars() {
        c32(t, C32::Key(OskPress::Char(ch)));
    }
}

/// Costs a mnemonic's checksum. The SeedQR digits of `words`, as a
/// camera reads them off the code.
fn seedqr(c: &Compute, words: &str) -> Result<Vec<u8>, String> {
    let m = faraday_bip::bip39::Mnemonic::parse(c, faraday_bip::bip39::Language::English, words)
        .map_err(|e| format!("{e:?}"))?;
    Ok(faraday_codec::seedqr::to_digits(&m)
        .expose()
        .as_bytes()
        .to_vec())
}

/// The wallet the guide made last, by its place in the session.
fn last_wallet(t: &Tour) -> Result<usize, String> {
    t.app
        .session
        .wallets
        .len()
        .checked_sub(1)
        .ok_or_else(|| "no wallet".to_string())
}

/// The wallet at `w` as a test kit's wallet, its spend paying `fee`.
fn kit_of(t: &Tour, w: usize, fee: u64) -> faraday_core::testkit::Kit {
    faraday_core::testkit::Kit {
        id: "guide",
        name: "New wallet 1",
        descriptor: t.app.session.wallets[w]
            .policy()
            .to_descriptor_checksummed(),
        cosigner: Some(1),
        fee,
    }
}
