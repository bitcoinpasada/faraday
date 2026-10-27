//! A Faraday build ID (`docs/DECISIONS.md` F5): read out of a kernel
//! release string, described for a person, and compared with another, as
//! the Starter's Keep data decides by.

use crate::strings::EN;
use crate::strings::fill;

/// How writing one build over a stick that holds another stands under F2
/// and F5's rules ([`compare`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fit {
    /// It can be written.
    Fits,
    /// It carries a newer Faraday than the one to be written: it can be
    /// written.
    Newer,
    /// The source is a dev build and this stick does not hold one: it
    /// gains a serial console and login. It can be written.
    Dev,
    /// It already carries this Faraday.
    Same,
}

/// The Faraday version in a release string: `0.2.0 (4d0680b1a2b3)` from
/// `6.6.84-faraday-0.2.0+4d0680b1a2b3`; a stick made before Faraday put
/// its version in the kernel shows as earlier.
pub fn version(release: Option<&str>) -> String {
    match release.and_then(|r| r.split_once("-faraday-")) {
        Some((_, build_id)) => describe(build_id),
        None => EN.build_0_1_0_or_earlier.to_string(),
    }
}

/// A build ID described (`docs/DECISIONS.md` F5): the part of the kernel
/// release string after `-faraday-`, or [`crate::BUILD`] directly.
///
/// | Input | Output |
/// |---|---|
/// | `0.2.0` | `0.2.0` |
/// | `0.1.0+ef24784b48d8.test` | `0.1.0 test release (ef24784b48d8)` |
/// | `0.1.0+ef24784b48d8.dirty-3fa9c1d2.test` | `0.1.0 test release (ef24784b48d8, changes 3fa9c1d2)` |
/// | `0.1.0+ef24784b48d8.dev` | `0.1.0 dev (ef24784b48d8)` |
/// | `0.1.0+ef24784b48d8.dirty-3fa9c1d2.dev` | `0.1.0 dev (ef24784b48d8, changes 3fa9c1d2)` |
/// | `0.1.0+4d0680b1a2b3` | `0.1.0 (4d0680b1a2b3)` |
/// | `0.1.0+4d0680b1a2b3.dirty` | `0.1.0 (4d0680b1a2b3.dirty)` |
///
/// The last two are the old format, kept. Anything after `+` that this
/// does not recognise follows the old rule, `number (rest)`.
pub fn describe(id: &str) -> String {
    let Some((number, rest)) = id.split_once('+') else {
        // A release: the version alone.
        return id.to_string();
    };
    let mut parts = rest.splitn(3, '.');
    let commit = parts.next().unwrap_or(rest);
    let after_commit: Vec<&str> = parts.collect();
    let (changes, kind) = match after_commit.as_slice() {
        [kind] if *kind == "test" || *kind == "dev" => (None, Some(*kind)),
        [dirty, kind] if dirty.starts_with("dirty-") && (*kind == "test" || *kind == "dev") => {
            (dirty.strip_prefix("dirty-"), Some(*kind))
        }
        _ => (None, None),
    };
    match (kind, changes) {
        (Some("test"), None) => fill(EN.build_test_release_short, &[number, commit]),
        (Some("test"), Some(fp)) => fill(EN.build_test_release, &[number, commit, fp]),
        (Some("dev"), None) => fill(EN.build_dev, &[number, commit]),
        (Some("dev"), Some(fp)) => fill(EN.build_dev_changes, &[number, commit, fp]),
        _ => format!("{number} ({rest})"),
    }
}

/// The build ID in a kernel release string (`docs/DECISIONS.md` F5):
/// `0.2.0+4d0680b1a2b3.test` from `6.6.84-faraday-0.2.0+4d0680b1a2b3.test`.
/// `None` for a release string Faraday did not make.
pub fn build_id(release: &str) -> Option<&str> {
    release.split_once("-faraday-").map(|(_, id)| id)
}

/// The version number a build ID carries, to compare: `(0, 2, 0)` from
/// `0.2.0+4d0680b1a2b3.test`.
pub fn version_number(id: &str) -> Option<(u64, u64, u64)> {
    let v = id.split(['+', '-']).next()?;
    let mut n = v.split('.').map(|p| p.parse::<u64>().ok());
    Some((n.next()??, n.next()??, n.next()??))
}

/// How writing build `source` over a stick that holds build `target`
/// stands under F2 and F5's rules, by build ID: the same build is
/// refused; a dev build over a stick that does not hold one, and a stick
/// with a higher version number, are warned about; anything else is
/// written. `target` is `None` for a stick made before Faraday put its
/// version in the kernel, which is earlier than any. The Starter's Keep
/// data decides by this.
pub fn compare(target: Option<&str>, source: &str) -> Fit {
    if target == Some(source) {
        return Fit::Same;
    }
    if source.ends_with(".dev") && !target.is_some_and(|t| t.ends_with(".dev")) {
        return Fit::Dev;
    }
    match (target.and_then(version_number), version_number(source)) {
        (Some(t), Some(s)) if t > s => Fit::Newer,
        _ => Fit::Fits,
    }
}
