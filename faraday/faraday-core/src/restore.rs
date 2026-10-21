//! Restoring a wallet from its backup (`docs/WALLETS.md` §2 and §5): the
//! wallet from its descriptor, its multisig config, or the split shares of
//! a multisig, which are merged as they arrive until every key is in hand;
//! then the seeds, typed back in.

use crate::strings::EN;
use osk_bip::multisig_config;
use osk_crypto::work::Compute;

/// What one multisig config file, whole or a share, says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigParts {
    /// `Name:`.
    pub name: Option<String>,
    /// `Policy: M of N`.
    pub m: usize,
    /// The N of the policy.
    pub n: usize,
    /// `Format:`.
    pub format: String,
    /// Each key line: fingerprint, the derivation in force, the xpub.
    pub keys: Vec<(String, String, String)>,
}

/// Reads a multisig config's lines without requiring every key.
pub fn parts(text: &str) -> Option<ConfigParts> {
    let mut name = None;
    let mut policy = None;
    let mut format = None;
    let mut derivation = String::new();
    let mut keys = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (k, v) = line.split_once(':')?;
        let (k, v) = (k.trim(), v.trim());
        match k.to_ascii_lowercase().as_str() {
            "name" => name = Some(v.to_string()),
            "policy" => {
                let (m, n) = v.split_once(" of ")?;
                policy = Some((m.trim().parse().ok()?, n.trim().parse().ok()?));
            }
            "format" => format = Some(v.to_string()),
            "derivation" => derivation = v.to_string(),
            fp if fp.len() == 8 && fp.chars().all(|c| c.is_ascii_hexdigit()) => {
                keys.push((fp.to_string(), derivation.clone(), v.to_string()));
            }
            _ => {}
        }
    }
    let (m, n) = policy?;
    if keys.is_empty() {
        return None;
    }
    Some(ConfigParts {
        name,
        m,
        n,
        format: format.unwrap_or_else(|| "P2WSH".into()),
        keys,
    })
}

/// Whether a config holds fewer keys than its policy names: a share.
pub fn is_share(text: &str) -> bool {
    parts(text).is_some_and(|p| p.keys.len() < p.n)
}

/// What a set of shares adds up to.
pub struct Merged {
    /// The quorum and size, from the shares.
    pub m: usize,
    /// Keys in the wallet.
    pub n: usize,
    /// The fingerprints in hand.
    pub have: Vec<String>,
    /// The whole config, once every key is in hand.
    pub whole: Option<String>,
}

/// Merges shares of one wallet. Shares that disagree on the quorum or the
/// format are refused.
pub fn merge(texts: &[String]) -> Result<Merged, String> {
    let mut all: Option<ConfigParts> = None;
    for t in texts {
        let p = parts(t).ok_or(EN.restore_not_a_multisig)?;
        match all.as_mut() {
            None => all = Some(p),
            Some(a) => {
                if a.m != p.m || a.n != p.n || !a.format.eq_ignore_ascii_case(&p.format) {
                    return Err(EN.restore_these_shares.into());
                }
                for k in p.keys {
                    if !a.keys.iter().any(|x| x.0.eq_ignore_ascii_case(&k.0)) {
                        a.keys.push(k);
                    }
                }
            }
        }
    }
    let a = all.ok_or(EN.restore_no_shares)?;
    let have: Vec<String> = a.keys.iter().map(|k| k.0.to_lowercase()).collect();
    // A Taproot multisig's sheets rebuild its descriptor: Coldcard's
    // config has no Taproot format.
    if a.format.eq_ignore_ascii_case("P2TR") {
        let whole = (a.keys.len() >= a.n).then(|| {
            let keys: Vec<String> = a
                .keys
                .iter()
                .map(|(fp, d, x)| {
                    let path = d.trim_start_matches("m/").replace('\'', "h");
                    let fp = fp.to_lowercase();
                    if path.is_empty() || path == "m" {
                        format!("[{fp}]{x}/<0;1>/*")
                    } else {
                        format!("[{fp}/{path}]{x}/<0;1>/*")
                    }
                })
                .collect();
            format!("tr({NUMS},sortedmulti_a({},{}))", a.m, keys.join(","))
        });
        return Ok(Merged {
            m: a.m,
            n: a.n,
            have,
            whole,
        });
    }
    let whole = (a.keys.len() >= a.n).then(|| {
        let mut out = String::from("# Multisig config rebuilt by Faraday from its shares\n");
        if let Some(n) = &a.name {
            out.push_str(&format!("Name: {n}\n"));
        }
        out.push_str(&format!(
            "Policy: {} of {}\nFormat: {}\n",
            a.m, a.n, a.format
        ));
        for (fp, d, x) in &a.keys {
            out.push_str(&format!("\nDerivation: {d}\n{fp}: {x}\n"));
        }
        out
    });
    Ok(Merged {
        m: a.m,
        n: a.n,
        have,
        whole,
    })
}

/// Costs what `parse` and `read_wallet` cost. Whether the rebuilt text
/// reads as a wallet.
pub fn readable(c: &Compute, whole: &str) -> bool {
    multisig_config::parse(c, whole).is_ok() || crate::wallet::read_wallet(c, whole).is_ok()
}

/// BIP-341's unspendable internal key, a Taproot multisig's.
const NUMS: &str = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";

#[cfg(test)]
mod tests {
    use super::*;

    /// The shares of a split backup rebuild the same wallet as the whole
    /// config, from any quorum of them.
    #[test]
    fn a_quorum_of_shares_rebuilds_the_wallet() {
        let c = crate::testkit::compute();
        let whole = "Name: T\nPolicy: 2 of 3\nFormat: P2WSH\n";
        let keys = [
            (
                "9a6a2580",
                "xpub6EeqK2JLwngrHJEQ4X4iqrySZV9qU3TgwMgf6NStLZa37AfNiHTtTE9ji1F9YQDLArJMLy8sw3Q2samVj5VQQjaaUHr5z2Hz57NWHJCfh31",
            ),
            (
                "5d388376",
                "xpub6EFjRTDekDh2sARTm1vbBFeVFb8srMNspMpgypVTSRnzabCbm5ch3oqiw5K9ah7mo9gLub8yU6ciBnrbFwtrXRJY93CThvb9oqVCK47dBEx",
            ),
            (
                "cf2e083d",
                "xpub6EvbTisSw8RtWaatFV6xEgY1v1TymXwGfu4kn8PfYcdbPthFWK8McYnVQwxF5EfQQeu6bYpgQ9vG3wgSREeeQSkUzvcidGMjujQroTd1VrT",
            ),
        ];
        let share = |which: &[usize]| {
            let mut s = format!("# Partial\n{whole}");
            for &i in which {
                s.push_str(&format!(
                    "\nDerivation: m/48'/0'/0'/2'\n{}: {}\n",
                    keys[i].0, keys[i].1
                ));
            }
            s
        };
        let one = merge(&[share(&[0, 1])]).unwrap();
        assert!(one.whole.is_none(), "one share alone does not rebuild");
        let two = merge(&[share(&[0, 1]), share(&[1, 2])]).unwrap();
        let rebuilt = two.whole.expect("two shares rebuild");
        let a = multisig_config::parse(&c, &rebuilt).unwrap();
        let mut full = whole.to_string();
        for k in keys {
            full.push_str(&format!("\nDerivation: m/48'/0'/0'/2'\n{}: {}\n", k.0, k.1));
        }
        assert_eq!(
            a.to_descriptor(),
            multisig_config::parse(&c, &full).unwrap().to_descriptor()
        );
        assert!(readable(&c, &rebuilt));
    }

    /// A rebuilt config is checked as any other: a key whose SLIP-132
    /// version names another script than the file's (a single-key `zpub`
    /// or a nested `Ypub` in a P2WSH wallet) does not read as a wallet.
    #[test]
    fn a_config_with_a_slip132_key_of_another_script_is_not_readable() {
        let c = crate::testkit::compute();
        use osk_bip::slip132::{Script, encode};
        let xpub = |i: usize| {
            [
                "xpub6EeqK2JLwngrHJEQ4X4iqrySZV9qU3TgwMgf6NStLZa37AfNiHTtTE9ji1F9YQDLArJMLy8sw3Q2samVj5VQQjaaUHr5z2Hz57NWHJCfh31",
                "xpub6EFjRTDekDh2sARTm1vbBFeVFb8srMNspMpgypVTSRnzabCbm5ch3oqiw5K9ah7mo9gLub8yU6ciBnrbFwtrXRJY93CThvb9oqVCK47dBEx",
            ][i]
        };
        let config = |second: &str| {
            format!(
                "Policy: 2 of 2\nFormat: P2WSH\nDerivation: m/48'/0'/0'/2'\n9a6a2580: {}\n5d388376: {second}\n",
                xpub(0)
            )
        };
        assert!(readable(&c, &config(xpub(1))));
        let zpub_multi = encode(xpub(1), Script::NativeMultisig).unwrap();
        assert!(zpub_multi.starts_with("Zpub"));
        assert!(readable(&c, &config(&zpub_multi)));
        for script in [Script::NativeSegwit, Script::NestedMultisig] {
            let other = encode(xpub(1), script).unwrap();
            assert!(!readable(&c, &config(&other)), "{other}");
        }
    }
}
