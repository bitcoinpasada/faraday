//! The convert tour.

use crate::tour::{Tour, type_text};
use faraday_core::convert::{self, ConvertAction as C, ROWS, cstep};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StorageEvent};

/// Tools' tile for the flow.
fn open(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|tile| tile.name == "Convert a seed")
        .ok_or("no Convert a seed tile")?;
    t.press(Action::Catalog(tile as u8));
    Ok(())
}

/// Convert a seed (`docs/SEED-CONVERSION.md` §6): From with no key, From
/// with an open vault's key, and that key loaded; To for a key of words; the
/// xprv's Result masked and shown; Check my copy typing, wrong and right; and
/// the Secret out sheet; Seed XOR's Settings card and its result of parts; a key
/// of seed bytes, SLIP-39 k of n's Settings card and its result of groups.
pub(crate) fn convert_tour(t: &mut Tour) -> Result<(), String> {
    open(t)?;
    t.shot("convert-from-empty")?;
    let vault = testkit::files()?
        .into_iter()
        .find(|(n, _)| n == "vault.kdbx")
        .ok_or("no test vault")?;
    t.app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![vault]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut t.app, Some(180));
    t.press(Action::Vault(V::OpenFrom(0, Screen::Catalog)));
    type_text(t, testkit::VAULT_PASSPHRASE);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..10 {
        if !t.app.vaults().open.is_empty() {
            break;
        }
        t.tick();
    }
    open(t)?;
    t.shot("convert-from-vault")?;
    let record = t
        .app
        .vaults()
        .records(t.app.vaults().open[0].id)
        .iter()
        .position(|(_, r)| r.kind() == faraday_vault::records::kind::KEY)
        .ok_or("no key in the test vault")?;
    t.press(Action::Convert(C::VaultKey(0, record)));
    t.shot("convert-to")?;
    t.press(Action::Convert(C::Step(cstep::FROM)));
    t.shot("convert-from")?;
    t.press(Action::Convert(C::Step(cstep::TO)));
    let xprv = ROWS
        .iter()
        .position(|r| r.name == "xprv")
        .ok_or("no xprv row")?;
    t.press(Action::Convert(C::To(xprv as u8)));
    t.shot("convert-result")?;
    t.press(Action::Convert(C::Show));
    t.shot("convert-result-shown")?;
    t.press(Action::Convert(C::Check));
    type_text(t, "xprv9s21");
    t.shot("convert-check")?;
    t.press(Action::Convert(C::Compare));
    t.shot("convert-check-differs")?;
    let value = convert::shown(&t.app).ok_or("no value")?.to_string();
    type_text(t, &value);
    t.press(Action::Convert(C::Compare));
    t.shot("convert-check-matches")?;
    t.press(Action::Convert(C::Save));
    t.shot("convert-save")?;
    t.press(Action::Cancel);
    // A text target and a code, each masked then shown; the key's own
    // Explore page, where the flow opens on it.
    for (row, name) in [("The words", "words"), ("SeedQR", "seedqr")] {
        t.press(Action::Convert(C::Step(cstep::TO)));
        let at = ROWS
            .iter()
            .position(|r| r.name == row)
            .ok_or("no such row")?;
        t.press(Action::Convert(C::To(at as u8)));
        t.shot(&format!("convert-{name}"))?;
        t.press(Action::Convert(C::Show));
        t.shot(&format!("convert-{name}-shown"))?;
    }
    // A target with settings, and a result of several parts checked one
    // part at a time.
    t.press(Action::Convert(C::Step(cstep::TO)));
    let xor = ROWS
        .iter()
        .position(|r| r.name == "Seed XOR parts")
        .ok_or("no Seed XOR row")?;
    t.press(Action::Convert(C::To(xor as u8)));
    t.shot("convert-settings")?;
    // The first press asks for the randomness, which the tour answers.
    t.press(Action::Convert(C::Make));
    t.tick();
    t.press(Action::Convert(C::Make));
    t.shot("convert-set")?;
    t.press(Action::Convert(C::Show));
    t.shot("convert-set-shown")?;
    let part = convert::shown_parts(&t.app).ok_or("no parts")?[1].to_string();
    t.press(Action::Convert(C::Check));
    type_text(t, &part);
    t.press(Action::Convert(C::Compare));
    t.shot("convert-set-matches")?;
    let fp = t.app.session.keys[0].fingerprint.0;
    t.press(Action::Convert(C::Done));
    t.press(Action::Explore);
    t.press(Action::XKey(fp));
    t.shot("convert-way-in-explore")?;
    t.press(Action::Convert(C::OpenOn(fp)));
    t.shot("convert-opened-on-key")?;
    t.press(Action::Convert(C::Done));
    slip_tour(t)
}

/// A key of seed bytes: the SLIP-39 k of n Settings card, a result of
/// groups, and a share checked by its group.
fn slip_tour(t: &mut Tour) -> Result<(), String> {
    let seed = [0x5au8; 16];
    t.app
        .session
        .keys
        .add_seed(&testkit::compute(), &seed, "Seed")
        .map_err(|_| "no seed key")?;
    let fp = t.app.session.keys[t.app.session.keys.len() - 1]
        .fingerprint
        .0;
    t.press(Action::Convert(C::OpenOn(fp)));
    t.shot("convert-seed-to")?;
    let slip = ROWS
        .iter()
        .position(|r| r.name == "SLIP-39, k of n")
        .ok_or("no SLIP-39 k of n row")?;
    t.press(Action::Convert(C::To(slip as u8)));
    t.shot("convert-slip-settings")?;
    t.press(Action::Convert(C::SlipGroups(1)));
    t.shot("convert-slip-groups")?;
    t.press(Action::Convert(C::Make));
    t.tick();
    t.press(Action::Convert(C::Make));
    t.press(Action::Convert(C::Show));
    t.shot("convert-slip-result")?;
    let share = convert::shown_parts(&t.app).ok_or("no shares")?[4].to_string();
    t.press(Action::Convert(C::Check));
    type_text(t, &share);
    t.press(Action::Convert(C::Compare));
    t.shot("convert-slip-matches")?;
    aezeed_tour(t)
}

/// LND's published cipher seed with no passphrase
/// (`tools/reference/aezeed/README.md`).
const AEZEED: &str = "above judge emerge veteran reform crunch system all snap please shoulder \
    vault hurt city quarter cover enlist swear success suggest drink wagon enrich body";

/// An LND aezeed as the source (`docs/SEED-CONVERSION.md` §4.5): From with
/// its row, the entry, and a result.
fn aezeed_tour(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Convert(C::Done));
    open(t)?;
    t.shot("convert-aezeed-from")?;
    t.press(Action::Convert(C::Aezeed));
    type_text(t, AEZEED);
    t.shot("convert-aezeed-entry")?;
    {
        use faraday_shell_api::{App, Event, Key};
        t.app.event(Event::Key(Key::Enter));
    }
    t.tick();
    testkit::settle(&mut t.app);
    t.tick();
    let seed = ROWS
        .iter()
        .position(|r| r.name == "Master seed, hex")
        .ok_or("no master seed row")?;
    t.press(Action::Convert(C::To(seed as u8)));
    t.tick();
    t.press(Action::Convert(C::Show));
    t.shot("convert-aezeed-result")?;
    set_tour(t)
}

/// SLIP-39's published case 4 (`tools/vectors/slip39`): a 2 of 3 group,
/// the shares of members 1 and 3.
const SET_A: &str = "shadow pistol academic acid actress prayer class unknown daughter sweater \
    depict flip twice unkind craft early superior advocate guest smoking";
const SET_B: &str = "shadow pistol academic always adequate wildlife fancy gross oasis cylinder \
    mustang wrist rescue view short owner flip making coding armed";

/// Hold one typed share with Enter.
fn hold_share(t: &mut Tour, share: &str) {
    use faraday_shell_api::{App, Event, Key};
    type_text(t, share);
    t.app.event(Event::Key(Key::Enter));
}

/// Make from a Settings card: the first press asks for the randomness,
/// which the tour answers.
fn make(t: &mut Tour) {
    t.press(Action::Convert(C::Make));
    t.tick();
    t.press(Action::Convert(C::Make));
}

/// A SLIP-39 set as the source (`docs/SEED-CONVERSION.md` §8): the entry
/// with shares held, Add, Replace and Rotate's Settings, and a Result.
fn set_tour(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Convert(C::Done));
    open(t)?;
    t.press(Action::Convert(C::SetSource));
    hold_share(t, SET_A);
    hold_share(t, SET_B);
    type_text(t, "shadow pistol acad");
    crate::tour::scroll(t, 400);
    t.shot("convert-set-entry")?;
    t.press(Action::Convert(C::SetAct(convert::SetAct::Add)));
    t.shot("convert-set-add")?;
    make(t);
    t.press(Action::Convert(C::Show));
    t.shot("convert-set-add-result")?;
    t.press(Action::Convert(C::Step(cstep::TO)));
    t.press(Action::Convert(C::SetAct(convert::SetAct::Replace)));
    t.shot("convert-set-replace")?;
    t.press(Action::Convert(C::Step(cstep::TO)));
    t.press(Action::Convert(C::SetAct(convert::SetAct::Rotate)));
    t.shot("convert-set-rotate")?;
    t.press(Action::Convert(C::Lost(0, 2)));
    t.shot("convert-set-rotate-limit")?;
    t.press(Action::Convert(C::Lost(0, -2)));
    t.press(Action::Convert(C::Destroy(0, 0)));
    t.press(Action::Convert(C::Destroy(0, 2)));
    t.shot("convert-set-rotate-destroy")?;
    t.press(Action::Convert(C::DestroyOthers));
    t.shot("convert-set-rotate-shape")?;
    make(t);
    t.press(Action::Convert(C::Show));
    t.shot("convert-set-rotate-result")?;
    Ok(())
}
