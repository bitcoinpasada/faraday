//! What script an input spends, from the PSBT's UTXO and script fields,
//! with the BIP-174 signer consistency checks.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt;

use bitcoin::hashes::hash160;
use bitcoin::opcodes::all::{OP_CHECKMULTISIG, OP_PUSHNUM_1, OP_PUSHNUM_16};
use bitcoin::psbt::Input;
use bitcoin::script::Instruction;
use bitcoin::{PublicKey, Script, TxOut};
use miniscript::{
    ExtParams, Miniscript, MiniscriptKey, ScriptContext, Segwitv0, SigType, Terminal, ToPublicKey,
};

/// The script an input spends, as far as this crate understands it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptKind {
    /// BIP-44 pay-to-pubkey-hash.
    P2pkh,
    /// BIP-49 p2wpkh nested in p2sh.
    P2shP2wpkh,
    /// BIP-84 native p2wpkh.
    P2wpkh,
    /// BIP-86 taproot key path (no script tree data in the input).
    P2trKey,
    /// `multi`/`sortedmulti` with `m` of `n` keys, in the given wrapper.
    Multisig {
        /// How the script is wrapped.
        wrapper: Wrapper,
        /// Signatures required.
        m: u8,
        /// Keys in the script.
        n: u8,
    },
    /// Taproot with script-path data (merkle root, leaf scripts or leaf
    /// hashes in key origins).
    P2trScript,
    /// A `wsh` or `sh(wsh)` script that parses as a miniscript and is
    /// not a plain `multi`: the Liana, Sparrow and Nunchuk recovery
    /// wallets.
    Miniscript {
        /// How the script is wrapped. `sh` alone is never this: a bare
        /// legacy miniscript is [`ScriptKind::Unknown`].
        wrapper: Wrapper,
    },
    /// Anything else: bare scripts, unknown witness versions, p2sh with a
    /// redeem script this crate does not recognise.
    Unknown,
}

impl ScriptKind {
    /// Whether the spend is ECDSA (legacy or SegWit v0) rather than
    /// Schnorr.
    pub fn is_ecdsa(self) -> bool {
        matches!(
            self,
            ScriptKind::P2pkh
                | ScriptKind::P2shP2wpkh
                | ScriptKind::P2wpkh
                | ScriptKind::Multisig { .. }
                | ScriptKind::Miniscript { .. }
        )
    }

    /// Whether this crate can sign and finalize the spend.
    pub fn is_supported(self) -> bool {
        !matches!(self, ScriptKind::Unknown)
    }

    /// Estimated weight units one input of this kind adds to a transaction
    /// once signed: the 41-byte outpoint/sequence skeleton at 4 wu per
    /// byte, plus scriptSig bytes at 4 wu and witness bytes at 1 wu.
    /// Signatures are counted at 72 bytes (DER + sighash byte) for ECDSA
    /// and 65 bytes for Schnorr; `Unknown` and `P2trScript` inputs get
    /// the p2wpkh figure so a fee rate is still shown.
    ///
    /// A miniscript input's own satisfaction is measured from the script
    /// and used instead of this figure wherever the script is at hand;
    /// the number here is one signature and a small script, which is the
    /// shape of the common case.
    pub fn estimated_weight(self) -> usize {
        const SKELETON: usize = 41 * 4;
        match self {
            ScriptKind::P2pkh => SKELETON + (1 + 72 + 1 + 33) * 4,
            ScriptKind::P2shP2wpkh => SKELETON + 23 * 4 + (1 + 1 + 72 + 1 + 33),
            ScriptKind::P2wpkh | ScriptKind::Unknown | ScriptKind::P2trScript => {
                SKELETON + (1 + 1 + 72 + 1 + 33)
            }
            ScriptKind::P2trKey => SKELETON + (1 + 1 + 65),
            ScriptKind::Miniscript { wrapper } => {
                let witness = 1 + (1 + 72) + (1 + 64);
                match wrapper {
                    Wrapper::P2sh => SKELETON + (1 + 72 + 1 + 64) * 4,
                    Wrapper::P2wsh => SKELETON + witness,
                    Wrapper::P2shP2wsh => SKELETON + 35 * 4 + witness,
                }
            }
            ScriptKind::Multisig { wrapper, m, n } => {
                let script = 1 + usize::from(n) * 34 + 1 + 1;
                let sigs = usize::from(m) * (1 + 72);
                match wrapper {
                    Wrapper::P2sh => SKELETON + (1 + sigs + 3 + script) * 4,
                    Wrapper::P2wsh => SKELETON + (1 + 1 + sigs + 1 + script),
                    Wrapper::P2shP2wsh => SKELETON + 35 * 4 + (1 + 1 + sigs + 1 + script),
                }
            }
        }
    }
}

impl fmt::Display for ScriptKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptKind::P2pkh => f.write_str("p2pkh"),
            ScriptKind::P2shP2wpkh => f.write_str("p2sh-p2wpkh"),
            ScriptKind::P2wpkh => f.write_str("p2wpkh"),
            ScriptKind::P2trKey => f.write_str("p2tr"),
            ScriptKind::P2trScript => f.write_str("p2tr script path"),
            ScriptKind::Miniscript { wrapper } => write!(f, "{wrapper} miniscript"),
            ScriptKind::Multisig { wrapper, m, n } => write!(f, "{wrapper} {m}-of-{n}"),
            ScriptKind::Unknown => f.write_str("unknown script"),
        }
    }
}

/// How a multisig script is wrapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Wrapper {
    /// Legacy p2sh with the multisig as redeem script.
    P2sh,
    /// Native p2wsh with the multisig as witness script.
    P2wsh,
    /// p2wsh nested in p2sh.
    P2shP2wsh,
}

impl fmt::Display for Wrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Wrapper::P2sh => f.write_str("p2sh"),
            Wrapper::P2wsh => f.write_str("p2wsh"),
            Wrapper::P2shP2wsh => f.write_str("p2sh-p2wsh"),
        }
    }
}

/// A parsed `OP_m <key>… OP_n OP_CHECKMULTISIG` script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Multisig {
    /// Signatures required.
    pub m: usize,
    /// Keys in script order.
    pub keys: Vec<PublicKey>,
}

impl Multisig {
    /// Parses a standard multisig script: `m` and `n` as `OP_1..OP_16`,
    /// compressed or uncompressed keys, `1 <= m <= n`.
    pub fn parse(script: &Script) -> Option<Self> {
        let mut m = None;
        let mut keys = Vec::new();
        let mut n = None;
        let mut done = false;
        for ins in script.instructions() {
            if done {
                return None;
            }
            match ins.ok()? {
                Instruction::Op(op) if m.is_none() => m = Some(small_int(op)?),
                Instruction::PushBytes(bytes) if n.is_none() => {
                    keys.push(PublicKey::from_slice(bytes.as_bytes()).ok()?);
                }
                Instruction::Op(op) if n.is_none() => n = Some(small_int(op)?),
                Instruction::Op(op) if op == OP_CHECKMULTISIG => done = true,
                _ => return None,
            }
        }
        let (m, n) = (m?, n?);
        if !done || m == 0 || m > n || keys.len() != n {
            return None;
        }
        Some(Self { m, keys })
    }

    /// Keys in the script.
    pub fn n(&self) -> usize {
        self.keys.len()
    }

    /// Whether the keys are in lexicographic order, as `sortedmulti`
    /// produces.
    pub fn is_sorted(&self) -> bool {
        self.keys
            .windows(2)
            .all(|w| w[0].to_bytes() <= w[1].to_bytes())
    }
}

fn small_int(op: bitcoin::Opcode) -> Option<usize> {
    let b = op.to_u8();
    if (OP_PUSHNUM_1.to_u8()..=OP_PUSHNUM_16.to_u8()).contains(&b) {
        Some(usize::from(b - OP_PUSHNUM_1.to_u8() + 1))
    } else {
        None
    }
}

/// An input classified from its PSBT fields.
pub(crate) struct Classified {
    pub kind: ScriptKind,
    /// The output being spent, when the PSBT carries it.
    pub utxo: Option<TxOut>,
    pub multisig: Option<Multisig>,
    /// The keys a [`ScriptKind::Miniscript`] witness script names, read
    /// through the miniscript rather than by searching the bytes. A
    /// `pkh` fragment carries only a hash in the script, so the key
    /// behind it is taken from the input's own `bip32_derivation`.
    pub script_keys: Vec<PublicKey>,
    /// Witness weight one satisfaction of a [`ScriptKind::Miniscript`]
    /// witness script takes, script included.
    pub satisfaction_weight: Option<usize>,
    /// BIP-174 signer checks that failed. Any entry makes the input
    /// untrustworthy: amounts or scripts may be lies.
    pub problems: Vec<&'static str>,
    /// The input spends SegWit v0 and carries no `non_witness_utxo`, so
    /// its amount is stated by the coordinator and committed to by
    /// nothing, and the transaction has more than one input. The data is
    /// consistent as far as it goes, which is why this is not a
    /// [`Classified::problems`] entry.
    ///
    /// The input count is part of the rule because the attack it stops
    /// needs two signatures the chain accepts, one per round, over two
    /// different lies about the total spent. BIP-143 commits each
    /// signature to its own input's stated amount, so a lone input
    /// signed under a lie yields a signature no node accepts and nothing
    /// to combine it with. The count is the transaction's rather than
    /// the number of inputs that name our keys: a coordinator can hide
    /// the key origin of the other input in each round and still combine
    /// the two signatures afterwards.
    pub amount_unverified: bool,
}

/// Classifies input `index` of `psbt`.
pub(crate) fn classify(psbt: &bitcoin::Psbt, index: usize) -> Classified {
    let input = &psbt.inputs[index];
    let prevout = psbt.unsigned_tx.input[index].previous_output;
    let mut problems = Vec::new();

    let non_witness = input.non_witness_utxo.as_ref().and_then(|tx| {
        if tx.compute_txid() != prevout.txid {
            problems.push("non_witness_utxo is not the transaction the input spends");
            return None;
        }
        let out = tx.output.get(prevout.vout as usize);
        if out.is_none() {
            problems.push("non_witness_utxo has no output at the spent index");
        }
        out
    });
    let utxo = match (input.witness_utxo.as_ref(), non_witness) {
        (Some(w), Some(nw)) => {
            if w != nw {
                problems.push("witness_utxo and non_witness_utxo disagree");
            }
            Some(nw.clone())
        }
        (Some(w), None) => Some(w.clone()),
        (None, Some(nw)) => Some(nw.clone()),
        (None, None) => None,
    };

    let Some(utxo) = utxo else {
        return Classified {
            kind: ScriptKind::Unknown,
            utxo: None,
            multisig: None,
            script_keys: Vec::new(),
            satisfaction_weight: None,
            problems,
            amount_unverified: false,
        };
    };
    let spk = utxo.script_pubkey.as_script();
    let finalized = input.final_script_sig.is_some() || input.final_script_witness.is_some();
    if finalized {
        // The finalizer clears the redeem/witness scripts, so only what the
        // scriptPubKey itself says is known.
        let kind = if spk.is_p2pkh() {
            ScriptKind::P2pkh
        } else if spk.is_p2wpkh() {
            ScriptKind::P2wpkh
        } else if spk.is_p2tr() {
            ScriptKind::P2trKey
        } else {
            ScriptKind::Unknown
        };
        return Classified {
            kind,
            utxo: Some(utxo),
            multisig: None,
            script_keys: Vec::new(),
            satisfaction_weight: None,
            problems,
            amount_unverified: false,
        };
    }
    let mut multisig = None;
    let kind = if spk.is_p2pkh() {
        if input.non_witness_utxo.is_none() {
            problems.push("witness_utxo given for a non-witness input");
        }
        ScriptKind::P2pkh
    } else if spk.is_p2wpkh() {
        ScriptKind::P2wpkh
    } else if spk.is_p2wsh() {
        match witness_script(input, spk, &mut problems) {
            WitnessScript::Multi(ms) => {
                let kind = ScriptKind::Multisig {
                    wrapper: Wrapper::P2wsh,
                    m: ms.m as u8,
                    n: ms.n() as u8,
                };
                multisig = Some(ms);
                kind
            }
            WitnessScript::Miniscript => ScriptKind::Miniscript {
                wrapper: Wrapper::P2wsh,
            },
            WitnessScript::Neither => ScriptKind::Unknown,
        }
    } else if spk.is_p2sh() {
        match &input.redeem_script {
            None => {
                problems.push("p2sh input without redeem_script");
                ScriptKind::Unknown
            }
            Some(redeem) => {
                if redeem.to_p2sh() != utxo.script_pubkey {
                    problems.push("redeem_script does not match the scriptPubKey");
                    ScriptKind::Unknown
                } else if redeem.is_p2wpkh() {
                    ScriptKind::P2shP2wpkh
                } else if redeem.is_p2wsh() {
                    match witness_script(input, redeem, &mut problems) {
                        WitnessScript::Multi(ms) => {
                            let kind = ScriptKind::Multisig {
                                wrapper: Wrapper::P2shP2wsh,
                                m: ms.m as u8,
                                n: ms.n() as u8,
                            };
                            multisig = Some(ms);
                            kind
                        }
                        WitnessScript::Miniscript => ScriptKind::Miniscript {
                            wrapper: Wrapper::P2shP2wsh,
                        },
                        WitnessScript::Neither => ScriptKind::Unknown,
                    }
                } else if let Some(ms) = Multisig::parse(redeem) {
                    if input.non_witness_utxo.is_none() {
                        problems.push("witness_utxo given for a non-witness input");
                    }
                    let kind = ScriptKind::Multisig {
                        wrapper: Wrapper::P2sh,
                        m: ms.m as u8,
                        n: ms.n() as u8,
                    };
                    multisig = Some(ms);
                    kind
                } else {
                    ScriptKind::Unknown
                }
            }
        }
    } else if spk.is_p2tr() {
        let script_path = input.tap_merkle_root.is_some()
            || !input.tap_scripts.is_empty()
            || !input.tap_script_sigs.is_empty()
            || input
                .tap_key_origins
                .values()
                .any(|(leaves, _)| !leaves.is_empty());
        if script_path {
            ScriptKind::P2trScript
        } else {
            ScriptKind::P2trKey
        }
    } else {
        ScriptKind::Unknown
    };

    // BIP-143 signs the amount an input states rather than the
    // transaction it came from, so a SegWit v0 amount is worth only the
    // previous transaction the PSBT carries beside it. Taproot commits to
    // every amount it spends and needs no such transaction. One input
    // alone cannot be lied to profitably: the single signature is made
    // over the lie and no node accepts it.
    let amount_unverified = psbt.inputs.len() > 1
        && input.non_witness_utxo.is_none()
        && matches!(
            kind,
            ScriptKind::P2wpkh
                | ScriptKind::P2shP2wpkh
                | ScriptKind::Multisig {
                    wrapper: Wrapper::P2wsh | Wrapper::P2shP2wsh,
                    ..
                }
                | ScriptKind::Miniscript { .. }
        );

    let (script_keys, satisfaction_weight) = match kind {
        ScriptKind::Miniscript { wrapper } => {
            let ws = input.witness_script.as_ref().expect("classified as p2wsh");
            let script_sig = match wrapper {
                Wrapper::P2shP2wsh => {
                    let redeem = input
                        .redeem_script
                        .as_ref()
                        .expect("classified as p2sh-p2wsh");
                    (redeem.len() + 1) * 4
                }
                _ => 0,
            };
            match read_miniscript(input, ws) {
                Some((keys, witness)) => (keys, Some(41 * 4 + script_sig + witness)),
                None => (Vec::new(), None),
            }
        }
        _ => (Vec::new(), None),
    };

    Classified {
        kind,
        utxo: Some(utxo),
        multisig,
        script_keys,
        satisfaction_weight,
        problems,
        amount_unverified,
    }
}

/// What a p2wsh input's witness script turned out to be.
enum WitnessScript {
    /// `OP_m <key>… OP_n OP_CHECKMULTISIG`.
    Multi(Multisig),
    /// A script `miniscript` reads, which is every wallet script that is
    /// not a plain multisig.
    Miniscript,
    /// Missing, mismatched, or neither of the two.
    Neither,
}

/// The input's witness script, checked against the p2wsh program in
/// `program_holder` (the scriptPubKey or the redeem script).
fn witness_script(
    input: &Input,
    program_holder: &Script,
    problems: &mut Vec<&'static str>,
) -> WitnessScript {
    let Some(ws) = &input.witness_script else {
        problems.push("p2wsh input without witness_script");
        return WitnessScript::Neither;
    };
    if ws.to_p2wsh().as_script() != program_holder {
        problems.push("witness_script does not match the witness program");
        return WitnessScript::Neither;
    }
    match Multisig::parse(ws) {
        Some(ms) => WitnessScript::Multi(ms),
        None if read_miniscript(input, ws).is_some() => WitnessScript::Miniscript,
        None => WitnessScript::Neither,
    }
}

/// Every key a parsed miniscript checks a signature against, in the
/// order the script writes them.
///
/// A `pkh` fragment read back from a script carries only a hash
/// (`miniscript` calls that a raw `pkh`); `by_hash` is where the key
/// behind such a hash comes from, and one the map does not hold is left
/// out, since the script does not name it.
pub(crate) fn script_key_set<Pk: MiniscriptKey, Ctx: ScriptContext>(
    ms: &Miniscript<Pk, Ctx>,
    by_hash: &BTreeMap<hash160::Hash, Pk>,
) -> Vec<Pk> {
    let mut keys = Vec::new();
    for node in ms.iter() {
        match &node.node {
            Terminal::PkK(pk) | Terminal::PkH(pk) => keys.push(pk.clone()),
            Terminal::RawPkH(hash) => {
                if let Some(pk) = by_hash.get(hash) {
                    keys.push(pk.clone());
                }
            }
            Terminal::Multi(thresh) => keys.extend(thresh.data().iter().cloned()),
            Terminal::MultiA(thresh) => keys.extend(thresh.data().iter().cloned()),
            _ => {}
        }
    }
    keys
}

/// The keys a SegWit v0 witness script names and the witness weight one
/// satisfaction of it takes.
///
/// The script is read as a miniscript, so the keys are the ones the
/// script actually checks signatures against and not every 33-byte push
/// in it. A `pkh` fragment leaves only a hash behind in the script;
/// `miniscript` calls that a raw `pkh`, and the key it stands for comes
/// from the input's own `bip32_derivation`, which is where a coordinator
/// states it.
pub(crate) fn read_miniscript(input: &Input, ws: &Script) -> Option<(Vec<PublicKey>, usize)> {
    let ext = ExtParams {
        raw_pkh: true,
        ..ExtParams::sane()
    };
    let ms = Miniscript::<PublicKey, Segwitv0>::parse_with_ext(ws, &ext).ok()?;
    let mut hashes = BTreeMap::new();
    for pk in input.bip32_derivation.keys() {
        let pk = PublicKey::new(*pk);
        hashes.insert(pk.to_pubkeyhash(SigType::Ecdsa), pk);
    }
    let keys = script_key_set(&ms, &hashes);
    if keys.is_empty() {
        return None;
    }
    // The satisfaction, the push of the script itself, and the count of
    // witness elements, all at one weight unit per byte.
    let size = ms.max_satisfaction_size().ok()?;
    Some((keys, size + 1 + ws.len() + 1))
}
