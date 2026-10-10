//! A wallet made from the seeds in hand (`docs/WALLETS.md` §2,
//! `docs/FAMILY.md` §4): for someone whose backup is seed words on paper,
//! perhaps several lists and a line like "2 of 3", with no descriptor.
//! Restore's seeds-first route and the Spend tab's words route share it.
//!
//! It holds the seeds typed in this flow, by fingerprint; how many keys
//! the wallet has and how many sign (`m` of `n`); an account key for
//! each cosigner whose seed is not here; the kind; and the path. The
//! seeds' account keys are derived at that path and the descriptor is
//! [`NewKind::descriptor`]'s, the one Create a wallet writes.

use crate::create::{self, NewKind};
use crate::wallet::FileKind;
use crate::{Faraday, Screen, family};

/// The kinds a wallet of one key can be.
pub const SINGLE: [NewKind; 4] = family::SINGLE;

/// The kinds a wallet of several keys can be.
pub const MULTI: [NewKind; 4] = [
    NewKind::Multi,
    NewKind::MultiNested,
    NewKind::MultiLegacy,
    NewKind::TapMulti,
];

/// The most keys a wallet made here has.
pub const MAX_KEYS: usize = 15;

/// The slider of signatures needed.
pub const SLIDE_M: u8 = 0;
/// The slider of keys.
pub const SLIDE_N: u8 = 1;
/// The backup's slider of keys left off each share.
pub const SLIDE_OMIT: u8 = 2;

/// The source a wallet made from seeds is loaded under.
pub const SOURCE: &str = "Typed seeds";

/// Where typing goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// A cosigner's account key box, by its place among the cosigners.
    Cosigner(u8),
    /// The custom path box.
    Path,
}

/// Everything the piece can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedsAction {
    /// On to the wallet's shape: M of N, the kind, the path.
    Shape,
    /// Back to the seeds.
    Keys,
    /// The kind, by its place in [`SeedsState::kinds`].
    Kind(u8),
    /// One account up or down.
    Account(bool),
    /// The kind's standard path.
    Standard,
    /// A path typed in.
    Custom,
    /// Typing goes to this box.
    Focus(Focus),
    /// Empty a cosigner's box.
    Clear(u8),
    /// The account key in this Inbox file, for the first cosigner
    /// without one.
    UseFile(usize),
    /// A seed already loaded, by fingerprint.
    UseLoaded([u8; 4]),
    /// A seed held in an open vault: the vault, the record.
    FromVault(usize, usize),
    /// Make the wallet.
    Make,
}

/// The piece's state.
#[derive(Debug, Default, Clone)]
pub struct SeedsState {
    /// The seeds typed in this flow, by fingerprint, in order.
    pub keys: Vec<[u8; 4]>,
    /// The wallet's shape is shown, after the seeds.
    pub shaping: bool,
    /// Signatures needed.
    pub m: usize,
    /// Keys in the wallet.
    pub n: usize,
    /// The kind.
    pub kind: NewKind,
    /// The account on the kind's standard path.
    pub account: u32,
    /// A path typed in, in place of the standard one.
    pub custom: Option<String>,
    /// What is typed or scanned for each cosigner without a seed here.
    pub cosigners: Vec<String>,
    /// Where typing goes.
    pub focus: Option<Focus>,
    /// The slider the arrow keys move.
    pub slider: u8,
    /// The last refusal.
    pub error: Option<String>,
    /// The kind was chosen first (Restore's Kind card): the number of
    /// keys stays within it, and each slot is a seed or a cosigner's
    /// box (`docs/NEW-WALLET.md` §12.1).
    pub fixed: bool,
}

impl SeedsState {
    /// The kinds a wallet of this many keys can be.
    pub fn kinds(&self) -> &'static [NewKind] {
        if self.n <= 1 { &SINGLE } else { &MULTI }
    }

    /// The path the seeds' keys are taken at.
    pub fn path(&self, network: osk_bip::keys::Network) -> String {
        match &self.custom {
            Some(p) => p.clone(),
            None => self.kind.path_at(network, self.account),
        }
    }

    /// Takes a seed into the flow, once. With the kind fixed it goes in
    /// the first empty slot, and is refused when none is.
    pub fn take(&mut self, fp: [u8; 4]) -> bool {
        if self.keys.contains(&fp) {
            return true;
        }
        if self.fixed {
            let Some(k) = self.first_empty() else {
                return false;
            };
            self.cosigners.remove(k);
            self.focus = None;
            self.keys.push(fp);
            return true;
        }
        self.keys.push(fp);
        self.fit();
        true
    }

    /// Fixes the kind, chosen before the seeds: the shape is open, one
    /// slot for each key.
    pub fn fix(&mut self, kind: NewKind) {
        self.fixed = true;
        self.shaping = true;
        self.kind = kind;
        if self.n == 0 {
            self.n = 1;
            self.m = 1;
        }
        self.set_n(self.n);
    }

    /// The first slot with neither a seed nor anything typed for a
    /// cosigner, by its place among the cosigners' boxes.
    pub fn first_empty(&self) -> Option<usize> {
        self.cosigners.iter().position(|c| c.trim().is_empty())
    }

    /// The fewest keys the shape takes.
    fn least(&self) -> usize {
        let kind = if self.fixed && self.kind.multi() {
            2
        } else {
            1
        };
        self.keys.len().max(kind)
    }

    /// Opens the shape at its defaults: as many keys as seeds, a
    /// majority of them to sign, the usual kind for that many.
    pub fn start_shape(&mut self) {
        self.shaping = true;
        self.n = self.keys.len().max(1);
        self.m = default_m(self.n);
        self.kind = self.kinds()[0];
        self.cosigners.clear();
        self.error = None;
    }

    /// Sets the number of keys: at least the seeds here, at most
    /// [`MAX_KEYS`]. The kind changes with it between one key and
    /// several, and the signatures needed stay within it.
    pub fn set_n(&mut self, n: usize) {
        let n = n.clamp(self.least(), self.kind.max_keys().min(MAX_KEYS));
        let was_multi = self.n > 1;
        if n != self.n {
            // A majority again, unless the person chose otherwise.
            if self.m == default_m(self.n) || self.m > n || self.m == 0 {
                self.m = default_m(n);
            }
        }
        self.n = n;
        if self.fixed {
            if self.kind.all_sign() {
                self.m = n;
            }
        } else if was_multi != (n > 1) || !self.kinds().contains(&self.kind) {
            self.kind = self.kinds()[0];
        }
        self.fit();
    }

    /// Sets the signatures needed, 1 to N; every key for MuSig2.
    pub fn set_m(&mut self, m: usize) {
        self.m = if self.fixed && self.kind.all_sign() {
            self.n
        } else {
            m.clamp(1, self.n.max(1))
        };
    }

    /// The cosigners' boxes, one for each key the seeds do not make.
    fn fit(&mut self) {
        if !self.shaping {
            return;
        }
        if self.fixed {
            // An empty slot goes first, then the last typed.
            let want = self.n.saturating_sub(self.keys.len());
            while self.cosigners.len() < want {
                self.cosigners.push(String::new());
            }
            while self.cosigners.len() > want {
                match self.cosigners.iter().rposition(|c| c.trim().is_empty()) {
                    Some(k) => self.cosigners.remove(k),
                    None => self.cosigners.pop().unwrap_or_default(),
                };
            }
            if matches!(self.focus, Some(Focus::Cosigner(k)) if usize::from(k) >= want) {
                self.focus = None;
            }
            return;
        }
        if self.n < self.keys.len() {
            self.set_n(self.keys.len());
            return;
        }
        let want = self.n - self.keys.len();
        self.cosigners.resize(want, String::new());
        if matches!(self.focus, Some(Focus::Cosigner(k)) if usize::from(k) >= want) {
            self.focus = None;
        }
    }

    /// The first cosigner without a key that reads.
    pub fn open_cosigner(&self) -> Option<usize> {
        self.cosigners
            .iter()
            .position(|c| cosigner_key(c).is_none())
    }

    /// The slider's ends and value.
    pub fn slider_at(&self, id: u8) -> Option<(usize, usize, usize)> {
        if !self.shaping {
            return None;
        }
        match id {
            // MuSig2: every key signs; there is nothing to choose.
            SLIDE_M if self.fixed && self.kind.all_sign() => None,
            SLIDE_M => Some((1, self.n.max(1), self.m)),
            SLIDE_N => Some((self.least(), self.kind.max_keys().min(MAX_KEYS), self.n)),
            _ => None,
        }
    }
}

/// A majority of `n`: 1 of 1, 2 of 2, 2 of 3, 3 of 5.
pub fn default_m(n: usize) -> usize {
    if n <= 1 { 1 } else { n / 2 + 1 }
}

/// A cosigner's account key as typed, pasted or scanned: `[fingerprint/
/// path]xpub` or the xpub alone, in any of its SLIP-132 forms (`zpub`,
/// `Zpub`, `tpub`, `Vpub`, …), with or without a `/<0;1>/*` tail.
/// Returns it as a descriptor writes it.
pub fn cosigner_key(text: &str) -> Option<String> {
    let t: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let t = t.split_once("/<").map_or(t.as_str(), |(a, _)| a);
    let t = t
        .strip_suffix("/0/*")
        .or_else(|| t.strip_suffix("/*"))
        .unwrap_or(t);
    let (origin, xpub) = match t.strip_prefix('[') {
        Some(rest) => {
            let (o, x) = rest.split_once(']')?;
            (Some(o), x)
        }
        None => (None, t),
    };
    let plain = crate::wallet::plain_xpub(xpub)?;
    let text = match origin {
        Some(o) => format!("[{o}]{plain}"),
        None => plain,
    };
    osk_bip::policy::PolicyKey::parse(&text)
        .ok()
        .map(|k| k.key_text())
}

/// The fingerprint a cosigner's key names, or its own identifier's when
/// it has no origin, for showing.
pub fn cosigner_label(key: &str) -> String {
    let short = |x: &str| {
        let n = x.chars().count();
        if n > 20 {
            let head: String = x.chars().take(10).collect();
            let tail: String = x.chars().skip(n - 6).collect();
            format!("{head}…{tail}")
        } else {
            x.to_string()
        }
    };
    match key.strip_prefix('[').and_then(|r| r.split_once(']')) {
        Some((o, x)) => {
            let fp = o.split('/').next().unwrap_or("").to_uppercase();
            format!("{fp} · {}", short(x))
        }
        None => short(key),
    }
}

impl Faraday {
    /// The piece the screen shows: Restore's in seeds-first mode, or the
    /// Spend tab's.
    pub fn seeds(&self) -> Option<&SeedsState> {
        match self.screen {
            Screen::Restore => self.restore.as_ref()?.seeds.as_ref(),
            Screen::Family => Some(&self.family.seeds),
            _ => None,
        }
    }

    fn seeds_mut(&mut self) -> Option<&mut SeedsState> {
        match self.screen {
            Screen::Restore => self.restore.as_mut()?.seeds.as_mut(),
            Screen::Family => Some(&mut self.family.seeds),
            _ => None,
        }
    }

    /// The seeds of the piece still in the session.
    pub fn seeds_here(&self, s: &SeedsState) -> Vec<[u8; 4]> {
        s.keys
            .iter()
            .copied()
            .filter(|fp| self.session_key(*fp).is_some())
            .collect()
    }

    fn session_key(&self, fp: [u8; 4]) -> Option<&crate::wallet::Key> {
        self.session
            .keys
            .iter()
            .find(|k| k.master.fingerprint().0 == fp)
    }

    /// Typing goes to one of the piece's boxes.
    pub(crate) fn seeds_typing(&self) -> bool {
        self.seeds().is_some_and(|s| s.shaping && s.focus.is_some())
    }

    /// A key added from Add a key, or scanned, that `back` returns to: a
    /// seed of Restore's seeds-first route or of the Spend tab's words
    /// route.
    pub(crate) fn seeds_took(&mut self, fp: osk_bip::keys::Fingerprint, back: Option<Screen>) {
        match back {
            Some(Screen::Restore) => self.restore_took(fp),
            Some(Screen::Family) if self.family.route == Some(family::Route::Words) => {
                // A seed typed before this flow kept its list, as the
                // wallet the words route opened from it: it is the first.
                if self.family.seeds.keys.is_empty()
                    && let Some(w) = self
                        .session
                        .wallets
                        .iter()
                        .find(|w| w.source == family::WORDS_SOURCE)
                    && let Some(first) = self.session.slots(w).first().and_then(|s| s.fingerprint)
                    && first != fp
                {
                    self.family.seeds.keys.push(first.0);
                }
                self.family.seeds.take(fp.0);
                if self.family.seeds.keys.len() >= 2 {
                    self.family_several_seeds();
                }
            }
            _ => {}
        }
    }

    /// The words route with two seeds or more: the single-key wallet the
    /// first opened goes, and the wallet's shape comes up.
    fn family_several_seeds(&mut self) {
        let before = self.session.wallets.len();
        self.session
            .wallets
            .retain(|w| w.source != family::WORDS_SOURCE);
        if self.session.wallets.len() != before {
            if self
                .family
                .wallet
                .as_deref()
                .is_some_and(|n| !self.session.wallets.iter().any(|w| w.name == n))
            {
                self.family.wallet = None;
            }
            self.wallet = self
                .wallet
                .min(self.session.wallets.len().saturating_sub(1));
        }
        let s = &mut self.family.seeds;
        if s.shaping {
            s.fit();
        } else {
            s.start_shape();
        }
    }

    /// An account key that arrived in the Inbox while the piece waits for
    /// a cosigner's: it fills the first box without one. Returns whether
    /// it did.
    pub(crate) fn seeds_take_file(&mut self, index: usize) -> bool {
        let Some(text) = self
            .inbox
            .get(index)
            .filter(|i| i.kind == FileKind::Key)
            .map(|i| String::from_utf8_lossy(&i.bytes).into_owned())
        else {
            return false;
        };
        let Some(s) = self.seeds_mut() else {
            return false;
        };
        // The box typing goes to, else the first without a key.
        let focused = match s.focus {
            Some(Focus::Cosigner(k)) => Some(usize::from(k)),
            _ => None,
        };
        let Some(slot) = s
            .shaping
            .then(|| focused.or_else(|| s.open_cosigner()))
            .flatten()
        else {
            return false;
        };
        let Some(key) = create::key_for(s.kind, &text).or_else(|| create::read_key(&text)) else {
            s.error = Some("No account key in that file".to_string());
            return false;
        };
        s.cosigners[slot] = key;
        s.focus = None;
        s.error = None;
        true
    }

    /// The descriptor the piece describes now, or what it still lacks.
    pub fn seeds_descriptor(&self, s: &SeedsState) -> Result<String, String> {
        let net = self.session.network();
        let path = s.path(net);
        let path = create::normal_path(&path)
            .filter(|p| p != "m")
            .ok_or_else(|| "Type a derivation path, like m/48'/0'/0'/2'".to_string())?;
        let mut keys = Vec::new();
        for fp in self.seeds_here(s) {
            let key = self.session_key(fp).ok_or("A seed is no longer loaded")?;
            keys.push(create::key_text_at(&key.master, &path)?);
        }
        if keys.is_empty() {
            return Err("No seed yet".to_string());
        }
        for c in &s.cosigners {
            let at = keys.len() + 1;
            match cosigner_key(c) {
                Some(key) => {
                    // A key of the other network makes no wallet here.
                    if let Ok(k) = osk_bip::policy::PolicyKey::parse(&key) {
                        let kn = crate::wallet::network_of_key(&k, net);
                        if kn.is_mainnet() != net.is_mainnet() {
                            return Err(format!(
                                "Key {at} is a {} key; this session is on {}",
                                crate::wallet::network_name(kn),
                                crate::wallet::network_name(net)
                            ));
                        }
                    }
                    keys.push(key);
                }
                None if c.trim().is_empty() => {
                    return Err(format!("Key {at} has no account key yet"));
                }
                None => return Err(format!("Key {at} is not an account key")),
            }
        }
        if keys.len() != s.n.max(1) {
            return Err(format!("{} of {} keys in hand", keys.len(), s.n));
        }
        let kind = if keys.len() == 1 && s.kind.multi() {
            NewKind::NativeSegwit
        } else {
            s.kind
        };
        let d = kind.descriptor(s.m.max(1), &keys);
        create::build(kind, s.m.max(1), &keys)?;
        Ok(d)
    }

    /// The first receive address of the wallet the piece describes.
    pub fn seeds_address(&self, s: &SeedsState) -> Result<String, String> {
        let d = self.seeds_descriptor(s)?;
        let p = crate::wallet::read_wallet(&d).map_err(|e| e.text())?;
        p.address_at(self.session.network(), false, 0)
            .map(|a| a.to_string())
            .map_err(|e| e.to_string())
    }

    /// A slider pressed, dragged or stepped to `value`.
    pub(crate) fn slide(&mut self, id: u8, value: u8) {
        if id == SLIDE_OMIT {
            self.act(crate::Action::BOmit(usize::from(value)));
            return;
        }
        let Some(s) = self.seeds_mut() else {
            return;
        };
        s.slider = id;
        s.focus = None;
        match id {
            SLIDE_M => s.set_m(usize::from(value)),
            SLIDE_N => s.set_n(usize::from(value)),
            _ => {}
        }
    }

    /// Runs one of the piece's actions.
    pub(crate) fn seeds_act(&mut self, a: SeedsAction) {
        use SeedsAction as S;
        let net = self.session.network();
        if let SeedsAction::Make = a {
            self.seeds_make();
            return;
        }
        if let S::UseLoaded(fp) = a {
            self.seeds_took(osk_bip::keys::Fingerprint(fp), Some(self.screen));
            if self.screen == Screen::Family {
                self.family_key_added();
            }
            return;
        }
        if let S::FromVault(v, record) = a {
            self.seeds_from_vault(v, record);
            return;
        }
        if let S::UseFile(index) = a {
            self.seeds_take_file(index);
            return;
        }
        let Some(s) = self.seeds_mut() else {
            return;
        };
        if !matches!(a, S::Focus(_) | S::Custom) {
            s.focus = None;
        }
        s.error = None;
        match a {
            S::Shape => s.start_shape(),
            S::Keys => s.shaping = false,
            S::Kind(k) => {
                if let Some(&kind) = s.kinds().get(usize::from(k)) {
                    s.kind = kind;
                }
            }
            S::Account(up) => {
                s.custom = None;
                s.account = if up {
                    s.account.saturating_add(1).min(0x7fff_ffff)
                } else {
                    s.account.saturating_sub(1)
                };
            }
            S::Standard => s.custom = None,
            S::Custom => {
                if s.custom.is_none() {
                    s.custom = Some(s.kind.path_at(net, s.account));
                }
                s.focus = Some(Focus::Path);
            }
            S::Focus(f) => s.focus = Some(f),
            S::Clear(k) => {
                if let Some(c) = s.cosigners.get_mut(usize::from(k)) {
                    c.clear();
                    s.focus = Some(Focus::Cosigner(k));
                }
            }
            S::Make | S::UseLoaded(_) | S::FromVault(..) | S::UseFile(_) => {}
        }
    }

    /// Loads a seed from an open vault into the flow.
    fn seeds_from_vault(&mut self, v: usize, record: usize) {
        let Some(want) = self
            .vaults
            .open
            .get(v)
            .and_then(|o| {
                o.contents
                    .of(faraday_vault::records::kind::KEY)
                    .find(|(i, _)| *i == record)
            })
            .and_then(|(_, r)| crate::vault_screens::key_fingerprint(self, r))
        else {
            return;
        };
        let (n, _) = self.vault_load_set(v, &std::collections::BTreeSet::from([record]));
        let Some(fp) = self
            .session
            .keys
            .iter()
            .map(|k| k.master.fingerprint())
            .find(|f| crate::wallet::fp_text(*f) == want)
        else {
            return;
        };
        if n > 0 {
            self.toast(&format!("{want} loaded from {}", self.vaults.open[v].name));
        }
        self.seeds_took(fp, Some(self.screen));
        if self.screen == Screen::Family {
            self.family_key_added();
        }
    }

    /// Makes the wallet the piece describes and carries on: Restore to
    /// its Check card, the Spend tab to its next page.
    fn seeds_make(&mut self) {
        if self.screen == Screen::Restore && self.restore.as_ref().is_some_and(|r| r.described) {
            self.restore_described_make();
            return;
        }
        let Some(s) = self.seeds().cloned() else {
            return;
        };
        let d = match self.seeds_descriptor(&s) {
            Ok(d) => d,
            Err(e) => {
                if let Some(s) = self.seeds_mut() {
                    s.error = Some(e);
                }
                return;
            }
        };
        let n = s.n.max(1);
        // Named for where it came from; the shape is shown beside it.
        let base = if n == 1 { "Typed seed" } else { "Typed seeds" };
        let taken = |x: &str| self.session.wallets.iter().any(|w| w.name == x);
        let name = std::iter::once(base.to_string())
            .chain((2..).map(|k| format!("{base} {k}")))
            .find(|x| !taken(x))
            .unwrap_or_else(|| base.to_string());
        let name = name.as_str();
        match self.screen {
            Screen::Restore => {
                self.restore_wallet(name, &d, SOURCE);
                if let Some(r) = self.restore.as_mut()
                    && r.wallet.is_some()
                {
                    // Made from the seeds, not read from a description.
                    r.described = false;
                    r.next(crate::rstep::SEEDS);
                }
                self.refresh_spend();
            }
            Screen::Family => match self.session.add_wallet(name, &d, SOURCE) {
                Ok(i) => {
                    self.family.wallet = Some(self.session.wallets[i].name.clone());
                    self.wallet = i;
                    self.refresh_spend();
                    self.family_opened();
                    self.family_settle();
                }
                Err(e) => self.family.seeds.error = Some(e.text()),
            },
            _ => {}
        }
    }

    /// Typing into the piece's boxes, and the arrow keys on its slider.
    /// Returns whether the key was taken.
    pub(crate) fn seeds_key(&mut self, key: osk_shell_api::Key) -> bool {
        use osk_shell_api::Key as K;
        let Some(s) = self.seeds() else {
            return false;
        };
        if !s.shaping {
            return false;
        }
        if s.focus.is_none() {
            let id = s.slider;
            let step = match key {
                K::Left => -1,
                K::Right => 1,
                _ => return false,
            };
            let Some((lo, hi, v)) = s.slider_at(id) else {
                return false;
            };
            let to = (v as i64 + step).clamp(lo as i64, hi as i64) as u8;
            self.slide(id, to);
            return true;
        }
        let Some(s) = self.seeds_mut() else {
            return false;
        };
        let Some(focus) = s.focus else {
            return false;
        };
        let field = match focus {
            Focus::Cosigner(k) => s.cosigners.get_mut(usize::from(k)),
            Focus::Path => s.custom.as_mut(),
        };
        let Some(field) = field else {
            s.focus = None;
            return false;
        };
        match key {
            K::Char(c) if !c.is_control() && field.chars().count() < 400 => field.push(c),
            K::Backspace => {
                field.pop();
            }
            K::Enter | K::Escape | K::Tab => s.focus = None,
            _ => {}
        }
        s.error = None;
        true
    }
}

/// The keys open vaults hold that are not loaded: the vault, the record
/// and the key's fingerprint as text.
pub(crate) fn vault_seeds(app: &Faraday) -> Vec<(usize, usize, String)> {
    let mut out = Vec::new();
    for (v, open) in app.vaults.open.iter().enumerate() {
        for (i, r) in open.contents.of(faraday_vault::records::kind::KEY) {
            if let Some(fp) = crate::vault_screens::key_fingerprint(app, r)
                && !app
                    .session
                    .keys
                    .iter()
                    .any(|k| crate::wallet::fp_text(k.master.fingerprint()) == fp)
            {
                out.push((v, i, fp));
            }
        }
    }
    out
}
