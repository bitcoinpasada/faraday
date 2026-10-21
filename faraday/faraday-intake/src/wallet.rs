//! The wallet description reader the device uses: a descriptor, a BIP-388 policy,
//! a threshold or silent payments record, a BIP 129 record, a multisig
//! config, a Coldcard export, a wallet `.json`, Bitcoin Core's
//! `listdescriptors`, a single-key file, a Caravan config or an Electrum
//! wallet file, each read as the one wallet it describes. A text it
//! does not read is a [`Refusal`], whose line is Faraday's own
//! (`faraday-strings`). Moved here from `faraday-core`'s `wallet` module,
//! which re-exports it.

use crate::{caravan, electrum, single_key, slip132};
use faraday_strings::{EN, fill};
use osk_bip::json::{self, Value};
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
    /// The demo is on, and this key is not one of its public seeds.
    NotAllowed(String),
    /// The session's network is fixed, at this one.
    NetworkFixed(Network),
    /// The demo goes to mainnet only with public seeds alone loaded, and
    /// this key is not one.
    NotPublicLoaded(String),
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
            Refusal::NetworkFixed(n) => fill(EN.wallet_runs_on_only, &[network_name(*n)]),
            Refusal::NotPublicLoaded(fp) => fill(EN.wallet_public_seeds_only, &[&fp.to_string()]),
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

/// Reads a wallet from text: a descriptor, a BIP-388 policy, a
/// threshold record, or another wallet's file (module doc). Comment lines (`#`) and blank lines are skipped.
///
/// One wallet has one checksum (`docs/DECISIONS.md` F35): every hardened
/// step is kept written `h`, however the text wrote it, so a wallet read
/// from a multisig config or typed with `'` is the same wallet, with the
/// same descriptor and checksum, as the one made here.
pub fn read_wallet(text: &str) -> Result<WalletPolicy, Refusal> {
    read_wallet_as_written(text).map(hardened_h)
}

/// `policy` with every hardened step of its keys' origins written `h`.
/// In a policy's text `'` is only ever a hardened marker; a record (a
/// threshold wallet's, a silent payments wallet's) is kept as it is.
pub fn hardened_h(policy: WalletPolicy) -> WalletPolicy {
    if policy.record().is_some() || policy.silent().is_some() {
        return policy;
    }
    let text = policy.to_text();
    if !text.contains('\'') {
        return policy;
    }
    WalletPolicy::parse_any(&text.replace('\'', "h")).unwrap_or(policy)
}

/// [`read_wallet`] with the origins as the text wrote them.
fn read_wallet_as_written(text: &str) -> Result<WalletPolicy, Refusal> {
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
    let plain = |d: &str| slip132::plain_descriptor(d).map_err(|m| Refusal::NotAWallet(m.text()));
    if electrum::encrypted_whole(text) {
        return Err(Refusal::NotAWallet(EN.wallet_electrum_encrypted.into()));
    }
    // A single-key file (Cobo, Keystone, Coldcard's for Wasabi), a
    // Caravan config or an Electrum wallet file, read as JSON. JSON too deep or too long is refused;
    // other text that is not quite JSON goes on to the readers below.
    let mut read_json: Option<Value> = None;
    if text
        .trim_start_matches('\u{feff}')
        .trim_start()
        .starts_with('{')
    {
        match json::parse(text) {
            Ok(v) => {
                if let Some(read) = single_key::read(&v) {
                    let k = read.map_err(Refusal::NotAWallet)?;
                    let d = k.descriptor().ok_or_else(|| {
                        Refusal::NotAWallet(fill(
                            EN.wallet_single_key_file,
                            &[EN.wallet_single_key_no_type],
                        ))
                    })?;
                    return WalletPolicy::parse_any(&d)
                        .map_err(|e| Refusal::NotAWallet(e.to_string()));
                }
                if caravan::looks_like(&v) {
                    let d = caravan::descriptor(&v).map_err(Refusal::NotAWallet)?;
                    return WalletPolicy::parse_any(&d)
                        .map_err(|e| Refusal::NotAWallet(e.to_string()));
                }
                if electrum::looks_like(&v) {
                    let d = electrum::descriptor(&v).map_err(Refusal::NotAWallet)?;
                    return WalletPolicy::parse_any(&d)
                        .map_err(|e| Refusal::NotAWallet(e.to_string()));
                }
                read_json = Some(v);
            }
            Err(json::Error::TooDeep) => {
                return Err(Refusal::NotAWallet(fill(
                    EN.wallet_json_too_deep,
                    &[&json::MAX_DEPTH.to_string()],
                )));
            }
            Err(json::Error::TooLarge) => {
                return Err(Refusal::NotAWallet(EN.wallet_json_too_large.into()));
            }
            Err(_) => {}
        }
    }
    // A wallet .json (Specter Desktop's, which Sparrow also writes and
    // reads) or Bitcoin Core's `listdescriptors`: the descriptor fields.
    // Text that starts as JSON and is not JSON is refused here.
    if text.trim_start().starts_with('{') {
        let no_descriptor = || Refusal::NotAWallet(EN.wallet_a_json_file.into());
        let v = read_json.ok_or_else(no_descriptor)?;
        let d = v
            .find("descriptor")
            .and_then(Value::as_str)
            .or_else(|| v.find("desc").and_then(Value::as_str))
            .ok_or_else(no_descriptor)?;
        return WalletPolicy::parse_any(&plain(d)?).or_else(|e| {
            let mut all = v.strings_named("descriptor");
            all.extend(v.strings_named("desc"));
            let all = all
                .iter()
                .map(|d| plain(d.as_str()))
                .collect::<Result<Vec<_>, _>>()?;
            multipath(&all).ok_or_else(|| Refusal::NotAWallet(e.to_string()))
        });
    }
    // BIP 129's descriptor record: read whole, its first address checked
    // against the wallet it states.
    if osk_bip::bsms::looks_like_record(text) && !osk_bip::bsms::is_signer_record(text) {
        return osk_bip::bsms::DescriptorRecord::parse(&bsms_as_upstream_reads(text))
            .map(|r| r.policy)
            .map_err(|e| Refusal::NotAWallet(fill(EN.wallet_bsms, &[&format!("{e:?}")])));
    }
    if osk_bip::multisig_config::looks_like_config(text) {
        let policy = osk_bip::multisig_config::parse(text)
            .map_err(|e| Refusal::NotAWallet(format!("{e:?}")))?;
        return config_keys_agree(text, &policy).map(|()| policy);
    }
    let written: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    // One key alone, `[fingerprint/path]vpub…` (Specter DIY) or the key
    // bare, whose version names a single-key script: that wallet. A key
    // whose version names none (`xpub`, `Zpub`) is no wallet on its own.
    if let [line] = written[..]
        && let Some(read) = slip132::key_line(line)
    {
        let k = read.map_err(|m| Refusal::NotAWallet(m.text()))?;
        let wrap = match k.named {
            Some(slip132::Script::NativeSegwit) => Some("wpkh(K/<0;1>/*)"),
            Some(slip132::Script::NestedSegwit) => Some("sh(wpkh(K/<0;1>/*))"),
            _ => None,
        };
        if let Some(wrap) = wrap {
            return WalletPolicy::parse_any(&wrap.replace('K', &k.text))
                .map_err(|e| Refusal::NotAWallet(e.to_string()));
        }
    }
    let lines = written
        .iter()
        .map(|l| match slip132::key_line(l) {
            Some(read) => read
                .map(|k| k.text)
                .map_err(|m| Refusal::NotAWallet(m.text())),
            None => plain(l),
        })
        .collect::<Result<Vec<String>, _>>()?;
    WalletPolicy::parse_any(&lines.concat())
        .or_else(|e| multipath(&lines).ok_or_else(|| Refusal::NotAWallet(e.to_string())))
}

/// Whether text is another wallet's file that holds a secret beside its
/// public keys: an Electrum wallet file with an `xprv`, a `seed` or
/// imported private keys ([`electrum::holds_secret`]). Costs a JSON parse
/// of a text that starts as JSON and names `wallet_type`.
pub fn holds_secret(text: &str) -> bool {
    let t = text.trim_start_matches('\u{feff}').trim_start();
    t.starts_with('{')
        && t.contains("\"wallet_type\"")
        && json::parse(t).is_ok_and(|v| electrum::looks_like(&v) && electrum::holds_secret(&v))
}

/// A multisig config's SLIP-132 keys checked against the wallet read from
/// it (`osk_bip::multisig_config` takes them without asking): each
/// version's script type must be the wallet's. Costs a base58 decode per
/// line that names such a key.
fn config_keys_agree(text: &str, policy: &WalletPolicy) -> Result<(), Refusal> {
    let around = slip132::Script::of_descriptor(&policy.to_descriptor());
    for line in text.lines() {
        let Some((_, value)) = line.split_once(':') else {
            continue;
        };
        if !slip132::mentions(value) {
            continue;
        }
        if let Some(slip132::Key {
            named: Some(named), ..
        }) = slip132::decode(value)
            && !slip132::agrees(Some(named), around)
        {
            let m = slip132::Mismatch {
                key: value.trim().to_string(),
                named,
                around,
            };
            return Err(Refusal::NotAWallet(m.text()));
        }
    }
    Ok(())
}

/// A BIP 129 descriptor record in the form `osk_bip::bsms` reads, for two
/// that Sparrow writes and accepts and it refuses (`docs/INTEROP.md` §5
/// I3): an address line all in upper case, bech32 or bech32m (`BC1…`,
/// `TB1…`, `BCRT1…`), is lower-cased, as bech32 allows; a template (`/**`)
/// whose paths line is `No path restrictions` reads as `/0/*,/1/*`. A
/// base58 address is never touched: its case is part of it. The first
/// address is checked on the result as on any record.
fn bsms_as_upstream_reads(text: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    if let [_, descriptor, paths, address, ..] = &mut lines[..] {
        if paths.trim() == osk_bip::bsms::NO_PATH_RESTRICTIONS && descriptor.contains("/**") {
            *paths = String::from("/0/*,/1/*");
        }
        let a = address.trim();
        if ["BC1", "TB1", "BCRT1"].iter().any(|p| a.starts_with(p))
            && !a.bytes().any(|b| b.is_ascii_lowercase())
        {
            *address = a.to_ascii_lowercase();
        }
    }
    lines.join("\n")
}

/// Descriptors other wallets write that are not one multipath
/// descriptor, read as the `<0;1>` wallet they describe:
///
/// - a receive and a change descriptor, the same but for `/0/*` and
///   `/1/*` (Sparrow's older export, Bitcoin Core's `listdescriptors`,
///   which lists one pair per kind: the wallet is the multisig or
///   miniscript one if there is one, else BIP-84's, 86's, 49's, 44's,
///   the order OpenSigner prefers);
/// - a receive descriptor alone, `/0/*` (Jade's export), or a change
///   descriptor alone, `/1/*` (Keystone's);
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
    // A change descriptor alone (Keystone's export): the same wallet.
    if let [only] = &plain[..]
        && wallets.is_empty()
        && only.contains("/1/*")
        && !only.contains("/0/*")
    {
        wallets.push(only.replace("/1/*", "/<0;1>/*"));
    }
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
    let file = json::parse(text).map_err(|_| bad(EN.wallet_no_xfp))?;
    let master = file
        .find("xfp")
        .and_then(Value::as_str)
        .ok_or_else(|| bad(EN.wallet_no_xfp))?;
    for (field, wrap) in [
        ("bip84", "wpkh(K)"),
        ("bip86", "tr(K)"),
        ("bip49", "sh(wpkh(K))"),
        ("bip44", "pkh(K)"),
    ] {
        let Some(account) = file.find(field) else {
            continue;
        };
        let (Some(deriv), Some(xpub)) = (
            account.get("deriv").and_then(Value::as_str),
            account.get("xpub").and_then(Value::as_str),
        ) else {
            continue;
        };
        let path = deriv.trim_start_matches('m').trim_start_matches('/');
        let key = format!("[{}/{path}]{xpub}/<0;1>/*", master.to_ascii_lowercase());
        let policy = WalletPolicy::parse_any(&wrap.replace('K', &key))
            .map_err(|e| Refusal::NotAWallet(e.to_string()))?;
        if let Some(first) = account.get("first").and_then(Value::as_str) {
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

/// The name a wallet `.json` carries in its `"label"` (Specter Desktop's,
/// Sparrow's for Specter), when it has one that is not blank: the first
/// member of that name in the file. Costs a JSON parse.
pub fn json_label(text: &str) -> Option<String> {
    if !text.trim_start().starts_with('{') {
        return None;
    }
    let file = json::parse(text).ok()?;
    let label = file.find("label")?.as_str()?;
    (!label.trim().is_empty()).then(|| label.to_string())
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
