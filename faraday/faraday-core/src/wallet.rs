//! The Wallets tab's session: keys and wallets held in memory, and the spend of
//! one transaction from loading it to the two results (`docs/WALLETS.md`
//! §2 and §4). Every piece of cryptography is `osk-bip`'s or
//! `osk-psbt`'s; this module only decides which call comes next.

use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::bitcoin::consensus::encode::serialize_hex;
use osk_bip::bitcoin::{Transaction, Txid};
use osk_bip::keys::{Fingerprint, MasterKey, Network, ScriptType};
use osk_bip::policy::{Template, WalletPolicy, Wrapper};
use osk_psbt::verify::Verdict;
use osk_psbt::{Aux, Context, Inspection, KeyRef, Nonce, Psbt};

/// The network a session starts on, before a wallet or a PSBT says
/// otherwise.
pub const START_NETWORK: Network = Network::Mainnet;

/// What a network is called on screen.
pub fn network_name(n: Network) -> &'static str {
    match n {
        Network::Mainnet => "Mainnet",
        Network::Testnet => "Testnet",
        Network::Signet => "Signet",
        Network::Regtest => "Regtest",
    }
}

/// A key that can sign, held for this session only.
pub struct Key {
    /// The master key.
    pub master: MasterKey,
    /// What the person called it.
    pub label: String,
    /// The words it was loaded from, for the backup's copy-by-hand step.
    /// Wiped with the session.
    pub words: Option<zeroize::Zeroizing<String>>,
    /// The BIP-39 list those words are from.
    pub language: Language,
    /// The public share these words are as a FROST share: 24 words'
    /// 32 bytes read as the share's scalar (`docs/PLANNING.md` §16.103).
    /// Every 24-word key has one; it signs as a share only for a loaded
    /// threshold record that lists it.
    pub share: Option<osk_bip::bitcoin::secp256k1::PublicKey>,
    /// The BIP-39 passphrase it was loaded with, when it has one. Wiped
    /// with the session.
    pub passphrase: Option<zeroize::Zeroizing<String>>,
}

impl Key {
    /// Runs `take` on the secret share these words are, rebuilt for the
    /// call and wiped after it.
    pub fn with_secret_share(&self, take: &mut dyn FnMut(&osk_bip::frost::SecShare)) {
        if let Some(share) = self
            .words
            .as_ref()
            .and_then(|w| Mnemonic::parse(self.language, w).ok())
            .and_then(|m| secret_share(&m))
        {
            take(&share);
        }
    }
}

/// The FROST share 24 words are, when they are 24 words.
fn secret_share(m: &Mnemonic) -> Option<osk_bip::frost::SecShare> {
    if m.word_count() != 24 {
        return None;
    }
    let entropy = m.entropy();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(entropy.expose().as_bytes());
    let share = osk_bip::frost::SecShare::from_bytes(&bytes).ok();
    zeroize::Zeroize::zeroize(&mut bytes);
    share
}

/// What one signing pass did.
pub struct Signed {
    /// The fingerprints that signed: keys, and shares by their own
    /// fingerprints.
    pub by: Vec<Fingerprint>,
    /// The carry section a threshold pass leaves for the shares still to
    /// sign: the secret nonce that travels on the stick.
    pub carry: Option<osk_psbt::threshold::CarrySection>,
}

/// A wallet described by its public keys.
pub struct Wallet {
    /// What the person called it.
    pub name: String,
    /// The policy, as it was read.
    pub policy: WalletPolicy,
    /// Where it came from: "Test wallet" or the file's name.
    pub source: String,
}

/// What the nonce check found ([`Session::nonce_check`]).
pub struct NonceCheck {
    /// Every signature, in input order.
    pub rows: Vec<NonceRow>,
    /// Nonces used twice under one key: the key, given away.
    pub reused: Vec<osk_psbt::verify::NonceReuse>,
}

impl NonceCheck {
    /// Whether nothing is wrong: every signature checked is valid, every
    /// one made here was recomputed, and no nonce repeats.
    pub fn sound(&self) -> bool {
        self.reused.is_empty()
            && self
                .rows
                .iter()
                .all(|r| !r.verdict.is_invalid() && !matches!(r.recomputed, Some(None)))
    }
}

/// One signature, as the nonce check found it.
pub struct NonceRow {
    /// The input it signs.
    pub input: usize,
    /// Its key: the master fingerprint, or the key's first characters.
    pub name: String,
    /// Whether it is the key's over this transaction.
    pub verdict: Verdict,
    /// For a key held here: the nonce rule that recomputed it, or `None`
    /// inside when none did. `None` outside for a key held elsewhere,
    /// whose nonce cannot be recomputed without its private key.
    pub recomputed: Option<Option<osk_psbt::verify::NonceMode>>,
}

/// One key slot of a wallet, in the descriptor's order.
pub struct Slot {
    /// The key's master fingerprint, when the descriptor gives one.
    pub fingerprint: Option<Fingerprint>,
    /// The label of the session key that fills it, when one does.
    pub held_by: Option<String>,
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
}

impl Refusal {
    /// The sentence a screen shows.
    pub fn text(&self) -> String {
        match self {
            Refusal::Words(e) => format!("Not a valid seed: {e}"),
            Refusal::Duplicate(fp) => format!("{fp} is already loaded"),
            Refusal::WrongKey { wanted, got } => {
                format!("These words give {got}, not {wanted}")
            }
            Refusal::NotAWallet(e) => format!("Not a wallet: {e}"),
            Refusal::Network { offered, loaded } => format!(
                "{} is not {}, the network of the loaded wallets",
                network_name(*offered),
                network_name(*loaded)
            ),
        }
    }
}

/// The fingerprint as eight hex digits.
pub fn fp_text(fp: Fingerprint) -> String {
    String::from_utf8_lossy(&fp.to_hex()).into_owned()
}

/// Keys and wallets for this session. Nothing here is written anywhere.
pub struct Session {
    /// Keys that can sign.
    pub keys: Vec<Key>,
    /// Wallets.
    pub wallets: Vec<Wallet>,
    /// The network every key derives for and every address is shown on.
    network: Network,
}

impl Default for Session {
    fn default() -> Self {
        Session {
            keys: Vec::new(),
            wallets: Vec::new(),
            network: START_NETWORK,
        }
    }
}

/// The network an extended key's version bytes name, as a session takes
/// it: a test key is testnet unless the session is already on a test
/// network.
fn network_of_kind(kind: osk_bip::bitcoin::NetworkKind, current: Network) -> Network {
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

/// The network a PSBT is on, when it says: its global xpubs, or the coin
/// type of a BIP-44, 49, 84, 86 or 48 key path. A PSBT that says neither
/// is taken on the session's network.
pub fn psbt_network(psbt: &Psbt, current: Network) -> Option<Network> {
    use osk_bip::bitcoin::bip32::ChildNumber;
    let inner = psbt.inner();
    if let Some(x) = inner.xpub.keys().next() {
        return Some(network_of_kind(x.network, current));
    }
    let paths = inner
        .inputs
        .iter()
        .flat_map(|i| {
            i.bip32_derivation
                .values()
                .map(|(_, p)| p)
                .chain(i.tap_key_origins.values().map(|(_, (_, p))| p))
        })
        .chain(inner.outputs.iter().flat_map(|o| {
            o.bip32_derivation
                .values()
                .map(|(_, p)| p)
                .chain(o.tap_key_origins.values().map(|(_, (_, p))| p))
        }));
    for path in paths {
        let v: Vec<ChildNumber> = path.into_iter().copied().collect();
        if let [
            ChildNumber::Hardened {
                index: 44 | 49 | 84 | 86 | 48,
            },
            ChildNumber::Hardened { index: c },
            ..,
        ] = v[..]
        {
            return Some(match c {
                0 => Network::Mainnet,
                _ => network_of_kind(osk_bip::bitcoin::NetworkKind::Test, current),
            });
        }
    }
    None
}

impl Session {
    /// A session on `network`.
    pub fn on(network: Network) -> Session {
        Session {
            network,
            ..Session::default()
        }
    }

    /// The network every key derives for.
    pub fn network(&self) -> Network {
        self.network
    }

    /// Moves the session to `network`. Keys are re-derived for it. While
    /// wallets are loaded it moves only between networks whose keys are
    /// written alike (the test networks), never between mainnet and a test
    /// network.
    pub fn set_network(&mut self, network: Network) -> Result<(), Refusal> {
        if network == self.network {
            return Ok(());
        }
        if !self.wallets.is_empty() && network.kind() != self.network.kind() {
            return Err(Refusal::Network {
                offered: network,
                loaded: self.network,
            });
        }
        for k in &mut self.keys {
            k.master = k.master.for_network(network);
        }
        self.network = network;
        Ok(())
    }

    /// Takes the network a wallet or a PSBT is on: the session moves to it
    /// when no wallet holds it elsewhere, and refuses it otherwise.
    pub fn follow(&mut self, network: Option<Network>) -> Result<(), Refusal> {
        let Some(n) = network else { return Ok(()) };
        if n.kind() == self.network.kind() {
            return Ok(());
        }
        self.set_network(n)
    }

    /// Takes the network of a PSBT that is about to be signed.
    pub fn follow_psbt(&mut self, psbt: &Psbt) -> Result<(), Refusal> {
        self.follow(psbt_network(psbt, self.network))
    }

    /// Whether any secret is held: the stick rule's test
    /// (`PLAN.md` §5.1).
    pub fn holds_secret(&self) -> bool {
        !self.keys.is_empty()
    }

    /// Adds the key the words give. `wanted` is the slot's fingerprint
    /// when the key is meant for one.
    pub fn add_words(
        &mut self,
        words: &str,
        label: &str,
        wanted: Option<Fingerprint>,
    ) -> Result<Fingerprint, Refusal> {
        self.add_words_with(words, "", label, wanted)
    }

    /// Adds the key the words and a BIP-39 passphrase give. The same words
    /// with another passphrase are another key. English is read first, then
    /// the other lists in BIP-39's order.
    pub fn add_words_with(
        &mut self,
        words: &str,
        passphrase: &str,
        label: &str,
        wanted: Option<Fingerprint>,
    ) -> Result<Fingerprint, Refusal> {
        let normal =
            zeroize::Zeroizing::new(words.split_whitespace().collect::<Vec<_>>().join(" "));
        let mut first = None;
        for lang in Language::ALL {
            match Mnemonic::parse(lang, &normal) {
                Ok(m) => return self.add_mnemonic(&m, passphrase, label, wanted),
                Err(e) => {
                    first.get_or_insert(e);
                }
            }
        }
        Err(Refusal::Words(
            first.map(|e| e.to_string()).unwrap_or_default(),
        ))
    }

    /// Adds the key a mnemonic and a BIP-39 passphrase give, keeping its
    /// words in its own list.
    pub fn add_mnemonic(
        &mut self,
        mnemonic: &Mnemonic,
        passphrase: &str,
        label: &str,
        wanted: Option<Fingerprint>,
    ) -> Result<Fingerprint, Refusal> {
        let lang = mnemonic.language();
        let seed = mnemonic
            .to_seed(passphrase.as_bytes())
            .map_err(|e| Refusal::Words(e.to_string()))?;
        let master = MasterKey::from_seed(&seed, self.network);
        let fp = master.fingerprint();
        if let Some(w) = wanted
            && w != fp
        {
            return Err(Refusal::WrongKey {
                wanted: fp_text(w),
                got: fp_text(fp),
            });
        }
        if self.keys.iter().any(|k| k.master.fingerprint() == fp) {
            return Err(Refusal::Duplicate(fp_text(fp)));
        }
        let label = if label.trim().is_empty() {
            fp_text(fp)
        } else {
            label.trim().to_string()
        };
        let mut words = crate::secret_text::room();
        for (k, &i) in mnemonic.indices().iter().enumerate() {
            if k > 0 {
                words.push(' ');
            }
            words.push_str(lang.word(i));
        }
        let secp = osk_bip::bitcoin::secp256k1::Secp256k1::new();
        let share = secret_share(mnemonic).map(|s| s.public_share(&secp));
        self.keys.push(Key {
            master,
            label,
            words: Some(words),
            language: lang,
            share,
            passphrase: (!passphrase.is_empty())
                .then(|| zeroize::Zeroizing::new(passphrase.to_string())),
        });
        Ok(fp)
    }

    /// Adds the key a master seed gives, as an `osk-backup` kind-2 backup
    /// carries one: there are no words to copy by hand.
    pub fn add_seed(&mut self, seed: &[u8], label: &str) -> Result<Fingerprint, Refusal> {
        let bytes = osk_crypto::SeedBytes::new(seed)
            .ok_or_else(|| Refusal::Words("a master seed is 16 to 64 bytes".to_string()))?;
        let master = MasterKey::from_seed_bytes(&osk_crypto::Secret::new(bytes), self.network);
        let fp = master.fingerprint();
        if self.keys.iter().any(|k| k.master.fingerprint() == fp) {
            return Err(Refusal::Duplicate(fp_text(fp)));
        }
        self.keys.push(Key {
            master,
            label: if label.trim().is_empty() {
                fp_text(fp)
            } else {
                label.trim().to_string()
            },
            words: None,
            language: Language::English,
            share: None,
            passphrase: None,
        });
        Ok(fp)
    }

    /// Adds the wallet the text describes, under `name`, unless the same
    /// descriptor is already loaded.
    pub fn add_wallet(&mut self, name: &str, text: &str, source: &str) -> Result<usize, Refusal> {
        let policy = read_wallet(text)?;
        let descriptor = same_wallet(&policy);
        if let Some(i) = self
            .wallets
            .iter()
            .position(|w| same_wallet(&w.policy) == descriptor)
        {
            return Ok(i);
        }
        self.follow(policy_network(&policy, self.network))?;
        self.wallets.push(Wallet {
            name: name.to_string(),
            policy,
            source: source.to_string(),
        });
        Ok(self.wallets.len() - 1)
    }

    /// Drops every key and wallet.
    pub fn wipe(&mut self) {
        self.keys.clear();
        self.wallets.clear();
    }

    /// The label of the session key with `fp`.
    pub fn key_label(&self, fp: Fingerprint) -> Option<&str> {
        self.keys
            .iter()
            .find(|k| k.master.fingerprint() == fp)
            .map(|k| k.label.as_str())
    }

    /// The wallet's slots, in the descriptor's order.
    pub fn slots(&self, wallet: &Wallet) -> Vec<Slot> {
        // A silent payments wallet's one slot is the key its two halves
        // were derived from.
        if let Some(r) = wallet.policy.silent() {
            return vec![Slot {
                fingerprint: Some(r.fingerprint),
                held_by: self.key_label(r.fingerprint).map(str::to_string),
            }];
        }
        // A threshold wallet's slots are its shares, each named by its
        // own fingerprint.
        if let Some(record) = wallet.policy.record() {
            let here = self.shares_here(wallet);
            return (0..record.n())
                .map(|i| Slot {
                    fingerprint: record.share_fingerprint(i),
                    held_by: here
                        .iter()
                        .find(|(id, _)| *id as usize == i)
                        .map(|(_, k)| k.label.clone()),
                })
                .collect();
        }
        wallet
            .policy
            .keys()
            .iter()
            .map(|k| {
                let fingerprint = k.fingerprint();
                Slot {
                    fingerprint,
                    held_by: fingerprint
                        .and_then(|fp| self.key_label(fp))
                        .map(str::to_string),
                }
            })
            .collect()
    }

    /// The quorum: (signatures needed, keys).
    pub fn quorum(wallet: &Wallet) -> (usize, usize) {
        if let Some(r) = wallet.policy.record() {
            return (r.t(), r.n());
        }
        wallet
            .policy
            .quorum()
            .or_else(|| wallet.policy.tapscript_quorum())
            .unwrap_or((1, wallet.policy.keys().len().max(1)))
    }

    /// The wallet's shape in a few words.
    pub fn shape(wallet: &Wallet) -> String {
        let script = |t: ScriptType| match t {
            ScriptType::Legacy => "legacy",
            ScriptType::NestedSegwit => "nested SegWit",
            ScriptType::NativeSegwit => "native SegWit",
            ScriptType::Taproot => "Taproot",
        };
        let (m, n) = Self::quorum(wallet);
        match Kind::of(&wallet.policy) {
            Kind::Single(t) => format!("Single key · {}", script(t)),
            Kind::Multi(Wrapper::Sh) => format!("{m} of {n} · legacy multisig"),
            Kind::Multi(Wrapper::ShWsh) => format!("{m} of {n} · nested SegWit multisig"),
            Kind::Multi(_) => format!("{m} of {n} · native SegWit multisig"),
            Kind::TapMulti => format!("{m} of {n} · Taproot multisig"),
            Kind::Miniscript => format!(
                "{} keys · miniscript with conditions",
                wallet.policy.keys().len()
            ),
            Kind::Tree => format!(
                "{} keys · Taproot key and leaves",
                wallet.policy.keys().len()
            ),
            Kind::MuSig => format!("{n} of {n} · MuSig2"),
            Kind::Threshold => format!("{m} of {n} · FROST"),
            Kind::Silent => "1 key · silent payments".to_string(),
            Kind::Other => "Wallet".to_string(),
        }
    }

    /// An address of the wallet, on the session's network.
    pub fn address(&self, wallet: &Wallet, change: bool, index: u32) -> String {
        wallet
            .policy
            .address_at(self.network, change, index)
            .map(|a| a.to_string())
            .unwrap_or_else(|e| format!("no address: {e}"))
    }

    fn key_refs(&self) -> Vec<KeyRef> {
        self.keys
            .iter()
            .filter_map(|k| KeyRef::from_master(&k.master, 0).ok())
            .collect()
    }

    fn policies(&self) -> Vec<WalletPolicy> {
        self.wallets.iter().map(|w| w.policy.clone()).collect()
    }

    /// The nonce check at the end of a spend: every signature
    /// the PSBT carries checked against its key and this transaction, each
    /// one a key held here made recomputed from that key (RFC 6979 or
    /// BIP-340 with no extra randomness), and any nonce used twice under
    /// one key. OpenSigner's own checks (`osk_psbt::verify`).
    pub fn nonce_check(&self, psbt: &Psbt) -> NonceCheck {
        let refs = self.key_refs();
        let policies = self.policies();
        let ctx = context(self.network, &refs, &policies);
        let masters: Vec<&MasterKey> = self.keys.iter().map(|k| &k.master).collect();
        let made = osk_psbt::verify::deterministic(psbt, &masters);
        let mut rows = Vec::new();
        for input in osk_psbt::verify::verify_signatures(psbt, &ctx) {
            for sig in input.signatures {
                let recomputed = made
                    .iter()
                    .find(|d| d.input == input.index && d.key == sig.key)
                    .map(|d| d.mode);
                rows.push(NonceRow {
                    input: input.index,
                    name: sig.name(),
                    verdict: sig.verdict,
                    recomputed,
                });
            }
        }
        NonceCheck {
            rows,
            reused: osk_psbt::verify::repeated_nonces(psbt, &ctx),
        }
    }

    fn share_refs(&self) -> Vec<osk_psbt::ShareRef> {
        self.keys
            .iter()
            .filter_map(|k| k.share.map(|pubshare| osk_psbt::ShareRef { pubshare }))
            .collect()
    }

    /// The inspection of `psbt` against this session's keys and wallets.
    pub fn inspect(&self, psbt: &Psbt) -> Inspection {
        self.inspect_with(psbt, None)
    }

    /// The inspection, with the carry section a threshold spend arrived
    /// with.
    pub fn inspect_with(
        &self,
        psbt: &Psbt,
        carry: Option<&osk_psbt::threshold::CarrySection>,
    ) -> Inspection {
        let refs = self.key_refs();
        let policies = self.policies();
        let shares = self.share_refs();
        let mut ctx = context(self.network, &refs, &policies);
        ctx.shares = &shares;
        ctx.carry = carry;
        osk_psbt::inspect(psbt, &ctx)
    }

    /// The loaded keys that are shares of the threshold wallet `w`, by
    /// identifier.
    pub fn shares_here(&self, w: &Wallet) -> Vec<(u32, &Key)> {
        let Some(record) = w.policy.record() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (id, pubshare) in record.info.pubshares.iter().enumerate() {
            let Some(pubshare) = pubshare else { continue };
            if let Some(k) = self
                .keys
                .iter()
                .find(|k| k.share.as_ref() == Some(pubshare))
            {
                out.push((id as u32, k));
            }
        }
        out
    }

    /// Signs `psbt` with every session key it names. Returns the
    /// fingerprints that signed.
    pub fn sign(
        &self,
        psbt: &mut Psbt,
        seed: [u8; 32],
        musig: &mut Option<osk_psbt::MusigSession>,
    ) -> Result<Vec<Fingerprint>, String> {
        self.sign_with(psbt, seed, musig, &[], None).map(|s| s.by)
    }

    /// Signs `psbt` with every session key and share it names. `others`
    /// are the shares chosen to sign with this one, by identifier, at a
    /// threshold spend's first location; `carry` is the section a later
    /// location reads.
    pub fn sign_with(
        &self,
        psbt: &mut Psbt,
        seed: [u8; 32],
        musig: &mut Option<osk_psbt::MusigSession>,
        others: &[u32],
        carry: Option<&osk_psbt::threshold::CarrySection>,
    ) -> Result<Signed, String> {
        let refs = self.key_refs();
        let policies = self.policies();
        let share_refs = self.share_refs();
        let mut ctx = context(self.network, &refs, &policies);
        ctx.shares = &share_refs;
        ctx.carry = carry;
        let inspection = osk_psbt::inspect(psbt, &ctx);
        let selection: Vec<Fingerprint> = inspection
            .participating_keys
            .iter()
            .copied()
            .filter(|fp| self.keys.iter().any(|k| k.master.fingerprint() == *fp))
            .collect();
        let shares_sign = inspection
            .inputs
            .iter()
            .any(|i| i.threshold.as_ref().is_some_and(|t| !t.ours.is_empty()));
        if selection.is_empty() && !shares_sign {
            return Err("No key in this session signs this transaction".to_string());
        }
        let masters: Vec<&MasterKey> = self.keys.iter().map(|k| &k.master).collect();
        // A share's scalar is rebuilt from its words only inside the call
        // the signer makes through its door.
        let members: Vec<&Key> = self.keys.iter().filter(|k| k.share.is_some()).collect();
        let doors: Vec<Box<osk_psbt::ShareDoor<'_>>> = members
            .iter()
            .map(|key| -> Box<osk_psbt::ShareDoor<'_>> {
                Box::new(move |take: &mut dyn FnMut(&osk_bip::frost::SecShare)| {
                    key.with_secret_share(take);
                })
            })
            .collect();
        let share_keys: Vec<osk_psbt::ShareKey> = members
            .iter()
            .zip(&doors)
            .filter_map(|(key, door)| {
                Some(osk_psbt::ShareKey {
                    pubshare: key.share?,
                    open: door.as_ref(),
                })
            })
            .collect();
        let result = osk_psbt::sign(
            psbt,
            &masters,
            &selection,
            &ctx,
            false,
            Nonce::default(),
            Aux::default(),
            musig,
            seed,
            &share_keys,
            others,
        )
        .map_err(|e| e.to_string())?;
        let mut by: Vec<Fingerprint> = result.signed_inputs.iter().map(|s| s.fingerprint).collect();
        by.dedup();
        Ok(Signed {
            by,
            carry: result.threshold.and_then(|t| t.carry),
        })
    }
}

fn context<'a>(network: Network, refs: &'a [KeyRef], policies: &'a [WalletPolicy]) -> Context<'a> {
    Context {
        network,
        keys: refs,
        wallets: policies,
        musig_session: None,
        shares: &[],
        carry: None,
    }
}

/// What kind of wallet a policy describes, which decides the steps of
/// its flows and the words they use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// One key: `pkh`, `sh(wpkh)`, `wpkh` or `tr` key path.
    Single(ScriptType),
    /// `multi` or `sortedmulti`, in `sh`, `wsh` or `sh(wsh)`.
    Multi(Wrapper),
    /// `tr` with an unspendable key and one `multi_a` or `sortedmulti_a`
    /// leaf.
    TapMulti,
    /// A `wsh` or `sh(wsh)` miniscript: spending paths with conditions.
    Miniscript,
    /// A `tr` with a key path and a tree of leaves.
    Tree,
    /// MuSig2: one aggregate key, every participant signs in two rounds.
    MuSig,
    /// FROST: a threshold of shares signs in two rounds.
    Threshold,
    /// Silent payments (BIP-352): one key's scan and spend halves, an
    /// `sp1…` address, and nothing to spend from yet.
    Silent,
    /// Anything else this build reads but has no flow for.
    Other,
}

/// The steps of a spend, in the order they may appear.
pub mod step {
    /// Which wallet the transaction spends from.
    pub const WALLET: u8 = 0;
    /// The wallet's first addresses, to compare with the online wallet.
    pub const CHECK: u8 = 1;
    /// The transaction in sentences, with its checks.
    pub const TRANSACTION: u8 = 2;
    /// The transaction id, before anyone signs.
    pub const TXID: u8 = 3;
    /// Which keys sign here and which elsewhere.
    pub const SIGNERS: u8 = 4;
    /// Signing with the keys held here.
    pub const SIGN: u8 = 5;
    /// Signatures arriving from other devices.
    pub const COLLECT: u8 = 6;
    /// The signed PSBT and the finished transaction.
    pub const FINISH: u8 = 7;
    /// Which way a wallet with several paths is spent.
    pub const PATH: u8 = 8;
    /// MuSig2's first round: everyone's nonces.
    pub const NONCES: u8 = 9;
}

impl Kind {
    /// The kind a policy describes.
    pub fn of(policy: &WalletPolicy) -> Kind {
        if policy.silent().is_some() {
            return Kind::Silent;
        }
        if policy.tapscript_quorum().is_some() {
            return Kind::TapMulti;
        }
        match policy.template() {
            Template::Single { script } => Kind::Single(script),
            Template::Multi { wrapper, .. } => Kind::Multi(wrapper),
            Template::Miniscript { .. } => Kind::Miniscript,
            Template::Tree => Kind::Tree,
            Template::MuSig => Kind::MuSig,
            Template::Threshold { .. } => Kind::Threshold,
            _ => Kind::Other,
        }
    }

    /// Whether more than one key can take part.
    pub fn has_cosigners(self) -> bool {
        !matches!(self, Kind::Single(_) | Kind::Silent | Kind::Other)
    }

    /// The steps of a spend from a wallet of this kind
    /// (`docs/WALLETS.md` §4). `txid` is whether the transaction's id is
    /// known before signing.
    pub fn steps(self, txid: bool) -> Vec<u8> {
        let mut v = vec![step::WALLET, step::CHECK];
        if matches!(self, Kind::Miniscript | Kind::Tree) {
            v.push(step::PATH);
        }
        v.push(step::TRANSACTION);
        if txid {
            v.push(step::TXID);
        }
        if self.has_cosigners() {
            v.push(step::SIGNERS);
        }
        if self == Kind::MuSig {
            v.push(step::NONCES);
        }
        v.push(step::SIGN);
        // A threshold spend is not collected here: each share signs in
        // turn, the carry file going from one to the next.
        if self.has_cosigners() && self != Kind::Threshold {
            v.push(step::COLLECT);
        }
        v.push(step::FINISH);
        v
    }
}

impl Session {
    /// The loaded wallet whose script the transaction's first input
    /// spends, found by re-deriving each wallet's script at the paths the
    /// input's own key origins state.
    pub fn wallet_for(&self, psbt: &Psbt) -> Option<usize> {
        let inner = psbt.inner();
        let input = inner.inputs.first()?;
        let txin = inner.unsigned_tx.input.first()?;
        let spk = match (&input.witness_utxo, &input.non_witness_utxo) {
            (Some(out), _) => out.script_pubkey.clone(),
            (None, Some(tx)) => tx
                .output
                .get(txin.previous_output.vout as usize)?
                .script_pubkey
                .clone(),
            _ => return None,
        };
        let mut places: Vec<(bool, u32)> = Vec::new();
        let paths = input
            .bip32_derivation
            .values()
            .map(|(_, p)| p.clone())
            .chain(input.tap_key_origins.values().map(|(_, (_, p))| p.clone()));
        for path in paths {
            let n: Vec<u32> = path.into_iter().map(|c| u32::from(*c)).collect();
            if n.len() >= 2 {
                let (chain, index) = (n[n.len() - 2], n[n.len() - 1]);
                if chain <= 1 && !places.contains(&(chain == 1, index)) {
                    places.push((chain == 1, index));
                }
            }
        }
        self.wallets.iter().position(|w| {
            places.iter().any(|(change, index)| {
                w.policy.script_at(*change, *index).ok().as_ref() == Some(&spk)
            })
        })
    }
}

/// Whether the transaction's id is fixed before it is signed: every
/// input spends a native SegWit or Taproot output, so no signature lands
/// in the part of the transaction its id is taken over.
pub fn txid_known(psbt: &Psbt) -> bool {
    let inner = psbt.inner();
    inner
        .inputs
        .iter()
        .zip(&inner.unsigned_tx.input)
        .all(|(input, txin)| {
            let spk = match (&input.witness_utxo, &input.non_witness_utxo) {
                (Some(out), _) => Some(out.script_pubkey.clone()),
                (None, Some(tx)) => tx
                    .output
                    .get(txin.previous_output.vout as usize)
                    .map(|o| o.script_pubkey.clone()),
                _ => None,
            };
            spk.is_some_and(|s| s.is_p2wpkh() || s.is_p2wsh() || s.is_p2tr())
        })
}

/// Signatures a spend from this wallet needs: the quorum of a multisig,
/// one for any other kind. A miniscript or a tree may want more on some
/// paths; for those the finish step says whether the script is satisfied.
pub fn needed(wallet: &Wallet) -> usize {
    if Kind::of(&wallet.policy) == Kind::MuSig {
        return wallet.policy.keys().len();
    }
    if let Some(r) = wallet.policy.record() {
        return r.t();
    }
    wallet
        .policy
        .quorum()
        .or_else(|| wallet.policy.tapscript_quorum())
        .map(|(m, _)| m)
        .unwrap_or(1)
}

/// A wallet's descriptor in one form, so that the same wallet read from a
/// descriptor, a wallet .json or a multisig config compares equal: they
/// write a hardened step as `h` or as `'`.
pub fn same_wallet(policy: &WalletPolicy) -> String {
    policy.to_descriptor().replace('\'', "h")
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
            .ok_or_else(|| Refusal::NotAWallet("a .json file with no descriptor".into()))?;
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
            .map_err(|e| Refusal::NotAWallet(format!("BSMS: {e:?}")));
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
    let bad = |why: &str| Refusal::NotAWallet(format!("Coldcard export: {why}"));
    let master = json_string(text, "xfp").ok_or_else(|| bad("no xfp"))?;
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
                return Err(bad(&format!("its first {field} address is not {first}")));
            }
        }
        return Ok(policy);
    }
    Err(bad("no single-sig account"))
}

/// The object under `key` in a JSON text, from its name to its closing
/// brace. Enough for the flat objects wallets write; not a JSON parser.
pub(crate) fn json_object<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let at = text.find(&format!("\"{key}\""))?;
    let rest = &text[at..];
    let open = rest.find('{')?;
    // The value must be the object itself, not a later one.
    if rest[key.len() + 2..open]
        .trim()
        .trim_start_matches(':')
        .trim()
        != ""
    {
        return None;
    }
    Some(&rest[..rest.find('}')? + 1])
}

/// An extended public key in BIP-32's own encoding, `xpub` or `tpub`,
/// whichever SLIP-132 spelling it came in (`ypub`, `zpub`, `Ypub`, `Zpub`
/// and their test-network forms). A key that is no extended public key is
/// `None`.
pub(crate) fn plain_xpub(key: &str) -> Option<String> {
    use osk_bip::bitcoin::base58;
    const MAIN: [[u8; 4]; 5] = [
        [0x04, 0x88, 0xb2, 0x1e],
        [0x04, 0x9d, 0x7c, 0xb2],
        [0x04, 0xb2, 0x47, 0x46],
        [0x02, 0x95, 0xb4, 0x3f],
        [0x02, 0xaa, 0x7e, 0xd3],
    ];
    const TEST: [[u8; 4]; 5] = [
        [0x04, 0x35, 0x87, 0xcf],
        [0x04, 0x4a, 0x52, 0x62],
        [0x04, 0x5f, 0x1c, 0xf6],
        [0x02, 0x42, 0x89, 0xef],
        [0x02, 0x57, 0x54, 0x83],
    ];
    let mut bytes = base58::decode_check(key.trim()).ok()?;
    if bytes.len() != 78 {
        return None;
    }
    let version: [u8; 4] = bytes[..4].try_into().ok()?;
    let plain = if MAIN.contains(&version) {
        MAIN[0]
    } else if TEST.contains(&version) {
        TEST[0]
    } else {
        return None;
    };
    bytes[..4].copy_from_slice(&plain);
    Some(base58::encode_check(&bytes))
}

/// The first string value under `key` in a JSON text, with its escapes
/// undone. Enough for the wallet files coordinators write; not a JSON
/// parser.
pub(crate) fn json_string(text: &str, key: &str) -> Option<String> {
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

/// What a file holds, as far as this build is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// A PSBT, binary or base64.
    Psbt,
    /// A wallet description.
    Wallet,
    /// A finished transaction, as hex.
    Transaction,
    /// A sheet to print on the desktop app.
    Sheet,
    /// A signed message: address, signature and text, one per line.
    Message,
    /// A cosigner's account key, `[fingerprint/path]xpub`.
    Key,
    /// A seed's words, English, checksum valid: a key that can be loaded.
    Words,
    /// A share of a split multisig backup: a config with some keys left off.
    Share,
    /// A Faraday vault (`docs/VAULT.md`), locked.
    Vault,
    /// Entries for a vault: `otpauth://` URIs and `field: value` lines.
    Entries,
    /// An OpenSigner encrypted backup (`osk-backup`).
    Backup,
    /// An EFI image to sign or check for Secure Boot.
    EfiImage,
    /// A threshold spend part-signed at another location: the PSBT with
    /// the secret nonce the next share signs with (`docs/PLANNING.md`
    /// §16.103 item 4).
    Carry,
    /// A KeePass database, sealed under its passphrase.
    Kdbx,
    /// A PDF to print: a backup sheet or a blank template.
    Pdf,
    /// A SLIP-39 share or a codex32 string: part of a seed, or a whole
    /// codex32 secret.
    SeedPart,
    /// Plain text Faraday reads as nothing else, such as recovery codes:
    /// kept as a note in a vault.
    Text,
    /// Nothing this build reads.
    Other,
}

/// What collecting signatures still asks for: "Collect 2 signatures"
/// before any is in, "Collect 1 more signature" after. "More" is said
/// only once there is one.
pub fn collect_line(missing: usize, collected: usize) -> String {
    format!(
        "Collect {missing}{} {}",
        if collected == 0 { "" } else { " more" },
        if missing == 1 {
            "signature"
        } else {
            "signatures"
        }
    )
}

/// Reads a PSBT from a file's bytes, binary or base64.
pub fn read_psbt(bytes: &[u8]) -> Option<Psbt> {
    if let Ok(p) = Psbt::parse_bytes(bytes) {
        return Some(p);
    }
    let text = std::str::from_utf8(bytes).ok()?;
    Psbt::parse_base64(text.trim()).ok()
}

/// The most a plain text file may hold to be kept as a note.
pub const TEXT_MAX: usize = 64 * 1024;

/// Plain text a person wrote or saved: printable, lines, and short
/// enough for a note.
fn is_text(text: &str) -> bool {
    !text.trim().is_empty()
        && text.len() <= TEXT_MAX
        && text
            .chars()
            .all(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
}

/// Says what a file holds.
pub fn classify(name: &str, bytes: &[u8]) -> FileKind {
    if faraday_vault::read_header(bytes).is_ok() {
        return FileKind::Vault;
    }
    if osk_backup::oskb::is_backup(bytes) {
        return FileKind::Backup;
    }
    if bytes.starts_with(b"%PDF-") {
        return FileKind::Pdf;
    }
    // KeePass's two signatures, little-endian.
    if bytes.starts_with(&[0x03, 0xD9, 0xA2, 0x9A, 0x67, 0xFB, 0x4B, 0xB5]) {
        return FileKind::Kdbx;
    }
    if crate::secureboot::is_image(name) && bytes.starts_with(b"MZ") {
        return FileKind::EfiImage;
    }
    if osk_psbt::threshold::Carry::looks_like_carry(bytes) {
        return FileKind::Carry;
    }
    if read_psbt(bytes).is_some() {
        return FileKind::Psbt;
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        if text.starts_with("faraday-sheet 1") {
            return FileKind::Sheet;
        }
        if let Some(m) = osk_psbt::message::parse_signed(text)
            && m.address.parse::<osk_bip::bitcoin::Address<_>>().is_ok()
        {
            return FileKind::Message;
        }
        if crate::restore::is_share(text) {
            return FileKind::Share;
        }
        if read_wallet(text).is_ok() {
            return FileKind::Wallet;
        }
        if crate::create::read_key(text).is_some() {
            return FileKind::Key;
        }
        if crate::forms::typed_mnemonic(Language::English, &crate::forms::file_words(text)).is_ok()
        {
            return FileKind::Words;
        }
        if faraday_vault::entries::looks_like_entries(text) {
            return FileKind::Entries;
        }
        if crate::inbox::part_of(text).is_some() {
            return FileKind::SeedPart;
        }
        // A raw transaction in hex, under any name: what a wallet or an
        // explorer gives to broadcast.
        let t = text.trim();
        if !t.is_empty()
            && t.bytes().all(|b| b.is_ascii_hexdigit())
            && osk_psbt::transaction::read_raw(t.as_bytes()).is_some()
        {
            return FileKind::Transaction;
        }
        if is_text(text) {
            return FileKind::Text;
        }
    }
    FileKind::Other
}

/// One transaction being signed, from the moment it is loaded.
pub struct Spend {
    /// The PSBT as it stands: every signature collected so far.
    pub psbt: Psbt,
    /// The file it came from.
    pub source: String,
    /// The unsigned transaction's id, which every collected copy must share.
    pub txid: Txid,
    /// Keys of this session that signed it here.
    pub signed_here: Vec<Fingerprint>,
    /// Fingerprints whose signatures arrived from other devices, with the
    /// file each came in.
    pub collected: Vec<(Fingerprint, String)>,
    /// The finished transaction, once there are enough signatures.
    pub finished: Option<Transaction>,
}

impl Spend {
    /// Starts a spend from a PSBT file.
    pub fn new(psbt: Psbt, source: &str) -> Spend {
        let txid = psbt.unsigned_tx().compute_txid();
        Spend {
            psbt,
            source: source.to_string(),
            txid,
            signed_here: Vec::new(),
            collected: Vec::new(),
            finished: None,
        }
    }

    /// Fingerprints with a valid signature on the first input; for a
    /// MuSig2 wallet, the participants whose partial signature is on it.
    pub fn signers(&self, session: &Session) -> Vec<Fingerprint> {
        if let Some(w) = session
            .wallet_for(&self.psbt)
            .and_then(|i| session.wallets.get(i))
            && Kind::of(&w.policy) == Kind::MuSig
        {
            let Some(input) = self.psbt.inner().inputs.first() else {
                return Vec::new();
            };
            if input.tap_key_sig.is_some() {
                return w
                    .policy
                    .keys()
                    .iter()
                    .filter_map(|k| k.fingerprint())
                    .collect();
            }
            let partials = osk_psbt::musig::partial_sigs(input).unwrap_or_default();
            return w
                .policy
                .keys()
                .iter()
                .filter(|k| {
                    partials
                        .iter()
                        .any(|p| p.participant == k.xpub().public_key)
                })
                .filter_map(|k| k.fingerprint())
                .collect();
        }
        let inspection = session.inspect(&self.psbt);
        let mut out: Vec<Fingerprint> = inspection
            .present_signatures()
            .filter(|s| s.input == 0 && s.verdict == Verdict::Valid)
            .filter_map(|s| s.fingerprint)
            .collect();
        out.dedup();
        out
    }

    /// Adds the signatures another device put on a copy of this
    /// transaction. A PSBT for any other transaction is refused, never
    /// merged. Returns the fingerprints it added.
    pub fn collect(
        &mut self,
        session: &Session,
        bytes: &[u8],
        source: &str,
    ) -> Result<Vec<Fingerprint>, String> {
        let other = read_psbt(bytes).ok_or("Not a PSBT")?;
        if other.unsigned_tx().compute_txid() != self.txid {
            return Err(format!("{source} is a PSBT for another transaction"));
        }
        let before = self.signers(session);
        let mut merged = self.psbt.inner().clone();
        merged
            .combine(other.into_inner())
            .map_err(|e| format!("Cannot combine: {e}"))?;
        // `combine` keeps the fields it has no type for, MuSig2's among them.
        let merged = Psbt::from(merged);
        if session
            .inspect(&merged)
            .present_signatures()
            .any(|s| s.verdict.is_invalid())
        {
            return Err(format!("A signature in {source} does not verify"));
        }
        self.psbt = merged;
        let added: Vec<Fingerprint> = self
            .signers(session)
            .into_iter()
            .filter(|fp| !before.contains(fp))
            .collect();
        for fp in &added {
            self.collected.push((*fp, source.to_string()));
        }
        Ok(added)
    }

    /// Finishes the transaction when it has enough signatures.
    pub fn finish(&mut self) -> Result<(), String> {
        let mut copy = self.psbt.clone();
        match osk_psbt::finalize(&mut copy) {
            Ok(Some(tx)) => {
                self.finished = Some(tx);
                Ok(())
            }
            Ok(None) => Err("Not enough signatures yet".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Why the transaction cannot be finished yet, when it cannot.
    pub fn finish_reason(&self) -> Option<String> {
        let mut copy = self.psbt.clone();
        match osk_psbt::finalize(&mut copy) {
            Ok(Some(_)) => None,
            Ok(None) => Some(
                "The script is not satisfied yet: another signature, or a timelock not yet reached"
                    .to_string(),
            ),
            Err(e) => Some(e.to_string()),
        }
    }

    /// The finished transaction as hex.
    pub fn finished_hex(&self) -> Option<String> {
        self.finished.as_ref().map(serialize_hex)
    }
}

/// What checking a signed message found.
pub struct MessageCheck {
    /// The signed text, as read.
    pub signed: osk_psbt::message::SignedText,
    /// The signature verifies against the address, and in which format;
    /// or why not.
    pub verdict: Result<osk_psbt::message::Format, String>,
    /// The loaded wallet the address belongs to, with its chain and index.
    pub wallet: Option<(usize, bool, u32)>,
}

impl Session {
    /// The single-key wallets a message can be signed for here.
    pub fn message_wallets(&self) -> Vec<usize> {
        self.wallets
            .iter()
            .enumerate()
            .filter(|(_, w)| matches!(Kind::of(&w.policy), Kind::Single(_)))
            .filter(|(_, w)| {
                w.policy
                    .keys()
                    .first()
                    .and_then(|k| k.fingerprint())
                    .is_some_and(|fp| self.key_label(fp).is_some())
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Signs `message` with the key behind address `index` of a
    /// single-key wallet's receive chain.
    pub fn sign_message(
        &self,
        wallet: usize,
        index: u32,
        message: &str,
        format: osk_psbt::message::Format,
    ) -> Result<osk_psbt::message::Signed, String> {
        let w = self.wallets.get(wallet).ok_or("no such wallet")?;
        let Kind::Single(script) = Kind::of(&w.policy) else {
            return Err("Messages are signed with single-key wallets".into());
        };
        let key = w.policy.keys().first().ok_or("the wallet has no key")?;
        let fp = key.fingerprint().ok_or("the wallet's xpub has no origin")?;
        let master = &self
            .keys
            .iter()
            .find(|k| k.master.fingerprint() == fp)
            .ok_or("the wallet's key is not loaded here")?
            .master;
        let origin = key.path().ok_or("the wallet's xpub has no path")?;
        let path = origin
            .child(
                osk_bip::bitcoin::bip32::ChildNumber::from_normal_idx(0)
                    .map_err(|e| e.to_string())?,
            )
            .child(
                osk_bip::bitcoin::bip32::ChildNumber::from_normal_idx(index)
                    .map_err(|e| e.to_string())?,
            );
        let derived = master.derive(&path);
        osk_psbt::message::sign(
            derived.secp(),
            derived.secret_key(),
            script,
            self.network,
            message,
            format,
            Nonce::default(),
            Aux::default(),
        )
        .map_err(|e| format!("{e:?}"))
    }

    /// Checks a signed message, and finds its address among the loaded
    /// wallets' first hundred addresses on each chain.
    pub fn check_message(&self, text: &str) -> Option<MessageCheck> {
        let signed = osk_psbt::message::parse_signed(text)?;
        let address = signed
            .address
            .parse::<osk_bip::bitcoin::Address<_>>()
            .ok()?
            .require_network(self.network.into())
            .ok()?;
        let verdict = osk_psbt::message::verify(&address, &signed.message, &signed.signature)
            .map(|c| c.format)
            .map_err(|e| format!("{e:?}"));
        let wallet = self.wallets.iter().enumerate().find_map(|(i, w)| {
            w.policy
                .find_address(self.network, &address, 100)
                .map(|(change, index)| (i, change, index))
        });
        Some(MessageCheck {
            signed,
            verdict,
            wallet,
        })
    }
}

/// What a path of a wallet with several needs, and whether this
/// transaction can take it.
pub struct PathView {
    /// The keys it needs, by slot number from 1, with whether each is here.
    pub keys: Vec<(usize, String, bool)>,
    /// Its locks, in words.
    pub locks: Vec<String>,
    /// Whether the first input's sequence allows every relative lock, and
    /// the absolute ones are not checked here.
    pub sequence_ok: bool,
}

/// Every path of a wallet, read against the transaction's first input.
pub fn paths(session: &Session, wallet: &Wallet, psbt: &Psbt) -> Vec<PathView> {
    use osk_bip::spend::Lock;
    let seq = psbt
        .unsigned_tx()
        .input
        .first()
        .map(|i| i.sequence.0)
        .unwrap_or(0xffff_ffff);
    // BIP-68: a relative lock is in force when bit 31 is clear; bit 22
    // chooses 512-second intervals over blocks; the low 16 bits count.
    let relative = seq & (1 << 31) == 0;
    let in_time = seq & (1 << 22) != 0;
    let value = seq & 0xffff;
    let slots = session.slots(wallet);
    wallet
        .policy
        .spend_paths()
        .iter()
        .map(|p| {
            let keys = p
                .keys
                .iter()
                .map(|&k| {
                    let s = &slots[k];
                    (
                        k + 1,
                        s.fingerprint.map(fp_text).unwrap_or_default(),
                        s.held_by.is_some(),
                    )
                })
                .collect();
            let mut ok = true;
            let locks = p
                .locks
                .iter()
                .map(|l| match l {
                    Lock::Blocks(n) => {
                        ok &= relative && !in_time && value >= *n;
                        format!(
                            "{n} blocks after the coins arrived (about {} days)",
                            n / 144
                        )
                    }
                    Lock::Intervals(n) => {
                        ok &= relative && in_time && value >= *n;
                        format!(
                            "{} days after the coins arrived",
                            u64::from(*n) * 512 / 86_400
                        )
                    }
                    Lock::Height(h) => format!("after block {h}"),
                    Lock::Time(t) => format!("after time {t}"),
                    Lock::Preimage => "a secret preimage".to_string(),
                })
                .collect();
            PathView {
                keys,
                locks,
                sequence_ok: ok,
            }
        })
        .collect()
}

/// MuSig2's first round on a spend: each participant's slot number,
/// fingerprint, whether its key is here, and whether its nonce is on the
/// PSBT.
pub fn musig_nonces(
    session: &Session,
    wallet: &Wallet,
    psbt: &Psbt,
) -> Vec<(usize, String, bool, bool)> {
    let nonces = psbt
        .inner()
        .inputs
        .first()
        .and_then(|i| osk_psbt::musig::pub_nonces(i).ok())
        .unwrap_or_default();
    session
        .slots(wallet)
        .iter()
        .zip(wallet.policy.keys())
        .enumerate()
        .map(|(i, (slot, key))| {
            let has = nonces
                .iter()
                .any(|n| n.participant == key.xpub().public_key);
            (
                i + 1,
                slot.fingerprint.map(fp_text).unwrap_or_default(),
                slot.held_by.is_some(),
                has,
            )
        })
        .collect()
}
