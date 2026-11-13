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
    faraday_crypto::hex::decode_exact::<4>(text)
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
            for (i, r) in self.vaults.records(open.id).of(kind::KEY) {
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
                let label = r
                    .public_text(records::field::KEY_LABEL)
                    .unwrap_or(EN.vaults_seed);
                keys.push(VaultKey {
                    record: i,
                    fp,
                    line: key_line(Fingerprint(fp), label),
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
        let Some(cs) = self.create.state() else {
            return;
        };
        if cs.built.is_some() || usize::from(slot) >= cs.slots.len() {
            return;
        }
        let Some(fp) = self.vault_key_load(v, r) else {
            return;
        };
        self.create.fill_slot_from_vault(slot, fp);
    }

    /// Costs the key's seed and master key. Loads record `r` of open vault
    /// `v` into the session, with the passphrase the record stores, as Load
    /// into session does, and says so; the vault stays open and unchanged.
    /// Returns the key's fingerprint, once it is loaded.
    pub(crate) fn vault_key_load(&mut self, v: usize, r: usize) -> Option<[u8; 4]> {
        if !self.may_load_keys() {
            return None;
        }
        let c = self.compute();
        let rec = self
            .vaults
            .open
            .get(v)
            .and_then(|o| self.vaults.records(o.id).get(r))
            .filter(|x| x.kind() == kind::KEY)?;
        let want = crate::vault::screen::key_fingerprint(&c, self, rec).and_then(|t| fp_bytes(&t));
        // The key's words are stretched in a job; the press is run again
        // when it is in, and finds it.
        let added: crate::vault::load_job::Then = Box::new(|app, keys, _| {
            if keys > 0 {
                let fp = app.session.keys.all().last().map(|k| k.fingerprint.0);
                app.vaults.key_added_swap(fp);
            }
        });
        self.vaults.key_added_swap(None);
        if crate::vault::load_job::start_replay(self, v, &BTreeSet::from([r]), added) {
            return None;
        }
        // The key the job added, else the one the record names if it is held.
        let fp = match self.vaults.key_added_swap(None) {
            Some(fp) => fp,
            None => want.filter(|fp| self.session.keys.all().any(|k| k.fingerprint.0 == *fp))?,
        };
        let name = self.vaults.open[v].name.clone();
        self.toast(&fill(
            EN.vaults_loaded_note,
            &[&fp_text(Fingerprint(fp)), &name],
        ));
        Some(fp)
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
        let Some(kind) = NewKind::of(w.policy()) else {
            return;
        };
        let name = w.name.clone();
        let account = create::account_of(w.policy());
        let bip87 = kind.has_bip87() && create::bip87_of(w.policy());
        let (m, _) = Session::quorum(w);
        let keys: Vec<(Option<[u8; 4]>, String)> = w
            .policy()
            .keys()
            .iter()
            .map(|k| (k.fingerprint().map(|f| f.0), k.key_text()))
            .collect();
        let wallet_slots = self.session.slots(w);
        let n = keys.len().max(1);
        // The seeds the vault keeps of every slot's key are stretched in
        // one job per vault; the press is run again when they are in.
        if self.may_load_keys() {
            let mut wanted: Vec<(usize, BTreeSet<usize>)> = Vec::new();
            for (j, (fp, _)) in keys.iter().enumerate() {
                let held =
                    fp.is_none_or(|fp| self.session.keys.all().any(|k| k.fingerprint.0 == fp));
                if held {
                    continue;
                }
                if let Some((v, r)) = wallet_slots.get(j).and_then(|s| self.vault_key_for(&c, s)) {
                    match wanted.iter_mut().find(|(u, _)| *u == v) {
                        Some((_, set)) => {
                            set.insert(r);
                        }
                        None => wanted.push((v, BTreeSet::from([r]))),
                    }
                }
            }
            for (v, set) in wanted {
                let nothing: crate::vault::load_job::Then = Box::new(|_, _, _| {});
                if crate::vault::load_job::start_replay(self, v, &set, nothing) {
                    return;
                }
            }
        }
        let mut slots = Vec::new();
        let mut xpubs = Vec::new();
        for (fp, text) in &keys {
            let held = fp.is_some_and(|fp| self.session.keys.all().any(|k| k.fingerprint.0 == fp));
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
            bip87,
            xpubs,
        };
        self.create.set(CreateState {
            owner,
            done,
            kind,
            m: if kind.all_sign() { n } else { m.clamp(1, n) },
            n,
            slots,
            account: next,
            bip87,
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
        self.create.refresh_from();
    }

    /// The Account row (F98, `docs/history/NEW-WALLET.md` §16.2, §17): the
    /// account every key of the creation is used at, for every kind that
    /// has one. `d` 0 is the row's **Change**, which opens its stepper;
    /// −1 and +1 step it.
    pub(crate) fn create_account_step(&mut self, d: i8) {
        self.create.step_account(d);
        self.create_from_refresh();
    }

    /// The multisig path row's **Change**: opens its choice.
    pub(crate) fn create_multi_path_change(&mut self) {
        self.create.open_path_choice();
        self.create_from_refresh();
    }

    /// The path choice beside the Account row: BIP-87's path (`bip87`) or
    /// the kind's own. A kind that is not a multisig of a descriptor has no
    /// choice. Of the keys already in slots, a cosigner's or a file's at
    /// the other path no longer fits and its slot is emptied (a slot of a
    /// wallet this was started from gets its xpub back where it fits); the
    /// keys held here stay.
    pub(crate) fn create_multi_path(&mut self, bip87: bool) {
        self.create.choose_path(bip87);
        self.create_from_refresh();
    }

    /// The line a slot left for a cosigner reads, when the creation was
    /// started from a wallet that has the cosigner's key at another
    /// account or script type: "Needs Key n's xpub at account n".
    pub(crate) fn create_slot_needs(&self, slot: usize) -> Option<String> {
        let c = self.create.state()?;
        let f = c.from.as_ref()?;
        if c.slots.get(slot)? != &Source::Later {
            return None;
        }
        f.xpubs.get(slot)?.as_ref()?;
        let line = if c.at_bip87() {
            EN.create_needs_xpub_bip87_account
        } else {
            EN.create_needs_xpub_account
        };
        Some(fill(
            line,
            &[&(slot + 1).to_string(), &c.account.to_string()],
        ))
    }

    /// Whether Check's Path row offers **Change**: the kind has an
    /// account, and nothing has used the made wallet yet. Its backup has
    /// not started, and the flow that set up its vaults is where it was
    /// when the wallet was made (`docs/history/NEW-WALLET.md` §17).
    pub(crate) fn create_can_take_back(&self) -> bool {
        let Some(c) = self.create.state() else {
            return false;
        };
        let Some(w) = c.built else {
            return false;
        };
        if !c.has_account() {
            return false;
        }
        if self.backup.state().is_some_and(|b| b.wallet == w) {
            return false;
        }
        // A wallet added here comes off the end; one found loaded already
        // stays where it is.
        if c.added && w + 1 != self.session.wallets.len() {
            return false;
        }
        match c.before_made.as_ref() {
            None => self.wallet_flow_of_owner(c.owner).is_none(),
            Some((f, was)) => self.flows.get(*f).is_some_and(|now| {
                now.step == was.step && now.wallet.as_ref().is_some_and(|wf| wf.owner == c.owner)
            }),
        }
    }

    /// Check's Path row **Change**: the made wallet is taken back and the
    /// card that carries the Account row opens with its stepper open
    /// (F98): Quorum, or Keys for a kind with no Quorum card. The keys
    /// stay in their slots; Keys' Continue makes the wallet again at the
    /// account chosen.
    pub(crate) fn create_take_back(&mut self) {
        if !self.create_can_take_back() {
            return;
        }
        let Some(crate::create::TakenBack {
            wallet: w,
            added,
            before,
        }) = self.create.take_back()
        else {
            return;
        };
        if let Some((f, was)) = before {
            self.flows.restore(f, *was);
        }
        if added {
            let sum = self.session.wallets[w].checksum().to_string();
            self.glance.forget_moving_to(&sum);
            self.session.wallets.remove(w);
            self.wallets.set_wallet(w.saturating_sub(1));
            self.spend_after_wallet_removed();
        }
    }
}
