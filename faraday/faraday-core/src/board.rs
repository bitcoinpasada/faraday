//! The backup board (`docs/BACKUP-FLOW.md` §19, F37): Back up's plan as a
//! card for each place, each divided into where a thing can be there (a
//! stick, the vault on a stick, paper), and beside the cards a list of
//! everything the backup can keep. A thing is dragged, or tapped and
//! placed, from the list onto a zone or from one zone to another; each
//! drop is an edit of the plan's map (`plan::Edit`), so the checks and
//! every card's line of fact follow at once.
//!
//! What is here: what a chip and a zone are, which zone takes which chip
//! and in what form, the presses and what each does to the map, and the
//! board as one frame draws it ([`BoardView`], worked out once per frame
//! through the drawing memo, `docs/SPEED.md` §3.3). A decision (whether a
//! drop is taken) computes from the state as it is, never from the memo.
//! The drawing is `board_screen.rs`. The same board is Back up's plan and
//! a wallet flow's own plan card (§19.12 item 1): whose it is follows the
//! screen ([`BoardOf`]).

use std::rc::Rc;

use opensigner_core::strings::fill;

use crate::Faraday;
use crate::plan::{self, Answers, At, Edit, Shape, Spot, What, form};
use crate::strings::EN;

/// A thing the board can keep, as the list offers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Thing {
    /// Seed `i` of the wallet.
    Seed(usize),
    /// Seed `i`'s passphrase.
    Pass(usize),
    /// The whole wallet description.
    Whole,
    /// Share `j` of the description (a partial description).
    Share(usize),
    /// Vault `v` of the plan: its stick, at a place.
    Vault(usize),
    /// The watch-only software: a place of its own.
    Software,
}

/// Where on the board a chip can be dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    /// Place `p`'s paper.
    Paper(usize),
    /// The files part of stick `s` at place `p`; a vault's chip there is
    /// the vault on that stick.
    Files(usize, usize),
    /// Vault `v`, on stick `s` at place `p`: one vault wherever its sticks
    /// are, so what it holds is the same at each.
    Vault(usize, usize, usize),
    /// Place `p`'s **+ stick** row: a new stick for what is dropped.
    NewStick(usize),
    /// Place `p`'s card as a whole: where a vault's stick goes.
    Card(usize),
    /// The software place's **Scanned in**.
    Scanned,
    /// The software place's stick: the stick of public files that goes
    /// to the online computer (`At::Files`).
    SoftStick,
    /// The list: a placed chip dropped here is taken off.
    List,
}

/// A chip: in the list, or placed in a zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chip {
    /// In the list: dropping it places a copy.
    List(Thing),
    /// Placed in a zone, with its forms there.
    Placed(Zone, Thing),
}

impl Chip {
    /// What the chip stands for.
    pub fn thing(self) -> Thing {
        match self {
            Chip::List(t) | Chip::Placed(_, t) => t,
        }
    }
}

/// A press on the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardPress {
    /// A chip pressed: selected (and a placed chip's forms open), or let
    /// go when it was.
    Chip(Chip),
    /// A zone pressed, or a chip dropped on it: the chip held placed
    /// there.
    Zone(Zone),
    /// The × on a placed chip: taken off.
    Off(Zone, Thing),
    /// A form of a placed chip ticked or unticked, by its place among the
    /// zone's forms.
    Form(Zone, Thing, u8),
    /// The forms popup's own surface: nothing.
    Popup,
    /// The chip held is let go.
    LetGo,
    /// The **+** card: a place more.
    AddPlace,
    /// The × on place `p`'s card.
    RemovePlace(usize),
    /// **+ vault**.
    AddVault,
    /// The × on vault `v`'s list chip: taken off when it holds nothing,
    /// else asked.
    RemoveVault(usize),
    /// **Take off** under a vault that holds something.
    RemoveVaultSure(usize),
    /// The × on the software place's card.
    RemoveSoftware,
    /// The × on a stick box's head: stick `s` at place `p` taken off with
    /// all it carries, its vault's stick and its files.
    RemoveStick(usize, usize),
    /// **Add**, at the top of the narrow column: the list as a sheet
    /// (`docs/BACKUP-FLOW.md` §19.12 item 3).
    OpenList,
}

/// Whose plan the board edits: Back up's (after the lock, its wallet's
/// flow's, §17.8), or a wallet flow's own plan card (§17.7, §19.12
/// item 1), by index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardOf {
    /// Back up's plan.
    Backup,
    /// Wallet flow `i`'s plan.
    Flow(usize),
}

/// What the board holds between presses: the chip held and how.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BoardState {
    /// The chip selected by a tap, or being dragged.
    pub held: Option<Chip>,
    /// The held placed chip's forms are open.
    pub forms: bool,
    /// A press that began on a chip, with where, in pixels: a drag once
    /// it moves past the slop.
    pub down: Option<(Chip, i32, i32)>,
    /// Where the dragged chip is, in pixels, while it is dragged.
    pub drag: Option<(i32, i32)>,
    /// The vault whose × asked first, it holding something.
    pub removing: Option<usize>,
}

/// What kind of zone a zone is: what it keeps, and in what forms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// On paper, by hand.
    Paper,
    /// Files on a stick.
    Files,
    /// Records in a vault.
    Vault,
    /// Scanned into the software.
    Scanned,
    /// The stick of public files.
    Public,
}

fn kind(z: Zone) -> Option<Kind> {
    match z {
        Zone::Paper(_) => Some(Kind::Paper),
        Zone::Files(..) | Zone::NewStick(_) => Some(Kind::Files),
        Zone::Vault(..) => Some(Kind::Vault),
        Zone::Scanned => Some(Kind::Scanned),
        Zone::SoftStick => Some(Kind::Public),
        Zone::Card(_) | Zone::List => None,
    }
}

/// The forms `t` takes in a zone of kind `k`, the first the one a drop
/// places (`docs/BACKUP-FLOW.md` §19.3).
fn forms_in(k: Kind, t: Thing) -> Vec<What> {
    match (k, t) {
        (Kind::Paper, Thing::Seed(i)) => vec![What::Words(i), What::SeedQr(i)],
        (Kind::Paper, Thing::Pass(i)) => vec![What::Passphrase(i)],
        (Kind::Paper, Thing::Whole) => vec![What::Sheet],
        (Kind::Paper, Thing::Share(j)) => vec![What::Share(j)],
        (Kind::Files, Thing::Seed(i)) => vec![What::SeedFile(i), What::SeedQrFile(i)],
        (Kind::Files, Thing::Pass(i)) => vec![What::PassphraseFile(i)],
        (Kind::Files, Thing::Whole) => vec![
            What::WalletFile(form::TEXT),
            What::WalletFile(form::QR),
            What::SoftwareFile,
        ],
        (Kind::Files, Thing::Share(j)) => {
            vec![What::ShareFile(j, form::TEXT), What::ShareFile(j, form::QR)]
        }
        (Kind::Files, Thing::Vault(v)) => vec![What::VaultStick(v)],
        (Kind::Vault, Thing::Seed(i)) => vec![What::Seed(i)],
        (Kind::Vault, Thing::Pass(i)) => vec![What::PassphraseSealed(i)],
        (Kind::Vault, Thing::Whole) => vec![What::Wallet],
        (Kind::Vault, Thing::Share(j)) => vec![What::Share(j)],
        (Kind::Scanned | Kind::Public, Thing::Whole) => vec![What::Wallet],
        (Kind::Public, Thing::Seed(i)) => vec![What::SeedFile(i)],
        _ => Vec::new(),
    }
}

/// The forms a placed chip has in its zone.
pub fn forms_of(z: Zone, t: Thing) -> Vec<What> {
    kind(z).map_or_else(Vec::new, |k| forms_in(k, t))
}

/// What a held `w` is on the board: the chip it is drawn as. A seed kept
/// in a vault with its passphrase is two chips there, the seed's and the
/// passphrase's.
fn things_of(w: What) -> Vec<Thing> {
    match w {
        What::Words(i)
        | What::SeedQr(i)
        | What::SeedFile(i)
        | What::SeedQrFile(i)
        | What::Seed(i) => vec![Thing::Seed(i)],
        What::Passphrase(i) | What::PassphraseFile(i) | What::PassphraseSealed(i) => {
            vec![Thing::Pass(i)]
        }
        What::SeedPassphrase(i) => vec![Thing::Seed(i), Thing::Pass(i)],
        What::Sheet | What::Wallet | What::WalletFile(_) | What::SoftwareFile => {
            vec![Thing::Whole]
        }
        What::Share(j) | What::ShareFile(j, _) => vec![Thing::Share(j)],
        What::VaultStick(v) => vec![Thing::Vault(v)],
    }
}

/// The forms of `t` that `held` (what a spot holds) keeps, as the zone's
/// forms: a seed kept in a vault with its passphrase is the seed there,
/// and the passphrase.
fn held_forms(held: &[What], k: Kind, t: Thing) -> Vec<What> {
    let mut out: Vec<What> = forms_in(k, t)
        .into_iter()
        .filter(|w| held.contains(w))
        .collect();
    if k == Kind::Vault {
        match t {
            Thing::Seed(i) | Thing::Pass(i) if held.contains(&What::SeedPassphrase(i)) => {
                out = vec![forms_in(k, t)[0]];
            }
            _ => {}
        }
    }
    out
}

/// The spot on the map `m` that zone `z` keeps things in. A new stick is
/// the one after the place's last.
fn at_of(m: &[Spot], z: Zone) -> Option<At> {
    Some(match z {
        Zone::Paper(p) | Zone::Card(p) => At::Place(p),
        Zone::Files(p, s) => At::Stick(p, s),
        Zone::Vault(_, _, v) => At::Vault(v),
        Zone::NewStick(p) => At::Stick(p, plan::sticks_at(m, p)),
        Zone::Scanned => At::Software,
        Zone::SoftStick => At::Files,
        Zone::List => return None,
    })
}

/// The place a zone is at.
fn place_of(z: Zone) -> Option<usize> {
    match z {
        Zone::Paper(p)
        | Zone::Files(p, _)
        | Zone::Vault(p, _, _)
        | Zone::NewStick(p)
        | Zone::Card(p) => Some(p),
        _ => None,
    }
}

/// Where `t` dropped on `z` goes: a vault's chip anywhere on a place's
/// card goes to the card (its first stick with no vault, or a new one),
/// except on **+ stick**, which makes a stick for it.
pub fn target(z: Zone, t: Thing) -> Zone {
    match (t, z) {
        (Thing::Vault(_), Zone::NewStick(_)) => z,
        (Thing::Vault(_), z) => place_of(z).map_or(z, Zone::Card),
        _ => z,
    }
}

/// Whether zone `z` takes `t` (`docs/BACKUP-FLOW.md` §19.3): a seed of a
/// key here, its passphrase, the description and its shares where the
/// zone has a form for them; a vault's stick on a place. After the lock
/// (`locked`) paper and files take nothing, and a vault and its sticks
/// change as §17.8 allows (§19.12 item 8). Costs a lookup.
pub fn takes(shape: &Shape, z: Zone, t: Thing, locked: bool) -> bool {
    let here = |i: usize| shape.seeds.get(i).is_some_and(|s| s.here);
    let thing_ok = match t {
        Thing::Seed(i) => here(i),
        Thing::Pass(i) => shape.seeds.get(i).is_some_and(|s| s.here && s.passphrase),
        Thing::Share(j) => shape.splits && j < shape.keys,
        Thing::Whole | Thing::Vault(_) | Thing::Software => true,
    };
    if !thing_ok {
        return false;
    }
    match (z, t) {
        (Zone::List, _) => false,
        (_, Thing::Software) => !locked,
        (Zone::Card(_) | Zone::NewStick(_), Thing::Vault(_)) => true,
        (_, Thing::Vault(_)) | (Zone::Card(_), _) => false,
        (Zone::Vault(..), _) => true,
        _ if locked => false,
        (Zone::Scanned, _) => false,
        (Zone::SoftStick, t) => t == Thing::Whole,
        (z, t) => !forms_of(z, t).is_empty(),
    }
}

/// The map `m` with `t` from the list kept at `z`: its first form there,
/// or a vault's stick on the place. A thing kept there already stays as
/// it is.
fn added(m: Vec<Spot>, z: Zone, t: Thing) -> Vec<Spot> {
    match (t, z) {
        (Thing::Vault(v), Zone::NewStick(p)) => {
            let m = plan::with_vault(m, v);
            let at = At::Stick(p, plan::sticks_at(&m, p));
            plan::with_added(&m, at, What::VaultStick(v))
        }
        (Thing::Vault(v), z) => match place_of(z) {
            Some(p) => plan::with_vault_stick(&plan::with_vault(m, v), p, v),
            None => m,
        },
        (t, z) => {
            let (Some(at), Some(w)) = (at_of(&m, z), forms_of(z, t).first().copied()) else {
                return m;
            };
            plan::edited(&m, Edit::Add(at, w), &[])
        }
    }
}

/// The map `m` with the chip of `t` at `z` taken off, every form of it
/// there. In a vault, a seed kept with its passphrase leaves the
/// passphrase kept alone, and the passphrase taken leaves the seed.
fn taken(m: Vec<Spot>, z: Zone, t: Thing) -> Vec<Spot> {
    let Some(at) = at_of(&m, z) else {
        return m;
    };
    let held = plan::holds_at(&m, at);
    if let (Zone::Vault(..), Thing::Seed(i) | Thing::Pass(i)) = (z, t)
        && held.contains(&What::SeedPassphrase(i))
    {
        let rest = match t {
            Thing::Seed(_) => What::PassphraseSealed(i),
            _ => What::Seed(i),
        };
        let m = plan::without(&m, at, What::SeedPassphrase(i));
        return plan::with_added(&m, at, rest);
    }
    let Some(k) = kind(z) else {
        return m;
    };
    held_forms(&held, k, t)
        .into_iter()
        .fold(m, |m, w| plan::edited(&m, Edit::Drop(at, w), &[]))
}

/// The map `m` with the chip of `t` moved from `from` to `to`: each form
/// it had becomes the form in the same place among `to`'s (words to a
/// words file, a SeedQR to a SeedQR picture), or `to`'s first. A vault's
/// stick goes to the place `to` is at.
fn moved(m: Vec<Spot>, from: Zone, t: Thing, to: Zone) -> Vec<Spot> {
    if from == to {
        return m;
    }
    if to == Zone::List {
        return taken(m, from, t);
    }
    if let Thing::Vault(v) = t {
        let Some(at) = at_of(&m, from) else {
            return m;
        };
        let m = plan::without(&m, at, What::VaultStick(v));
        return added(m, to, t);
    }
    let (Some(fk), Some(tk)) = (kind(from), kind(to)) else {
        return m;
    };
    let had: Vec<usize> = {
        let held = at_of(&m, from).map_or_else(Vec::new, |at| plan::holds_at(&m, at));
        let mine = held_forms(&held, fk, t);
        forms_in(fk, t)
            .iter()
            .enumerate()
            .filter(|(_, w)| mine.contains(w))
            .map(|(k, _)| k)
            .collect()
    };
    let to_forms = forms_in(tk, t);
    let m = taken(m, from, t);
    let Some(at) = at_of(&m, to) else {
        return m;
    };
    let mut put: Vec<What> = Vec::new();
    for k in had {
        if let Some(&w) = to_forms.get(k).or(to_forms.first())
            && !put.contains(&w)
        {
            put.push(w);
        }
    }
    if put.is_empty()
        && let Some(&w) = to_forms.first()
    {
        put.push(w);
    }
    put.into_iter().fold(m, |m, w| plan::with_added(&m, at, w))
}

/// The map `m` with form `k` of the chip of `t` at `z` turned on or off;
/// the last form on stays on (`docs/BACKUP-FLOW.md` §19.3).
fn form_toggled(m: Vec<Spot>, z: Zone, t: Thing, k: usize) -> Vec<Spot> {
    let (Some(at), Some(kd)) = (at_of(&m, z), kind(z)) else {
        return m;
    };
    let Some(&w) = forms_in(kd, t).get(k) else {
        return m;
    };
    let held = held_forms(&plan::holds_at(&m, at), kd, t);
    if held.contains(&w) {
        if held.len() > 1 {
            plan::without(&m, at, w)
        } else {
            m
        }
    } else {
        plan::with_added(&m, at, w)
    }
}

/// Writes the map out: `a`'s map with `f` made to it, as the plan. The
/// description counts as split while any share of it is kept, so the
/// checklist makes the shares (§19.1: Whole and shares are both offered,
/// with nothing to choose first).
fn map_edit(a: &mut Answers, shape: &Shape, f: impl FnOnce(Vec<Spot>) -> Vec<Spot>) {
    a.ensure_map(shape);
    let m = f(plan::map(shape, a));
    a.split = shape.splits
        && m.iter().any(|s| {
            s.holds
                .iter()
                .any(|(w, _)| matches!(w, What::Share(_) | What::ShareFile(..)))
        });
    a.set_map(m);
}

/// One thing a place, the software place or a vault keeps, as its chip
/// is drawn.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ChipView {
    /// The chip, as a press names it.
    pub chip: Chip,
    /// Its name: a fingerprint for a seed or a passphrase.
    pub name: String,
    /// Its forms there, joined ("Words · SeedQR"); empty where it has
    /// one form only.
    pub marks: String,
    /// Each of its forms there, with whether it is on: the popup's rows.
    pub forms: Vec<(String, bool)>,
    /// A check names it (`docs/BACKUP-FLOW.md` §19.5).
    pub warn: bool,
    /// Drawn at every place that keeps its vault's stick.
    pub link: bool,
    /// What an existing vault held before this backup: dimmed, read-only
    /// (§19.12 item 7).
    pub fixed: bool,
    /// Its × is drawn: it can be taken off here.
    pub off: bool,
}

/// A zone, as it is drawn: what it is and its chips.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ZoneView {
    /// The zone.
    pub zone: Zone,
    /// Its chips.
    pub chips: Vec<ChipView>,
}

/// One stick at a place.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StickView {
    /// "Stick 1".
    pub label: String,
    /// The vault on it: its chip, and the vault's own zone.
    pub vault: Option<(ChipView, ZoneView)>,
    /// The vault's header band, "Vault 1 · sealed"; empty with no vault.
    pub sealed: String,
    /// Its files.
    pub files: ZoneView,
}

/// One place's card.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CardView {
    /// The place, by number from 0.
    pub p: usize,
    /// Its name.
    pub name: String,
    /// Its sticks, in order.
    pub sticks: Vec<StickView>,
    /// Its paper.
    pub paper: ZoneView,
    /// What finding it alone gives, and how that reads.
    pub alone: (String, plan::AloneKind),
}

/// The software place's card.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SoftView {
    /// The software chosen, or "Watch-only software".
    pub title: String,
    /// What is scanned in.
    pub scanned: ZoneView,
    /// Its stick of public files, when it has one.
    pub stick: Option<ZoneView>,
}

/// One entry of the list.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Entry {
    /// The chip.
    pub chip: Chip,
    /// What it reads.
    pub name: String,
    /// The line at its right: how many places keep it, or "Not here".
    pub count: String,
    /// It cannot be moved: a cosigner's seed.
    pub dim: bool,
    /// Under it: the keys a share carries, by fingerprint.
    pub keys: Vec<String>,
    /// For a vault: what it holds, said before it is taken off.
    pub holds: Option<String>,
}

/// The board as one frame draws it: worked out once per frame
/// ([`Faraday::board_view`]).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BoardView {
    /// The wallet's shape: which zones take the chip held.
    pub shape: Shape,
    /// The plan's three checks, under the board (§19.14).
    pub check: plan::Check,
    /// A wallet flow's own plan, by index; `None` for Back up's.
    pub flow: Option<usize>,
    /// The places, in order.
    pub places: Vec<CardView>,
    /// The software place, when the board has one.
    pub software: Option<SoftView>,
    /// The seeds.
    pub seeds: Vec<Entry>,
    /// The passphrases.
    pub passes: Vec<Entry>,
    /// The whole description, then each share.
    pub description: Vec<Entry>,
    /// The keys left off each share, and the most there may be; `None`
    /// for a description that does not split.
    pub omit: Option<(usize, usize)>,
    /// The vaults.
    pub vaults: Vec<Entry>,
    /// The software's entry, while the board has no software place.
    pub software_entry: Option<Entry>,
    /// Each check that stands, a line under the board.
    pub warnings: Vec<String>,
    /// After the lock: paper and files take nothing.
    pub locked: bool,
    /// Place names are typed by a press on a card's name.
    pub names_typed: bool,
    /// The place whose name is being typed.
    pub naming: Option<usize>,
}

/// What place `p` keeps, on paper or on a stick there, read through the
/// vaults whose sticks it keeps: the things, each once.
fn things_at(m: &[Spot], p: usize) -> Vec<Thing> {
    let mut out: Vec<Thing> = Vec::new();
    let push = |t: Thing, out: &mut Vec<Thing>| {
        if !out.contains(&t) {
            out.push(t);
        }
    };
    for s in m {
        if !matches!(s.at, At::Place(q) | At::Stick(q, _) if q == p) {
            continue;
        }
        for (w, _) in &s.holds {
            for t in things_of(*w) {
                push(t, &mut out);
            }
            if let What::VaultStick(v) = w {
                for x in plan::holds_at(m, At::Vault(*v)) {
                    for t in things_of(x) {
                        push(t, &mut out);
                    }
                }
            }
        }
    }
    out
}

/// "2 places", "1 place", or nothing.
fn count_text(n: usize) -> String {
    match n {
        0 => String::new(),
        1 => EN.board_one_place.to_string(),
        n => fill(EN.board_places, &[&n.to_string()]),
    }
}

/// A form's name on a chip, or `None` where the zone has no other form
/// to tell it from.
fn form_label(w: What, software: &str) -> Option<String> {
    Some(match w {
        What::Words(_) => EN.board_words.to_string(),
        What::SeedQr(_) => EN.board_seedqr.to_string(),
        What::SeedFile(_) => EN.board_words_file.to_string(),
        What::SeedQrFile(_) => EN.board_seedqr_file.to_string(),
        What::PassphraseFile(_) => EN.board_passphrase_file.to_string(),
        What::WalletFile(f) | What::ShareFile(_, f) if f == form::TEXT => {
            EN.board_text_file.to_string()
        }
        What::WalletFile(_) | What::ShareFile(..) => EN.board_qr_picture.to_string(),
        What::SoftwareFile => fill(EN.board_software_file, &[software]),
        _ => return None,
    })
}

impl Faraday {
    /// Whose plan the board on screen edits: a wallet flow's on the flow's
    /// screen, Back up's otherwise, while Back up is open.
    pub fn board_of(&self) -> Option<BoardOf> {
        if self.screen == crate::Screen::BackupFlow {
            let i = self.flow_on.filter(|&i| i < self.backup_flows.len())?;
            self.backup_flows[i].wallet.as_ref()?;
            return Some(BoardOf::Flow(i));
        }
        self.backup.as_ref().map(|_| BoardOf::Backup)
    }

    /// Whether the board is on screen: Back up's plan with its board card
    /// open, or a wallet flow's plan card with its board open. Costs a
    /// lookup.
    pub fn board_shown(&self) -> bool {
        match self.board_of() {
            Some(BoardOf::Backup) => self.backup.as_ref().is_some_and(|b| {
                self.screen == crate::Screen::Backup
                    && b.stage == crate::BStage::Plan
                    && b.q == Some(crate::qstep::BOARD)
            }),
            Some(BoardOf::Flow(i)) => self.backup_flows.get(i).is_some_and(|f| {
                f.live.open == Some(crate::backup_flow::Step::Where.n())
                    && f.live.plan_q == Some(crate::qstep::BOARD)
            }),
            None => false,
        }
    }

    /// What the board on screen holds between presses.
    pub(crate) fn board_state(&self) -> Option<&BoardState> {
        match self.board_of()? {
            BoardOf::Backup => self.backup.as_ref().map(|b| &b.board),
            BoardOf::Flow(i) => self.backup_flows.get(i).map(|f| &f.live.board),
        }
    }

    /// What the board on screen holds between presses, to change.
    fn board_state_mut(&mut self) -> Option<&mut BoardState> {
        match self.board_of()? {
            BoardOf::Backup => self.backup.as_mut().map(|b| &mut b.board),
            BoardOf::Flow(i) => self.backup_flows.get_mut(i).map(|f| &mut f.live.board),
        }
    }

    /// The chip held on the board on screen let go, its forms and a
    /// vault's question closed: a press anywhere else (§19.2).
    pub(crate) fn board_let_go(&mut self) {
        if let Some(st) = self.board_state_mut() {
            st.held = None;
            st.forms = false;
            st.removing = None;
        }
    }

    /// Works out the wallet's shape (`plan_shape`, its fingerprints kept):
    /// the plan the board on screen edits, as it is now. Back up's after
    /// its wallet's flow is past the lock is that flow's (§17.8).
    pub fn board_plan(&self) -> Option<(Shape, Answers)> {
        let flow = match self.board_of()? {
            BoardOf::Flow(i) => Some(i),
            BoardOf::Backup => self.backup_locked_flow(),
        };
        if let Some(i) = flow {
            let wf = self.backup_flows.get(i)?.wallet.as_ref()?;
            return Some((wf.shape.clone(), wf.answers.clone()));
        }
        let b = self.backup.as_ref()?;
        Some((self.plan_shape(b.wallet), b.answers.clone()))
    }

    /// The flow whose plan the board on screen edits past its lock, where
    /// it is: paper and files stay as made (§17.8, §19.12 item 8).
    fn board_locked(&self) -> Option<usize> {
        match self.board_of()? {
            BoardOf::Backup => self.backup_locked_flow(),
            BoardOf::Flow(i) => {
                let f = self.backup_flows.get(i)?;
                (f.made && f.step != crate::backup_flow::Step::Done).then_some(i)
            }
        }
    }

    /// The place names are Back up's, typed with a vault open: on Back
    /// up's plan, and on a flow's before its lock while Back up is open on
    /// its wallet (§17.7). A flow keeps no name.
    fn board_backup_names(&self) -> bool {
        match self.board_of() {
            Some(BoardOf::Backup) => true,
            Some(BoardOf::Flow(i)) => {
                let Some(f) = self.backup_flows.get(i) else {
                    return false;
                };
                let cs = f.wallet.as_ref().and_then(|w| w.checksum.as_deref());
                !f.made
                    && self.backup.as_ref().is_some_and(|b| {
                        self.session.wallets.get(b.wallet).map(|w| w.checksum()) == cs
                    })
            }
            None => false,
        }
    }

    /// Place `p`'s name on the board on screen.
    fn board_place_name(&self, p: usize) -> String {
        if self.board_backup_names() {
            self.place_name(p)
        } else {
            crate::backup_flow::BackupFlow::place_name(p)
        }
    }

    /// Vault `v`'s title on the board on screen, with the name of the
    /// open vault set up for it.
    fn board_vault_title(&self, v: usize) -> String {
        match self.board_of() {
            Some(BoardOf::Flow(i)) => {
                let n = (v + 1).to_string();
                match self
                    .flow_open_index(i, v)
                    .map(|o| &self.vaults.open[o].name)
                {
                    Some(name) if !name.is_empty() => fill(EN.plan_vault_2, &[&n, name]),
                    _ => fill(EN.plan_vault, &[&n]),
                }
            }
            _ => self.backup_vault_title(v),
        }
    }

    /// Works out the plan's map and its checks, once a frame: the board as
    /// this frame draws it, kept in the drawing memo while a frame is
    /// drawn (`docs/SPEED.md` §3.3). Drawing reads it; nothing decides
    /// from it.
    pub(crate) fn board_view(&self) -> Option<Rc<BoardView>> {
        if self.drawing.is_on() {
            return self
                .drawing
                .board
                .get_or_init(|| self.board_view_uncached().map(Rc::new))
                .clone();
        }
        self.board_view_uncached().map(Rc::new)
    }

    /// Works out the plan's map, its checks and each place's line of
    /// fact: the board, for [`Faraday::board_view`].
    fn board_view_uncached(&self) -> Option<BoardView> {
        let of = self.board_of()?;
        let state = self.board_state()?;
        let locked = self.board_locked().is_some();
        let (shape, a) = self.board_plan()?;
        let m = plan::map(&shape, &a);
        let places = plan::places_of(&m).max(1);
        let software_name = software_name(&a);
        let seed_name = |i: usize| {
            shape
                .seeds
                .get(i)
                .map_or_else(String::new, |s| s.name.clone())
        };
        let name_of = |t: Thing| -> String {
            match t {
                Thing::Seed(i) | Thing::Pass(i) => seed_name(i),
                Thing::Whole => EN.board_whole.to_string(),
                Thing::Share(j) => fill(EN.board_share, &[&(j + 1).to_string()]),
                Thing::Vault(v) => self.board_vault_title(v),
                Thing::Software => EN.board_watch_only.to_string(),
            }
        };
        // The checks (`docs/BACKUP-FLOW.md` §19.5): each a line, and the
        // chips each names marked.
        let place_name = |p: usize| self.board_place_name(p);
        let mut warnings: Vec<String> = Vec::new();
        let mut marked: Vec<(usize, Thing)> = Vec::new();
        let warn = |line: String, w: &mut Vec<String>| {
            if !w.contains(&line) {
                w.push(line);
            }
        };
        for i in shape.here() {
            if let Some(line) = plan::kept_nowhere(&shape, &a, &[i]) {
                warn(line, &mut warnings);
            }
        }
        let things: Vec<Vec<Thing>> = (0..places).map(|p| things_at(&m, p)).collect();
        let lost_no = plan::check(&shape, &a).lost == plan::Lost::No;
        for (p, here) in things.iter().enumerate() {
            for i in shape.here() {
                // A passphrase beside its seed's words, on paper or a stick
                // there (§19.5).
                let words = plan::holds_at(&m, At::Place(p))
                    .into_iter()
                    .chain(
                        m.iter()
                            .filter(|s| matches!(s.at, At::Stick(q, _) if q == p))
                            .flat_map(|s| s.holds.iter().map(|(w, _)| *w)),
                    )
                    .collect::<Vec<What>>();
                let has_words = words.iter().any(|w| {
                    matches!(w, What::Words(j) | What::SeedQr(j) | What::SeedFile(j)
                        | What::SeedQrFile(j) if *j == i)
                });
                let has_pass = words
                    .iter()
                    .any(|w| matches!(w, What::Passphrase(j) | What::PassphraseFile(j) if *j == i));
                if has_words && has_pass {
                    warn(
                        fill(EN.board_pass_with_words, &[&place_name(p), &seed_name(i)]),
                        &mut warnings,
                    );
                    marked.push((p, Thing::Seed(i)));
                    marked.push((p, Thing::Pass(i)));
                }
            }
            // A seed whose every copy is at this one place, where losing
            // the place loses the wallet (`lost_why`'s "kept only at").
            if lost_no {
                for i in shape.here() {
                    let at: Vec<usize> = (0..places)
                        .filter(|&q| things[q].contains(&Thing::Seed(i)))
                        .collect();
                    if at == [p] && here.contains(&Thing::Seed(i)) {
                        marked.push((p, Thing::Seed(i)));
                    }
                }
            }
        }
        if let Some(line) = plan::lost_why(&shape, &a, &place_name, self.medium.noun()) {
            warn(line, &mut warnings);
        }
        if let Some(p) = plan::all_at_one_place(&shape, &a) {
            warn(
                fill(EN.plan_all_at_one_place, &[&place_name(p)]),
                &mut warnings,
            );
        }
        for p in 0..places {
            if places > 1 && plan::alone(&shape, &a, At::Place(p)).spend == plan::Found::Yes {
                warn(
                    fill(EN.board_alone_spends, &[&place_name(p)]),
                    &mut warnings,
                );
            }
        }
        if let Some(v) = a.stickless(&shape) {
            warn(
                fill(
                    EN.plan_vault_s_needs,
                    &[&(v + 1).to_string(), self.medium.noun()],
                ),
                &mut warnings,
            );
        }
        if let Some(v) = a.empty_vault() {
            warn(
                fill(EN.backup_flow_vault_holds_nothing, &[&(v + 1).to_string()]),
                &mut warnings,
            );
        }
        let vault_places = |v: usize| -> usize {
            (0..places)
                .filter(|&p| {
                    m.iter().any(|s| {
                        matches!(s.at, At::Stick(q, _) if q == p)
                            && s.holds.iter().any(|(w, _)| *w == What::VaultStick(v))
                    })
                })
                .count()
        };
        // A placed chip of `t` in zone `z`, with the forms of it `held`
        // keeps there.
        let chip = |z: Zone, t: Thing, held: &[What], p: Option<usize>| -> ChipView {
            let all = forms_of(z, t);
            let k = kind(z);
            let on = k.map_or_else(Vec::new, |k| held_forms(held, k, t));
            let labels: Vec<Option<String>> =
                all.iter().map(|w| form_label(*w, &software_name)).collect();
            let marks: Vec<String> = all
                .iter()
                .zip(&labels)
                .filter(|(w, _)| on.contains(w))
                .filter_map(|(_, l)| l.clone())
                .collect();
            let forms = if all.len() > 1 {
                all.iter()
                    .zip(labels)
                    .map(|(w, l)| (l.unwrap_or_default(), on.contains(w)))
                    .collect()
            } else {
                Vec::new()
            };
            ChipView {
                chip: Chip::Placed(z, t),
                name: name_of(t),
                marks: marks.join(" · "),
                forms,
                warn: p.is_some_and(|p| marked.contains(&(p, t))),
                link: matches!(t, Thing::Vault(v) if vault_places(v) > 1),
                fixed: false,
                off: !(locked
                    && matches!(k, Some(Kind::Paper | Kind::Files | Kind::Public))
                    && !matches!(t, Thing::Vault(_))),
            }
        };
        let zone = |z: Zone, at: At, p: Option<usize>, skip_vaults: bool| -> ZoneView {
            let held = plan::holds_at(&m, at);
            let mut ts: Vec<Thing> = Vec::new();
            for w in &held {
                for t in things_of(*w) {
                    if (skip_vaults && matches!(t, Thing::Vault(_))) || ts.contains(&t) {
                        continue;
                    }
                    ts.push(t);
                }
            }
            ZoneView {
                zone: z,
                chips: ts.into_iter().map(|t| chip(z, t, &held, p)).collect(),
            }
        };
        let cards: Vec<CardView> = (0..places)
            .map(|p| {
                let sticks = (0..plan::sticks_at(&m, p))
                    .map(|s| {
                        let vault = plan::stick_vault(&m, p, s).map(|v| {
                            let held = plan::holds_at(&m, At::Stick(p, s));
                            let head = chip(Zone::Files(p, s), Thing::Vault(v), &held, Some(p));
                            let mut inside =
                                zone(Zone::Vault(p, s, v), At::Vault(v), Some(p), false);
                            inside.chips.extend(self.board_before(&shape, &a, v));
                            (head, inside)
                        });
                        let sealed = vault.as_ref().map_or_else(String::new, |(head, _)| {
                            fill(EN.board_sealed, &[&head.name])
                        });
                        StickView {
                            label: fill(EN.board_stick, &[&(s + 1).to_string()]),
                            vault,
                            sealed,
                            files: zone(Zone::Files(p, s), At::Stick(p, s), Some(p), true),
                        }
                    })
                    .collect();
                let alone = plan::alone(&shape, &a, At::Place(p));
                CardView {
                    p,
                    name: place_name(p),
                    sticks,
                    paper: zone(Zone::Paper(p), At::Place(p), Some(p), true),
                    alone: (alone.text().to_string(), alone.kind()),
                }
            })
            .collect();
        let has_software = m.iter().any(|s| s.at == At::Software);
        let has_files = m.iter().any(|s| s.at == At::Files);
        let software = (has_software || has_files).then(|| SoftView {
            title: software_name.clone(),
            scanned: zone(Zone::Scanned, At::Software, None, false),
            stick: has_files.then(|| zone(Zone::SoftStick, At::Files, None, false)),
        });
        // How many places keep each thing: the numbered places, and the
        // software place for the description.
        let count = |t: Thing| -> usize {
            let soft = (has_software || has_files)
                && [At::Software, At::Files].iter().any(|&at| {
                    plan::holds_at(&m, at)
                        .into_iter()
                        .any(|w| things_of(w).contains(&t))
                });
            things.iter().filter(|h| h.contains(&t)).count() + usize::from(soft)
        };
        let entry = |t: Thing| Entry {
            chip: Chip::List(t),
            name: name_of(t),
            count: count_text(count(t)),
            dim: false,
            keys: Vec::new(),
            holds: None,
        };
        let seeds = (0..shape.seeds.len())
            .map(|i| {
                if shape.seeds[i].here {
                    entry(Thing::Seed(i))
                } else {
                    Entry {
                        count: EN.board_not_here.to_string(),
                        dim: true,
                        ..entry(Thing::Seed(i))
                    }
                }
            })
            .collect();
        let passes = shape
            .here()
            .into_iter()
            .filter(|&i| shape.seeds[i].passphrase)
            .map(|i| entry(Thing::Pass(i)))
            .collect();
        let mut description = vec![entry(Thing::Whole)];
        let omit = shape.splits.then(|| {
            let most = shape.m.saturating_sub(1).min(shape.keys.saturating_sub(1));
            (a.omit.min(most), most)
        });
        if let Some((omit, _)) = omit {
            let split = crate::backup::split_plan(shape.keys, shape.m, omit);
            for j in 0..shape.keys {
                let keys: Vec<String> = split
                    .get(j)
                    .map(|row| row.iter().map(|&k| seed_name(k).to_lowercase()).collect())
                    .unwrap_or_default();
                description.push(Entry {
                    keys,
                    ..entry(Thing::Share(j))
                });
            }
        }
        let shown_vaults: Vec<usize> = {
            let named: Vec<usize> = m
                .iter()
                .filter_map(|s| match s.at {
                    At::Vault(v) => Some(v),
                    _ => None,
                })
                .collect();
            let n = named
                .iter()
                .map(|v| v + 1)
                .max()
                .unwrap_or(0)
                .max(shape.vaults);
            (0..n)
                .filter(|v| named.contains(v) || *v < shape.vaults)
                .collect()
        };
        let vaults = shown_vaults
            .into_iter()
            .map(|v| {
                let held: Vec<String> = plan::holds_at(&m, At::Vault(v))
                    .into_iter()
                    .flat_map(things_of)
                    .map(name_of)
                    .fold(Vec::new(), |mut acc, n| {
                        if !acc.contains(&n) {
                            acc.push(n);
                        }
                        acc
                    });
                Entry {
                    count: count_text(vault_places(v)),
                    holds: (state.removing == Some(v) && !held.is_empty()).then(|| {
                        fill(
                            EN.board_vault_holds,
                            &[&name_of(Thing::Vault(v)), &held.join(", ")],
                        )
                    }),
                    ..entry(Thing::Vault(v))
                }
            })
            .collect();
        // Place names are typed while a vault is open, before the lock,
        // where they are Back up's (§17.7).
        let names_typed = !locked && !self.vaults.open.is_empty() && self.board_backup_names();
        let naming = self
            .backup
            .as_ref()
            .and_then(|b| b.naming)
            .filter(|_| names_typed);
        Some(BoardView {
            flow: match of {
                BoardOf::Flow(i) => Some(i),
                BoardOf::Backup => None,
            },
            places: cards,
            software,
            seeds,
            passes,
            description,
            omit,
            vaults,
            software_entry: (!has_software && !locked).then(|| entry(Thing::Software)),
            warnings,
            locked,
            names_typed,
            naming,
            check: plan::check(&shape, &a),
            shape,
        })
    }

    /// Works out the board: the placed chips the plan's checks name, each
    /// by its zone, the chips drawn with the warning mark
    /// (`docs/BACKUP-FLOW.md` §19.5).
    pub fn board_marks(&self) -> Vec<(Zone, Thing)> {
        let Some(v) = self.board_view() else {
            return Vec::new();
        };
        let mut zones: Vec<&ZoneView> = Vec::new();
        for c in &v.places {
            zones.push(&c.paper);
            for s in &c.sticks {
                zones.push(&s.files);
                if let Some((_, z)) = &s.vault {
                    zones.push(z);
                }
            }
        }
        zones
            .into_iter()
            .flat_map(|z| z.chips.iter())
            .filter(|c| c.warn)
            .filter_map(|c| match c.chip {
                Chip::Placed(z, t) => Some((z, t)),
                Chip::List(_) => None,
            })
            .collect()
    }

    /// Reads the open vault's records (a key's fingerprint is kept once
    /// worked out): what existing vault `v` held before this backup, as
    /// dimmed chips that take no press (`docs/BACKUP-FLOW.md` §19.12
    /// item 7), less what the plan keeps there now.
    fn board_before(&self, shape: &Shape, a: &Answers, v: usize) -> Vec<ChipView> {
        // The open vault set up for `v`, what it held before as this
        // backup or flow saw it first, and the wallet's checksum.
        let (o, before, sum) = match self.board_of() {
            Some(BoardOf::Flow(i)) => {
                let Some(wf) = self.backup_flows.get(i).and_then(|f| f.wallet.as_ref()) else {
                    return Vec::new();
                };
                (
                    self.flow_open_index(i, v),
                    &wf.before,
                    wf.checksum.clone().unwrap_or_default(),
                )
            }
            _ => {
                let Some(b) = self.backup.as_ref() else {
                    return Vec::new();
                };
                let sum = self
                    .session
                    .wallets
                    .get(b.wallet)
                    .map(|w| w.checksum().to_string())
                    .unwrap_or_default();
                (self.backup_vault_set_up(v), &b.before, sum)
            }
        };
        let Some(o) = o else {
            return Vec::new();
        };
        let Some(open) = self.vaults.open.get(o) else {
            return Vec::new();
        };
        let salt = open.header().salt;
        let tokens = before
            .iter()
            .find(|(s, _)| *s == salt)
            .map_or_else(|| self.vault_tokens(o), |(_, t)| t.clone());
        let held = plan::holds_at(&plan::map(shape, a), At::Vault(v));
        let seed_of = |fp: &str| {
            shape
                .seeds
                .iter()
                .position(|s| s.name.eq_ignore_ascii_case(fp))
        };
        let mut out = Vec::new();
        for t in tokens {
            let (thing, name, ours) = match t.split_at(1) {
                ("k" | "p", fp) => {
                    let i = seed_of(fp);
                    let ours = i.is_some_and(|i| {
                        held.contains(&What::Seed(i)) || held.contains(&What::SeedPassphrase(i))
                    });
                    (
                        Thing::Seed(i.unwrap_or(usize::MAX)),
                        fp.to_uppercase(),
                        ours,
                    )
                }
                ("a", fp) => {
                    let i = seed_of(fp);
                    let ours = i.is_some_and(|i| {
                        held.contains(&What::PassphraseSealed(i))
                            || held.contains(&What::SeedPassphrase(i))
                    });
                    (
                        Thing::Pass(i.unwrap_or(usize::MAX)),
                        fp.to_uppercase(),
                        ours,
                    )
                }
                ("w", s) => (
                    Thing::Whole,
                    if s == sum {
                        EN.board_whole.to_string()
                    } else {
                        format!("#{s}")
                    },
                    s == sum && held.contains(&What::Wallet),
                ),
                ("s", rest) => {
                    let (s, n) = rest.split_once(':').unwrap_or((rest, "0"));
                    let n: usize = n.parse().unwrap_or(0);
                    let j = n.saturating_sub(1);
                    (
                        Thing::Share(j),
                        fill(EN.board_share, &[&n.to_string()]),
                        s == sum && held.contains(&What::Share(j)),
                    )
                }
                _ => continue,
            };
            if ours {
                continue;
            }
            out.push(ChipView {
                chip: Chip::Placed(Zone::List, thing),
                name,
                marks: String::new(),
                forms: Vec::new(),
                warn: false,
                link: false,
                fixed: true,
                off: false,
            });
        }
        out
    }

    /// The plan's edit `f`, made to Back up's plan; after the lock through
    /// the wallet's flow, with its refusals (§17.8). Computes from the
    /// state as it is.
    fn board_change(&mut self, f: &dyn Fn(&mut Answers, &Shape)) {
        if let Some(BoardOf::Flow(i)) = self.board_of() {
            return self.flow_wallet_edit(i, f);
        }
        if let Some(i) = self.backup_locked_flow() {
            return self.flow_wallet_edit(i, f);
        }
        let Some(w) = self.backup.as_ref().map(|b| b.wallet) else {
            return;
        };
        let shape = self.plan_shape(w);
        if let Some(b) = self.backup.as_mut() {
            f(&mut b.answers, &shape);
            let places = b.answers.places;
            b.naming = b.naming.filter(|&p| p < places);
        }
    }

    /// Whether `c` can be picked up: not a cosigner's seed, not what an
    /// existing vault held before.
    pub(crate) fn board_movable(&self, c: Chip) -> bool {
        let Some((shape, _)) = self.board_plan() else {
            return false;
        };
        if matches!(c, Chip::Placed(Zone::List, _)) {
            return false;
        }
        // After the lock paper and files stay as made (§19.12 item 8).
        if self.board_locked().is_some()
            && let Chip::Placed(z, t) = c
            && matches!(kind(z), Some(Kind::Paper | Kind::Files | Kind::Public))
            && !matches!(t, Thing::Vault(_))
        {
            return false;
        }
        match c.thing() {
            Thing::Seed(i) => shape.seeds.get(i).is_some_and(|s| s.here),
            _ => true,
        }
    }

    /// Whether zone `z` takes the thing `t` now: as [`takes`], before or
    /// after the lock. Computes from the state as it is.
    fn board_takes(&self, z: Zone, t: Thing) -> bool {
        let Some((shape, _)) = self.board_plan() else {
            return false;
        };
        takes(&shape, target(z, t), t, self.board_locked().is_some())
    }

    /// A press on the board (`docs/BACKUP-FLOW.md` §19.2, §19.3).
    pub(crate) fn board_act(&mut self, press: BoardPress) {
        let Some(state) = self.board_state().cloned() else {
            return;
        };
        let locked = self.board_locked().is_some();
        let set = |app: &mut Faraday, held: Option<Chip>, forms: bool| {
            if let Some(st) = app.board_state_mut() {
                st.held = held;
                st.forms = forms;
                st.removing = None;
            }
        };
        // A chip pressed in the list's sheet closes it, the chip selected
        // (§19.12 item 3).
        if matches!(press, BoardPress::Chip(Chip::List(_)))
            && self.sheet == Some(crate::Sheet::BoardList)
        {
            self.sheet = None;
        }
        match press {
            BoardPress::OpenList => {
                set(self, None, false);
                self.sheet = Some(crate::Sheet::BoardList);
                self.sheet_scroll = (0.0, None);
            }
            BoardPress::Chip(c) => {
                if c == Chip::List(Thing::Software) {
                    set(self, None, false);
                    return self.board_put(Chip::List(Thing::Software), Zone::Scanned);
                }
                if locked
                    && let Chip::Placed(z, t) = c
                    && matches!(kind(z), Some(Kind::Paper | Kind::Files | Kind::Public))
                    && !matches!(t, Thing::Vault(_))
                {
                    set(self, None, false);
                    return self.toast(EN.backup_flow_files_after_lock);
                }
                if !self.board_movable(c) || state.held == Some(c) {
                    return set(self, None, false);
                }
                // A placed chip's forms open with it; after the lock paper
                // and files stay as made (§19.12 item 8).
                let forms = match c {
                    Chip::Placed(z, t) => {
                        forms_of(z, t).len() > 1
                            && !(locked && matches!(kind(z), Some(Kind::Paper | Kind::Files)))
                    }
                    Chip::List(_) => false,
                };
                set(self, Some(c), forms);
            }
            BoardPress::Zone(z) => {
                let Some(c) = state.held else {
                    // After the lock a press on paper or a stick's files
                    // says why nothing there changes (§19.12 item 8).
                    if locked && matches!(kind(z), Some(Kind::Paper | Kind::Files | Kind::Public)) {
                        self.toast(EN.backup_flow_files_after_lock);
                    }
                    return;
                };
                set(self, None, false);
                self.board_put(c, z);
            }
            BoardPress::Off(z, t) => {
                set(self, None, false);
                // After the lock paper and files stay as made; a vault's
                // stick changes as §17.8 allows (§19.12 item 8).
                let file = matches!(kind(z), Some(Kind::Paper | Kind::Files | Kind::Public));
                if locked && file && !matches!(t, Thing::Vault(_)) {
                    return self.toast(EN.backup_flow_files_after_lock);
                }
                self.board_change(&|a, shape| map_edit(a, shape, |m| moved(m, z, t, Zone::List)));
            }
            BoardPress::Form(z, t, k) => {
                if locked {
                    return self.toast(EN.backup_flow_files_after_lock);
                }
                self.board_change(&|a, shape| {
                    map_edit(a, shape, |m| form_toggled(m, z, t, usize::from(k)))
                });
            }
            BoardPress::Popup => {}
            BoardPress::LetGo => set(self, None, false),
            BoardPress::AddPlace => {
                set(self, None, false);
                self.board_change(&|a, shape| {
                    if a.places < crate::qstep::MAX_PLACES {
                        a.add_place(shape);
                    }
                });
            }
            BoardPress::RemovePlace(p) => {
                set(self, None, false);
                // After the lock only the last place goes (§17.8): a place
                // before another would renumber sticks already named.
                let places = self.board_plan().map_or(0, |(_, a)| a.places);
                if locked && p + 1 != places {
                    return;
                }
                let names = !locked && self.board_backup_names();
                self.board_change(&|a, shape| a.remove_place(shape, p));
                if names
                    && self.board_plan().is_some_and(|(_, a)| a.places < places)
                    && let Some(b) = self.backup.as_mut()
                    && p < b.names.len()
                {
                    b.names.remove(p);
                    b.naming = None;
                }
            }
            BoardPress::AddVault => {
                set(self, None, false);
                self.board_change(&|a, shape| {
                    a.add_vault(shape);
                });
            }
            BoardPress::RemoveVault(v) => {
                // A vault that holds something asks first, with what it
                // holds (§19.12 item 7).
                let holds = self.board_plan().is_some_and(|(shape, a)| {
                    !plan::holds_at(&plan::map(&shape, &a), At::Vault(v)).is_empty()
                });
                if holds && state.removing != Some(v) {
                    set(self, None, false);
                    if let Some(st) = self.board_state_mut() {
                        st.removing = Some(v);
                    }
                    return;
                }
                set(self, None, false);
                self.board_change(&|a, shape| a.remove_vault(shape, v));
            }
            BoardPress::RemoveVaultSure(v) => {
                set(self, None, false);
                self.board_change(&|a, shape| a.remove_vault(shape, v));
            }
            BoardPress::RemoveStick(p, s) => {
                set(self, None, false);
                // After the lock files stay as made: a stick that carries
                // any goes only once it carries a vault alone (§17.8).
                let files = self.board_plan().is_some_and(|(shape, a)| {
                    plan::holds_at(&plan::map(&shape, &a), At::Stick(p, s))
                        .iter()
                        .any(|w| !matches!(w, What::VaultStick(_)))
                });
                if locked && files {
                    return self.toast(EN.backup_flow_files_after_lock);
                }
                self.board_change(&|a, shape| {
                    map_edit(a, shape, |m| {
                        plan::edited(&m, plan::Edit::Clear(At::Stick(p, s)), &[])
                    })
                });
            }
            BoardPress::RemoveSoftware => {
                set(self, None, false);
                if locked {
                    return self.toast(EN.backup_flow_files_after_lock);
                }
                self.board_change(&|a, shape| {
                    map_edit(a, shape, |m| {
                        let m = plan::edited(&m, plan::Edit::Clear(At::Software), &[]);
                        plan::edited(&m, plan::Edit::Clear(At::Files), &[])
                    })
                });
            }
        }
    }

    /// Chip `c` dropped on zone `z`: kept there, moved there, or taken off
    /// on the list; refused when the zone does not take it, with §17.8's
    /// line after the lock (§19.12 item 8).
    fn board_put(&mut self, c: Chip, z: Zone) {
        let t = c.thing();
        if t == Thing::Software {
            if self.board_locked().is_some() {
                return self.toast(EN.backup_flow_files_after_lock);
            }
            return self.board_change(&|a, shape| {
                if !a.software_on(shape) {
                    a.toggle_software(shape);
                }
            });
        }
        if z == Zone::List {
            if let Chip::Placed(from, _) = c {
                self.board_act(BoardPress::Off(from, t));
            }
            return;
        }
        let to = target(z, t);
        if !self.board_takes(z, t) {
            if self.board_locked().is_some() && kind(to).is_some() {
                self.toast(EN.backup_flow_files_after_lock);
            }
            return;
        }
        match c {
            Chip::List(_) => {
                self.board_change(&|a, shape| map_edit(a, shape, |m| added(m, to, t)));
            }
            Chip::Placed(from, _) => {
                self.board_change(&|a, shape| map_edit(a, shape, |m| moved(m, from, t, to)));
            }
        }
    }

    /// A finger down on the board: a press on a movable chip may become
    /// its drag. Returns whether it did, so the page does not scroll
    /// under it.
    pub(crate) fn board_down(&mut self, found: Option<crate::Action>, x: i32, y: i32) -> bool {
        let Some(crate::Action::Board(BoardPress::Chip(c))) = found else {
            if let Some(st) = self.board_state_mut() {
                st.down = None;
                st.drag = None;
            }
            return false;
        };
        // The narrow column places by tap only (§19.12 item 3); a sheet
        // over the board takes no drag.
        if self.compact || self.sheet.is_some() || !self.board_movable(c) {
            return false;
        }
        if !matches!(
            self.screen,
            crate::Screen::Backup | crate::Screen::BackupFlow
        ) {
            return false;
        }
        if c == Chip::List(Thing::Software) && self.board_locked().is_some() {
            return false;
        }
        if let Some(st) = self.board_state_mut() {
            st.down = Some((c, x, y));
            st.drag = None;
            return true;
        }
        false
    }

    /// The finger moved: past the slop a chip pressed is dragged and
    /// follows it. Returns whether the board took the move.
    pub(crate) fn board_move(&mut self, x: i32, y: i32) -> bool {
        let slop = crate::motion::SLOP * self.f;
        let Some(b) = self.board_state_mut() else {
            return false;
        };
        let Some((c, x0, y0)) = b.down else {
            return false;
        };
        if b.drag.is_none() {
            let far = ((x - x0) as f32).abs() > slop || ((y - y0) as f32).abs() > slop;
            if !far {
                return false;
            }
            b.held = Some(c);
            b.forms = false;
            b.removing = None;
        }
        b.drag = Some((x, y));
        self.pressed = None;
        true
    }

    /// The finger lifted: a dragged chip is dropped on the zone under it
    /// that takes it, or goes back. Returns whether it was a drag.
    pub(crate) fn board_up(&mut self, x: i32, y: i32) -> bool {
        let Some(b) = self.board_state_mut() else {
            return false;
        };
        let down = b.down.take();
        let Some(_) = b.drag.take() else {
            return false;
        };
        let Some((c, ..)) = down else {
            return false;
        };
        self.pressed = None;
        let t = c.thing();
        // The topmost zone under the finger that takes it: a chip or its
        // × stands for its zone.
        let under = self.hits.iter().rev().find_map(|(r, a)| {
            if !r.contains(x, y) {
                return None;
            }
            let z = match a {
                crate::Action::Board(BoardPress::Zone(z))
                | crate::Action::Board(BoardPress::Chip(Chip::Placed(z, _)))
                | crate::Action::Board(BoardPress::Off(z, _)) => *z,
                crate::Action::Board(BoardPress::Chip(Chip::List(_))) => Zone::List,
                _ => return None,
            };
            let ok = (z == Zone::List && matches!(c, Chip::Placed(..)))
                || (t == Thing::Software && z != Zone::List)
                || self.board_takes(z, t);
            ok.then_some(z)
        });
        match under {
            Some(z) => {
                if let Some(st) = self.board_state_mut() {
                    st.held = Some(c);
                }
                self.board_act(BoardPress::Zone(z));
            }
            None => self.board_act(BoardPress::LetGo),
        }
        true
    }
}

/// The software the plan names: the first chosen, or "Watch-only
/// software" when it is none or "Not sure". The software place's title,
/// and the name of its own file's form.
fn software_name(a: &Answers) -> String {
    a.software
        .iter()
        .position(|&on| on)
        .filter(|&k| k != plan::software::NOT_SURE)
        .and_then(|k| crate::screens::SOFTWARE_ROWS.get(k))
        .map_or_else(|| EN.board_watch_only.to_string(), |s| (*s).to_string())
}
