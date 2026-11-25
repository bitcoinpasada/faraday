//! The board's lines of words: a list chip's count, a form's name on a
//! chip, the sheet's line over a drop that let a place see the balance,
//! the line naming where the description is short, and the software's
//! name (`docs/BACKUP-FLOW.md` §17).

use super::*;
use crate::plan::Written;

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
        What::Written(c) => written_label(c)?.to_string(),
        What::PassphraseFile(_) => EN.board_passphrase_file.to_string(),
        What::WalletFile(f) | What::ShareFile(_, f) if f == form::TEXT => {
            EN.board_text_file.to_string()
        }
        What::WalletFile(_) | What::ShareFile(..) => EN.board_qr_picture.to_string(),
        What::SoftwareFile => fill(EN.board_software_file, &[software]),
        _ => return None,
    })
}

/// A copy's name on a chip: a seed's words, SeedQR or file, a codex32
/// string's forms (`docs/CODEX32.md` §9.5), a Krux code on a stick or
/// drawn on paper (`docs/KEF.md` §15.1). A Seed XOR part and a SLIP-39
/// share have none: they are the only form in their zone.
fn written_label(c: Written) -> Option<&'static str> {
    match c.form {
        KeyForm::Words => match (c.writing, c.medium) {
            (Writing::Words, Medium::Paper) => Some(EN.board_words),
            (Writing::SeedQr, Medium::Paper) => Some(EN.board_seedqr),
            (Writing::Words, Medium::File) => Some(EN.board_words_file),
            (Writing::SeedQr, Medium::File) => Some(EN.board_seedqr_file),
            _ => None,
        },
        KeyForm::Codex32 => match (c.writing, c.medium) {
            (Writing::Codex32String, Medium::Paper) => Some(EN.codex32_string),
            (Writing::Codex32Qr, Medium::Paper) => Some(EN.codex32_codex32qr),
            (Writing::Codex32String, Medium::File) => Some(EN.board_text_file),
            (Writing::Codex32Qr, Medium::File) => Some(EN.board_qr_picture),
            _ => None,
        },
        KeyForm::Krux => Some(EN.board_kef_code),
        KeyForm::Xor | KeyForm::Slip39 | KeyForm::Xprv | KeyForm::HexSeed => None,
    }
}

/// The line on the sheet over a drop that let a place see the balance
/// (`docs/BACKUP-FLOW.md` §17.5): what kept there, `held`, makes the
/// description. The whole description, else the shares there; a seed kept
/// beside a share is not part of it, since Restore does not put the two
/// together.
pub(super) fn sees_line(held: &[What]) -> String {
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
    let what = match shares.as_slice() {
        [] => return EN.board_sees_whole.to_string(),
        [j] => fill(EN.board_sees_share, &[&(j + 1).to_string()]),
        [rest @ .., last] => fill(
            EN.board_sees_shares,
            &[
                &rest
                    .iter()
                    .map(|j| (j + 1).to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                &(last + 1).to_string(),
            ],
        ),
    };
    if shares.len() == 1 {
        fill(EN.board_sees_alone, &[&what])
    } else {
        fill(EN.board_sees_together, &[&what])
    }
}

/// The line under the board when nothing kept puts the description back
/// together (`docs/BACKUP-FLOW.md` §17.19 item 5): what is kept, and what
/// would complete it.
pub(crate) fn description_line(shape: &Shape, m: &plan::Mapped) -> String {
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
            let f = app.flows[i].wallet.as_ref()?;
            // By id where the flow keeps one: a twin of the checksum that
            // came first is another wallet.
            match f.id {
                Some(id) => app.session.wallets.iter().find(|w| w.id() == id)?,
                None => {
                    let sum = f.checksum.as_deref()?;
                    app.session.wallets.iter().find(|w| w.checksum() == sum)?
                }
            }
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

/// What the board calls each of its sticks (`docs/BACKUP-FLOW.md` §16,
/// §17.4): its pill's number, one numbering across every place, so a
/// stick has one name on the board, its menus, the checklist and its pill.
pub(super) struct StickNumbers {
    /// Stick `s` of place `p`'s number: `at[p][s]`.
    at: Vec<Vec<usize>>,
    /// The software place's stick of files.
    pub files: usize,
    /// The first number after every stick of the plan: the board's empty
    /// sticks take the numbers from here, place by place.
    next: usize,
}

impl StickNumbers {
    /// Stick `s` of place `p`.
    pub(super) fn of(&self, p: usize, s: usize) -> usize {
        self.at.get(p).and_then(|x| x.get(s)).copied().unwrap_or(0)
    }

    /// Place `p`'s empty stick `k`, the board's empty sticks `empty` by
    /// place numbered after the plan's.
    pub(super) fn empty(&self, empty: &[usize], p: usize, k: usize) -> usize {
        self.next + empty.iter().take(p).sum::<usize>() + k
    }
}

/// The pill's number of every stick of the map `m` (`docs/BACKUP-FLOW.md`
/// §16): the number the flow `f` gives its stick where the flow has a
/// stick for every one of the board's (the boot stick is 1); else from 2
/// in the order the flow asks for them, each place's vaults' sticks in
/// vault order then its sticks of files, place by place, then the stick
/// of files. A lookup per stick.
pub(super) fn stick_numbers(
    m: &[Spot],
    f: Option<&crate::backup_flow::BackupFlow>,
) -> StickNumbers {
    use crate::backup_flow::{Target, stick_target};
    let places = plan::places_of(m).max(1);
    let targets: Vec<Vec<Option<Target>>> = (0..places)
        .map(|p| {
            (0..plan::sticks_at(m, p))
                .map(|s| stick_target(m, p, s))
                .collect()
        })
        .collect();
    let mut order: Vec<Target> = targets.iter().flatten().flatten().copied().collect();
    order.sort_by_key(|t| t.key());
    let has_files = m.iter().any(|s| s.at == At::Files);
    if has_files {
        order.push(Target::Files);
    }
    let slot = |f: &crate::backup_flow::BackupFlow, t: Target| {
        f.slots
            .iter()
            .position(|x| x.target == t)
            .map(|k| f.stick_number(k))
    };
    let f = f.filter(|f| order.iter().all(|&t| slot(f, t).is_some()));
    let number = |t: Target| match f {
        Some(f) => slot(f, t).unwrap_or(0),
        None => 2 + order.iter().position(|&x| x == t).unwrap_or(order.len()),
    };
    let at: Vec<Vec<usize>> = targets
        .iter()
        .map(|ts| ts.iter().map(|t| t.map_or(0, number)).collect())
        .collect();
    let files = number(Target::Files);
    let last = match f {
        Some(f) => 1 + f.own_sticks(),
        None => 1 + order.len(),
    };
    StickNumbers {
        at,
        files,
        next: last + 1,
    }
}

/// "Stick {}", or "Card or stick {}" where the medium is a card or stick
/// (`docs/BACKUP-FLOW.md` §17.17 item 4).
pub(crate) fn stick_word(medium: crate::Medium) -> &'static str {
    match medium {
        crate::Medium::Stick => EN.board_stick,
        crate::Medium::SdCard => EN.board_card_or_stick,
    }
}
