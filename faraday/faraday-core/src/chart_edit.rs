//! Changing the backup on the wallet's chart (`docs/history/NEW-WALLET.md`
//! §9.5–§9.7): a thing moved, copied, added, destroyed or a place
//! removed, each shown first as the check's three lines before and after
//! and made only once confirmed; from the first such change the map is
//! the plan. Dates checked, marks (lost, exposed), a key's holder and a
//! vault's name are notes the plan's vault record keeps (type 11, field
//! 5), in the vault alone.

use crate::glance::sheet::Target;
use crate::glance::{self, Press};
use crate::plan::{self, At, Edit, Mark, Note, What};
use crate::secret_text::SecretText;
use crate::strings::EN;
use crate::strings::fill;
use crate::{Action, Faraday, Sheet};
use faraday_crypto::work::Compute;

/// What the chart's sheets hold between presses.
#[derive(Default)]
pub struct Work {
    /// The change asked for, waiting to be confirmed.
    pub ask: Option<Ask>,
    /// The name typed: a key's holder, a vault's.
    pub text: String,
    /// A passphrase typed to be checked, wiped once compared.
    pub secret: SecretText,
    /// What the last check of a passphrase found.
    pub result: Option<bool>,
    /// Where each thing of a place being removed goes, by its line: a
    /// place by number, [`OFF`] off the map, `None` not chosen yet.
    pub dest: Vec<Option<u8>>,
    /// A passphrase is shown, to be copied.
    pub reveal: bool,
    /// The wallet whose money is moving to a new wallet, by checksum
    /// (`docs/history/NEW-WALLET.md` §9.6).
    pub moving: Option<String>,
    /// The new wallet it moves to, once Create has made it, by checksum.
    pub moving_to: Option<String>,
}

impl Work {
    /// Create made wallet `i` of `session`: the wallet the money moves
    /// to, while a move is under way.
    pub(crate) fn moved_to(&mut self, session: &crate::wallet::Session, i: usize) {
        if self.moving.is_some() {
            self.moving_to = session.wallets.get(i).map(|w| w.checksum().to_string());
        }
    }
}

/// A destination of [`Work::dest`]: off the map, the copy destroyed.
pub const OFF: u8 = u8::MAX;

/// A change to where things are kept, asked for and not yet made.
pub struct Ask {
    /// The chart it was asked on.
    pub press: Press,
    /// The change.
    pub edit: Edit,
    /// What it does, as the sheet's title says it.
    pub title: String,
    /// The check now.
    pub before: plan::Check,
    /// The check once it is made.
    pub after: plan::Check,
    /// The plan once it is made.
    answers: plan::Answers,
    /// The places' names once it is made.
    names: Vec<String>,
    /// For a place removed: where each thing goes.
    dest: Vec<(What, Option<usize>)>,
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
        let Some(v) = v.or_else(|| self.plan_vault_of(c, wallet)) else {
            return Vec::new();
        };
        let Some(open) = self.vaults.open.get(v) else {
            return Vec::new();
        };
        let want = crate::wallet::same_wallet(wallet.policy());
        self.vaults
            .described_of(c, open.id, kind::PLAN)
            .filter(|(_, _, d)| d.same == want)
            .flat_map(|(_, r, _)| {
                r.public_all(field::PLAN_NOTE)
                    .filter_map(|b| Note::from_text(&String::from_utf8_lossy(b)))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Today's local date, `YYYY-MM-DD`; empty when the clock is not
    /// known (`docs/TOTP.md` §2.3).
    pub(crate) fn today(&self) -> String {
        self.clock_now()
            .map(|t| crate::gpg::date(crate::vault::local_secs(t, self.tz_min())))
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
    /// saved into the current one first. False with no vault open.
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
        let want = crate::wallet::same_wallet(wallet.policy());
        // This wallet's plan records, by place.
        let ours: Vec<usize> = self.vaults.open.get(v).map_or_else(Vec::new, |o| {
            self.vaults
                .described_of(&c, o.id, kind::PLAN)
                .filter(|(_, _, d)| d.same == want)
                .map(|(i, _, _)| i)
                .collect()
        });
        let has = !ours.is_empty();
        if !has {
            // No record of this plan here yet: the plan the chart shows,
            // saved with the notes.
            let Some((answers, names)) = glance::plan_of_now(&c, self, w) else {
                return false;
            };
            self.plan_store(v, w, &answers, &names, &[], Some(&notes));
            return true;
        }
        let Some(id) = self.vaults.open.get(v).map(|o| o.id) else {
            return false;
        };
        let texts: Vec<String> = notes.iter().map(Note::to_text).collect();
        let values: Vec<&[u8]> = texts.iter().map(|t| t.as_bytes()).collect();
        if self
            .vaults
            .edit(id, |s| {
                s.set_public_all_in(id, &ours, field::PLAN_NOTE, &values)
            })
            .is_err()
        {
            self.toast(EN.chart_edit_that_does);
            return false;
        }
        self.vaults.changed(id);
        true
    }

    /// Asks for `edit` on the chart of `press`: the sheet shows the check
    /// before and after, and Confirm makes it.
    pub(crate) fn chart_ask(&mut self, press: Press, edit: Edit) {
        let c = self.compute();
        let Some(w) = self.press_wallet(press) else {
            return;
        };
        let Some((answers, names)) = glance::plan_of_now(&c, self, w) else {
            return;
        };
        let shape = self.plan_shape_now(w);
        let boxes = plan::map(&c, &shape, &answers);
        let dest: Vec<(What, Option<usize>)> = match edit {
            // Each thing the place keeps, its sticks' too, as the chart
            // lists them.
            Edit::RemovePlace(p) => plan::by_place(&boxes)
                .iter()
                .filter(|s| s.at == At::Place(p))
                .flat_map(|s| s.holds.iter().map(|(w, _)| *w))
                .enumerate()
                .map(|(j, what)| {
                    let to = self.glance.work_mut().dest.get(j).copied().flatten();
                    (what, to.filter(|&t| t != OFF).map(usize::from))
                })
                .collect(),
            _ => Vec::new(),
        };
        let places = plan::places_of(&boxes);
        let mut after = answers.clone();
        match edit {
            // A SeedQR drawn in part is the plan's answers' alone, not its
            // map's (`docs/history/NEW-WALLET.md` §14.7 item 3): destroyed, it goes
            // from them, and the plan stays what it was.
            Edit::Drop(at @ At::Place(p), What::SeedQr(i))
                if plan::is_part(&answers, at, What::SeedQr(i))
                    && !plan::holds_at(&boxes, at).contains(&What::SeedQr(i)) =>
            {
                after.part.retain(|&x| x != (p, i));
            }
            _ => {
                if let Edit::RemovePlace(p) = edit {
                    after.part.retain(|&(q, _)| q != p);
                    for (q, _) in &mut after.part {
                        if *q > p {
                            *q -= 1;
                        }
                    }
                }
                after.set_map(plan::edited(&boxes, edit, &dest));
            }
        }
        let mut new_names = names.clone();
        new_names.resize(places, String::new());
        match edit {
            Edit::RemovePlace(p) if p < new_names.len() => {
                new_names.remove(p);
            }
            Edit::Add(At::Place(p), _) | Edit::Move(_, _, At::Place(p)) if p >= places => {
                new_names.push(String::new());
            }
            _ => {}
        }
        let title = self.edit_title(&c, press, w, edit, &names);
        self.glance.work_mut().ask = Some(Ask {
            press,
            edit,
            title,
            before: plan::check(&c, &shape, &answers),
            after: plan::check(&c, &shape, &after),
            answers: after,
            names: new_names,
            dest,
        });
        self.glance.set_chart(Some((press, Target::Confirm)));
        self.sheet = Some(Sheet::Chart);
        self.scroll.reset_sheet();
    }

    /// Costs what `of_now` and `of_vault_now` cost. What an edit does, in
    /// words: "Move Key 1 words to Place 3".
    fn edit_title(
        &self,
        c: &Compute,
        press: Press,
        w: usize,
        edit: Edit,
        names: &[String],
    ) -> String {
        let g = match press {
            Press::Loaded(_) => glance::of_now(c, self, w),
            Press::Vault(v, r) => glance::of_vault_now(c, self, v, r),
        };
        let place = |p: usize| {
            let places = g.as_ref().map_or(0, |g| match &g.backup {
                glance::Backup::Plan { nodes, .. } => nodes
                    .iter()
                    .filter(|n| matches!(n.at, At::Place(_)))
                    .count(),
                _ => 0,
            });
            if p >= places {
                EN.chart_edit_a_new_place.to_string()
            } else {
                names
                    .get(p)
                    .map(|n| n.trim())
                    .filter(|n| !n.is_empty())
                    .map_or_else(
                        || fill(EN.chart_edit_place, &[&(p + 1).to_string()]),
                        str::to_string,
                    )
            }
        };
        let spot = |at: At| match at {
            At::Place(p) => place(p),
            other => g
                .as_ref()
                .and_then(|g| match &g.backup {
                    glance::Backup::Plan { nodes, .. } => {
                        nodes.iter().find(|n| n.at == other).map(|n| n.name.clone())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| match other {
                    At::Vault(v) => fill(EN.chart_edit_vault, &[&(v + 1).to_string()]),
                    At::Files => fill(EN.chart_edit_of_files, &[self.medium.cap()]),
                    _ => EN.chart_edit_watch_only.to_string(),
                }),
        };
        let label = |at: At, what: What| {
            g.as_ref()
                .and_then(|g| glance::label_of(g, at, what))
                .unwrap_or_else(|| what.label(&self.plan_shape_now(w), self.medium))
        };
        match edit {
            Edit::Add(at @ At::Vault(_), what) => fill(
                EN.chart_edit_save_into,
                &[
                    &glance::thing_label(g.as_ref(), what).to_string(),
                    &spot(at).to_string(),
                ],
            ),
            Edit::Add(At::Files, what) => fill(
                EN.chart_edit_as_a_file,
                &[&glance::thing_label(g.as_ref(), what).to_string()],
            ),
            Edit::Add(at, what) => fill(
                EN.chart_edit_place_title,
                &[
                    &glance::thing_label(g.as_ref(), what).to_string(),
                    &spot(at).to_string(),
                ],
            ),
            Edit::Move(from, what, to) => fill(
                EN.chart_edit_move_title,
                &[&label(from, what).to_string(), &spot(to).to_string()],
            ),
            Edit::Drop(at, what) => fill(EN.chart_edit_off_plan, &[&label(at, what).to_string()]),
            Edit::RemovePlace(p) => fill(EN.chart_edit_remove, &[&place(p).to_string()]),
            Edit::Clear(at) => fill(EN.chart_edit_off_plan, &[&spot(at).to_string()]),
        }
    }

    /// Makes the change asked for: the map is the plan from now on, kept
    /// as the backup's record and in the vault that keeps the plan, with
    /// its notes following what moved; then the flow that makes a new
    /// copy, where it makes one.
    pub(crate) fn chart_confirm(&mut self) {
        let c = self.compute();
        let Some(ask) = self.glance.work_mut().ask.take() else {
            return;
        };
        self.glance.set_chart(None);
        if self.sheet == Some(Sheet::Chart) {
            self.sheet = None;
        }
        let Some(w) = self.press_wallet(ask.press) else {
            return;
        };
        let from_vault = matches!(ask.press, Press::Vault(..));
        let screen = self.screen;
        if !self.chart_backup(w, from_vault) {
            return;
        }
        self.screen = screen;
        // A backup past its lock: the change goes through its flow, with
        // its refusals, and is kept only where the flow takes it
        // (`docs/BACKUP-FLOW.md` §15.8).
        if let Some(i) = self.backup_locked_flow() {
            let before = self.flows[i].wallet.as_ref().map(|w| w.answers.clone());
            let new = ask.answers.clone();
            self.flow_wallet_edit(i, &|a, _| *a = new.clone());
            let after = self.flows[i].wallet.as_ref().map(|w| w.answers.clone());
            if after == before {
                return;
            }
        }
        let mut extras = self.backup.chart_edit(&ask.answers, &ask.names);
        self.backups_sync();
        if let Some(v) = self.notes_vault(&c, ask.press, w)
            && let Some(wallet) = self.session.wallets.get(w)
        {
            for line in self.plan_extras(&c, wallet, v) {
                if !extras.contains(&line) {
                    extras.push(line);
                }
            }
            let notes =
                plan::notes_after(&self.plan_notes(&c, wallet, Some(v)), ask.edit, &ask.dest);
            self.plan_store(v, w, &ask.answers, &ask.names, &extras, Some(&notes));
            self.vault_summaries_refresh();
        }
        self.chart_then(ask.press, w, ask.edit);
    }

    /// Costs what `described_of` costs. The lines of work a later plan
    /// dropped that open vault `v`'s record keeps for `wallet` ("vault.ofv
    /// holds seed 9A6A2580").
    fn plan_extras(&self, c: &Compute, wallet: &crate::wallet::Wallet, v: usize) -> Vec<String> {
        use faraday_vault::records::{field, kind};
        let want = crate::wallet::same_wallet(wallet.policy());
        // What stands between the vault and the seed in such a line.
        let holds_seed = EN
            .held_holds_seed
            .trim_start_matches("{}")
            .trim_end_matches("{}");
        let Some(open) = self.vaults.open.get(v) else {
            return Vec::new();
        };
        self.vaults
            .described_of(c, open.id, kind::PLAN)
            .filter(|(_, _, d)| d.same == want)
            .flat_map(|(_, r, _)| {
                r.public_all(field::PLAN_HOLDS)
                    .map(|b| String::from_utf8_lossy(b).into_owned())
                    .filter(|l| l.contains(holds_seed) && !l.contains(": "))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// After an edit is made: the flow that makes what it added, each by
    /// the flows that exist (§9.1): a copy by hand with its check, a vault
    /// item, a file, a sheet, the stick visit for a vault's stick, a
    /// passphrase shown to be copied.
    fn chart_then(&mut self, press: Press, w: usize, edit: Edit) {
        let c = self.compute();
        use crate::glance::sheet::{ChartAction as C, Step};
        let item = |it: plan::Item| {
            Action::Chart(C::Do(Press::Loaded(w), Step::Item(crate::bstep::of(it))))
        };
        let Edit::Add(at, what) = edit else {
            return;
        };
        match (at, what) {
            (At::Place(_), What::Words(i) | What::SeedQr(i)) => self.act(item(plan::Item::Copy(i))),
            (At::Place(_), What::Sheet) => self.public_out(w, 3),
            (At::Place(_), What::Share(_)) => self.public_out(w, 4),
            (At::Place(_), What::VaultStick(_)) => self.act(Action::WriteAsk),
            (At::Place(_), What::Passphrase(i)) => {
                // The new line, and its key: the passphrase shown to copy.
                let g = match press {
                    Press::Loaded(_) => glance::of_now(&c, self, w),
                    Press::Vault(v, r) => glance::of_vault_now(&c, self, v, r),
                };
                let Some(g) = g else {
                    return;
                };
                let k = g.keys.iter().position(|k| k.seed == Some(i));
                let line = match &g.backup {
                    glance::Backup::Plan { nodes, .. } => {
                        nodes.iter().enumerate().find_map(|(n, node)| {
                            let j = (node.at == at)
                                .then(|| node.whats.iter().position(|x| *x == what))
                                .flatten()?;
                            Some((n as u8, j as u8))
                        })
                    }
                    _ => None,
                };
                if let (Some(k), Some((n, j))) = (k, line) {
                    self.glance.work_mut().reveal = false;
                    self.glance
                        .set_chart(Some((press, Target::PassShow(k as u8, n, j))));
                    self.sheet = Some(Sheet::Chart);
                }
            }
            (At::Vault(v), _) => self.act(item(plan::Item::Vault(v))),
            (At::Files, What::SeedFile(_)) => self.act(item(plan::Item::SeedFiles)),
            _ => {}
        }
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

    /// Every thing in backup node `n`, checked here today.
    pub(crate) fn chart_checked(&mut self, press: Press, n: u8) {
        let c = self.compute();
        let Some(things) = self.chart_things(&c, press, n, None) else {
            return;
        };
        let today = self.today();
        let kept = self.chart_notes_change(press, &|notes: &mut Vec<Note>| {
            for &(at, what) in &things {
                notes.retain(|x| !matches!(x, Note::Checked(a, w, _) if *a == at && *w == what));
                notes.push(Note::Checked(at, what, today.clone()));
            }
        });
        if kept {
            self.toast(EN.chart_edit_checked);
        }
    }

    /// Key `k` of the chart, its holder's backup confirmed today.
    pub(crate) fn chart_key_checked(&mut self, press: Press, k: u8) {
        let c = self.compute();
        let Some(fp) = self.chart_key_fp(&c, press, k) else {
            return;
        };
        let today = self.today();
        if self.chart_notes_change(press, &|notes: &mut Vec<Note>| {
            notes.retain(|x| !matches!(x, Note::KeyChecked(f, _) if *f == fp));
            notes.push(Note::KeyChecked(fp, today.clone()));
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
        let name = self.glance.work_mut().text.trim().to_string();
        let done = match target {
            Target::Holder(k) => {
                let Some(fp) = self.chart_key_fp(&c, press, k) else {
                    return;
                };
                self.chart_notes_change(press, &|notes: &mut Vec<Note>| {
                    notes.retain(|x| !matches!(x, Note::Holder(f, _) if *f == fp));
                    if !name.is_empty() {
                        notes.push(Note::Holder(fp, name.clone()));
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
                        notes.push(Note::VaultName(v, name.clone()));
                    }
                })
            }
            _ => false,
        };
        if done {
            self.glance.work_mut().text.clear();
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
        let fp = self.chart_key_fp(&c, press, k);
        let matched = fp.and_then(|fp| {
            let key = self.session.keys.position_of(fp)?;
            let made =
                self.session
                    .keys
                    .fingerprint_with(&c, key, &self.glance.work_mut().secret)?;
            Some(made.0 == fp)
        });
        self.glance.work_mut().secret.clear();
        let matched = matched.unwrap_or(false);
        self.glance.work_mut().result = Some(matched);
        if matched && let Some(things) = self.chart_things(&c, press, n, Some(j)) {
            let today = self.today();
            self.chart_notes_change(press, &|notes: &mut Vec<Note>| {
                for &(at, what) in &things {
                    notes
                        .retain(|x| !matches!(x, Note::Checked(a, w, _) if *a == at && *w == what));
                    notes.push(Note::Checked(at, what, today.clone()));
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
        let Some(sum) = self
            .session
            .wallets
            .get(w)
            .map(|x| x.checksum().to_string())
        else {
            return;
        };
        self.glance.work_mut().moving = Some(sum);
        self.glance.set_chart(None);
        self.sheet = None;
        self.act(Action::CreateWallet);
    }

    /// The loaded wallet whose money is moving to a new wallet, once
    /// that wallet is made.
    pub(crate) fn moving_from(&self) -> Option<usize> {
        self.glance.work().moving_to.as_ref()?;
        let from = self.glance.work().moving.as_ref()?;
        self.session
            .wallets
            .iter()
            .position(|w| w.checksum() == *from)
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
                let t = &mut self.glance.work_mut().text;
                match key {
                    KeyIn::Char(c) if !c.is_control() && t.chars().count() < 48 => t.push(c),
                    KeyIn::Backspace | KeyIn::Delete => {
                        t.pop();
                    }
                    KeyIn::Enter => self.chart_save_name(press),
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
                        self.glance.work_mut().secret.push(c);
                        self.glance.work_mut().result = None;
                    }
                    KeyIn::Backspace | KeyIn::Delete => {
                        self.glance.work_mut().secret.pop();
                        self.glance.work_mut().result = None;
                    }
                    KeyIn::Enter => self.chart_pass_try(press),
                    KeyIn::Escape => {
                        self.glance.work_mut().secret.clear();
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
        let mut text = format!(
            "faraday-sheet 1\nkind: envelopes\nname: {}\nshape: {}\n",
            g.name.replace('\n', " "),
            g.shape
        );
        for n in nodes {
            let At::Place(p) = n.at else {
                continue;
            };
            text.push_str(&format!("place: Place {}\n", p + 1));
            for what in &n.whats {
                let label = match what {
                    What::VaultStick(v) => fill(EN.chart_edit_vault_stick, &[&(v + 1).to_string()]),
                    w => glance::thing_label(Some(&g), *w),
                };
                text.push_str(&format!("thing: {label}\n"));
            }
        }
        let Ok(pdf) = crate::pdf::sheet(&c, &text) else {
            return;
        };
        let name = format!("{}-envelopes.pdf", crate::file_stem(&g.name));
        self.put_outbox(&name, pdf);
        self.toast_out(&name);
    }
}

/// Where the money moving from loaded wallet `w` goes: the new wallet's
/// name and its first receive address, read through `r`
/// ([`glance::Reads`]).
pub(crate) fn moving_to_on(r: &impl glance::Reads, w: usize) -> Option<(String, String)> {
    let app = r.app();
    if app.moving_from() != Some(w) {
        return None;
    }
    let to = app.glance.work().moving_to.as_ref()?;
    let nw = app.session.wallets.iter().find(|x| x.checksum() == *to)?;
    Some((
        nw.name.clone(),
        crate::held::receive_line_on(r, nw, false, 0).0,
    ))
}
