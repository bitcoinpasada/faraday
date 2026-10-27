//! A wallet of several keys registered from the PSBT that spends it.
//!
//! An input spent from a script of more than one key is signed only for
//! a registered wallet ([`crate::WarningKind::WalletNotRegistered`]).
//! A PSBT that carries every cosigner's extended key with its origin
//! (BIP 174's `PSBT_GLOBAL_XPUB`), and whose blocked inputs are one
//! `multi` or `sortedmulti` shape in one wrapper, defines that wallet
//! itself: [`Registration`] is what it says, and [`register`] builds the
//! descriptor and checks that it gives every such input's script, and
//! every change output's that claims it, and that one of its keys is the
//! person's, before anything is added.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use bitcoin::bip32::{ChildNumber, DerivationPath, Xpub};
use bitcoin::hashes::hash160;
use bitcoin::psbt::Input;
use bitcoin::secp256k1::{Secp256k1, Verification, XOnlyPublicKey};
use bitcoin::taproot::{TapLeafHash, TapNodeHash};
use bitcoin::{Script, ScriptBuf};
use faraday_bip::keys::Fingerprint;
use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::{Compute, Work};
use miniscript::{
    ExtParams, Miniscript, MiniscriptKey, ScriptContext, Segwitv0, SigType, Tap, Terminal,
    ToPublicKey,
};

use crate::Psbt;
use crate::classify::{ScriptKind, Wrapper, classify};

/// One cosigner of the wallet a PSBT defines, in the order the
/// descriptor lists them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredKey {
    /// The master fingerprint the global xpub's origin names.
    pub fingerprint: Fingerprint,
    /// The path from that master down to the extended key.
    pub path: DerivationPath,
    /// The extended key itself.
    pub xpub: Xpub,
    /// Whether a loaded key, derived at the path the first input states
    /// for this cosigner, is the cosigner's key there.
    pub ours: bool,
}

/// One way to read the wallet's keys: sorted (`sortedmulti`) or in the
/// order the cosigners are listed (`multi`), and the descriptor that
/// reading writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading {
    /// `sortedmulti`.
    pub sorted: bool,
    /// The descriptor, `wsh(sortedmulti(2,[fp/path]xpub/<0;1>/*,…))`.
    pub descriptor: String,
}

/// The wallet a PSBT defines: its shape, its cosigners and the
/// descriptors written from them. Worked out once with the inspection,
/// so the screens read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registration {
    /// Signatures required.
    pub threshold: usize,
    /// How the script is wrapped.
    pub wrapper: Wrapper,
    /// The cosigners in descriptor order.
    pub cosigners: Vec<RegisteredKey>,
    /// The readings the spend leaves, `sortedmulti` first: one, or both
    /// where the keys are sorted at every address the spend names and
    /// both give every one of its scripts, which only the person can
    /// tell apart.
    pub readings: Vec<Reading>,
}

/// Why the wallet a PSBT defines was not registered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterRefusal {
    /// The descriptor written from the global xpubs does not read as a
    /// wallet.
    Descriptor,
    /// The wallet does not give this input's script at the chain and
    /// index the input states.
    Input(usize),
    /// This output claims the wallet's chain and index and is not the
    /// wallet's script there.
    Output(usize),
    /// No key of the wallet is one of this device's: no cosigner's
    /// extended key is what a loaded key gives at that cosigner's path.
    NotYours,
    /// This cosigner is under a loaded key's fingerprint and is not what
    /// that key gives at its path: a key that claims to be the person's
    /// and is someone else's.
    NotYourKey(usize),
}

/// The keys a miniscript names, added to `keys` and `hashes`: each key,
/// and each `pkh` hash the script carries whose key `by_hash` does not
/// name, since that is a key too.
fn keys_of<Pk: MiniscriptKey + Ord, Ctx: ScriptContext>(
    ms: &Miniscript<Pk, Ctx>,
    by_hash: &BTreeMap<hash160::Hash, Pk>,
    keys: &mut BTreeSet<Pk>,
    hashes: &mut BTreeSet<hash160::Hash>,
) {
    for node in ms.iter() {
        match &node.node {
            Terminal::PkK(pk) | Terminal::PkH(pk) => {
                keys.insert(pk.clone());
            }
            Terminal::RawPkH(hash) => match by_hash.get(hash) {
                Some(pk) => {
                    keys.insert(pk.clone());
                }
                None => {
                    hashes.insert(*hash);
                }
            },
            Terminal::Multi(thresh) => keys.extend(thresh.data().iter().cloned()),
            Terminal::MultiA(thresh) => keys.extend(thresh.data().iter().cloned()),
            _ => {}
        }
    }
}

/// Costs a script derivation (the output key the tree's root tweaks
/// the internal key to). The internal key of a taproot input whose
/// carried leaves are its whole tree: every control block names the same
/// internal key and gives the same root, that key tweaked by that root
/// is the output spent, and every hash a control block lists beside a
/// leaf is a carried leaf or a node built from carried leaves. `None`
/// for a tree with a leaf left out, or with no leaf.
fn whole_tree<C: Verification>(
    c: &Compute,
    secp: &Secp256k1<C>,
    input: &Input,
    spk: Option<&Script>,
) -> Option<XOnlyPublicKey> {
    let spk = spk?;
    let mut built: BTreeSet<TapNodeHash> = BTreeSet::new();
    let mut beside: Vec<TapNodeHash> = Vec::new();
    let mut top: Option<(XOnlyPublicKey, TapNodeHash)> = None;
    for (control, (leaf, version)) in &input.tap_scripts {
        let mut node = TapNodeHash::from(TapLeafHash::from_script(leaf, *version));
        built.insert(node);
        for sibling in control.merkle_branch.iter() {
            beside.push(*sibling);
            node = TapNodeHash::from_node_hashes(node, *sibling);
            built.insert(node);
        }
        if *top.get_or_insert((control.internal_key, node)) != (control.internal_key, node) {
            return None;
        }
    }
    let (internal, root) = top?;
    if !beside.iter().all(|h| built.contains(h)) {
        return None;
    }
    c.count(Work::Address);
    (ScriptBuf::new_p2tr(secp, internal, Some(root)).as_script() == spk).then_some(internal)
}

/// Costs a miniscript parse of the witness script or of each carried
/// leaf, and for a taproot input what `whole_tree` costs. Whether input
/// `input` of kind `kind`, spending `spk`, spends a script of more than
/// one key: a `multi` of two or more; a miniscript naming two or more;
/// or a taproot script path that carries leaves which, with the internal
/// key, name two or more keys between them, any of which does not read
/// as a miniscript, or which are not the output's whole tree
/// ([`whole_tree`]). The internal key counts unless it is BIP 341's
/// unspendable point, the one unspendable key a PSBT can show to be
/// one. A `pkh` hash counts as its key where the input states the key,
/// otherwise as a key of its own. A taproot input that carries no leaf
/// spends by its key path, one key, whatever root it states.
pub(crate) fn several_keys<C: Verification>(
    c: &Compute,
    secp: &Secp256k1<C>,
    input: &Input,
    kind: ScriptKind,
    spk: Option<&Script>,
) -> bool {
    let ext = ExtParams {
        raw_pkh: true,
        ..ExtParams::sane()
    };
    match kind {
        ScriptKind::Multisig { n, .. } => n > 1,
        ScriptKind::Miniscript { .. } => input.witness_script.as_ref().is_none_or(|ws| {
            c.count(Work::DescriptorParse);
            let Ok(ms) = Miniscript::<bitcoin::PublicKey, Segwitv0>::parse_with_ext(ws, &ext)
            else {
                return true;
            };
            let mut by_hash = BTreeMap::new();
            for pk in input.bip32_derivation.keys() {
                let pk = bitcoin::PublicKey::new(*pk);
                by_hash.insert(pk.to_pubkeyhash(SigType::Ecdsa), pk);
            }
            let (mut keys, mut hashes) = (BTreeSet::new(), BTreeSet::new());
            keys_of(&ms, &by_hash, &mut keys, &mut hashes);
            keys.len() + hashes.len() > 1
        }),
        ScriptKind::P2trScript if input.tap_scripts.is_empty() => false,
        ScriptKind::P2trScript => {
            let Some(internal) = whole_tree(c, secp, input, spk) else {
                return true;
            };
            let mut by_hash = BTreeMap::new();
            for key in input.tap_key_origins.keys() {
                by_hash.insert(key.to_pubkeyhash(SigType::Schnorr), *key);
            }
            let (mut keys, mut hashes) = (BTreeSet::new(), BTreeSet::new());
            if !faraday_bip::tapmulti::is_nums(&internal) {
                keys.insert(internal);
            }
            for (leaf, _) in input.tap_scripts.values() {
                c.count(Work::DescriptorParse);
                match Miniscript::<XOnlyPublicKey, Tap>::parse_with_ext(leaf, &ext) {
                    Ok(ms) => keys_of(&ms, &by_hash, &mut keys, &mut hashes),
                    Err(_) => return true,
                }
            }
            keys.len() + hashes.len() > 1
        }
        _ => false,
    }
}

/// The global xpub whose origin is `path` less its last two steps, a
/// chain (0 or 1) and an unhardened index, under `fingerprint`. `None`
/// when no entry or more than one is, or when the key is at the master
/// itself, which names no account.
fn global_for<'a>(
    globals: &'a BTreeMap<Xpub, (bitcoin::bip32::Fingerprint, DerivationPath)>,
    fingerprint: Fingerprint,
    path: &DerivationPath,
) -> Option<(&'a Xpub, &'a DerivationPath)> {
    let steps: Vec<ChildNumber> = path.into_iter().copied().collect();
    if steps.len() < 3
        || !matches!(
            (steps[steps.len() - 2], steps[steps.len() - 1]),
            (
                ChildNumber::Normal { index: 0 | 1 },
                ChildNumber::Normal { .. }
            )
        )
    {
        return None;
    }
    let prefix = &steps[..steps.len() - 2];
    let mut found = globals.iter().filter(|(_, (fp, p))| {
        Fingerprint::from(*fp) == fingerprint && p.into_iter().copied().eq(prefix.iter().copied())
    });
    let (xpub, (_, p)) = found.next()?;
    if found.next().is_some() {
        return None;
    }
    Some((xpub, p))
}

/// Costs a descriptor parse and a script derivation per input of the
/// shape and per output that claims the wallet, for each of the two
/// readings when both are left (through `gives`). The wallet the PSBT
/// defines for the inputs `blocked`, which are the ones held for want of
/// a registered wallet; `ours` is, per input, which of its keys in
/// script order a loaded key derives. `None` where the PSBT does not
/// define one: no global xpubs, an input that is not a `multi`, inputs
/// of different shapes, a key with no origin, at the master, or with no
/// one global xpub for its origin, or keys in an order no one `multi`
/// gives.
///
/// Keys sorted at every address read as `sortedmulti`; keys in one
/// order of cosigners at every address read as `multi`. Where both
/// hold, each is tried against the spend's scripts and those that give
/// them all are kept, `sortedmulti` first; where neither does, both are
/// kept and the hold refuses either by name.
pub(crate) fn offer(
    c: &Compute,
    psbt: &Psbt,
    blocked: &[usize],
    ours: &BTreeMap<usize, Vec<bool>>,
) -> Option<Registration> {
    let inner = psbt.inner();
    if inner.xpub.is_empty() || blocked.is_empty() {
        return None;
    }
    let mut shape: Option<(Wrapper, u8, u8)> = None;
    let mut orders: Vec<Vec<Xpub>> = Vec::new();
    let mut sorted = true;
    let mut first: Option<Vec<RegisteredKey>> = None;
    for &i in blocked {
        let cl = classify(inner, i);
        let ScriptKind::Multisig { wrapper, m, n } = cl.kind else {
            return None;
        };
        if *shape.get_or_insert((wrapper, m, n)) != (wrapper, m, n) {
            return None;
        }
        let ms = cl.multisig?;
        let input = &inner.inputs[i];
        let mut order = Vec::with_capacity(ms.keys.len());
        let mut keys = Vec::with_capacity(ms.keys.len());
        for (k, key) in ms.keys.iter().enumerate() {
            let (fp, path) = input.bip32_derivation.get(&key.inner)?;
            let fingerprint = Fingerprint::from(*fp);
            let (xpub, account) = global_for(&inner.xpub, fingerprint, path)?;
            if order.contains(xpub) {
                return None;
            }
            order.push(*xpub);
            keys.push(RegisteredKey {
                fingerprint,
                path: account.clone(),
                xpub: *xpub,
                ours: ours
                    .get(&i)
                    .and_then(|o| o.get(k))
                    .copied()
                    .unwrap_or(false),
            });
        }
        sorted &= ms
            .keys
            .windows(2)
            .all(|w| w[0].to_bytes() <= w[1].to_bytes());
        orders.push(order);
        first.get_or_insert(keys);
    }
    let (wrapper, m, _) = shape?;
    let cosigners = first?;
    let in_order = orders.iter().all(|o| *o == orders[0]);
    let keys: Vec<String> = cosigners
        .iter()
        .map(|k| format!("[{}/{:#}]{}/<0;1>/*", k.fingerprint, k.path, k.xpub))
        .collect();
    let reading = |sorted: bool| {
        let multi = format!(
            "{}({m},{})",
            if sorted { "sortedmulti" } else { "multi" },
            keys.join(",")
        );
        let descriptor = match wrapper {
            Wrapper::P2sh => format!("sh({multi})"),
            Wrapper::P2wsh => format!("wsh({multi})"),
            Wrapper::P2shP2wsh => format!("sh(wsh({multi}))"),
        };
        Reading { sorted, descriptor }
    };
    let readings: Vec<Reading> = [(sorted, true), (in_order, false)]
        .into_iter()
        .filter(|(left, _)| *left)
        .map(|(_, s)| reading(s))
        .collect();
    if readings.is_empty() {
        return None;
    }
    let mut registration = Registration {
        threshold: usize::from(m),
        wrapper,
        cosigners,
        readings: Vec::new(),
    };
    let given: Vec<Reading> = if readings.len() > 1 {
        readings
            .iter()
            .filter(|r| gives(c, psbt, &registration, &r.descriptor).is_ok())
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    registration.readings = if given.is_empty() { readings } else { given };
    Some(registration)
}

/// Costs a descriptor parse and a script derivation per input of the
/// registration's shape and per output that claims the wallet. The
/// wallet `descriptor` writes, once it gives the script of every input
/// of `psbt` of the shape (the wrapper, threshold and count of keys) at
/// the chain and index that input states, and of every output whose key
/// origins name one of its chains. The first that it does not give is
/// the refusal.
fn gives(
    c: &Compute,
    psbt: &Psbt,
    registration: &Registration,
    descriptor: &str,
) -> Result<WalletPolicy, RegisterRefusal> {
    let policy =
        WalletPolicy::from_descriptor(c, descriptor).map_err(|_| RegisterRefusal::Descriptor)?;
    let inner = psbt.inner();
    for (i, input) in inner.inputs.iter().enumerate() {
        if input.final_script_sig.is_some() || input.final_script_witness.is_some() {
            continue;
        }
        let cl = classify(inner, i);
        let ScriptKind::Multisig { wrapper, m, n } = cl.kind else {
            continue;
        };
        if wrapper != registration.wrapper
            || usize::from(m) != registration.threshold
            || usize::from(n) != registration.cosigners.len()
        {
            continue;
        }
        let spk = cl.utxo.map(|u| u.script_pubkey);
        let place = input
            .bip32_derivation
            .values()
            .find_map(|(fp, path)| policy.leaf_of((*fp).into(), path));
        let given = match (place, spk) {
            (Some((change, index)), Some(spk)) => {
                policy.script_at(c, change, index).is_ok_and(|s| s == spk)
            }
            _ => false,
        };
        if !given {
            return Err(RegisterRefusal::Input(i));
        }
    }
    for (i, out) in inner.outputs.iter().enumerate() {
        let place = out
            .bip32_derivation
            .values()
            .find_map(|(fp, path)| policy.leaf_of((*fp).into(), path));
        let Some((change, index)) = place else {
            continue;
        };
        let spk = &inner.unsigned_tx.output[i].script_pubkey;
        if !policy.script_at(c, change, index).is_ok_and(|s| s == *spk) {
            return Err(RegisterRefusal::Output(i));
        }
    }
    Ok(policy)
}

/// Costs what `gives` costs, and a key derivation per cosigner asked
/// about (through `yours`). The wallet `registration` describes, read
/// as its reading `reading`, once it gives the spend's scripts (the
/// first input or output it does not give is the refusal) and once a
/// cosigner is the person's: `yours` gives, for a fingerprint and path,
/// the extended public key a loaded key with that fingerprint has
/// there (`None` where no key with that fingerprint is loaded), and it
/// must be that cosigner's; a cosigner under a loaded fingerprint that is
/// not what the loaded key gives there is refused by its place
/// ([`RegisterRefusal::NotYourKey`]). A wallet none of whose keys is
/// this device's is never registered: whoever wrote the PSBT could name
/// a key of theirs under the person's fingerprint, and the wallet would
/// then vouch for their outputs as change.
pub fn register(
    c: &Compute,
    psbt: &Psbt,
    registration: &Registration,
    reading: usize,
    yours: &dyn Fn(Fingerprint, &DerivationPath) -> Option<Xpub>,
) -> Result<WalletPolicy, RegisterRefusal> {
    let descriptor = &registration
        .readings
        .get(reading)
        .ok_or(RegisterRefusal::Descriptor)?
        .descriptor;
    let policy = gives(c, psbt, registration, descriptor)?;
    let mut mine = false;
    for (i, k) in registration.cosigners.iter().enumerate() {
        c.count(Work::KeyDerive);
        match yours(k.fingerprint, &k.path) {
            Some(x) if x == k.xpub => mine = true,
            Some(_) => return Err(RegisterRefusal::NotYourKey(i)),
            None => {}
        }
    }
    if !mine {
        return Err(RegisterRefusal::NotYours);
    }
    Ok(policy)
}
