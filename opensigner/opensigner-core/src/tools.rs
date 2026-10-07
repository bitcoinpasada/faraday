//! Tools › the standalone calculators (`docs/DESIGN.md` §5): hashes,
//! encodings, a descriptor checksum, a key conversion, the units, and
//! the miniscript compiler.
//!
//! Nothing here is a secret and nothing is stored. Each tool takes a
//! string or a number, works something out from it, and shows the
//! answer; no screen carries an eye, and leaving the tool drops what
//! was typed.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use osk_bip::descriptor;
use osk_bip::keys::{Network, ScriptType};
use osk_bip::miniscript::Segwitv0;
use osk_bip::miniscript::descriptor::Descriptor;
use osk_bip::miniscript::policy::{Concrete, Liftable};
use osk_bip::policy::WalletPolicy;
use osk_bip::slip132;
use osk_bip::spend::SpendPath;
use osk_bip::xkey;
use osk_codec::encodings::{self, Reading};
use osk_psbt::bitcoin::hashes::{Hash, ripemd160, sha256};
use osk_psbt::bitcoin::{Psbt, ScriptBuf, Txid, Witness};
use osk_psbt::verify::Comparison;
use osk_ui::components::Denomination;

/// Tools › Compare transactions: the two PSBTs read one after the
/// other, and what differs between them (`docs/PLANNING.md` §16.111
/// rule 4).
///
/// Nothing here is secret and nothing is kept: leaving the tool drops
/// the first transaction and the comparison with it.
#[derive(Debug, Default)]
pub struct CompareTransactions {
    first: Option<osk_psbt::Psbt>,
    comparison: Option<Comparison>,
}

impl CompareTransactions {
    /// An empty tool, waiting for the first transaction.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the first transaction has been read.
    pub fn has_first(&self) -> bool {
        self.first.is_some()
    }

    /// The id of the first transaction, which is what the row that
    /// names it shows.
    pub fn first_txid(&self) -> Option<Txid> {
        Some(self.first.as_ref()?.unsigned_tx().compute_txid())
    }

    /// Takes `psbt` as the first transaction, dropping whatever was
    /// there.
    pub fn set_first(&mut self, psbt: osk_psbt::Psbt) {
        self.first = Some(psbt);
        self.comparison = None;
    }

    /// Compares `psbt` with the first, which must have been read.
    pub fn compare_with(&mut self, psbt: &osk_psbt::Psbt) {
        if let Some(first) = self.first.as_ref() {
            self.comparison = Some(osk_psbt::verify::compare(first, psbt));
        }
    }

    /// What differs, once both have been read.
    pub fn comparison(&self) -> Option<&Comparison> {
        self.comparison.as_ref()
    }

    /// Drops both transactions and the comparison.
    pub fn clear(&mut self) {
        self.first = None;
        self.comparison = None;
    }
}

/// The calculators, in the order Tools lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    /// SHA-256, SHA-256d and HASH160 of what is typed.
    Hashes,
    /// What a string is encoded in and what it decodes to.
    Encodings,
    /// A descriptor's checksum, and whether the one it arrived with
    /// holds.
    Descriptor,
    /// Every spelling of one extended public key.
    ConvertKey,
    /// One amount in all four units at once.
    Units,
    /// A policy compiled into a descriptor.
    Miniscript,
}

impl Tool {
    /// The six, in the order Tools lists them.
    pub const ALL: [Tool; 6] = [
        Tool::Hashes,
        Tool::Encodings,
        Tool::Descriptor,
        Tool::ConvertKey,
        Tool::Units,
        Tool::Miniscript,
    ];
}

/// How the Hashes field is read (`docs/DESIGN.md` §4.2 mode row).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadAs {
    /// Hex when every character is a hex digit and there is an even
    /// number of them, UTF-8 text otherwise.
    Auto,
    /// The characters' own bytes, whatever they spell.
    Text,
    /// Hex digits, two per byte.
    Hex,
}

impl ReadAs {
    /// The three, in the order the Choice lists them.
    pub const ALL: [ReadAs; 3] = [ReadAs::Auto, ReadAs::Text, ReadAs::Hex];
}

/// How many characters a calculator's field takes. Long enough for a
/// multisig descriptor, which is the longest string any of them reads.
const MAX_CHARS: usize = 1024;

/// How many digits the Units field takes: 21 million bitcoin is fifteen
/// digits of satoshi.
const MAX_DIGITS: usize = 16;

/// The state of whichever calculator is open. One is open at a time, so
/// one field, one mode and one unit serve all five.
#[derive(Debug, Default)]
pub struct Calculator {
    typed: String,
    read_as: Option<ReadAs>,
    from: Denomination,
    script: PolicyScript,
}

/// What a compiled policy is wrapped in (`docs/DESIGN.md` §4.2 mode
/// row).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PolicyScript {
    /// `wsh(...)`: the policy compiles to one Segwit v0 script.
    #[default]
    Segwit,
    /// `tr(...)`: the compiler chooses an internal key and a tree.
    Taproot,
}

impl PolicyScript {
    /// The two, in the order the Choice lists them.
    pub const ALL: [PolicyScript; 2] = [PolicyScript::Segwit, PolicyScript::Taproot];
}

impl Calculator {
    /// An empty field, read automatically, counting satoshi.
    pub fn new() -> Self {
        Calculator {
            typed: String::new(),
            read_as: None,
            from: Denomination::Sat,
            script: PolicyScript::Segwit,
        }
    }

    /// What a compiled policy is wrapped in.
    pub fn script(&self) -> PolicyScript {
        self.script
    }

    /// Sets that, which leaves what is typed where it is: the same
    /// policy in the other wrapper is another descriptor, and the fact
    /// rows say what it is.
    pub fn set_script(&mut self, script: PolicyScript) {
        self.script = script;
    }

    /// What has been typed.
    pub fn typed(&self) -> &str {
        &self.typed
    }

    /// How the Hashes field is read.
    pub fn read_as(&self) -> ReadAs {
        self.read_as.unwrap_or(ReadAs::Auto)
    }

    /// Forces how the field is read.
    pub fn set_read_as(&mut self, read_as: ReadAs) {
        self.read_as = Some(read_as);
    }

    /// Which unit the Units field is typed in.
    pub fn from(&self) -> Denomination {
        self.from
    }

    /// Sets that unit, which leaves the digits where they are: the same
    /// number in another unit is another amount, and the fact rows say
    /// what it is.
    pub fn set_from(&mut self, from: Denomination) {
        self.from = from;
    }

    /// Empties the field.
    pub fn clear(&mut self) {
        self.typed.clear();
    }

    /// Puts `text` in the field, in place of whatever was there: what a
    /// scan or a paste hands the tool. The field's own limits still
    /// hold, so a payload longer than the tool takes is cut to it.
    pub fn set(&mut self, tool: Tool, text: String) {
        self.typed.clear();
        for c in text.chars() {
            if !self.push(tool, c) {
                break;
            }
        }
    }

    /// Types one character; ignored where the field is full or the tool
    /// takes no such character.
    pub fn push(&mut self, tool: Tool, c: char) -> bool {
        let max = if tool == Tool::Units {
            MAX_DIGITS
        } else {
            MAX_CHARS
        };
        if tool == Tool::Units && !c.is_ascii_digit() {
            return false;
        }
        if self.typed.chars().count() >= max {
            return false;
        }
        self.typed.push(c);
        true
    }

    /// Removes the last character.
    pub fn pop(&mut self) {
        self.typed.pop();
    }

    /// The amount the Units field names, in satoshi, capped at the
    /// twenty-one million that exist.
    pub fn sats(&self) -> u64 {
        let digits: u64 = self.typed.parse().unwrap_or(0);
        digits.saturating_mul(self.from.sat()).min(MAX_SATS)
    }

    /// Whether ✓ leads anywhere: the field says something this tool can
    /// work out an answer from.
    pub fn ready(&self, tool: Tool, network: Network, keys: &[(String, String)]) -> bool {
        match tool {
            Tool::Hashes => !self.typed.trim().is_empty(),
            Tool::Encodings => encodings::read(&self.typed).is_some(),
            Tool::Descriptor => checksum_facts(&self.typed).is_some(),
            Tool::ConvertKey => matches!(key_facts(&self.typed, network), KeyReading::Public(_)),
            // The Units screen is the answer on a class with the room
            // for it, and ✓ opens the same three units as a Record on
            // one without.
            Tool::Units => !self.typed.is_empty(),
            Tool::Miniscript => compile(&self.typed, self.script, keys).is_ok(),
        }
    }

    /// The bytes the Hashes field names under the mode it is read in.
    pub fn input(&self) -> Vec<u8> {
        read_input(self.typed.trim(), self.read_as())
    }
}

/// Every satoshi there will ever be.
pub const MAX_SATS: u64 = 2_100_000_000_000_000;

/// The bytes `text` names: its hex when the mode says hex (or says
/// nothing and the text is hex), its own UTF-8 bytes otherwise.
pub fn read_input(text: &str, read_as: ReadAs) -> Vec<u8> {
    match read_as {
        ReadAs::Text => text.as_bytes().to_vec(),
        ReadAs::Hex => encodings::from_hex(text).unwrap_or_default(),
        ReadAs::Auto => encodings::from_hex(text).unwrap_or_else(|| text.as_bytes().to_vec()),
    }
}

/// The three hashes of some bytes, and how many bytes they were.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hashes {
    /// How long the input was.
    pub len: usize,
    /// SHA-256.
    pub sha256: [u8; 32],
    /// SHA-256 of SHA-256.
    pub sha256d: [u8; 32],
    /// RIPEMD-160 of SHA-256, which Bitcoin calls HASH160.
    pub hash160: [u8; 20],
}

/// The three hashes of `bytes`.
pub fn hashes(bytes: &[u8]) -> Hashes {
    let once = osk_crypto::sha256(bytes);
    Hashes {
        len: bytes.len(),
        sha256: once,
        sha256d: osk_crypto::sha256(&once),
        hash160: ripemd160::Hash::hash(sha256::Hash::hash(bytes).as_byte_array()).to_byte_array(),
    }
}

/// What [`encodings::read`] makes of a string, for the Encodings tool.
pub fn encoding_of(text: &str) -> Option<Reading> {
    encodings::read(text)
}

/// The human-readable part the network setting spells its addresses
/// with, which is what the Encodings tool offers a hex string under.
pub fn hrp(network: Network) -> &'static str {
    match network {
        Network::Mainnet => "bc",
        _ => "tb",
    }
}

/// What the Descriptor checksum tool works out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChecksumFacts {
    /// The descriptor with the checksum this device computes.
    pub with_checksum: String,
    /// The checksum this device computed, on its own.
    pub checksum: String,
    /// The checksum that arrived with the text, and whether it holds.
    pub given: Option<(String, bool)>,
    /// The wallet the descriptor is, where it is one this device can
    /// load.
    pub wallet: Option<WalletPolicy>,
}

/// The checksum of `text`, or `None` when it is not a descriptor this
/// device can checksum at all.
pub fn checksum_facts(text: &str) -> Option<ChecksumFacts> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let (body, given) = match text.split_once('#') {
        Some((body, tail)) => (body, Some(String::from(tail))),
        None => (text, None),
    };
    let sum = descriptor::descriptor_checksum(body)?;
    let sum = String::from_utf8(sum.to_vec()).ok()?;
    let with_checksum = alloc::format!("{body}#{sum}");
    Some(ChecksumFacts {
        given: given.map(|g| {
            let holds = g == sum;
            (g, holds)
        }),
        wallet: WalletPolicy::parse_any(body).ok(),
        checksum: sum,
        with_checksum,
    })
}

/// What the miniscript compiler made of a policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyFacts {
    /// The descriptor, with its checksum.
    pub descriptor: String,
    /// The key tokens the descriptor names, in the order it writes them.
    pub keys: Vec<String>,
    /// The ways the compiled script can be spent.
    pub paths: Vec<SpendPath>,
    /// The wallet the descriptor is, when every key is an extended
    /// public key with an origin and both chains.
    pub wallet: Option<WalletPolicy>,
}

/// Compiles a concrete policy into a descriptor.
///
/// `keys` is the substitution a person gets for free: each pair is a
/// loaded key's fingerprint and the key expression it stands for, so a
/// policy can be typed as `pk(73c5da0a)` rather than as an xpub. A
/// token that is neither a listed fingerprint nor something
/// `miniscript` reads as a key is the compiler's to refuse.
///
/// The error is the compiler's own message on one line, which is what
/// the caption row shows.
pub fn compile(
    text: &str,
    script: PolicyScript,
    keys: &[(String, String)],
) -> Result<PolicyFacts, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err(String::new());
    }
    let mut filled = String::from(text);
    for (fingerprint, expression) in keys {
        filled = filled.replace(fingerprint.as_str(), expression);
    }
    let policy: Concrete<String> = filled.parse().map_err(one_line)?;
    let descriptor = match script {
        PolicyScript::Segwit => policy
            .compile::<Segwitv0>()
            .map_err(one_line)
            .and_then(|ms| Descriptor::new_wsh(ms).map_err(one_line))?,
        // `compile_tr` with no unspendable key takes the internal key
        // out of the policy itself, which is what a person typing
        // `or(pk(A),...)` means by the first branch.
        PolicyScript::Taproot => policy.compile_tr(None).map_err(one_line)?,
    };
    let text = alloc::format!("{descriptor}");
    let paths = descriptor
        .lift()
        .ok()
        .and_then(|semantic| {
            let keys = policy_keys(&descriptor, &text);
            osk_bip::spend::spend_paths(&semantic, &keys)
        })
        .unwrap_or_default();
    Ok(PolicyFacts {
        keys: policy_keys(&descriptor, &text),
        wallet: WalletPolicy::parse_any(&text).ok(),
        paths,
        descriptor: text,
    })
}

/// The key tokens a compiled descriptor names, in the order its text
/// writes them. `Tr` visits its leaves before its internal key, so the
/// position in the text is what orders them for a reader.
fn policy_keys(descriptor: &Descriptor<String>, text: &str) -> Vec<String> {
    use osk_bip::miniscript::ForEachKey;
    let mut keys: Vec<String> = Vec::new();
    descriptor.for_each_key(|key| {
        if !keys.contains(key) {
            keys.push(key.clone());
        }
        true
    });
    keys.sort_by_key(|key| text.find(key.as_str()).unwrap_or(usize::MAX));
    keys
}

/// A compiler message as one line: what a caption row has room for.
fn one_line<E: core::fmt::Display>(error: E) -> String {
    let text = alloc::format!("{error}");
    let line = text.lines().next().unwrap_or("").trim();
    String::from(line)
}

/// What the Convert key tool made of the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyReading {
    /// An extended public key, in every spelling.
    Public(Box<KeyFacts>),
    /// An extended private key, which this tool does not convert.
    Private,
    /// Not an extended key at all.
    None,
}

/// Every spelling of one extended public key, and what it says about
/// itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyFacts {
    /// The BIP-32 form: `xpub` or `tpub`.
    pub bip32: String,
    /// The SLIP-132 form for each script type, in [`ScriptType::ALL`]
    /// order.
    pub slip132: Vec<(ScriptType, String)>,
    /// Which chain the version bytes name.
    pub network: Network,
    /// How far below the master the key is.
    pub depth: u8,
    /// The key's own fingerprint.
    pub fingerprint: String,
    /// The child number the key was derived at.
    pub child: String,
}

/// Reads `text` as an extended key and writes it every other way.
pub fn key_facts(text: &str, network: Network) -> KeyReading {
    let text = text.trim();
    if text.is_empty() {
        return KeyReading::None;
    }
    if xkey::decode_xpriv(text).is_ok() {
        return KeyReading::Private;
    }
    let Ok((xpub, _)) = slip132::decode_xpub(text) else {
        return KeyReading::None;
    };
    let on = match xpub.network {
        osk_psbt::bitcoin::NetworkKind::Main => Network::Mainnet,
        // The four test networks share one set of version bytes, so the
        // key cannot say which of them it is: the setting does.
        _ if network == Network::Mainnet => Network::Testnet,
        _ => network,
    };
    let child = xpub.child_number;
    KeyReading::Public(Box::new(KeyFacts {
        bip32: slip132::encode_xpub(&xpub, ScriptType::Legacy),
        slip132: ScriptType::ALL
            .iter()
            .map(|s| (*s, slip132::encode_xpub(&xpub, *s)))
            .collect(),
        network: on,
        depth: xpub.depth,
        fingerprint: crate::text::hex(&xpub.fingerprint().to_bytes()),
        child: alloc::format!("{child}"),
    }))
}

/// A raw transaction, in hex text or in bytes, wrapped in an unsigned
/// PSBT so that `osk_psbt`'s inspector and the Sign review screens read
/// it the way they read a PSBT.
///
/// The wrapper carries no signatures, because a PSBT's unsigned
/// transaction may carry none; the id that comes back is the id of the
/// transaction that arrived, signatures and all.
pub fn raw_transaction(bytes: &[u8]) -> Option<(Vec<u8>, Txid)> {
    let tx = osk_psbt::transaction::read_raw(bytes)?;
    let txid = tx.compute_txid();
    let mut unsigned = tx;
    for input in &mut unsigned.input {
        input.script_sig = ScriptBuf::new();
        input.witness = Witness::new();
    }
    let psbt = Psbt::from_unsigned_tx(unsigned).ok()?;
    Some((psbt.serialize(), txid))
}
