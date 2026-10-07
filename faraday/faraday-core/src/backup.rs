//! A wallet's backup, in two parts kept differently (`docs/WALLETS.md`
//! §5):
//!
//! - the seeds, copied by hand from the screen: words with their BIP-39
//!   indices, and the SeedQR drawn as a ruled grid, checked afterwards by
//!   typing the digits back. Never a file, never a printer.
//! - the wallet in public keys: the descriptor as a QR code and as files,
//!   a multisig config, and for a multisig the split plan.
//!
//! The sheets ([`sheet_wallet`], [`sheet_blank`]) hold nothing secret;
//! they go to the Outbox as PDFs ([`crate::pdf`]), printed wherever the
//! stick goes next.

use osk_bip::keys::{Fingerprint, Network};

use crate::wallet::{Kind, Session, Wallet, fp_text};

/// Which keys each sheet of a split backup carries: sheet `i` carries `d`
/// keys starting at key `i`, wrapping, where `d` is `n` less the keys each
/// sheet leaves off (at most `m − 1`). The split plan.
pub fn split_plan(n: usize, m: usize, omit: usize) -> Vec<Vec<usize>> {
    let max_omit = m.saturating_sub(1).min(n.saturating_sub(1));
    let drop = omit.min(max_omit);
    let d = n - drop;
    (0..n)
        .map(|i| {
            let mut row: Vec<usize> = (0..d).map(|j| (i + j) % n).collect();
            row.sort_unstable();
            row
        })
        .collect()
}

/// What a split plan achieves, found by trying every set of sheets rather
/// than trusting the arithmetic. The split audit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Audit {
    /// Every `m` sheets together hold every key.
    pub every_quorum_rebuilds: bool,
    /// The fewest sheets that hold every key.
    pub smallest_rebuild: usize,
    /// Keys on each sheet.
    pub keys_per_sheet: usize,
    /// The fewest sheets any one key is on.
    pub copies_per_key: usize,
}

/// Audits a plan for an `m`-of-`n` wallet.
pub fn audit(plan: &[Vec<usize>], m: usize) -> Audit {
    let n = plan.len();
    let masks: Vec<u32> = plan
        .iter()
        .map(|row| row.iter().fold(0u32, |acc, k| acc | (1 << k)))
        .collect();
    let all = (1u32 << n) - 1;
    let mut every = true;
    let mut smallest = n;
    for mask in 1..=all {
        let size = mask.count_ones() as usize;
        let cover = (0..n)
            .filter(|i| mask & (1 << i) != 0)
            .fold(0u32, |acc, i| acc | masks[i]);
        let rebuilds = cover == all;
        if size == m && !rebuilds {
            every = false;
        }
        if rebuilds && size < smallest {
            smallest = size;
        }
    }
    Audit {
        every_quorum_rebuilds: every,
        smallest_rebuild: smallest,
        keys_per_sheet: plan.first().map(Vec::len).unwrap_or(0),
        copies_per_key: (0..n)
            .map(|k| plan.iter().filter(|r| r.contains(&k)).count())
            .min()
            .unwrap_or(0),
    }
}

/// The multisig config Coldcard writes and Sparrow imports as "Coldcard
/// Multisig"; SeedSigner recognises it by "multisig setup file" in its
/// first line. `only` keeps those key lines alone, for a split sheet.
pub fn multisig_config(wallet: &Wallet, only: Option<&[usize]>) -> Option<String> {
    let format = match Kind::of(&wallet.policy) {
        Kind::Multi(osk_bip::policy::Wrapper::Sh) => "P2SH",
        Kind::Multi(osk_bip::policy::Wrapper::ShWsh) => "P2SH-P2WSH",
        Kind::Multi(_) => "P2WSH",
        _ => return None,
    };
    Some(config(
        wallet,
        only,
        format,
        "# Coldcard Multisig setup file (created by Faraday)\n",
    ))
}

/// Whether a wallet's backup can be split between its signers: a
/// multisig, `wsh`, `sh` or Taproot's `sortedmulti_a`.
pub fn splits(wallet: &Wallet) -> bool {
    matches!(Kind::of(&wallet.policy), Kind::Multi(_) | Kind::TapMulti)
}

/// One sheet of a split backup, holding the keys `keep` names: the
/// multisig config's lines for a `wsh` or `sh` multisig; for a Taproot
/// multisig the same lines with `Format: P2TR`, which Faraday rebuilds
/// as `tr(NUMS, sortedmulti_a(…))` (`restore::merge`). Coldcard's own
/// config has no Taproot format, so a Taproot wallet has no whole config.
pub fn split_share(wallet: &Wallet, keep: &[usize]) -> Option<String> {
    match Kind::of(&wallet.policy) {
        Kind::Multi(_) => multisig_config(wallet, Some(keep)),
        Kind::TapMulti => Some(config(
            wallet,
            Some(keep),
            "P2TR",
            "# Faraday split backup sheet: Taproot multisig, tr(NUMS, sortedmulti_a)\n",
        )),
        _ => None,
    }
}

fn config(wallet: &Wallet, only: Option<&[usize]>, format: &str, head: &str) -> String {
    let (m, n) = Session::quorum(wallet);
    let mut out = String::from(head);
    if let Some(keep) = only {
        out.push_str(&format!(
            "# Partial: {} of {n} xpubs. Any {m} sheets together rebuild the wallet.\n",
            keep.len()
        ));
    }
    out.push_str(&format!(
        "#\nName: {}\nPolicy: {m} of {n}\nFormat: {format}\n",
        sanitize(&wallet.name)
    ));
    for (i, key) in wallet.policy.keys().iter().enumerate() {
        if only.is_some_and(|keep| !keep.contains(&i)) {
            continue;
        }
        let fp = key
            .fingerprint()
            .map(fp_text)
            .unwrap_or_else(|| "00000000".into());
        let path = key
            .path()
            .map(|p| {
                format!(
                    "m/{}",
                    p.to_string().trim_start_matches("m/").replace('h', "'")
                )
            })
            .unwrap_or_else(|| "m".into());
        out.push_str(&format!(
            "\nDerivation: {path}\n{}: {}\n",
            fp.to_uppercase(),
            key.xpub()
        ));
    }
    out
}

/// A name Coldcard accepts: up to twenty printable ASCII characters.
/// The wallet as the JSON file Specter Desktop writes and Sparrow imports
/// ("Specter Desktop" in its import list): a label, the block height to
/// scan from, and the descriptor. The descriptor and the label carry no
/// character JSON needs escaped: a descriptor is ASCII without quotes or
/// backslashes, and the label is sanitized.
pub fn wallet_json(name: &str, policy: &osk_bip::policy::WalletPolicy) -> String {
    let label: String = sanitize(name)
        .chars()
        .filter(|c| *c != '"' && *c != '\\')
        .collect();
    format!(
        "{{\n  \"label\": \"{label}\",\n  \"blockheight\": 0,\n  \"descriptor\": \"{}\"\n}}\n",
        policy.to_descriptor_checksummed()
    )
}

fn sanitize(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .take(20)
        .collect()
}

/// One split sheet as a sheet file to print: which sheet it is, the
/// quorum, and the share's lines, which its QR code holds too.
pub fn sheet_share(wallet: &Wallet, part: usize, of: usize, share: &str) -> String {
    let (m, n) = Session::quorum(wallet);
    let mut out = String::from("faraday-sheet 1\nkind: share\n");
    out.push_str(&format!("name: {}\n", wallet.name));
    out.push_str(&format!("shape: {}\n", Session::shape(wallet)));
    out.push_str(&format!("part: {part} of {of}\n"));
    out.push_str(&format!("quorum: {m} of {n}\n"));
    for line in share.lines() {
        out.push_str(&format!("line: {line}\n"));
    }
    out
}

/// The sheet file for a wallet's backup page: everything on it is public.
/// The desktop app turns it into a PDF; the format is lines of
/// `field: value` after a first line naming it.
pub fn sheet_wallet(session: &Session, wallet: &Wallet) -> String {
    let mut out = String::from("faraday-sheet 1\nkind: wallet\n");
    out.push_str(&format!("network: {}\n", session.network().name()));
    out.push_str(&format!("name: {}\n", wallet.name));
    out.push_str(&format!("shape: {}\n", Session::shape(wallet)));
    out.push_str(&format!(
        "descriptor: {}\n",
        wallet.policy.to_descriptor_checksummed()
    ));
    for (i, slot) in session.slots(wallet).iter().enumerate() {
        let key = &wallet.policy.keys()[i];
        out.push_str(&format!(
            "key: {} {} {}\n",
            i + 1,
            slot.fingerprint
                .map(fp_text)
                .unwrap_or_else(|| "--------".into()),
            key.key_text()
        ));
    }
    for (label, change, index) in [
        ("0/0", false, 0),
        ("0/1", false, 1),
        ("0/2", false, 2),
        ("1/0", true, 0),
    ] {
        out.push_str(&format!(
            "address: {label} {}\n",
            session.address(wallet, change, index)
        ));
    }
    out
}

/// The blank template: numbered word lines and the SeedQR grid's fixed
/// squares, for a seed of `words` words. It holds no secret and may go to
/// any printer. `wallet` adds the path, script and network to fill in.
pub fn sheet_blank(words: usize, wallet: Option<&Wallet>, network: Network) -> String {
    let mut out = format!(
        "faraday-sheet 1\nkind: blank\nwords: {words}\nnetwork: {}\n",
        network.name()
    );
    if let Some(w) = wallet {
        out.push_str(&format!("name: {}\nshape: {}\n", w.name, Session::shape(w)));
    }
    out
}

/// Whether module (x, y) of a SeedQR of side `n` is fixed by the QR
/// standard rather than by the seed: finders and their separators, timing,
/// the alignment square, the format areas and the dark module. These are
/// the same for every seed of the same length, so the blank template can
/// carry them and the copy-by-hand grid draws them grey.
pub fn structural(x: usize, y: usize, n: usize) -> bool {
    let finder = (x < 8 && y < 8) || (x >= n - 8 && y < 8) || (x < 8 && y >= n - 8);
    let timing = x == 6 || y == 6;
    let format = (y == 8 && (x < 9 || x >= n - 8)) || (x == 8 && (y < 9 || y >= n - 8));
    let version = (n - 17) / 4;
    let align = version >= 2 && {
        let c = n - 7;
        x.abs_diff(c) <= 2 && y.abs_diff(c) <= 2
    };
    finder || timing || format || align
}

/// The squares fixed for every SeedQR of side `n`: finders and their
/// separators, timing and the alignment square. The format areas are left
/// out: they depend on the mask the seed's code was given.
pub fn structural_fixed(x: usize, y: usize, n: usize) -> bool {
    let finder = (x < 8 && y < 8) || (x >= n - 8 && y < 8) || (x < 8 && y >= n - 8);
    let format = (y == 8 && (x < 9 || x >= n - 8)) || (x == 8 && (y < 9 || y >= n - 8));
    let version = (n - 17) / 4;
    let align = version >= 2 && {
        let c = n - 7;
        x.abs_diff(c) <= 2 && y.abs_diff(c) <= 2
    };
    (finder && !format) || ((x == 6 || y == 6) && !format) || align
}

/// What a typed-back copy of a standard SeedQR's digits says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyCheck {
    /// Every digit matches.
    Matches,
    /// Digits so far match; more to type.
    SoFar {
        /// Digits typed.
        typed: usize,
        /// Digits in all.
        of: usize,
    },
    /// A word's four digits differ.
    WrongWord(usize),
}

/// Compares typed digits with the seed's: four digits per word.
pub fn check_copy(typed: &str, digits: &[u8]) -> CopyCheck {
    let t: Vec<u8> = typed.bytes().filter(u8::is_ascii_digit).collect();
    for (i, (a, b)) in t.iter().zip(digits).enumerate() {
        if a != b {
            return CopyCheck::WrongWord(i / 4 + 1);
        }
    }
    if t.len() >= digits.len() {
        CopyCheck::Matches
    } else {
        CopyCheck::SoFar {
            typed: t.len(),
            of: digits.len(),
        }
    }
}

/// The envelope: which sheet goes with which seed, one line per key.
pub fn envelope(session: &Session, wallet: &Wallet, plan: Option<&[Vec<usize>]>) -> Vec<String> {
    let slots = session.slots(wallet);
    slots
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let fp = s
                .fingerprint
                .map(fp_text)
                .unwrap_or_else(|| "--------".into());
            let sheet = match plan {
                Some(p) => format!("share {} of {}", i + 1, p.len()),
                None => "the backup sheet".to_string(),
            };
            format!(
                "Envelope {}: the seed of key {} {}, with {sheet}",
                i + 1,
                i + 1,
                fp
            )
        })
        .collect()
}

/// The fingerprint a key line on a sheet carries.
pub fn fp_of(fp: Option<Fingerprint>) -> String {
    fp.map(fp_text).unwrap_or_else(|| "--------".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The audit, for the shapes the backup is made in: a 3-of-5 split
    /// with two keys left off each sheet rebuilds from any three sheets,
    /// and no single sheet holds every key.
    #[test]
    fn any_quorum_of_split_sheets_rebuilds_the_wallet() {
        for (n, m) in [(3, 2), (5, 3), (7, 4), (15, 8)] {
            let plan = split_plan(n, m, m - 1);
            let a = audit(&plan, m);
            assert!(a.every_quorum_rebuilds, "{m} of {n}");
            assert!(
                a.keys_per_sheet < n,
                "{m} of {n}: one sheet holds every key"
            );
        }
    }

    /// Every test wallet written as a wallet .json reads back as the same
    /// wallet.
    #[test]
    fn a_wallet_json_reads_back() {
        for kit in crate::testkit::kits() {
            let policy = crate::wallet::read_wallet(&kit.descriptor).expect("kit");
            let json = wallet_json(kit.name, &policy);
            let back = crate::wallet::read_wallet(&json).expect(kit.id);
            assert_eq!(
                back.to_descriptor_checksummed(),
                policy.to_descriptor_checksummed(),
                "{}",
                kit.id
            );
        }
    }

    /// A typed copy that differs names the word, counting from one.
    #[test]
    fn a_wrong_digit_names_its_word() {
        let digits = b"013801380138";
        assert_eq!(
            check_copy("01380138", digits),
            CopyCheck::SoFar { typed: 8, of: 12 }
        );
        assert_eq!(check_copy("0138 0139", digits), CopyCheck::WrongWord(2));
        assert_eq!(check_copy("013801380138", digits), CopyCheck::Matches);
    }
}
