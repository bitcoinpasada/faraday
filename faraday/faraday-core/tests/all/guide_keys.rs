//! Where the guide opens: every screen and every flow step works out a key,
//! and every key resolves to a page the guide has or to one still to write
//! (`docs/GUIDE-IN-APP.md` §1.3).

use faraday_core::Screen;
use faraday_core::backup_flow::{Kind as FlowKind, Step as FlowStep};
use faraday_core::guide::{GuideAim, GuideAt, guide_aim, guide_key};
use std::collections::BTreeSet;

/// Lists every value of an enum, failing to build when one is added
/// and not listed.
macro_rules! every {
    ($t:ident: $($v:ident),* $(,)?) => {{
        #[allow(dead_code)]
        fn listed(x: $t) {
            match x {
                $($t::$v => {})*
            }
        }
        vec![$($t::$v),*]
    }};
}

fn screens() -> Vec<Screen> {
    every!(Screen: Home, WalletsStart, Wallets, Spend, Files, Entry, Backup,
        Message, CheckMessage, Create, Restore, Settings, SetTime, Randomness, Vaults,
        Backups, CreateVault, KeyGen, Bip85, Convert, KefTool, QrTool, OtsCheck, CoreWallet,
        Silent, Explore, Lightning, Tools, Unlock, VaultContents, Codes, SpendBackup, Vanity,
        Decode, Invoice, NonceCheck, SignerCheck, Compare, Catalog, Transfer, BackupFlow)
}

/// The cards each flow screen has, by the number its step module gives.
fn steps(s: Screen) -> Vec<u8> {
    use faraday_core::{
        convert, core_wallet, cstep, kef_tool, keygen, rstep, spend_backup, vanity, wallet,
    };
    let counted = |n: usize| (0..n as u8).collect();
    match s {
        Screen::Create => counted(cstep::COUNT),
        Screen::KeyGen => counted(keygen::kstep::COUNT),
        Screen::Restore => counted(usize::from(rstep::COUNT)),
        Screen::Spend => (0..=wallet::step::NONCES).collect(),
        Screen::Message => (0..=faraday_core::mstep::SIGN).collect(),
        Screen::Backup => vec![
            faraday_core::qstep::SOFTWARE,
            faraday_core::qstep::PLAN,
            faraday_core::qstep::BOARD,
        ],
        Screen::Bip85 => counted(faraday_core::bip85::pstep::COUNT),
        Screen::QrTool => counted(faraday_core::qr_tool::qstep::COUNT),
        Screen::SpendBackup => vec![
            spend_backup::page::MAP,
            spend_backup::page::HOLDING,
            spend_backup::page::CHECK,
            spend_backup::page::WRITE,
            spend_backup::page::BRING,
        ],
        Screen::Convert => counted(convert::cstep::COUNT),
        Screen::KefTool => counted(kef_tool::kstep::COUNT),
        Screen::CoreWallet => counted(core_wallet::tool::cstep::COUNT),
        Screen::Vanity => counted(vanity::vstep::COUNT),
        _ => Vec::new(),
    }
}

/// The ways in other than a step number: what is open by position in a
/// list, a checklist item, a step of an embedded spend, a code to open.
fn parts(s: Screen) -> Vec<GuideAt> {
    use faraday_core::{bstep, wallet::step};
    match s {
        Screen::Settings => (0..9).map(GuideAt::Part).collect(),
        Screen::VaultContents => (0..6).map(GuideAt::Part).collect(),
        Screen::Entry => (0..4).map(GuideAt::Part).collect(),
        Screen::Backup => [
            bstep::BLANK,
            bstep::PUBLIC,
            bstep::SHEETS,
            bstep::ENVELOPE,
            bstep::FILES,
            bstep::SHOW,
            bstep::STICKS,
            bstep::COPY,
            bstep::CODEX32,
            bstep::PASSPHRASE,
            bstep::KEF,
            bstep::VAULT,
        ]
        .into_iter()
        .map(GuideAt::Item)
        .collect(),
        Screen::QrTool => vec![GuideAt::Opening],
        Screen::SpendBackup => [
            step::TRANSACTION,
            step::TXID,
            step::PATH,
            step::NONCES,
            step::SIGNERS,
            step::SIGN,
            step::COLLECT,
            step::FINISH,
        ]
        .into_iter()
        .map(GuideAt::Inner)
        .collect(),
        _ => Vec::new(),
    }
}

/// Every key the app works out, with the place it belongs to.
fn walk() -> Vec<(&'static str, bool)> {
    let mut out = Vec::new();
    for s in screens() {
        if s == Screen::BackupFlow {
            out.push((guide_key(s, GuideAt::Sheet), true));
            out.push((guide_key(s, GuideAt::Head), true));
            for k in every!(FlowKind: Gpg, SecureBoot, Wallet) {
                out.push((guide_key(s, GuideAt::Kind(k)), false));
                for st in every!(FlowStep: Where, Sticks, Vaults, Make, Lock, Write, Done) {
                    out.push((guide_key(s, GuideAt::Flow(k, st)), true));
                }
            }
            continue;
        }
        out.push((guide_key(s, GuideAt::Whole), false));
        for n in steps(s) {
            out.push((guide_key(s, GuideAt::Step(n)), true));
        }
        for at in parts(s) {
            out.push((guide_key(s, at), true));
        }
    }
    out
}

fn place(key: &str) -> &str {
    key.split('.').next().unwrap_or(key)
}

#[test]
fn every_step_has_a_part_of_its_own() {
    for (key, step) in walk() {
        assert!(
            !step || key.contains('.'),
            "a step opens its place's key {key}"
        );
    }
}

#[test]
fn no_key_is_worked_out_twice() {
    let keys: Vec<_> = walk().into_iter().map(|(k, _)| k).collect();
    let set: BTreeSet<_> = keys.iter().collect();
    assert_eq!(
        set.len(),
        keys.len(),
        "a key stands for two places or steps"
    );
}

#[test]
fn every_key_opens_a_page() {
    let mut missing = BTreeSet::new();
    for (key, _) in walk() {
        if guide_aim(key) == GuideAim::Contents {
            missing.insert(place(key));
        }
    }
    assert!(missing.is_empty(), "places with no page: {missing:?}");
}

#[test]
fn a_part_without_a_section_opens_its_place() {
    // Home has its lead only.
    assert_eq!(guide_aim("home"), GuideAim::Section("home"));
    assert_eq!(guide_aim("home.nothing"), GuideAim::Lead("home"));
    assert_eq!(guide_aim("no-such-place.part"), GuideAim::Contents);
}
