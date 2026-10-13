//! The wallet description reader the device uses, shared with the
//! Companion (`docs/COMPANION.md` §4.5): a descriptor, a BIP-388 policy,
//! a threshold or silent payments record, a BIP 129 record, a multisig
//! config, a Coldcard export, a wallet `.json` or Bitcoin Core's
//! `listdescriptors`, each read as the one wallet it describes. A text it
//! does not read is a [`Refusal`], whose line is Faraday's own
//! (`faraday-strings`). Moved here from `faraday-core`'s `wallet` module,
//! which re-exports it.

use faraday_strings::{EN, fill};
use osk_bip::keys::Network;
use osk_bip::policy::WalletPolicy;

/// What a network is called on screen.
pub fn network_name(n: Network) -> &'static str {
    match n {
        Network::Mainnet => EN.wallet_mainnet,
        Network::Testnet => EN.wallet_testnet,
        Network::Signet => EN.wallet_signet,
        Network::Regtest => EN.wallet_regtest,
    }
}

/// Why a key or a wallet was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The words are not a valid BIP-39 mnemonic.
    Words(String),
    /// The key is already in the session.
    Duplicate(String),
    /// A key for a slot was asked for and this is another key.
    WrongKey {
        /// The fingerprint that was expected.
        wanted: String,
        /// The fingerprint the words give.
        got: String,
    },
    /// Words for a wallet's missing key give a key that is none of the
    /// wallet's.
    NotOfWallet {
        /// The fingerprint the words give.
        got: String,
        /// The wallet's name.
        wallet: String,
    },
    /// The text is not a wallet this build reads.
    NotAWallet(String),
    /// The wallet or transaction is on another network than the wallets
    /// already loaded.
    Network {
        /// The network it is on.
        offered: Network,
        /// The network of the loaded wallets.
        loaded: Network,
    },
    /// The session takes only the keys its shell names, and this is
    /// another ([`Session::only`]).
    NotAllowed(String),
}

impl Refusal {
    /// The sentence a screen shows.
    pub fn text(&self) -> String {
        match self {
            Refusal::Words(e) => fill(EN.wallet_not_a_valid, &[&e.to_string()]),
            Refusal::Duplicate(fp) => fill(EN.wallet_is_already_loaded, &[&fp.to_string()]),
            Refusal::WrongKey { wanted, got } => fill(
                EN.wallet_these_words_give,
                &[&got.to_string(), &wanted.to_string()],
            ),
            Refusal::NotOfWallet { got, wallet } => fill(
                EN.wallet_these_words,
                &[&got.to_string(), &wallet.to_string()],
            ),
            Refusal::NotAWallet(e) => fill(EN.wallet_not_a_wallet, &[&e.to_string()]),
            Refusal::Network { offered, loaded } => fill(
                EN.wallet_is_not_the_network,
                &[network_name(*offered), network_name(*loaded)],
            ),
            Refusal::NotAllowed(fp) => fill(EN.wallet_is_not_a_key, &[&fp.to_string()]),
        }
    }
}

/// The network an extended key's version bytes name, as a session takes
/// it: a test key is testnet unless the session is already on a test
/// network.
pub fn network_of_kind(kind: osk_bip::bitcoin::NetworkKind, current: Network) -> Network {
    match kind {
        osk_bip::bitcoin::NetworkKind::Main => Network::Mainnet,
        osk_bip::bitcoin::NetworkKind::Test if !current.is_mainnet() => current,
        osk_bip::bitcoin::NetworkKind::Test => Network::Testnet,
    }
}

/// The network a key expression is on.
pub fn network_of_key(key: &osk_bip::policy::PolicyKey, current: Network) -> Network {
    network_of_kind(key.xpub().network, current)
}

/// The network a wallet's keys are on, when it says.
pub fn policy_network(policy: &WalletPolicy, current: Network) -> Option<Network> {
    if let Some(s) = policy.silent() {
        return Some(s.network);
    }
    policy
        .keys()
        .first()
        .map(|k| network_of_kind(k.xpub().network, current))
}

/// Reads a wallet from text: a descriptor, a BIP-388 policy, or a
/// threshold record. Comment lines (`#`) and blank lines are skipped.
pub fn read_wallet(text: &str) -> Result<WalletPolicy, Refusal> {
    // A threshold record and a silent payments record are read whole:
    // their lines are their fields.
    if osk_bip::threshold::ThresholdRecord::looks_like_record(text)
        || osk_bip::silent_wallet::SilentWallet::looks_like_record(text)
    {
        return WalletPolicy::parse_any(text).map_err(|e| Refusal::NotAWallet(e.to_string()));
    }
    if osk_bip::coldcard::looks_like_export(text) {
        return coldcard_export(text);
    }
    // A wallet .json (Specter Desktop's, which Sparrow also writes and
    // reads) or Bitcoin Core's `listdescriptors`: the descriptor fields.
    if text.trim_start().starts_with('{') {
        let d = json_string(text, "descriptor")
            .or_else(|| json_string(text, "desc"))
            .ok_or_else(|| Refusal::NotAWallet(EN.wallet_a_json_file.into()))?;
        return WalletPolicy::parse_any(&d).or_else(|e| {
            let mut all = json_strings(text, "descriptor");
            all.extend(json_strings(text, "desc"));
            multipath(&all).ok_or_else(|| Refusal::NotAWallet(e.to_string()))
        });
    }
    let body: String = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect::<Vec<_>>()
        .join("");
    // BIP 129's descriptor record: read whole, its first address checked
    // against the wallet it states.
    if osk_bip::bsms::looks_like_record(text) && !osk_bip::bsms::is_signer_record(text) {
        return osk_bip::bsms::DescriptorRecord::parse(text)
            .map(|r| r.policy)
            .map_err(|e| Refusal::NotAWallet(fill(EN.wallet_bsms, &[&format!("{e:?}")])));
    }
    if osk_bip::multisig_config::looks_like_config(text) {
        return osk_bip::multisig_config::parse(text)
            .map_err(|e| Refusal::NotAWallet(format!("{e:?}")));
    }
    WalletPolicy::parse_any(&body).or_else(|e| {
        let lines: Vec<String> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(String::from)
            .collect();
        multipath(&lines).ok_or_else(|| Refusal::NotAWallet(e.to_string()))
    })
}

/// Descriptors other wallets write that are not one multipath
/// descriptor, read as the `<0;1>` wallet they describe:
///
/// - a receive and a change descriptor, the same but for `/0/*` and
///   `/1/*` (Sparrow's older export, Bitcoin Core's `listdescriptors`,
///   which lists one pair per kind: the wallet is the multisig or
///   miniscript one if there is one, else BIP-84's, 86's, 49's, 44's,
///   the order OpenSigner prefers);
/// - a receive descriptor alone, `/0/*` (Jade's and Keystone's export);
/// - one descriptor whose keys carry no derivation (Specter Desktop's
///   wallet .json, and the one Sparrow writes for Specter);
/// - any of these after a label, `Receive: wpkh(…)` (Sparrow).
///
/// Each descriptor must read on its own, checksum and all, before it is
/// rewritten; one that does not refuses the file.
fn multipath(descs: &[String]) -> Option<WalletPolicy> {
    use std::str::FromStr;
    let mut plain = Vec::new();
    for d in descs {
        let d = without_label(d.trim());
        osk_bip::miniscript::Descriptor::<osk_bip::miniscript::DescriptorPublicKey>::from_str(d)
            .ok()?;
        plain.push(d.split('#').next().unwrap_or(d).to_string());
    }
    let mut wallets: Vec<String> = plain
        .iter()
        .filter(|d| d.contains("/0/*") && !d.contains("/1/*"))
        .filter(|d| plain.len() == 1 || plain.contains(&d.replace("/0/*", "/1/*")))
        .map(|d| d.replace("/0/*", "/<0;1>/*"))
        .collect();
    if plain.len() == 1 && wallets.is_empty() {
        wallets.extend(with_chains(&plain[0]));
    }
    let rank = |d: &String| {
        if d.starts_with("wpkh(") {
            1
        } else if d.starts_with("tr(") && !d.contains(',') {
            2
        } else if d.starts_with("sh(wpkh(") {
            3
        } else if d.starts_with("pkh(") {
            4
        } else {
            0
        }
    };
    wallets.sort_by_key(rank);
    wallets.iter().find_map(|d| WalletPolicy::parse_any(d).ok())
}

/// A descriptor line without the label before it: `Receive: wpkh(…)`.
fn without_label(line: &str) -> &str {
    match line.split_once(": ") {
        Some((label, rest)) if label.chars().all(|c| c.is_ascii_alphabetic() || c == ' ') => {
            rest.trim()
        }
        _ => line,
    }
}

/// `d` with `/<0;1>/*` after every extended key, when none of its keys
/// carries a derivation; otherwise `None`.
fn with_chains(d: &str) -> Option<String> {
    let mut out = String::new();
    let mut rest = d;
    let mut found = false;
    while let Some(at) = ["xpub", "tpub"].iter().filter_map(|p| rest.find(p)).min() {
        let len = rest[at..]
            .find(|c: char| !c.is_ascii_alphanumeric())
            .unwrap_or(rest.len() - at);
        let end = at + len;
        if rest[end..].starts_with('/') {
            return None;
        }
        out.push_str(&rest[..end]);
        out.push_str("/<0;1>/*");
        rest = &rest[end..];
        found = true;
    }
    out.push_str(rest);
    found.then_some(out)
}

/// Every string value under `key` in a JSON text.
fn json_strings(text: &str, key: &str) -> Vec<String> {
    let pattern = format!("\"{key}\"");
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(at) = text[from..].find(&pattern) {
        if let Some(v) = json_string(&text[from + at..], key) {
            out.push(v);
        }
        from += at + pattern.len();
    }
    out
}

/// Coldcard's account export, which Nunchuk, Passport and Sparrow also
/// write: one object per single-sig account. The wallet is the first of
/// BIP-84, BIP-86, BIP-49 and BIP-44 the file has, as OpenSigner chooses.
///
/// Each account's key is rooted at the file's top-level `xfp`, the
/// master's (Coldcard's `docs/generic-wallet-export.md`); the `xfp`
/// inside an account is that account key's own fingerprint, and a key
/// origin written with it would match no seed. `osk_bip::coldcard::parse`
/// takes the inner one, so the file is read here. Where the account
/// states its first address, the wallet read must give the same one.
fn coldcard_export(text: &str) -> Result<WalletPolicy, Refusal> {
    let bad = |why: &str| Refusal::NotAWallet(fill(EN.wallet_coldcard_export, &[why]));
    let master = json_string(text, "xfp").ok_or_else(|| bad(EN.wallet_no_xfp))?;
    for (field, wrap) in [
        ("bip84", "wpkh(K)"),
        ("bip86", "tr(K)"),
        ("bip49", "sh(wpkh(K))"),
        ("bip44", "pkh(K)"),
    ] {
        let Some(at) = text.find(&format!("\"{field}\"")) else {
            continue;
        };
        let account = &text[at..at + text[at..].find('}').unwrap_or(text.len() - at)];
        let (Some(deriv), Some(xpub)) =
            (json_string(account, "deriv"), json_string(account, "xpub"))
        else {
            continue;
        };
        let path = deriv.trim_start_matches('m').trim_start_matches('/');
        let key = format!("[{}/{path}]{xpub}/<0;1>/*", master.to_ascii_lowercase());
        let policy = WalletPolicy::parse_any(&wrap.replace('K', &key))
            .map_err(|e| Refusal::NotAWallet(e.to_string()))?;
        if let Some(first) = json_string(account, "first") {
            let found = osk_bip::keys::Network::ALL.iter().any(|&net| {
                policy
                    .address_at(net, false, 0)
                    .is_ok_and(|a| a.to_string() == first)
            });
            if !found {
                return Err(bad(&fill(
                    EN.wallet_its_first_address,
                    &[field, &first.to_string()],
                )));
            }
        }
        return Ok(policy);
    }
    Err(bad(EN.wallet_no_single_sig))
}

/// The first string value under `key` in a JSON text, with its escapes
/// undone. Enough for the wallet files coordinators write; not a JSON
/// parser.
pub fn json_string(text: &str, key: &str) -> Option<String> {
    let at = text.find(&format!("\"{key}\""))? + key.len() + 2;
    let rest = text[at..].trim_start().strip_prefix(':')?.trim_start();
    let mut chars = rest.strip_prefix('"')?.chars();
    let mut out = String::new();
    loop {
        match chars.next()? {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                c => out.push(c),
            },
            c => out.push(c),
        }
    }
}

/// A wallet's first receive address, worked out as the device works it
/// out (`WalletPolicy::address_at`), on the network its keys name, with
/// that network; `None` for a wallet with no address at an index (a
/// silent payments wallet).
pub fn first_address(policy: &WalletPolicy) -> Option<(Network, String)> {
    let net = policy_network(policy, Network::Mainnet).unwrap_or(Network::Mainnet);
    policy
        .address_at(net, false, 0)
        .ok()
        .map(|a| (net, a.to_string()))
}
