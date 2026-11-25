//! Restoring a wallet from its backup (`docs/WALLETS.md` §2 and §5): the
//! wallet from its descriptor, its multisig config, or the split shares of
//! a multisig, which are merged as they arrive until every key is in hand;
//! then the seeds, typed back in.

use crate::strings::EN;
use faraday_bip::bitcoin::bip32::Xpub;
use faraday_bip::multisig_config;
use faraday_crypto::work::Compute;
use faraday_text::{Text, text};

use crate::wallet::{Session, Wallet, fp_text};
use crate::wallet_backup::split_share;
use crate::{
    Action, Faraday, FileKind, NOT_A_KEY, Screen, VaultAsk, create, flow, restore, rstep, seeds,
    stem, vault, wallet,
};
use faraday_bip::keys::Network;
use std::collections::BTreeSet;

/// What one multisig config file, whole or a share, says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigParts {
    /// `Name:`.
    pub name: Option<Text>,
    /// `Policy: M of N`.
    pub m: usize,
    /// The N of the policy.
    pub n: usize,
    /// `Format:`, P2SH when the file has none (F263).
    pub format: Text,
    /// Each key line: fingerprint, the derivation in force, the xpub.
    pub keys: Vec<(Text, Text, Text)>,
}

/// The longest `Derivation:` value read: a BIP-32 path at its deepest,
/// `m` and 255 steps of `/2147483647h`. Each key line keeps a copy
/// of the derivation in force, so a longer one is refused rather than
/// copied fifteen times.
const MAX_DERIVATION_LEN: usize = 1 + 255 * 12;

/// Whether `v` is a derivation path as Faraday's own sheets write one:
/// empty (a share without a `Derivation:` line has none), `m` or `m/`, or
/// an optional lowercase `m/` followed by 1 to 255 steps separated by single
/// `/`. A step is ASCII digits only, with no leading zero unless it is `0`,
/// a value below 2^31 and at most one trailing `'` or `h`. Anything else
/// is refused, so the value is written into the rebuilt wallet as read.
fn is_path(v: &str) -> bool {
    if matches!(v, "" | "m" | "m/") {
        return true;
    }
    let body = v.strip_prefix("m/").unwrap_or(v);
    let steps = body.split('/');
    steps.clone().count() <= 255
        && steps.into_iter().all(|step| {
            let digits = step.strip_suffix(['\'', 'h']).unwrap_or(step);
            !digits.is_empty()
                && digits.bytes().all(|b| b.is_ascii_digit())
                && (digits == "0" || !digits.starts_with('0'))
                && digits.len() <= 10
                && digits.parse::<u64>().is_ok_and(|n| n < 1 << 31)
        })
}

/// Reads a multisig config's lines without requiring every key. A
/// policy of more than [`multisig_config::MAX_KEYS`] keys, more key
/// lines than the policy names, a sixteenth key line or a derivation longer than a BIP-32 path can be is refused,
/// so a long file costs no more than a wallet does.
pub fn parts(text: &str) -> Option<ConfigParts> {
    let mut name = None;
    let mut policy = None;
    let mut format = None;
    let mut derivation = Text::default();
    let mut keys = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (k, v) = line.split_once(':')?;
        let (k, v) = (k.trim(), v.trim());
        match Text::ascii_lowercase(k).as_str() {
            "name" => name = Some(Text::of(v)),
            "policy" => {
                let (m, n) = v.split_once(" of ")?;
                policy = Some((m.trim().parse().ok()?, n.trim().parse().ok()?));
            }
            "format" => format = Some(Text::of(v)),
            "derivation" => {
                if v.len() > MAX_DERIVATION_LEN {
                    return None;
                }
                if !is_path(v) {
                    return None;
                }
                derivation = Text::of(v);
            }
            fp if fp.len() == 8 && fp.chars().all(|c| c.is_ascii_hexdigit()) => {
                if keys.len() == multisig_config::MAX_KEYS {
                    return None;
                }
                // One extended public key, as a Coldcard file reads it.
                faraday_bip::slip132::decode(v).ok()?;
                keys.push((Text::of(fp), derivation.clone(), Text::of(v)));
            }
            _ => {}
        }
    }
    let (m, n) = policy?;
    if keys.is_empty() || n > multisig_config::MAX_KEYS || keys.len() > n {
        return None;
    }
    Some(ConfigParts {
        name,
        m,
        n,
        format: format.unwrap_or_else(|| Text::of("P2SH")),
        keys,
    })
}

/// A seed's extended public key at a path, for [`Faraday::restore_refusal`]
/// to match a slot's whole key: `None` when it cannot be worked out.
pub(crate) type KeyAt<'a> =
    dyn Fn(&faraday_bip::bitcoin::bip32::DerivationPath) -> Option<Xpub> + 'a;

/// Whether a config holds fewer keys than its policy names: a share.
pub fn is_share(text: &str) -> bool {
    parts(text).is_some_and(|p| p.keys.len() < p.n)
}

/// What Restore's pool of shares comes to ([`Faraday::restore_pool`]).
pub struct Pool {
    /// The texts that go to [`merge`].
    pub texts: Vec<Text>,
    /// What the screen says where two different wallets of one checksum
    /// each rebuild from the shares: neither is taken.
    pub two: Option<Text>,
    /// What the texts add up to, or `two`.
    pub merged: Result<Merged, Text>,
}

impl Pool {
    fn of(texts: Vec<Text>, two: Option<Text>) -> Pool {
        let merged = match &two {
            Some(e) => Err(e.duplicate()),
            None => merge(&texts),
        };
        Pool { texts, two, merged }
    }

    /// Whether the pool holds anything to show: a share, or two wallets.
    pub fn shown(&self) -> bool {
        self.two.is_some() || !self.texts.is_empty()
    }
}

/// A wallet some vault share record names, whose sheets rebuild it.
struct Candidate {
    /// [`restore_candidate_key`]: one wallet whatever its keys' order.
    key: Text,
    /// The descriptor's checksum, as the record's wallet wrote it.
    checksum: Text,
    /// The wallet's name in the record, as the screen names a wallet.
    name: Text,
    /// What tells it from another of the same name: its keys.
    tell: Text,
    quorum: Option<(usize, usize)>,
    format: &'static str,
    /// Each key: fingerprint, derivation as a config writes it, xpub.
    keys: Vec<(Text, Text, Text)>,
}

impl Candidate {
    fn of(key: Text, policy: &faraday_bip::policy::WalletPolicy, name: Text) -> Option<Candidate> {
        let format = crate::wallet_backup::sheets_format(policy)?;
        let keys: Vec<(Text, Text, Text)> = policy
            .keys()
            .iter()
            .map(|k| {
                let fp = k
                    .fingerprint()
                    .map(fp_text)
                    .unwrap_or_else(|| Text::of("00000000"));
                let path = k
                    .path()
                    .map(|p| {
                        let shown = text!("{p}");
                        text!("m/{}", shown.trim_start_matches("m/"))
                    })
                    .unwrap_or_else(|| Text::of("m"));
                (fp, path, text!("{}", k.xpub()))
            })
            .collect();
        let mut tell = Text::of("");
        for (i, (f, d, _)) in keys.iter().enumerate() {
            if i > 0 {
                tell.push_str(", ");
            }
            tell.push_str(&text!("{} {}", &**f, &**d));
        }
        let full = policy.to_descriptor_checksummed();
        let checksum = Text::of(full.rsplit('#').next().unwrap_or(""));
        Some(Candidate {
            key,
            checksum,
            name,
            tell,
            quorum: policy.quorum().or_else(|| policy.tapscript_quorum()),
            format,
            keys,
        })
    }

    /// Whether `own` is a text of this wallet: its quorum and Format, and
    /// each of its keys' fingerprint, derivation and xpub.
    fn holds(&self, own: &ConfigParts) -> bool {
        self.quorum == Some((own.m, own.n))
            && own.format.trim().eq_ignore_ascii_case(self.format)
            && own.keys.iter().all(|(f, d, x)| {
                self.keys.iter().any(|(cf, cd, cx)| {
                    cf.eq_ignore_ascii_case(f) && same_derivation(cd, d) && cx.as_str() == x.trim()
                })
            })
    }
}

/// A vault share record whose wallet reads.
struct PoolRecord {
    /// Which open vault it is in.
    vault: usize,
    text: Text,
    own: ConfigParts,
    /// The candidate its wallet is, where the sheets rebuild it.
    names: Option<usize>,
}

/// A wallet told by its descriptor with the keys of each `sortedmulti`
/// and `sortedmulti_a` in order, so one wallet in two key orders is one.
fn restore_candidate_key(policy: &faraday_bip::policy::WalletPolicy) -> Text {
    let once = wallet::same_wallet(policy);
    const OPEN: &str = "sortedmulti_a(";
    let mut out = Text::of("");
    let mut rest: &str = &once;
    while let Some(at) = rest.find(OPEN) {
        let args = &rest[at + OPEN.len()..];
        let Some(end) = args.find(')') else { break };
        let mut keys: Vec<&str> = args[..end].split(',').collect();
        if keys.len() > 1 {
            keys[1..].sort_unstable();
        }
        out.push_str(&rest[..at + OPEN.len()]);
        for (i, k) in keys.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(k);
        }
        rest = &args[end..];
    }
    out.push_str(rest);
    out
}

/// What a set of shares adds up to.
pub struct Merged {
    /// The quorum and size, from the shares.
    pub m: usize,
    /// Keys in the wallet.
    pub n: usize,
    /// The fingerprints in hand.
    pub have: Vec<Text>,
    /// The whole config, once every key is in hand.
    pub whole: Option<Text>,
    /// The `Name:` the shares carry, as written: untrusted.
    pub name: Option<Text>,
}

/// Room for the lines of `keys` in a rebuilt text, so it is written once
/// and never regrown.
fn keys_size(keys: &[(Text, Text, Text)]) -> usize {
    keys.iter()
        .map(|(fp, d, x)| fp.len() + d.len() + x.len() + 24)
        .sum()
}

/// A derivation as written, `'` and `h` alike, for comparing two.
fn same_derivation(a: &str, b: &str) -> bool {
    let norm = |d: &str| -> Text {
        d.trim()
            .chars()
            .map(|c| {
                if c == '\'' {
                    'h'
                } else {
                    c.to_ascii_lowercase()
                }
            })
            .collect()
    };
    norm(a) == norm(b)
}

/// Merges shares of one wallet. Shares that disagree on the quorum, the
/// format or the name (where both carry one), that hold more keys than
/// the policy names, or that give one fingerprint two xpubs or two
/// derivations, are refused: sheets of two wallets are never combined.
/// So is a share that names one fingerprint twice: it does not say which
/// is the wallet's.
pub fn merge<T: AsRef<str>>(texts: &[T]) -> Result<Merged, Text> {
    let mut all: Option<ConfigParts> = None;
    for t in texts {
        let p = parts(t.as_ref()).ok_or_else(|| Text::of(EN.restore_not_a_multisig))?;
        for (i, k) in p.keys.iter().enumerate() {
            if p.keys[..i].iter().any(|x| x.0.eq_ignore_ascii_case(&k.0)) {
                return Err(Text::fill(EN.restore_key_twice, &[&*Text::lowercase(&k.0)]));
            }
        }
        match all.as_mut() {
            None => all = Some(p),
            Some(a) => {
                if a.m != p.m || a.n != p.n || !a.format.eq_ignore_ascii_case(&p.format) {
                    return Err(Text::of(EN.restore_these_shares));
                }
                match (&a.name, &p.name) {
                    (Some(x), Some(y)) if x.trim() != y.trim() => {
                        return Err(Text::of(EN.restore_these_shares));
                    }
                    (None, Some(_)) => a.name = p.name.clone(),
                    _ => {}
                }
                for k in p.keys {
                    let same_fp = || a.keys.iter().filter(|x| x.0.eq_ignore_ascii_case(&k.0));
                    if same_fp().any(|x| x.2 == k.2 && same_derivation(&x.1, &k.1)) {
                        continue;
                    }
                    if same_fp().next().is_some() || a.keys.len() >= a.n {
                        return Err(Text::of(EN.restore_these_shares));
                    }
                    a.keys.push(k);
                }
            }
        }
    }
    let a = all.ok_or_else(|| Text::of(EN.restore_no_shares))?;
    let have: Vec<Text> = a.keys.iter().map(|k| Text::lowercase(&k.0)).collect();
    // A Taproot multisig's sheets rebuild its descriptor: Coldcard's
    // config has no Taproot format.
    if a.format.eq_ignore_ascii_case("P2TR") {
        let whole = (a.keys.len() >= a.n).then(|| {
            use std::fmt::Write as _;
            let size = keys_size(&a.keys) + NUMS.len() + 48;
            let mut out = Text::with_room(size);
            let _ = write!(out, "tr({NUMS},sortedmulti_a({},", a.m);
            for (i, (fp, d, x)) in a.keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let path: Text = d
                    .trim_start_matches("m/")
                    .chars()
                    .map(|c| if c == '\'' { 'h' } else { c })
                    .collect();
                let fp = Text::lowercase(fp);
                if path.is_empty() || path == "m" {
                    let _ = write!(out, "[{}]{}/<0;1>/*", &*fp, &**x);
                } else {
                    let _ = write!(out, "[{}/{}]{}/<0;1>/*", &*fp, &*path, &**x);
                }
            }
            out.push_str("))");
            out
        });
        return Ok(Merged {
            m: a.m,
            n: a.n,
            have,
            whole,
            name: a.name,
        });
    }
    let whole = (a.keys.len() >= a.n).then(|| {
        use std::fmt::Write as _;
        let size =
            keys_size(&a.keys) + a.name.as_ref().map_or(0, |n| n.len() + 8) + a.format.len() + 128;
        let mut out = Text::with_room(size);
        out.push_str("# Multisig config rebuilt by Faraday from its shares\n");
        if let Some(n) = &a.name {
            let _ = writeln!(out, "Name: {}", &**n); // not shown: a line of the rebuilt config file
        }
        let _ = write!(out, "Policy: {} of {}\nFormat: {}\n", a.m, a.n, &*a.format);
        for (fp, d, x) in &a.keys {
            let _ = write!(out, "\nDerivation: {}\n{}: {}\n", &**d, &**fp, &**x);
        }
        out
    });
    Ok(Merged {
        m: a.m,
        n: a.n,
        have,
        whole,
        name: a.name,
    })
}

/// The name a wallet rebuilt from shares is added under: the config's
/// `Name:`, or for a Taproot descriptor, which has none, the shares' own;
/// the fallback when neither is there.
fn rebuilt_name(c: &Compute, whole: &str, shares: Option<Text>) -> Text {
    let in_text = multisig_config::parse_named(c, whole)
        .ok()
        .and_then(|(_, n)| n);
    name_or_sheets(in_text, shares, EN.act_restored_wallet)
}

/// The name found in a rebuilt text, or when it has none (a Taproot
/// descriptor) the sheets' own when not blank, or `fallback`.
pub(crate) fn name_or_sheets(in_text: Option<Text>, sheets: Option<Text>, fallback: &str) -> Text {
    in_text
        .or_else(|| sheets.filter(|n| !n.trim().is_empty()))
        .unwrap_or_else(|| Text::of(fallback))
}

/// Costs what `parse` and `read_wallet` cost. Whether the rebuilt text
/// reads as a wallet.
pub fn readable(c: &Compute, whole: &str) -> bool {
    multisig_config::parse(c, whole).is_ok() || crate::wallet::read_wallet(c, whole).is_ok()
}

/// Costs a parse of the wallet and four address derivations. Whether
/// `texts`, a whole config or some sheets of a split backup, rebuild
/// `wallet` as Restore rebuilds it ([`merge`], then
/// [`crate::wallet::read_wallet`]): the rebuilt wallet's first receive
/// and change addresses on `network` are the wallet's own.
pub fn rebuilds(c: &Compute, network: Network, wallet: &Wallet, texts: &[impl AsRef<str>]) -> bool {
    let Some(whole) = merge(texts).ok().and_then(|m| m.whole) else {
        return false;
    };
    let Ok(back) = crate::wallet::read_wallet(c, &whole) else {
        return false;
    };
    [false, true].into_iter().all(|change| {
        match (
            wallet.policy().address_at(c, network, change, 0),
            back.address_at(c, network, change, 0),
        ) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        }
    })
}

/// Costs [`rebuilds`] for each set of sheets tried. Whether the split
/// sheets of `plan` rebuild `wallet`: every set of `m` of them where
/// there are at most ten such sets, else the first `m` and the last `m`.
pub(crate) fn sheets_rebuild(
    c: &Compute,
    network: Network,
    wallet: &Wallet,
    plan: &[Vec<usize>],
) -> bool {
    let Some(sheets) = plan
        .iter()
        .map(|row| split_share(wallet, row).map(|s| Text::of(&s)))
        .collect::<Option<Vec<Text>>>()
    else {
        return false;
    };
    let (m, _) = Session::quorum(wallet);
    let n = sheets.len();
    if m == 0 || m > n {
        return false;
    }
    let sets: Vec<Vec<usize>> = if choose(n, m) <= 10 {
        (0u32..1 << n)
            .filter(|mask| mask.count_ones() as usize == m)
            .map(|mask| (0..n).filter(|i| mask & (1 << i) != 0).collect())
            .collect()
    } else {
        vec![(0..m).collect(), (n - m..n).collect()]
    };
    sets.iter().all(|set| {
        let some: Vec<Text> = set.iter().map(|&j| sheets[j].duplicate()).collect();
        rebuilds(c, network, wallet, &some)
    })
}

/// Whether two of a policy's keys name one fingerprint: one seed at two
/// of its keys, which [`merge`] cannot tell apart from sheets of two
/// wallets and refuses. A lookup.
pub(crate) fn repeats_a_seed(policy: &faraday_bip::policy::WalletPolicy) -> bool {
    let fps: Vec<_> = policy
        .keys()
        .iter()
        .filter_map(|k| k.fingerprint())
        .collect();
    fps.iter().enumerate().any(|(i, f)| fps[..i].contains(f))
}

/// Costs [`rebuilds`]. `wallet`'s whole Coldcard config, where it has one
/// and it rebuilds the wallet.
pub fn checked_config(c: &Compute, network: Network, wallet: &Wallet) -> Option<Text> {
    let text = Text::of(&crate::wallet_backup::multisig_config(wallet, None)?);
    rebuilds(c, network, wallet, core::slice::from_ref(&text)).then_some(text)
}

/// `n` choose `k`, saturating.
fn choose(n: usize, k: usize) -> usize {
    (0..k).fold(1usize, |acc, i| acc.saturating_mul(n - i) / (i + 1))
}

/// BIP-341's unspendable internal key, a Taproot multisig's.
const NUMS: &str = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";

/// Restoring a wallet: the cards' state.
#[derive(Default)]
pub struct RestoreState {
    /// The open card.
    pub open: Option<u8>,
    /// Cards closed as done.
    pub done: [bool; rstep::COUNT as usize],
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// The Inbox shares chosen, by name.
    pub shares: Vec<Text>,
    /// The wallet, by index into the session: read from its description,
    /// or made from the seeds.
    pub wallet: Option<usize>,
    /// The wallet came from its description: the seeds are matched to its
    /// keys.
    pub described: bool,
    /// The last refusal.
    pub error: Option<Text>,
    /// The kind, the quorum and the seeds and cosigners' xpubs in hand,
    /// for a wallet made from the seeds.
    pub seeds: Option<seeds::SeedsState>,
    /// Kind's **More kinds** is expanded.
    pub more_kinds: bool,
    /// The open slot's **Other forms** is expanded.
    pub slot_forms: bool,
    /// The description's keys marked Not here, by place.
    pub absent: Vec<u8>,
    /// Made from a loaded key (its row's From this key): the wallet is
    /// made, not restored.
    pub from_key: bool,
    /// The screen Restore was opened from: Back and Done go there. None
    /// is Wallets start.
    pub opener: Option<Screen>,
    /// Seeds' row, a vault already on a stick: on, that one stick is
    /// brought in and its vault unlocked before any seed is typed
    /// (`docs/BACKUP-FLOW.md` §11). Restore asks nothing else about vaults.
    pub vaults: VaultAsk,
    /// The number the wallet's backup flow is tied to (`Faraday::owner_seq`).
    pub owner: u64,
    /// Opened by a check that found no wallet loaded: the check, and the
    /// screen it was opened from. The wallet read here returns to it.
    pub then: Option<(Check, Screen)>,
}

impl RestoreState {
    /// Where Back and Done go: the check's screen, the screen Restore was
    /// opened from, or Wallets start.
    pub fn back_to(&self) -> Screen {
        match self.then {
            Some((_, from)) => from,
            None => self.opener.unwrap_or(Screen::WalletsStart),
        }
    }
}

/// A check that leads into loading a wallet when none is loaded, and
/// takes the person back to itself with the wallet in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Check {
    /// Check an address: back to its scanner.
    Address,
    /// Decode a transaction: back to the decoded transaction.
    Decode,
}

impl RestoreState {
    /// Whether a seed, a cosigner or a key of the description is already
    /// in: Seeds' row for a vault on a stick is offered before that.
    pub fn seeds_in(&self, session: &Session) -> bool {
        let typed = self.seeds.as_ref().is_some_and(|s| {
            !s.keys.is_empty() || s.cosigners.iter().any(|c| !c.trim().is_empty())
        });
        let held = self.described
            && self
                .wallet
                .and_then(|i| session.wallets.get(i))
                .is_some_and(|w| session.slots(w).iter().any(|s| s.held_by.is_some()));
        typed || held
    }

    /// The person turned on Seeds' row, a vault already on a stick.
    pub fn uses_existing_vault(&self) -> bool {
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
        if kind.multi() {
            v.push(rstep::QUORUM);
        }
        v.extend([rstep::DESCRIPTION, rstep::SEEDS, rstep::CHECK, rstep::DONE]);
        v
    }

    /// The step open: the one `open` names, or with none open the first not
    /// done (the last when all are): a step is always open.
    pub fn open_step(&self) -> Option<u8> {
        let steps = self.steps();
        self.open
            .or_else(|| steps.iter().copied().find(|&k| !self.done[k as usize]))
            .or_else(|| steps.last().copied())
    }

    /// What Your choices says of step `k`: the answer, or `None` for not
    /// chosen yet. A lookup, never a secret: the seeds' row counts the
    /// slots filled.
    fn row_value(&self, app: &Faraday, k: u8) -> Option<Text> {
        let wl = self.wallet.and_then(|i| app.session.wallets.get(i));
        let reached = self.done[k as usize] || self.open_step() == Some(k);
        match k {
            rstep::KIND => Some(match wl.filter(|_| self.described) {
                Some(w) => Session::shape(w),
                None => Text::of(self.kind().name()),
            }),
            rstep::QUORUM if reached => self.seeds.as_ref().map(|s| {
                Text::fill(
                    EN.restore_threshold,
                    &[&Text::num(s.m as u64), &Text::num(s.n as u64)],
                )
            }),
            rstep::DESCRIPTION => match wl {
                Some(w) if self.described => Some(Text::of(&w.name)),
                _ if self.done[k as usize] => Some(Text::of(EN.restore_none)),
                _ => None,
            },
            rstep::SEEDS if reached => {
                let (here, all) = restore_filled(app);
                Some(Text::fill(
                    EN.restore_of_filled,
                    &[&Text::num(here as u64), &Text::num(all as u64)],
                ))
            }
            // The wallet's name: its first address is derived when the
            // card is drawn, and one held back is not shown by a lookup.
            rstep::CHECK if reached && self.done[rstep::SEEDS as usize] => {
                wl.map(|w| Text::of(&w.name))
            }
            _ => None,
        }
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

    /// The flow's state to change, only for the seeds' state that
    /// `Faraday::seeds_mut` hands out by reference for every screen.
    pub(crate) fn state_mut(&mut self) -> Option<&mut RestoreState> {
        self.state.as_mut()
    }

    /// Shows `error` as the last refusal.
    pub(crate) fn set_error(&mut self, error: impl AsRef<str>) {
        if let Some(r) = self.state.as_mut() {
            r.error = Some(Text::of(error.as_ref()));
        }
    }

    /// How far the column is scrolled, units, while the flow is open.
    pub(crate) fn scroll_y(&self) -> Option<f32> {
        self.state.as_ref().map(|r| r.scroll.y)
    }

    /// The column scrolled to `y` units, while the flow is open.
    pub(crate) fn set_scroll_y(&mut self, y: f32) {
        if let Some(r) = self.state.as_mut() {
            r.scroll.y = y;
        }
    }

    /// Sets the column's scroll, as the drawing pass noted it.
    pub(crate) fn set_scroll(&mut self, to: flow::Scroll) {
        if let Some(r) = self.state.as_mut() {
            r.scroll = to;
        }
    }

    /// The vault's stick is brought in and unlocked: Seeds opens.
    pub(crate) fn open_after_setup(&mut self) {
        if let Some(r) = self.state.as_mut() {
            r.open = Some(crate::rstep::SEEDS);
            r.scroll.follow = true;
        }
    }

    /// The wallet was made from the typed seeds, not read from a
    /// description: its Seeds card is done.
    pub(crate) fn made_from_seeds(&mut self) {
        if let Some(r) = self.state.as_mut()
            && r.wallet.is_some()
        {
            r.described = false;
            r.next(crate::rstep::SEEDS);
        }
    }

    /// Starts the flow on `state`.
    pub(crate) fn set(&mut self, state: RestoreState) {
        self.state = Some(state);
    }

    /// Wallet `gone` left the session list: the place kept moves up, and
    /// whether the flow was on it (it must end).
    pub(crate) fn wallet_left(&mut self, gone: usize) -> bool {
        let Some(r) = self.state.as_mut() else {
            return false;
        };
        match r.wallet {
            Some(w) => match crate::wallet::place_after_removal(w, gone) {
                Some(w) => {
                    r.wallet = Some(w);
                    false
                }
                None => true,
            },
            None => false,
        }
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

/// Restore's steps, plugged into Back, Forward and the choices that move
/// on (F103, F104).
pub(crate) struct RestoreSteps;

/// The one [`RestoreSteps`].
pub(crate) static RESTORE_STEPS: RestoreSteps = RestoreSteps;

impl crate::trail::StepFlow for RestoreSteps {
    fn steps(&self, app: &Faraday) -> Option<crate::trail::Steps> {
        let r = app.restore()?;
        let mut s = crate::trail::Steps::new(r.steps(), r.open_step());
        s.answered = r
            .steps()
            .into_iter()
            .filter(|&k| r.done[k as usize])
            .collect();
        // The wallet made from its seeds stays made: Back goes no further
        // than Check.
        s.first = if r.done[rstep::SEEDS as usize] && r.wallet.is_some() {
            Some(rstep::CHECK)
        } else {
            None
        };
        Some(s)
    }

    fn open(&self, _: &Faraday, k: u8) -> Action {
        Action::RStep(k)
    }

    fn name(&self, _: &Faraday, k: u8) -> &'static str {
        crate::screens::RSTEPS[usize::from(k)]
    }

    fn contained(&self) -> Option<&dyn crate::flow::Contained> {
        Some(self)
    }

    fn moves_on(&self, app: &Faraday, chosen: Action) -> Option<Action> {
        let r = app.restore()?;
        match chosen {
            // A kind read from a description is not chosen here.
            Action::RKind(_) if !r.described => Some(Action::RNext(rstep::KIND)),
            _ => None,
        }
    }
}

impl crate::flow::Contained for RestoreSteps {
    fn rows(&self, app: &Faraday) -> Vec<crate::flow::Row> {
        let Some(r) = app.restore() else {
            return Vec::new();
        };
        let open = r.open_step();
        r.steps()
            .into_iter()
            // The last step is the end card: it has no answer to list.
            .filter(|&k| k != rstep::DONE)
            .map(|k| crate::flow::Row {
                label: Text::of(crate::screens::RSTEPS[usize::from(k)]),
                value: r.row_value(app, k),
                mono: false,
                opens: k,
                current: open == Some(k),
                mark: None,
                address: false,
            })
            .collect()
    }

    fn hide_shown(&self, app: &mut crate::Faraday) {
        // No step of this flow shows a secret.
        let _ = app;
    }

    fn count_fixed(&self, _: &Faraday) -> bool {
        false
    }

    /// Restore's act code clears no answer when an earlier one changes: a
    /// kind pressed again only resets the quorum's counts.
    fn still_fits(&self, _: &Faraday, _changed: u8, _later: u8) -> bool {
        true
    }
}

/// Restore's slots: how many are filled (a seed here, a cosigner's
/// xpub, or Not here), of how many. A lookup.
pub(crate) fn restore_filled(app: &Faraday) -> (usize, usize) {
    let Some(r) = app.restore() else {
        return (0, 0);
    };
    if r.described
        && let Some(w) = r.wallet.and_then(|i| app.session.wallets.get(i))
    {
        let slots = app.session.slots(w);
        let filled = slots
            .iter()
            .enumerate()
            .filter(|(k, s)| s.held_by.is_some() || r.absent.contains(&(*k as u8)))
            .count();
        return (filled, slots.len());
    }
    let Some(s) = r.seeds.as_ref() else {
        return (0, 0);
    };
    let xpubs = s
        .cosigners
        .iter()
        .filter(|c| crate::seeds::cosigner_key(c).is_some())
        .count();
    (app.seeds_here(s).len() + xpubs, s.n.max(1))
}

impl Faraday {
    /// Whether Restore has a vault's stick still to bring in and unlock:
    /// its seeds wait for it.
    pub(crate) fn restore_setup_pending(&self) -> bool {
        self.restore.state().is_some_and(|r| {
            self.setup_pending(
                r.owner,
                r.uses_existing_vault(),
                r.done[rstep::SEEDS as usize],
            )
        })
    }

    /// Seeds' row, a vault already on a stick: on brings in that one stick
    /// and unlocks its vault before any seed is typed; off drops it. It is
    /// fixed once a seed is in, the wallet is made or the vault is open.
    fn restore_existing_vault(&mut self, on: bool) {
        let Some(r) = self.restore.state() else {
            return;
        };
        if r.done[rstep::SEEDS as usize]
            || r.uses_existing_vault() == on
            || (on && r.seeds_in(&self.session))
        {
            return;
        }
        let owner = r.owner;
        if self.setup_vault_open(owner) {
            self.toast(EN.create_vaults_locked);
            return;
        }
        if let Some(r) = self.restore.state.as_mut() {
            r.vaults = VaultAsk {
                answer: on.then_some(true),
                count: 1,
            };
        }
        self.wallet_vaults_changed(owner);
        if on {
            self.wallet_flow_begin(owner);
        }
    }

    /// The Restore flow's state, while it is open.
    pub fn restore(&self) -> Option<&RestoreState> {
        self.restore.state()
    }

    /// Costs what the actions below cost, the keys' and wallets' work among
    /// it: handles the Restore wallet flow's actions (`Action::R*`, `RestoreWallet`, `RestoreShares`, `RestoreShareAt`, `KeyWallet`).
    pub(crate) fn restore_act(&mut self, action: Action, c: &Compute) {
        match action {
            Action::RFromVault(v, r) => self.restore_from_vault(v, r),
            Action::RestoreWallet if self.restore_setup_pending() => {
                self.screen = Screen::Restore;
            }
            // A check with no wallet loaded: the same Restore, and the
            // wallet read in it goes back to the check.
            Action::RestoreFor(check) => {
                let owner = self.fresh_owner();
                self.kept.pooled_drop();
                self.restore.state = Some(RestoreState {
                    owner,
                    then: Some((check, self.screen)),
                    ..RestoreState::new()
                });
                self.screen = Screen::Restore;
            }
            Action::RestoreWallet => {
                let owner = self.fresh_owner();
                self.kept.pooled_drop();
                self.restore.state = Some(RestoreState {
                    owner,
                    opener: Some(self.screen),
                    ..RestoreState::new()
                });
                self.screen = Screen::Restore;
            }
            Action::RExistingVault(on) => self.restore_existing_vault(on),
            // A vault's stick still to bring in and unlock comes before
            // the seeds.
            Action::RStep(k) if k == rstep::SEEDS && self.restore_setup_pending() => {}
            Action::RStep(k) => {
                if let Some(r) = self.restore.state.as_mut() {
                    // A step is always open: a press on the open one does
                    // nothing.
                    if r.open_step() != Some(k) {
                        r.open = Some(k);
                        r.scroll.follow = true;
                    }
                }
            }
            Action::RNext(k) => {
                if let Some(r) = self.restore.state.as_mut() {
                    r.next(k);
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
                    .filter(|(_, i)| crate::create::wallet_flow_kind(i) == FileKind::Wallet)
                    .map(|(k, _)| k)
                    .collect();
                if let [one] = files.as_slice() {
                    self.act(Action::RUse(*one));
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
            Action::RSlotForms => {
                if let Some(r) = self.restore.state.as_mut() {
                    r.slot_forms = !r.slot_forms;
                }
            }
            Action::RSlotForm(code) => {
                if matches!(self.sheet, Some(crate::Sheet::OtherForms(_))) {
                    self.sheet = None;
                }
                if let Some(r) = self.restore.state.as_mut() {
                    r.error = None;
                }
                self.act(Action::Entry(None));
                if self.screen == Screen::Entry
                    && let Some(f) = crate::forms::Form::from_code(code)
                {
                    self.entry.choose_form(f);
                }
            }
            Action::RSlotScan => {
                if self.may_load_keys() {
                    if let Some(r) = self.restore.state.as_mut() {
                        r.error = None;
                    }
                    self.entry.reset();
                    self.entry.set_back(Some(Screen::Restore));
                    self.act(Action::ScanSeed);
                }
            }
            Action::RUse(i) => {
                if let Some(item) = self.inbox.get(i) {
                    let text = Text::from_utf8_lossy(&item.bytes);
                    let name = faraday_bip::multisig_config::parse_named(c, &text)
                        .ok()
                        .and_then(|(_, n)| n)
                        .unwrap_or_else(|| Text::of(&stem(&item.name)));
                    let source = item.name.clone();
                    self.restore_wallet(&name, &text, &source, wallet::Origin::Named);
                }
            }
            Action::RShare(i) => {
                if let Some(item) = self.inbox.get(i) {
                    let n = Text::of(&item.name);
                    if let Some(r) = self.restore.state.as_mut() {
                        if let Some(p) = r.shares.iter().position(|x| *x == n) {
                            r.shares.remove(p);
                        } else {
                            r.shares.push(n);
                        }
                    }
                }
            }
            Action::RRebuild => match self.restore_merged(c) {
                Ok(m) => match m.whole {
                    Some(whole) => {
                        let name = rebuilt_name(c, &whole, m.name);
                        self.restore_wallet(
                            &name,
                            &whole,
                            EN.act_its_shares,
                            wallet::Origin::Named,
                        );
                    }
                    None => {
                        if let Some(r) = self.restore.state.as_mut() {
                            r.error = Some(Text::fill(
                                EN.act_of_keys_in_hand,
                                &[&Text::num(m.have.len() as u64), &Text::num(m.n as u64)],
                            ));
                        }
                    }
                },
                Err(e) => {
                    if let Some(r) = self.restore.state.as_mut() {
                        r.error = Some(e);
                    }
                }
            },
            Action::RSeeds => {
                if let Some(r) = self.restore.state.as_mut() {
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
                    opener: Some(self.screen),
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
                let shares: Vec<usize> = self
                    .inbox
                    .iter()
                    .enumerate()
                    .filter(|(_, i)| i.kind == FileKind::Share)
                    .map(|(k, _)| k)
                    .collect();
                self.restore_open_on_shares(&shares);
            }
            Action::RestoreShareAt(k) => self.restore_open_on_shares(&[k]),
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

    /// Costs what `key_at` costs, once for each slot of fingerprint `fp`.
    /// Why Restore does not take the seed whose key is `fp`, if it does
    /// not: with the description known, not one of its keys; from the
    /// seeds alone, no slot left. Once the wallet is made and Keys is
    /// closed, a key added is no seed of Restore's: nothing to refuse.
    /// `key_at` gives the seed's extended public key at a path: a slot of
    /// its fingerprint is its slot only when the slot's key is that key,
    /// all of it, not its four bytes alone. Without it the fingerprint
    /// finds the slot, and the key is matched once it is loaded
    /// ([`Faraday::restore_took`]).
    pub(crate) fn restore_refusal(
        &self,
        fp: faraday_bip::keys::Fingerprint,
        key_at: Option<&KeyAt<'_>>,
    ) -> Option<Text> {
        let r = self.restore.state.as_ref().filter(|r| !r.keys_made())?;
        if r.described {
            let w = self.session.wallets.get(r.wallet?)?;
            let ours = self.session.slots(w).iter().any(|s| {
                s.fingerprint == Some(fp)
                    && match (key_at, s.key.as_ref()) {
                        (Some(at), Some(k)) => k.path().and_then(at).is_some_and(|x| {
                            x.public_key == k.xpub().public_key
                                && x.chain_code == k.xpub().chain_code
                        }),
                        _ => true,
                    }
            });
            return (!ours).then(|| Text::of(NOT_A_KEY));
        }
        let s = r.seeds.as_ref()?;
        (!s.keys.contains(&fp.0) && s.first_empty().is_none())
            .then(|| Text::of(EN.restore_refusal_every))
    }

    /// Costs a PBKDF2, and a key derivation for each slot of the seed's
    /// fingerprint. Why Restore does not take the seed words `m` make with
    /// the passphrase typed on Add a key, if it does not
    /// ([`Faraday::restore_refusal`]), its whole key matched.
    pub(crate) fn restore_refusal_typed(
        &self,
        c: &Compute,
        m: &faraday_bip::bip39::Mnemonic,
    ) -> Option<Text> {
        let pass = self.entry.state().passphrase.as_str();
        self.restore_refusal_of(c, || faraday_keys::identity::Material::Words(m), pass)
    }

    /// Costs what [`Faraday::restore_refusal_typed`] costs. Why Restore
    /// does not take the key `material` makes with `pass`, if it does not,
    /// before anything is loaded.
    pub(crate) fn restore_refusal_of<'a>(
        &self,
        c: &Compute,
        material: impl Fn() -> faraday_keys::identity::Material<'a>,
        pass: &str,
    ) -> Option<Text> {
        let network = self.session.network();
        let fp = faraday_keys::identity::fingerprint(c, material(), pass, network)?;
        let at = |p: &faraday_bip::bitcoin::bip32::DerivationPath| {
            faraday_keys::identity::xpub_at(c, material(), pass, network, p)
        };
        self.restore_refusal(fp, Some(&at))
    }

    /// A seed for Restore's open slot, loaded: matched to the
    /// description's keys or put in the slot.
    pub(crate) fn restore_took(&mut self, fp: faraday_bip::keys::Fingerprint) {
        let c = self.compute();
        // The key is loaded, the last of its fingerprint: its whole key at
        // each slot's path.
        let keys = &self.session.keys;
        let just = keys
            .all()
            .enumerate()
            .filter(|(_, i)| i.fingerprint == fp)
            .map(|(k, _)| k)
            .last();
        let at = |p: &faraday_bip::bitcoin::bip32::DerivationPath| -> Option<Xpub> {
            let k = just?;
            let text = keys
                .key_text(&c, k, &faraday_keys::texts::Spec::Path(p.clone()))?
                .ok()?;
            text.rsplit(']').next()?.parse().ok()
        };
        if let Some(why) = self.restore_refusal(fp, Some(&at)) {
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
        let places: Vec<usize> = self
            .restore
            .state()
            .and_then(|r| r.wallet.filter(|_| r.described))
            .and_then(|i| self.session.wallets.get(i))
            .map(|w| {
                self.session
                    .slots(w)
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.fingerprint == Some(fp))
                    .map(|(k, _)| k)
                    .collect()
            })
            .unwrap_or_default();
        let Some(r) = self.restore.state.as_mut() else {
            return;
        };
        if !places.is_empty() {
            r.absent.retain(|&a| !places.contains(&usize::from(a)));
        } else if !r.described
            && let Some(s) = r.seeds.as_mut()
        {
            s.take(fp.0);
        }
        r.error = None;
    }

    /// A key removed from the session: the seed is no longer one Restore
    /// holds.
    pub(crate) fn restore_key_gone(&mut self, fp: faraday_bip::keys::Fingerprint) {
        if let Some(s) = self.restore.state.as_mut().and_then(|r| r.seeds.as_mut()) {
            s.drop_key(fp.0);
        }
    }

    /// Restore opened on its Description card with the Files shares `at`
    /// ticked, each only where it agrees with those ticked before it (one
    /// wallet's shares); a Restore already open and not yet made keeps
    /// what it has. The wallet is made at once when the shares ticked and
    /// those in the open vaults hold every key, as a share read while
    /// Restore waits does.
    fn restore_open_on_shares(&mut self, at: &[usize]) {
        let c = self.compute();
        let mut r = match self.restore.state.take() {
            Some(r) if !r.described && !r.done[rstep::SEEDS as usize] => r,
            _ => RestoreState {
                owner: self.fresh_owner(),
                opener: Some(self.screen),
                ..RestoreState::new()
            },
        };
        let mut texts = self.restore_texts_of(&r);
        for &k in at {
            let Some(i) = self.inbox.get(k).filter(|i| i.kind == FileKind::Share) else {
                continue;
            };
            if r.shares.iter().any(|s| i.name == s.as_str()) {
                continue;
            }
            let mut trial = texts.clone();
            trial.push(Text::from_utf8_lossy(&i.bytes));
            if restore::merge(&trial).is_ok() {
                texts = trial;
                r.shares.push(Text::of(&i.name));
            }
        }
        r.open = Some(rstep::DESCRIPTION);
        r.scroll.follow = true;
        self.restore.state = Some(r);
        self.screen = Screen::Restore;
        let whole = self.restore_merged(&c).is_ok_and(|m| m.whole.is_some());
        if whole {
            self.act(Action::RRebuild);
        }
    }

    /// The texts of the Files shares a Restore has ticked.
    fn restore_texts_of(&self, r: &RestoreState) -> Vec<Text> {
        self.inbox
            .iter()
            .filter(|i| i.kind == FileKind::Share && r.shares.iter().any(|s| i.name == s.as_str()))
            .map(|i| Text::from_utf8_lossy(&i.bytes))
            .collect()
    }

    /// A description or a share read by the camera while Restore waits
    /// for one: the wallet loads, or the share is taken. Returns whether
    /// it was.
    pub(crate) fn restore_arrived(&mut self, at: usize) -> bool {
        let c = self.compute();
        let Some((kind, name)) = self
            .inbox
            .get(at)
            .map(|i| (crate::create::wallet_flow_kind(i), i.name.clone()))
        else {
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
                if !r.shares.iter().any(|s| name == s.as_str()) {
                    r.shares.push(Text::of(&name));
                }
                r.open = Some(rstep::DESCRIPTION);
                r.scroll.follow = true;
                let whole = self.restore_merged(&c).is_ok_and(|m| m.whole.is_some());
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
            r.error = Some(Text::fill(
                EN.restore_described_make_still,
                &[
                    &Text::num(open as u64),
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
            rec.public_text(field::WALLET).map(Text::of),
            Text::of(&crate::vault::wallet_name_in(
                rec,
                field::WALLET_NAME,
                EN.restore_from_vault_wallet,
            )),
        ) else {
            return;
        };
        let source = open.name.clone();
        let ticks = self
            .vaults
            .described(&c, open.id, r)
            .map(|d| (d.policy.clone(), rec.keys_checked()));
        let before = self.session.wallets.len();
        self.restore_wallet_in(&name, &text, &source, wallet::Origin::Vault, false);
        // The record's ticks, key by key, to a wallet this added.
        if let Some((own, checked)) = ticks
            && let Some(i) = self.restore.state().and_then(|s| s.wallet)
            && i >= before
            && let Some(w) = self.session.wallets.get_mut(i)
        {
            w.merge_key_ticks(&own, &crate::vault::record_ticks(&own, &checked));
        }
        let Some(w) = self
            .restore
            .state()
            .and_then(|s| s.wallet)
            .and_then(|i| self.session.wallets.get(i))
        else {
            return;
        };
        // The vault's keys for this wallet's slots load with it.
        let want: Vec<Text> = self
            .session
            .slots(w)
            .iter()
            .filter_map(|s| s.fingerprint.map(|f| Text::of(&fp_text(f))))
            .collect();
        let open = &self.vaults.open[v];
        // Keys load only where they may: with a stick attached, or before
        // the self-test has passed, the wallet comes alone.
        let may = self.may_load_keys();
        let keep: BTreeSet<usize> = self
            .vaults
            .records(open.id)
            .of(kind::KEY)
            .filter(|(_, k)| {
                may && vault::screen::key_fingerprint(&c, self, *k)
                    .is_some_and(|fp| want.iter().any(|w| *w == *fp))
            })
            .map(|(i, _)| i)
            .chain([r])
            .collect();
        let all: BTreeSet<usize> = (0..self.vaults.records(open.id).len()).collect();
        let saved = self
            .vaults
            .swap_skip(v, all.difference(&keep).copied().collect());
        // The keys are stretched in a job; the screen is Restore's when
        // they are in.
        let then: crate::vault::load_job::Then = Box::new(move |app, keys, _| {
            app.screen = Screen::Restore;
            if keys > 0 {
                app.toast(&Text::fill(
                    EN.restore_from_vault_count,
                    &[
                        &Text::num(keys as u64),
                        (if keys == 1 {
                            EN.restore_from_vault_key
                        } else {
                            EN.restore_from_vault_keys
                        }),
                        &source,
                    ],
                ));
            }
        });
        self.vault_load(v, false, then);
        self.vaults.swap_skip(v, saved);
    }

    /// Costs what `read_kept` costs. The texts of the shares chosen on the
    /// restore screen and those kept in the open vaults that rebuild one
    /// wallet with them ([`Faraday::restore_pool`]), for tests.
    pub fn restore_share_texts(&self, c: &Compute) -> Vec<Text> {
        self.restore_pool(c).texts.clone()
    }

    /// What the pool adds up to: an error saying so where two different
    /// wallets of one checksum complete.
    pub fn restore_merged(&self, c: &Compute) -> Result<restore::Merged, Text> {
        let pool = self.restore_pool(c);
        match &pool.two {
            Some(e) => Err(e.duplicate()),
            None => restore::merge(&pool.texts),
        }
    }

    /// Costs what `read_kept` costs the first time after the shares, the
    /// Inbox or a vault's share records change. The pool of shares, kept
    /// by what it is worked out from.
    pub fn restore_pool(&self, c: &Compute) -> std::rc::Rc<restore::Pool> {
        let key = self.restore_pool_key();
        if key == 0 {
            self.kept.pooled_drop();
        }
        if let Some(p) = self.kept.pooled(key) {
            return p;
        }
        let p = std::rc::Rc::new(self.restore_pool_work(c));
        self.kept.pooled_put(key, p.clone());
        p
    }

    /// A sign of everything the pool is worked out from: the shares
    /// chosen, how often the Inbox's files and the open vaults' records
    /// changed, and which vaults are open. No file is read.
    fn restore_pool_key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        let Some(r) = self.restore.state.as_ref() else {
            return 0;
        };
        r.shares.hash(&mut h);
        self.inbox.edits().hash(&mut h);
        self.vaults.edits().hash(&mut h);
        for o in &self.vaults.open {
            o.id.hash(&mut h);
        }
        h.finish() | 1
    }

    /// The pool of shares. The shares in hand are returned as they are
    /// where they merge whole on their own. Otherwise each wallet the
    /// vaults' share records name, whose description reads and has
    /// sheets, is tried on its own (key order told apart by
    /// [`restore_candidate_key`]): the shares in hand and the records'
    /// texts that agree with it on quorum, Format and each key's
    /// fingerprint, derivation and extended key, the records that name it
    /// first. It is complete where what they merge to reads as it. The
    /// first complete one is taken; two that share a checksum are refused;
    /// where none completes, the one with the most keys.
    fn restore_pool_work(&self, c: &Compute) -> restore::Pool {
        let Some(r) = self.restore.state.as_ref() else {
            return restore::Pool::of(Vec::new(), None);
        };
        let hand_texts: Vec<Text> = self
            .inbox
            .iter()
            .filter(|i| i.kind == FileKind::Share && r.shares.iter().any(|s| i.name == s.as_str()))
            .map(|i| Text::from_utf8_lossy(&i.bytes))
            .collect();
        if !hand_texts.is_empty() && restore::merge(&hand_texts).is_ok_and(|m| m.whole.is_some()) {
            return restore::Pool::of(hand_texts, None);
        }
        use faraday_vault::records::{field, kind};
        let mut cands: Vec<Candidate> = Vec::new();
        let mut recs: Vec<PoolRecord> = Vec::new();
        for (vi, o) in self.vaults.open.iter().enumerate() {
            for (_, rec) in self.vaults.records(o.id).of(kind::WALLET_SHARE) {
                let Some(t) = rec.public_text(field::SHARE) else {
                    continue;
                };
                let Some(own) = parts(t) else {
                    continue;
                };
                let Some(policy) = rec
                    .public_text(field::SHARE_WALLET)
                    .and_then(|d| self.kept.read_kept(c, d))
                else {
                    continue;
                };
                let key = restore_candidate_key(&policy);
                let at = match cands.iter().position(|k| k.key == key) {
                    Some(at) => at,
                    None => {
                        let name = Text::of(&crate::vault::wallet_name_in(
                            rec,
                            field::SHARE_WALLET_NAME,
                            EN.vault_wallet,
                        ));
                        match Candidate::of(key, &policy, name) {
                            Some(k) => {
                                cands.push(k);
                                cands.len() - 1
                            }
                            None => {
                                recs.push(PoolRecord {
                                    vault: vi,
                                    text: Text::of(t),
                                    own,
                                    names: None,
                                });
                                continue;
                            }
                        }
                    }
                };
                recs.push(PoolRecord {
                    vault: vi,
                    text: Text::of(t),
                    own,
                    names: Some(at),
                });
            }
        }
        let built: Vec<Option<(Vec<Text>, bool)>> = (0..cands.len())
            .map(|ci| self.restore_pool_of(c, &hand_texts, &cands, &recs, ci, None))
            .collect();
        let done: Vec<usize> = (0..cands.len())
            .filter(|i| built[*i].as_ref().is_some_and(|b| b.1))
            .collect();
        for (n, a) in done.iter().enumerate() {
            for b in &done[n + 1..] {
                if cands[*a].checksum == cands[*b].checksum {
                    let e = self.restore_two_text(c, &hand_texts, &cands, &recs, (*a, *b));
                    return restore::Pool::of(hand_texts, Some(e));
                }
            }
        }
        if let Some(i) = done.first() {
            let texts = built.into_iter().nth(*i).flatten().map(|b| b.0);
            return restore::Pool::of(texts.unwrap_or(hand_texts), None);
        }
        let mut best: Option<(usize, usize)> = None;
        for (i, b) in built.iter().enumerate() {
            let Some((texts, _)) = b else { continue };
            let have = restore::merge(texts).map_or(0, |m| m.have.len());
            if best.is_none_or(|(_, h)| have > h) {
                best = Some((i, have));
            }
        }
        match best {
            Some((i, _)) => {
                let texts = built.into_iter().nth(i).flatten().map(|b| b.0);
                restore::Pool::of(texts.unwrap_or(hand_texts), None)
            }
            None => restore::Pool::of(hand_texts, None),
        }
    }

    /// One candidate's pool from `hand` and the records (of the open
    /// vault `only` alone where it is given), and whether it completes as
    /// that candidate; `None` where a text in hand is not of it.
    fn restore_pool_of(
        &self,
        c: &Compute,
        hand: &[Text],
        cands: &[Candidate],
        recs: &[PoolRecord],
        ci: usize,
        only: Option<usize>,
    ) -> Option<(Vec<Text>, bool)> {
        let cand = &cands[ci];
        if !hand
            .iter()
            .filter_map(|t| parts(t.as_str()))
            .all(|p| cand.holds(&p))
        {
            return None;
        }
        let mut pool: Vec<Text> = hand.to_vec();
        let named = recs.iter().filter(|r| r.names == Some(ci));
        let others = recs.iter().filter(|r| r.names != Some(ci));
        for rec in named.chain(others) {
            if only.is_some_and(|v| v != rec.vault)
                || !cand.holds(&rec.own)
                || pool.iter().any(|x| x.trim() == rec.text.trim())
            {
                continue;
            }
            let mut trial = pool.clone();
            trial.push(rec.text.duplicate());
            if restore::merge(&trial).is_ok() {
                pool = trial;
            }
        }
        let complete = restore::merge(&pool)
            .ok()
            .and_then(|m| m.whole)
            .and_then(|w| self.kept.read_kept(c, &w))
            .is_some_and(|p| restore_candidate_key(&p) == cand.key);
        Some((pool, complete))
    }

    /// What the screen says where wallets `two` of one checksum both
    /// rebuild: both named, and the steps from this screen that end it,
    /// each tried: a share file in the Inbox only one of them holds, or
    /// only one open vault.
    fn restore_two_text(
        &self,
        c: &Compute,
        hand: &[Text],
        cands: &[Candidate],
        recs: &[PoolRecord],
        two: (usize, usize),
    ) -> Text {
        let (a, b) = two;
        let ends = |hand: &[Text], only: Option<usize>| {
            let done = |ci| {
                self.restore_pool_of(c, hand, cands, recs, ci, only)
                    .is_some_and(|p| p.1)
            };
            done(a) != done(b)
        };
        let r = self.restore.state.as_ref();
        let tick = self.inbox.iter().any(|i| {
            i.kind == FileKind::Share
                && !r.is_some_and(|r| r.shares.iter().any(|s| i.name == s.as_str()))
                && {
                    let t = Text::from_utf8_lossy(&i.bytes);
                    parts(t.as_str()).is_some() && {
                        let mut more = hand.to_vec();
                        more.push(t);
                        ends(&more, None)
                    }
                }
        });
        // One vault alone that rebuilds one wallet its own records name.
        let vault = (0..self.vaults.open.len()).any(|v| {
            let names = |ci| recs.iter().any(|r| r.vault == v && r.names == Some(ci));
            let done = |ci| {
                self.restore_pool_of(c, hand, cands, recs, ci, Some(v))
                    .is_some_and(|p| p.1)
            };
            (done(a) && names(a) && !done(b)) || (done(b) && names(b) && !done(a))
        });
        let (na, nb) = if cands[a].name == cands[b].name {
            let tell = |k: &Candidate, other: &Candidate| {
                if k.tell == other.tell {
                    text!("{} ({}, {})", &*k.name, &*k.tell, &*k.checksum)
                } else {
                    text!("{} ({})", &*k.name, &*k.tell)
                }
            };
            (tell(&cands[a], &cands[b]), tell(&cands[b], &cands[a]))
        } else {
            (cands[a].name.duplicate(), cands[b].name.duplicate())
        };
        let mut out = Text::fill(EN.restore_two_wallets, &[&na, &nb]);
        if tick {
            out.push(' ');
            out.push_str(EN.restore_two_wallets_tick);
        }
        if vault {
            out.push(' ');
            out.push_str(if tick {
                EN.restore_two_wallets_vault_or
            } else {
                EN.restore_two_wallets_vault
            });
        }
        out
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
        self.restore_wallet_in(name, text, source, origin, true);
    }

    /// Costs what `restore_wallet` costs; `confirm` false is a vault's
    /// wallet, the person's own record, added with no sheet (F254).
    fn restore_wallet_in(
        &mut self,
        name: &str,
        text: &str,
        source: &str,
        origin: wallet::Origin,
        confirm: bool,
    ) {
        let c = self.compute();
        let added = if confirm {
            self.session.add_wallet(&c, name, text, source, origin)
        } else {
            self.session.add_wallet_kept(&c, name, text, source, origin)
        };
        match added {
            Ok(i) => self.restore_added(i, origin),
            Err(wallet::Refusal::Unconfirmed) => self.session.wallet_for_restore(),
            Err(e) => {
                if let Some(r) = self.restore.state.as_mut() {
                    r.error = Some(e.text());
                }
            }
        }
    }

    /// Costs what `restore_wallet` costs. Restore goes on from the wallet
    /// `i`, added now or once its sheet was confirmed.
    pub(crate) fn restore_added(&mut self, i: usize, origin: wallet::Origin) {
        self.wallets.set_wallet(i);
        let w = &self.session.wallets[i];
        let kind = create::NewKind::of(w.policy());
        let (m, n) = Session::quorum(w);
        // The flow that decided where the seeds go knows the wallet
        // once it is made from them.
        if let Some(owner) = self
            .restore
            .state()
            .filter(|r| r.uses_existing_vault())
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
            // On to the seeds.
            r.next(rstep::DESCRIPTION);
        }
        if origin != wallet::Origin::TypedSeeds
            && let Some(then) = self.restore.state().and_then(|r| r.then)
        {
            self.restore_return(then);
        }
    }

    /// Costs what the check costs. A wallet is loaded for a check: Restore
    /// ends and the check is shown again with it.
    fn restore_return(&mut self, (check, from): (Check, Screen)) {
        let Some(r) = self.restore.take() else {
            return;
        };
        self.flow_drop_unmade(self.wallet_flow_of_owner(r.owner));
        self.screen = from;
        match check {
            Check::Address => crate::address_check::reopen(self),
            Check::Decode => {
                let c = self.compute();
                self.decode.reread_ours(&c, &self.session);
            }
        }
    }
}

// What is typed in a cosigner's box is held: it takes any text, a key
// that is not an account key among it. The rest is not secret: the shares
// chosen are the names of Inbox files, which the Inbox answers for, and
// the seeds' state holds fingerprints (the seeds are the session's keys).
// A restore's vaults are written to their sticks while it is open. A lock
// drops it.
impl crate::at_lock::AtLock for RestoreArea {
    fn holds(&self) -> crate::at_lock::Holds {
        crate::at_lock::Holds::typed(
            self.state()
                .and_then(|r| r.seeds.as_ref())
                .is_some_and(|s| s.cosigner_typed()),
        )
    }
    fn lock(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A vault's share record for the backup flow: a wallet with two keys
    /// of one seed is not split, and gets none; Savings, whose sheets
    /// rebuild it, gets its record.
    #[test]
    fn a_share_record_is_made_only_for_a_wallet_its_sheets_rebuild() {
        use crate::testkit::key;
        let c = crate::testkit::compute();
        let mut app = crate::Faraday::new();
        app.session = crate::testkit::session();
        let (a, b) = ("m/48'/1'/0'/2'", "m/48'/1'/1'/2'");
        let twice = text!(
            "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
            key(0, a),
            key(0, b),
            key(1, a)
        );
        let mut wf = crate::backup_flow::WalletFlow::blank();
        wf.answers.omit = 1;
        wf.name = faraday_text::Text::of("Test");
        let savings = crate::testkit::savings();
        for (d, splits) in [(&*twice, false), (&*savings, true)] {
            let i = app
                .session
                .add_wallet_confirmed(&c, "Test", d, "Test", wallet::Origin::Named)
                .expect("reads");
            let w = &app.session.wallets[i];
            assert_eq!(crate::wallet_backup::splits(w), splits, "{d}");
            let id = w.id();
            for j in 0..3 {
                let got = crate::backup_flow::wallet_share_record(&app, &c, &wf, id, j);
                if splits {
                    assert!(matches!(got, Some(Ok(_))), "{d}: sheet {j}");
                } else {
                    assert!(got.is_none(), "{d}: sheet {j}");
                }
            }
        }
    }

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
            use std::fmt::Write as _;
            let mut s = text!("# Partial\n{whole}");
            for &i in which {
                let _ = write!(
                    s,
                    "\nDerivation: m/48'/0'/0'/2'\n{}: {}\n",
                    keys[i].0, keys[i].1
                );
            }
            s
        };
        let one = merge(&[share(&[0, 1])]).unwrap();
        assert!(one.whole.is_none(), "one share alone does not rebuild");
        let two = merge(&[share(&[0, 1]), share(&[1, 2])]).unwrap();
        let rebuilt = two.whole.expect("two shares rebuild");
        let a = multisig_config::parse(&c, &rebuilt).unwrap();
        let mut full = Text::of(whole);
        for k in keys {
            use std::fmt::Write as _;
            let _ = write!(full, "\nDerivation: m/48'/0'/0'/2'\n{}: {}\n", k.0, k.1);
        }
        assert_eq!(
            a.to_descriptor(),
            multisig_config::parse(&c, &full).unwrap().to_descriptor()
        );
        assert!(readable(&c, &rebuilt));
    }

    /// A config of fifteen keys (`Policy: 2 of 15`) is read; a file with
    /// a sixteenth key line, a policy over fifteen keys, more key lines
    /// than its policy names, or a derivation longer than a BIP-32 path
    /// can be is refused before it is copied into every key. Shares that
    /// add up to more keys than the policy are refused.
    #[test]
    fn a_config_is_read_up_to_fifteen_keys_and_refused_past_them() {
        const KEY: &str = "xpub6EeqK2JLwngrHJEQ4X4iqrySZV9qU3TgwMgf6NStLZa37AfNiHTtTE9ji1F9YQDLArJMLy8sw3Q2samVj5VQQjaaUHr5z2Hz57NWHJCfh31";
        let config = |n: usize, lines: usize, derivation: &str| {
            use std::fmt::Write as _;
            let mut s = text!("Policy: 2 of {n}\nFormat: P2WSH\nDerivation: {derivation}\n");
            for i in 0..lines {
                let _ = writeln!(s, "{i:08x}: {KEY}");
            }
            s
        };
        let path = "m/48'/0'/0'/2'";
        assert_eq!(parts(&config(15, 15, path)).map(|p| p.keys.len()), Some(15));
        assert!(parts(&config(15, 16, path)).is_none());
        assert!(parts(&config(16, 16, path)).is_none());
        assert!(parts(&config(3, 4, path)).is_none());
        let deepest = text!("m{}", Text::repeated("/2147483647h", 255).as_str());
        assert!(parts(&config(3, 3, &deepest)).is_some());
        assert!(parts(&config(3, 3, &text!("{}/", &*deepest))).is_none());
        let share = |first: usize| {
            use std::fmt::Write as _;
            let mut s = Text::of("Policy: 2 of 3\nFormat: P2WSH\nDerivation: m/0\n");
            for i in first..first + 2 {
                let _ = writeln!(s, "{i:08x}: {KEY}");
            }
            s
        };
        assert!(merge(&[share(0), share(1)]).is_ok());
        assert!(
            merge(&[share(0), share(2)]).is_err(),
            "four keys in a 2 of 3"
        );
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
            text!(
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
            assert!(!readable(&c, &config(&other)), "{}", other.as_str());
        }
    }
}
