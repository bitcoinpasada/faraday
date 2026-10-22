//! Explore's address list and its search (F76): the receive and change
//! addresses of the account the explored path is in, a page at a time
//! from any index, and the first hundred, thousand or ten thousand of
//! each chain searched for an address typed or pasted, whole or in part.
//!
//! The addresses are worked out from the account's extended public key
//! as a single-key wallet over it (`WalletPolicy::from_xpub`), the same
//! derivation every wallet's addresses go through, and kept on Explore's
//! state with what they are worked out from ([`Kept`]). A search is not a
//! thread: every tick derives a budget of indices and comes back, the
//! budget resized from how long the last tick took, as a vanity grind
//! runs (`crate::grind`), so a frame is still a frame on a Pi 3.

use faraday_bip::bitcoin::bip32::{ChildNumber, DerivationPath};
use faraday_bip::bitcoin::secp256k1::{Secp256k1, VerifyOnly};
use faraday_bip::bitcoin::{Address, Network as BtcNetwork};
use faraday_bip::keys::{Network, ScriptType};
use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::Compute;

use super::implied_script;
use crate::strings::EN;
use crate::{Faraday, Screen};

/// Addresses on one page of the list.
pub const PAGE: u32 = 10;

/// How deep a search goes, in indices of each chain: the first hundred,
/// the depth "Is this address mine?" looks at for a loaded wallet
/// (`docs/QR.md` §2); a thousand, a wallet used for years; ten thousand,
/// a shop's or a service's: 1.9 s on the build machine, about twenty on
/// a Pi 3 (F76).
pub const DEPTHS: [u32; 3] = [100, 1_000, 10_000];

/// The most addresses kept of each chain from index 0: the deepest
/// search's.
pub const RUN_KEPT: usize = 10_000;

/// The most matches a search keeps to show; past it, only how many.
pub const MATCHES_KEPT: usize = 50;

/// The longest text the search field takes: a Taproot address on
/// regtest is 64 characters, and a pasted one may carry no more.
pub const QUERY_MAX: usize = 90;

/// How long one tick of a search aims to take, ms.
const TICK_MS: u64 = 50;

/// The first tick's indices, small enough to come back at once on the
/// slowest device, since it is the tick the rate is first measured over.
const FIRST_BUDGET: u32 = 4;

/// The most indices one tick takes, whatever the rate measured.
const MAX_BUDGET: u32 = 4_096;

/// Which of Explore's pages is on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    /// The path, and what it gives.
    #[default]
    Path,
    /// The account's addresses, a page at a time.
    Addresses,
    /// Find an address among the account's.
    Find,
}

/// A press on the address list or the search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XAction {
    /// Open a page.
    Page(Page),
    /// The list's chain: receive (`false`) or change (`true`).
    Chain(bool),
    /// The list a page on (`1`) or back (`-1`).
    Step(i32),
    /// How deep the search goes, by its place in [`DEPTHS`].
    Depth(u8),
    /// Start the search, or stop the one under way.
    Search,
    /// The clipboard's text into the search field.
    Paste,
}

/// What an account's addresses are worked out from: the key, the
/// account's path, the script, the network and whether the key is
/// loaded. Another of any is another list.
pub type AccountOf = (Option<[u8; 4]>, DerivationPath, ScriptType, Network, bool);

/// The addresses worked out for one account, kept on Explore's state.
///
/// Invariant: every address held is the address of the account `of`
/// names at its chain and index, and nothing is held for any other
/// account: [`Kept::of_account`] drops everything when asked for
/// another, before anything is looked up or derived.
#[derive(Default)]
pub struct Kept {
    /// The account they are of; `None` until one is asked.
    of: Option<AccountOf>,
    /// The account as a single-key wallet over its extended public key;
    /// `None` where the key is not loaded.
    policy: Option<WalletPolicy>,
    /// Each chain's addresses from index 0, as far as derived:
    /// `run[1][i]` is change address `i`. At most [`RUN_KEPT`] each.
    run: [Vec<String>; 2],
    /// One page past the run: its chain, its first index, its addresses.
    window: Option<(bool, u32, Vec<String>)>,
}

impl Kept {
    /// What is kept for `of`: everything kept for another account is
    /// dropped first. Returns whether it was another.
    fn of_account(&mut self, of: &AccountOf) -> bool {
        if self.of.as_ref() == Some(of) {
            return false;
        }
        *self = Kept {
            of: Some(of.clone()),
            ..Kept::default()
        };
        true
    }

    /// How many addresses are kept, both chains and the page past them.
    pub fn len(&self) -> usize {
        self.run[0].len() + self.run[1].len() + self.window.as_ref().map_or(0, |w| w.2.len())
    }

    /// Whether nothing is kept.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Costs an address derivation when it is not kept. The address at
    /// `change`/`index`, kept in the run when it extends it; `None`
    /// where the key is not loaded.
    fn at(
        &mut self,
        c: &Compute,
        secp: &Secp256k1<VerifyOnly>,
        net: Network,
        change: bool,
        index: u32,
    ) -> Option<String> {
        let policy = self.policy.as_ref()?;
        let run = &mut self.run[usize::from(change)];
        let i = index as usize;
        if let Some(a) = run.get(i) {
            return Some(a.clone());
        }
        let a = derive(c, secp, policy, net, change, index);
        if i == run.len() && i < RUN_KEPT {
            run.push(a.clone());
        }
        Some(a)
    }

    /// Costs an address derivation for each address not kept. The page
    /// of [`PAGE`] addresses from `from` on `change`'s chain: in the run
    /// where it reaches, else as the one page kept past it.
    fn page(&mut self, c: &Compute, net: Network, change: bool, from: u32) -> Vec<(u32, String)> {
        let secp = Secp256k1::verification_only();
        let end = from.saturating_add(PAGE).min(INDEX_END);
        let reach = self.run[usize::from(change)].len();
        if from as usize <= reach && end as usize <= RUN_KEPT {
            return (from..end)
                .filter_map(|i| Some((i, self.at(c, &secp, net, change, i)?)))
                .collect();
        }
        if let Some((ch, f, list)) = &self.window
            && *ch == change
            && *f == from
        {
            return (from..).zip(list.iter().cloned()).collect();
        }
        let Some(policy) = self.policy.as_ref() else {
            return Vec::new();
        };
        let list: Vec<String> = (from..end)
            .map(|i| derive(c, &secp, policy, net, change, i))
            .collect();
        self.window = Some((change, from, list.clone()));
        (from..).zip(list).collect()
    }
}

/// One past the largest unhardened index.
const INDEX_END: u32 = 1 << 31;

/// The first index of the list's last page.
pub const LAST_FROM: u32 = INDEX_END - PAGE;

/// Costs an address derivation. The address of `policy` at
/// `change`/`index` on `net`, or empty where there is none (an index
/// whose key is invalid, one chance in 2¹²⁷).
fn derive(
    c: &Compute,
    secp: &Secp256k1<VerifyOnly>,
    policy: &WalletPolicy,
    net: Network,
    change: bool,
    index: u32,
) -> String {
    policy
        .script_at_with(c, secp, change, index)
        .ok()
        .and_then(|s| Address::from_script(&s, BtcNetwork::from(net)).ok())
        .map_or_else(String::new, |a| a.to_string())
}

/// One address a search found, and where in it the match lies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    /// The chain: `true` for change.
    pub change: bool,
    /// The index on the chain.
    pub index: u32,
    /// The address.
    pub address: String,
    /// The bytes of `address` that matched, found once when the search
    /// matched it (bech32 in either case). An address is ASCII, so a byte
    /// is a character, and the screen's text of it
    /// ([`Session::shown`](crate::wallet::Session::shown)) has the same
    /// length, so the range holds for it too.
    pub mark: std::ops::Range<usize>,
}

/// A search under way or done, and what it found.
pub struct Search {
    /// The account it looks in.
    pub of: AccountOf,
    /// What it looks for, as typed.
    pub query: String,
    /// The same in lower case, for a bech32 address, which is one
    /// whatever the case it is typed in.
    lower: String,
    /// How many indices of each chain it looks at.
    pub depth: u32,
    /// The next index it looks at on both chains; `depth` once done.
    pub next: u32,
    /// The first [`MATCHES_KEPT`] matches, the receive address before
    /// the change one at each index.
    pub found: Vec<Found>,
    /// How many addresses matched in all.
    pub matched: u32,
    /// Indices the next tick takes.
    budget: u32,
    /// When the last tick came, ms.
    last_ms: u64,
}

impl Search {
    /// Whether it is still looking.
    pub fn running(&self) -> bool {
        self.next < self.depth
    }

    /// Whether `address` is what is looked for: holds the text typed, in
    /// either case for a bech32 address.
    pub fn matches(&self, address: &str) -> bool {
        self.mark(address).is_some()
    }

    /// Where in `address` the text looked for lies, as bytes: the first
    /// place it is, as typed, else (bech32 only) in either case.
    fn mark(&self, address: &str) -> Option<std::ops::Range<usize>> {
        if self.query.is_empty() {
            return None;
        }
        let range = |q: &str, a: &str| a.find(q).map(|i| i..i + q.len());
        range(&self.query, address).or_else(|| {
            if bech32(address) {
                range(&self.lower, &address.to_ascii_lowercase())
            } else {
                None
            }
        })
    }

    /// Whether `address` is the whole of what is looked for, which no
    /// other address can be.
    fn whole(&self, address: &str) -> bool {
        address == self.query || (bech32(address) && address == self.lower)
    }

    /// The budget for the next tick, from `tested` indices taken since
    /// the last one.
    fn ticked(&mut self, tested: u32, now_ms: u64) {
        let span = now_ms.saturating_sub(self.last_ms).max(1);
        self.last_ms = now_ms;
        let next = u64::from(tested.max(1)) * TICK_MS / span;
        self.budget = (next.min(u64::from(MAX_BUDGET)) as u32).max(FIRST_BUDGET);
    }
}

/// Whether `address` is bech32 (SegWit or Taproot), which is written
/// in lower case.
fn bech32(address: &str) -> bool {
    address.starts_with("bc1") || address.starts_with("tb1") || address.starts_with("bcrt1")
}

/// The account a path is in: the path without its chain and index
/// where it ends `…/0/i` or `…/1/i`, or the path itself where every
/// level is hardened. `None` for a multisig purpose (BIP-45, BIP-48),
/// whose accounts pay to no single key, and for any other shape.
pub fn account_of(path: &DerivationPath) -> Option<DerivationPath> {
    let levels: &[ChildNumber] = path.as_ref();
    if matches!(
        levels.first(),
        Some(ChildNumber::Hardened { index: 45 | 48 } | ChildNumber::Normal { index: 45 })
    ) {
        return None;
    }
    match levels {
        [
            account @ ..,
            ChildNumber::Normal { index: 0 | 1 },
            ChildNumber::Normal { .. },
        ] => Some(DerivationPath::from(account)),
        _ if !levels.is_empty() && levels.iter().all(ChildNumber::is_hardened) => {
            Some(path.clone())
        }
        _ => None,
    }
}

impl Faraday {
    /// What the explored path's account's addresses are worked out from,
    /// or `None` where the path is in no account (`account_of`).
    pub fn explore_account(&self) -> Option<AccountOf> {
        let e = self.explore.as_ref()?;
        let path = self.explore_path().ok()?;
        let account = account_of(&path)?;
        let script = implied_script(&path).unwrap_or(e.script);
        let loaded = self
            .session
            .keys
            .all()
            .any(|k| Some(k.fingerprint.0) == e.key);
        Some((e.key, account, script, self.session.network(), loaded))
    }

    /// Costs a key derivation the first time for an account. Explore's
    /// kept addresses made those of `of`, with the account's extended
    /// public key derived the first time.
    fn explore_keep(&self, c: &Compute, of: &AccountOf) {
        let Some(e) = self.explore.as_ref() else {
            return;
        };
        if !e.kept.borrow_mut().of_account(of) {
            return;
        }
        let (key, account, script, ..) = of;
        let policy = key
            .and_then(|fp| self.session.keys.position_of(fp))
            .and_then(|place| self.session.keys.xpub_at(c, place, account))
            .and_then(|xpub| WalletPolicy::from_xpub(xpub, *script).ok());
        e.kept.borrow_mut().policy = policy;
    }

    /// Costs a key derivation and an address derivation for each
    /// address not yet kept. The list's page as it stands: each index
    /// with its address, worked out by whoever asks first, a frame
    /// included, and kept on Explore's state ([`Kept`]), a lookup after.
    pub fn explore_list(&self, c: &Compute) -> Vec<(u32, String)> {
        let (Some(e), Some(of)) = (self.explore.as_ref(), self.explore_account()) else {
            return Vec::new();
        };
        self.explore_keep(c, &of);
        let net = self.session.network();
        e.kept.borrow_mut().page(c, net, e.change, e.from)
    }

    /// How many addresses Explore keeps.
    pub fn explore_kept(&self) -> usize {
        self.explore.as_ref().map_or(0, |e| e.kept.borrow().len())
    }

    /// A press on the address list or the search.
    pub(crate) fn explore_x(&mut self, a: XAction) {
        let clipboard = self.clipboard;
        let Some(e) = self.explore.as_mut() else {
            return;
        };
        match a {
            XAction::Page(p) => {
                e.page = p;
                e.note = None;
                self.list_offset = 0.0;
            }
            XAction::Chain(change) => e.change = change,
            XAction::Step(d) => {
                let to = i64::from(e.from) + i64::from(d) * i64::from(PAGE);
                e.from = to.clamp(0, i64::from(LAST_FROM)) as u32;
            }
            XAction::Depth(i) => {
                if let Some(&d) = DEPTHS.get(usize::from(i)) {
                    e.depth = d;
                    e.search = None;
                }
            }
            XAction::Search => self.explore_search(),
            XAction::Paste => {
                if clipboard {
                    self.commands
                        .push_back(faraday_shell_api::Command::RequestClipboard {
                            kind: faraday_shell_api::FileKind::Text,
                        });
                }
            }
        }
        if self
            .explore
            .as_ref()
            .is_some_and(|e| e.page == Page::Addresses)
        {
            let c = self.compute();
            let _ = self.explore_list(&c);
        }
    }

    /// Search starts on what is typed, or the one under way stops where
    /// it is.
    fn explore_search(&mut self) {
        let of = self.explore_account();
        let now = self.now_ms;
        let Some(e) = self.explore.as_mut() else {
            return;
        };
        if let Some(s) = e.search.as_mut()
            && s.running()
        {
            s.depth = s.next;
            return;
        }
        let Some(of) = of else {
            return;
        };
        if e.query.is_empty() {
            return;
        }
        e.note = None;
        e.search = Some(Search {
            of,
            lower: e.query.to_ascii_lowercase(),
            query: e.query.clone(),
            depth: e.depth,
            next: 0,
            found: Vec::new(),
            matched: 0,
            budget: FIRST_BUDGET,
            last_ms: now,
        });
    }

    /// Costs an address derivation for each address of the tick's
    /// indices not yet kept, on both chains. One tick of a search under
    /// way. Returns whether to draw.
    pub(crate) fn explore_tick(&mut self, now_ms: u64) -> bool {
        if self.screen != Screen::Explore
            || !self
                .explore
                .as_ref()
                .and_then(|e| e.search.as_ref())
                .is_some_and(Search::running)
        {
            return false;
        }
        let c = self.compute();
        let of = self.explore_account();
        let net = self.session.network();
        let Some(e) = self.explore.as_mut() else {
            return false;
        };
        // A search of another account than the one explored now is gone.
        if e.search.as_ref().is_none_or(|s| Some(&s.of) != of.as_ref()) {
            e.search = None;
            return true;
        }
        let Some(of) = of else {
            return false;
        };
        self.explore_keep(&c, &of);
        let Some(e) = self.explore.as_mut() else {
            return false;
        };
        let (Some(s), kept) = (e.search.as_mut(), &e.kept) else {
            return false;
        };
        let mut kept = kept.borrow_mut();
        let secp = Secp256k1::verification_only();
        let end = s.next.saturating_add(s.budget).min(s.depth);
        let start = s.next;
        'indices: for i in start..end {
            for change in [false, true] {
                let Some(a) = kept.at(&c, &secp, net, change, i) else {
                    s.next = s.depth;
                    break 'indices;
                };
                let Some(mark) = s.mark(&a) else {
                    continue;
                };
                s.matched += 1;
                let whole = s.whole(&a);
                if s.found.len() < MATCHES_KEPT {
                    s.found.push(Found {
                        change,
                        index: i,
                        address: a,
                        mark,
                    });
                }
                // A whole address is one address: the search ends at it.
                if whole {
                    s.depth = i + 1;
                    s.next = i + 1;
                    break 'indices;
                }
            }
            s.next = i + 1;
        }
        let tested = s.next.saturating_sub(start);
        s.ticked(tested, now_ms);
        true
    }

    /// A key on the address list or the search: the list's first index
    /// typed in digits, a page on or back with the arrows; the search's
    /// text, Enter to search, Paste; Escape back to the path. Returns
    /// whether the key was taken.
    pub(crate) fn explore_page_key(&mut self, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key as K;
        let Some(e) = self.explore.as_mut() else {
            return false;
        };
        match (e.page, key) {
            (_, K::Escape) => self.explore_x(XAction::Page(Page::Path)),
            (Page::Find, K::Char(ch)) => self.explore_type(Some(ch)),
            (Page::Find, K::Backspace) => self.explore_type(None),
            (Page::Find, K::Enter) => self.explore_x(XAction::Search),
            (Page::Find, K::Paste) => self.explore_x(XAction::Paste),
            (Page::Addresses, K::Char(ch)) if ch.is_ascii_digit() => {
                let n = u64::from(e.from) * 10 + u64::from(ch as u8 - b'0');
                e.from = n.min(u64::from(LAST_FROM)) as u32;
                self.explore_x(XAction::Step(0));
            }
            (Page::Addresses, K::Backspace) => {
                e.from /= 10;
                self.explore_x(XAction::Step(0));
            }
            (Page::Addresses, K::Down) => self.explore_x(XAction::Step(1)),
            (Page::Addresses, K::Up) => self.explore_x(XAction::Step(-1)),
            _ => return false,
        }
        true
    }

    /// Typing into the search field: an address's characters, and
    /// Backspace. Any change drops the result, which was for the text
    /// before.
    pub(crate) fn explore_type(&mut self, ch: Option<char>) {
        let Some(e) = self.explore.as_mut() else {
            return;
        };
        match ch {
            Some(ch) if ch.is_ascii_alphanumeric() && e.query.len() < QUERY_MAX => {
                e.query.push(ch);
            }
            Some(_) => return,
            None => {
                if e.query.pop().is_none() {
                    return;
                }
            }
        }
        e.search = None;
        e.note = None;
    }

    /// Costs what `pasted_secret` costs. The clipboard's text into the
    /// search field, when Find an address is on screen: an address, or a
    /// payment link's address, whole or in part; a secret is refused and
    /// named, and nothing to paste says so. Returns whether it was taken
    /// here.
    pub(crate) fn explore_pasted(&mut self, text: &str) -> bool {
        if self.screen != Screen::Explore
            || self.sheet.is_some()
            || !self.explore.as_ref().is_some_and(|e| e.page == Page::Find)
        {
            return false;
        }
        let c = self.compute();
        let t = text.trim();
        let secret = crate::scan_key::pasted_secret(&c, t, false);
        let t = t.strip_prefix("bitcoin:").unwrap_or(t);
        let t = t.split('?').next().unwrap_or(t);
        let Some(e) = self.explore.as_mut() else {
            return true;
        };
        e.search = None;
        e.note = if t.is_empty() {
            Some(EN.pasted_nothing)
        } else if let Some(why) = secret {
            Some(why)
        } else if t.len() > QUERY_MAX || !t.bytes().all(|b| b.is_ascii_alphanumeric()) {
            Some(EN.explore_not_an_address)
        } else {
            e.query = t.to_string();
            None
        };
        true
    }
}
