//! The backup flow area's state: the flows begun, the one on screen, the
//! keys made here that are held until their backup is done, the number
//! a wallet's flow is tied to while it is made, and the board's list
//! scroll. Reading the area is reading its flows (a read-only `Deref`);
//! [`crate::Faraday::backup_flows`] reads them from outside the crate.
//! Changed through the methods here, and by the backup flow's own glue in
//! `backup_flow/` and its children.

use faraday_bip::keys::Fingerprint;

use faraday_crypto::work::Compute;
use faraday_text::Text;

use super::{
    BackupFlow, BackupRow, Kind, Pick, Problem, Reading, Step, Subject, VaultPlan, WalletFlow,
    hex_text,
};
use crate::board::BoardState;

/// The backup flow area.
#[derive(Default)]
pub(crate) struct BackupFlowArea {
    /// The backup flows, at most one on screen (`docs/BACKUP-FLOW.md` §7).
    pub(super) all: Vec<BackupFlow>,
    /// The flow on screen.
    pub(super) on: Option<usize>,
    /// The keys made here whose backup is not yet done, by fingerprint
    /// (`docs/WALLETS.md` §13): they sign nothing until it
    /// is. Never a secret; kept in a vault beside the seed and in the plan
    /// record.
    pub(super) made_here: Vec<Fingerprint>,
    /// The keys whose copy was written since the app started, by
    /// fingerprint: a vault's record that says its backup is pending holds
    /// none of them, as that mark may be kept for another key of its words.
    copied: Vec<Fingerprint>,
    /// The key records a lock in this session marked "backup pending": by
    /// the vault's identifier, the record's id and the file the seal wrote,
    /// with the keys held here it was marked for. An unlock takes off no
    /// other mark: not one in another copy of the same vault.
    sealed_for: Vec<SealedFor>,
    /// The last number given to a Make a new wallet or a Restore, which a
    /// wallet's backup flow is tied to while the wallet is being made.
    pub(super) owner_seq: u64,
    /// The board's list of things, design units scrolled past
    /// (`docs/BACKUP-FLOW.md` §17.14).
    board_list_offset: f32,
    /// A GPG key's paperkey being typed back, for the restore flow whose
    /// key it names (`gpg_restore.rs`): secret, wiped when it drops, at a
    /// lock and when its flow goes. Kept in step with that flow: it is
    /// read only while the flow's certificate names the same key. Boxed: the
    /// certificate it holds is large, and the area sits by value in `Faraday`.
    pub(super) restoring: Option<Box<super::gpg_restore::RestoreLive>>,
    /// The Sticks and vaults card of each Make a new wallet and Restore
    /// that has vaults answered Yes (`docs/STICK-SETUP.md` §2).
    layouts: Vec<super::layout::SetupLayout>,
    /// The Restore a GPG key sheet's rows: the public keys in Files, read
    /// when it opened.
    pub(super) restore_rows: Vec<super::gpg_restore::SheetRow>,
}

/// A key record a lock in this session marked "backup pending"
/// ([`BackupFlowArea::sealed_for`]).
struct SealedFor {
    vault: [u8; 32],
    record: [u8; 32],
    /// The SHA-256 of the file the open vault was opened from.
    from: Option<[u8; 32]>,
    /// The SHA-256 of the file the seal wrote; `None` while the seal has
    /// not written it, when the entry matches no file.
    file: Option<[u8; 32]>,
    keys: Vec<Fingerprint>,
}

/// Reading the area is reading its flows: `self.flows[i]` reads as the
/// accessor [`crate::Faraday::backup_flows`] does. Nothing is changed
/// through it.
impl std::ops::Deref for BackupFlowArea {
    type Target = Vec<BackupFlow>;
    fn deref(&self) -> &Vec<BackupFlow> {
        &self.all
    }
}

impl BackupFlowArea {
    /// A record the person moved into the vault of file id `salt` is one
    /// the vault held before every wallet flow that keeps what it held
    /// (`WalletFlow::before`): `tokens` are added to that.
    pub(crate) fn held_before_add(&mut self, salt: [u8; 32], tokens: &[Text]) {
        for f in self.all.iter_mut().filter_map(|f| f.wallet.as_mut()) {
            if let Some((_, held)) = f.before.iter_mut().find(|(s, _)| *s == salt) {
                for t in tokens {
                    if !held.iter().any(|h| h.as_str() == t.as_str()) {
                        held.push(Text::of(t));
                    }
                }
            }
        }
    }

    /// Adds the choices a saved Keep sheet made to flow `i`'s.
    pub(crate) fn keeps_add(
        &mut self,
        i: usize,
        left: &[([u8; 4], u8)],
        asks: &[crate::vault::keep::Ask],
    ) {
        if let Some(f) = self.all.get_mut(i) {
            f.keeps.add(left, asks);
        }
    }

    /// The Sticks and vaults card of Make a new wallet or Restore `owner`.
    pub(crate) fn layout_of(&self, owner: u64) -> Option<&super::layout::SetupLayout> {
        self.layouts.iter().find(|l| l.owner() == owner)
    }

    /// The same, to change.
    pub(crate) fn layout_edit(&mut self, owner: u64) -> Option<&mut super::layout::SetupLayout> {
        self.layouts.iter_mut().find(|l| l.owner() == owner)
    }

    /// The card of `owner` has `n` vaults; it is made if there is none.
    pub(crate) fn layout_vaults(&mut self, owner: u64, n: usize, medium: crate::Medium) {
        match self.layout_edit(owner) {
            Some(l) => l.set_vaults(n),
            None => self
                .layouts
                .push(super::layout::SetupLayout::new(owner, n, medium)),
        }
    }

    /// The card of `owner` is let go.
    pub(crate) fn layout_drop(&mut self, owner: u64) {
        self.layouts.retain(|l| l.owner() != owner);
    }

    /// Costs what `place_setup` costs. Flow `i`'s setup takes the card's
    /// sticks.
    pub(crate) fn apply_layout(
        &mut self,
        i: usize,
        c: &Compute,
        places: Vec<Vec<usize>>,
        existing: Vec<bool>,
    ) {
        if let Some(f) = self.all.get_mut(i) {
            f.place_setup(c, places, existing);
        }
    }

    /// The flow on screen.
    pub(crate) fn on(&self) -> Option<usize> {
        self.on
    }

    /// The keys made here whose backup is not yet done.
    pub(crate) fn made_here(&self) -> &[Fingerprint] {
        &self.made_here
    }

    /// Key `fp` was made here: it signs nothing until its backup is done.
    pub(crate) fn note_made(&mut self, fp: Fingerprint) {
        self.copied.retain(|f| *f != fp);
        if !self.made_here.contains(&fp) {
            self.made_here.push(fp);
        }
    }

    /// The keys made here, as the held keys stored say.
    pub(crate) fn set_made_here(&mut self, made: Vec<Fingerprint>) {
        self.made_here = made;
    }

    /// Key `fp`, and the keys `same` with its words, are released: a copy
    /// of them is written.
    pub(crate) fn release_made(&mut self, fp: Fingerprint, same: &[Fingerprint]) {
        self.made_here.retain(|f| *f != fp && !same.contains(f));
        self.note_copied(same.iter().copied().chain(std::iter::once(fp)));
    }

    /// The copies of keys `fps` are written.
    pub(crate) fn note_copied(&mut self, fps: impl IntoIterator<Item = Fingerprint>) {
        for f in fps {
            if !self.copied.contains(&f) {
                self.copied.push(f);
            }
        }
    }

    /// The keys whose copy was written since the app started.
    pub(crate) fn copied(&self) -> &[Fingerprint] {
        &self.copied
    }

    /// Key record `record` of open vault `vault`, opened from the file whose
    /// SHA-256 is `opened`, is marked "backup pending" by a lock for keys
    /// `keys`: afresh when `fresh`, else added to a mark this session made
    /// in that file or in this seal (a mark from an earlier session stays
    /// unknown here). The mark is tied to a file once its seal writes one
    /// ([`Self::note_sealed_file`]).
    // A lock of a file this session wrote that changes nothing writes
    // nothing, so the older sealed entry (naming fewer keys) stays; safe
    // only while session keys leave all at once, so a way to remove one
    // key at a time must first extend the entry on an unwritten seal (or
    // refuse the clear).
    pub(crate) fn note_sealed_for(
        &mut self,
        (vault, opened): ([u8; 32], Option<[u8; 32]>),
        record: [u8; 32],
        keys: &[Fingerprint],
        fresh: bool,
    ) {
        let waiting = self.sealed_for.iter().position(|s| {
            s.vault == vault && s.record == record && s.from == opened && s.file.is_none()
        });
        let mut all: Vec<Fingerprint> = match (waiting, fresh) {
            (Some(i), false) => self.sealed_for.remove(i).keys,
            (Some(i), true) => {
                self.sealed_for.remove(i);
                Vec::new()
            }
            (None, true) => Vec::new(),
            (None, false) => match opened.and_then(|f| self.sealed_for(vault, record, Some(f))) {
                Some(earlier) => earlier.to_vec(),
                None => return,
            },
        };
        for &k in keys {
            if !all.contains(&k) {
                all.push(k);
            }
        }
        self.sealed_for.push(SealedFor {
            vault,
            record,
            from: opened,
            file: None,
            keys: all,
        });
    }

    /// Open vault `vault`, opened from the file whose SHA-256 is `opened`,
    /// was sealed into the file whose SHA-256 is `file`: the marks its lock
    /// made are that file's. A seal that writes nothing ties none.
    pub(crate) fn note_sealed_file(
        &mut self,
        (vault, opened): ([u8; 32], Option<[u8; 32]>),
        file: [u8; 32],
    ) {
        for s in &mut self.sealed_for {
            if s.vault == vault && s.from == opened && s.file.is_none() {
                s.file = Some(file);
            }
        }
    }

    /// The keys a lock in this session marked key record `record` of vault
    /// `vault` "backup pending" for, in the file whose SHA-256 is `file`;
    /// `None` for a mark it did not make in that file.
    pub(crate) fn sealed_for(
        &self,
        vault: [u8; 32],
        record: [u8; 32],
        file: Option<[u8; 32]>,
    ) -> Option<&[Fingerprint]> {
        let file = file?;
        self.sealed_for
            .iter()
            .find(|s| s.vault == vault && s.record == record && s.file == Some(file))
            .map(|s| s.keys.as_slice())
    }

    /// The board's list of things, design units scrolled past.
    pub(crate) fn board_list_offset(&self) -> f32 {
        self.board_list_offset
    }

    /// Sets the board's list scroll.
    pub(crate) fn set_board_list_offset(&mut self, y: f32) {
        self.board_list_offset = y;
    }

    /// Changes flow `i`'s board as `f` says, when the flow is there.
    pub(crate) fn board_edit<R>(
        &mut self,
        i: usize,
        f: impl FnOnce(&mut BoardState) -> R,
    ) -> Option<R> {
        self.all.get_mut(i).map(|w| f(&mut w.live.board))
    }

    /// The boards not on screen (every flow's but `shown`'s) let go of
    /// their empty sticks, menu and focus.
    pub(crate) fn board_tidy(&mut self, shown: Option<usize>) {
        for (i, f) in self.all.iter_mut().enumerate() {
            if shown != Some(i) {
                f.live.board.empty.clear();
                f.live.board.menu = None;
                f.live.board.focus = None;
            }
        }
    }

    /// How far the flow on screen is scrolled, units.
    pub(crate) fn scroll_y(&self) -> Option<f32> {
        self.all.get(self.on?).map(|f| f.live.scroll.y)
    }

    /// The flow on screen scrolled to `y` units.
    pub(crate) fn set_scroll_y(&mut self, y: f32) {
        if let Some(f) = self.on.and_then(|i| self.all.get_mut(i)) {
            f.live.scroll.y = y;
        }
    }

    /// Sets the flow on screen's scroll, as the drawing pass noted it.
    pub(crate) fn set_scroll(&mut self, scroll: crate::flow::Scroll) {
        if let Some(i) = self.on
            && let Some(f) = self.all.get_mut(i)
        {
            f.live.scroll = scroll;
        }
    }

    /// Create's flow put back as it was before its wallet was made: found
    /// by its owner, and only where it is still there, not made, and at the
    /// step it stood at. Otherwise nothing moves and the flow stays as it
    /// is, and the flow comes back to the caller.
    pub(crate) fn restore(&mut self, was: BackupFlow) -> Option<BackupFlow> {
        let Some(owner) = was.wallet.as_ref().map(|w| w.owner).filter(|o| *o != 0) else {
            return Some(was);
        };
        let Some(i) = self.all.iter().position(|f| {
            f.kind == Kind::Wallet
                && !f.made
                && f.step == was.step
                && f.wallet.as_ref().is_some_and(|w| w.owner == owner)
        }) else {
            return Some(was);
        };
        self.all[i] = was;
        None
    }

    /// Whether any flow of Create's `owner` is there, made or not.
    pub(crate) fn has_owner(&self, owner: u64) -> bool {
        self.all
            .iter()
            .any(|f| f.kind == Kind::Wallet && f.wallet.as_ref().is_some_and(|w| w.owner == owner))
    }

    /// Puts `was` back as a flow of its own where no flow of its owner is
    /// there at all, so nothing is overwritten.
    pub(crate) fn put_back(&mut self, was: BackupFlow) {
        let owner = was.wallet.as_ref().map_or(0, |w| w.owner);
        if owner != 0 && !self.has_owner(owner) {
            self.all.push(was);
        }
    }

    /// Whether Create's flow `owner` stands as `was` says it stood: still
    /// there, not made, at that step.
    pub(crate) fn stands_as(&self, owner: u64, step: Step) -> bool {
        self.all.iter().any(|f| {
            f.kind == Kind::Wallet
                && !f.made
                && f.step == step
                && f.wallet.as_ref().is_some_and(|w| w.owner == owner)
        })
    }

    /// Drops flow `i`; what points past it follows. Whether it was the
    /// flow on screen.
    pub(crate) fn drop_flow(&mut self, i: usize) -> bool {
        // A restore's lines go with its flow.
        if self.restoring(i).is_some() {
            self.restore_wipe();
        }
        self.all.remove(i);
        match self.on {
            Some(k) if k == i => {
                self.on = None;
                true
            }
            Some(k) if k > i => {
                self.on = Some(k - 1);
                false
            }
            _ => false,
        }
    }

    /// The files written to the stick of label `label` and serial
    /// `serial` named `from` (in any case) are named `to`, as the stick's
    /// file was renamed.
    pub(crate) fn rename_written(
        &mut self,
        label: &str,
        serial: Option<u32>,
        from: &str,
        to: &str,
    ) {
        for f in &mut self.all {
            for slot in &mut f.slots {
                let Some(w) = slot.written.as_mut() else {
                    continue;
                };
                if w.label != label || w.serial != serial {
                    continue;
                }
                for (n, _) in &mut w.files {
                    if n.eq_ignore_ascii_case(from) {
                        *n = Text::of(to);
                    }
                }
            }
        }
    }

    /// Sets the codex32 string flow `i`'s round stopped on, or clears it
    /// (`Live::c32_wanted`).
    pub(crate) fn set_c32_wanted(&mut self, i: usize, want: Option<crate::plan::codex32::Wanted>) {
        if let Some(f) = self.all.get_mut(i) {
            f.live.c32_wanted = want;
        }
    }

    /// Marks slot `k` of flow `i` as the one the stick put in next is for
    /// (`BackupFlow::want`).
    pub(crate) fn want(&mut self, i: usize, k: usize) {
        if let Some(f) = self.all.get_mut(i) {
            f.want(k);
        }
    }

    /// Changes the flows as `f` says, for the tests' kit.
    #[cfg(any(test, feature = "testkit"))]
    pub(crate) fn change_flows<R>(&mut self, f: impl FnOnce(&mut Vec<BackupFlow>) -> R) -> R {
        f(&mut self.all)
    }
}

/// Which flow a gate line is of.
enum GateFlow {
    Found(usize),
    /// Two or more flows fit and the line cannot tell them apart.
    Unsure,
    None,
}

impl BackupFlowArea {
    /// The flow that made the key `subject`, when it is kept.
    pub(crate) fn of(&self, kind: Kind, subject: &[u8]) -> Option<&BackupFlow> {
        let id = hex_text(subject);
        self.all
            .iter()
            .find(|f| f.kind == kind && f.made && f.subject.as_ref().is_some_and(|s| s.id == id))
    }

    /// The writes the key `subject` waits for: how many are done and how
    /// many there are. `None` once every place the plan gives it is
    /// written, and for a key no flow made here, which signs at once
    /// (§8).
    pub(crate) fn waits(&self, kind: Kind, subject: &[u8]) -> Option<(usize, usize)> {
        self.of(kind, subject)
            .filter(|f| f.waiting())
            .map(|f| f.written_of())
    }

    /// The key page's Backup row: `None` for a key unlocked from a stick.
    pub(crate) fn row(&self, kind: Kind, subject: &[u8]) -> Option<BackupRow> {
        let f = self.of(kind, subject)?;
        let (n, m) = f.written_of();
        let en = &crate::strings::EN;
        let line = if f.all_written() {
            Text::of(en.backup_flow_row_all)
        } else if n == 0 {
            Text::of(en.backup_flow_row_none)
        } else {
            Text::fill(
                en.backup_flow_row_some,
                &[&Text::num(n as u64), &Text::num(m as u64)],
            )
        };
        Some(BackupRow {
            line,
            waiting: !f.all_written(),
        })
    }

    /// The made flow of `kind` and subject `sum` a gate line is of: for a wallet, by its id where both have one (an equal id before a flow with none), by the checksum alone where the line has none and one flow has it.
    fn gate_flow(
        &self,
        kind: Kind,
        sum: &str,
        wid: Option<faraday_bip::policy::WalletId>,
    ) -> GateFlow {
        let cands: Vec<usize> = (0..self.all.len())
            .filter(|&i| {
                let f = &self.all[i];
                f.kind == kind && f.made && f.subject.as_ref().is_some_and(|s| s.id == sum)
            })
            .collect();
        let id_of = |i: usize| self.all[i].wallet.as_ref().and_then(|w| w.id);
        if !kind.is_wallet() {
            return cands
                .first()
                .map_or(GateFlow::None, |&i| GateFlow::Found(i));
        }
        match wid {
            Some(w) => cands
                .iter()
                .copied()
                .find(|&i| id_of(i) == Some(w))
                .or_else(|| cands.iter().copied().find(|&i| id_of(i).is_none()))
                .map_or(GateFlow::None, GateFlow::Found),
            None => match cands.as_slice() {
                [] => GateFlow::None,
                [i] => GateFlow::Found(*i),
                _ => GateFlow::Unsure,
            },
        }
    }

    /// Costs what `BackupFlow::new` costs, once a key whose flow did not
    /// read. Reads the keys that wait. One whose flow did not read comes
    /// back as a flow of its own at Where, made and unwritten, so it still
    /// waits.
    pub(crate) fn gate_restore(&mut self, c: &Compute, bytes: &[u8]) {
        for line in String::from_utf8_lossy(bytes).lines() {
            let mut p = line.splitn(5, '\t');
            let (Some(k), Some(id), Some(name)) = (p.next(), p.next(), p.next()) else {
                continue;
            };
            let held = p.next().unwrap_or("");
            // The wallet's id, where the line has one (a line written
            // before it was kept has none).
            let wid = p.next().and_then(faraday_bip::policy::WalletId::from_hex);
            let Some(kind) = Kind::from_code(k) else {
                continue;
            };
            match self.gate_flow(kind, id, wid) {
                GateFlow::Found(i) => {
                    // What the gate holds back is held, whatever the flow's
                    // own state says.
                    if let Some(w) = self.all[i].wallet.as_mut()
                        && kind.is_wallet()
                    {
                        for h in held.split(',').filter(|x| !x.is_empty()) {
                            if !w.held.iter().any(|x| x.eq_ignore_ascii_case(h)) {
                                w.held.push(Text::of(h));
                            }
                        }
                    }
                    continue;
                }
                // A line that cannot tell two flows of one checksum apart
                // joins neither: it is a flow of its own, so its key waits.
                GateFlow::None | GateFlow::Unsure => {}
            }
            let mut f = BackupFlow::new(c, kind);
            f.made = true;
            f.on_screen = false;
            f.subject = Some(Subject {
                id: Text::of(id),
                name: Text::of(name),
            });
            if kind.is_wallet() {
                let mut w = WalletFlow::blank();
                w.checksum = Some(Text::of(id));
                w.id = wid;
                w.name = Text::of(name);
                w.held = held
                    .split(',')
                    .filter(|x| !x.is_empty())
                    .map(Text::of)
                    .collect();
                f.wallet = Some(w);
            }
            self.all.push(f);
        }
    }

    /// The flow Home leads with and the Power sheet lists: made, and not
    /// every place written.
    pub(crate) fn pending(&self) -> Option<(usize, &BackupFlow)> {
        self.all.iter().enumerate().find(|(_, f)| f.waiting())
    }

    /// What the Power sheet says of each backup not finished (§9).
    pub(crate) fn power_lines(&self) -> Vec<String> {
        self.all
            .iter()
            .filter(|f| f.waiting())
            .map(|f| {
                let (n, m) = f.written_of();
                crate::strings::fill(
                    crate::strings::EN.backup_flow_power,
                    &[
                        &f.subject
                            .as_ref()
                            .map_or(f.kind.name().to_string(), |s| String::from(&*s.name)),
                        &n.to_string(),
                        &m.to_string(),
                    ],
                )
            })
            .collect()
    }

    /// A flow not finished and not yet at the key made, of this kind: what
    /// the kind's tile resumes (**Continue**).
    pub(crate) fn resumable(&self, kind: Kind) -> Option<usize> {
        self.all
            .iter()
            .position(|f| f.kind == kind && !f.made && f.step != Step::Done && f.restore.is_none())
    }

    /// Copies and public files kept in Files for the flow's writes are
    /// listed on the visit as In the backup, not tickable.
    pub(crate) fn in_the_backup(&self, name: &str) -> bool {
        self.all.iter().any(|f| {
            f.made
                && f.step != Step::Done
                && (f.sealing.iter().any(|s| s.name == name)
                    // A Secure Boot key's enrolment files, written last.
                    || f.public.iter().any(|n| n == name)
                    || f.slots.iter().any(|s| s.files.iter().any(|n| n == name)))
        })
    }
}

/// What one flow works out and changes of itself.
impl BackupFlow {
    /// Whether the stick read is one this flow wrote: it holds a file the
    /// flow wrote for another stick (by SHA-256) and reads as that stick
    /// (its label and serial): the first copy of the same vault, or another
    /// place's files. A stick that reads otherwise is compared with what
    /// was read for its own place.
    pub(crate) fn holds_written(&self, r: &Reading) -> Option<Problem> {
        let f = self;
        for (k, s) in f.slots.iter().enumerate() {
            if k == r.slot {
                continue;
            }
            let Some(w) = &s.written else {
                continue;
            };
            // A vault file by its SHA-256; a file beside it, which is not
            // read, by its name in the listing (a stick of files holds no
            // vault, `docs/BACKUP-FLOW.md` §17.4).
            let holds = r
                .files
                .iter()
                .any(|(vr, _)| w.files.iter().any(|(_, sha)| *sha == vr.sha))
                || w.files.iter().any(|(n, _)| {
                    !n.to_ascii_lowercase().ends_with(".kdbx")
                        && r.listing.iter().any(|(x, _)| x == n)
                });
            if holds && w.label == r.label && w.serial == r.serial {
                let mine = f.vaults_of(r.slot);
                let theirs = f.vaults_of(k);
                let same_vault = mine.iter().any(|v| theirs.contains(v));
                return Some(if same_vault {
                    Problem::HasFirst
                } else {
                    Problem::HasFilesOf(s.target)
                });
            }
        }
        None
    }

    /// The stick of place `p` carries the key's public files.
    pub(crate) fn public_at(&mut self, p: usize) {
        if self.kind.is_wallet() {
            return;
        }
        let f = self;
        if p >= f.plan.places.len() || f.slots_at(p).is_empty() {
            return;
        }
        // Files already written stay where they are.
        if f.slots
            .iter()
            .any(|s| s.written.is_some() && s.files.iter().any(|n| f.public.contains(n)))
        {
            return;
        }
        f.plan.public_at = p;
        let names = f.public.clone();
        for s in &mut f.slots {
            s.files.retain(|n| !names.contains(n));
        }
        if f.kind != Kind::SecureBoot
            && let Some(k) = f.public_slot()
        {
            f.slots[k].files.extend(names);
        }
    }

    pub(crate) fn needs_bind(&self, slot: usize) -> bool {
        let f = self;
        let Some(s) = f.slots.get(slot) else {
            return false;
        };
        if s.bound.is_some() || self.is_later(slot) {
            return false;
        }
        match s.target {
            t if t.vault_stick() => {
                if !f.adding {
                    return true;
                }
                // The round reads only a stick that holds a vault the key
                // is to be copied into.
                f.vaults_of(slot)
                    .iter()
                    .any(|&v| f.plan.vaults[v].existing && !f.plan.vaults[v].found())
            }
            _ => false,
        }
    }

    /// "I do not have its passphrase": a new vault beside the one on the
    /// stick; the plan's vault becomes new and is written beside that
    /// file at step 6.
    pub(crate) fn beside_it(&mut self, v: usize) {
        let f = self;
        let Some(vp) = f.plan.vaults.get_mut(v) else {
            return;
        };
        // The file stays in Files; it is no longer the plan's vault.
        *vp = VaultPlan::default();
        f.live.no_pass = None;
        if f.live.unlocking == Some(v) {
            f.live.unlocking = None;
        }
        for s in &mut f.slots {
            if let Some(b) = s.bound.as_mut() {
                for (pv, pick) in &mut b.picks {
                    if *pv == v {
                        *pick = Pick::New;
                    }
                }
            }
        }
    }

    /// Leave this stick for another. A slot that is not written loses its
    /// binding, at step 6 too (a stick lost or broken, a read-only one whose
    /// write failed): the next stick takes the path of a stick named after
    /// step 2.
    pub(crate) fn use_another(&mut self) {
        let f = self;
        if let Some(r) = f.live.reading.take() {
            if let Some(s) = f.slots.get_mut(r.slot)
                && s.written.is_none()
            {
                s.bound = None;
            }
            f.live.ignored.push(r.id);
        }
        f.live.problem = None;
    }

    pub(crate) fn refuse(&mut self, p: Problem) {
        if let Some(r) = self.live.reading.as_mut() {
            r.problem = Some(p);
        }
    }
}

/// Whether `n`, a file on a stick, is one of `files` the flow writes
/// there, or a numbered copy of one: its stem, `-` and more, then the same
/// extension. A lookup per file.
pub(super) fn own_name(files: &[Text], n: &str) -> bool {
    let numbered = |stem: &str, of: &str| of.strip_prefix(stem).is_some_and(|t| t.starts_with('-'));
    files.iter().any(|x| {
        *x == n
            || match (x.rsplit_once('.'), n.rsplit_once('.')) {
                (Some((xs, xe)), Some((ns, ne))) => xe == ne && numbered(xs, ns),
                (None, None) => numbered(x, n),
                _ => false,
            }
    })
}

// A backup flow's restore.
impl crate::at_lock::AtLock for BackupFlowArea {
    fn holds(&self) -> crate::at_lock::Holds {
        // The flows themselves go on across a lock (`lock_flows`); a GPG
        // restore's typed lines do not.
        crate::at_lock::Holds::typed(self.restoring.is_some())
    }
    fn lock(&mut self) {
        self.restore_wipe();
    }
}

#[cfg(test)]
mod tests {
    use super::own_name;

    /// A stick must read as it did, but for the flow's own files: a file
    /// named with one of their stems but of another type is somebody
    /// else's, and the stick reads as changed.
    #[test]
    fn a_file_under_an_own_stem_of_another_type_is_not_the_flows_own() {
        let files = vec![faraday_text::Text::of("release-descriptor.txt")];
        assert!(own_name(&files, "release-descriptor.txt"));
        assert!(own_name(&files, "release-descriptor-2.txt"));
        assert!(!own_name(&files, "release-descriptor-2.pdf"));
        assert!(!own_name(&files, "release-descriptor-notes.exe"));
        assert!(!own_name(&files, "release-descriptor.txt.exe"));
    }
}
