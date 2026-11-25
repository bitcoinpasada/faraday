//! The nonce check of a transaction brought in, as the flow's one card
//! (`docs/FLOW-CONTAINER.md` §2): the file it opened from and one row per
//! input at the left, the check as the card. On a small panel the page
//! draws as before.

use crate::flow::{Contained, Pick, Row};
use crate::strings::EN;
use crate::trail::{StepFlow, Steps};
use crate::wallet::NonceRow;
use crate::{Action, Faraday, Screen};
use faraday_text::Text;

/// The check's one step.
pub(crate) const CHECK: u8 = 0;

/// What the pane says of one input: its number, how many signatures it
/// has and its verdict, worked out once when the check opens.
#[derive(Debug, Clone, PartialEq)]
pub struct InputRow {
    /// The input's number, as the card's rows name it.
    pub input: usize,
    /// How many signatures the check found on it.
    pub signatures: usize,
    /// Its verdict in a few words: the card's own for the first wrong
    /// signature, else the first not checked (a hash preimage is not one),
    /// else the first; none when it has no signature.
    pub verdict: Option<Text>,
    /// The signature the verdict is of is wrong.
    pub wrong: bool,
}

/// How much a signature's row matters to its input's verdict: wrong, then
/// not checked, then any.
fn rank(r: &NonceRow) -> u8 {
    if r.wrong() {
        2
    } else if matches!(r.verdict, faraday_psbt::verify::Verdict::Unchecked(_)) && !r.preimage {
        1
    } else {
        0
    }
}

/// Costs one pass over the check's rows. One row per input of a
/// transaction of `inputs` inputs, from the check's rows (`NonceCheck::rows`).
pub(crate) fn input_rows(rows: &[NonceRow], inputs: usize) -> Vec<InputRow> {
    let mut best: Vec<Option<&NonceRow>> = vec![None; inputs];
    let mut counts = vec![0usize; inputs];
    for r in rows {
        let Some(slot) = best.get_mut(r.input) else {
            continue;
        };
        counts[r.input] += 1;
        if slot.is_none_or(|b| rank(r) > rank(b)) {
            *slot = Some(r);
        }
    }
    best.into_iter()
        .zip(counts)
        .enumerate()
        .map(|(i, (shown, signatures))| InputRow {
            input: i,
            signatures,
            verdict: shown.map(|r| crate::screens::row_verdict(r).0),
            wrong: shown.is_some_and(|r| r.wrong()),
        })
        .collect()
}

/// Whether the nonce check is drawn as the flow: the one place every site
/// asks. A small panel draws the page.
pub(crate) fn contained_check(app: &Faraday) -> bool {
    app.screen == Screen::NonceCheck && !app.compact && app.decode.outside().is_some()
}

/// The check's step, plugged into Back and Your choices.
pub(crate) struct NonceSteps;

/// The one [`NonceSteps`].
pub(crate) static NONCE_STEPS: NonceSteps = NonceSteps;

impl StepFlow for NonceSteps {
    fn steps(&self, _: &Faraday) -> Option<Steps> {
        Some(Steps::new(vec![CHECK], Some(CHECK)))
    }

    fn open(&self, _: &Faraday, _: u8) -> Action {
        Action::Nav(Screen::NonceCheck)
    }

    fn name(&self, _: &Faraday, _: u8) -> &'static str {
        EN.nonce_screen_title
    }

    fn contained(&self) -> Option<&dyn Contained> {
        Some(self)
    }
}

impl Contained for NonceSteps {
    fn rows(&self, app: &Faraday) -> Vec<Row> {
        let Some(o) = app.decode.outside() else {
            return Vec::new();
        };
        vec![Row {
            label: Text::of(EN.nonce_screen_title),
            value: Some(o.source.clone()),
            mono: false,
            address: false,
            opens: CHECK,
            current: false,
            mark: None,
        }]
    }

    fn hide_shown(&self, _: &mut Faraday) {
        // The hex is public: no step shows a secret.
    }

    fn count_fixed(&self, _: &Faraday) -> bool {
        true
    }

    /// The only step in the order stands without a count.
    fn heads(&self, _: &Faraday) -> usize {
        1
    }

    fn pick(&self, app: &Faraday, _: u8) -> Option<Pick> {
        Some(Pick {
            question: EN.nonce_pane_question,
            groups: super::nonce_pane::groups(app),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_psbt::verify::{Unchecked, Verdict};

    fn row(input: usize, verdict: Verdict, preimage: bool) -> NonceRow {
        NonceRow {
            input,
            name: Text::of("abcd"),
            verdict,
            preimage,
            recomputed: None,
            unmatched: None,
            entry: None,
        }
    }

    fn said(rows: &[NonceRow]) -> (Option<Text>, bool) {
        let r = input_rows(rows, 1).remove(0);
        (r.verdict, r.wrong)
    }

    fn words(r: &NonceRow) -> Text {
        crate::screens::row_verdict(r).0
    }

    #[test]
    fn a_wrong_signature_is_the_inputs_verdict() {
        let (ok, unchecked, bad) = (
            row(0, Verdict::Valid, false),
            row(0, Verdict::Unchecked(Unchecked::UnknownKey), false),
            row(0, Verdict::Invalid, false),
        );
        let (got, wrong) = said(&[ok.clone(), unchecked.clone(), bad.clone()]);
        assert_eq!(got, Some(words(&bad)));
        assert!(wrong);
    }

    #[test]
    fn a_signature_not_checked_is_not_said_valid() {
        let (ok, unchecked) = (
            row(0, Verdict::Valid, false),
            row(0, Verdict::Unchecked(Unchecked::UnknownKey), false),
        );
        let (got, wrong) = said(&[ok, unchecked.clone()]);
        assert_eq!(got, Some(words(&unchecked)));
        assert!(!wrong);
    }

    #[test]
    fn a_preimage_does_not_stand_for_a_signature_not_checked() {
        let (ok, preimage) = (
            row(0, Verdict::Valid, false),
            row(0, Verdict::Unchecked(Unchecked::UnsupportedScript), true),
        );
        let (got, _) = said(&[ok.clone(), preimage]);
        assert_eq!(got, Some(words(&ok)));
    }

    #[test]
    fn otherwise_the_first_says_it() {
        let (a, b) = (
            row(0, Verdict::Valid, false),
            row(0, Verdict::Unchecked(Unchecked::UnsupportedScript), true),
        );
        assert_eq!(said(&[a.clone(), b.clone()]).0, Some(words(&a)));
        assert_eq!(said(&[b.clone(), a]).0, Some(words(&b)));
        assert_eq!(said(&[]).0, None);
    }
}
