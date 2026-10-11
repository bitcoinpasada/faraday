//! The miniscript compiler as a calculator: a concrete policy in, a
//! descriptor out, with the keys it names, the ways it can be spent, and
//! the wallet it is when it is one.

use alloc::string::String;
use alloc::vec::Vec;

use miniscript::Segwitv0;
use miniscript::descriptor::Descriptor;
use miniscript::policy::{Concrete, Liftable};

use crate::policy::WalletPolicy;
use crate::spend::SpendPath;

/// What a compiled policy is wrapped in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PolicyScript {
    /// `wsh(...)`: the policy compiles to one Segwit v0 script.
    #[default]
    Segwit,
    /// `tr(...)`: the compiler chooses an internal key and a tree.
    Taproot,
}

impl PolicyScript {
    /// The two, in the order a Choice lists them.
    pub const ALL: [PolicyScript; 2] = [PolicyScript::Segwit, PolicyScript::Taproot];
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
/// `keys` is a substitution: each pair is a token and the key expression
/// it stands for, so a policy can name a key by a loaded key's
/// fingerprint, `pk(73c5da0a)`, rather than as an xpub. A token that is
/// neither a listed one nor something `miniscript` reads as a key is the
/// compiler's to refuse.
///
/// The error is the compiler's own message on one line.
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
    for (token, expression) in keys {
        filled = filled.replace(token.as_str(), expression);
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
            crate::spend::spend_paths(&semantic, &keys)
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
    use miniscript::ForEachKey;
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

/// A compiler message as one line.
fn one_line<E: core::fmt::Display>(error: E) -> String {
    let text = alloc::format!("{error}");
    let line = text.lines().next().unwrap_or("").trim();
    String::from(line)
}
