//! The pane of the nonce check as the flow's one card: the file or scan
//! it opened from, then All inputs and a row per input, the chosen one
//! marked (`nonce_flow`). Public facts only, read from what the check
//! worked out when it opened.

use super::nonce_flow::InputRow;
use crate::flow::PickGroup;
use crate::pick::PickRow;
use crate::strings::EN;
use crate::{Action, Faraday};
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
/// input does not match, its nonce is reused, its key was used before. A
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

/// The pane's groups. Costs a line per input shown, at most [`SHOWN`].
pub(crate) fn groups(app: &Faraday) -> Vec<PickGroup> {
    let Some(o) = app.decode.outside() else {
        return Vec::new();
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
        let ok = !rest
            .iter()
            .any(|r| r.wrong || warning(&o.check, r.input).is_some());
        let words = if ok {
            EN.nonce_check_none_wrong
        } else {
            EN.nonce_check_one_wrong
        };
        let head = EN
            .nonce_check_more
            .replacen("{}", &rest.len().to_string(), 1)
            .replacen("{}", "", 1);
        let mut row = PickRow::way_in(Icon::Hashtag, head.trim(), press(None));
        // The words are the second line, so a wrong one is read in the error role.
        row.line = words.trim_start_matches("· ").to_string();
        row.dimmed = ok;
        row.error = !ok;
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
    fn the_input_chosen_is_the_row_marked() {
        let on = input_row(&input(false), None, true);
        let off = input_row(&input(false), None, false);
        assert_eq!(on.kind, Kind::Pick { chosen: true });
        assert_eq!(off.kind, Kind::Pick { chosen: false });
    }
}
