//! The board's lines of words: a list chip's count, a form's name on a
//! chip, the sheet's line over a drop that let a place see the balance,
//! the line naming where the description is short, and the software's
//! name (`docs/BACKUP-FLOW.md` §17).

use super::*;

/// "2 places", "1 place", or nothing.
pub(super) fn count_text(n: usize) -> String {
    match n {
        0 => String::new(),
        1 => EN.board_one_place.to_string(),
        n => fill(EN.board_places, &[&n.to_string()]),
    }
}

/// A form's name on a chip, or `None` where the zone has no other form
/// to tell it from.
pub(super) fn form_label(w: What, software: &str) -> Option<String> {
    Some(match w {
        What::Words(_) => EN.board_words.to_string(),
        What::SeedQr(_) => EN.board_seedqr.to_string(),
        What::SeedFile(_) => EN.board_words_file.to_string(),
        What::SeedQrFile(_) => EN.board_seedqr_file.to_string(),
        What::PassphraseFile(_) => EN.board_passphrase_file.to_string(),
        What::WalletFile(f) | What::ShareFile(_, f) if f == form::TEXT => {
            EN.board_text_file.to_string()
        }
        What::WalletFile(_) | What::ShareFile(..) => EN.board_qr_picture.to_string(),
        What::SoftwareFile => fill(EN.board_software_file, &[software]),
        // A codex32 string's forms (`docs/CODEX32.md` §9.5).
        What::Codex32Share(..) => EN.codex32_string.to_string(),
        What::Codex32QrPaper(..) => EN.codex32_codex32qr.to_string(),
        What::Codex32ShareFile(..) => EN.board_text_file.to_string(),
        What::Codex32QrFile(..) => EN.board_qr_picture.to_string(),
        _ => return None,
    })
}

/// The line on the sheet over a drop that let a place see the balance
/// (`docs/BACKUP-FLOW.md` §17.5): what kept there, `held`, makes the
/// description. The whole description; else its shares there, with the
/// seeds there whose keys those shares left off.
pub(super) fn sees_line(shape: &Shape, a: &Answers, held: &[What]) -> String {
    let whole = held.iter().any(|w| {
        matches!(
            w,
            What::Sheet | What::Wallet | What::WalletFile(_) | What::SoftwareFile
        )
    });
    if whole {
        return EN.board_sees_whole.to_string();
    }
    let mut shares: Vec<usize> = held
        .iter()
        .filter_map(|w| match w {
            What::Share(j) | What::ShareFile(j, _) => Some(*j),
            _ => None,
        })
        .collect();
    shares.sort_unstable();
    shares.dedup();
    let split = crate::wallet_backup::split_plan(shape.keys, shape.m, a.omit);
    let carried = |k: usize| {
        shares
            .iter()
            .any(|&j| split.get(j).is_some_and(|row| row.contains(&k)))
    };
    let mut seeds: Vec<usize> = held
        .iter()
        .filter_map(|w| match w {
            What::Words(i)
            | What::SeedQr(i)
            | What::Seed(i)
            | What::SeedPassphrase(i)
            | What::SeedFile(i)
            | What::SeedQrFile(i) => Some(*i),
            _ => None,
        })
        .filter(|&i| !carried(i))
        .collect();
    seeds.sort_unstable();
    seeds.dedup();
    let listed = |one: &str, many: &str, names: Vec<String>| -> Option<String> {
        match names.as_slice() {
            [] => None,
            [n] => Some(fill(one, &[n])),
            [rest @ .., last] => Some(fill(many, &[&rest.join(", "), last])),
        }
    };
    let share_part = listed(
        EN.board_sees_share,
        EN.board_sees_shares,
        shares.iter().map(|j| (j + 1).to_string()).collect(),
    );
    let seed_part = listed(
        EN.board_sees_seed,
        EN.board_sees_seeds,
        seeds
            .iter()
            .map(|&i| {
                shape
                    .seeds
                    .get(i)
                    .map_or_else(String::new, |s| s.name.clone())
            })
            .collect(),
    );
    let count = shares.len() + seeds.len();
    let what = match (share_part, seed_part) {
        (Some(a), Some(b)) => fill(EN.board_sees_both, &[&a, &b]),
        (Some(a), None) | (None, Some(a)) => a,
        (None, None) => return EN.board_sees_whole.to_string(),
    };
    if count == 1 {
        fill(EN.board_sees_alone, &[&what])
    } else {
        fill(EN.board_sees_together, &[&what])
    }
}

/// The line under the board when nothing kept puts the description back
/// together (`docs/BACKUP-FLOW.md` §17.19 item 5): what is kept, and what
/// would complete it.
pub(super) fn description_line(shape: &Shape, m: &plan::Mapped) -> String {
    let kept = plan::shares_kept_on(shape, m);
    if kept.is_empty() {
        return EN.board_desc_nothing.to_string();
    }
    let held: Vec<String> = kept
        .iter()
        .map(|&(j, n)| match n {
            1 => fill(EN.board_sees_share, &[&(j + 1).to_string()]),
            2 => fill(EN.board_share_twice, &[&(j + 1).to_string()]),
            n => fill(
                EN.board_share_times,
                &[&(j + 1).to_string(), &n.to_string()],
            ),
        })
        .collect();
    let joined = |v: &[String]| match v {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => fill(EN.board_sees_both, &[&rest.join(", "), last]),
    };
    let missing: Vec<String> = (0..shape.keys)
        .filter(|j| !kept.iter().any(|(k, _)| k == j))
        .map(|j| (j + 1).to_string())
        .collect();
    let add = match missing.as_slice() {
        [] => return fill(EN.board_desc_kept_add, &[&joined(&held)]),
        [one] => fill(EN.board_sees_share, &[one]),
        [rest @ .., last] => fill(EN.board_sees_shares, &[&rest.join(", "), last]),
    };
    fill(EN.board_desc_missing, &[&joined(&held), &add])
}

/// The software the plan names: the first chosen, or "Watch-only
/// software" when it is none or "Not sure". The software place's title,
/// and the name of its own file's form.
pub(super) fn software_name(a: &Answers) -> String {
    a.software
        .iter()
        .position(|&on| on)
        .filter(|&k| k != plan::software::NOT_SURE)
        .and_then(|k| crate::screens::SOFTWARE_ROWS.get(k))
        .map_or_else(|| EN.board_watch_only.to_string(), |s| (*s).to_string())
}

/// Whether the board offers partial descriptions: `Some(None)` where the
/// description splits; for a multisig whose description does not
/// (`multi`, `multi_a`, another internal key, two keys of one seed), the
/// line saying why; none after the lock or for any other wallet. A
/// lookup.
pub(super) fn partials(
    app: &Faraday,
    of: BoardOf,
    splits: bool,
    locked: bool,
) -> Option<Option<&'static str>> {
    if locked {
        return None;
    }
    if splits {
        return Some(None);
    }
    let w = match of {
        BoardOf::Backup => app.session.wallets.get(app.backup.state()?.wallet)?,
        BoardOf::Flow(i) => {
            let sum = app.flows[i].wallet.as_ref()?.checksum.as_deref()?;
            app.session.wallets.iter().find(|w| w.checksum() == sum)?
        }
    };
    let multisig = matches!(
        crate::wallet::Kind::of(w.policy()),
        crate::wallet::Kind::Multi(_) | crate::wallet::Kind::TapMulti
    );
    multisig.then_some(Some(if crate::restore::repeats_a_seed(w.policy()) {
        EN.board_partials_one_seed
    } else {
        EN.board_partials_not_offered
    }))
}
