//! Keys from a vault on Create's Keys card, and a new wallet made from
//! one already held (`docs/history/NEW-WALLET.md` §16, F45).
//!
//! The Keys card offers, under each slot's session keys, the keys of
//! every open vault that are not loaded, and one Unlock row for each
//! locked vault whose file Faraday has read. A wallet's card offers
//! **New wallet from this one**: Create filled from that wallet at the
//! next account its keys and kind do not use.

use std::collections::BTreeSet;
use std::rc::Rc;

use crate::strings::fill;
use faraday_bip::keys::Fingerprint;
use faraday_crypto::work::Compute;
use faraday_vault::records::{self, kind};

use crate::create::{self, FromWallet, NewKind, Source};
use crate::strings::EN;
use crate::wallet::{Session, fp_text, key_line};
use crate::{CreateState, Faraday, Screen, cstep};

/// One key of an open vault, as the Keys card offers it.
#[derive(Debug, Clone)]
pub(crate) struct VaultKey {
    /// Its record's place in the vault's slot.
    pub record: usize,
    /// Its master fingerprint.
    pub fp: [u8; 4],
    /// Its fingerprint and label, as a session key's button reads.
    pub line: String,
    /// It is 24 words: a share a threshold wallet can be dealt from.
    pub share: bool,
}

/// An open vault's keys that are not loaded, under its name.
#[derive(Debug, Clone)]
pub(crate) struct VaultKeys {
    /// The open vault, by its place in `vaults.open`.
    pub vault: usize,
    /// Its file's name.
    pub name: String,
    /// Its keys not in the session.
    pub keys: Vec<VaultKey>,
}

/// What the Keys card offers from the vaults (`docs/history/NEW-WALLET.md`
/// §16.1): the open vaults' keys, and the locked vaults, each by its
/// place in [`Faraday::vault_files`] and its name.
#[derive(Debug, Clone, Default)]
pub(crate) struct CreateVaults {
    /// The open vaults that hold keys not loaded.
    pub open: Vec<VaultKeys>,
    /// The locked vaults.
    pub locked: Vec<(usize, String)>,
}

/// A fingerprint written as eight hex digits, as bytes.
fn fp_bytes(text: &str) -> Option<[u8; 4]> {
    u32::from_str_radix(text, 16).ok().map(u32::to_be_bytes)
}

impl Faraday {
    /// Costs a read of each vault file's header and a hash of each open
    /// vault's key records. Works out
    /// [`crate::drawing::View::create_vaults`].
    pub(crate) fn create_vaults_now(&self, c: &Compute) -> Rc<CreateVaults> {
        Rc::new(self.create_vaults_uncached(c))
    }

    /// Costs what `key_fingerprint` costs. Works out
    /// [`crate::drawing::View::create_vaults`] (see
    /// [`Faraday::create_vaults_now`]).
    fn create_vaults_uncached(&self, c: &Compute) -> CreateVaults {
        let mut out = CreateVaults::default();
        for (v, open) in self.vaults.open.iter().enumerate() {
            let mut keys = Vec::new();
            for (i, r) in self.vaults.store.vault_records(open.id).of(kind::KEY) {
                // The fingerprint the unlock worked out; the record's own
                // label.
                let Some(text) = crate::vault::screen::key_fingerprint(c, self, r) else {
                    continue;
                };
                let Some(fp) = fp_bytes(&text) else {
                    continue;
                };
                if self.session.keys.all().any(|k| k.fingerprint.0 == fp) {
                    continue;
                }
                let label = r.text(records::field::KEY_LABEL).unwrap_or(EN.vaults_seed);
                keys.push(VaultKey {
                    record: i,
                    fp,
                    line: key_line(Fingerprint(fp), label),
                    share: crate::wallet::record_words(c, r).is_some_and(|m| m.word_count() == 24),
                });
            }
            if !keys.is_empty() {
                out.open.push(VaultKeys {
                    vault: v,
                    name: open.name.clone(),
                    keys,
                });
            }
        }
        out.locked = self
            .vault_files()
            .into_iter()
            .enumerate()
            .filter(|(_, f)| f.open.is_none())
            .map(|(i, f)| (i, f.name))
            .collect();
        out
    }

    /// The Keys card's press on a vault's key (`docs/history/NEW-WALLET.md`
    /// §16.1): loads that one record into the session, with the
    /// passphrase the record stores, as Load into session does, and fills
    /// slot `slot` with it. The vault stays open and unchanged. Costs the
    /// key's seed and master key.
    pub(crate) fn create_slot_vault(&mut self, slot: u8, v: usize, r: usize) {
        let c = self.compute();
        if !self.may_load_keys() {
            return;
        }
        let Some(cs) = self.create.as_ref() else {
            return;
        };
        if cs.built.is_some() || usize::from(slot) >= cs.slots.len() {
            return;
        }
        let Some(rec) = self
            .vaults
            .open
            .get(v)
            .and_then(|o| self.vaults.store.vault_records(o.id).records.get(r))
            .filter(|x| x.kind == kind::KEY)
        else {
            return;
        };
        let want = crate::vault::screen::key_fingerprint(&c, self, rec).and_then(|t| fp_bytes(&t));
        let before = self.session.keys.len();
        self.vault_load_set(v, &BTreeSet::from([r]));
        let fp = if self.session.keys.len() > before {
            self.session.keys.all().last().map(|k| k.fingerprint.0)
        } else {
            want.filter(|fp| self.session.keys.all().any(|k| k.fingerprint.0 == *fp))
        };
        let Some(fp) = fp else {
            return;
        };
        let name = self.vaults.open[v].name.clone();
        self.toast(&fill(
            EN.vaults_loaded_from,
            &[&fp_text(Fingerprint(fp)), &name],
        ));
        if let Some(c) = self.create.as_mut()
            && let Some(s) = c.slots.get_mut(usize::from(slot))
        {
            *s = Source::Here(fp);
            if c.pass_slot == Some(slot) {
                c.close_pass();
            }
        }
    }

    /// The lowest account no loaded wallet of `kind` over the keys `fps`
    /// uses (`docs/history/NEW-WALLET.md` §16.2).
    fn create_free_account(&self, kind: NewKind, fps: &[[u8; 4]]) -> u32 {
        let set = |list: Vec<[u8; 4]>| list.into_iter().collect::<BTreeSet<_>>();
        let want = set(fps.to_vec());
        let used: BTreeSet<u32> = self
            .session
            .wallets
            .iter()
            .filter(|w| NewKind::of(w.policy()) == Some(kind))
            .filter(|w| {
                set(w
                    .policy()
                    .keys()
                    .iter()
                    .filter_map(|k| k.fingerprint())
                    .map(|f| f.0)
                    .collect())
                    == want
            })
            .map(|w| create::account_of(w.policy()))
            .collect();
        (0..=u32::MAX >> 1).find(|a| !used.contains(a)).unwrap_or(0)
    }

    /// **New wallet from this one** on wallet `i`'s card
    /// (`docs/history/NEW-WALLET.md` §16.2): Create filled from the wallet, its
    /// kind, quorum and keys, at the lowest account its keys and kind do
    /// not use, opened on the first card that needs something. A key the
    /// wallet has in an open vault is loaded from there. The wallet is
    /// not changed.
    pub(crate) fn create_from_wallet(&mut self, i: usize) {
        let c = self.compute();
        let Some(w) = self.session.wallets.get(i) else {
            return;
        };
        let Some(kind) = NewKind::of(w.policy()).filter(|k| !k.threshold()) else {
            return;
        };
        let name = w.name.clone();
        let account = create::account_of(w.policy());
        let (m, _) = Session::quorum(w);
        let keys: Vec<(Option<[u8; 4]>, String)> = w
            .policy()
            .keys()
            .iter()
            .map(|k| (k.fingerprint().map(|f| f.0), k.key_text()))
            .collect();
        let wallet_slots = self.session.slots(w);
        let n = keys.len().max(1);
        let mut slots = Vec::new();
        let mut xpubs = Vec::new();
        for (j, (fp, text)) in keys.iter().enumerate() {
            let here =
                |app: &Faraday, fp: [u8; 4]| app.session.keys.all().any(|k| k.fingerprint.0 == fp);
            let held = match fp {
                Some(fp) if here(self, *fp) => true,
                Some(fp) if self.may_load_keys() => {
                    if let Some((v, r)) =
                        wallet_slots.get(j).and_then(|s| self.vault_key_for(&c, s))
                    {
                        self.vault_load_set(v, &BTreeSet::from([r]));
                    }
                    here(self, *fp)
                }
                _ => false,
            };
            match fp {
                Some(fp) if held => {
                    slots.push(Source::Here(*fp));
                    xpubs.push(None);
                }
                _ => {
                    slots.push(Source::Empty);
                    xpubs.push(Some(text.clone()));
                }
            }
        }
        let fps: Vec<[u8; 4]> = keys.iter().filter_map(|(fp, _)| *fp).collect();
        let next = self.create_free_account(kind, &fps);
        let mut done = [false; cstep::COUNT];
        done[cstep::KIND as usize] = true;
        done[cstep::QUORUM as usize] = true;
        let owner = self.fresh_owner();
        let from = FromWallet {
            name,
            kind,
            account,
            xpubs,
        };
        self.create = Some(CreateState {
            owner,
            done,
            kind,
            m: if kind.all_sign() { n } else { m.clamp(1, n) },
            n,
            slots,
            account: next,
            from: Some(from),
            more_kinds: !matches!(kind, NewKind::NativeSegwit | NewKind::Multi),
            ..CreateState::default()
        });
        self.create_from_refresh();
        self.screen = Screen::Create;
        self.create_open_next();
    }

    /// The slots of a creation started from a wallet whose key there is
    /// a cosigner's xpub, after its kind or account changed: the xpub
    /// where it is still that key, **Someone else** where another
    /// account or script type needs the cosigner's xpub again. A slot the
    /// person filled otherwise is kept.
    pub(crate) fn create_from_refresh(&mut self) {
        let Some(c) = self.create.as_mut() else {
            return;
        };
        if c.built.is_some() {
            return;
        }
        let Some(f) = c.from.as_ref() else {
            return;
        };
        for (i, s) in c.slots.iter_mut().enumerate() {
            let Some(want) = f.xpub_slot(i, c.kind, c.account) else {
                continue;
            };
            let auto = match &*s {
                Source::Empty | Source::Later => true,
                Source::Cosigner(t) => f.xpubs.get(i).and_then(|x| x.as_ref()) == Some(t),
                Source::Here(_) => false,
            };
            if auto {
                *s = want;
            }
        }
    }

    /// Kind's Account stepper: the account every key of the creation is
    /// used at (`docs/history/NEW-WALLET.md` §16.2, §17), for every kind that
    /// has one.
    pub(crate) fn create_account_step(&mut self, d: i8) {
        if let Some(c) = self.create.as_mut()
            && c.built.is_none()
            && c.kind.has_account()
        {
            c.account =
                (i64::from(c.account) + i64::from(d)).clamp(0, i64::from(u32::MAX >> 1)) as u32;
        }
        self.create_from_refresh();
    }

    /// The line a slot left for a cosigner reads, when the creation was
    /// started from a wallet that has the cosigner's key at another
    /// account or script type: "Needs Key n's xpub at account n".
    pub(crate) fn create_slot_needs(&self, slot: usize) -> Option<String> {
        let c = self.create.as_ref()?;
        let f = c.from.as_ref()?;
        if c.slots.get(slot)? != &Source::Later {
            return None;
        }
        f.xpubs.get(slot)?.as_ref()?;
        Some(fill(
            EN.create_needs_xpub_at,
            &[&(slot + 1).to_string(), &c.account.to_string()],
        ))
    }

    /// Whether Check's Path row offers **Change**: the kind has an
    /// account, and nothing has used the made wallet yet. Its backup has
    /// not started, and the flow that set up its vaults is where it was
    /// when the wallet was made (`docs/history/NEW-WALLET.md` §17).
    pub(crate) fn create_can_take_back(&self) -> bool {
        let Some(c) = self.create.as_ref() else {
            return false;
        };
        let Some(w) = c.built else {
            return false;
        };
        if !c.kind.has_account() || c.kind.threshold() {
            return false;
        }
        if self.backup.as_ref().is_some_and(|b| b.wallet == w) {
            return false;
        }
        // A wallet added here comes off the end; one found loaded already
        // stays where it is.
        if c.added && w + 1 != self.session.wallets.len() {
            return false;
        }
        match c.before_made.as_ref() {
            None => self.wallet_flow_of_owner(c.owner).is_none(),
            Some((f, was)) => self.backup_flows.get(*f).is_some_and(|now| {
                now.step == was.step && now.wallet.as_ref().is_some_and(|wf| wf.owner == c.owner)
            }),
        }
    }

    /// Check's Path row **Change**: the made wallet is taken back and
    /// Kind opens on its Account. The keys stay in their slots; Keys'
    /// Continue makes the wallet again at the account chosen.
    pub(crate) fn create_take_back(&mut self) {
        if !self.create_can_take_back() {
            return;
        }
        let Some(c) = self.create.as_mut() else {
            return;
        };
        let Some(w) = c.built.take() else {
            return;
        };
        c.done[crate::cstep::KEYS as usize] = false;
        c.done[crate::cstep::CHECK as usize] = false;
        c.open = Some(crate::cstep::KIND);
        c.scroll.follow = true;
        c.path.clear();
        c.error = None;
        let added = std::mem::take(&mut c.added);
        let before = c.before_made.take();
        if let Some((f, was)) = before {
            self.backup_flows[f] = *was;
        }
        if added {
            let sum = self.session.wallets[w].checksum().to_string();
            if self.chart_work.moving_to.as_deref() == Some(sum.as_str()) {
                self.chart_work.moving_to = None;
            }
            self.session.wallets.remove(w);
            self.wallet = w.saturating_sub(1);
        }
    }
}
