//! BIP-32 private keys: the master key from a seed, hardened derivation,
//! and the master fingerprint.
//!
//! Everything that holds private material lives in this file so that the
//! secret lint (`tools/lint-secrets.sh`, rule 3) can keep it free of heap
//! text. The non-secret account view ([`AccountXpub`]) lives in
//! [`crate::account`].
//!
//! ```
//! use osk_bip::bip39::{Language, Mnemonic};
//! use osk_bip::keys::{MasterKey, Network, ScriptType};
//!
//! let m = Mnemonic::from_entropy(Language::English, &[0u8; 16]).unwrap();
//! let seed = m.to_seed(b"").unwrap();
//! let master = MasterKey::from_seed(&seed, Network::Mainnet);
//! assert_eq!(master.fingerprint().to_hex(), *b"73c5da0a");
//! let account = master.account_xpub(ScriptType::NativeSegwit, 0).unwrap();
//! assert_eq!(account.path().len(), 3);
//! ```
//!
//! # Erasure is best-effort
//!
//! [`MasterKey`] and [`DerivedKey`] wrap `bitcoin::bip32::Xpriv`, which is
//! a plain `Copy` type with no zeroizing behaviour of its own. On drop we
//! overwrite the private key (`SecretKey::non_secure_erase`) and the chain
//! code, and the key handed to a pinned page is erased where it stood, but
//! copies made on the stack inside `bitcoin` during derivation, and the
//! HMAC state inside `Xpriv::new_master`, are outside our reach. This is
//! the interim discipline until `Sealed<T>` from `docs/PLANNING.md` §5.1
//! exists.
//!
//! # One context per key
//!
//! A [`MasterKey`] carries the libsecp256k1 context its own operations
//! run in, randomized when the key is made and again whenever the
//! session hands down fresh entropy ([`MasterKey::reblind`]). Callers
//! that sign with a key use [`MasterKey::secp`] rather than making a
//! context of their own, so that no curve operation on a private key
//! runs unblinded (security review M1). Contexts that only verify are
//! built where they are needed and need no blinding: there is no secret
//! in them.

use core::fmt;

use bitcoin::NetworkKind;
use bitcoin::bip32::Xpriv;
use bitcoin::secp256k1::{All, Secp256k1};
use osk_crypto::{Pinned, Secret, SeedBytes, Zeroize, hmac_sha512};

pub use bitcoin::bip32::{ChildNumber, DerivationPath, Xpub};
pub use bitcoin::secp256k1::SecretKey;

use crate::account::{AccountXpub, MultisigAccountXpub};

/// The Bitcoin network a key or address belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Network {
    /// Mainnet: coin type 0, `xpub`, `bc1` addresses.
    Mainnet,
    /// Testnet (testnet3): coin type 1, `tpub`, `tb1` addresses.
    Testnet,
    /// Signet: same key encoding and address prefixes as testnet.
    Signet,
    /// Regtest: coin type 1, `tpub`, `bcrt1` addresses.
    Regtest,
}

impl Network {
    /// Every network, mainnet first.
    pub const ALL: [Network; 4] = [
        Network::Mainnet,
        Network::Testnet,
        Network::Signet,
        Network::Regtest,
    ];

    /// Lower-case name: `mainnet`, `testnet`, `signet` or `regtest`.
    pub fn name(self) -> &'static str {
        match self {
            Network::Mainnet => "mainnet",
            Network::Testnet => "testnet",
            Network::Signet => "signet",
            Network::Regtest => "regtest",
        }
    }

    /// Parses a name produced by [`name`](Self::name); also accepts
    /// `bitcoin` for mainnet.
    pub fn from_name(name: &str) -> Option<Network> {
        match name {
            "mainnet" | "bitcoin" => Some(Network::Mainnet),
            "testnet" => Some(Network::Testnet),
            "signet" => Some(Network::Signet),
            "regtest" => Some(Network::Regtest),
            _ => None,
        }
    }

    /// The SLIP-44 coin type used in account paths: 0 for mainnet, 1 for
    /// every test network.
    pub fn coin_type(self) -> u32 {
        match self {
            Network::Mainnet => 0,
            _ => 1,
        }
    }

    /// Whether this is mainnet.
    pub fn is_mainnet(self) -> bool {
        self == Network::Mainnet
    }

    /// Extended-key version class: mainnet or test.
    pub fn kind(self) -> NetworkKind {
        NetworkKind::from(bitcoin::Network::from(self))
    }
}

impl From<Network> for bitcoin::Network {
    fn from(n: Network) -> Self {
        match n {
            Network::Mainnet => bitcoin::Network::Bitcoin,
            Network::Testnet => bitcoin::Network::Testnet,
            Network::Signet => bitcoin::Network::Signet,
            Network::Regtest => bitcoin::Network::Regtest,
        }
    }
}

impl From<Network> for NetworkKind {
    fn from(n: Network) -> Self {
        n.kind()
    }
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Single-signature script type, which fixes the BIP-43 purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptType {
    /// BIP-44, pay-to-pubkey-hash (`1…` / `m…`/`n…`).
    Legacy,
    /// BIP-49, pay-to-witness-pubkey-hash nested in P2SH (`3…` / `2…`).
    NestedSegwit,
    /// BIP-84, native pay-to-witness-pubkey-hash (`bc1q…` / `tb1q…`).
    NativeSegwit,
    /// BIP-86, pay-to-taproot with an unspendable script path (`bc1p…`).
    Taproot,
}

impl ScriptType {
    /// Every script type, in BIP number order.
    pub const ALL: [ScriptType; 4] = [
        ScriptType::Legacy,
        ScriptType::NestedSegwit,
        ScriptType::NativeSegwit,
        ScriptType::Taproot,
    ];

    /// The BIP-43 purpose: 44, 49, 84 or 86.
    pub fn purpose(self) -> u32 {
        match self {
            ScriptType::Legacy => 44,
            ScriptType::NestedSegwit => 49,
            ScriptType::NativeSegwit => 84,
            ScriptType::Taproot => 86,
        }
    }

    /// Short name for display: `legacy`, `nested-segwit`, `native-segwit`
    /// or `taproot`.
    pub fn name(self) -> &'static str {
        match self {
            ScriptType::Legacy => "legacy",
            ScriptType::NestedSegwit => "nested-segwit",
            ScriptType::NativeSegwit => "native-segwit",
            ScriptType::Taproot => "taproot",
        }
    }
}

impl fmt::Display for ScriptType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Multisig script type, which fixes the fourth component of a BIP-48
/// account path.
///
/// BIP-48 defines two: `1'` for P2SH-P2WSH and `2'` for P2WSH. The `3'`
/// some wallets use for Taproot multisig is not in the standard and is
/// not derived here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MultisigScriptType {
    /// BIP-48 script type `1'`: multisig witness script nested in P2SH.
    NestedSegwit,
    /// BIP-48 script type `2'`: native P2WSH multisig.
    NativeSegwit,
}

impl MultisigScriptType {
    /// Both BIP-48 script types, in path order.
    pub const ALL: [MultisigScriptType; 2] = [
        MultisigScriptType::NestedSegwit,
        MultisigScriptType::NativeSegwit,
    ];

    /// The script-type component of the account path: 1 or 2, hardened.
    pub fn index(self) -> u32 {
        match self {
            MultisigScriptType::NestedSegwit => 1,
            MultisigScriptType::NativeSegwit => 2,
        }
    }

    /// Short name for display: `nested-segwit-multisig` or
    /// `native-segwit-multisig`.
    pub fn name(self) -> &'static str {
        match self {
            MultisigScriptType::NestedSegwit => "nested-segwit-multisig",
            MultisigScriptType::NativeSegwit => "native-segwit-multisig",
        }
    }
}

impl fmt::Display for MultisigScriptType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Why a derivation request was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// An account or address index is 2³¹ or larger.
    IndexOutOfRange,
    /// The extended private key is not at depth 0 with a zero parent
    /// fingerprint and child number, so it is not a master key.
    NotMaster,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::IndexOutOfRange => f.write_str("index must be below 2^31"),
            Error::NotMaster => f.write_str("extended private key is not a master key"),
        }
    }
}

impl core::error::Error for Error {}

/// Why an extended private key offered as text was not taken as a master
/// key. Both halves are the existing vocabulary: the decoder's reason, or
/// the master-key rule [`Error::NotMaster`] states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    /// The text is not a valid extended private key.
    Decode(crate::xkey::Error),
    /// It decodes, but it is not a master key.
    NotMaster,
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportError::Decode(e) => e.fmt(f),
            ImportError::NotMaster => Error::NotMaster.fmt(f),
        }
    }
}

impl core::error::Error for ImportError {}

/// The first four bytes of `HASH160(compressed public key)`, BIP-32's key
/// identifier. Non-secret; identifies a wallet on screen and in key
/// origins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fingerprint(pub [u8; 4]);

impl Fingerprint {
    /// The raw bytes.
    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }

    /// Lower-case hex, eight ASCII bytes.
    pub fn to_hex(self) -> [u8; 8] {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut out = [0u8; 8];
        for (i, b) in self.0.iter().enumerate() {
            out[2 * i] = DIGITS[usize::from(b >> 4)];
            out[2 * i + 1] = DIGITS[usize::from(b & 0x0f)];
        }
        out
    }
}

impl fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hex = self.to_hex();
        // Hex digits are ASCII, so the buffer is valid UTF-8.
        f.write_str(core::str::from_utf8(&hex).expect("ascii hex"))
    }
}

impl From<bitcoin::bip32::Fingerprint> for Fingerprint {
    fn from(fp: bitcoin::bip32::Fingerprint) -> Self {
        Fingerprint(fp.to_bytes())
    }
}

impl From<Fingerprint> for bitcoin::bip32::Fingerprint {
    fn from(fp: Fingerprint) -> Self {
        bitcoin::bip32::Fingerprint::from(fp.0)
    }
}

/// Longest base58check extended private key: 78 bytes plus a 4-byte
/// checksum encode to at most 112 characters.
pub const XPRIV_ASCII_MAX: usize = 112;

/// An extended private key in its base58check text form, in a fixed
/// buffer that zeroizes on drop. The explorer shows it while a hold
/// button is held (`docs/PLANNING.md` §4.6); nothing else reads it.
pub struct XprivAscii {
    buf: [u8; XPRIV_ASCII_MAX],
    len: u8,
}

impl XprivAscii {
    fn of(xpriv: &Xpriv) -> Self {
        let mut out = XprivAscii {
            buf: [0; XPRIV_ASCII_MAX],
            len: 0,
        };
        // `Display` writes plain ASCII; a key that did not fit (which
        // cannot happen at 78 bytes) leaves the text truncated, never
        // panics.
        let _ = core::fmt::write(&mut out, format_args!("{xpriv}"));
        out
    }

    /// The text.
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..usize::from(self.len)]).expect("base58 is ascii")
    }
}

impl core::fmt::Write for XprivAscii {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let n = usize::from(self.len);
        let bytes = s.as_bytes();
        if n + bytes.len() > XPRIV_ASCII_MAX {
            return Err(core::fmt::Error);
        }
        self.buf[n..n + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len() as u8;
        Ok(())
    }
}

impl Drop for XprivAscii {
    fn drop(&mut self) {
        use osk_crypto::Zeroize;
        self.buf.zeroize();
        self.len = 0;
    }
}

/// Longest WIF: a compressed key is 38 bytes with its checksum, which
/// base58check writes in 52 characters.
pub const WIF_ASCII_MAX: usize = 53;

/// A private key in its compressed WIF text form, in a fixed buffer that
/// zeroizes on drop. BIP-85's application 2' derives one
/// ([`crate::bip85::child_wif`]); nothing else writes a WIF.
pub struct WifAscii {
    buf: [u8; WIF_ASCII_MAX],
    len: u8,
}

impl WifAscii {
    /// The compressed WIF of `key` on `network`.
    pub fn new(key: &SecretKey, network: Network) -> Self {
        let mut private = bitcoin::PrivateKey::new(*key, network.kind());
        let mut out = WifAscii {
            buf: [0; WIF_ASCII_MAX],
            len: 0,
        };
        // `Display` on a private key is its WIF, plain ASCII; a key that
        // did not fit (which cannot happen at 38 bytes) leaves the text
        // truncated rather than panicking.
        let _ = fmt::write(&mut out, format_args!("{private}"));
        private.inner.non_secure_erase();
        out
    }

    /// The text.
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..usize::from(self.len)]).expect("base58 is ascii")
    }
}

impl fmt::Write for WifAscii {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let n = usize::from(self.len);
        let bytes = s.as_bytes();
        if n + bytes.len() > WIF_ASCII_MAX {
            return Err(fmt::Error);
        }
        self.buf[n..n + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len() as u8;
        Ok(())
    }
}

impl Drop for WifAscii {
    fn drop(&mut self) {
        self.buf.zeroize();
        self.len = 0;
    }
}

/// Domain separation for the blind a key derives for itself.
const SECP_BLIND_LABEL: &[u8] = b"osk-secp-blind";

/// A signing context blinded for `xpriv`.
///
/// A libsecp256k1 context multiplies its intermediate values by a secret
/// blinding factor, so that a power or timing trace of one signature
/// does not line up with a trace of the next (security review M1). The
/// factor comes from 32 bytes the caller supplies, and the crate is
/// built without `rand`, so there is no random number to hand it: this
/// derives one from the key's own chain code, which an attacker who
/// could predict it would already hold the key. A session hands down
/// something better with [`MasterKey::reblind`], which mixes in the
/// entropy the shell supplied.
fn blinded_context(xpriv: &Xpriv) -> Secp256k1<All> {
    let mut secp = Secp256k1::new();
    let mut derived = hmac_sha512(xpriv.chain_code.as_bytes(), SECP_BLIND_LABEL);
    let mut blind = [0u8; 32];
    blind.copy_from_slice(&derived[..32]);
    secp.seeded_randomize(&blind);
    blind.zeroize();
    derived.zeroize();
    secp
}

/// Overwrites the private key and chain code of an `Xpriv` in place.
///
/// Both writes have to survive the optimiser, which is free to drop a
/// plain store to a value nothing reads again: `non_secure_erase` writes
/// volatile, and so does `zeroize`.
fn erase(xpriv: &mut Xpriv) {
    xpriv.private_key.non_secure_erase();
    AsMut::<[u8; 32]>::as_mut(&mut xpriv.chain_code).zeroize();
}

/// An `Xpriv` that a [`Pinned`] page can hold: erasing it is zeroizing
/// it, by the same best-effort rule as [`MasterKey`]'s drop.
struct XprivCell(Xpriv);

impl Zeroize for XprivCell {
    fn zeroize(&mut self) {
        erase(&mut self.0);
    }
}

/// The BIP-32 master private key for one network.
///
/// Built from a 64-byte seed. Implements none of `Debug`, `Display` or
/// `Clone`; the only things that leave are non-secret ([`fingerprint`],
/// [`xpub`], [`account_xpub`]) or a [`DerivedKey`] under the same rules.
///
/// [`fingerprint`]: Self::fingerprint
/// [`xpub`]: Self::xpub
/// [`account_xpub`]: Self::account_xpub
pub struct MasterKey {
    xpriv: Pinned<XprivCell>,
    network: Network,
    /// The context every signature and derivation of this key runs in,
    /// blinded when the key was made and again whenever the session
    /// hands down fresh entropy ([`reblind`](Self::reblind)). One per
    /// key, so that a context is only ever blinded for the key it
    /// operates on.
    secp: Secp256k1<All>,
}

impl MasterKey {
    /// The extended private key itself.
    fn xpriv(&self) -> &Xpriv {
        &self.xpriv.expose().0
    }

    /// Wraps `xpriv` on a pinned page and erases it where it stood, so
    /// that the only copy is the pinned one.
    fn hold(xpriv: &mut Xpriv, network: Network) -> Self {
        let held = MasterKey {
            xpriv: Pinned::new(XprivCell(*xpriv)),
            network,
            secp: blinded_context(xpriv),
        };
        erase(xpriv);
        held
    }

    /// The signing context this key's operations run in.
    ///
    /// Signing code outside this crate uses it rather than making a
    /// context of its own, so that every curve operation on this key is
    /// blinded (security review M1).
    pub fn secp(&self) -> &Secp256k1<All> {
        &self.secp
    }

    /// Randomizes the signing context from `blind`.
    ///
    /// The session derives `blind` from the session key, so it is
    /// unpredictable and new after every entropy rotation. Blinding
    /// changes no result: the same key and message give the same
    /// signature whatever the context was randomized with.
    pub fn reblind(&mut self, blind: &[u8; 32]) {
        self.secp.seeded_randomize(blind);
    }

    /// The same key with its context randomized from `blind`, for
    /// callers that build and hand on a key in one expression.
    #[must_use]
    pub fn blinded(mut self, blind: &[u8; 32]) -> Self {
        self.reblind(blind);
        self
    }

    /// Derives the master key from a BIP-39 (or other) 64-byte seed.
    pub fn from_seed(seed: &Secret<[u8; 64]>, network: Network) -> Self {
        // `new_master` fails only if HMAC-SHA512 yields a key of zero or at
        // least the curve order, probability about 2^-128.
        let mut xpriv = Xpriv::new_master(network, seed.expose()).expect("seed yields a valid key");
        Self::hold(&mut xpriv, network)
    }

    /// Derives the master key from a seed of the length it was made at:
    /// 64 bytes from BIP-39 words, 16 or 32 from a SLIP-39 master
    /// secret, which is the BIP-32 seed itself.
    pub fn from_seed_bytes(seed: &Secret<SeedBytes>, network: Network) -> Self {
        let mut xpriv =
            Xpriv::new_master(network, seed.expose().as_bytes()).expect("seed yields a valid key");
        Self::hold(&mut xpriv, network)
    }

    /// Wraps an existing master extended private key (`xprv`/`tprv` at
    /// depth 0). The network comes from the key's version bytes: mainnet
    /// for `xprv`, testnet for the test version shared by testnet, signet
    /// and regtest; use [`for_network`](Self::for_network) to relabel.
    ///
    /// This exists for test vectors that are stated as a master `tprv`
    /// (BIP-174) and for keys already in memory. A key that arrives as
    /// text goes through [`decode`](Self::decode), which applies every
    /// BIP-32 vector-5 rule before `bitcoin` sees the bytes.
    pub fn from_xpriv(mut xpriv: Xpriv) -> Result<Self, Error> {
        let root_child = ChildNumber::from_normal_idx(0).expect("0 is a valid index");
        if xpriv.depth != 0
            || xpriv.parent_fingerprint != bitcoin::bip32::Fingerprint::default()
            || xpriv.child_number != root_child
        {
            erase(&mut xpriv);
            return Err(Error::NotMaster);
        }
        let network = match xpriv.network {
            NetworkKind::Main => Network::Mainnet,
            NetworkKind::Test => Network::Testnet,
        };
        Ok(Self::hold(&mut xpriv, network))
    }

    /// A depth-0 extended private key built from a chain code and a
    /// private key, which is what BIP-85's application 32' derives: the
    /// child number, the depth and the parent fingerprint are zero.
    ///
    /// `None` where the 32 bytes are not a valid secret key, which BIP-32
    /// puts below 1 in 2¹²⁷ and which BIP-85 says to hard fail on.
    pub fn from_chain_code_and_key(
        chain_code: &[u8; 32],
        key: &[u8; 32],
        network: Network,
    ) -> Option<Self> {
        let mut secret = SecretKey::from_slice(key).ok()?;
        let mut xpriv = Xpriv {
            network: network.kind(),
            depth: 0,
            parent_fingerprint: bitcoin::bip32::Fingerprint::default(),
            child_number: ChildNumber::from_normal_idx(0).expect("0 is a valid index"),
            private_key: secret,
            chain_code: bitcoin::bip32::ChainCode::from(*chain_code),
        };
        let held = Self::hold(&mut xpriv, network);
        secret.non_secure_erase();
        Some(held)
    }

    /// Imports a master extended private key written as text
    /// (`xprv…`/`tprv…`), by the strict rules of [`crate::xkey`].
    ///
    /// This is the only way an `xprv` typed, pasted or scanned by a
    /// person becomes a key: `bitcoin`'s own decoders accept seven of the
    /// sixteen serialisations BIP-32 test vector 5 rejects.
    pub fn decode(text: &str) -> Result<Self, ImportError> {
        let mut xpriv = crate::xkey::decode_xpriv(text).map_err(ImportError::Decode)?;
        // `Xpriv` is `Copy`, so handing it on leaves this frame's copy
        // behind; it is erased whichever way the import went.
        let key = Self::from_xpriv(xpriv).map_err(|_| ImportError::NotMaster);
        erase(&mut xpriv);
        key
    }

    /// The network this key was created for.
    pub fn network(&self) -> Network {
        self.network
    }

    /// The same master key re-labelled for `network`. Key material and the
    /// fingerprint are network-independent; only the coin type in account
    /// paths and the version bytes of extended keys change.
    ///
    /// The new key carries a copy of this one's context, blind and all:
    /// the two keys are the same key material, so a context blinded for
    /// one is blinded for the other.
    pub fn for_network(&self, network: Network) -> MasterKey {
        let mut xpriv = *self.xpriv();
        xpriv.network = network.kind();
        let held = MasterKey {
            xpriv: Pinned::new(XprivCell(xpriv)),
            network,
            secp: self.secp.clone(),
        };
        // `Xpriv` is `Copy`: this frame's copy goes no further.
        erase(&mut xpriv);
        held
    }

    /// The master fingerprint, e.g. `73c5da0a` for the "abandon … about"
    /// mnemonic.
    pub fn fingerprint(&self) -> Fingerprint {
        self.xpriv().fingerprint(&self.secp).into()
    }

    /// The master extended public key (depth 0).
    pub fn xpub(&self) -> Xpub {
        Xpub::from_priv(&self.secp, self.xpriv())
    }

    /// The master extended private key as text (`xprv…`/`tprv…`), for a
    /// hold-to-reveal row. The buffer zeroizes on drop.
    pub fn xpriv_ascii(&self) -> XprivAscii {
        XprivAscii::of(self.xpriv())
    }

    /// Derives the private child at `path` (relative to the master, so
    /// `m/84h/0h/0h` and `84h/0h/0h` parse to the same thing).
    pub fn derive(&self, path: &DerivationPath) -> DerivedKey {
        // Fails only on the 2^-128 event that a child key is invalid.
        let mut xpriv = self
            .xpriv()
            .derive_priv(&self.secp, path)
            .expect("child derivation yields a valid key");
        let derived = DerivedKey {
            xpriv,
            master_fingerprint: self.xpriv().fingerprint(&self.secp).into(),
            path: path.clone(),
            // Cloning copies the blinded context as it stands, so the
            // child runs in a context blinded like its parent's.
            secp: self.secp.clone(),
        };
        // `Xpriv` is `Copy`: the struct took a copy and this frame still
        // holds one, which goes no further.
        erase(&mut xpriv);
        derived
    }

    /// The account path `m/purpose'/coin'/account'` for `script_type` on
    /// this key's network. `account` must be below 2³¹.
    pub fn account_path(
        &self,
        script_type: ScriptType,
        account: u32,
    ) -> Result<DerivationPath, Error> {
        let hardened =
            |i: u32| ChildNumber::from_hardened_idx(i).map_err(|_| Error::IndexOutOfRange);
        Ok(DerivationPath::from(
            [
                hardened(script_type.purpose())?,
                hardened(self.network.coin_type())?,
                hardened(account)?,
            ]
            .as_slice(),
        ))
    }

    /// The BIP-48 account path `m/48'/coin'/account'/script'` for a
    /// multisig account on this key's network. `account` must be below
    /// 2³¹.
    pub fn multisig_account_path(
        &self,
        script_type: MultisigScriptType,
        account: u32,
    ) -> Result<DerivationPath, Error> {
        let hardened =
            |i: u32| ChildNumber::from_hardened_idx(i).map_err(|_| Error::IndexOutOfRange);
        Ok(DerivationPath::from(
            [
                hardened(48)?,
                hardened(self.network.coin_type())?,
                hardened(account)?,
                hardened(script_type.index())?,
            ]
            .as_slice(),
        ))
    }

    /// The non-secret account view of a BIP-48 multisig account: the
    /// account xpub and its origin. Addresses need the other cosigners,
    /// so this carries no address or descriptor of its own.
    pub fn multisig_account_xpub(
        &self,
        script_type: MultisigScriptType,
        account: u32,
    ) -> Result<MultisigAccountXpub, Error> {
        let path = self.multisig_account_path(script_type, account)?;
        let derived = self.derive(&path);
        Ok(MultisigAccountXpub::new(
            derived.to_xpub(),
            derived.master_fingerprint,
            path,
            script_type,
            self.network,
        ))
    }

    /// Hands `f` the private key of the BIP-48 account node itself,
    /// with the non-secret view of the same node, for the length of one
    /// call.
    ///
    /// The account node's own key is what BIP 129's key record is
    /// signed with ([`crate::bsms::signer_record`]), and it is the only
    /// caller this door has. The derived key is erased when the call
    /// returns, as every [`DerivedKey`] is, so nothing of it outlives
    /// `f`; copying the secret out of `f` would defeat that.
    pub fn with_multisig_account_secret<T>(
        &self,
        script_type: MultisigScriptType,
        account: u32,
        f: impl FnOnce(&SecretKey, &MultisigAccountXpub) -> T,
    ) -> Result<T, Error> {
        let path = self.multisig_account_path(script_type, account)?;
        let derived = self.derive(&path);
        let view = MultisigAccountXpub::new(
            derived.to_xpub(),
            derived.master_fingerprint,
            path,
            script_type,
            self.network,
        );
        Ok(f(derived.secret_key(), &view))
    }

    /// The non-secret account view for `script_type` and `account`: the
    /// account xpub plus everything needed to build addresses and a
    /// descriptor. `account` must be below 2³¹.
    pub fn account_xpub(
        &self,
        script_type: ScriptType,
        account: u32,
    ) -> Result<AccountXpub, Error> {
        let path = self.account_path(script_type, account)?;
        let derived = self.derive(&path);
        Ok(AccountXpub::new(
            derived.to_xpub(),
            derived.master_fingerprint,
            path,
            script_type,
            self.network,
        ))
    }
}

/// A private key derived from a [`MasterKey`], with its path and the
/// master fingerprint so a signer can state its key origin.
///
/// Same rules as [`MasterKey`]: no `Debug`, `Display` or `Clone`; erased
/// on drop.
pub struct DerivedKey {
    xpriv: Xpriv,
    master_fingerprint: Fingerprint,
    path: DerivationPath,
    /// The master's context, blinded as it was when this key was
    /// derived.
    secp: Secp256k1<All>,
}

impl DerivedKey {
    /// The signing context this key's operations run in, blinded like
    /// the master's.
    pub fn secp(&self) -> &Secp256k1<All> {
        &self.secp
    }

    /// The path this key was derived at, relative to the master.
    pub fn path(&self) -> &DerivationPath {
        &self.path
    }

    /// Fingerprint of the master key this was derived from.
    pub fn master_fingerprint(&self) -> Fingerprint {
        self.master_fingerprint
    }

    /// This key's own fingerprint.
    pub fn fingerprint(&self) -> Fingerprint {
        self.xpriv.fingerprint(&self.secp).into()
    }

    /// The matching extended public key.
    pub fn to_xpub(&self) -> Xpub {
        Xpub::from_priv(&self.secp, &self.xpriv)
    }

    /// This key as text (`xprv…`/`tprv…`), for a hold-to-reveal row. The
    /// buffer zeroizes on drop.
    pub fn xpriv_ascii(&self) -> XprivAscii {
        XprivAscii::of(&self.xpriv)
    }

    /// The raw private key.
    ///
    /// This is the only plaintext escape from the key types; signing uses
    /// it. Do not copy it into anything that outlives this borrow.
    pub fn secret_key(&self) -> &SecretKey {
        &self.xpriv.private_key
    }
}

impl Drop for DerivedKey {
    fn drop(&mut self) {
        erase(&mut self.xpriv);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bip39::{Language, Mnemonic};

    #[test]
    fn xpriv_text_matches_rust_bitcoin_and_zeroizes() {
        let m = Mnemonic::from_entropy(Language::English, &[0u8; 16]).unwrap();
        let master = MasterKey::from_seed(&m.to_seed(b"").unwrap(), Network::Mainnet);
        let text = master.xpriv_ascii();
        assert!(text.as_str().starts_with("xprv9s21ZrQH143K"));
        assert_eq!(text.as_str().len(), 111);
        let path: DerivationPath = "m/84h/0h/0h".parse().unwrap();
        let child = master.derive(&path).xpriv_ascii();
        assert!(child.as_str().starts_with("xprv9"));
        assert_ne!(child.as_str(), text.as_str());
        let mut copy = XprivAscii::of(master.xpriv());
        assert_eq!(copy.as_str(), text.as_str());
        drop(text);
        // A buffer that cannot hold the text fails cleanly.
        copy.len = XPRIV_ASCII_MAX as u8;
        assert!(core::fmt::Write::write_str(&mut copy, "x").is_err());
    }
}
