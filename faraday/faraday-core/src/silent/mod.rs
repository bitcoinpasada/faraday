//! Silent payments (BIP-352), the receiving side: a loaded key's
//! `sp1…` address and its labelled ones, the public record a coordinator
//! keeps, the scan key a scanner needs, and a silent payments wallet among
//! the session's wallets, with a check of whether a transaction pays it.
//! Every computation is `faraday-bip::silent`'s and
//! `silent_wallet`'s. The address
//! and the record are public; the scan key shows every payment to the
//! address, so it is a secret: into the vault, or out only through the
//! secret sheet. Nothing here is kept; the keys are derived when used.

pub mod check;
pub(crate) mod screen;

use crate::strings::EN;
use faraday_bip::policy::{WalletId, WalletPolicy};
use faraday_bip::silent_wallet::{MAX_LABELS, SilentWallet};
use faraday_crypto::work::Compute;
use faraday_keys::silent::Receiver;
use faraday_keys::vault::{NotSet, VaultId};
use faraday_text::{Text, text};
use faraday_vault::Record;
use faraday_vault::records::{field, kind};
use std::hash::{Hash, Hasher};
use zeroize::Zeroizing;

use crate::wallet::{Session, fp_text};
use crate::{Faraday, QrFormat, QrSource, QrView, Screen, flow};

/// The cards of the flow.
pub mod sstep {
    /// Which loaded key.
    pub const KEY: u8 = 0;
    /// The address, and a label.
    pub const ADDRESS: u8 = 1;
    /// The scan key.
    pub const SCAN: u8 = 2;
    /// Whether a transaction pays the wallet.
    pub const CHECK: u8 = 3;
    /// How many cards.
    pub const COUNT: usize = 4;
}

/// The flow's state: a key and a label number. None of it is a secret.
pub struct SilentState {
    /// The open card.
    pub open: Option<u8>,
    /// Cards closed as done.
    pub done: [bool; sstep::COUNT],
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// The key, by its fingerprint.
    pub key: Option<[u8; 4]>,
    /// The label shown: 0 for none, else 1 to [`MAX_LABELS`].
    pub label: u32,
    /// Labels handed out so far, for the record.
    pub labels: u32,
    /// The session wallet this is the page of, once there is one.
    pub wallet: Option<usize>,
    /// A transaction being checked: the file it came from, and the check
    /// with its previous outputs as far as Files has them.
    pub check: Option<(Text, crate::silent::check::Check)>,
    /// Where the flow returns to.
    pub back: Screen,
    /// The name typed for the address's DNS record.
    pub dns_name: Text,
    /// The domain typed for the address's DNS record.
    pub dns_domain: Text,
    /// Which of the two fields typing goes to: `Some(false)` the name,
    /// `Some(true)` the domain.
    pub dns_focus: Option<bool>,
    /// What the page shows of the key, kept by what it is derived from
    /// ([`Faraday::silent_shown`]). Public: no key is kept.
    pub(crate) shown: std::cell::RefCell<Option<(SilentShownOf, SilentShown)>>,
}

/// The labels handed out in this session, by silent payments wallet,
/// whatever page, key or vault is open. The [`Session`] holds it, so
/// only [`Session::wipe`] ends it: Lock and power off. Public: a record
/// is two public keys, where they came from and a count.
#[derive(Default)]
pub(crate) struct HandedOut(Vec<Handed>);

/// One wallet's count.
struct Handed {
    /// [`crate::wallet::same_wallet`] of `policy`, worked out once.
    same: Text,
    /// The record, its `labels` the most handed out.
    policy: WalletPolicy,
}

impl Handed {
    /// Whether this is the wallet of `same` and `id`. A comparison.
    fn is(&self, same: &str, id: WalletId) -> bool {
        *self.same == *same && self.policy.id() == id
    }

    fn labels(&self) -> u32 {
        self.policy.silent().map_or(0, |r| r.labels)
    }
}

impl HandedOut {
    /// Costs writing `record` out. Its wallet has handed out as many
    /// labels as it states; the count kept is never lowered.
    fn keep(&mut self, record: SilentWallet) {
        let count = record.labels;
        if count == 0 {
            return;
        }
        let policy = WalletPolicy::of_silent(record);
        let same = crate::wallet::same_wallet(&policy);
        match self.0.iter_mut().find(|h| h.is(&same, policy.id())) {
            Some(h) if h.labels() < count => h.policy = policy,
            Some(_) => {}
            None => self.0.push(Handed { same, policy }),
        }
    }

    /// The count kept for the wallet of `same` and `id`, or none. A
    /// lookup.
    fn of(&self, same: &str, id: WalletId) -> u32 {
        let kept = self.0.iter().find(|h| h.is(same, id));
        kept.map_or(0, Handed::labels)
    }

    /// Costs a lookup of each wallet, and for each one raised a copy of its
    /// record, a seal of its policy and, at its next
    /// [`crate::wallet::Wallet::same`], a write of its descriptor. Each of
    /// `wallets` with fewer labels than were handed out is raised to that
    /// count; nothing else of its record changes.
    fn raise(&self, wallets: &mut [crate::wallet::Wallet]) {
        if self.0.is_empty() {
            return;
        }
        for w in wallets {
            if w.policy().silent().is_none() {
                continue;
            }
            let count = self.of(w.same(), w.id());
            if let Some(mut held) = w.policy().silent().cloned()
                && held.labels < count
            {
                held.labels = count;
                w.set_policy(WalletPolicy::of_silent(held));
            }
        }
    }

    /// Whether no label was handed out.
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The record of a key's page: what its key `fp` gives at account 0.
fn account_zero(r: &Receiver, fp: [u8; 4], labels: u32) -> SilentWallet {
    SilentWallet {
        network: r.network(),
        fingerprint: faraday_bip::keys::Fingerprint(fp),
        account: 0,
        scan: r.scan_public_key(),
        spend: r.spend_public_key(),
        labels,
    }
}

/// Costs two key derivations. The wallet a page of loaded key `fp` is of.
fn own_record(c: &Compute, session: &Session, fp: [u8; 4]) -> Option<WalletPolicy> {
    let place = session.keys.position_of(fp)?;
    let r = session.keys.silent_receiver(c, place, 0)?;
    Some(WalletPolicy::of_silent(account_zero(&r, fp, 0)))
}

/// Costs writing `own` out. The session's wallet that is the silent
/// payments wallet `own`: not another account of its key, nor a record
/// that only names its fingerprint.
fn wallet_of(session: &Session, own: &WalletPolicy) -> Option<usize> {
    let same = crate::wallet::same_wallet(own);
    session
        .wallets
        .iter()
        .position(|w| w.policy().silent().is_some() && w.same() == &*same && w.id() == own.id())
}

/// What the DNS record of the address on show comes to, from the name and
/// domain typed.
pub(crate) struct Dns {
    /// The name is typed and is not a DNS label.
    pub name_bad: bool,
    /// The domain is typed and is not a DNS name.
    pub domain_bad: bool,
    /// The human-readable name and the record line, when both fields are
    /// filled and valid.
    pub record: Option<(Text, Text)>,
}

impl SilentState {
    /// The DNS record of `address` for the name and domain typed. Only
    /// string formatting.
    pub(crate) fn dns(&self, address: &str) -> Dns {
        use faraday_bip::silent::dns_record;
        let (n, d) = (self.dns_name.as_str(), self.dns_domain.as_str());
        let name_bad = !n.is_empty() && dns_record(n, "a", address).is_err();
        let domain_bad = !d.is_empty() && dns_record("a", d, address).is_err();
        let record = if n.is_empty() || d.is_empty() {
            None
        } else {
            dns_record(n, d, address)
                .ok()
                .map(|r| (text!("\u{20bf}{n}@{d}"), Text::of(r.as_str())))
        };
        Dns {
            name_bad,
            // Each fine alone but too long together: the domain is the
            // one to shorten.
            domain_bad: domain_bad
                || (!name_bad && !n.is_empty() && !d.is_empty() && record.is_none()),
            record,
        }
    }
}

/// What the Silent page shows that is derived from its key: whether the
/// wallet's key is loaded, the address on show, and how the wallet's
/// record stands against the loaded keys. Public.
#[derive(Clone, Default)]
pub struct SilentShown {
    /// Whether the wallet's key is loaded and gives the wallet's two
    /// points ([`Faraday::silent_key_here`]).
    pub key_here: bool,
    /// The address on show ([`Faraday::silent_address`]).
    pub address: Option<Text>,
    /// How the record of the wallet the page is of stands against the
    /// loaded keys; `None` on a key's page with no wallet, whose address
    /// is derived from the key.
    pub check: Option<RecordCheck>,
}

/// What [`SilentShown`] is derived from: the key, the label, the wallet
/// and its record, the ids of the loaded keys, and the network.
pub(crate) type SilentShownOf = (
    Option<[u8; 4]>,
    u32,
    Option<usize>,
    Option<SilentWallet>,
    Vec<[u8; 32]>,
    faraday_bip::keys::Network,
);

/// How a silent payments wallet's record stands against the loaded keys:
/// both of its points, the scan key and the spend key, against what a
/// loaded key gives: the record's own fingerprint's key at its account, or
/// any other loaded key's points named under another fingerprint. Public.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RecordCheck {
    /// A loaded key with the record's fingerprint gives both of its
    /// points at its account: its address is that key's, derived here.
    Matches,
    /// No key with the record's fingerprint is loaded that could check
    /// it: its address is as the record states it, not checked.
    #[default]
    Unchecked,
    /// A key with the record's fingerprint is loaded and gives another
    /// scan key or spend key at the record's account, or a key of another
    /// fingerprint gives both of its points: the record has no address
    /// here.
    Differs,
}

/// Costs two key derivations per loaded key with the record's
/// fingerprint; where none of them matches and none differs (no loaded
/// key has the fingerprint), two per loaded key of another fingerprint,
/// four at an account other than 0 (the record's account and account 0).
/// Worked out once per change of the loaded keys and kept after
/// (`kept`'s silent card, one per session wallet at least), never per
/// frame. Record `w` checked against
/// the loaded keys, with the receiver of the key that gives both of its
/// points and that key's place, when one does. A key of another
/// fingerprint that gives both of its points, at its account or at
/// account 0, makes it differ: it names a loaded key's points under a
/// fingerprint that is not the key's. A match under the record's own
/// fingerprint wins.
pub(crate) fn check_record(
    c: &Compute,
    keys: &faraday_keys::Keys,
    w: &SilentWallet,
) -> (RecordCheck, Option<(Receiver, usize)>) {
    let mut differs = false;
    let mut own = false;
    for place in 0..keys.len() {
        if keys[place].fingerprint != w.fingerprint {
            continue;
        }
        own = true;
        let Some(r) = keys.silent_receiver(c, place, w.account) else {
            continue;
        };
        if r.scan_public_key() == w.scan && r.spend_public_key() == w.spend {
            return (RecordCheck::Matches, Some((r, place)));
        }
        differs = true;
    }
    if !own {
        differs = other_key_gives(c, keys, w).is_some();
    }
    let check = if differs {
        RecordCheck::Differs
    } else {
        RecordCheck::Unchecked
    };
    (check, None)
}

/// Costs two key derivations per loaded key (four at an account other
/// than 0). The fingerprint of the first loaded key that gives both of
/// the record's points, at its account or at account 0, while the record
/// states a fingerprint of its own that no loaded key has.
pub(crate) fn other_key_gives(
    c: &Compute,
    keys: &faraday_keys::Keys,
    w: &SilentWallet,
) -> Option<faraday_bip::keys::Fingerprint> {
    if keys.all().any(|k| k.fingerprint == w.fingerprint) {
        return None;
    }
    let gives = |place: usize, account: u32| {
        keys.silent_receiver(c, place, account)
            .is_some_and(|r| r.scan_public_key() == w.scan && r.spend_public_key() == w.spend)
    };
    (0..keys.len())
        .find(|&place| gives(place, w.account) || (w.account != 0 && gives(place, 0)))
        .map(|place| keys[place].fingerprint)
}

/// The line for a record that differs: key `fp` is the one the record
/// names; where it is not loaded, the line says so, since another loaded
/// key is what gives the record's points.
pub(crate) fn differs_line(keys: &faraday_keys::Keys, fp: [u8; 4]) -> Text {
    let f = faraday_bip::keys::Fingerprint(fp);
    let named = fp_text(f);
    if keys.all().any(|k| k.fingerprint == f) {
        Text::fill(EN.silent_record_differs, &[&named])
    } else {
        Text::fill(EN.silent_record_differs_unloaded, &[&named])
    }
}

/// The ids of every loaded key: what a record is checked against (a key
/// of another fingerprint can give its points).
pub(crate) fn ids_of(keys: &faraday_keys::Keys) -> Vec<[u8; 32]> {
    keys.all().map(|k| k.id.0).collect()
}

/// The Silent payments flow's area: its state while it is open.
#[derive(Default)]
pub(crate) struct SilentArea {
    state: Option<SilentState>,
    /// A sign of the open vaults, the session's label counts and those
    /// handed out, as
    /// [`Faraday::silent_vaults_catch_up`] last saw them. Public.
    followed: Option<u64>,
    /// The refused writes of a label count already told of
    /// ([`Faraday::silent_records_follow`]), hashed. Public.
    told: Vec<u64>,
    /// Each write a vault refused as more than it can hold, or as
    /// read-only: the vault and wallet, hashed, and the vault's version
    /// ([`faraday_keys::vault::OpenVaults::version`]) then. Public.
    full: Vec<(u64, u64)>,
}

/// Holds nothing a lock wipes: a fingerprint and the DNS record's text.
impl crate::at_lock::Public for SilentArea {}

impl SilentArea {
    /// Wallet `gone` left the session list: the place kept moves up. The
    /// page of the one gone closes (the state is forgotten, where it
    /// returns to is given): the key's own page is another page.
    pub(crate) fn wallet_left(&mut self, gone: usize) -> Option<Screen> {
        let s = self.state.as_mut()?;
        let w = s.wallet?;
        match crate::wallet::place_after_removal(w, gone) {
            Some(w) => {
                s.wallet = Some(w);
                None
            }
            None => {
                let back = s.back;
                self.state = None;
                Some(back)
            }
        }
    }

    /// The flow's state, while it is open.
    pub(crate) fn state(&self) -> Option<&SilentState> {
        self.state.as_ref()
    }

    /// Typing goes to the DNS record's domain (`true`) or name (`false`),
    /// while the flow is open.
    pub(crate) fn focus_dns(&mut self, domain: bool) {
        if let Some(s) = self.state.as_mut() {
            s.dns_focus = Some(domain);
        }
    }

    /// Forgets the flow's state.
    pub(crate) fn clear(&mut self) {
        self.state = None;
    }

    /// A key typed into the DNS record's name or domain, while the
    /// address card is open and one of them has focus. Returns whether
    /// the key was taken.
    pub(crate) fn key(&mut self, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key as K;
        let Some(s) = self.state.as_mut() else {
            return false;
        };
        let Some(domain) = s.dns_focus.filter(|_| s.open == Some(sstep::ADDRESS)) else {
            return false;
        };
        let text = if domain {
            &mut s.dns_domain
        } else {
            &mut s.dns_name
        };
        match key {
            K::Char(c) if !c.is_control() => {
                if text.chars().count() < 253 {
                    text.push(c);
                }
            }
            K::Backspace => {
                text.pop();
            }
            _ => return false,
        }
        true
    }

    /// How far the column is scrolled, units, while it is open.
    pub(crate) fn scroll_y(&self) -> Option<f32> {
        self.state.as_ref().map(|x| x.scroll.y)
    }

    /// The column scrolled to `y` units, while it is open.
    pub(crate) fn set_scroll_y(&mut self, y: f32) {
        if let Some(x) = self.state.as_mut() {
            x.scroll.y = y;
        }
    }

    /// Takes the column's scroll as the pass left it.
    pub(crate) fn set_scroll(&mut self, to: flow::Scroll) {
        if let Some(s) = self.state.as_mut() {
            s.scroll = to;
        }
    }

    /// Opens the flow over the session's keys, to return to `back`.
    pub(crate) fn open(&mut self, session: &Session, back: Screen) {
        let key = (session.keys.len() == 1).then(|| session.keys[0].fingerprint.0);
        self.open_with(back, key);
    }

    /// Opens the flow over the loaded key with fingerprint `fp`, to return
    /// to `back`: its address at account 0.
    pub(crate) fn open_for(&mut self, back: Screen, fp: [u8; 4]) {
        self.open_with(back, Some(fp));
    }

    fn open_with(&mut self, back: Screen, key: Option<[u8; 4]>) {
        self.state = Some(SilentState {
            open: Some(if key.is_some() {
                sstep::ADDRESS
            } else {
                sstep::KEY
            }),
            done: [key.is_some(), key.is_some(), false, false],
            scroll: flow::Scroll::default(),
            key,
            label: 0,
            labels: 0,
            wallet: None,
            check: None,
            back,
            dns_name: Text::new(),
            dns_domain: Text::new(),
            dns_focus: None,
            shown: Default::default(),
        });
    }

    /// The page's count raised to `most`, and the page made that of
    /// session wallet `link`, where there is one.
    fn start_from(&mut self, most: u32, link: Option<usize>) {
        if let Some(s) = self.state.as_mut() {
            s.labels = s.labels.max(most);
            s.wallet = link.or(s.wallet);
        }
    }

    /// Opens the flow as the page of session wallet `i`, a silent
    /// payments wallet: its address shows whether or not its key is
    /// loaded; its labels, its scan key and a check need the key. Returns
    /// whether it opened.
    pub(crate) fn open_wallet(&mut self, session: &Session, i: usize) -> bool {
        let Some(r) = session.wallets.get(i).and_then(|w| w.policy().silent()) else {
            return false;
        };
        self.state = Some(SilentState {
            open: Some(sstep::ADDRESS),
            done: [true, true, false, false],
            scroll: flow::Scroll::default(),
            key: Some(r.fingerprint.0),
            label: 0,
            labels: r.labels,
            wallet: Some(i),
            check: None,
            back: Screen::Wallets,
            dns_name: Text::new(),
            dns_domain: Text::new(),
            dns_focus: None,
            shown: Default::default(),
        });
        true
    }

    /// The fingerprint the page names: the record's on a wallet's page
    /// (a key pressed on the Key step is not the record's and never
    /// names its address), the chosen key's on a key's page.
    pub(crate) fn named(&self, session: &Session) -> Option<[u8; 4]> {
        match self.wallet(session) {
            Some(w) => Some(w.fingerprint.0),
            None => self.state.as_ref()?.key,
        }
    }

    /// The record of the wallet the flow is the page of.
    pub(crate) fn wallet<'a>(&self, session: &'a Session) -> Option<&'a SilentWallet> {
        let i = self.state.as_ref()?.wallet?;
        session.wallets.get(i)?.policy().silent()
    }

    /// Costs what [`check_record`] costs on a wallet's page (a derivation
    /// per loaded key, four at an account other than 0), two on a key's
    /// page. How the record of the wallet the page is of stands
    /// against the loaded keys (`None` on a key's page with no wallet),
    /// and the scan and spend keys derived from the loaded key with the
    /// key's fingerprint. On a wallet's page they are derived at the
    /// record's account and only from a key that gives both of the
    /// record's points (a fingerprint is four bytes; the wallet is the two
    /// points); on a key's page, at account 0.
    fn derived(
        &self,
        c: &Compute,
        session: &Session,
    ) -> (Option<RecordCheck>, Option<(Receiver, [u8; 4])>) {
        let Some(s) = self.state.as_ref() else {
            return (None, None);
        };
        if let Some(w) = self.wallet(session) {
            // The record's own key checks it, whichever key the page has
            // pressed.
            let (check, r) = check_record(c, &session.keys, w);
            let r = r.map(|(r, place)| (r, session.keys[place].fingerprint.0));
            return (Some(check), r);
        }
        let r = s.key.and_then(|fp| {
            let place = session.keys.position_of(fp)?;
            let r = session.keys.silent_receiver(c, place, 0)?;
            Some((r, session.keys[place].fingerprint.0))
        });
        (None, r)
    }

    /// Costs what `derived` costs. The scan and spend keys, derived from
    /// the loaded key: `None` while that key is not loaded, or where it
    /// does not give both of the wallet's points at its account.
    fn receiver(&self, c: &Compute, session: &Session) -> Option<(Receiver, [u8; 4])> {
        self.derived(c, session).1
    }

    /// The address on show from what [`SilentArea::derived`] gives: the
    /// one derived from the key wherever the key gives it; with no key to
    /// check it, the record's own unlabelled one, which the page marks
    /// unchecked; none where the record differs from the key.
    fn address_of(
        &self,
        session: &Session,
        check: Option<RecordCheck>,
        r: Option<&(Receiver, [u8; 4])>,
    ) -> Option<Text> {
        let s = self.state.as_ref()?;
        if let Some((r, _)) = r {
            return r.address(s.label);
        }
        match check {
            Some(RecordCheck::Unchecked) if s.label == 0 => {
                self.wallet(session).map(|w| Text::of(&w.address()))
            }
            _ => None,
        }
    }

    /// What the page shows of its key as it was last worked out, when that
    /// was from what stands now; `None` before. A lookup.
    pub(crate) fn shown_kept(&self, session: &Session) -> Option<SilentShown> {
        let s = self.state.as_ref()?;
        let loaded = ids_of(&session.keys);
        let record = self.wallet(session).cloned();
        let of = (s.key, s.label, s.wallet, record, loaded, session.network());
        let kept = s.shown.borrow();
        kept.as_ref()
            .filter(|(o, _)| *o == of)
            .map(|(_, v)| v.clone())
    }

    /// Costs two key derivations and an address the first time. What the
    /// page shows of its key ([`SilentShown`]), kept on the flow's state by
    /// what it is derived from, by whoever asks first, a frame included: a
    /// lookup after.
    pub(crate) fn shown(&self, c: &Compute, session: &Session) -> SilentShown {
        let Some(s) = self.state.as_ref() else {
            return SilentShown::default();
        };
        let loaded = ids_of(&session.keys);
        let record = self.wallet(session).cloned();
        let of = (s.key, s.label, s.wallet, record, loaded, session.network());
        if let Some((o, shown)) = s.shown.borrow().as_ref()
            && *o == of
        {
            return shown.clone();
        }
        let (check, r) = self.derived(c, session);
        let shown = SilentShown {
            key_here: r.is_some(),
            address: self.address_of(session, check, r.as_ref()),
            check,
        };
        *s.shown.borrow_mut() = Some((of, shown.clone()));
        shown
    }

    /// Costs what [`check_record`] costs: whether the wallet's key is loaded and
    /// gives both of its points, which labels, the scan key and a check
    /// need.
    pub(crate) fn key_here(&self, c: &Compute, session: &Session) -> bool {
        self.receiver(c, session).is_some()
    }

    /// Costs two key derivations and an address: the address on show,
    /// unlabelled or with the chosen label, derived from the loaded key
    /// that gives both of the wallet's points. With no such key loaded, a
    /// wallet's own unlabelled address as its record states it, which
    /// [`SilentShown::check`] marks unchecked; none where a loaded key
    /// contradicts the record (the fingerprint's key gives other points, or
    /// another key gives them).
    pub(crate) fn address(&self, c: &Compute, session: &Session) -> Option<Text> {
        let (check, r) = self.derived(c, session);
        self.address_of(session, check, r.as_ref())
    }

    /// Costs what `derived` costs. The public record a coordinator keeps:
    /// the network, the key's origin, the labels handed out and the
    /// address. A wallet's record as it is, checked or not; none where a
    /// loaded key contradicts the record (the fingerprint's key gives other
    /// points, or another key gives them).
    pub(crate) fn record(&self, c: &Compute, session: &Session) -> Option<Text> {
        if let Some(w) = self.wallet(session) {
            return match self.derived(c, session).0 {
                Some(RecordCheck::Differs) => None,
                _ => {
                    let mut w = w.clone();
                    w.labels = self.labels_out(session);
                    Some(Text::of(&w.to_text()))
                }
            };
        }
        let s = self.state.as_ref()?;
        let (r, fp) = self.receiver(c, session)?;
        Some(Text::of(
            &account_zero(&r, fp, s.labels.max(s.label)).to_text(),
        ))
    }

    /// Costs what `address`, `record` and `of` cost. The QR view of the
    /// address on show (as it is, or as a `bitcoin:` link) or of the
    /// record, with the name and label of its picture.
    pub(crate) fn code(
        &self,
        c: &Compute,
        session: &Session,
        code: crate::Code,
    ) -> Result<QrView, Text> {
        let s = self
            .state
            .as_ref()
            .ok_or_else(|| Text::of(EN.silent_not_open))?;
        let fp = fp_text(faraday_bip::keys::Fingerprint(
            self.named(session)
                .ok_or_else(|| Text::of(EN.silent_choose_a_key))?,
        ));
        let mut lines = vec![Text::fill(EN.silent_silent_payment, &[&text!("{}", &*fp)])];
        let check = self.derived(c, session).0;
        if check == Some(RecordCheck::Differs) {
            let named = self.named(session).unwrap_or_default();
            return Err(differs_line(&session.keys, named));
        }
        let (title, text, name) = match code {
            crate::Code::Silent(uri) => {
                let a = self
                    .address(c, session)
                    .ok_or_else(|| Text::of(EN.silent_no_address))?;
                let mut name = text!("silent-{}", &*fp);
                if s.label > 0 {
                    lines.push(Text::fill(
                        EN.silent_label,
                        &[&Text::num(u64::from(s.label))],
                    ));
                    name.push_str(&text!("-label-{}", s.label));
                }
                let text = if uri {
                    name.push_str("-link");
                    faraday_bip::silent::uri(&a)
                        .map(|u| Text::of(u.as_str()))
                        .unwrap_or(a)
                } else {
                    a
                };
                lines.push(Text::of(EN.silent_public_receives));
                if check == Some(RecordCheck::Unchecked) {
                    lines.push(Text::fill(EN.silent_not_checked, &[&text!("{}", &*fp)]));
                }
                (EN.silent_silent_payments_address, text, name)
            }
            crate::Code::SilentDns => {
                let a = self
                    .address(c, session)
                    .ok_or_else(|| Text::of(EN.silent_no_address))?;
                let (_, text) = s
                    .dns(&a)
                    .record
                    .ok_or_else(|| Text::of(EN.silent_no_dns_record))?;
                let mut name = text!("silent-{}", &*fp);
                if s.label > 0 {
                    lines.push(Text::fill(
                        EN.silent_label,
                        &[&Text::num(u64::from(s.label))],
                    ));
                    name.push_str(&text!("-label-{}", s.label));
                }
                name.push_str("-dns");
                lines.push(Text::of(EN.silent_public_receives));
                if check == Some(RecordCheck::Unchecked) {
                    lines.push(Text::fill(EN.silent_not_checked, &[&text!("{}", &*fp)]));
                }
                (EN.silent_silent_payments_dns_record, text, name)
            }
            _ => {
                let text = self
                    .record(c, session)
                    .ok_or_else(|| Text::of(EN.silent_no_record))?;
                lines.push(Text::of(EN.silent_public_watch));
                if check == Some(RecordCheck::Unchecked) {
                    lines.push(Text::fill(EN.silent_not_checked, &[&text!("{}", &*fp)]));
                }
                (
                    EN.silent_silent_payments_record,
                    text,
                    text!("silent-{}-record", &*fp),
                )
            }
        };
        Ok(QrView::of(
            c,
            title,
            QrSource::Text(text.duplicate()),
            QrFormat::Ur,
            crate::QR_PARTS[1],
        )?
        .public(&text!("{}.png", &*name), lines))
    }

    /// A press that changes only this flow: a card, the key, the label.
    /// Returns the silent payments record a label handed out makes the
    /// session wallet `.0` carry, for the caller to put there.
    pub(crate) fn act(
        &mut self,
        action: crate::Action,
        session: &Session,
    ) -> Option<(usize, SilentWallet)> {
        use crate::Action as A;
        let wallet = self.state.as_ref().and_then(|s| s.wallet);
        let record = self.wallet(session).cloned();
        let s = self.state.as_mut()?;
        match action {
            A::SStep(k) => {
                // A wallet's page opens any card, and so does a chosen key;
                // before one, only the key's card opens. A step is always
                // open: a press on the open one does nothing.
                if s.open != Some(k) && (s.wallet.is_some() || s.key.is_some() || k == sstep::KEY) {
                    s.open = Some(k);
                    s.scroll.follow = true;
                }
            }
            A::SKey(fp) => {
                // On a key's page, another key starts from its own count.
                // A wallet's page stays that wallet's.
                if s.wallet.is_none() && s.key != Some(fp) {
                    s.label = 0;
                    s.labels = 0;
                }
                s.key = Some(fp);
                s.check = None;
                s.done[usize::from(sstep::KEY)] = true;
                s.done[usize::from(sstep::ADDRESS)] = true;
                s.open = Some(sstep::ADDRESS);
                s.scroll.follow = true;
            }
            A::SLabel(d) => {
                // From no label, "+" is the first label not yet handed
                // out; at the most there is, the last.
                let next = if s.label == 0 && d > 0 {
                    let held = record.as_ref().map_or(0, |r| r.labels);
                    i64::from(s.labels.max(held)) + 1
                } else {
                    i64::from(s.label) + i64::from(d)
                };
                s.label = next.clamp(0, i64::from(MAX_LABELS)) as u32;
                s.labels = s.labels.max(s.label);
                // A label shown is a label handed out: the wallet's record
                // lists it from now on.
                if let (Some(i), Some(mut r)) = (wallet, record)
                    && s.labels > r.labels
                {
                    r.labels = s.labels;
                    return Some((i, r));
                }
            }
            A::SAnother => {
                let back = s.back;
                self.open(session, back);
            }
            _ => {}
        }
        None
    }

    /// The session wallet `i` is now the flow's page.
    pub(crate) fn set_wallet(&mut self, i: usize) {
        if let Some(s) = self.state.as_mut() {
            s.wallet = Some(i);
        }
    }

    /// The transaction checked, with the file it came from.
    pub(crate) fn set_check(&mut self, name: Text, check: crate::silent::check::Check) {
        if let Some(s) = self.state.as_mut() {
            s.check = Some((name, check));
        }
    }

    /// The labels the page's wallet has handed out: its record's, or the
    /// page's own count where that is higher.
    pub(crate) fn labels_out(&self, session: &Session) -> u32 {
        self.wallet(session)
            .map_or(0, |w| w.labels)
            .max(self.state.as_ref().map_or(0, |s| s.labels))
    }

    /// Costs what `receiver` costs. What a check of `check` comes to, or
    /// `None` while a previous transaction is still missing. An unsigned
    /// input is no answer, and the change label is always looked for.
    pub(crate) fn answer(
        &self,
        c: &Compute,
        session: &Session,
        check: &crate::silent::check::Check,
    ) -> Option<crate::silent::check::Checked> {
        use crate::silent::check::{Checked, Paid};
        let prevouts = check.prevouts()?;
        let tx = check.tx();
        for (i, input) in tx.input.iter().enumerate() {
            if faraday_bip::silent::needs_signature_data(prevouts[i].as_bytes())
                && !faraday_psbt::transaction::is_signed(&input.script_sig, &input.witness)
            {
                return Some(Checked::Unsigned);
            }
        }
        let Some((receiver, _)) = self.receiver(c, session) else {
            return Some(Checked::KeyNotLoaded);
        };
        let labels_out = self.labels_out(session);
        let labels: Vec<u32> = (0..=labels_out).collect();
        let scripts: Vec<&[u8]> = prevouts.iter().map(|s| s.as_bytes()).collect();
        match receiver.payments_in(tx, &scripts, &labels) {
            Err(_) => Some(Checked::NoInputs),
            Ok(found) if found.is_empty() => Some(Checked::NotPaid),
            Ok(found) => Some(Checked::Paid(
                found
                    .into_iter()
                    .map(|p| Paid {
                        vout: p.vout,
                        amount: p.amount,
                        label: p.label,
                    })
                    .collect(),
            )),
        }
    }
}

/// Silent payments' steps, plugged into Back and Forward (F104).
pub(crate) struct SilentSteps;

/// The one [`SilentSteps`].
pub(crate) static SILENT_STEPS: SilentSteps = SilentSteps;

impl crate::trail::StepFlow for SilentSteps {
    fn steps(&self, app: &Faraday) -> Option<crate::trail::Steps> {
        let s = app.silent.state()?;
        let order = (0..sstep::COUNT as u8).collect();
        let mut steps = crate::trail::Steps::new(order, s.open);
        // A wallet's page, or a chosen key, opens any card.
        steps.answered = (0..sstep::COUNT as u8)
            .filter(|&k| s.key.is_some() || s.wallet.is_some() || s.done[usize::from(k)])
            .collect();
        Some(steps)
    }

    fn open(&self, _: &Faraday, k: u8) -> crate::Action {
        crate::Action::SStep(k)
    }

    fn name(&self, _: &Faraday, k: u8) -> &'static str {
        screen::TITLES[usize::from(k)]
    }

    fn contained(&self) -> Option<&dyn crate::flow::Contained> {
        Some(self)
    }
}

impl crate::flow::Contained for SilentSteps {
    fn rows(&self, app: &Faraday) -> Vec<crate::flow::Row> {
        screen::rows(app)
    }

    fn hide_shown(&self, _: &mut Faraday) {
        // The page shows public keys and addresses: nothing is shown behind an eye.
    }

    fn count_fixed(&self, _: &Faraday) -> bool {
        true
    }

    fn still_fits(&self, _: &Faraday, _changed: u8, _later: u8) -> bool {
        // The act code keeps every later answer.
        true
    }
}

impl Faraday {
    /// After a press: notes the press that left a Coldcard export of two
    /// accounts waiting, to run again once one is chosen, and opens the
    /// account dialog over what is on screen while one waits.
    pub(crate) fn account_after(&mut self, asked: bool, action: crate::Action) {
        // Add to vault loads nothing, so it is not among the presses
        // `replays` names for a held wallet; it does wait for an account.
        let adds = matches!(
            action,
            crate::Action::Vault(crate::vault::VaultAction::AddFile(_))
        );
        if !asked && (crate::wallet::replays(action) || adds) {
            let inbox = self.inbox_print();
            self.session.account_replay(action, inbox);
        }
        if self.session.account_asked().is_some() && self.sheet != Some(crate::Sheet::Account) {
            self.sheet = Some(crate::Sheet::Account);
        }
    }

    /// Costs what `add_policy` costs: the account dialog's choice, or its
    /// Cancel (`None`). A load's choice is kept for the session; the press
    /// that asked runs again while Files holds what it held, and an
    /// export that came in any other way is added here. Add to vault
    /// loads nothing: it runs again on the file it asked about, wherever
    /// Files now holds it, or says that file is gone.
    pub(crate) fn account_act(&mut self, chosen: Option<crate::wallet::Account>) {
        self.sheet = None;
        let Some(account) = chosen else {
            self.session.account_drop();
            return;
        };
        let Some(ask) = self.session.account_choose(account) else {
            return;
        };
        if let Some((crate::Action::Vault(crate::vault::VaultAction::AddFile(_)), _)) = ask.replay {
            let c = self.compute();
            let at = self.inbox.iter().position(|item| {
                item.kind == crate::wallet::FileKind::Wallet
                    && *item.name == *ask.source
                    && ask.is_export(&c, &faraday_text::Text::from_utf8_lossy(&item.bytes))
            });
            match at {
                Some(k) => {
                    self.session.account_for_add(Some(account));
                    self.act(crate::Action::Vault(crate::vault::VaultAction::AddFile(k)));
                    self.session.account_for_add(None);
                }
                None => {
                    let said = faraday_text::Text::fill(EN.vault_key_file_gone, &[&ask.source]);
                    self.toast(&said);
                }
            }
            return;
        }
        if let Some((press, inbox)) = ask.replay
            && inbox == self.inbox_print()
        {
            return self.act(press);
        }
        let policy = match account {
            crate::wallet::Account::SingleSig => ask.single,
            crate::wallet::Account::Silent => ask.silent,
        };
        let c = self.compute();
        match self
            .session
            .add_policy(&c, &ask.name, policy, &ask.source, ask.origin)
        {
            Ok(i) => {
                self.wallets.set_wallet(i);
                self.refresh_spend();
                self.toast_added(i);
            }
            // Held for its sheet, which opens next.
            Err(crate::wallet::Refusal::Unconfirmed) => {}
            Err(e) => self.toast(&e.text()),
        }
    }

    /// The flow's state, while it is open.
    pub fn silent(&self) -> Option<&SilentState> {
        self.silent.state()
    }

    /// Costs what [`Faraday::silent_count_start`] costs. Opens the flow.
    pub(crate) fn silent_open(&mut self) {
        self.silent.open(&self.session, self.screen);
        self.screen = Screen::Silent;
        let c = self.compute();
        self.silent_count_start(&c, true);
    }

    /// Costs what [`Faraday::silent_count_start`] costs. Opens the flow
    /// over the loaded key with fingerprint `fp`.
    pub(crate) fn silent_open_for(&mut self, fp: [u8; 4]) {
        self.silent.open_for(self.screen, fp);
        self.screen = Screen::Silent;
        let c = self.compute();
        self.silent_count_start(&c, true);
    }

    /// Costs what `own_record` costs, and what
    /// [`Faraday::silent_count_known`] costs. On the press that enters a key's page (`entered`)
    /// or chooses its key, never in drawing: the page's count is raised
    /// to the highest of what the session has handed out for that wallet,
    /// the session wallet's and each open vault's own record of it. Entering makes the page that session wallet's; a key press
    /// never does, and leaves a wallet's page as it is.
    fn silent_count_start(&mut self, c: &Compute, entered: bool) {
        let Some((fp, linked)) = self
            .silent
            .state()
            .and_then(|s| Some((s.key?, s.wallet.is_some())))
        else {
            return;
        };
        if linked && !entered {
            return;
        }
        let Some(own) = own_record(c, &self.session, fp) else {
            return;
        };
        let at = wallet_of(&self.session, &own);
        let held = at.and_then(|i| self.session.wallets[i].policy().silent());
        let most = held
            .map_or(0, |r| r.labels)
            .max(self.silent_count_known(c, &own));
        self.silent.start_from(most, at.filter(|_| entered));
    }

    /// Costs what `silent_record` costs, writing it out and what
    /// [`Faraday::silent_count_known`] costs. Where an unlock from the
    /// page lands back on it, where a vault opened since the page's count
    /// was worked out may have handed out more, and the page's count is
    /// raised to that; and on "+" from no label, before the step, as a
    /// guard on the press that hands out a label.
    pub(crate) fn silent_recount(&mut self, c: &Compute) {
        let Some(page) = self.silent_record(c).and_then(|t| SilentWallet::parse(&t)) else {
            return;
        };
        let most = self.silent_count_known(c, &WalletPolicy::of_silent(page));
        self.silent.start_from(most, None);
    }

    /// Costs writing `own` out, and what `described_of` costs for each
    /// open vault. The most labels of the silent payments wallet `own`
    /// that the session has handed out or an open vault's own record of
    /// it states, or none.
    fn silent_count_known(&self, c: &Compute, own: &WalletPolicy) -> u32 {
        let same = crate::wallet::same_wallet(own);
        let in_vaults = self.vaults.open.iter().flat_map(|o| {
            self.vaults
                .described_of(c, o.id, kind::WALLET)
                .filter(|(_, _, d)| *d.same == *same && d.policy.id() == own.id())
                .filter_map(|(_, _, d)| d.policy.silent())
        });
        let handed = self.session.silent_handed.of(&same, own.id());
        in_vaults.map(|r| r.labels).fold(handed, u32::max)
    }

    /// Opens the flow with loaded key `fp` picked.
    pub(crate) fn silent_open_on(&mut self, fp: [u8; 4]) {
        self.silent.open_for(&self.session, self.screen, fp);
        self.screen = Screen::Silent;
    }

    /// The record of the wallet the flow is the page of.
    pub fn silent_wallet(&self) -> Option<&SilentWallet> {
        self.silent.wallet(&self.session)
    }

    /// Costs two key derivations and an address the first time. What the
    /// page shows of its key ([`SilentShown`]), kept on the flow's state.
    pub(crate) fn silent_shown(&self, c: &Compute) -> SilentShown {
        self.silent.shown(c, &self.session)
    }

    /// Costs what [`check_record`] costs the first time. How session wallet `i`'s
    /// silent payments record stands against the loaded keys, kept by the
    /// record and the ids of the loaded keys (a
    /// lookup after); `None` for any other wallet.
    pub(crate) fn silent_wallet_check(&self, c: &Compute, i: usize) -> Option<RecordCheck> {
        let w = self.session.wallets.get(i)?.policy().silent()?;
        Some(
            self.kept
                .silent_check(c, &self.session.keys, w, self.session.wallets.len()),
        )
    }

    /// Costs what `silent_wallet_check` costs and an address the first
    /// time. Session wallet `i`'s silent payments record check and its
    /// encoded address, kept together; `None` for any other wallet.
    pub(crate) fn silent_wallet_card(&self, c: &Compute, i: usize) -> Option<(RecordCheck, Text)> {
        let w = self.session.wallets.get(i)?.policy().silent()?;
        Some(
            self.kept
                .silent_card(c, &self.session.keys, w, self.session.wallets.len()),
        )
    }

    /// Costs what [`check_record`] costs. [`Faraday::silent_wallet_check`] worked
    /// out from the record and the keys as they are, for a decision.
    pub(crate) fn silent_wallet_check_now(&self, c: &Compute, i: usize) -> Option<RecordCheck> {
        let w = self.session.wallets.get(i)?.policy().silent()?;
        Some(check_record(c, &self.session.keys, w).0)
    }

    /// Costs two key derivations for a silent payments wallet, nothing for
    /// another. The lines under wallet `i`'s code, `lines`: a silent
    /// payments wallet's "descriptor" is its address, so its code is
    /// marked not checked while no key with its fingerprint is loaded, and
    /// not made where a loaded key contradicts the record.
    pub(crate) fn silent_marks(
        &self,
        c: &Compute,
        i: usize,
        mut lines: Vec<Text>,
    ) -> Result<Vec<Text>, Text> {
        let Some(r) = self
            .session
            .wallets
            .get(i)
            .and_then(|w| w.policy().silent())
        else {
            return Ok(lines);
        };
        let fp = fp_text(r.fingerprint);
        match self.silent_wallet_check_now(c, i) {
            Some(RecordCheck::Differs) => Err(differs_line(&self.session.keys, r.fingerprint.0)),
            Some(RecordCheck::Unchecked) => {
                lines.push(Text::fill(EN.silent_not_checked, &[&fp]));
                Ok(lines)
            }
            _ => Ok(lines),
        }
    }

    /// Costs two key derivations for a silent payments wallet, nothing for
    /// another. Session wallet `i`, for its public files: none for a
    /// silent payments wallet whose record a loaded key contradicts, whose
    /// "descriptor" is an address that is not the key's.
    pub(crate) fn public_wallet(&self, c: &Compute, i: usize) -> Option<&crate::wallet::Wallet> {
        if self.silent_wallet_check_now(c, i) == Some(RecordCheck::Differs) {
            return None;
        }
        self.session.wallets.get(i)
    }

    /// Says why a wallet `added` from a vault was refused, where it is a
    /// silent payments record a loaded key contradicts; any other refusal
    /// is left to the count of wallets loaded.
    pub(crate) fn silent_refused(&mut self, added: Result<usize, crate::wallet::Refusal>) {
        if let Err(
            e @ (crate::wallet::Refusal::SilentDiffers(_)
            | crate::wallet::Refusal::SilentOtherKey { .. }),
        ) = added
        {
            self.toast(&e.text());
        }
    }

    /// Costs two key derivations: whether the wallet's key is loaded.
    pub fn silent_key_here(&self, c: &Compute) -> bool {
        self.silent.key_here(c, &self.session)
    }

    /// Costs two key derivations and an address, none for the wallet's own:
    /// the address on show.
    pub fn silent_address(&self, c: &Compute) -> Option<Text> {
        self.silent.address(c, &self.session)
    }

    /// Costs two key derivations. The public record a coordinator keeps.
    pub fn silent_record(&self, c: &Compute) -> Option<Text> {
        self.silent.record(c, &self.session)
    }

    /// Costs what `silent_address`, `silent_record` and `of` cost. The QR
    /// view of the address on show or of the record.
    ///
    /// While the page's key made here waits for its backup, no code of the
    /// address or of its DNS record is made, as for a receive address
    /// (F31), and the record's own code, a public file as a descriptor is,
    /// carries the warning first, as a descriptor's does.
    pub(crate) fn silent_code(&self, c: &Compute, code: crate::Code) -> Result<QrView, Text> {
        let held = self.silent_held(c);
        if held && matches!(code, crate::Code::Silent(_) | crate::Code::SilentDns) {
            return Err(Text::of(EN.code_view_no_address));
        }
        let mut view = self.silent.code(c, &self.session, code)?;
        if held {
            view.label.insert(0, Text::of(EN.glance_back_up_before));
        }
        Ok(view)
    }

    /// A key for the DNS record's fields; whether it was taken.
    pub(crate) fn silent_key(&mut self, key: faraday_shell_api::Key) -> bool {
        self.screen == Screen::Silent && self.silent.key(key)
    }

    /// One press inside the flow.
    pub(crate) fn silent_act(&mut self, action: crate::Action) {
        let c = self.compute();
        use crate::Action as A;
        match action {
            A::Silent => self.silent_open(),
            A::SQr(uri) => {
                if self.silent_address(&c).is_some() {
                    let view = self.silent_code(&c, crate::Code::Silent(uri));
                    self.open_qr(view);
                }
            }
            A::SRecord => {
                let fp = self.silent.named(&self.session);
                if let (Some(text), Some(fp)) = (self.silent_record(&c), fp) {
                    let name = text!(
                        "silent-{}.txt",
                        &*fp_text(faraday_bip::keys::Fingerprint(fp))
                    );
                    self.put_outbox(&name, text.as_bytes().to_vec());
                    self.toast_out(&name);
                }
            }
            A::SScanVault => {
                if self.vaults.open.get(self.vaults.current).is_none() {
                    return self.toast(EN.silent_no_vault_open);
                }
                // The scan key, BIP-392's `spscan1q…`, and the key's fingerprint.
                let Some((r, fp)) = self.silent.receiver(&c, &self.session) else {
                    return;
                };
                let Some(key) = r.scan_key(faraday_keys::Reveal::VaultEntry(
                    faraday_keys::Part::ScanKey,
                )) else {
                    return;
                };
                let title = Text::fill(
                    EN.silent_silent_payments_scan,
                    &[&*fp_text(faraday_bip::keys::Fingerprint(fp))],
                );
                let note = text!("{}\n{}", &*title, key.as_str());
                self.vault_push(
                    Record::new(kind::NOTE).with(field::NOTE, note.as_bytes()),
                    &Text::fill(EN.silent_is_in_the_vault, &[&title]),
                );
            }
            A::SScanOut => {
                // The scan key, BIP-392's `spscan1q…`, and the key's fingerprint.
                let Some((r, fp)) = self.silent.receiver(&c, &self.session) else {
                    return;
                };
                let Some(key) = r.scan_key(faraday_keys::Reveal::OutboxFile(
                    faraday_keys::Part::ScanKey,
                )) else {
                    return;
                };
                let name = text!(
                    "spscan-{}.txt",
                    &*fp_text(faraday_bip::keys::Fingerprint(fp))
                );
                self.offer_secret(crate::secrets::SecretOut::new(
                    name,
                    Zeroizing::new(text!("{}\n", key.as_str()).as_bytes().to_vec()),
                    EN.silent_a_silent_payments,
                    EN.silent_whoever_has,
                ));
            }
            A::SDescriptorOut(i) => self.silent_descriptor_out(i),
            A::SAddWallet => self.silent_add_wallet(),
            A::SWallet(i) => {
                if self.silent.open_wallet(&self.session, i) {
                    self.screen = Screen::Silent;
                    let most = self.silent_count_known(&c, self.session.wallets[i].policy());
                    self.silent.start_from(most, None);
                }
            }
            A::SCheck(k) => self.silent_check(k),
            // No label is stepped to while no address is shown: it would
            // count as handed out.
            A::SLabel(_) if self.silent_held(&c) => {}
            _ => {
                let from_none = self.silent.state().is_some_and(|s| s.label == 0);
                if matches!(action, A::SLabel(d) if d > 0) && from_none {
                    self.silent_recount(&c);
                }
                if let Some((i, r)) = self.silent.act(action, &self.session) {
                    self.session.wallets[i].set_policy(WalletPolicy::of_silent(r));
                }
                match action {
                    A::SKey(_) => self.silent_count_start(&c, false),
                    A::SAnother => self.silent_count_start(&c, true),
                    A::SLabel(_) => self.silent_keep_count(&c),
                    _ => {}
                }
            }
        }
    }

    /// The key's silent payments wallet into the session's wallets: the
    /// two public keys, the key's origin and the labels handed out.
    /// Costs what [`check_record`] costs. Session wallet `i`'s BIP-392 descriptor,
    /// `sp([fp/352h/coinh/accounth]spscan1q…)`, offered through the secret
    /// sheet, from the loaded key whose scan and spend keys are the
    /// record's at its account; a toast where no such key is loaded.
    fn silent_descriptor_out(&mut self, i: usize) {
        let c = self.compute();
        let Some(w) = self.session.wallets.get(i) else {
            return;
        };
        let Some(record) = w.policy().silent() else {
            return;
        };
        let (_, found) = check_record(&c, &self.session.keys, record);
        let Some((r, _)) = found else {
            return self.toast(&Text::fill(
                EN.silent_descriptor_needs_key,
                &[&*fp_text(record.fingerprint)],
            ));
        };
        let Some(d) = r.descriptor(
            faraday_keys::Reveal::OutboxFile(faraday_keys::Part::ScanKey),
            record.fingerprint,
            record.account,
        ) else {
            return;
        };
        let name = text!("{}-sp-descriptor.txt", crate::file_stem(&w.name).as_str());
        self.offer_secret(crate::secrets::SecretOut::new(
            name,
            Zeroizing::new(text!("{}\n", d.as_str()).as_bytes().to_vec()),
            EN.silent_a_silent_payments_descriptor,
            EN.silent_whoever_has,
        ));
    }

    fn silent_add_wallet(&mut self) {
        let c = self.compute();
        let Some(text) = self.silent_record(&c) else {
            return self.toast(EN.silent_load_the_key);
        };
        let Some(fp) = self.silent.named(&self.session) else {
            return;
        };
        let name = Text::fill(
            EN.silent_silent_payments_n,
            &[&*fp_text(faraday_bip::keys::Fingerprint(fp))],
        );
        match self.session.add_wallet(
            &c,
            &name,
            &text,
            EN.silent_silent_payments,
            crate::wallet::Origin::Named,
        ) {
            Ok(i) => {
                self.silent.set_wallet(i);
                self.toast(EN.silent_added_to_your);
            }
            Err(e) => self.toast(&e.text()),
        }
    }

    /// Costs `silent_record` and writing the record out, on a label
    /// press. The labels the page's wallet has handed out are kept in the
    /// session ([`HandedOut`]); the catch-up after the press raises by
    /// them. Measured on the build machine (a release test build, medians
    /// of five alternating runs against the code without the count kept
    /// or followed, 1 and 4 open vaults of 20 wallet records): a label
    /// press, this and its catch-up, takes 0.3 to 0.44 ms more.
    fn silent_keep_count(&mut self, c: &Compute) {
        if let Some(page) = self.silent_record(c).and_then(|t| SilentWallet::parse(&t)) {
            self.session.silent_handed.keep(page);
        }
    }

    /// Costs a lookup of each session wallet, kept count and open vault,
    /// and no more unless a vault was opened or closed, a silent payments
    /// wallet with labels handed out came, went or took another count, or
    /// a write waits (below); then, for each such wallet, what
    /// [`Faraday::silent_records_follow`] costs.
    /// A change of an open vault's records ([`Vaults::edits`]) counts as
    /// one of these.
    /// Run after a press, a job's answer and a storage event, never in
    /// drawing: however a vault or a wallet came in (an unlock, a load, a
    /// file), a session wallet with fewer labels than the session handed
    /// out is raised to that count, and an open vault's record of a wallet
    /// with more labels, in the session's wallets or handed out, follows
    /// it. One try for each such change. A write waits while its vault is
    /// being sealed (not tried), and while memory is short for it or the
    /// seal cannot write it: until it is written, each run does all of the
    /// above, and tries each write refused for those two reasons. A write
    /// refused as more than the vault can hold, or because the vault is
    /// read-only, does not make it wait: it is tried again only once that
    /// vault's records have changed, which a read-only vault's never do.
    /// The refusals not yet told of are one toast.
    /// A vault written has its records read again by the next frame that
    /// reads them ([`Vaults::changed`]): for the press that hands out a
    /// new label, that frame took about 1.9 ms more for each open vault
    /// of 20 records (measured as [`Faraday::silent_keep_count`]'s
    /// press). A try that a vault refuses is not timed.
    pub(crate) fn silent_vaults_catch_up(&mut self) {
        let session = &mut self.session;
        session.silent_handed.raise(&mut session.wallets);
        let mut h = std::collections::hash_map::DefaultHasher::new();
        let mut held = Vec::new();
        let mut have = Vec::new();
        for w in &self.session.wallets {
            have.push((w.same(), w.id()));
            if let Some(r) = w.policy().silent().filter(|r| r.labels > 0) {
                (w.same(), r.labels).hash(&mut h);
                held.push((r.labels, w.policy()));
            }
        }
        // A count handed out for a wallet the session does not hold.
        for k in &self.session.silent_handed.0 {
            if !have.iter().any(|&(same, id)| k.is(same, id)) {
                (&*k.same, k.labels()).hash(&mut h);
                held.push((k.labels(), &k.policy));
            }
        }
        for o in &self.vaults.open {
            o.id.hash(&mut h);
        }
        // A record put into an open vault may state fewer.
        self.vaults.edits().hash(&mut h);
        let sign = (!held.is_empty()).then(|| h.finish());
        if self.silent.followed == sign {
            return;
        }
        let held: Vec<_> = held
            .into_iter()
            .map(|(count, policy)| (count, policy.clone()))
            .collect();
        let c = self.compute();
        let mut none_waits = true;
        let mut tell = Vec::new();
        for (count, policy) in held {
            none_waits &= self.silent_records_follow(&c, count, |_| Some(policy), &mut tell);
        }
        // While a write waits, the next run tries again: memory may have
        // come, or a seal ended, with no change that the sign shows.
        self.silent.followed = sign.filter(|_| none_waits);
        if !tell.is_empty() {
            let lines: Vec<Text> = tell.into_iter().map(|(_, line)| line).collect();
            self.toast(&Text::joined(&lines, " · "));
        }
    }

    /// Costs what `described_of` costs for each open vault, and, where a
    /// silent payments record in one states fewer than `count` labels, what
    /// `wallet` costs and a walk of the vault for each record written.
    /// Every open vault's own record of `wallet` is raised to `count`,
    /// never down: a payment to a label is looked for only up to the count
    /// of the record loaded. A vault that is not open, or holds no record
    /// of this wallet, is left as it is. A vault being sealed is not tried;
    /// a read-only one is refused as one that cannot be written, told of
    /// once and not tried again; and one that refused this wallet as more than it can
    /// hold is not tried until its records change. A line naming the vault
    /// and why goes onto `tell`, with the vault, for each refusal not told
    /// of before. Returns whether no write waits for the next run
    /// ([`Faraday::silent_vaults_catch_up`]).
    fn silent_records_follow(
        &mut self,
        c: &Compute,
        count: u32,
        wallet: impl FnOnce(&Self) -> Option<WalletPolicy>,
        tell: &mut Vec<(VaultId, Text)>,
    ) -> bool {
        let behind: Vec<_> = self
            .vaults
            .open
            .iter()
            .flat_map(|o| {
                self.vaults
                    .described_of(c, o.id, kind::WALLET)
                    .filter(|(_, _, d)| d.policy.silent().is_some_and(|r| r.labels < count))
                    .map(|(i, _, d)| (o.id, i, d.same.duplicate(), d.policy.clone()))
            })
            .collect();
        let mut none_waits = true;
        if behind.is_empty() {
            return none_waits;
        }
        let Some(wallet) = wallet(self) else {
            return none_waits;
        };
        let same = crate::wallet::same_wallet(&wallet);
        for (id, i, held, policy) in behind {
            if held != same || policy.id() != wallet.id() {
                continue;
            }
            let Some(mut raised) = policy.silent().cloned() else {
                continue;
            };
            // A vault being sealed takes no edit until its seal ends. A
            // read-only one never takes one, so it is not waited for. None
            // comes this far today: its Faraday records are not read.
            let read_only = self.vaults.store().read_only(id).is_some();
            if !read_only && self.vaults.store().is_locked(id) {
                none_waits = false;
                continue;
            }
            let version = self.vaults.store().version(id);
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (id, &*same).hash(&mut h);
            let of = h.finish();
            if self.silent.full.contains(&(of, version)) {
                continue;
            }
            raised.labels = count;
            let text = Text::of(&raised.to_text());
            // The store would refuse a read-only vault's edit as full,
            // which it is not.
            let wrote = if read_only {
                Err(NotSet::Unwritable)
            } else {
                self.vaults.edit_or(id, |s| {
                    s.set_publics_in(c, id, i, &[(field::WALLET, text.as_bytes())])
                })
            };
            // A refusal is told of once, until what is refused changes.
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (id, &*same, count).hash(&mut h);
            let refused = h.finish();
            match wrote {
                Ok(()) => {
                    self.silent.told.retain(|t| *t != refused);
                    self.silent.full.retain(|f| f.0 != of);
                    self.vaults.changed(id);
                }
                Err(why) => {
                    if read_only || matches!(why, NotSet::Full) {
                        self.silent.full.retain(|f| f.0 != of);
                        self.silent.full.push((of, version));
                    } else {
                        none_waits = false;
                    }
                    if self.silent.told.contains(&refused) {
                        continue;
                    }
                    self.silent.told.push(refused);
                    let why = self.not_set_text(Some(id), why);
                    let open = self.vaults.open.iter().find(|o| o.id == id);
                    let name = open.map_or("", |o| &*o.name);
                    let line = Text::fill(EN.silent_label_count_not_saved, &[name, &why]);
                    // Two wallets one vault refuses for one reason are
                    // one line; two vaults of one name are two.
                    let line = (id, line);
                    if !tell.contains(&line) {
                        tell.push(line);
                    }
                }
            }
        }
        none_waits
    }

    /// Reads Inbox file `k` as the transaction to check, takes every
    /// previous transaction Files has for it, and works the answer out
    /// once every previous output is known.
    fn silent_check(&mut self, k: usize) {
        let c = self.compute();
        let Some(item) = self.inbox.get(k) else {
            return;
        };
        let Some(mut check) = crate::silent::check::Check::read(&item.bytes) else {
            return self.toast(EN.silent_not_a_transaction);
        };
        let name = Text::of(&item.name);
        // Each pass takes a transaction Files has for an input still
        // open; a pass that takes nothing ends it.
        while check.waiting_for().is_some() {
            let took = self.inbox.iter().any(|it| check.add_previous(&it.bytes));
            if !took {
                break;
            }
        }
        if let Some(answer) = self.silent.answer(&c, &self.session, &check) {
            check.set_result(answer);
        }
        self.silent.set_check(name, check);
    }
}
