//! The pane of the nonce check as the flow's one card: the file or scan
//! it opened from, then All inputs and a row per input, the chosen one
//! marked (`nonce_flow`). Public facts only, read from what the check
//! worked out when it opened. With nothing to check, the ways in.

use super::nonce_flow::InputRow;
use crate::flow::PickGroup;
use crate::pick::PickRow;
use crate::strings::EN;
use crate::wallet::Rest;
use crate::{Action, Faraday, Screen};
use faraday_draw::widgets::Icon;

/// "N signatures", or "1 signature".
fn count_line(n: usize) -> String {
    if n == 1 {
        EN.nonce_pane_one_signature.to_string()
    } else {
        EN.nonce_pane_signatures.replacen("{}", &n.to_string(), 1)
    }
}

/// The inputs the pane lists, as the card's rows are capped.
const SHOWN: usize = 12;

/// What a warning at the card's top says of input `i`, in a few words: the
/// input does not match, its nonce is reused, its nonce was used before. A
/// warning wins over the signature's own verdict.
fn warning(check: &crate::wallet::NonceCheck, i: usize) -> Option<&'static str> {
    if check.mismatched.contains(&i) {
        Some(EN.nonce_pane_mismatch)
    } else if check.reused.iter().any(|r| r.inputs.contains(&i)) {
        Some(EN.nonce_pane_reused)
    } else if check.used_before.iter().any(|u| u.inputs.contains(&i)) {
        Some(EN.nonce_pane_used_before)
    } else {
        None
    }
}

/// The pane's row of one input: its verdict, or the warning that names it,
/// in the error role when either is wrong; marked when it is the one chosen.
fn input_row(r: &InputRow, warned: Option<&str>, chosen: bool) -> PickRow {
    let line = match &r.verdict {
        _ if warned.is_some() => EN
            .nonce_pane_line
            .replacen("{}", &count_line(r.signatures), 1)
            .replacen("{}", warned.unwrap_or_default(), 1),
        Some(said) if r.signatures > 0 => EN
            .nonce_pane_line
            .replacen("{}", &count_line(r.signatures), 1)
            .replacen("{}", said, 1),
        _ => EN.nonce_pane_no_signature.to_string(),
    };
    let mut row = PickRow::pick(
        Icon::Hashtag,
        EN.nonce_check_input.replacen("{}", &r.input.to_string(), 1),
        line,
        chosen,
        Action::NonceCheck(super::CheckPress::Input(Some(r.input))),
    );
    row.error = r.wrong || warned.is_some();
    row
}

/// The second line of the last row: none wrong, one wrong, or nothing
/// where an input is not known.
fn last_row_line(state: Rest) -> &'static str {
    match state {
        Rest::Fine => EN.nonce_check_none_wrong.trim_start_matches("· "),
        Rest::Wrong => EN.nonce_check_one_wrong.trim_start_matches("· "),
        Rest::Unknown => "",
    }
}

/// The row standing for the `n` inputs past [`SHOWN`]: "none wrong" and
/// dimmed when all are fine, "one is wrong" in the error role when one is,
/// and no second line, not dimmed, where one is not known.
fn last_row(n: usize, state: Rest, action: Action) -> PickRow {
    let head = EN
        .nonce_check_more
        .replacen("{}", &n.to_string(), 1)
        .replacen("{}", "", 1);
    let mut row = PickRow::way_in(Icon::Hashtag, head.trim(), action);
    row.line = last_row_line(state).to_string();
    row.dimmed = state == Rest::Fine;
    row.error = state == Rest::Wrong;
    row
}

/// With nothing to check: the files in Files to check rather than sign,
/// the camera and Files. A lookup per master a file's origins name.
fn ways_in(app: &Faraday) -> Vec<PickGroup> {
    crate::pick::Decl::new(EN.decode_choose_transaction)
        .files(app, |k, _| {
            app.checked_not_signed(k).then(|| {
                crate::pick::Take::of(false, Action::NonceCheck(super::CheckPress::Inbox(k)))
            })
        })
        .ways(vec![
            PickRow::way_in(Icon::Scan, EN.compact_scan, Action::Scan),
            PickRow::way_in(Icon::File, EN.parts_open_files, Action::Nav(Screen::Files)),
        ])
        .groups()
}

/// The pane's groups. Costs a line per input shown, at most [`SHOWN`].
pub(crate) fn groups(app: &Faraday) -> Vec<PickGroup> {
    let Some(o) = app.decode.outside() else {
        return if app.decode.nonce_waits() {
            ways_in(app)
        } else {
            Vec::new()
        };
    };
    let press = |input| Action::NonceCheck(super::CheckPress::Input(input));
    // It names where the check came from; a press on it chooses All inputs.
    let file = PickRow::way_in(Icon::File, &o.source, press(None));
    let total: usize = o.rows.iter().map(|r| r.signatures).sum();
    let mut rows = vec![PickRow::pick(
        Icon::Checklist,
        EN.nonce_pane_all.to_string(),
        count_line(total),
        o.input.is_none(),
        press(None),
    )];
    for r in o.rows.iter().take(SHOWN) {
        let warned = warning(&o.check, r.input);
        rows.push(input_row(r, warned, o.input == Some(r.input)));
    }
    if o.rows.len() > SHOWN {
        let rest = &o.rows[SHOWN..];
        let state = o.check.rest(rest.iter().map(|r| r.input));
        let row = last_row(rest.len(), state, press(None));
        rows.push(row);
    }
    vec![
        PickGroup {
            head: EN.nonce_pane_file,
            rows: vec![file],
        },
        PickGroup {
            head: EN.nonce_pane_inputs,
            rows,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pick::Kind;

    fn input(wrong: bool) -> InputRow {
        InputRow {
            input: 2,
            signatures: 1,
            verdict: None,
            wrong,
        }
    }

    #[test]
    fn a_row_a_warning_names_reads_as_an_error() {
        let row = input_row(&input(false), Some(EN.nonce_pane_reused), false);
        assert!(row.error);
        assert!(row.line.contains(EN.nonce_pane_reused));
        assert!(!input_row(&input(false), None, false).error);
        assert!(input_row(&input(true), None, false).error);
    }

    #[test]
    fn the_last_row_reads_as_what_the_rest_is() {
        let press = || Action::NonceCheck(super::super::CheckPress::Input(None));
        let fine = last_row(2, Rest::Fine, press());
        assert!(fine.dimmed && !fine.error);
        assert_eq!(
            fine.line,
            EN.nonce_check_none_wrong.trim_start_matches("· ")
        );
        let wrong = last_row(2, Rest::Wrong, press());
        assert!(wrong.error && !wrong.dimmed);
        assert_eq!(
            wrong.line,
            EN.nonce_check_one_wrong.trim_start_matches("· ")
        );
        let unknown = last_row(2, Rest::Unknown, press());
        assert!(!unknown.error && !unknown.dimmed && unknown.line.is_empty());
    }

    #[test]
    fn the_input_chosen_is_the_row_marked() {
        let on = input_row(&input(false), None, true);
        let off = input_row(&input(false), None, false);
        assert_eq!(on.kind, Kind::Pick { chosen: true });
        assert_eq!(off.kind, Kind::Pick { chosen: false });
    }
}
