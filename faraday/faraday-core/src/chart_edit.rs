//! Changing the backup on the wallet's chart (`docs/NEW-WALLET.md`
//! §9.5–§9.7): a thing moved, copied, added, destroyed or a place
//! removed, each shown first as the check's three lines before and after
//! and made only once confirmed; from the first such change the map is
//! the plan. Dates checked, marks (lost, exposed), a key's holder and a
//! vault's name are notes the plan's vault record keeps (type 11, field
//! 5), in the vault alone.

use crate::glance::{self, Press};
use crate::glance_sheet::Target;
use crate::plan::{self, At, Edit, Mark, Note, What};
use crate::secret_text::SecretText;
use crate::{Action, Faraday, Sheet};

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
    /// (`docs/NEW-WALLET.md` §9.6).
    pub moving: Option<String>,
    /// The new wallet it moves to, once Create has made it, by checksum.
    pub moving_to: Option<String>,
}

impl Work {
    /// Create made wallet `i` of `session`: the wallet the money moves
    /// to, while a move is under way.
    pub(crate) fn moved_to(&mut self, session: &crate::wallet::Session, i: usize) {
        if self.moving.is_some() {
            self.moving_to = session.wallets.get(i).map(|w| w.policy.checksum());
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
    /// The notes the plan's record keeps for `wallet` (§9.5): in open
    /// vault `v`, or in whichever open vault keeps its plan. None kept,
    /// or no vault open: none.
    pub(crate) fn plan_notes(&self, wallet: &crate::wallet::Wallet, v: Option<usize>) -> Vec<Note> {
        use faraday_vault::records::{field, kind};
        let Some(v) = v.or_else(|| self.plan_vault_of(wallet)) else {
            return Vec::new();
        };
        let Some(open) = self.vaults.open.get(v) else {
            return Vec::new();
        };
        let want = crate::wallet::same_wallet(&wallet.policy);
        open.contents
            .of(kind::PLAN)
            .filter(|(_, r)| {
                r.text(field::PLAN_WALLET)
                    .and_then(|t| crate::wallet::read_wallet(t).ok())
                    .is_some_and(|p| crate::wallet::same_wallet(&p) == want)
            })
            .flat_map(|(_, r)| {
                r.fields
                    .iter()
                    .filter(|f| f.number == field::PLAN_NOTE)
                    .filter_map(|f| Note::from_text(&String::from_utf8_lossy(&f.bytes)))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Today's date on this computer's clock, `YYYY-MM-DD`; empty when the
    /// clock is not known.
    pub(crate) fn today(&self) -> String {
        self.clock_now().map(crate::gpg::date).unwrap_or_default()
    }

    /// The loaded wallet a chart is of: loaded first, with its seeds the
    /// vault keeps, when the chart is an open vault's.
    pub(crate) fn press_wallet(&mut self, press: Press) -> Option<usize> {
        match press {
            Press::Loaded(w) => Some(w).filter(|&w| w < self.session.wallets.len()),
            Press::Vault(v, r) => self.vault_load_wallet(v, r),
        }
    }

    /// The open vault a chart's notes and plan are kept in: the vault
    /// drawing it, else the one keeping the wallet's plan, else the
    /// current one.
    fn notes_vault(&self, press: Press, w: usize) -> Option<usize> {
        if let Press::Vault(v, _) = press {
            return Some(v);
        }
        let wallet = self.session.wallets.get(w)?;
        self.plan_vault_of(wallet).or_else(|| {
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
        use faraday_vault::records::{field, kind};
        let Some(w) = self.press_wallet(press) else {
            return false;
        };
        let Some(v) = self.notes_vault(press, w) else {
            return false;
        };
        let Some(wallet) = self.session.wallets.get(w) else {
            return false;
        };
        let mut notes = self.plan_notes(wallet, Some(v));
        change(&mut notes);
        let want = crate::wallet::same_wallet(&wallet.policy);
        let same = |r: &faraday_vault::records::Record| {
            r.kind == kind::PLAN
                && r.text(field::PLAN_WALLET)
                    .and_then(|t| crate::wallet::read_wallet(t).ok())
                    .is_some_and(|p| crate::wallet::same_wallet(&p) == want)
        };
        let has = self
            .vaults
            .open
            .get(v)
            .is_some_and(|o| o.contents.records.iter().any(same));
        if !has {
            // No record of this plan here yet: the plan the chart shows,
            // saved with the notes.
            let Some((answers, names)) = glance::plan_of(self, w) else {
                return false;
            };
            self.plan_store(v, w, &answers, &names, &[], Some(&notes));
            return true;
        }
        let Some(open) = self.vaults.open.get_mut(v) else {
            return false;
        };
        let mut trial = open.contents.clone();
        for r in trial.records.iter_mut().filter(|r| same(r)) {
            r.fields.retain(|f| f.number != field::PLAN_NOTE);
            for n in &notes {
                r.push(field::PLAN_NOTE, n.to_text().as_bytes());
            }
        }
        if trial.used() > open.header().slot_len as usize {
            self.toast("That does not fit in this vault's slot size");
            return false;
        }
        open.contents = trial;
        open.changes += 1;
        true
    }

    /// Asks for `edit` on the chart of `press`: the sheet shows the check
    /// before and after, and Confirm makes it.
    pub(crate) fn chart_ask(&mut self, press: Press, edit: Edit) {
        let Some(w) = self.press_wallet(press) else {
            return;
        };
        let Some((answers, names)) = glance::plan_of(self, w) else {
            return;
        };
        let shape = self.plan_shape(w);
        let boxes = plan::map(&shape, &answers);
        let dest: Vec<(What, Option<usize>)> = match edit {
            Edit::RemovePlace(p) => boxes
                .iter()
                .filter(|s| s.at == At::Place(p))
                .flat_map(|s| s.holds.iter().map(|(w, _)| *w))
                .enumerate()
                .map(|(j, what)| {
                    let to = self.chart_work.dest.get(j).copied().flatten();
                    (what, to.filter(|&t| t != OFF).map(usize::from))
                })
                .collect(),
            _ => Vec::new(),
        };
        let places = plan::places_of(&boxes);
        let mut after = answers.clone();
        after.set_map(plan::edited(&boxes, edit, &dest));
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
        let title = self.edit_title(press, w, edit, &names);
        self.chart_work.ask = Some(Ask {
            press,
            edit,
            title,
            before: plan::check(&shape, &answers),
            after: plan::check(&shape, &after),
            answers: after,
            names: new_names,
            dest,
        });
        self.chart = Some((press, Target::Confirm));
        self.sheet = Some(Sheet::Chart);
        self.sheet_scroll = (0.0, None);
    }

    /// What an edit does, in words: "Move Key 1 words to Place 3".
    fn edit_title(&self, press: Press, w: usize, edit: Edit, names: &[String]) -> String {
        let g = match press {
            Press::Loaded(_) => glance::of(self, w),
            Press::Vault(v, r) => glance::of_vault(self, v, r),
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
                "a new place".to_string()
            } else {
                names
                    .get(p)
                    .map(|n| n.trim())
                    .filter(|n| !n.is_empty())
                    .map_or_else(|| format!("Place {}", p + 1), str::to_string)
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
                    At::Vault(v) => format!("Vault {}", v + 1),
                    At::Files => format!("{} of files", self.medium.cap()),
                    _ => "Watch-only software".to_string(),
                }),
        };
        let label = |at: At, what: What| {
            g.as_ref()
                .and_then(|g| glance::label_of(g, at, what))
                .unwrap_or_else(|| what.label(&self.plan_shape(w), self.medium))
        };
        match edit {
            Edit::Add(at @ At::Vault(_), what) => {
                format!(
                    "Save {} into {}",
                    glance::thing_label(g.as_ref(), what),
                    spot(at)
                )
            }
            Edit::Add(At::Files, what) => {
                format!("{} as a file", glance::thing_label(g.as_ref(), what))
            }
            Edit::Add(at, what) => {
                format!("{} at {}", glance::thing_label(g.as_ref(), what), spot(at))
            }
            Edit::Move(from, what, to) => format!("Move {} to {}", label(from, what), spot(to)),
            Edit::Drop(at, what) => format!("{} off the plan", label(at, what)),
            Edit::RemovePlace(p) => format!("Remove {}", place(p)),
            Edit::Clear(at) => format!("{} off the plan", spot(at)),
        }
    }

    /// Makes the change asked for: the map is the plan from now on, kept
    /// as the backup's record and in the vault that keeps the plan, with
    /// its notes following what moved; then the flow that makes a new
    /// copy, where it makes one.
    pub(crate) fn chart_confirm(&mut self) {
        let Some(ask) = self.chart_work.ask.take() else {
            return;
        };
        self.chart = None;
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
        let mut extras = Vec::new();
        if let Some(b) = self.backup.as_mut() {
            b.answers = ask.answers.clone();
            b.names = ask.names.clone();
            extras = b.extras.clone();
        }
        self.backups_sync();
        if let Some(v) = self.notes_vault(ask.press, w)
            && let Some(wallet) = self.session.wallets.get(w)
        {
            for line in self.plan_extras(wallet, v) {
                if !extras.contains(&line) {
                    extras.push(line);
                }
            }
            let notes = plan::notes_after(&self.plan_notes(wallet, Some(v)), ask.edit, &ask.dest);
            self.plan_store(v, w, &ask.answers, &ask.names, &extras, Some(&notes));
            self.vault_summaries_refresh();
        }
        self.chart_then(ask.press, w, ask.edit);
    }

    /// The lines of work a later plan dropped that open vault `v`'s record
    /// keeps for `wallet` ("vault.ofv holds seed 9A6A2580").
    fn plan_extras(&self, wallet: &crate::wallet::Wallet, v: usize) -> Vec<String> {
        use faraday_vault::records::{field, kind};
        let want = crate::wallet::same_wallet(&wallet.policy);
        let Some(open) = self.vaults.open.get(v) else {
            return Vec::new();
        };
        open.contents
            .of(kind::PLAN)
            .filter(|(_, r)| {
                r.text(field::PLAN_WALLET)
                    .and_then(|t| crate::wallet::read_wallet(t).ok())
                    .is_some_and(|p| crate::wallet::same_wallet(&p) == want)
            })
            .flat_map(|(_, r)| {
                r.fields
                    .iter()
                    .filter(|f| f.number == field::PLAN_HOLDS)
                    .map(|f| String::from_utf8_lossy(&f.bytes).into_owned())
                    .filter(|l| l.contains(" holds seed ") && !l.contains(": "))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// After an edit is made: the flow that makes what it added, each by
    /// the flows that exist (§9.1): a copy by hand with its check, a vault
    /// item, a file, a sheet, the stick visit for a vault's stick, a
    /// passphrase shown to be copied.
    fn chart_then(&mut self, press: Press, w: usize, edit: Edit) {
        use crate::glance_sheet::{ChartAction as C, Step};
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
                    Press::Loaded(_) => glance::of(self, w),
                    Press::Vault(v, r) => glance::of_vault(self, v, r),
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
                    self.chart_work.reveal = false;
                    self.chart = Some((press, Target::PassShow(k as u8, n, j)));
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
        let Some(things) = self.chart_things(press, n, line) else {
            return;
        };
        let kept = self.chart_notes_change(press, &|notes: &mut Vec<Note>| {
            for &(at, what) in &things {
                notes.retain(|x| !matches!(x, Note::Marked(a, w, _) if *a == at && *w == what));
                notes.push(Note::Marked(at, what, mark));
            }
        });
        if kept {
            self.chart = Some((press, Target::Marked(n, line)));
            self.sheet = Some(Sheet::Chart);
            self.sheet_scroll = (0.0, None);
        }
    }

    /// Every thing in backup node `n`, checked here today.
    pub(crate) fn chart_checked(&mut self, press: Press, n: u8) {
        let Some(things) = self.chart_things(press, n, None) else {
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
            self.toast("Checked today");
        }
    }

    /// Key `k` of the chart, its holder's backup confirmed today.
    pub(crate) fn chart_key_checked(&mut self, press: Press, k: u8) {
        let Some(fp) = self.chart_key_fp(press, k) else {
            return;
        };
        let today = self.today();
        if self.chart_notes_change(press, &|notes: &mut Vec<Note>| {
            notes.retain(|x| !matches!(x, Note::KeyChecked(f, _) if *f == fp));
            notes.push(Note::KeyChecked(fp, today.clone()));
        }) {
            self.toast("Checked today");
        }
    }

    /// The name typed on the open sheet kept: a key's holder, or a
    /// vault's name. An empty name takes the note away.
    pub(crate) fn chart_save_name(&mut self, press: Press) {
        let Some((_, target)) = self.chart else {
            return;
        };
        let name = self.chart_work.text.trim().to_string();
        let done = match target {
            Target::Holder(k) => {
                let Some(fp) = self.chart_key_fp(press, k) else {
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
                let Some(At::Vault(v)) = crate::glance_sheet::node_at(self, press, n) else {
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
            self.chart_work.text.clear();
            self.chart = None;
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
        let Some((_, Target::PassCheck(k, n, j))) = self.chart else {
            return;
        };
        let fp = self.chart_key_fp(press, k);
        let matched = fp.and_then(|fp| {
            let key = self
                .session
                .keys
                .iter()
                .find(|key| key.master.fingerprint().0 == fp)?;
            let words = key.words.as_ref()?;
            let m = osk_bip::bip39::Mnemonic::parse(key.language, words).ok()?;
            let seed = m.to_seed(self.chart_work.secret.as_bytes()).ok()?;
            let made = osk_bip::keys::MasterKey::from_seed(&seed, self.session.network());
            Some(made.fingerprint().0 == fp)
        });
        self.chart_work.secret.clear();
        let matched = matched.unwrap_or(false);
        self.chart_work.result = Some(matched);
        if matched && let Some(things) = self.chart_things(press, n, Some(j)) {
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

    /// What is at line `line` of backup node `n` on the chart of `press`,
    /// or every thing in the node.
    fn chart_things(&self, press: Press, n: u8, line: Option<u8>) -> Option<Vec<(At, What)>> {
        let g = match press {
            Press::Loaded(w) => glance::of(self, w),
            Press::Vault(v, r) => glance::of_vault(self, v, r),
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

    /// The master fingerprint of key `k` of the chart of `press`.
    fn chart_key_fp(&self, press: Press, k: u8) -> Option<[u8; 4]> {
        let g = match press {
            Press::Loaded(w) => glance::of(self, w),
            Press::Vault(v, r) => glance::of_vault(self, v, r),
        }?;
        g.keys.get(usize::from(k))?.fp
    }

    /// The money of the chart's wallet moves to a new wallet: Create a
    /// wallet, and once it is made, the spend of every coin to it.
    pub(crate) fn chart_move_money(&mut self, press: Press) {
        let Some(w) = self.press_wallet(press) else {
            return;
        };
        let Some(sum) = self.session.wallets.get(w).map(|x| x.policy.checksum()) else {
            return;
        };
        self.chart_work.moving = Some(sum);
        self.chart = None;
        self.sheet = None;
        self.act(Action::CreateWallet);
    }

    /// The loaded wallet whose money is moving to a new wallet, once
    /// that wallet is made.
    pub(crate) fn moving_from(&self) -> Option<usize> {
        self.chart_work.moving_to.as_ref()?;
        let from = self.chart_work.moving.as_ref()?;
        self.session
            .wallets
            .iter()
            .position(|w| w.policy.checksum() == *from)
    }

    /// Where the money moving from loaded wallet `w` goes: the new
    /// wallet's name and its first receive address.
    pub(crate) fn moving_to(&self, w: usize) -> Option<(String, String)> {
        if self.moving_from() != Some(w) {
            return None;
        }
        let to = self.chart_work.moving_to.as_ref()?;
        let nw = self
            .session
            .wallets
            .iter()
            .find(|x| x.policy.checksum() == *to)?;
        Some((nw.name.clone(), self.session.address_shown(nw, false, 0)))
    }

    /// Typing into the open sheet's field: a name, or a passphrase to be
    /// checked. Returns whether the key was taken.
    pub(crate) fn chart_key(&mut self, key: osk_shell_api::Key) -> bool {
        use osk_shell_api::Key as KeyIn;
        let Some((press, target)) = self.chart.filter(|_| self.sheet == Some(Sheet::Chart)) else {
            return false;
        };
        match target {
            Target::Holder(_) | Target::VaultName(_) => {
                let t = &mut self.chart_work.text;
                match key {
                    KeyIn::Char(c) if !c.is_control() && t.chars().count() < 48 => t.push(c),
                    KeyIn::Backspace | KeyIn::Delete => {
                        t.pop();
                    }
                    KeyIn::Enter => self.chart_save_name(press),
                    KeyIn::Escape => {
                        self.chart = None;
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
                        self.chart_work.secret.push(c);
                        self.chart_work.result = None;
                    }
                    KeyIn::Backspace | KeyIn::Delete => {
                        self.chart_work.secret.pop();
                        self.chart_work.result = None;
                    }
                    KeyIn::Enter => self.chart_pass_try(press),
                    KeyIn::Escape => {
                        self.chart_work.secret.clear();
                        self.chart = None;
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
                self.chart,
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
        let Some(g) = glance::of(self, w) else {
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
                    What::VaultStick(v) => format!("Vault {} stick", v + 1),
                    w => glance::thing_label(Some(&g), *w),
                };
                text.push_str(&format!("thing: {label}\n"));
            }
        }
        let Ok(pdf) = crate::pdf::sheet(&text) else {
            return;
        };
        let name = format!("{}-envelopes.pdf", crate::file_stem(&g.name));
        self.put_outbox(&name, pdf);
        self.toast_out(&name);
    }
}
