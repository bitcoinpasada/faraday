//! Restoring a wallet from its backup (`docs/WALLETS.md` §2 and §5): the
//! wallet from its descriptor, its multisig config, or the split shares of
//! a multisig, which are merged as they arrive until every key is in hand;
//! then the seeds, typed back in.

use crate::strings::EN;
use crate::strings::fill;
use faraday_bip::multisig_config;
use faraday_crypto::work::Compute;

use crate::wallet::{Session, fp_text};
use crate::{
    Action, Faraday, FileKind, MAX_VAULTS, NOT_A_KEY, Screen, VaultAsk, create, flow, restore,
    rstep, seeds, stem, vault, wallet,
};
use std::collections::BTreeSet;

/// What one multisig config file, whole or a share, says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigParts {
    /// `Name:`.
    pub name: Option<String>,
    /// `Policy: M of N`.
    pub m: usize,
    /// The N of the policy.
    pub n: usize,
    /// `Format:`.
    pub format: String,
    /// Each key line: fingerprint, the derivation in force, the xpub.
    pub keys: Vec<(String, String, String)>,
}

/// Reads a multisig config's lines without requiring every key.
pub fn parts(text: &str) -> Option<ConfigParts> {
    let mut name = None;
    let mut policy = None;
    let mut format = None;
    let mut derivation = String::new();
    let mut keys = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (k, v) = line.split_once(':')?;
        let (k, v) = (k.trim(), v.trim());
        match k.to_ascii_lowercase().as_str() {
            "name" => name = Some(v.to_string()),
            "policy" => {
                let (m, n) = v.split_once(" of ")?;
                policy = Some((m.trim().parse().ok()?, n.trim().parse().ok()?));
            }
            "format" => format = Some(v.to_string()),
            "derivation" => derivation = v.to_string(),
            fp if fp.len() == 8 && fp.chars().all(|c| c.is_ascii_hexdigit()) => {
                keys.push((fp.to_string(), derivation.clone(), v.to_string()));
            }
            _ => {}
        }
    }
    let (m, n) = policy?;
    if keys.is_empty() {
        return None;
    }
    Some(ConfigParts {
        name,
        m,
        n,
        format: format.unwrap_or_else(|| "P2WSH".into()),
        keys,
    })
}

/// Whether a config holds fewer keys than its policy names: a share.
pub fn is_share(text: &str) -> bool {
    parts(text).is_some_and(|p| p.keys.len() < p.n)
}

/// What a set of shares adds up to.
pub struct Merged {
    /// The quorum and size, from the shares.
    pub m: usize,
    /// Keys in the wallet.
    pub n: usize,
    /// The fingerprints in hand.
    pub have: Vec<String>,
    /// The whole config, once every key is in hand.
    pub whole: Option<String>,
}

/// Merges shares of one wallet. Shares that disagree on the quorum or the
/// format are refused.
pub fn merge(texts: &[String]) -> Result<Merged, String> {
    let mut all: Option<ConfigParts> = None;
    for t in texts {
        let p = parts(t).ok_or(EN.restore_not_a_multisig)?;
        match all.as_mut() {
            None => all = Some(p),
            Some(a) => {
                if a.m != p.m || a.n != p.n || !a.format.eq_ignore_ascii_case(&p.format) {
                    return Err(EN.restore_these_shares.into());
                }
                for k in p.keys {
                    if !a.keys.iter().any(|x| x.0.eq_ignore_ascii_case(&k.0)) {
                        a.keys.push(k);
                    }
                }
            }
        }
    }
    let a = all.ok_or(EN.restore_no_shares)?;
    let have: Vec<String> = a.keys.iter().map(|k| k.0.to_lowercase()).collect();
    // A Taproot multisig's sheets rebuild its descriptor: Coldcard's
    // config has no Taproot format.
    if a.format.eq_ignore_ascii_case("P2TR") {
        let whole = (a.keys.len() >= a.n).then(|| {
            let keys: Vec<String> = a
                .keys
                .iter()
                .map(|(fp, d, x)| {
                    let path = d.trim_start_matches("m/").replace('\'', "h");
                    let fp = fp.to_lowercase();
                    if path.is_empty() || path == "m" {
                        format!("[{fp}]{x}/<0;1>/*")
                    } else {
                        format!("[{fp}/{path}]{x}/<0;1>/*")
                    }
                })
                .collect();
            format!("tr({NUMS},sortedmulti_a({},{}))", a.m, keys.join(","))
        });
        return Ok(Merged {
            m: a.m,
            n: a.n,
            have,
            whole,
        });
    }
    let whole = (a.keys.len() >= a.n).then(|| {
        let mut out = String::from("# Multisig config rebuilt by Faraday from its shares\n");
        if let Some(n) = &a.name {
            out.push_str(&format!("Name: {n}\n"));
        }
        out.push_str(&format!(
            "Policy: {} of {}\nFormat: {}\n",
            a.m, a.n, a.format
        ));
        for (fp, d, x) in &a.keys {
            out.push_str(&format!("\nDerivation: {d}\n{fp}: {x}\n"));
        }
        out
    });
    Ok(Merged {
        m: a.m,
        n: a.n,
        have,
        whole,
    })
}

/// Costs what `parse` and `read_wallet` cost. Whether the rebuilt text
/// reads as a wallet.
pub fn readable(c: &Compute, whole: &str) -> bool {
    multisig_config::parse(c, whole).is_ok() || crate::wallet::read_wallet(c, whole).is_ok()
}

/// BIP-341's unspendable internal key, a Taproot multisig's.
const NUMS: &str = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";

/// Restoring a wallet: the cards' state (`docs/history/NEW-WALLET.md` §12).
#[derive(Default)]
pub struct RestoreState {
    /// The open card.
    pub open: Option<u8>,
    /// Cards closed as done.
    pub done: [bool; rstep::COUNT as usize],
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// The Inbox shares chosen, by name.
    pub shares: Vec<String>,
    /// The wallet, by index into the session: read from its description,
    /// or made from the seeds.
    pub wallet: Option<usize>,
    /// The wallet came from its description: the seeds are matched to its
    /// keys.
    pub described: bool,
    /// The last refusal.
    pub error: Option<String>,
    /// The kind, the quorum and the seeds and cosigners' xpubs in hand,
    /// for a wallet made from the seeds.
    pub seeds: Option<seeds::SeedsState>,
    /// Kind's **More kinds** is expanded.
    pub more_kinds: bool,
    /// The description's keys marked Not here, by place.
    pub absent: Vec<u8>,
    /// Open from a key on Wallets: Back goes to Wallets.
    pub from_key: bool,
    /// Open from Home: Back goes to Home.
    pub from_home: bool,
    /// Whether vaults are part of the backup and how many; with yes, the
    /// sticks and the vaults come before the seeds are typed
    /// (`docs/BACKUP-FLOW.md` §11).
    pub vaults: VaultAsk,
    /// The number the wallet's backup flow is tied to (`Faraday::owner_seq`).
    pub owner: u64,
}

impl RestoreState {
    /// The person answered yes to using vaults.
    pub fn into_vaults(&self) -> bool {
        self.vaults.answer == Some(true)
    }

    /// The wallet is made from the Keys card, and the card is closed: a
    /// key loaded now is not one of its seeds.
    pub fn keys_made(&self) -> bool {
        self.done[rstep::SEEDS as usize] && self.open != Some(rstep::SEEDS)
    }

    /// A fresh Restore, open on Kind with a single key ticked.
    pub fn new() -> RestoreState {
        let mut s = seeds::SeedsState::default();
        s.fix(create::NewKind::NativeSegwit);
        // The question of vaults is done only once the person has
        // answered it and pressed its own Continue.
        let done = [false; rstep::COUNT as usize];
        RestoreState {
            open: Some(rstep::KIND),
            seeds: Some(s),
            done,
            ..RestoreState::default()
        }
    }

    /// The kind chosen or read.
    pub fn kind(&self) -> create::NewKind {
        self.seeds.as_ref().map(|s| s.kind).unwrap_or_default()
    }

    /// The cards this kind has, in order: Quorum for a multisig.
    pub fn steps(&self) -> Vec<u8> {
        let kind = self.kind();
        let mut v = vec![rstep::KIND];
        if kind.multi() && !kind.threshold() {
            v.push(rstep::QUORUM);
        }
        v.extend([rstep::DESCRIPTION, rstep::WHERE]);
        if self.into_vaults() {
            v.push(rstep::HOW_MANY);
        }
        v.extend([rstep::SEEDS, rstep::CHECK, rstep::DONE]);
        v
    }

    /// Closes card `k` as done and opens the next not done after it.
    pub fn next(&mut self, k: u8) {
        self.done[k as usize] = true;
        let steps = self.steps();
        let at = steps.iter().position(|&s| s == k).unwrap_or(0);
        self.open = steps[at..]
            .iter()
            .chain(steps[..at].iter())
            .copied()
            .find(|&s| !self.done[s as usize]);
        self.scroll.follow = true;
    }
}

/// Restore's area: the wallet being restored, while the flow is open.
#[derive(Default)]
pub(crate) struct RestoreArea {
    state: Option<RestoreState>,
}

impl RestoreArea {
    /// The flow's state, while it is open.
    pub(crate) fn state(&self) -> Option<&RestoreState> {
        self.state.as_ref()
    }

    /// The flow's state, for the writers in other areas, until they are
    /// narrowed.
    pub(crate) fn state_mut(&mut self) -> Option<&mut RestoreState> {
        self.state.as_mut()
    }

    /// Starts the flow on `state`.
    pub(crate) fn set(&mut self, state: RestoreState) {
        self.state = Some(state);
    }

    /// Takes the flow out, leaving none.
    pub(crate) fn take(&mut self) -> Option<RestoreState> {
        self.state.take()
    }

    /// Forgets the flow.
    pub(crate) fn clear(&mut self) {
        self.state = None;
    }
}

impl Faraday {
    /// The Restore flow's state, while it is open.
    pub fn restore(&self) -> Option<&RestoreState> {
        self.restore.state()
    }

    /// Costs what the actions below cost, the keys' and wallets' work among
    /// it: handles the Restore wallet flow's actions (`Action::R*`, `RestoreWallet`, `RestoreShares`, `KeyWallet`).
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub(crate) fn restore_act(&mut self, action: Action, c: &Compute) {
        match action {
            Action::RFromVault(v, r) => self.restore_from_vault(v, r),
            Action::RestoreWallet
                if self.restore.state.as_ref().is_some_and(|r| {
                    self.setup_pending(r.owner, r.into_vaults(), r.done[rstep::SEEDS as usize])
                }) =>
            {
                self.screen = Screen::Restore;
            }
            Action::RestoreWallet => {
                let owner = self.fresh_owner();
                self.restore.state = Some(RestoreState {
                    owner,
                    from_home: self.screen == Screen::Home,
                    ..RestoreState::new()
                });
                self.screen = Screen::Restore;
            }
            // The seeds wait for the question of vaults and for the
            // setup it asked for (`docs/history/NEW-WALLET.md` §12.3 item 4).
            Action::RStep(k)
                if k == rstep::SEEDS
                    && self.restore.state.as_ref().is_some_and(|r| {
                        let made = r.done[rstep::SEEDS as usize];
                        (r.vaults.answer.is_none() && !made)
                            || self.setup_pending(r.owner, r.into_vaults(), made)
                    }) => {}
            Action::RVaults(_) | Action::RVaultN(_)
                if self.restore.state.as_ref().is_some_and(|r| {
                    !r.done[rstep::SEEDS as usize] && self.setup_vault_open(r.owner)
                }) =>
            {
                self.toast(EN.create_vaults_locked);
            }
            Action::RStep(k) => {
                if let Some(r) = self.restore.state.as_mut() {
                    r.open = if r.open == Some(k) { None } else { Some(k) };
                    r.scroll.follow = true;
                }
            }
            // The question of vaults waits for its answer.
            Action::RNext(k)
                if k == rstep::WHERE
                    && self
                        .restore
                        .state()
                        .is_some_and(|r| r.vaults.answer.is_none()) => {}
            // How many vaults: Continue opens the setup, which comes back
            // to Seeds.
            Action::RNext(k) if k == rstep::HOW_MANY => {
                if let Some(r) = self.restore.state.as_mut() {
                    r.done[k as usize] = true;
                    let owner = r.owner;
                    self.wallet_flow_begin(owner);
                }
            }
            Action::RNext(k) => {
                if let Some(r) = self.restore.state.as_mut() {
                    r.next(k);
                }
            }
            Action::RVaults(yes) => {
                if let Some(r) = self.restore.state.as_mut()
                    && !r.done[rstep::SEEDS as usize]
                    && r.vaults.answer != Some(yes)
                {
                    let owner = r.owner;
                    r.vaults.answer = Some(yes);
                    r.done[rstep::WHERE as usize] = false;
                    r.done[rstep::HOW_MANY as usize] = false;
                    self.wallet_vaults_changed(owner);
                }
            }
            Action::RVaultN(n) => {
                if let Some(r) = self.restore.state.as_mut()
                    && !r.done[rstep::SEEDS as usize]
                {
                    let owner = r.owner;
                    r.vaults.count = usize::from(n).clamp(1, MAX_VAULTS);
                    r.done[rstep::HOW_MANY as usize] = false;
                    self.wallet_vaults_changed(owner);
                }
            }
            Action::RKind(i) => self.restore_kind(i),
            Action::RMoreKinds => {
                if let Some(r) = self.restore.state.as_mut() {
                    r.more_kinds = true;
                }
            }
            Action::RFromFiles => {
                let files: Vec<usize> = self
                    .inbox
                    .iter()
                    .enumerate()
                    .filter(|(_, i)| i.kind == FileKind::Wallet)
                    .map(|(k, _)| k)
                    .collect();
                match files.as_slice() {
                    [one] => self.act(Action::RUse(*one)),
                    _ => {
                        if let Some(r) = self.restore.state.as_mut() {
                            r.open = Some(rstep::DESCRIPTION);
                            r.scroll.follow = true;
                        }
                    }
                }
            }
            Action::RAbsent(k) => {
                if let Some(r) = self.restore.state.as_mut()
                    && r.described
                {
                    match r.absent.iter().position(|&a| a == k) {
                        Some(p) => {
                            r.absent.remove(p);
                        }
                        None => r.absent.push(k),
                    }
                    r.error = None;
                }
            }
            // The passphrase is asked after the seed: on Add a key for
            // typed words, on the scan's sheet for a SeedQR (`seed_read`).
            Action::RSlotWords => {
                if let Some(r) = self.restore.state.as_mut() {
                    r.error = None;
                }
                self.act(Action::Entry(None));
            }
            Action::RSlotScan => {
                if self.may_load_keys() {
                    if let Some(r) = self.restore.state.as_mut() {
                        r.error = None;
                    }
                    self.entry.reset();
                    self.entry.state_mut().back = Some(Screen::Restore);
                    self.act(Action::ScanSeed);
                }
            }
            Action::RUse(i) => {
                if let Some(item) = self.inbox.get(i) {
                    let text = String::from_utf8_lossy(&item.bytes).into_owned();
                    let name = faraday_bip::multisig_config::parse_named(c, &text)
                        .ok()
                        .and_then(|(_, n)| n)
                        .unwrap_or_else(|| stem(&item.name));
                    let source = item.name.clone();
                    self.restore_wallet(&name, &text, &source, wallet::Origin::Named);
                }
            }
            Action::RShare(i) => {
                if let Some(item) = self.inbox.get(i) {
                    let n = item.name.clone();
                    if let Some(r) = self.restore.state.as_mut() {
                        if let Some(p) = r.shares.iter().position(|x| *x == n) {
                            r.shares.remove(p);
                        } else {
                            r.shares.push(n);
                        }
                    }
                }
            }
            Action::RRebuild => {
                let texts = self.restore_share_texts(c);
                match restore::merge(&texts) {
                    Ok(m) => match m.whole {
                        Some(whole) => {
                            let name = faraday_bip::multisig_config::parse_named(c, &whole)
                                .ok()
                                .and_then(|(_, n)| n)
                                .unwrap_or_else(|| EN.act_restored_wallet.to_string());
                            self.restore_wallet(
                                &name,
                                &whole,
                                EN.act_its_shares,
                                wallet::Origin::Named,
                            );
                        }
                        None => {
                            if let Some(r) = self.restore.state.as_mut() {
                                r.error = Some(fill(
                                    EN.act_of_keys_in_hand,
                                    &[&m.have.len().to_string(), &m.n.to_string()],
                                ));
                            }
                        }
                    },
                    Err(e) => {
                        if let Some(r) = self.restore.state.as_mut() {
                            r.error = Some(e);
                        }
                    }
                }
            }
            Action::RSeeds => {
                if let Some(r) = self.restore.state.as_mut()
                    && !r.kind().threshold()
                {
                    // The wallet comes from the seeds now, not a file.
                    r.wallet = None;
                    r.described = false;
                    r.absent.clear();
                    r.error = None;
                    r.done[rstep::SEEDS as usize] = false;
                    r.done[rstep::CHECK as usize] = false;
                    r.next(rstep::DESCRIPTION);
                }
            }
            Action::KeyWallet(fp, n) => {
                use create::NewKind;
                let mut s = seeds::SeedsState::default();
                s.fix(if n > 1 {
                    NewKind::Multi
                } else {
                    NewKind::NativeSegwit
                });
                s.set_n(usize::from(n));
                s.take(fp);
                let mut r = RestoreState {
                    owner: self.fresh_owner(),
                    seeds: Some(s),
                    from_key: true,
                    ..RestoreState::default()
                };
                for k in [rstep::KIND, rstep::QUORUM, rstep::DESCRIPTION] {
                    r.done[k as usize] = true;
                }
                r.open = Some(rstep::SEEDS);
                self.restore.state = Some(r);
                self.screen = Screen::Restore;
            }
            Action::RestoreShares => {
                let texts: Vec<String> = self
                    .inbox
                    .iter()
                    .filter(|i| i.kind == FileKind::Share)
                    .map(|i| String::from_utf8_lossy(&i.bytes).into_owned())
                    .collect();
                match restore::merge(&texts).map(|m| m.whole) {
                    Ok(Some(whole)) => {
                        let name = faraday_bip::multisig_config::parse_named(c, &whole)
                            .ok()
                            .and_then(|(_, n)| n)
                            .unwrap_or_else(|| EN.act_restored_wallet.to_string());
                        match self.session.add_wallet(
                            c,
                            &name,
                            &whole,
                            EN.act_its_shares,
                            wallet::Origin::Named,
                        ) {
                            Ok(i) => {
                                self.wallets.set_wallet(i);
                                self.screen = Screen::Wallets;
                                self.refresh_spend();
                                self.toast(&fill(
                                    EN.act_restored_from_shares,
                                    &[&name.to_string()],
                                ));
                            }
                            Err(e) => self.toast(&e.text()),
                        }
                    }
                    Ok(None) => self.toast(EN.act_keys_missing),
                    Err(e) => self.toast(&e),
                }
            }
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }

    /// Restore's Kind: a row pressed. The quorum goes to 2 of 3 for a
    /// multisig, one key for a single key; a description read has chosen
    /// already.
    pub(crate) fn restore_kind(&mut self, i: u8) {
        let Some(&kind) = create::NewKind::ALL.get(usize::from(i)) else {
            return;
        };
        let Some(r) = self.restore.state.as_mut().filter(|r| !r.described) else {
            return;
        };
        let Some(s) = r.seeds.as_mut() else {
            return;
        };
        let was = s.kind;
        s.fix(kind);
        if was.multi() != kind.multi() || was.all_sign() != kind.all_sign() {
            s.set_n(if kind.multi() { 3 } else { 1 });
        }
        r.error = None;
    }

    /// Why Restore does not take the seed whose key is `fp`, if it does
    /// not: with the description known, not one of its keys; from the
    /// seeds alone, no slot left. Once the wallet is made and Keys is
    /// closed, a key added is no seed of Restore's: nothing to refuse.
    pub(crate) fn restore_refusal(&self, fp: faraday_bip::keys::Fingerprint) -> Option<String> {
        let r = self.restore.state.as_ref().filter(|r| !r.keys_made())?;
        if r.described {
            let w = self.session.wallets.get(r.wallet?)?;
            // A threshold wallet's slots are shares, matched as they load.
            if w.policy().record().is_some() {
                return None;
            }
            let ours = self
                .session
                .slots(w)
                .iter()
                .any(|s| s.fingerprint == Some(fp));
            return (!ours).then(|| NOT_A_KEY.to_string());
        }
        let s = r.seeds.as_ref()?;
        (!s.keys.contains(&fp.0) && s.first_empty().is_none())
            .then(|| EN.restore_refusal_every.to_string())
    }

    /// A seed for Restore's open slot, loaded: matched to the
    /// description's keys or put in the slot (`docs/history/NEW-WALLET.md`
    /// §12.1).
    pub(crate) fn restore_took(&mut self, fp: faraday_bip::keys::Fingerprint) {
        if let Some(why) = self.restore_refusal(fp) {
            if let Some(r) = self.restore.state.as_mut() {
                r.error = Some(why);
            }
            return;
        }
        if self
            .restore
            .state
            .as_ref()
            .is_some_and(RestoreState::keys_made)
        {
            return;
        }
        // Matched: its slot reads "Key k of the wallet" and is here.
        let place = self
            .restore
            .state()
            .and_then(|r| r.wallet.filter(|_| r.described))
            .and_then(|i| self.session.wallets.get(i))
            .and_then(|w| {
                self.session
                    .slots(w)
                    .iter()
                    .position(|s| s.fingerprint == Some(fp))
            });
        let Some(r) = self.restore.state.as_mut() else {
            return;
        };
        if let Some(k) = place {
            r.absent.retain(|&a| usize::from(a) != k);
        } else if !r.described
            && let Some(s) = r.seeds.as_mut()
        {
            s.take(fp.0);
        }
        r.error = None;
    }

    /// A description or a share read by the camera while Restore waits
    /// for one: the wallet loads, or the share is taken. Returns whether
    /// it was.
    pub(crate) fn restore_arrived(&mut self, at: usize) -> bool {
        let c = self.compute();
        let Some((kind, name)) = self.inbox.get(at).map(|i| (i.kind, i.name.clone())) else {
            return false;
        };
        let Some(r) = self.restore.state.as_mut().filter(|r| !r.described) else {
            return false;
        };
        match kind {
            FileKind::Wallet => {
                self.act(Action::RUse(at));
                true
            }
            FileKind::Share => {
                if !r.shares.contains(&name) {
                    r.shares.push(name);
                }
                r.open = Some(rstep::DESCRIPTION);
                r.scroll.follow = true;
                let whole =
                    restore::merge(&self.restore_share_texts(&c)).is_ok_and(|m| m.whole.is_some());
                if whole {
                    self.act(Action::RRebuild);
                }
                true
            }
            _ => false,
        }
    }

    /// Restore's Make the wallet with the description known: every slot
    /// has its seed or is Not here.
    pub(crate) fn restore_described_make(&mut self) {
        let Some(r) = self.restore.state.as_ref() else {
            return;
        };
        let Some(w) = r.wallet.and_then(|i| self.session.wallets.get(i)) else {
            return;
        };
        let open = self
            .session
            .slots(w)
            .iter()
            .enumerate()
            .filter(|(k, s)| s.held_by.is_none() && !r.absent.contains(&(*k as u8)))
            .count();
        let Some(r) = self.restore.state.as_mut() else {
            return;
        };
        if open > 0 {
            r.error = Some(fill(
                EN.restore_described_make_still,
                &[
                    &open.to_string(),
                    (if open == 1 {
                        EN.restore_described_make_slot
                    } else {
                        EN.restore_described_make_slots
                    }),
                ],
            ));
            return;
        }
        r.next(rstep::SEEDS);
        self.refresh_spend();
    }

    pub(crate) fn restore_from_vault(&mut self, v: usize, r: usize) {
        let c = self.compute();
        use faraday_vault::records::{field, kind};
        let Some(open) = self.vaults.open.get(v) else {
            return;
        };
        let Some(rec) = self.vaults.records(open.id).get(r) else {
            return;
        };
        let (Some(text), name) = (
            rec.public_text(field::WALLET).map(str::to_string),
            rec.public_text(field::WALLET_NAME)
                .unwrap_or(EN.restore_from_vault_wallet)
                .to_string(),
        ) else {
            return;
        };
        let source = open.name.clone();
        self.restore_wallet(&name, &text, &source, wallet::Origin::Vault);
        let Some(w) = self
            .restore
            .state()
            .and_then(|s| s.wallet)
            .and_then(|i| self.session.wallets.get(i))
        else {
            return;
        };
        // The vault's keys for this wallet's slots load with it.
        let want: Vec<String> = self
            .session
            .slots(w)
            .iter()
            .filter_map(|s| s.fingerprint.map(fp_text))
            .collect();
        let open = &self.vaults.open[v];
        let keep: BTreeSet<usize> = self
            .vaults
            .records(open.id)
            .of(kind::KEY)
            .filter(|(_, k)| {
                vault::screen::key_fingerprint(&c, self, *k).is_some_and(|fp| want.contains(&fp))
            })
            .map(|(i, _)| i)
            .chain([r])
            .collect();
        let all: BTreeSet<usize> = (0..self.vaults.records(open.id).len()).collect();
        let saved = self
            .vaults
            .swap_skip(v, all.difference(&keep).copied().collect());
        let (keys, _) = self.vault_load(v, false);
        self.vaults.swap_skip(v, saved);
        self.screen = Screen::Restore;
        if keys > 0 {
            self.toast(&fill(
                EN.restore_from_vault_count,
                &[
                    &keys.to_string(),
                    (if keys == 1 {
                        EN.restore_from_vault_key
                    } else {
                        EN.restore_from_vault_keys
                    }),
                    &source.to_string(),
                ],
            ));
        }
    }

    /// Costs what `read_kept` costs. The texts of the shares chosen on the
    /// restore screen, and those kept in the open vaults, each vault
    /// share's wallet read as a description no record keeps
    /// ([`Kept::read_kept`]).
    pub fn restore_share_texts(&self, c: &Compute) -> Vec<String> {
        let Some(r) = self.restore.state.as_ref() else {
            return Vec::new();
        };
        let mut texts: Vec<String> = self
            .inbox
            .iter()
            .filter(|i| r.shares.contains(&i.name))
            .map(|i| String::from_utf8_lossy(&i.bytes).into_owned())
            .collect();
        // The shares kept in every open vault (`docs/BACKUP-FLOW.md`
        // §15.6), each where it agrees with the shares in hand.
        use faraday_vault::records::{field, kind};
        // One wallet's shares, told by field 5, never by name: the first
        // that merges with what is in hand.
        let mut wallet: Option<String> = None;
        for o in &self.vaults.open {
            for (_, rec) in self.vaults.records(o.id).of(kind::WALLET_SHARE) {
                let Some(t) = rec.public_text(field::SHARE) else {
                    continue;
                };
                let of = rec
                    .public_text(field::SHARE_WALLET)
                    .and_then(|d| self.kept.read_kept(c, d))
                    .map(|p| p.checksum().to_string());
                if wallet.is_some() && of != wallet {
                    continue;
                }
                if texts.iter().any(|x| x.trim() == t.trim()) {
                    continue;
                }
                let mut trial = texts.clone();
                trial.push(t.to_string());
                if restore::merge(&trial).is_ok() {
                    texts = trial;
                    wallet = of;
                }
            }
        }
        texts
    }

    /// Loads the wallet a description gives and fills Restore's Kind and
    /// Quorum from it: the seeds are matched to its keys next. `origin`
    /// is what `source` is, given by the caller, which already knows.
    pub(crate) fn restore_wallet(
        &mut self,
        name: &str,
        text: &str,
        source: &str,
        origin: wallet::Origin,
    ) {
        let c = self.compute();
        match self.session.add_wallet(&c, name, text, source, origin) {
            Ok(i) => {
                self.wallets.set_wallet(i);
                let w = &self.session.wallets[i];
                let kind = create::NewKind::of(w.policy());
                let (m, n) = Session::quorum(w);
                // The flow that decided where the seeds go knows the wallet
                // once it is made from them.
                if let Some(owner) = self
                    .restore
                    .state()
                    .filter(|r| r.into_vaults())
                    .map(|r| r.owner)
                {
                    self.wallet_flow_attach(i, owner);
                }
                if let Some(r) = self.restore.state.as_mut() {
                    r.wallet = Some(i);
                    r.described = true;
                    r.absent.clear();
                    r.error = None;
                    if let Some(s) = r.seeds.as_mut() {
                        s.fix(kind.unwrap_or(s.kind));
                        // A description's keys are matched to the seeds
                        // from none; a wallet made from the seeds keeps
                        // them, its keys.
                        if origin != wallet::Origin::TypedSeeds {
                            s.keys.clear();
                        }
                        s.set_n(n);
                        s.set_m(m);
                    }
                    for k in [rstep::KIND, rstep::QUORUM, rstep::DESCRIPTION] {
                        r.done[k as usize] = true;
                    }
                    r.done[rstep::SEEDS as usize] = false;
                    r.done[rstep::CHECK as usize] = false;
                    // On to the next card not done: the question of vaults
                    // when it is not answered yet, before the seeds
                    // (`docs/history/NEW-WALLET.md` §12.3 item 4).
                    r.next(rstep::DESCRIPTION);
                }
            }
            Err(e) => {
                if let Some(r) = self.restore.state.as_mut() {
                    r.error = Some(e.text());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shares of a split backup rebuild the same wallet as the whole
    /// config, from any quorum of them.
    #[test]
    fn a_quorum_of_shares_rebuilds_the_wallet() {
        let c = crate::testkit::compute();
        let whole = "Name: T\nPolicy: 2 of 3\nFormat: P2WSH\n";
        let keys = [
            (
                "9a6a2580",
                "xpub6EeqK2JLwngrHJEQ4X4iqrySZV9qU3TgwMgf6NStLZa37AfNiHTtTE9ji1F9YQDLArJMLy8sw3Q2samVj5VQQjaaUHr5z2Hz57NWHJCfh31",
            ),
            (
                "5d388376",
                "xpub6EFjRTDekDh2sARTm1vbBFeVFb8srMNspMpgypVTSRnzabCbm5ch3oqiw5K9ah7mo9gLub8yU6ciBnrbFwtrXRJY93CThvb9oqVCK47dBEx",
            ),
            (
                "cf2e083d",
                "xpub6EvbTisSw8RtWaatFV6xEgY1v1TymXwGfu4kn8PfYcdbPthFWK8McYnVQwxF5EfQQeu6bYpgQ9vG3wgSREeeQSkUzvcidGMjujQroTd1VrT",
            ),
        ];
        let share = |which: &[usize]| {
            let mut s = format!("# Partial\n{whole}");
            for &i in which {
                s.push_str(&format!(
                    "\nDerivation: m/48'/0'/0'/2'\n{}: {}\n",
                    keys[i].0, keys[i].1
                ));
            }
            s
        };
        let one = merge(&[share(&[0, 1])]).unwrap();
        assert!(one.whole.is_none(), "one share alone does not rebuild");
        let two = merge(&[share(&[0, 1]), share(&[1, 2])]).unwrap();
        let rebuilt = two.whole.expect("two shares rebuild");
        let a = multisig_config::parse(&c, &rebuilt).unwrap();
        let mut full = whole.to_string();
        for k in keys {
            full.push_str(&format!("\nDerivation: m/48'/0'/0'/2'\n{}: {}\n", k.0, k.1));
        }
        assert_eq!(
            a.to_descriptor(),
            multisig_config::parse(&c, &full).unwrap().to_descriptor()
        );
        assert!(readable(&c, &rebuilt));
    }

    /// A rebuilt config is checked as any other: a key whose SLIP-132
    /// version names another script than the file's (a single-key `zpub`
    /// or a nested `Ypub` in a P2WSH wallet) does not read as a wallet.
    #[test]
    fn a_config_with_a_slip132_key_of_another_script_is_not_readable() {
        let c = crate::testkit::compute();
        use faraday_bip::slip132::{Script, encode};
        let xpub = |i: usize| {
            [
                "xpub6EeqK2JLwngrHJEQ4X4iqrySZV9qU3TgwMgf6NStLZa37AfNiHTtTE9ji1F9YQDLArJMLy8sw3Q2samVj5VQQjaaUHr5z2Hz57NWHJCfh31",
                "xpub6EFjRTDekDh2sARTm1vbBFeVFb8srMNspMpgypVTSRnzabCbm5ch3oqiw5K9ah7mo9gLub8yU6ciBnrbFwtrXRJY93CThvb9oqVCK47dBEx",
            ][i]
        };
        let config = |second: &str| {
            format!(
                "Policy: 2 of 2\nFormat: P2WSH\nDerivation: m/48'/0'/0'/2'\n9a6a2580: {}\n5d388376: {second}\n",
                xpub(0)
            )
        };
        assert!(readable(&c, &config(xpub(1))));
        let zpub_multi = encode(xpub(1), Script::NativeMultisig).unwrap();
        assert!(zpub_multi.starts_with("Zpub"));
        assert!(readable(&c, &config(&zpub_multi)));
        for script in [Script::NativeSegwit, Script::NestedMultisig] {
            let other = encode(xpub(1), script).unwrap();
            assert!(!readable(&c, &config(&other)), "{other}");
        }
    }
}
