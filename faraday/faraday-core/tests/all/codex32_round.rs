//! The round after a lock types a codex32 string it does not hold in
//! place (`docs/CODEX32.md` §9.5, `docs/BACKUP-FLOW.md` §17.23), as a
//! person backing up would notice it, against BIP 93's vector 2
//! (`tools/vectors/bip93/vectors.json`): after a power-on **Copy the
//! key** names the string the vault takes and offers typing it from its
//! copy on Add a key's codex32 form; the right string typed, the copy goes
//! on and the vault's stick holds it; a string of another set, another
//! share of the set or a set of another seed is refused by name and
//! nothing is written.

use crate::backup_flow_wallet::{
    PHRASE as FLOW_PHRASE, flow_into_vault, ofv, restart, step, write_sticks,
};
use crate::codex32_kept::{A, D, FP, ix, key_from_strings, share_records};
use crate::codex32_vault_set::{BIP93, SECRET, locked_with, sealed_strings};
use crate::common::*;
use faraday_bip::codex32::Codex32;
use faraday_core::backup_flow::Step;
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::board::Thing;
use faraday_core::forms::Form;
use faraday_core::{Action, Faraday, Screen, testkit};
use faraday_shell_api::{App, Event, Key};

/// The plan changed after the lock to put string D in the vault, the
/// round's vault unlocked after a power-on, the key typed back from
/// strings A and C, and **Copy the key** pressed: it stops on D, which
/// this power-on does not hold, and offers typing it.
fn round_stops_on_d(shell: &mut Shell, app: &mut Faraday) {
    shell.flow(app, F::ChangePlan);
    flow_into_vault(shell, app, 0, Thing::String(0, ix('d')));
    assert!(
        app.flow().unwrap().adding,
        "a change of the vault is a round"
    );
    assert_eq!(step(app), Step::Vaults);
    unlock(app, FLOW_PHRASE);
    assert_eq!(step(app), Step::Make, "{:?}", unlock_error(app));
    // The key, wiped at the lock, typed back from strings A and C with Add
    // the key from its copy; D is on paper elsewhere.
    key_from_strings(app);
    app.press(Action::Nav(Screen::BackupFlow));
    let k = app
        .session
        .keys
        .position_of(FP)
        .expect("the key typed back");
    assert!(
        !app.session.keys.holds_codex32(k, b'd'),
        "a power-on holds no D"
    );
    shell.flow(app, F::CopyKey);
    assert!(app.flow().unwrap().adding, "the copy waits on D");
    let wanted = app.flow().unwrap().live.c32_wanted.clone();
    assert_eq!(wanted.map(|w| w.index), Some(b'd'), "the round names D");
    shell.flow(app, F::TypeString);
    assert_eq!(app.screen, Screen::Entry, "typed in place");
    assert_eq!(testkit::entry(app).form, Form::Codex32);
}

/// `s` typed on Add a key's codex32 form, then Enter.
fn type_string(app: &mut Faraday, s: &str) {
    type_text(app, s);
    app.event(Event::Key(Key::Enter));
}

/// What the entry says of the last string typed.
fn refusal(app: &Faraday) -> String {
    testkit::entry(app).error.clone().unwrap_or_default()
}

/// Share `index` of a 2-of-3 set named `id` over `seed`.
fn share_of(seed: &[u8], id: &[u8; 4], index: u8) -> String {
    let secret = Codex32::from_seed(2, *id, seed).unwrap();
    let shares = secret.split(2, 3, &[0x11; 64]).unwrap();
    let s = shares.iter().find(|s| s.share_index() == index).unwrap();
    s.encode().as_str().to_string()
}

/// Vector 2's seed.
fn vector_seed() -> Vec<u8> {
    assert!(BIP93.contains(SECRET));
    let s = Codex32::parse(SECRET).unwrap().to_seed().unwrap();
    s.expose().bytes().to_vec()
}

/// After a lock and a fresh start, the round reaches Copy the key for a
/// codex32 plan, offers typing the string it does not hold, and with the
/// right string typed copies it into the vault and writes it: the vault
/// sealed on its stick holds D.
#[test]
fn after_a_power_on_the_round_types_the_missing_string_in_place_and_writes_it() {
    let mut shell = Shell::new();
    let mut app = locked_with(&mut shell, &[]);
    round_stops_on_d(&mut shell, &mut app);
    type_string(&mut app, D);
    assert_eq!(app.screen, Screen::BackupFlow, "{}", refusal(&app));
    assert!(!app.flow().unwrap().adding, "the copy went on");
    assert!(app.flow().unwrap().live.c32_wanted.is_none());
    assert_eq!(
        share_records(&app, 0),
        vec![(D.as_bytes().to_vec(), FP.to_vec())]
    );
    shell.flow(&mut app, F::MadeContinue);
    assert_eq!(step(&app), Step::Lock, "{:?}", app.toast_text());
    shell.flow(&mut app, F::LockWrite);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1"]);
    assert_eq!(
        step(&app),
        Step::Done,
        "{:?}",
        app.flow().unwrap().live.problem
    );
    let sealed = ofv(&shell, "S1").expect("Vault 1 on S1");
    assert_eq!(
        sealed_strings(&sealed, FLOW_PHRASE),
        vec![D.as_bytes().to_vec()],
        "the sealed copy holds D"
    );
}

/// A string of another set, another share of the set, or the set's share
/// D over another seed is refused with its reason, naming it, and nothing
/// is written: the vault holds what it held, the session does not hold
/// D, and the round still waits on it. Vault 1 keeps string A, so D
/// typed rebuilds the set and its seed is checked.
#[test]
fn a_string_of_another_set_or_seed_is_refused_by_name_and_nothing_is_written() {
    let mut shell = Shell::new();
    let mut app = locked_with(&mut shell, &['a']);
    round_stops_on_d(&mut shell, &mut app);
    let before = share_records(&app, 0);
    assert_eq!(before, vec![(A.as_bytes().to_vec(), FP.to_vec())]);
    let seed = vector_seed();
    let other_set = share_of(&seed, b"w0za", b'd');
    let other_seed = share_of(&[0x42; 16], b"name", b'd');
    assert_ne!(other_seed, D);
    let cases: [(&str, &[&str]); 3] = [
        (&other_set, &["w0za", "name"]),
        (crate::codex32_kept::C, &["Share C", "D"]),
        (&other_seed, &["D", "name", "another seed", "fab6868a"]),
    ];
    for (typed, names) in cases {
        type_string(&mut app, typed);
        assert_eq!(app.screen, Screen::Entry, "{typed} refused");
        let line = refusal(&app);
        for n in names {
            assert!(line.contains(n), "{line:?} names {n}");
        }
        assert!(
            testkit::entry(&app).parts.codex32.is_empty(),
            "nothing kept"
        );
        let k = app.session.keys.position_of(FP).unwrap();
        assert!(!app.session.keys.holds_codex32(k, b'd'));
        assert_eq!(share_records(&app, 0), before, "nothing written");
    }
    app.press(Action::Nav(Screen::BackupFlow));
    assert_eq!(step(&app), Step::Make);
    assert!(app.flow().unwrap().adding, "the round still waits on D");
}
