//! The Sticks and vaults card of Make a new wallet and Restore
//! (`docs/STICK-SETUP.md` §2): how many vaults and how many sticks, and
//! which vaults go on which stick, laid out and moved as on the backup
//! board (the board's chips, held and dragged alike). The layout is made
//! when Use vaults is answered Yes and is read when Continue builds the
//! setup's plan. Drawing is `layout_screen.rs`.

use super::area::BackupFlowArea;
use super::{BackupFlow, Step, VaultPlan};
use crate::Medium;
use crate::board::lines::stick_word;
use crate::board::{BoardPress, BoardState, Chip, Menu, MenuRow, Thing, Verb, Zone};
use crate::strings::{EN, fill};
use crate::{Action, MAX_VAULTS};
use faraday_crypto::work::Compute;

/// What the checks under the layout say (`docs/STICK-SETUP.md` §2.5).
pub(crate) struct Check {
    /// The line.
    pub line: String,
    /// It holds Continue.
    pub blocks: bool,
}

/// The vaults laid on the sticks of one Make a new wallet or Restore.
pub(crate) struct SetupLayout {
    /// The Make a new wallet or Restore it belongs to.
    owner: u64,
    /// How many vaults.
    vaults: usize,
    /// Each stick's vaults, by vault index; the sticks are Stick 2,
    /// Stick 3, … in order.
    on: Vec<Vec<usize>>,
    /// Whether the Sticks count still follows the vaults' (it does until
    /// it is set).
    follow: bool,
    /// Something was moved by hand: a count changed no longer places
    /// everything afresh.
    edited: bool,
    /// Whether each vault is already on a stick.
    existing: Vec<bool>,
    /// The chip held, dragged or menu open, as on the board.
    board: BoardState,
    /// What the sticks are called.
    medium: Medium,
}

impl SetupLayout {
    /// The layout of `vaults` vaults, each on a stick of its own, on
    /// `medium`.
    pub(crate) fn new(owner: u64, vaults: usize, medium: Medium) -> SetupLayout {
        let vaults = vaults.clamp(1, MAX_VAULTS);
        SetupLayout {
            owner,
            medium,
            vaults,
            on: Self::first(vaults, vaults),
            follow: true,
            edited: false,
            existing: vec![false; vaults],
            board: BoardState::default(),
        }
    }

    /// What the sticks are called.
    pub(crate) fn medium(&self) -> Medium {
        self.medium
    }

    /// The first placement: vault `v` on stick `v`, the rest on the last
    /// stick, and a stick past the vaults holds a copy of the last vault.
    fn first(vaults: usize, sticks: usize) -> Vec<Vec<usize>> {
        let mut on = vec![Vec::new(); sticks];
        for v in 0..vaults {
            on[v.min(sticks - 1)].push(v);
        }
        for stick in on.iter_mut().skip(vaults) {
            stick.push(vaults - 1);
        }
        on
    }

    /// The Make a new wallet or Restore it belongs to.
    pub(crate) fn owner(&self) -> u64 {
        self.owner
    }

    /// How many vaults.
    pub(crate) fn vaults(&self) -> usize {
        self.vaults
    }

    /// How many sticks.
    pub(crate) fn sticks(&self) -> usize {
        self.on.len()
    }

    /// Stick `s`'s vaults.
    pub(crate) fn on(&self, s: usize) -> &[usize] {
        self.on.get(s).map_or(&[], Vec::as_slice)
    }

    /// Whether vault `v` is already on a stick.
    pub(crate) fn existing(&self, v: usize) -> bool {
        self.existing.get(v).copied().unwrap_or(false)
    }

    /// Vault `v` is new, or already on a stick.
    pub(crate) fn set_existing(&mut self, v: usize, existing: bool) {
        if let Some(e) = self.existing.get_mut(v) {
            *e = existing;
        }
    }

    /// The board state: what is held, dragged, and open.
    pub(crate) fn board(&self) -> &BoardState {
        &self.board
    }

    /// The board state, to change.
    pub(crate) fn board_edit(&mut self) -> &mut BoardState {
        &mut self.board
    }

    /// The name of stick `s`, as the board names it.
    pub(crate) fn stick_name(&self, s: usize) -> String {
        fill(stick_word(self.medium), &[&(s + 2).to_string()])
    }

    /// The name of vault `v`.
    pub(crate) fn vault_name(v: usize) -> String {
        fill(EN.plan_vault, &[&(v + 1).to_string()])
    }

    /// Whether stick `s` holds vault `v`.
    pub(crate) fn holds(&self, s: usize, v: usize) -> bool {
        self.on(s).contains(&v)
    }

    /// The sticks that hold vault `v`.
    pub(crate) fn sticks_of(&self, v: usize) -> Vec<usize> {
        (0..self.on.len()).filter(|&s| self.holds(s, v)).collect()
    }

    /// Sets the number of vaults: a vault more goes where the first
    /// placement puts it; a vault fewer is the last one.
    pub(crate) fn set_vaults(&mut self, n: usize) {
        let n = n.clamp(1, MAX_VAULTS);
        if n == self.vaults {
            return;
        }
        let was = self.vaults;
        self.vaults = n;
        self.existing.resize(n, false);
        if !self.edited {
            let sticks = if self.follow { n } else { self.on.len() };
            self.on = Self::first(n, sticks);
            return;
        }
        for stick in &mut self.on {
            stick.retain(|&v| v < n);
        }
        if self.follow {
            self.on.resize(n, Vec::new());
        }
        let last = self.on.len() - 1;
        for v in was..n {
            self.on[v.min(last)].push(v);
        }
    }

    /// Sets the number of sticks: the last one goes, with a vault it held
    /// alone going back to the list; a stick more takes the vault of its
    /// number that is on none, or a copy of the last vault.
    pub(crate) fn set_sticks(&mut self, n: usize) {
        let n = n.clamp(1, MAX_VAULTS);
        self.follow = false;
        if n == self.on.len() {
            return;
        }
        if !self.edited {
            self.on = Self::first(self.vaults, n);
            return;
        }
        self.on.truncate(n);
        for s in self.on.len()..n {
            let stick = if s >= self.vaults {
                vec![self.vaults - 1]
            } else if self.sticks_of(s).is_empty() {
                vec![s]
            } else {
                Vec::new()
            };
            self.on.push(stick);
        }
    }

    /// Vault `v` also on stick `s`.
    pub(crate) fn put(&mut self, v: usize, s: usize) {
        if v < self.vaults && s < self.on.len() && !self.holds(s, v) {
            self.on[s].push(v);
            self.edited = true;
        }
    }

    /// Vault `v` off stick `s`.
    pub(crate) fn take_off(&mut self, v: usize, s: usize) {
        if let Some(stick) = self.on.get_mut(s) {
            stick.retain(|&u| u != v);
            self.edited = true;
        }
    }

    /// Vault `v` off stick `from` and on stick `to`.
    pub(crate) fn move_to(&mut self, v: usize, from: usize, to: usize) {
        if from != to {
            self.put(v, to);
            self.take_off(v, from);
        }
    }

    /// The checks under the layout, in the order of
    /// `docs/STICK-SETUP.md` §2.5.
    pub(crate) fn checks(&self) -> Vec<Check> {
        let mut out = Vec::new();
        for v in 0..self.vaults {
            if self.sticks_of(v).is_empty() {
                out.push(Check {
                    line: fill(
                        EN.backup_flow_layout_on_none,
                        &[&Self::vault_name(v), self.medium.noun()],
                    ),
                    blocks: true,
                });
            }
        }
        for (s, vs) in self.on.iter().enumerate() {
            let stick = self.stick_name(s);
            let names: Vec<String> = vs.iter().map(|&v| Self::vault_name(v)).collect();
            let line = match names.as_slice() {
                [] => fill(EN.backup_flow_layout_stick_empty, &[&stick]),
                [_] => continue,
                [a, b] => fill(EN.backup_flow_layout_share_two, &[a, b, &stick]),
                [rest @ .., last] => fill(
                    EN.backup_flow_layout_share_many,
                    &[&format!("{} and {last}", rest.join(", ")), &stick],
                ),
            };
            out.push(Check {
                line,
                blocks: false,
            });
        }
        out
    }

    /// Whether Continue is held: a vault is on no stick.
    pub(crate) fn blocked(&self) -> bool {
        (0..self.vaults).any(|v| self.sticks_of(v).is_empty())
    }

    /// What the setup's plan takes: each stick that holds a vault as one
    /// place, and whether each vault is already on a stick.
    pub(crate) fn plan_parts(&self) -> (Vec<Vec<usize>>, Vec<bool>) {
        let places = self.on.iter().filter(|s| !s.is_empty()).cloned().collect();
        (places, self.existing.clone())
    }

    /// The ⋮ menu of chip `c`, worked out for the sticks as they are now.
    fn menu_of(&self, c: Chip, verb: Option<Verb>, anchor: Option<(i32, i32, i32, i32)>) -> Menu {
        let Some(Thing::Vault(v)) = Some(c.thing()) else {
            return Menu {
                chip: c,
                verb,
                title: String::new(),
                rows: Vec::new(),
                at: 0,
                keys: false,
                from_list: false,
                anchor,
            };
        };
        let from = match c {
            Chip::Placed(Zone::Card(s), _) => Some(s),
            _ => None,
        };
        let item = |label: &str, p: BoardPress| MenuRow::Item(label.to_string(), p);
        let boxes = |only_free: bool| -> Vec<MenuRow> {
            (0..self.on.len())
                .filter(|&s| Some(s) != from && (!only_free || !self.holds(s, v)))
                .map(|s| item(&self.stick_name(s), BoardPress::MenuTo(Zone::Card(s))))
                .collect()
        };
        let (title, rows) = match (c, verb) {
            (Chip::List(_), _) => (EN.backup_flow_layout_put_on.to_string(), boxes(true)),
            (_, Some(Verb::Move)) => (EN.board_menu_move.to_string(), boxes(true)),
            (_, Some(_)) => (EN.board_menu_copy.to_string(), boxes(true)),
            (Chip::Placed(z, _), None) => {
                let mut rows = Vec::new();
                if !boxes(true).is_empty() {
                    rows.push(item(EN.board_menu_move, BoardPress::MenuVerb(Verb::Move)));
                    rows.push(item(EN.board_menu_copy, BoardPress::MenuVerb(Verb::Copy)));
                }
                rows.push(item(EN.board_take_off, BoardPress::Off(z, Thing::Vault(v))));
                (Self::vault_name(v), rows)
            }
        };
        let verb = match (c, verb) {
            (Chip::List(_), _) => Some(Verb::Add),
            _ => verb,
        };
        Menu {
            chip: c,
            verb,
            title,
            rows,
            at: 0,
            keys: false,
            from_list: false,
            anchor,
        }
    }

    /// A press of the card's board. Whether the card changed.
    fn press(&mut self, p: BoardPress, anchor: Option<(i32, i32, i32, i32)>) {
        match p {
            BoardPress::Chip(c) => {
                self.board.held = if self.board.held == Some(c) {
                    None
                } else {
                    Some(c)
                };
            }
            BoardPress::Zone(z) => self.drop_on(z),
            BoardPress::Off(Zone::Card(s), Thing::Vault(v)) => {
                self.take_off(v, s);
                self.shut();
            }
            BoardPress::Menu(c) => {
                self.board.held = None;
                self.board.menu = Some(self.menu_of(c, None, anchor));
            }
            BoardPress::MenuVerb(verb) => {
                if let Some(m) = self.board.menu.take() {
                    self.board.menu = Some(self.menu_of(m.chip, Some(verb), m.anchor));
                }
            }
            BoardPress::MenuTo(Zone::Card(to)) => {
                if let Some(m) = self.board.menu.take()
                    && let Thing::Vault(v) = m.chip.thing()
                {
                    match (m.chip, m.verb) {
                        (Chip::Placed(Zone::Card(from), _), Some(Verb::Move)) => {
                            self.move_to(v, from, to);
                        }
                        _ => self.put(v, to),
                    }
                }
            }
            BoardPress::Form(_, Thing::Vault(v), k) => self.set_existing(v, k == 1),
            BoardPress::LetGo => self.board.held = None,
            BoardPress::MenuClose => self.board.menu = None,
            _ => {}
        }
    }

    /// The chip held, dropped or tapped onto zone `z`.
    fn drop_on(&mut self, z: Zone) {
        let Some(c) = self.board.held.take() else {
            return;
        };
        let Thing::Vault(v) = c.thing() else {
            return;
        };
        match (c, z) {
            (Chip::List(_), Zone::Card(s)) => self.put(v, s),
            (Chip::Placed(Zone::Card(from), _), Zone::Card(to)) => self.move_to(v, from, to),
            (Chip::Placed(Zone::Card(from), _), Zone::List) => self.take_off(v, from),
            _ => {}
        }
    }

    /// Whether the chip `c` can be dropped on zone `z`.
    fn takes(&self, c: Chip, z: Zone) -> bool {
        let Thing::Vault(v) = c.thing() else {
            return false;
        };
        match (c, z) {
            (Chip::List(_), Zone::Card(s)) => s < self.on.len() && !self.holds(s, v),
            (Chip::Placed(Zone::Card(from), _), Zone::Card(to)) => {
                from != to && to < self.on.len() && !self.holds(to, v)
            }
            (Chip::Placed(..), Zone::List) => true,
            _ => false,
        }
    }

    /// Whether zone `z` takes the chip held now: drawn edged.
    pub(crate) fn held_takes(&self, z: Zone) -> bool {
        self.board.held.is_some_and(|c| self.takes(c, z))
    }

    fn shut(&mut self) {
        self.board.menu = None;
    }
}

impl BackupFlowArea {
    /// A press of the card's board of `owner`. Whether its ⋮ menu is open
    /// after it.
    pub(crate) fn layout_press(
        &mut self,
        owner: u64,
        p: BoardPress,
        anchor: Option<(i32, i32, i32, i32)>,
    ) -> bool {
        let Some(l) = self.layout_edit(owner) else {
            return false;
        };
        l.press(p, anchor);
        l.board().menu.is_some()
    }

    /// Whether the card of `owner` holds Continue: a vault is on no stick.
    pub(crate) fn layout_blocked(&self, owner: u64) -> bool {
        self.layout_of(owner).is_some_and(SetupLayout::blocked)
    }

    /// The card's ⋮ menu, while it is open.
    pub(crate) fn layout_menu(&self, owner: u64) -> Option<Menu> {
        self.layout_of(owner)?.board().menu.clone()
    }

    /// A finger down on the card of `owner`: a chip pressed may become its
    /// drag, unless `still` (a sheet is open, or the window is compact).
    /// Whether the card took it.
    pub(crate) fn layout_down(
        &mut self,
        owner: u64,
        found: Option<Action>,
        at: (i32, i32),
        still: bool,
    ) -> bool {
        let Some(l) = self.layout_edit(owner) else {
            return false;
        };
        let b = l.board_edit();
        b.drag = None;
        let Some(Action::Board(BoardPress::Chip(c))) = found else {
            b.down = None;
            return false;
        };
        if still {
            return false;
        }
        b.down = Some((c, at.0, at.1));
        true
    }

    /// The finger moved with a chip of the card pressed: it is dragged
    /// past `slop`. `None`: no chip is pressed.
    pub(crate) fn layout_drag(&mut self, owner: u64, at: (i32, i32), slop: f32) -> Option<bool> {
        let b = self.layout_edit(owner)?.board_edit();
        let (c, x0, y0) = b.down?;
        if b.drag.is_none() {
            let far = ((at.0 - x0) as f32).abs() > slop || ((at.1 - y0) as f32).abs() > slop;
            if !far {
                return Some(false);
            }
            b.held = Some(c);
        }
        b.drag = Some(at);
        Some(true)
    }

    /// The finger lifted: the press that drops the chip dragged on the
    /// zone under it that takes it, or lets it go. `None`: nothing was
    /// dragged.
    pub(crate) fn layout_lift(
        &mut self,
        owner: u64,
        at: (i32, i32),
        hits: &[(faraday_draw::Rect, Action)],
    ) -> Option<BoardPress> {
        let b = self.layout_edit(owner)?.board_edit();
        let down = b.down.take();
        b.drag.take()?;
        let (c, ..) = down?;
        let l = self.layout_of(owner)?;
        let under = hits.iter().rev().find_map(|(r, a)| {
            if !r.contains(at.0, at.1) {
                return None;
            }
            let z = match a {
                Action::Board(BoardPress::Zone(z))
                | Action::Board(BoardPress::Chip(Chip::Placed(z, _)))
                | Action::Board(BoardPress::Off(z, _)) => *z,
                Action::Board(BoardPress::Chip(Chip::List(_))) => Zone::List,
                _ => return None,
            };
            l.takes(c, z).then_some(z)
        });
        Some(match under {
            Some(z) => {
                if let Some(l) = self.layout_edit(owner) {
                    l.board_edit().held = Some(c);
                }
                BoardPress::Zone(z)
            }
            None => BoardPress::LetGo,
        })
    }

    /// Costs what `place_setup` costs. Flow `i`'s setup follows the card
    /// of `owner`: Continue builds the plan from the layout.
    pub(crate) fn layout_apply(&mut self, i: usize, owner: u64, c: &Compute) {
        if let Some((places, existing)) = self.layout_of(owner).map(SetupLayout::plan_parts) {
            self.apply_layout(i, c, places, existing);
        }
    }

    /// The card follows the answers of Use vaults and the count: Yes makes
    /// it with the vaults counted, No drops it.
    pub(crate) fn layout_follow(&mut self, owner: u64, count: Option<usize>, medium: Medium) {
        match count {
            None => self.layout_drop(owner),
            Some(n) => self.layout_vaults(owner, n, medium),
        }
    }
}

impl BackupFlow {
    /// Costs what `sync_slots` costs. The setup's sticks are the card's
    /// layout: `places` are the sticks that hold a vault, each with its
    /// vaults, and `existing` says which vaults are already on a stick.
    /// Every stick and vault already read or opened stays as it is.
    pub(crate) fn place_setup(
        &mut self,
        c: &Compute,
        places: Vec<Vec<usize>>,
        existing: Vec<bool>,
    ) {
        let count = existing.len();
        let together: Vec<usize> = (0..places.len()).filter(|&p| places[p].len() > 1).collect();
        let same = self.plan.places == places
            && self.plan.together == together
            && self.plan.vaults.len() == count
            && existing
                .iter()
                .zip(&self.plan.vaults)
                .all(|(&e, vp)| vp.found() || vp.existing == e);
        if same {
            return;
        }
        // A stick read for a place that now holds other vaults is not that
        // place's: it is forgotten, with the vaults chosen from it.
        let moved = |p: usize| self.plan.places.get(p) != places.get(p);
        for s in &mut self.slots {
            if s.target.place().is_some_and(moved) {
                s.bound = None;
            }
        }
        for vp in &mut self.plan.vaults {
            if !vp.found() && vp.from.as_ref().is_some_and(|f| moved(f.0)) {
                vp.from = None;
                vp.file = None;
            }
        }
        self.plan.places = places;
        self.plan.together = together;
        self.plan.vaults.truncate(count);
        self.plan.vaults.resize_with(count, VaultPlan::default);
        for (vp, &e) in self.plan.vaults.iter_mut().zip(&existing) {
            if !vp.found() {
                vp.existing = e;
            }
        }
        if let Some(w) = self.wallet.as_mut() {
            w.shape.vaults = count;
            w.answers = crate::plan::Answers::defaults(&w.shape);
            w.answers.custom = true;
        }
        self.made_in.retain(|&v| v < count);
        self.sync_slots(c);
        for s in &mut self.slots {
            if let Some(b) = s.bound.as_mut() {
                b.picks.retain(|(v, _)| *v < count);
            }
        }
        self.step = Step::Sticks;
        self.live.open = Some(Step::Sticks.n());
    }
}
