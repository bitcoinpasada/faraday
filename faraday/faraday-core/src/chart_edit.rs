//! What the wallet's chart keeps (`docs/WALLETS.md` §12):
//! dates checked, marks (lost, exposed), a key's holder and a
//! vault's name are notes the plan's vault record keeps (type 11, field
//! 5), in the vault alone, and moving the money to a new wallet. The map
//! is changed on the backup's board.

use crate::glance::sheet::Target;
use crate::glance::{self, Press};
use crate::plan::{At, Mark, Note, What};
use crate::secret_text::SecretText;
use crate::strings::EN;
use crate::{Action, Faraday, Sheet};
use faraday_crypto::work::Compute;
use faraday_text::{Text, text};

/// What the chart's sheets hold between presses.
#[derive(Default)]
pub struct Work {
    /// The name typed: a key's holder, a vault's.
    pub text: Text,
    /// A passphrase typed to be checked, wiped once compared.
    pub secret: SecretText,
    /// What the last check of a passphrase found.
    pub result: Option<bool>,
    /// A letter typed was refused: a passphrase here is printable ASCII.
    pub refused: bool,
    /// The wallet whose money is moving to a new wallet, by checksum
    /// (`docs/WALLETS.md` §12).
    pub moving: Option<Text>,
    /// The new wallet it moves to, once Create has made it, by checksum.
    pub moving_to: Option<Text>,
}

impl Work {
    /// Create made wallet `i` of `session`: the wallet the money moves
    /// to, while a move is under way.
    pub(crate) fn moved_to(&mut self, session: &crate::wallet::Session, i: usize) {
        if self.moving.is_some() {
            self.moving_to = session.wallets.get(i).map(|w| Text::of(w.checksum()));
        }
    }
}

/// What [`Faraday::press_wallet_wait`] found.
pub(crate) enum PressWallet {
    /// The loaded wallet's place.
    Ready(usize),
    /// Its seeds are being loaded; the press is run again after.
    Loading,
    /// No wallet.
    None,
}

impl Faraday {
    /// Costs what `plan_vault_of` and `described_of` cost. The notes the
    /// plan's record keeps for `wallet` (§9.5): in open vault `v`, or in
    /// whichever open vault keeps its plan. None kept, or no vault open:
    /// none.
    pub(crate) fn plan_notes(
        &self,
        c: &Compute,
        wallet: &crate::wallet::Wallet,
        v: Option<usize>,
    ) -> Vec<Note> {
        use faraday_vault::records::{field, kind};
        let want = wallet.same();
        let kept_in = |v: usize| {
            self.vaults
                .described_of(c, self.vaults.open[v].id, kind::PLAN)
                .any(|(_, _, d)| d.same == *want)
        };
        let Some(v) = v.or_else(|| (0..self.vaults.open.len()).find(|&v| kept_in(v))) else {
            return Vec::new();
        };
        let Some(open) = self.vaults.open.get(v) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (_, r, d) in self
            .vaults
            .described_of(c, open.id, kind::PLAN)
            .filter(|(_, _, d)| d.same == *want)
        {
            // A record the wallet's keys do not read is not read here.
            if !crate::vault::plan_store::reads_for(d, r, wallet.policy()) {
                continue;
            }
            for b in r.public_all(field::PLAN_NOTE) {
                // The seeds are counted only for a record that has notes.
                let seeds = self.plan_shape_of(wallet).seeds.len();
                out.extend(crate::vault::plan_store::note_in_order(
                    d,
                    &Text::from_utf8_lossy(b),
                    wallet.policy(),
                    seeds,
                ));
            }
        }
        out
    }

    /// Today's local date, `YYYY-MM-DD`; empty unless the clock is
    /// checked (`docs/TOTP.md` §1.3): a date kept is a checked one.
    pub(crate) fn today(&self) -> Text {
        if !self.clock_checked() {
            return Text::new();
        }
        self.clock_now()
            .map(|t| {
                Text::of(&crate::gpg::date(crate::vault::local_secs(
                    t,
                    self.tz_min(),
                )))
            })
            .unwrap_or_default()
    }

    /// The loaded wallet a chart is of: loaded first, with its seeds the
    /// vault keeps, when the chart is an open vault's.
    pub(crate) fn press_wallet(&mut self, press: Press) -> Option<usize> {
        match press {
            Press::Loaded(w) => Some(w).filter(|&w| w < self.session.wallets.len()),
            Press::Vault(v, r) => self.vault_load_wallet(v, r),
        }
    }

    /// Costs what `press_wallet` costs. As `press_wallet`, saying whether
    /// a load of seeds was started for it: a press that has changed state
    /// of its own must do so only after this answers, for it is run again
    /// when the load has, and finds that state used up.
    pub(crate) fn press_wallet_wait(&mut self, press: Press) -> PressWallet {
        let started = self.vaults.loads_started();
        match self.press_wallet(press) {
            Some(w) => PressWallet::Ready(w),
            None if self.vaults.loads_started() != started => PressWallet::Loading,
            None => PressWallet::None,
        }
    }

    /// Costs what `plan_vault_of` costs. The open vault a chart's notes and
    /// plan are kept in: the vault drawing it, else the one keeping the
    /// wallet's plan, else the current one.
    fn notes_vault(&self, c: &Compute, press: Press, w: usize) -> Option<usize> {
        if let Press::Vault(v, _) = press {
            return Some(v);
        }
        let wallet = self.session.wallets.get(w)?;
        self.plan_vault_of(c, wallet).or_else(|| {
            self.vaults
                .open
                .get(self.vaults.current)
                .map(|_| self.vaults.current)
        })
    }

    /// Changes the notes kept for the chart's wallet with `change`, in the
    /// vault that keeps its plan; a plan kept in no open vault yet is
    /// saved into the current one first. False with no vault open, and
    /// with a toast when every record kept reads only for another key order.
    pub(crate) fn chart_notes_change(
        &mut self,
        press: Press,
        change: &dyn Fn(&mut Vec<Note>),
    ) -> bool {
        let c = self.compute();
        use faraday_vault::records::{field, kind};
        let Some(w) = self.press_wallet(press) else {
            return false;
        };
        let Some(v) = self.notes_vault(&c, press, w) else {
            return false;
        };
        let Some(wallet) = self.session.wallets.get(w) else {
            return false;
        };
        let mut notes = self.plan_notes(&c, wallet, Some(v));
        change(&mut notes);
        let want = wallet.same();
        // This wallet's plan records, by place, each with the notes in its
        // own seed numbers (its keys may be in another order); None for
        // one that does not map onto the wallet's keys, or whose shares
        // the other order would change.
        let ours: Vec<(usize, Option<Vec<faraday_text::Text>>)> =
            self.vaults.open.get(v).map_or_else(Vec::new, |o| {
                self.vaults
                    .described_of(&c, o.id, kind::PLAN)
                    .filter(|(_, _, d)| d.same == want)
                    .map(|(i, r, d)| {
                        if !crate::vault::plan_store::reads_for(d, r, wallet.policy()) {
                            return (i, None);
                        }
                        let map = crate::vault::plan_store::seed_map(wallet.policy(), &d.policy);
                        let same = crate::vault::plan_store::same_keys(d, wallet.policy());
                        let texts = match (&map, same) {
                            (_, true) => Some(notes.iter().map(|n| n.to_text()).collect()),
                            (Some(m), false) => Some(
                                notes
                                    .iter()
                                    .filter_map(|n| Some(n.renumbered(m)?.to_text()))
                                    .collect(),
                            ),
                            (None, false) => None,
                        };
                        (i, texts)
                    })
                    .collect()
            });
        let has = !ours.is_empty();
        if !has {
            // No record of this plan here yet: the plan the chart shows,
            // saved with the notes.
            let Some((answers, names)) = crate::wallet_backup::plan_now(&c, self, w) else {
                return false;
            };
            self.plan_store(v, w, &answers, &names, &[], Some(&notes));
            return true;
        }
        let Some(id) = self.vaults.open.get(v).map(|o| o.id) else {
            return false;
        };
        let c = self.compute();
        // Each record's notes in its own seed numbers; one that does not
        // map onto the wallet's keys is left as it is. The room is
        // checked for every record before any is written.
        let kept: Vec<(usize, Vec<&[u8]>)> = ours
            .iter()
            .filter_map(|(i, texts)| {
                Some((*i, texts.as_ref()?.iter().map(|t| t.as_bytes()).collect()))
            })
            .collect();
        let each: Vec<(usize, &[&[u8]])> = kept.iter().map(|(i, v)| (*i, v.as_slice())).collect();
        // No record reads for this wallet: nothing is saved, and the
        // toast says why.
        if each.is_empty() {
            self.toast(EN.chart_edit_keys_order);
            return false;
        }
        if let Err(why) = self.vaults.edit(id, |s| {
            s.set_public_each_in(&c, id, field::PLAN_NOTE, &each)
        }) {
            let text = self.not_set_text(Some(id), why);
            self.toast(&text);
            return false;
        }
        self.vaults.changed(id);
        true
    }

    /// Marks the thing at line `line` of backup node `n` (or every thing
    /// in node `n`) `mark`, in the vault, and shows what follows.
    pub(crate) fn chart_mark(&mut self, press: Press, n: u8, line: Option<u8>, mark: Mark) {
        let c = self.compute();
        let Some(things) = self.chart_things(&c, press, n, line) else {
            return;
        };
        let kept = self.chart_notes_change(press, &|notes: &mut Vec<Note>| {
            for &(at, what) in &things {
                notes.retain(|x| !matches!(x, Note::Marked(a, w, _) if *a == at && *w == what));
                notes.push(Note::Marked(at, what, mark));
            }
        });
        if kept {
            self.glance
                .set_chart(Some((press, Target::Marked(n, line))));
            self.sheet = Some(Sheet::Chart);
            self.scroll.reset_sheet();
        }
    }

    /// Costs what the plan the chart shows costs. A kept old Seed XOR
    /// part of seed `i` with fingerprint `fp` destroyed: it leaves the
    /// plan and every open vault's copy of it, with its notes. While the
    /// backup flow is open on the wallet's checklist the plan is the
    /// flow's unsaved one: the part leaves it as the flow's own
    /// I destroyed it does, and the flow's save writes it with its notes.
    pub(crate) fn chart_destroyed(&mut self, press: Press, i: usize, fp: [u8; 4]) {
        let c = self.compute();
        let Some(w) = self.press_wallet(press) else {
            return;
        };
        if let Some(b) = self.backup.state()
            && b.wallet == w
            && b.stage == crate::BStage::Checklist
        {
            let shape = self.plan_shape_now(w);
            self.backup
                .change_answers(&shape, &|a, _| a.destroy_kept(i, fp));
            self.backup.old_part_destroyed(i, fp);
            return;
        }
        let Some(v) = self.notes_vault(&c, press, w) else {
            return;
        };
        let Some((mut answers, names)) = crate::wallet_backup::plan_now(&c, self, w) else {
            return;
        };
        let shape = self.plan_shape_now(w);
        let before = answers.clone();
        answers.destroy_kept(i, fp);
        // Each open vault that keeps the wallet's plan, the one the chart
        // is drawn from first, holds it without the part, with the lines
        // it already holds of other vaults' seeds. The device's record
        // follows once a vault holds it, or when no vault keeps the plan.
        let mut vaults = self.plan_vaults_of(&c, w);
        if let Some(at) = vaults.iter().position(|&o| o == v) {
            vaults.swap(0, at);
        }
        let mut written = vaults.is_empty();
        for o in vaults {
            let Some(wallet) = self.session.wallets.get(w) else {
                return;
            };
            let notes = self.plan_notes(&c, wallet, Some(o));
            let notes = crate::plan::xor::notes_after_destroyed(&notes, i, fp);
            let extras = self.vaults.plan_extras_of(&c, o, wallet, &shape, &before);
            written |= self.plan_store(o, w, &answers, &names, &extras, Some(&notes));
        }
        if written {
            self.chart_destroyed_record(w, &answers);
        }
        // The open Back up keeps its unsaved edits, without the part.
        if self.backup.state().is_some_and(|b| b.wallet == w) {
            self.backup
                .change_answers(&shape, &|a, _| a.destroy_kept(i, fp));
        }
    }

    /// The device's own record of wallet `w`'s plan, held without the part
    /// the plan `answers` lost.
    fn chart_destroyed_record(&mut self, w: usize, answers: &crate::plan::Answers) {
        let Some(wallet) = self.session.wallets.get(w) else {
            return;
        };
        let sum = Text::of(wallet.checksum());
        let mut records = self.backups_area.records().to_vec();
        if let Some(r) = records
            .iter_mut()
            .find(|r| r.sum == sum && !r.answers.is_empty())
        {
            r.answers = Text::of(&answers.to_text());
            self.backups_area.set_records(records);
        }
    }

    /// Costs what `described_of` costs. The open vaults that keep a plan
    /// record of wallet `w`.
    fn plan_vaults_of(&self, c: &Compute, w: usize) -> Vec<usize> {
        use faraday_vault::records::kind;
        let Some(wallet) = self.session.wallets.get(w) else {
            return Vec::new();
        };
        let want = wallet.same();
        (0..self.vaults.open.len())
            .filter(|&o| {
                self.vaults
                    .described_of(c, self.vaults.open[o].id, kind::PLAN)
                    .any(|(_, _, d)| d.same == *want)
            })
            .collect()
    }

    /// Costs what `notes_vault` costs. Whether the vault a date would be
    /// written into is a copy that is not the current one: it would live
    /// only there, so nothing is written, and the toast says so.
    /// True too while the wallet's seeds are loading: the press is run
    /// again after.
    fn chart_copy_not_current(&mut self, c: &Compute, press: Press) -> bool {
        let w = match self.press_wallet_wait(press) {
            PressWallet::Ready(w) => Some(w),
            PressWallet::Loading => return true,
            PressWallet::None => None,
        };
        if let Some(w) = w
            && let Some(v) = self.notes_vault(c, press, w)
            && self.vaults.open.get(v).is_some_and(|o| o.is_not_current())
        {
            self.toast(EN.chart_edit_not_current);
            return true;
        }
        false
    }

    /// Every thing in backup node `n`, checked here today.
    pub(crate) fn chart_checked(&mut self, press: Press, n: u8) {
        let c = self.compute();
        let Some(things) = self.chart_things(&c, press, n, None) else {
            return;
        };
        if self.chart_copy_not_current(&c, press) {
            return;
        }
        // A part or share of a set reads Checked only by a match
        // (`docs/decisions/F276.md`): never by this mark.
        let things: Vec<_> = things
            .into_iter()
            .filter(|(_, w)| {
                !matches!(
                    w,
                    What::XorPart(..) | What::XorKept(..) | What::Slip39Share(..)
                )
            })
            .collect();
        let today = self.today();
        let kept = self.chart_notes_change(press, &|notes: &mut Vec<Note>| {
            for &(at, what) in &things {
                notes.retain(|x| !matches!(x, Note::Checked(a, w, _) if *a == at && *w == what));
                notes.push(Note::Checked(at, what, faraday_text::Text::of(&today)));
            }
        });
        if kept {
            // A date written for a place ends its "Older copy" mark.
            let sum = self
                .press_wallet(press)
                .and_then(|w| self.session.wallets.get(w))
                .map(|w| Text::of(w.checksum()));
            if let Some(sum) = sum {
                for &(at, what) in &things {
                    if let At::Place(p) = at {
                        self.backups_area.older_clear(&sum, p, what);
                    }
                }
            }
            self.toast(EN.chart_edit_checked);
        }
    }

    /// Costs what `chart_notes_change` costs. Every coin of the chart's
    /// wallet moved today, in the vault (`docs/BACKUP-FLOW.md` §18.4): one
    /// date per wallet, a later press replacing it. Nothing without a
    /// checked clock (the card then offers Set the time instead).
    pub(crate) fn chart_moved(&mut self, press: Press) {
        let c = self.compute();
        if self.chart_copy_not_current(&c, press) {
            return;
        }
        let today = self.today();
        if today.is_empty() {
            return;
        }
        if self.chart_notes_change(press, &|notes: &mut Vec<Note>| {
            notes.retain(|x| !matches!(x, Note::Moved(_)));
            notes.push(Note::Moved(faraday_text::Text::of(&today)));
        }) {
            self.toast(EN.chart_edit_moved);
        }
    }

    /// Key `k` of the chart, its holder's backup confirmed today.
    pub(crate) fn chart_key_checked(&mut self, press: Press, k: u8) {
        let c = self.compute();
        let Some(fp) = self.chart_key_fp(&c, press, k) else {
            return;
        };
        if self.chart_copy_not_current(&c, press) {
            return;
        }
        let today = self.today();
        if self.chart_notes_change(press, &|notes: &mut Vec<Note>| {
            notes.retain(|x| !matches!(x, Note::KeyChecked(f, _) if *f == fp));
            notes.push(Note::KeyChecked(fp, faraday_text::Text::of(&today)));
        }) {
            self.toast(EN.chart_edit_checked);
        }
    }

    /// The name typed on the open sheet kept: a key's holder, or a
    /// vault's name. An empty name takes the note away.
    pub(crate) fn chart_save_name(&mut self, press: Press) {
        let c = self.compute();
        let Some((_, target)) = self.glance.chart() else {
            return;
        };
        let name = Text::of(self.glance.work().text.trim());
        let done = match target {
            Target::Holder(k) => {
                let Some(fp) = self.chart_key_fp(&c, press, k) else {
                    return;
                };
                self.chart_notes_change(press, &|notes: &mut Vec<Note>| {
                    notes.retain(|x| !matches!(x, Note::Holder(f, _) if *f == fp));
                    if !name.is_empty() {
                        notes.push(Note::Holder(fp, faraday_text::Text::of(&name)));
                    }
                })
            }
            Target::VaultName(n) => {
                let Some(At::Vault(v)) = crate::glance::sheet::node_at_now(&c, self, press, n)
                else {
                    return;
                };
                self.chart_notes_change(press, &|notes: &mut Vec<Note>| {
                    notes.retain(|x| !matches!(x, Note::VaultName(u, _) if *u == v));
                    if !name.is_empty() {
                        notes.push(Note::VaultName(v, faraday_text::Text::of(&name)));
                    }
                })
            }
            _ => false,
        };
        if done {
            self.glance.clear_text();
            self.glance.set_chart(None);
            if self.sheet == Some(Sheet::Chart) {
                self.sheet = None;
            }
        }
    }

    /// The passphrase typed on the open sheet compared, by the key it
    /// makes with the seed held here, with the key loaded: the typed text
    /// is wiped at once, and only the result is kept, as the date the
    /// passphrase line was checked when it matched.
    pub(crate) fn chart_pass_try(&mut self, press: Press) {
        let c = self.compute();
        let Some((_, Target::PassCheck(k, n, j))) = self.glance.chart() else {
            return;
        };
        // The typed passphrase stays until the wallet's seeds are loaded
        // and the press is run again.
        if matches!(self.press_wallet_wait(press), PressWallet::Loading) {
            return;
        }
        let fp = self.chart_key_fp(&c, press, k);
        let matched = fp.and_then(|fp| {
            let key = self.session.keys.position_of(fp)?;
            let made = self
                .session
                .keys
                .fingerprint_with(&c, key, &self.glance.work().secret)?;
            Some(made.0 == fp)
        });
        let matched = matched.unwrap_or(false);
        self.glance.finish_pass_check(matched);
        if matched && let Some(things) = self.chart_things(&c, press, n, Some(j)) {
            let today = self.today();
            self.chart_notes_change(press, &|notes: &mut Vec<Note>| {
                for &(at, what) in &things {
                    notes
                        .retain(|x| !matches!(x, Note::Checked(a, w, _) if *a == at && *w == what));
                    notes.push(Note::Checked(at, what, faraday_text::Text::of(&today)));
                }
            });
        }
    }

    /// Costs what `of_now` and `of_vault_now` cost. What is at line `line`
    /// of backup node `n` on the chart of `press`, or every thing in the
    /// node.
    fn chart_things(
        &self,
        c: &Compute,
        press: Press,
        n: u8,
        line: Option<u8>,
    ) -> Option<Vec<(At, What)>> {
        let g = match press {
            Press::Loaded(w) => glance::of_now(c, self, w),
            Press::Vault(v, r) => glance::of_vault_now(c, self, v, r),
        }?;
        let glance::Backup::Plan { nodes, .. } = &g.backup else {
            return None;
        };
        let node = nodes.get(usize::from(n))?;
        Some(match line {
            Some(j) => vec![(node.at, *node.whats.get(usize::from(j))?)],
            None => node.whats.iter().map(|w| (node.at, *w)).collect(),
        })
    }

    /// Costs what `of_now` and `of_vault_now` cost. The master fingerprint
    /// of key `k` of the chart of `press`.
    fn chart_key_fp(&self, c: &Compute, press: Press, k: u8) -> Option<[u8; 4]> {
        let g = match press {
            Press::Loaded(w) => glance::of_now(c, self, w),
            Press::Vault(v, r) => glance::of_vault_now(c, self, v, r),
        }?;
        g.keys.get(usize::from(k))?.fp
    }

    /// The money of the chart's wallet moves to a new wallet: Create a
    /// wallet, and once it is made, the spend of every coin to it.
    pub(crate) fn chart_move_money(&mut self, press: Press) {
        let Some(w) = self.press_wallet(press) else {
            return;
        };
        let Some(sum) = self.session.wallets.get(w).map(|x| Text::of(x.checksum())) else {
            return;
        };
        self.glance.set_moving(sum.as_str());
        self.glance.set_chart(None);
        self.sheet = None;
        self.act(Action::CreateWallet);
        // The person moves money to a wallet of their choosing: no kind
        // is ticked (owner, 2026-10-24).
        self.create.leave_kind_unchosen();
    }

    /// The loaded wallet whose money is moving to a new wallet, once
    /// that wallet is made.
    pub(crate) fn moving_from(&self) -> Option<usize> {
        self.glance.work().moving_to.as_ref()?;
        let from = self.glance.work().moving.as_ref()?;
        self.session
            .wallets
            .iter()
            .position(|w| w.checksum() == from.as_str())
    }

    /// Typing into the open sheet's field: a name, or a passphrase to be
    /// checked. Returns whether the key was taken.
    pub(crate) fn chart_key(&mut self, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key as KeyIn;
        let Some((press, target)) = self
            .glance
            .chart()
            .filter(|_| self.sheet == Some(Sheet::Chart))
        else {
            return false;
        };
        match target {
            Target::Holder(_) | Target::VaultName(_) => {
                match key {
                    KeyIn::Char(c) if !c.is_control() => self.glance.type_name_char(c),
                    KeyIn::Backspace | KeyIn::Delete => self.glance.erase_name_char(),
                    KeyIn::Enter => {
                        self.act(Action::Chart(glance::sheet::ChartAction::Save(press)))
                    }
                    // While the seeds load, Escape is Cancel and drops them.
                    KeyIn::Escape if self.worker.busy().is_some() => self.act(Action::Cancel),
                    KeyIn::Escape => {
                        self.glance.set_chart(None);
                        self.sheet = None;
                    }
                    _ => {}
                }
                true
            }
            Target::PassCheck(..) => {
                match key {
                    // BIP-39 passphrases here are printable ASCII.
                    KeyIn::Char(c) if (' '..='~').contains(&c) => {
                        self.glance.type_pass_char(c);
                    }
                    // Anything else is refused, and the sheet says so.
                    KeyIn::Char(c) if !c.is_control() => self.glance.refuse_pass_char(),
                    KeyIn::Backspace | KeyIn::Delete => self.glance.erase_pass_char(),
                    KeyIn::Enter => {
                        self.act(Action::Chart(glance::sheet::ChartAction::PassTry(press)))
                    }
                    KeyIn::Escape if self.worker.busy().is_some() => self.act(Action::Cancel),
                    KeyIn::Escape => {
                        self.glance.close_pass_check();
                        self.glance.set_chart(None);
                        self.sheet = None;
                    }
                    _ => {}
                }
                true
            }
            _ => false,
        }
    }

    /// Whether the open sheet is a field being typed in.
    pub(crate) fn chart_typing(&self) -> bool {
        self.sheet == Some(Sheet::Chart)
            && matches!(
                self.glance.chart(),
                Some((
                    _,
                    Target::Holder(_) | Target::VaultName(_) | Target::PassCheck(..)
                ))
            )
    }
}

impl Faraday {
    /// What goes in each place's envelope, as a PDF For the stick: each
    /// place by its number, never by the name the vault keeps for it, and
    /// what it holds.
    pub(crate) fn envelopes_out(&mut self, w: usize) {
        let c = self.compute();
        let Some(g) = glance::of_now(&c, self, w) else {
            return;
        };
        let glance::Backup::Plan { nodes, .. } = &g.backup else {
            return;
        };
        let mut text = text!(
            "faraday-sheet 1\nkind: envelopes\nname: {}\nshape: {}\n",
            crate::wallet::wallet_name(&g.name),
            &*g.shape
        );
        for n in nodes {
            let At::Place(p) = n.at else {
                continue;
            };
            text.push_str(&text!("place: Place {}\n", p + 1));
            for what in &n.whats {
                let label = match what {
                    What::VaultStick(v) => {
                        Text::fill(EN.chart_edit_vault_stick, &[&Text::num(*v as u64 + 1)])
                    }
                    w => glance::thing_label(Some(&g), *w),
                };
                text.push_str(&text!("thing: {}\n", &*label));
            }
        }
        let Ok(pdf) = faraday_paper::pdf::sheet(&c, &text) else {
            return;
        };
        let name = text!("{}-envelopes.pdf", crate::file_stem(&g.name).as_str());
        self.put_outbox(&name, pdf);
        self.toast_out(&name);
    }
}

/// Where the money moving from loaded wallet `w` goes: the new wallet's
/// name and its first receive address, read through `r`
/// ([`glance::Reads`]).
pub(crate) fn moving_to_on(r: &impl glance::Reads, w: usize) -> Option<(Text, Text)> {
    let app = r.app();
    if app.moving_from() != Some(w) {
        return None;
    }
    let to = app.glance.work().moving_to.as_ref()?;
    let nw = app
        .session
        .wallets
        .iter()
        .find(|x| x.checksum() == to.as_str())?;
    Some((
        Text::of(&nw.name),
        Text::of(&crate::held::receive_line_on(r, nw, false, 0).0),
    ))
}
