//! The Load sheet an unlock shows (`docs/DECISIONS.md` F62): after an
//! unlock started from the boot sheet or the Vaults tab, **Load from
//! {vault}** lists a card per wallet the vault holds, with the seeds of it
//! the vault keeps, and a Keys card for the keys in none of its wallets.
//! Each card starts ticked from its record's Load at unlock; ticks changed
//! here are for this unlock only. **Load ticked** loads what is ticked and
//! not loaded already; **Not now** loads nothing and leaves the vault
//! open. An unlock a flow asked for returns to its flow and shows none.

use std::collections::BTreeSet;
use std::rc::Rc;

use faraday_vault::records::{self, field, kind};
use opensigner_core::strings::fill;
use osk_bip::keys::Fingerprint;

use crate::strings::EN;
use crate::vaults::VaultAction;
use crate::wallet::{fp_text, key_line, same_wallet};
use crate::{Faraday, Screen, Sheet};

/// The sheet's state while it is up.
#[derive(Debug, Clone)]
pub struct LoadSheet {
    /// The open vault it is for, by its file name.
    pub name: String,
    /// Unlocked on the boot sheet: the import goes on after it.
    pub boot: bool,
    /// How many wallets the session held before: the landing after it
    /// counts what this unlock, or the import, brought.
    pub wallets_before: usize,
    /// The wallet records and Keys-card key records ticked.
    pub ticked: BTreeSet<usize>,
}

/// A seed of a wallet card, or a key of the Keys card.
#[derive(Debug, Clone)]
pub struct LoadSeed {
    /// Its key record.
    pub record: usize,
    /// Its fingerprint and label, as a session key's line reads.
    pub line: String,
    /// The other wallets of the vault it is in, by name.
    pub also: Vec<String>,
    /// What the sheet says of them: "also in Spending", "also in
    /// Spending and Taproot", "also in 3 other wallets".
    pub also_line: Option<String>,
    /// The session has it.
    pub loaded: bool,
    /// On the Keys card: ticked to load.
    pub ticked: bool,
}

/// A wallet's card.
#[derive(Debug, Clone)]
pub struct LoadCard {
    /// Its wallet record.
    pub record: usize,
    /// Its name.
    pub name: String,
    /// Its kind, as the wallet's card says it ("2-of-3 · native SegWit").
    pub kind: String,
    /// The seeds of it the vault keeps.
    pub seeds: Vec<LoadSeed>,
    /// "2 of 3 seeds kept elsewhere", or "Watch-only here".
    pub elsewhere: Option<String>,
    /// The session has the wallet.
    pub loaded: bool,
    /// The wallet and every seed on the card are loaded: no checkbox.
    pub done: bool,
    /// Ticked to load.
    pub ticked: bool,
}

/// A seed an open vault keeps without a passphrase, as the sheet of
/// such seeds lists it (F62).
#[derive(Debug, Clone)]
pub struct PassSeed {
    /// Its open vault.
    pub vault: usize,
    /// Its key record.
    pub record: usize,
    /// The vault's name.
    pub vault_name: String,
    /// Its fingerprint and label.
    pub line: String,
}

/// The sheet of seeds kept with no passphrase, opened from a wallet
/// short of a key (F62).
#[derive(Debug, Clone)]
pub struct PassSeeds {
    /// The wallet that asked, by its place in the session.
    pub wallet: usize,
    /// The fingerprints of its keys not here when it asked.
    pub missing: Vec<[u8; 4]>,
    /// The seed whose passphrase is being typed: open vault and record.
    pub picked: Option<(usize, usize)>,
}

/// What the sheet shows.
#[derive(Debug, Clone, Default)]
pub struct LoadView {
    /// "Load from {vault}".
    pub title: String,
    /// "2 wallets · 1 key".
    pub count: String,
    /// One card per wallet, in the vault's order.
    pub cards: Vec<LoadCard>,
    /// The Keys card: keys in no wallet of the vault.
    pub keys: Vec<LoadSeed>,
}

impl LoadView {
    /// Something is ticked: Load ticked is enabled, and the line above
    /// the cards reads Unselect all.
    pub fn any_ticked(&self) -> bool {
        self.cards.iter().any(|c| c.ticked) || self.keys.iter().any(|k| k.ticked)
    }

    /// Something can be ticked: not everything is loaded.
    pub fn tickable(&self) -> bool {
        self.cards.iter().any(|c| !c.done) || self.keys.iter().any(|k| !k.loaded)
    }

    /// The records a tick can name, each once.
    fn tickable_records(&self) -> Vec<usize> {
        self.cards
            .iter()
            .filter(|c| !c.done)
            .map(|c| c.record)
            .chain(self.keys.iter().filter(|k| !k.loaded).map(|k| k.record))
            .collect()
    }

    /// The records Load ticked loads: each ticked card's wallet and seeds,
    /// each ticked key, only what the session does not hold, each once.
    fn to_load(&self) -> BTreeSet<usize> {
        let mut set = BTreeSet::new();
        for c in self.cards.iter().filter(|c| c.ticked) {
            if !c.loaded {
                set.insert(c.record);
            }
            set.extend(c.seeds.iter().filter(|s| !s.loaded).map(|s| s.record));
        }
        set.extend(
            self.keys
                .iter()
                .filter(|k| k.ticked && !k.loaded)
                .map(|k| k.record),
        );
        set
    }
}

/// A fingerprint written as eight hex digits.
fn fp_of(text: &str) -> Option<Fingerprint> {
    u32::from_str_radix(text, 16)
        .ok()
        .map(|n| Fingerprint(n.to_be_bytes()))
}

impl Faraday {
    /// The first wallet of open vault `v` that key record `r` is a seed
    /// of, by name: such a key loads with it and has no Load at unlock of
    /// its own (F62). Costs a parse of each wallet record of the vault.
    pub(crate) fn vault_key_wallet(&self, v: usize, r: usize) -> Option<String> {
        let open = self.vaults.open.get(v)?;
        let rec = open
            .contents
            .records
            .get(r)
            .filter(|x| x.kind == kind::KEY)?;
        let fp = crate::vault_screens::key_fingerprint(self, rec)?;
        open.contents.of(kind::WALLET).find_map(|(_, w)| {
            let p = crate::wallet::read_wallet(w.text(field::WALLET)?).ok()?;
            p.keys()
                .iter()
                .any(|k| k.fingerprint().map(fp_text).as_deref() == Some(fp.as_str()))
                .then(|| {
                    w.text(field::WALLET_NAME)
                        .unwrap_or(EN.vaults_wallet)
                        .to_string()
                })
        })
    }

    /// The seeds open vaults keep as words without a BIP-39 passphrase,
    /// not loaded, in no wallet of their vault: ones a wallet short of a
    /// key may use with a passphrase never stored (F62), each with its
    /// vault and line. Kept in the drawing memo while a frame is drawn
    /// (`docs/SPEED.md` §3.3); a press works it out afresh. Costs a parse
    /// of each open vault's wallet records.
    pub fn vault_passphrase_seeds(&self) -> Rc<Vec<PassSeed>> {
        if self.drawing.is_on() {
            return self
                .drawing
                .passphrase_seeds
                .get_or_init(|| Rc::new(self.vault_passphrase_seeds_now()))
                .clone();
        }
        Rc::new(self.vault_passphrase_seeds_now())
    }

    /// Works out [`Faraday::vault_passphrase_seeds`]. Costs a parse of
    /// each open vault's wallet records.
    fn vault_passphrase_seeds_now(&self) -> Vec<PassSeed> {
        let mut out = Vec::new();
        for (v, open) in self.vaults.open.iter().enumerate() {
            for (i, r) in open.contents.of(kind::KEY) {
                if r.field(field::KEY_PASSPHRASE).is_some() || records::words_of(r).is_none() {
                    continue;
                }
                let Some(fp) = crate::vault_screens::key_fingerprint(self, r) else {
                    continue;
                };
                let loaded = self
                    .session
                    .keys
                    .iter()
                    .any(|k| fp_text(k.fingerprint()) == fp);
                if loaded || self.vault_key_wallet(v, i).is_some() {
                    continue;
                }
                let label = r.text(field::KEY_LABEL).unwrap_or("");
                out.push(PassSeed {
                    vault: v,
                    record: i,
                    vault_name: open.label(),
                    line: fp_of(&fp).map_or_else(|| fp.clone(), |f| key_line(f, label)),
                });
            }
        }
        out
    }

    /// **Load with a passphrase** from wallet `w` short of a key (F62):
    /// the sheet of the seeds kept with no passphrase, or, with one, its
    /// passphrase field at once.
    pub(crate) fn pass_seeds_open(&mut self, w: usize) {
        let seeds = self.vault_passphrase_seeds();
        let Some(wallet) = self.session.wallets.get(w) else {
            return;
        };
        if seeds.is_empty() {
            return;
        }
        let missing: Vec<[u8; 4]> = self
            .session
            .slots(wallet)
            .iter()
            .filter(|s| s.held_by.is_none())
            .filter_map(|s| s.fingerprint.map(|f| f.0))
            .collect();
        self.vaults.pass_seeds = Some(PassSeeds {
            wallet: w,
            missing,
            picked: None,
        });
        self.vaults.prompt = None;
        self.sheet = Some(Sheet::PassphraseSeeds);
        if let [one] = seeds.as_slice() {
            self.pass_seed_pick(one.vault, one.record);
        }
    }

    /// Seed `r` of open vault `v` picked on the sheet: its passphrase
    /// field.
    pub(crate) fn pass_seed_pick(&mut self, v: usize, r: usize) {
        let Some(p) = self.vaults.pass_seeds.as_mut() else {
            return;
        };
        p.picked = Some((v, r));
        self.vaults.prompt = Some(crate::vaults::Prompt {
            purpose: crate::vaults::Purpose::ForWallet(v, r),
            text: Default::default(),
            error: None,
        });
        self.vaults.focus = self.may_load_keys().then_some(crate::vaults::Focus::Prompt);
    }

    /// Back from a seed's field to the list.
    pub(crate) fn pass_seed_list(&mut self) {
        if let Some(p) = self.vaults.pass_seeds.as_mut() {
            p.picked = None;
        }
        self.vaults.prompt = None;
        self.vaults.focus = None;
    }

    /// The sheet closed: nothing typed is kept.
    pub(crate) fn pass_seeds_close(&mut self) {
        self.vaults.pass_seeds = None;
        self.vaults.prompt = None;
        self.vaults.focus = None;
        if self.sheet == Some(Sheet::PassphraseSeeds) {
            self.sheet = None;
        }
    }

    /// The passphrase typed for seed `r` of open vault `v`: the key it
    /// gives loads only when it is one the wallet is missing here; else
    /// nothing loads and the field says what key it gave. The passphrase
    /// is never stored. Costs a BIP-39 seed and a master key.
    pub(crate) fn pass_seed_try(&mut self, v: usize, r: usize) {
        if !self.may_load_keys() {
            return;
        }
        let Some(p) = self.vaults.pass_seeds.as_ref() else {
            return;
        };
        let (w, missing) = (p.wallet, p.missing.clone());
        let Some(rec) = self
            .vaults
            .open
            .get(v)
            .and_then(|o| o.contents.records.get(r))
        else {
            return;
        };
        let Some(m) = records::words_of(rec) else {
            return;
        };
        let label = rec.text(field::KEY_LABEL).unwrap_or("").to_string();
        let passphrase = zeroize::Zeroizing::new(
            self.vaults
                .prompt
                .as_ref()
                .map(|p| p.text.text.to_string())
                .unwrap_or_default(),
        );
        let Ok(seed) = m.to_seed(passphrase.as_bytes()) else {
            return;
        };
        let fp = osk_bip::keys::MasterKey::from_seed(&seed, self.session.network()).fingerprint();
        if !missing.contains(&fp.0) {
            let wallet = self
                .session
                .wallets
                .get(w)
                .map_or_else(String::new, |x| x.name.clone());
            if let Some(p) = self.vaults.prompt.as_mut() {
                p.text.clear();
                p.error = Some(fill(EN.pass_seeds_not_a_key, &[&fp_text(fp), &wallet]));
            }
            return;
        }
        let words = crate::vaults::phrase_of(&m);
        match self
            .session
            .add_words_with(&words, &passphrase, &label, None)
        {
            Ok(fp) => {
                self.pass_seeds_close();
                self.refresh_spend();
                self.toast(&fill(EN.vaults_key_is_loaded, &[&fp_text(fp)]));
            }
            Err(e) => {
                if let Some(p) = self.vaults.prompt.as_mut() {
                    p.error = Some(e.text());
                }
            }
        }
    }

    /// The open vault the sheet is for, by its place.
    fn load_sheet_vault(&self) -> Option<usize> {
        let s = self.vaults.load.as_ref()?;
        self.vaults.open.iter().position(|o| o.name == s.name)
    }

    /// What the sheet shows, kept in the drawing memo while a frame is
    /// drawn (`docs/SPEED.md` §3.3). Costs a parse of each wallet record
    /// of the vault; the key fingerprints are the ones the unlock worked
    /// out.
    pub fn vault_load_view(&self) -> Option<Rc<LoadView>> {
        let s = self.vaults.load.as_ref()?;
        let v = self.load_sheet_vault()?;
        if self.drawing.is_on() {
            return Some(
                self.drawing
                    .load_sheet
                    .get_or_init(|| Rc::new(self.load_view_of(v, &s.ticked)))
                    .clone(),
            );
        }
        Some(Rc::new(self.load_view_of(v, &s.ticked)))
    }

    /// Works out the sheet for open vault `v` with `ticked`. Costs a parse
    /// of each wallet record of the vault.
    fn load_view_of(&self, v: usize, ticked: &BTreeSet<usize>) -> LoadView {
        let Some(open) = self.vaults.open.get(v) else {
            return LoadView::default();
        };
        let held = |fp: &str| {
            self.session
                .keys
                .iter()
                .any(|k| fp_text(k.fingerprint()) == fp)
        };
        // The vault's keys: record, fingerprint, label.
        let keys: Vec<(usize, Option<String>, String)> = open
            .contents
            .of(kind::KEY)
            .map(|(i, r)| {
                (
                    i,
                    crate::vault_screens::key_fingerprint(self, r),
                    r.text(field::KEY_LABEL).unwrap_or("").to_string(),
                )
            })
            .collect();
        // Each wallet: record, name, kind, its keys' fingerprints (None
        // for a key with no origin), whether the session has it.
        struct W {
            record: usize,
            name: String,
            kind: String,
            fps: Vec<Option<String>>,
            loaded: bool,
        }
        let wallets: Vec<W> = open
            .contents
            .of(kind::WALLET)
            .map(|(i, r)| {
                let name = r
                    .text(field::WALLET_NAME)
                    .unwrap_or(EN.vaults_wallet)
                    .to_string();
                let policy = crate::wallet::read_wallet(r.text(field::WALLET).unwrap_or("")).ok();
                let fps = policy.as_ref().map_or_else(Vec::new, |p| {
                    p.keys()
                        .iter()
                        .map(|k| k.fingerprint().map(fp_text))
                        .collect()
                });
                let loaded = policy.as_ref().is_some_and(|p| {
                    let want = same_wallet(p);
                    self.session
                        .wallets
                        .iter()
                        .any(|w| same_wallet(w.policy()) == want)
                });
                W {
                    record: i,
                    name,
                    kind: crate::vault_screens::wallet_shape(r).unwrap_or_default(),
                    fps,
                    loaded,
                }
            })
            .collect();
        let in_wallet = |fp: &Option<String>, w: &W| fp.is_some() && w.fps.contains(fp);
        let seed = |(i, fp, label): &(usize, Option<String>, String), also: Vec<String>| {
            let line = match fp.as_deref().and_then(fp_of) {
                Some(f) => key_line(f, label),
                None if label.is_empty() => EN.vaults_seed.to_string(),
                None => label.clone(),
            };
            let loaded = fp.as_deref().is_some_and(held);
            let also_line = match also.as_slice() {
                [] => None,
                [one] => Some(fill(EN.load_sheet_also_in, &[one])),
                [one, two] => Some(fill(EN.load_sheet_also_in_two, &[one, two])),
                more => Some(fill(EN.load_sheet_also_in_n, &[&more.len().to_string()])),
            };
            LoadSeed {
                record: *i,
                line,
                also,
                also_line,
                loaded,
                ticked: false,
            }
        };
        let mut view = LoadView {
            title: fill(EN.load_sheet_title, &[&open.label()]),
            ..LoadView::default()
        };
        for w in &wallets {
            let seeds: Vec<LoadSeed> = keys
                .iter()
                .filter(|k| in_wallet(&k.1, w))
                .map(|k| {
                    let also = wallets
                        .iter()
                        .filter(|o| o.record != w.record && in_wallet(&k.1, o))
                        .map(|o| o.name.clone())
                        .collect();
                    seed(k, also)
                })
                .collect();
            let total = w.fps.len();
            let here = w
                .fps
                .iter()
                .filter(|f| f.is_some() && keys.iter().any(|k| &k.1 == *f))
                .count();
            let elsewhere = if total == 0 {
                None
            } else if here == 0 {
                Some(EN.load_sheet_watch_only.to_string())
            } else if here < total {
                Some(fill(
                    EN.load_sheet_kept_elsewhere,
                    &[&(total - here).to_string(), &total.to_string()],
                ))
            } else {
                None
            };
            let done = w.loaded && seeds.iter().all(|s| s.loaded);
            view.cards.push(LoadCard {
                record: w.record,
                name: w.name.clone(),
                kind: w.kind.clone(),
                elsewhere,
                loaded: w.loaded,
                done,
                ticked: !done && ticked.contains(&w.record),
                seeds,
            });
        }
        for k in keys
            .iter()
            .filter(|k| !wallets.iter().any(|w| in_wallet(&k.1, w)))
        {
            let mut s = seed(k, Vec::new());
            s.ticked = !s.loaded && ticked.contains(&s.record);
            view.keys.push(s);
        }
        let n =
            |k: usize, one: &str, many: &str| format!("{k} {}", if k == 1 { one } else { many });
        let parts: Vec<String> = [
            (
                view.cards.len(),
                EN.load_sheet_wallet,
                EN.load_sheet_wallets,
            ),
            (keys.len(), EN.load_sheet_key, EN.load_sheet_keys),
        ]
        .into_iter()
        .filter(|(k, ..)| *k > 0)
        .map(|(k, one, many)| n(k, one, many))
        .collect();
        view.count = parts.join(" · ");
        view
    }

    /// Open vault `v` was just unlocked from the boot sheet (`boot`) or
    /// the Vaults tab: its Load sheet comes up, its cards ticked from
    /// each record's Load at unlock. Returns false, showing nothing, when
    /// the vault holds no wallet and no key, or the session holds all of
    /// them already. Costs a parse of each wallet record.
    pub(crate) fn load_sheet_offer(&mut self, v: usize, boot: bool, wallets_before: usize) -> bool {
        let Some(open) = self.vaults.open.get(v) else {
            return false;
        };
        let records: BTreeSet<usize> = open
            .contents
            .records
            .iter()
            .enumerate()
            .filter(|(_, r)| matches!(r.kind, kind::KEY | kind::WALLET))
            .map(|(i, _)| i)
            .collect();
        if records.is_empty() {
            return false;
        }
        // Every record that would load at unlock; the view leaves out a
        // key on a wallet's card, which has no checkbox of its own, and
        // anything loaded already.
        let ticked: BTreeSet<usize> = records
            .into_iter()
            .filter(|&i| records::loads_at_unlock(&open.contents.records[i]))
            .collect();
        // Everything it holds is in the session already: nothing to ask.
        if !self.load_view_of(v, &ticked).tickable() {
            return false;
        }
        let Some(open) = self.vaults.open.get(v) else {
            return false;
        };
        self.vaults.load = Some(LoadSheet {
            name: open.name.clone(),
            boot,
            wallets_before,
            ticked,
        });
        self.sheet = Some(Sheet::UnlockLoad);
        true
    }

    /// Runs a press on the Load sheet.
    pub(crate) fn load_sheet_act(&mut self, a: VaultAction) {
        let Some(v) = self.load_sheet_vault() else {
            // Its vault is gone (locked): nothing is left to load.
            if self.sheet == Some(Sheet::UnlockLoad) {
                self.sheet = None;
            }
            self.vaults.load = None;
            return;
        };
        let Some(ticked) = self.vaults.load.as_ref().map(|s| s.ticked.clone()) else {
            return;
        };
        let view = self.load_view_of(v, &ticked);
        match a {
            VaultAction::LoadTick(r) => {
                if !view.tickable_records().contains(&r) {
                    return;
                }
                if let Some(s) = self.vaults.load.as_mut()
                    && !s.ticked.remove(&r)
                {
                    s.ticked.insert(r);
                }
            }
            VaultAction::LoadTickAll => {
                let all = view.tickable_records();
                if let Some(s) = self.vaults.load.as_mut() {
                    if view.any_ticked() {
                        for r in all {
                            s.ticked.remove(&r);
                        }
                    } else {
                        s.ticked.extend(all);
                    }
                }
            }
            VaultAction::LoadTicked => {
                if !view.any_ticked() || !self.may_load_keys() {
                    return;
                }
                let set = view.to_load();
                let (keys, wallets) = self.vault_load_set(v, &set);
                if wallets == 0 && keys > 0 {
                    self.toast(&fill(
                        EN.vaults_loaded,
                        &[
                            &keys.to_string(),
                            (if keys == 1 {
                                EN.vaults_seed_inline
                            } else {
                                EN.vaults_seeds
                            }),
                        ],
                    ));
                }
                self.load_sheet_close(v);
            }
            VaultAction::LoadNotNow => self.load_sheet_close(v),
            _ => {}
        }
    }

    /// The sheet is done with: the boot import goes on to its next vault
    /// or lands, as F60 has it; an unlock from the Vaults tab lands on
    /// Wallets when wallets loaded, else stays on Files.
    fn load_sheet_close(&mut self, v: usize) {
        let Some(s) = self.vaults.load.take() else {
            return;
        };
        if self.sheet == Some(Sheet::UnlockLoad) {
            self.sheet = None;
        }
        let file = self.vaults.open[v].name.clone();
        if s.boot {
            let label = self.import.as_ref().map(|i| i.label.clone());
            if self.import_after_unlock() {
                self.unlock_landed(s.wallets_before, &file, label.as_deref());
            }
        } else {
            if self.screen == Screen::Unlock {
                self.screen = Screen::Files;
            }
            self.unlock_landed(s.wallets_before, &file, None);
        }
    }
}
