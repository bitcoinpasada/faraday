//! BIP 387's tapscript multisig: `multi_a` and `sortedmulti_a` inside a
//! `tr()`.
//!
//! This is the taproot multisig Sparrow builds and Nunchuk and Coldcard
//! sign: an unspendable internal key — BIP 341's `H`, the NUMS point —
//! and one leaf holding `k` of `n` keys, so that the only way to spend
//! is the script path.
//!
//! `miniscript` reads `multi_a` but has no `sortedmulti_a`, so the
//! sorted form is read here: the descriptor is parsed as its `multi_a`
//! twin, and the script is rebuilt at every derivation index with the
//! x-only keys sorted, which is what BIP 387 says the sorted form means
//! — "on the keys that are to be put into the output script, i.e. after
//! all extended keys are derived".

use alloc::string::String;
use alloc::vec::Vec;

use bitcoin::opcodes::all::{OP_CHECKSIG, OP_CHECKSIGADD, OP_NUMEQUAL};
use bitcoin::script::Builder;
use bitcoin::secp256k1::{Secp256k1, Verification, XOnlyPublicKey};
use bitcoin::taproot::TaprootBuilder;
use bitcoin::{PublicKey, ScriptBuf};

/// BIP 341's `H`: the x-only point with no known discrete logarithm,
/// which every wallet that wants a script-only taproot output writes as
/// its internal key.
pub const NUMS: [u8; 32] = [
    0x50, 0x92, 0x9b, 0x74, 0xc1, 0xa0, 0x49, 0x54, 0xb7, 0x8b, 0x4b, 0x60, 0x35, 0xe9, 0x7a, 0x5e,
    0x07, 0x8a, 0x5a, 0x0f, 0x28, 0xec, 0x96, 0xd5, 0x47, 0xbf, 0xee, 0x9a, 0xce, 0x80, 0x3a, 0xc0,
];

/// Whether an internal key is the NUMS point, so that the wallet's coins
/// can be spent only through the tree.
pub fn is_nums(key: &XOnlyPublicKey) -> bool {
    key.serialize() == NUMS
}

/// The tapscript BIP 387 writes for a threshold over these keys, in the
/// order given: `KEY_1 OP_CHECKSIG KEY_2 OP_CHECKSIGADD … k
/// OP_NUMEQUAL`.
pub fn multi_a_script(threshold: usize, keys: &[XOnlyPublicKey]) -> ScriptBuf {
    let mut builder = Builder::new();
    for (i, key) in keys.iter().enumerate() {
        builder = builder.push_x_only_key(key);
        builder = builder.push_opcode(if i == 0 { OP_CHECKSIG } else { OP_CHECKSIGADD });
    }
    builder
        .push_int(threshold as i64)
        .push_opcode(OP_NUMEQUAL)
        .into_script()
}

/// The same script with the keys sorted lexicographically, which is what
/// `sortedmulti_a` pays to.
pub fn sorted_multi_a_script(threshold: usize, keys: &[XOnlyPublicKey]) -> ScriptBuf {
    let mut keys: Vec<XOnlyPublicKey> = keys.to_vec();
    keys.sort_by_key(XOnlyPublicKey::serialize);
    multi_a_script(threshold, &keys)
}

/// The output script of `tr(internal, <leaf>)`: the internal key tweaked
/// by the single leaf's Merkle root.
pub fn output_script<C: Verification>(
    secp: &Secp256k1<C>,
    internal: XOnlyPublicKey,
    leaf: &ScriptBuf,
) -> Option<ScriptBuf> {
    let info = TaprootBuilder::new()
        .add_leaf(0, leaf.clone())
        .ok()?
        .finalize(secp, internal)
        .ok()?;
    Some(ScriptBuf::new_p2tr_tweaked(info.output_key()))
}

/// `sortedmulti_a` written as the `multi_a` `miniscript` can read, when
/// the text is a `tr()` whose tree is one `sortedmulti_a` leaf.
///
/// Nothing else is rewritten: a `sortedmulti_a` deeper in a tree would
/// need the whole tree rebuilt at every index, and no wallet in the
/// field writes one.
pub(crate) fn unsorted_text(text: &str) -> Option<String> {
    let inner = text.trim().strip_prefix("tr(")?.strip_suffix(')')?;
    let (_, tree) = inner.split_once(',')?;
    let tree = tree.trim();
    if !tree.starts_with("sortedmulti_a(") || !tree.ends_with(')') {
        return None;
    }
    Some(text.trim().replacen("sortedmulti_a(", "multi_a(", 1))
}

/// The internal key and the x-only keys of a derived one-leaf
/// `multi_a`, in the order the script writes them.
pub fn derived_keys(
    script: &miniscript::Descriptor<PublicKey>,
) -> Option<(XOnlyPublicKey, Vec<XOnlyPublicKey>)> {
    let miniscript::Descriptor::Tr(tr) = script else {
        return None;
    };
    let internal = tr.internal_key().inner.x_only_public_key().0;
    let mut leaves = tr.iter_scripts();
    let (_, leaf) = leaves.next()?;
    if leaves.next().is_some() {
        return None;
    }
    let keys: Vec<XOnlyPublicKey> = leaf
        .iter_pk()
        .map(|k| k.inner.x_only_public_key().0)
        .collect();
    Some((internal, keys))
}
