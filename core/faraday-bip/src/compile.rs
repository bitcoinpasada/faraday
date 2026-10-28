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
use faraday_crypto::work::Compute;

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

/// Why a policy was not compiled: a kind, which the app words, with the
/// compiler's own message on one line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// Nothing typed.
    Empty,
    /// Not a concrete policy the compiler reads.
    Policy(String),
    /// A policy no script compiles from.
    Script(String),
}

/// Costs a miniscript compilation and a parse of the descriptor:
/// compiles a concrete policy into a descriptor.
///
/// `keys` is a substitution: each pair is a token and the key expression
/// it stands for, so a policy can name a key by a loaded key's
/// fingerprint, `pk(73c5da0a)`, rather than as an xpub. A token that is
/// neither a listed one nor something `miniscript` reads as a key is the
/// compiler's to refuse.
///
/// The refusal carries the compiler's own message on one line.
pub fn compile(
    c: &Compute,
    text: &str,
    script: PolicyScript,
    keys: &[(String, String)],
) -> Result<PolicyFacts, Refusal> {
    let text = text.trim();
    if text.is_empty() {
        return Err(Refusal::Empty);
    }
    let filled = substitute(text, keys);
    let policy: Concrete<String> = filled.parse().map_err(|e| Refusal::Policy(one_line(e)))?;
    let descriptor = match script {
        PolicyScript::Segwit => policy
            .compile::<Segwitv0>()
            .map_err(one_line)
            .and_then(|ms| Descriptor::new_wsh(ms).map_err(one_line)),
        // `compile_tr` with no unspendable key takes the internal key
        // out of the policy itself, which is what a person typing
        // `or(pk(A),...)` means by the first branch.
        PolicyScript::Taproot => policy.compile_tr(None).map_err(one_line),
    }
    .map_err(Refusal::Script)?;
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
        wallet: WalletPolicy::parse_any(c, &text).ok(),
        paths,
        descriptor: text,
    })
}

/// `text` with each token written as a whole argument, after `(` or `,`
/// and before `,` or `)`, replaced by its expression, in one pass: a
/// token inside a longer word or an origin's brackets, and a token that
/// an expression put in happens to contain, stay as they were.
fn substitute(text: &str, keys: &[(String, String)]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    let mut before = None;
    while !rest.is_empty() {
        let end = rest.find(['(', ',', ')']).unwrap_or(rest.len());
        let (argument, tail) = rest.split_at(end);
        let after = tail.chars().next();
        let name = argument.trim();
        let whole = matches!(before, Some('(' | ',')) && matches!(after, Some(',' | ')'));
        match keys.iter().find(|(token, _)| token == name) {
            Some((_, expression)) if whole && !name.is_empty() => out.push_str(expression),
            _ => out.push_str(argument),
        }
        let Some(delimiter) = after else { break };
        out.push(delimiter);
        before = Some(delimiter);
        rest = &tail[1..];
    }
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    fn keys_of(text: &str, tokens: &[(&str, &str)]) -> Result<Vec<String>, Refusal> {
        let tokens: Vec<(String, String)> = tokens
            .iter()
            .map(|(t, e)| (String::from(*t), String::from(*e)))
            .collect();
        compile(&Compute::unchecked(), text, PolicyScript::Segwit, &tokens).map(|f| f.keys)
    }

    #[test]
    fn a_token_stands_for_its_key_where_a_whole_key_is_written() {
        assert_eq!(
            keys_of("or(pk(73c5da0a),pk(abcd1234))", &[("73c5da0a", "KEYA")]).unwrap(),
            ["KEYA", "abcd1234"]
        );
    }

    #[test]
    fn a_token_inside_a_longer_word_is_left_alone() {
        assert_eq!(
            keys_of("pk(x73c5da0a)", &[("73c5da0a", "KEYA")]).unwrap(),
            ["x73c5da0a"]
        );
        assert_eq!(
            keys_of("pk(73c5da0ab)", &[("73c5da0a", "KEYA")]).unwrap(),
            ["73c5da0ab"]
        );
    }

    #[test]
    fn what_a_token_became_is_not_read_for_tokens_again() {
        assert_eq!(
            keys_of("or(pk(aa),pk(bb))", &[("aa", "bbkey"), ("bb", "other")]).unwrap(),
            ["bbkey", "other"]
        );
    }
}
