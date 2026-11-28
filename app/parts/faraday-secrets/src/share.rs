//! The multisig config a file may be, read without requiring every key: only to tell a share (fewer keys than its policy names) from a whole wallet (`main`'s `restore::parts` and `is_share`).

use faraday_bip::multisig_config;
use faraday_text::Text;


/// What one multisig config file, whole or a share, says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigParts {
    /// `Name:`.
    pub name: Option<Text>,
    /// `Policy: M of N`.
    pub m: usize,
    /// The N of the policy.
    pub n: usize,
    /// `Format:`, P2SH when the file has none (F263).
    pub format: Text,
    /// Each key line: fingerprint, the derivation in force, the xpub.
    pub keys: Vec<(Text, Text, Text)>,
}

/// The longest `Derivation:` value read: a BIP-32 path at its deepest,
/// `m` and 255 steps of `/2147483647h`. Each key line keeps a copy
/// of the derivation in force, so a longer one is refused rather than
/// copied fifteen times.
const MAX_DERIVATION_LEN: usize = 1 + 255 * 12;

/// Whether `v` is a derivation path as Faraday's own sheets write one:
/// empty (a share without a `Derivation:` line has none), `m` or `m/`, or
/// an optional lowercase `m/` followed by 1 to 255 steps separated by single
/// `/`. A step is ASCII digits only, with no leading zero unless it is `0`,
/// a value below 2^31 and at most one trailing `'` or `h`. Anything else
/// is refused, so the value is written into the rebuilt wallet as read.
fn is_path(v: &str) -> bool {
    if matches!(v, "" | "m" | "m/") {
        return true;
    }
    let body = v.strip_prefix("m/").unwrap_or(v);
    let steps = body.split('/');
    steps.clone().count() <= 255
        && steps.into_iter().all(|step| {
            let digits = step.strip_suffix(['\'', 'h']).unwrap_or(step);
            !digits.is_empty()
                && digits.bytes().all(|b| b.is_ascii_digit())
                && (digits == "0" || !digits.starts_with('0'))
                && digits.len() <= 10
                && digits.parse::<u64>().is_ok_and(|n| n < 1 << 31)
        })
}

/// Reads a multisig config's lines without requiring every key. A
/// policy of more than [`multisig_config::MAX_KEYS`] keys, more key
/// lines than the policy names, a sixteenth key line or a derivation longer than a BIP-32 path can be is refused,
/// so a long file costs no more than a wallet does.
pub fn parts(text: &str) -> Option<ConfigParts> {
    let mut name = None;
    let mut policy = None;
    let mut format = None;
    let mut derivation = Text::default();
    let mut keys = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (k, v) = line.split_once(':')?;
        let (k, v) = (k.trim(), v.trim());
        match Text::ascii_lowercase(k).as_str() {
            "name" => name = Some(Text::of(v)),
            "policy" => {
                let (m, n) = v.split_once(" of ")?;
                policy = Some((m.trim().parse().ok()?, n.trim().parse().ok()?));
            }
            "format" => format = Some(Text::of(v)),
            "derivation" => {
                if v.len() > MAX_DERIVATION_LEN {
                    return None;
                }
                if !is_path(v) {
                    return None;
                }
                derivation = Text::of(v);
            }
            fp if fp.len() == 8 && fp.chars().all(|c| c.is_ascii_hexdigit()) => {
                if keys.len() == multisig_config::MAX_KEYS {
                    return None;
                }
                // One extended public key, as a Coldcard file reads it.
                faraday_bip::slip132::decode(v).ok()?;
                keys.push((Text::of(fp), derivation.clone(), Text::of(v)));
            }
            _ => {}
        }
    }
    let (m, n) = policy?;
    if keys.is_empty() || n > multisig_config::MAX_KEYS || keys.len() > n {
        return None;
    }
    Some(ConfigParts {
        name,
        m,
        n,
        format: format.unwrap_or_else(|| Text::of("P2SH")),
        keys,
    })
}

/// Whether a config holds fewer keys than its policy names: a share.
pub fn is_share(text: &str) -> bool {
    parts(text).is_some_and(|p| p.keys.len() < p.n)
}
