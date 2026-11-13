//! The wallet description reader the device uses: a descriptor, a BIP-388 policy,
//! a threshold or silent payments record, a BIP 129 record, a multisig
//! config, a Coldcard export, a wallet `.json`, Bitcoin Core's
//! `listdescriptors`, a single-key file, a Caravan config, an Electrum
//! wallet file or Specter DIY's `addwallet` command, each read as the one wallet it describes. A text it
//! does not read is a [`Refusal`], whose line is Faraday's own
//! (`faraday-strings`). Moved here from `faraday-core`'s `wallet` module,
//! which re-exports it.

pub use crate::names::wallet_name;
use crate::{caravan, electrum, single_key, specter_diy};
use faraday_bip::json::{self, Value};
use faraday_bip::keys::Network;
use faraday_bip::keys::ScriptType;
use faraday_bip::policy::{Template, WalletPolicy};
use faraday_bip::slip132::{self, Mismatch, Script};
use faraday_crypto::work::Compute;
use faraday_strings::{EN, bip};
use faraday_text::{Text, text};

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
    Words(Text),
    /// The key is already in the session: its fingerprint, which is
    /// public.
    Duplicate(faraday_bip::keys::Fingerprint),
    /// A key for a slot was asked for and this is another key.
    WrongKey {
        /// The fingerprint that was expected.
        wanted: Text,
        /// The fingerprint the words give.
        got: Text,
    },
    /// Words for a wallet's missing key give a key that is none of the
    /// wallet's.
    NotOfWallet {
        /// The fingerprint the words give.
        got: Text,
        /// The wallet's name.
        wallet: Text,
    },
    /// The text is not a wallet this build reads.
    NotAWallet(Text),
    /// The wallet or transaction is on another network than the wallets
    /// already loaded.
    Network {
        /// The network it is on.
        offered: Network,
        /// The network of the loaded wallets.
        loaded: Network,
    },
    /// The demo is on, and this key is not one of its public seeds.
    NotAllowed(Text),
    /// A stick is attached, called this ("stick", "SD card"): keys load
    /// only with none (`docs/SECURITY-MODEL.md`).
    StickAttached(&'static str),
    /// A silent payments wallet whose scan key or spend key is not what
    /// the loaded key with its fingerprint (this one) gives at its
    /// account.
    SilentDiffers(Text),
    /// A key of the wallet has a loaded key's master fingerprint (this
    /// one) and is not what that key gives at its origin path.
    NotYourKey(Text),
    /// A key that is not one of the wallet's spends it alone by the key
    /// path, and the person has not confirmed it yet (F132).
    KeyPathUnconfirmed,
    /// A Coldcard export holding a single-sig account and a silent
    /// payments account, and the person has not chosen which loads yet.
    AccountUnchosen,
    /// A MuSig2 wallet two of whose keys come from one seed (one master
    /// fingerprint): the MuSig2 check matches one participant to each
    /// master fingerprint, so a second account of one seed never signs.
    MuSigOneSeed,
}

impl Refusal {
    /// The sentence a screen shows.
    pub fn text(&self) -> Text {
        match self {
            Refusal::Words(e) => Text::fill(EN.wallet_not_a_valid, &[e]),
            Refusal::Duplicate(fp) => Text::fill(
                EN.wallet_is_already_loaded,
                &[&Text::from_utf8_lossy(&fp.to_hex())],
            ),
            Refusal::WrongKey { wanted, got } => {
                Text::fill(EN.wallet_these_words_give, &[got, wanted])
            }
            Refusal::NotOfWallet { got, wallet } => {
                Text::fill(EN.wallet_these_words, &[got, wallet])
            }
            Refusal::NotAWallet(e) => Text::fill(EN.wallet_not_a_wallet, &[e]),
            Refusal::Network { offered, loaded } => Text::fill(
                EN.wallet_is_not_the_network,
                &[network_name(*offered), network_name(*loaded)],
            ),
            Refusal::NotAllowed(fp) => Text::fill(EN.wallet_is_not_a_key, &[fp]),
            Refusal::StickAttached(noun) => Text::fill(EN.stick_keys_keys_load, &[noun]),
            Refusal::SilentDiffers(fp) => Text::fill(EN.wallet_silent_differs, &[fp]),
            Refusal::NotYourKey(fp) => Text::fill(EN.wallet_not_your_key, &[fp]),
            Refusal::KeyPathUnconfirmed => Text::of(EN.wallet_key_path_unconfirmed),
            Refusal::AccountUnchosen => Text::of(EN.wallet_account_unchosen),
            Refusal::MuSigOneSeed => Text::of(EN.wallet_musig_one_seed),
        }
    }
}

/// The network an extended key's version bytes name, as a session takes
/// it: a test key is testnet unless the session is already on a test
/// network.
pub fn network_of_kind(kind: faraday_bip::bitcoin::NetworkKind, current: Network) -> Network {
    match kind {
        faraday_bip::bitcoin::NetworkKind::Main => Network::Mainnet,
        faraday_bip::bitcoin::NetworkKind::Test if !current.is_mainnet() => current,
        faraday_bip::bitcoin::NetworkKind::Test => Network::Testnet,
    }
}

/// The network a key expression is on.
pub fn network_of_key(key: &faraday_bip::policy::PolicyKey, current: Network) -> Network {
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

/// Costs a parse of the description and its keys, and for some forms an
/// address derivation to check the first address they state: reads a
/// wallet from text, a descriptor, a BIP-388 policy, a silent payments record,
/// or another wallet's file (module doc). Comment lines (`#`) and blank
/// lines are skipped.
///
/// One wallet has one checksum (`docs/DECISIONS.md` F35, F66): the
/// policy keeps every hardened step written `h`, however the text wrote
/// it, so a wallet read from a multisig config or typed with `'` is the
/// same wallet, with the same descriptor and checksum, as the one made
/// here.
///
/// A wallet one of whose keys is a bare extended public key, with no
/// origin, is refused: nothing says whose key it is or at what path.
pub fn read_wallet(c: &Compute, text: &str) -> Result<WalletPolicy, Refusal> {
    let policy = read_any(c, text)?;
    if policy.keys().iter().any(|k| k.fingerprint().is_none()) {
        return Err(Refusal::NotAWallet(Text::of(EN.wallet_key_no_origin)));
    }
    Ok(policy)
}

/// Costs what [`read_wallet`] costs: what it reads, before its refusal
/// of a key with no origin.
fn read_any(c: &Compute, text: &str) -> Result<WalletPolicy, Refusal> {
    if let Some(read) = read_whole(c, text) {
        return read;
    }
    let plain =
        |d: &str| slip132::plain_descriptor(d).map_err(|m| Refusal::NotAWallet(mismatch_text(&m)));
    if electrum::encrypted_whole(text) {
        return Err(Refusal::NotAWallet(Text::of(EN.wallet_electrum_encrypted)));
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
                    let k = read.map_err(|e| Refusal::NotAWallet(Text::of(&e)))?;
                    let d = k.descriptor().ok_or_else(|| {
                        Refusal::NotAWallet(Text::fill(
                            EN.wallet_single_key_file,
                            &[EN.wallet_single_key_no_type],
                        ))
                    })?;
                    return WalletPolicy::parse_any(c, &d)
                        .map_err(|e| Refusal::NotAWallet(bip::policy_error(&e)));
                }
                if caravan::looks_like(&v) {
                    let d = caravan::descriptor(&v).map_err(Refusal::NotAWallet)?;
                    return WalletPolicy::parse_any(c, &d)
                        .map_err(|e| Refusal::NotAWallet(bip::policy_error(&e)));
                }
                if electrum::looks_like(&v) {
                    let d =
                        electrum::descriptor(&v).map_err(|e| Refusal::NotAWallet(Text::of(&e)))?;
                    return WalletPolicy::parse_any(c, &d)
                        .map_err(|e| Refusal::NotAWallet(bip::policy_error(&e)));
                }
                read_json = Some(v);
            }
            Err(json::Error::TooDeep) => {
                return Err(Refusal::NotAWallet(Text::fill(
                    EN.wallet_json_too_deep,
                    &[&Text::num(json::MAX_DEPTH as u64)],
                )));
            }
            Err(json::Error::TooLarge) => {
                return Err(Refusal::NotAWallet(Text::of(EN.wallet_json_too_large)));
            }
            Err(_) => {}
        }
    }
    // A wallet .json (Specter Desktop's, which Sparrow also writes and
    // reads) or Bitcoin Core's `listdescriptors`: the descriptor fields.
    // Text that starts as JSON and is not JSON is refused here.
    if text.trim_start().starts_with('{') {
        let no_descriptor = || Refusal::NotAWallet(Text::of(EN.wallet_a_json_file));
        let v = read_json.ok_or_else(no_descriptor)?;
        let mut all = v.strings_named("descriptor");
        all.extend(v.strings_named("desc"));
        let all = all
            .iter()
            .map(|d| plain(d.as_str()))
            .collect::<Result<Vec<_>, _>>()?;
        if all.is_empty() {
            return Err(no_descriptor());
        }
        if all.len() > 1 {
            return one_of_several(c, &all);
        }
        return WalletPolicy::parse_any(c, &all[0]).or_else(|e| {
            multipath(c, &all).unwrap_or_else(|| Err(Refusal::NotAWallet(bip::policy_error(&e))))
        });
    }
    // BIP 129's descriptor record: read whole, its first address checked
    // against the wallet it states.
    if faraday_bip::bsms::looks_like_record(text) && !faraday_bip::bsms::is_signer_record(text) {
        return faraday_bip::bsms::DescriptorRecord::parse(c, text)
            .map(|r| r.policy)
            .map_err(|e| Refusal::NotAWallet(Text::fill(EN.wallet_bsms, &[&bip::bsms_error(&e)])));
    }
    if faraday_bip::multisig_config::looks_like_config(text) {
        // A key whose SLIP-132 version names another script than the
        // file's `Format:` is refused as a key; the refusal names it.
        return faraday_bip::multisig_config::parse(c, text).map_err(|e| {
            match faraday_bip::multisig_config::slip132_mismatch(text) {
                Some(m) if e == faraday_bip::policy::Error::Key => {
                    Refusal::NotAWallet(mismatch_text(&m))
                }
                _ => Refusal::NotAWallet(bip::policy_error(&e)),
            }
        });
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
        let k = read.map_err(|m| Refusal::NotAWallet(mismatch_text(&m)))?;
        let wrap = match k.named {
            Some(Script::NativeSegwit) => Some("wpkh(K/<0;1>/*)"),
            Some(Script::NestedSegwit) => Some("sh(wpkh(K/<0;1>/*))"),
            _ => None,
        };
        if let Some(wrap) = wrap {
            return WalletPolicy::parse_any(c, &Text::replaced(wrap, "K", &k.text))
                .map_err(|e| Refusal::NotAWallet(bip::policy_error(&e)));
        }
    }
    let lines = written
        .iter()
        .map(|l| match slip132::key_line(l) {
            Some(read) => read
                .map(|k| k.text)
                .map_err(|m| Refusal::NotAWallet(mismatch_text(&m))),
            None => plain(l),
        })
        .collect::<Result<Vec<_>, _>>()?;
    WalletPolicy::parse_any(c, &lines.iter().map(|l| l.as_str()).collect::<Text>()).or_else(|e| {
        multipath(c, &lines).unwrap_or_else(|| Err(Refusal::NotAWallet(bip::policy_error(&e))))
    })
}

/// Costs what [`read_wallet`] costs: the forms [`read_wallet`] reads
/// whole before any other, by what the text begins with or holds: a
/// silent payments record, whose lines are its fields; BIP-392's `sp()`
/// descriptor or its key expression alone; Coldcard's account export;
/// Specter DIY's `addwallet` command. `None` for any other text.
fn read_whole(c: &Compute, text: &str) -> Option<Result<WalletPolicy, Refusal>> {
    if let Some(line) = sp_line(text) {
        return Some(
            faraday_bip::silent_wallet::parse_descriptor(c, line)
                .map(WalletPolicy::of_silent)
                .map_err(|e| silent_refusal(&e)),
        );
    }
    if faraday_bip::silent_wallet::SilentWallet::looks_like_record(text) {
        return Some(
            WalletPolicy::parse_any(c, text)
                .map_err(|e| Refusal::NotAWallet(bip::policy_error(&e))),
        );
    }
    if faraday_bip::coldcard::looks_like_export(text) {
        return Some(coldcard_export(c, text));
    }
    specter_diy::split(text).map(|(_, d)| specter_diy_wallet(c, d))
}

/// The one line of `text` that is BIP-392's `sp(…)` descriptor
/// (Keystone's and Sparrow's) or its key expression alone,
/// `[origin]spscan1q…` (Specter DIY's), when the text is that line
/// alone, beside blank and comment lines.
fn sp_line(text: &str) -> Option<&str> {
    let mut written = text
        .trim_start_matches('\u{feff}')
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'));
    let line = written.next()?;
    if written.next().is_some() {
        return None;
    }
    // Compared without a copy: the line carries the scan private key.
    let starts = |t: &str, p: &str| t.get(..p.len()).is_some_and(|h| h.eq_ignore_ascii_case(p));
    let key = line.rsplit(']').next().unwrap_or(line);
    let sp = starts(line, "sp("); // not wording: a descriptor's function name
    (sp || starts(key, "spscan1") || starts(key, "tspscan1")).then_some(line)
}

/// The sentence a refused `sp()` descriptor shows.
fn silent_refusal(e: &faraday_bip::silent_wallet::ReadError) -> Refusal {
    use faraday_bip::silent_wallet::ReadError as R;
    let why = match e {
        R::NoOrigin => return Refusal::NotAWallet(Text::of(EN.wallet_sp_no_origin)),
        R::Origin => EN.wallet_sp_origin,
        R::Key(_) => EN.wallet_sp_key,
        R::Checksum => EN.wallet_sp_checksum,
        R::Shape => EN.wallet_sp_shape,
    };
    Refusal::NotAWallet(Text::fill(EN.wallet_sp, &[why]))
}

/// Costs a parse of the descriptor and of each wallet tried: the
/// descriptor of Specter DIY's `addwallet {name}&{descriptor}`
/// ([`specter_diy::split`]), read as any descriptor is, its checksum
/// checked where it has one, and keys none of which carries a
/// derivation read as the two chains ([`multipath`]). A command with no
/// descriptor after its `&` is refused.
fn specter_diy_wallet(c: &Compute, d: &str) -> Result<WalletPolicy, Refusal> {
    if !d.contains('(') {
        return Err(Refusal::NotAWallet(Text::of(
            EN.wallet_specter_diy_no_descriptor,
        )));
    }
    let d = slip132::plain_descriptor(d).map_err(|m| Refusal::NotAWallet(mismatch_text(&m)))?;
    WalletPolicy::parse_any(c, &d).or_else(|e| {
        multipath(c, core::slice::from_ref(&d))
            .unwrap_or_else(|| Err(Refusal::NotAWallet(bip::policy_error(&e))))
    })
}

/// The name a wallet's text carries, when it carries one that is not
/// blank: an `addwallet` command's ([`specter_diy::name`]) or a wallet
/// `.json`'s `"label"` ([`json_label`]). Costs a JSON parse of text that
/// starts as JSON.
pub fn carried_name(text: &str) -> Option<Text> {
    specter_diy::name(text)
        .map(|n| Text::of(&n))
        .or_else(|| json_label(text))
}

/// Whether text is another wallet's file that holds a secret beside its
/// public keys: an Electrum wallet file with an `xprv`, a `seed` or
/// imported private keys ([`electrum::holds_secret`]), or any text with
/// BIP-392's `spspend1q…` in it, the silent payments spend private key.
/// BIP-392's `spscan1q…` (an `sp()` descriptor, Specter DIY's key alone,
/// Coldcard's `bip352` account) is no such secret: it lets its holder see
/// the payments the wallet receives and spend none (owner, 2026-10-29),
/// so its file is a wallet file. Costs a JSON parse of a text that starts
/// as JSON and names `wallet_type`.
pub fn holds_secret(text: &str) -> bool {
    let t = text.trim_start_matches('\u{feff}').trim_start();
    if crate::holds_spend_key(t) {
        return true;
    }
    t.starts_with('{') && t.contains("\"wallet_type\"") && electrum::text_holds_secret(t)
}

/// Costs a parse of each descriptor and of each wallet tried: descriptors
/// other wallets write that are not one multipath descriptor, read as
/// the `<0;1>` wallet they describe:
///
/// - a receive and a change descriptor, the same but for `/0/*` and
///   `/1/*` (Sparrow's older export, Bitcoin Core's `listdescriptors`,
///   which lists one pair per kind: the wallet is the multisig,
///   miniscript, script-tree or MuSig2 one if there is one, else
///   BIP-84's, 86's, 49's, 44's, in that order);
/// - a receive descriptor alone, `/0/*` (Jade's export), or a change
///   descriptor alone, `/1/*` (Keystone's);
/// - one descriptor whose keys carry no derivation (Specter Desktop's
///   wallet .json, and the one Sparrow writes for Specter);
/// - any of these after a label, `Receive: wpkh(…)` (Sparrow).
///
/// Each descriptor must read on its own, checksum and all, before it is
/// rewritten; one that does not refuses the file, and so do two different
/// wallets of the best kind. A silent payments wallet has no place in that
/// order (neither Bitcoin Core's `listdescriptors` nor Sparrow writes one
/// beside another wallet), so a file that holds one beside another wallet is
/// refused as holding more than one wallet.
fn multipath<S: AsRef<str>>(c: &Compute, descs: &[S]) -> Option<Result<WalletPolicy, Refusal>> {
    let wallets = multipath_wallets(c, descs)?;
    let mut found = Vec::new();
    let mut seen: Vec<Seen> = Vec::new();
    for d in wallets {
        if let Ok(w) = WalletPolicy::parse_any(c, &d) {
            let this = Seen(faraday_crypto::Secret::new(w.to_descriptor().into_bytes()));
            if !seen.contains(&this) {
                seen.push(this);
                found.push((rank(&w), w));
            }
        }
    }
    pick_best(found)
}

/// A descriptor kept only to tell a wallet already seen: it may hold a
/// silent payments scan key, so it is wiped when dropped and compared
/// without a branch on its bytes.
struct Seen(faraday_crypto::Secret<Vec<u8>>);

impl PartialEq for Seen {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

/// The wallet of the best kind among `found`, or the refusal when that
/// kind holds more than one different wallet; `None` when there is none.
fn pick_best(found: Vec<(u8, WalletPolicy)>) -> Option<Result<WalletPolicy, Refusal>> {
    let best = found.iter().map(|(r, ..)| *r).min()?;
    if found.len() > 1 && found.iter().any(|(r, ..)| *r == UNRANKED) {
        return Some(Err(Refusal::NotAWallet(Text::of(EN.wallet_several))));
    }
    let mut top = found.into_iter().filter(|(r, ..)| *r == best);
    Some(match (top.next(), top.next()) {
        (Some((_, w)), None) => Ok(w),
        _ => Err(Refusal::NotAWallet(Text::of(EN.wallet_several))),
    })
}

/// Costs a parse of each descriptor and a sorted set of them: the `<0;1>`
/// wallets [`multipath`] reads out of descriptors, none of them read yet; `None` when a
/// descriptor does not read on its own.
fn multipath_wallets<S: AsRef<str>>(c: &Compute, descs: &[S]) -> Option<Vec<Text>> {
    let mut plain = Vec::new();
    for d in descs {
        let d = without_label(d.as_ref().trim());
        if !faraday_bip::descriptor::reads_as_descriptor(c, d) {
            return None;
        }
        plain.push(Text::of(d.split('#').next().unwrap_or(d)));
    }
    Some(chain_wallets(&plain))
}

/// The `<0;1>` wallets the receive and change descriptors in `plain` (all
/// the file's, checksums dropped) make, and the lone ones that stand for a
/// wallet.
fn chain_wallets(plain: &[Text]) -> Vec<Text> {
    let have: std::collections::BTreeSet<&str> = plain.iter().map(|t| t.as_str()).collect();
    let mut wallets: Vec<Text> = plain
        .iter()
        .filter(|d| d.contains("/0/*") && !d.contains("/1/*"))
        .filter(|d| plain.len() == 1 || have.contains(replaced(d, "/0/*", "/1/*").as_str()))
        .map(|d| replaced(d, "/0/*", "/<0;1>/*"))
        .collect();
    // A change descriptor alone (Keystone's export): the same wallet.
    if let [only] = plain
        && wallets.is_empty()
        && only.contains("/1/*")
        && !only.contains("/0/*")
    {
        wallets.push(replaced(only, "/1/*", "/<0;1>/*"));
    }
    if plain.len() == 1 && wallets.is_empty() {
        wallets.extend(with_chains(&plain[0]));
    }
    wallets
}

/// `d` with every `from` written `to`, as a [`Text`]: a descriptor may
/// hold a scan key, so no copy of it is left unwiped.
fn replaced(d: &str, from: &str, to: &str) -> Text {
    let mut out = Text::with_room(d.len() + to.len());
    let mut rest = d;
    while let Some(at) = rest.find(from) {
        out.push_str(&rest[..at]);
        out.push_str(to);
        rest = &rest[at + from.len()..];
    }
    out.push_str(rest);
    out
}

/// The place of a wallet that has none in the order [`rank`] gives: a
/// silent payments wallet, which a file of several wallets cannot hold.
const UNRANKED: u8 = u8::MAX;

/// Where a wallet stands in the order [`multipath`] documents, by its kind
/// as read: 0 the multisig, miniscript, script-tree or MuSig2 wallet, 1
/// BIP-84's, 2 BIP-86's, 3 BIP-49's, 4 BIP-44's, and [`UNRANKED`] for a
/// silent payments wallet.
fn rank(w: &WalletPolicy) -> u8 {
    match w.template() {
        Template::Single { script } => match script {
            ScriptType::NativeSegwit => 1,
            ScriptType::Taproot => 2,
            ScriptType::NestedSegwit => 3,
            ScriptType::Legacy => 4,
        },
        Template::Silent => UNRANKED,
        Template::Multi { .. } | Template::MuSig | Template::Miniscript { .. } | Template::Tree => {
            0
        }
    }
}

/// Costs a parse of each descriptor and of each wallet tried, and a
/// sorted list of the wallets found (linear in the descriptors, bounded by
/// the JSON size cap): the wallet a file with several descriptors holds,
/// by the order [`multipath`] documents whatever order the file writes
/// them in. Each descriptor that reads whole, and each receive and change
/// pair, is a wallet; a descriptor that does not read on its own refuses
/// the file, and so does a best-ranked kind holding more than one
/// different wallet. A lone `/0/*` or `/1/*` among several is not a wallet,
/// and a silent payments wallet beside any other refuses the file.
fn one_of_several<S: AsRef<str>>(c: &Compute, all: &[S]) -> Result<WalletPolicy, Refusal> {
    let mut found: Vec<(u8, WalletPolicy)> = Vec::new();
    let mut seen: Vec<Seen> = Vec::new();
    let mut add = |w: WalletPolicy| {
        let this = Seen(faraday_crypto::Secret::new(w.to_descriptor().into_bytes()));
        if !seen.contains(&this) {
            seen.push(this);
            found.push((rank(&w), w));
        }
    };
    let mut plain = Vec::new();
    for d in all {
        let t = without_label(d.as_ref().trim());
        match WalletPolicy::parse_any(c, d.as_ref()) {
            Ok(w) => add(w),
            Err(e) if !faraday_bip::descriptor::reads_as_descriptor(c, t) => {
                return Err(Refusal::NotAWallet(bip::policy_error(&e)));
            }
            Err(_) => {}
        }
        plain.push(Text::of(t.split('#').next().unwrap_or(t)));
    }
    for d in chain_wallets(&plain) {
        if let Ok(w) = WalletPolicy::parse_any(c, &d) {
            add(w);
        }
    }
    pick_best(found).unwrap_or_else(|| Err(Refusal::NotAWallet(Text::of(EN.wallet_a_json_file))))
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
fn with_chains(d: &str) -> Option<Text> {
    let mut out = Text::with_room(d.len() + 16);
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

/// Costs what `faraday_bip::coldcard::read` costs: Coldcard's account
/// export, which Nunchuk, Passport and Sparrow also write, read by `faraday_bip::coldcard::read` (rooted at the top-level
/// `xfp`, the stated first address checked), its refusals in Faraday's
/// own sentences.
fn coldcard_export(c: &Compute, text: &str) -> Result<WalletPolicy, Refusal> {
    use faraday_bip::coldcard::ReadError;
    let bad = |why: &str| Refusal::NotAWallet(Text::fill(EN.wallet_coldcard_export, &[why]));
    faraday_bip::coldcard::read(c, text)
        .map(|(policy, _)| policy)
        .map_err(|e| match e {
            ReadError::NoXfp => bad(EN.wallet_no_xfp),
            ReadError::NoAccount => bad(EN.wallet_no_single_sig),
            ReadError::FirstAddress { account, first } => {
                bad(&Text::fill(EN.wallet_its_first_address, &[account, &first]))
            }
            ReadError::NoDeriv(_) => {
                Refusal::NotAWallet(bip::policy_error(&faraday_bip::policy::Error::Key))
            }
            ReadError::Policy(e) => Refusal::NotAWallet(bip::policy_error(&e)),
            ReadError::Silent(e) => silent_refusal(&e),
        })
}

/// What a script type is called in a refusal.
pub fn script_name(s: Script) -> &'static str {
    match s {
        Script::NativeSegwit => EN.wallet_native_segwit,
        Script::NestedSegwit => EN.wallet_nested_segwit,
        Script::NativeMultisig => EN.wallet_slip132_native_multisig,
        Script::NestedMultisig => EN.wallet_slip132_nested_multisig,
        Script::Taproot => EN.wallet_taproot,
        Script::Legacy => EN.wallet_legacy,
        Script::LegacyMultisig => EN.wallet_slip132_legacy_multisig,
        Script::SilentPayments => EN.wallet_slip132_silent_payments,
        Script::Other => EN.wallet_slip132_other,
    }
}

/// The sentence a SLIP-132 refusal shows (`faraday_bip::slip132`,
/// `docs/INTEROP.md` §5, SLIP-132 keys): the key, shortened, and both types.
pub fn mismatch_text(m: &Mismatch) -> Text {
    let k: &str = &m.key;
    let short = if k.len() > 16 && k.is_ascii() {
        text!("{}…{}", &k[..8], &k[k.len() - 6..])
    } else {
        Text::of(k)
    };
    Text::fill(
        EN.wallet_slip132_mismatch,
        &[&short, script_name(m.named), script_name(m.around)],
    )
}

/// The name a wallet `.json` carries in its `"label"` (Specter Desktop's,
/// Sparrow's for Specter), when it has one that is not blank: the first
/// member of that name in the file. Costs a JSON parse.
pub fn json_label(text: &str) -> Option<Text> {
    if !text.trim_start().starts_with('{') {
        return None;
    }
    let file = json::parse(text).ok()?;
    let label = wallet_name(file.find("label")?.as_str()?);
    (!label.is_empty()).then(|| Text::of(&label))
}

/// Costs an address derivation: a wallet's first receive address,
/// worked out as the device works it out (`WalletPolicy::address_at`), on the network its keys name, with
/// that network; `None` for a wallet with no address at an index (a
/// silent payments wallet).
pub fn first_address(c: &Compute, policy: &WalletPolicy) -> Option<(Network, Text)> {
    let net = policy_network(policy, Network::Mainnet).unwrap_or(Network::Mainnet);
    policy
        .address_at(c, net, false, 0)
        .ok()
        .map(|a| (net, text!("{a}")))
}
