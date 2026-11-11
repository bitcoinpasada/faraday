//! A wallet's spend paths in words, for its card, the chart's wallet sheet
//! and the wallet sheet's PDF (`docs/BACKUP-FLOW.md` §18.4): the keys of
//! each path, when it can spend, and the margin to move each coin
//! within. The paths and the shortest wait are read once when the
//! wallet's policy is set ([`crate::wallet::Spend`]); this only words
//! them.

use faraday_bip::spend::{Lock, SpendPath};
use faraday_strings::{EN, fill};

use crate::wallet::{Shortest, Waits};

/// A whole number with a space between its thousands: `52 560`.
pub(crate) fn spaced(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}

/// Slot numbers as a list: `2`, `2 and 3`, `1, 2 and 3`.
fn list(keys: &[usize]) -> String {
    let n: Vec<String> = keys.iter().map(|k| (k + 1).to_string()).collect();
    match n.split_last() {
        Some((last, [])) => last.clone(),
        Some((last, rest)) => fill(EN.wallet_paths_list_and, &[&rest.join(", "), last]),
        None => String::new(),
    }
}

/// The keys a path wants: `Key 1`, `Keys 2 and 3, 2 of 2`.
pub(crate) fn keys_text(keys: &[usize]) -> String {
    match keys {
        [k] => fill(EN.wallet_paths_key, &[&(k + 1).to_string()]),
        _ => fill(
            EN.wallet_paths_keys,
            &[
                &list(keys),
                &keys.len().to_string(),
                &keys.len().to_string(),
            ],
        ),
    }
}

/// One wait in words.
fn lock_text(lock: Lock) -> String {
    match lock {
        Lock::Blocks(n) => fill(
            EN.wallet_paths_blocks,
            &[
                &spaced(u64::from(n)),
                &faraday_bip::recovery::days_of_blocks(n).to_string(),
            ],
        ),
        Lock::Intervals(n) => fill(
            EN.wallet_paths_intervals,
            &[&(u64::from(n) * 512 / 86_400).to_string()],
        ),
        Lock::Height(h) => fill(EN.wallet_paths_after_block, &[&spaced(u64::from(h))]),
        Lock::Time(t) => {
            let (y, m, d) = faraday_crypto::date::civil_from_days(i64::from(t / 86_400));
            fill(
                EN.wallet_paths_after_date,
                &[&format!("{y:04}-{m:02}-{d:02}")],
            )
        }
        Lock::Preimage => EN.wallet_paths_preimage.to_string(),
    }
}

/// When a path can spend: `Any time`, or its waits.
pub(crate) fn when_text(path: &SpendPath) -> String {
    if path.locks.is_empty() {
        return EN.wallet_paths_any_time.to_string();
    }
    path.locks
        .iter()
        .map(|l| lock_text(*l))
        .collect::<Vec<_>>()
        .join(" · ")
}

/// The card's group: a row per path, its keys and when, and then the
/// margin. Empty where no path has a relative wait.
pub(crate) fn rows(spend: &Waits) -> Vec<(String, String)> {
    let Some(shortest) = spend.shortest else {
        return Vec::new();
    };
    let mut rows: Vec<(String, String)> = spend
        .paths
        .iter()
        .map(|p| (keys_text(&p.keys), when_text(p)))
        .collect();
    rows.push((
        EN.wallet_paths_move_within.to_string(),
        margin_text(shortest),
    ));
    rows
}

/// The margin: `about 328 days of arriving`; a wait in 512-second units
/// is in days without the "about".
pub(crate) fn margin_text(s: Shortest) -> String {
    let template = match s.lock {
        Lock::Blocks(_) => EN.wallet_paths_within_about,
        _ => EN.wallet_paths_within,
    };
    fill(template, &[&s.margin.to_string()])
}

/// The sheet file's lines for a wallet (`docs/BACKUP-FLOW.md` §18.4): a
/// `path` and its `wait` for each path, and for a wallet with a
/// relative wait the `unmoved` days, blocks (`-` for 512-second units)
/// and margin, and the keys that can `alone` spend. Nothing for a wallet
/// with no relative wait.
pub(crate) fn sheet_lines(spend: &Waits) -> String {
    let Some(s) = spend.shortest else {
        return String::new();
    };
    let mut out = String::new();
    for p in &spend.paths {
        out.push_str(&format!(
            "path: {}\nwait: {}\n",
            keys_text(&p.keys),
            when_text(p)
        ));
    }
    let blocks = match s.lock {
        Lock::Blocks(n) => n.to_string(),
        _ => "-".to_string(),
    };
    out.push_str(&format!("unmoved: {} {blocks} {}\n", s.days, s.margin));
    out.push_str(&format!(
        "alone: {}\n",
        keys_text(&spend.paths[s.path].keys)
    ));
    out
}
