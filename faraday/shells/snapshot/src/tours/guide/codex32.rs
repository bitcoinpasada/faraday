//! The pictures of `docs/guide/walk/codex32.md`.

use super::{NAME, c32, c32_type, fresh_session, kind_at, open_item, vault_made, write_sticks};
use crate::tour::Tour;
use faraday_core::board::{BoardPress as BP, Chip, Thing, Zone};
use faraday_core::codex32::{C32, OskPress, WAYS, Way as C32Way};
use faraday_core::create::NewKind;
use faraday_core::{Action, Screen, cstep};
use faraday_crypto::work::Compute;

/// Costs what making a codex32 set and its wallet costs. §4.11: Make a
/// new wallet, Single key, backup in shares at 2 of 3:
/// Quorum, Keys, its slot opening Make codex32 shares on the identifier,
/// vector 2's characters typed and its three strings typed back, Strings
/// made, Lock in; Check; the board's standard way; the checklist's
/// codex32 item.
pub(super) fn shares_backup(_c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::{bstep, plan};
    fresh_session(t);
    t.press(Action::CreateWallet);
    // Kind opens with this kind ticked; pressed, it moves on (F103).
    t.press(Action::CKind(kind_at(NewKind::Shares)?));
    t.shot_marking("4-11-quorum", Action::CNext(cstep::QUORUM))?;
    t.press(Action::CNext(cstep::QUORUM));
    t.press(Action::CVaults(false));
    t.shot_marking("4-11-keys", Action::KeyGen(Some(0)))?;
    t.press(Action::KeyGen(Some(0)));
    t.shot("4-11-identifier")?;
    c32_type(t, "name");
    c32(t, C32::Next);
    let typed = WAYS
        .iter()
        .position(|w| *w == C32Way::Typed)
        .ok_or("no typed way")?;
    c32(t, C32::Way(typed as u8));
    c32(t, C32::Next);
    for s in &NAME[..2] {
        c32_type(t, &s[9..35]);
    }
    // The worksheets, then each string typed back.
    c32(t, C32::Next);
    c32(t, C32::Next);
    for s in &NAME {
        c32_type(t, &s[3..]);
        c32(t, C32::Next);
    }
    t.shot("4-11-strings-made")?;
    c32(t, C32::Lock);
    c32(t, C32::Next);
    if t.app.create().and_then(|c| c.open) != Some(cstep::CHECK) {
        t.press(Action::CStep(cstep::CHECK));
    }
    t.shot_marking("4-11-check", Action::CNext(cstep::CHECK))?;
    t.press(Action::CNext(cstep::CHECK));
    t.press(Action::CBackup);
    t.shot_marking("4-11-board", Action::BChecklist)?;
    t.press(Action::BChecklist);
    t.press(Action::BNext(bstep::BLANK));
    let item = bstep::of(plan::Item::Codex32(0));
    if open_item(t) != Some(item) {
        t.press(Action::BStep(item));
    }
    t.shot("4-11-strings-item")?;
    shares_kept(t)
}

/// §4.11 on: the plan changed so string A is also in a vault on a stick
/// at Place 1 and string C a file and a Codex32QR picture on a stick at
/// Place 2; each string typed back; the vault made and A saved into it as
/// a share record, the vault item naming the set before it is made, the
/// vault's Bitcoin keys listing it; the sticks
/// written.
fn shares_kept(t: &mut Tour) -> Result<(), String> {
    use faraday_core::codex32_backup::Press;
    use faraday_core::vault::VaultAction as V;
    use faraday_core::{bstep, plan};
    let ix = |c: u8| plan::codex32::index_of(c).ok_or("no index");
    let (a, c) = (ix(b'a')?, ix(b'c')?);
    t.press(Action::BPlan);
    t.press(Action::Board(BP::Customize));
    t.press(Action::Board(BP::AddVault));
    t.press(Action::Board(BP::Chip(Chip::List(Thing::Vault(0)))));
    t.press(Action::Board(BP::Zone(Zone::Card(0))));
    t.press(Action::Board(BP::Chip(Chip::List(Thing::String(0, a)))));
    t.press(Action::Board(BP::Zone(Zone::Vault(0, 0, 0))));
    t.press(Action::Board(BP::Chip(Chip::List(Thing::String(0, c)))));
    t.press(Action::Board(BP::Zone(Zone::NewStick(1))));
    let file = Thing::String(0, c);
    t.press(Action::Board(BP::Chip(Chip::Placed(
        Zone::Files(1, 0),
        file,
    ))));
    t.press(Action::Board(BP::Form(Zone::Files(1, 0), file, 1)));
    t.press(Action::Board(BP::LetGo));
    t.shot_marking("4-11-board-kept", Action::BChecklist)?;
    t.press(Action::BChecklist);
    if open_item(t) == Some(bstep::BLANK) {
        t.press(Action::BNext(bstep::BLANK));
    }
    let item = bstep::of(plan::Item::Codex32(0));
    if open_item(t) != Some(item) {
        t.press(Action::BStep(item));
    }
    for s in &NAME {
        let j = plan::codex32::index_of(s.as_bytes()[8]).ok_or("no index")?;
        t.press(Action::BC32(Press::Open(j as u8)));
        for ch in s[3..].chars() {
            t.press(Action::BC32(Press::Key(OskPress::Char(ch))));
        }
        t.press(Action::BC32(Press::Next));
    }
    let vault = bstep::of(plan::Item::Vault(0));
    if open_item(t) != Some(vault) {
        t.press(Action::BStep(vault));
    }
    // The vault item names the set whose string it keeps.
    t.shot_marking(
        "4-11-vault-item",
        Action::Vault(V::CreateFrom(Screen::Backup)),
    )?;
    t.press(Action::Vault(V::CreateFrom(Screen::Backup)));
    vault_made(t);
    t.press(Action::BVaultSave(0));
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::Category(0)));
    t.shot("4-11-vault-contents")?;
    t.press(Action::Nav(Screen::Backup));
    let envelopes = bstep::of(plan::Item::Envelopes);
    t.press(Action::BStep(envelopes));
    t.press(Action::BNext(envelopes));
    let sticks = bstep::of(plan::Item::Sticks);
    if open_item(t) != Some(sticks) {
        t.press(Action::BStep(sticks));
    }
    t.shot("codex32-shares-sticks")?;
    write_sticks(
        t,
        "codex32-shares-sticks",
        &[("codex32-shares-sticks-insert-1", "4-11-files")],
    )
}
