//! The Load sheet an unlock shows (`docs/DECISIONS.md` F62): after an
//! unlock started from the boot sheet or the Vaults tab, **Load from
//! {vault}** lists a card per wallet the vault holds, with the seeds of it
//! the vault keeps, and a Keys card for the keys in none of its wallets.
//! Each card starts ticked from its record's Load at unlock; ticks changed
//! here are for this unlock only. **Load ticked** loads what is ticked and
//! not loaded already; **Not now** loads nothing and leaves the vault
//! open. An unlock a flow asked for returns to its flow and shows none.
//!
//! What the sheet shows is worked out on [`super::Vaults`] here; its
//! presses are the vault's glue, in `vault/mod.rs`.

use faraday_crypto::work::Compute;
use faraday_vault::records::{field, kind};

use crate::strings::{EN, fill};
use crate::wallet::{Session, fp_text, key_line, same_wallet};
use std::collections::BTreeSet;

use faraday_bip::keys::Fingerprint;

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
    pub(crate) fn tickable_records(&self) -> Vec<usize> {
        self.cards
            .iter()
            .filter(|c| !c.done)
            .map(|c| c.record)
            .chain(self.keys.iter().filter(|k| !k.loaded).map(|k| k.record))
            .collect()
    }

    /// The records Load ticked loads: each ticked card's wallet and seeds,
    /// each ticked key, only what the session does not hold, each once.
    pub(crate) fn to_load(&self) -> BTreeSet<usize> {
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
pub(crate) fn fp_of(text: &str) -> Option<Fingerprint> {
    u32::from_str_radix(text, 16)
        .ok()
        .map(|n| Fingerprint(n.to_be_bytes()))
}

impl super::Vaults {
    /// Costs what `key_fingerprint` and `described_of` cost. The first
    /// wallet of open vault `v` that key record `r` is a seed of, by name:
    /// such a key loads with it and has no Load at unlock of its own (F62).
    /// Reads the wallet records as the vault keeps them read
    /// ([`crate::vault::Vaults::described`]).
    pub(crate) fn key_wallet(
        &self,
        c: &Compute,
        network: faraday_bip::keys::Network,
        v: usize,
        r: usize,
    ) -> Option<String> {
        let open = self.open.get(v)?;
        let rec = self
            .records(open.id)
            .get(r)
            .filter(|x| x.kind() == kind::KEY)?;
        let fp = self.key_fingerprint(c, network, rec)?;
        self.described_of(c, open.id, kind::WALLET)
            .find_map(|(_, w, d)| {
                let p = &d.policy;
                p.keys()
                    .iter()
                    .any(|k| k.fingerprint().map(fp_text).as_deref() == Some(fp.as_str()))
                    .then(|| {
                        w.public_text(field::WALLET_NAME)
                            .unwrap_or(EN.vaults_wallet)
                            .to_string()
                    })
            })
    }

    /// Costs what `word_count`, `key_fingerprint` and `vault_key_wallet`
    /// cost. Works out [`Faraday::vault_passphrase_seeds_now`].
    pub(crate) fn passphrase_seeds(&self, c: &Compute, session: &Session) -> Vec<PassSeed> {
        let mut out = Vec::new();
        for (v, open) in self.open.iter().enumerate() {
            for (i, r) in self.records(open.id).of(kind::KEY) {
                // Words, as the vault keeps them read: no parse here.
                if r.has(field::KEY_PASSPHRASE) || self.word_count(c, open.id, i).is_none() {
                    continue;
                }
                let Some(fp) = self.key_fingerprint(c, session.network(), r) else {
                    continue;
                };
                let loaded = session.keys.all().any(|k| fp_text(k.fingerprint) == fp);
                if loaded || self.key_wallet(c, session.network(), v, i).is_some() {
                    continue;
                }
                let label = r.public_text(field::KEY_LABEL).unwrap_or("");
                out.push(PassSeed {
                    vault: v,
                    record: i,
                    vault_name: open.label(self.store()),
                    line: fp_of(&fp).map_or_else(|| fp.clone(), |f| key_line(f, label)),
                });
            }
        }
        out
    }

    /// The open vault the sheet is for, by its place.
    pub(crate) fn load_sheet_vault(&self) -> Option<usize> {
        let s = self.load.as_ref()?;
        self.open.iter().position(|o| o.name == s.name)
    }

    /// Costs what `key_fingerprint` and `described` cost. Works out the
    /// sheet for open vault `v` with `ticked`, from the wallet records as
    /// the vault keeps them read ([`crate::vault::Vaults::described`]).
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub(crate) fn load_view(
        &self,
        c: &Compute,
        session: &Session,
        v: usize,
        ticked: &BTreeSet<usize>,
    ) -> LoadView {
        let Some(open) = self.open.get(v) else {
            return LoadView::default();
        };
        let held = |fp: &str| session.keys.all().any(|k| fp_text(k.fingerprint) == fp);
        // The vault's keys: record, fingerprint, label.
        let keys: Vec<(usize, Option<String>, String)> = self
            .records(open.id)
            .of(kind::KEY)
            .map(|(i, r)| {
                (
                    i,
                    self.key_fingerprint(c, session.network(), r),
                    r.public_text(field::KEY_LABEL).unwrap_or("").to_string(),
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
        let wallets: Vec<W> = self
            .records(open.id)
            .of(kind::WALLET)
            .map(|(i, r)| {
                let name = r
                    .public_text(field::WALLET_NAME)
                    .unwrap_or(EN.vaults_wallet)
                    .to_string();
                let policy = self.described(c, open.id, i).map(|d| &d.policy);
                let fps = policy.as_ref().map_or_else(Vec::new, |p| {
                    p.keys()
                        .iter()
                        .map(|k| k.fingerprint().map(fp_text))
                        .collect()
                });
                let loaded = self.described(c, open.id, i).is_some_and(|d| {
                    session
                        .wallets
                        .iter()
                        .any(|w| same_wallet(w.policy()) == d.same)
                });
                W {
                    record: i,
                    name,
                    kind: self
                        .described(c, open.id, i)
                        .map(|d| d.shape.clone())
                        .unwrap_or_default(),
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
                [one] => Some(fill(EN.load_sheet_also_in_wallet, &[one])),
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
            title: fill(EN.load_sheet_title, &[&open.label(self.store())]),
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
}
