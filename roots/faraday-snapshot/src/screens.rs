//! The screens check (07 §5.3): at the frame after a lock, a hide, a page left and a sheet
//! closed, the frame's text is read against every text form of the planted test secrets, and a
//! kept image that still holds a frame which showed one is a hit.

use faraday_runtime::Mark;
use faraday_testkit::test_seeds;

use crate::runner::Look;

/// The text forms of the planted test secrets: each seed's phrase, and each of its words as a
/// text of its own (a word-list cell).
fn forms() -> Vec<(String, bool)> {
    let mut forms = Vec::new();
    for seed in test_seeds() {
        forms.push((String::from(seed.words), false));
        for word in seed.words.split_whitespace() {
            let form = (String::from(word), true);
            if !forms.contains(&form) {
                forms.push(form);
            }
        }
    }
    forms
}

/// Whether `now` is the frame after a lock, a hide, a page left or a sheet closed (TS17):
/// the runtime's marks, and a change of page or sheets between two frames.
#[must_use]
pub fn is_event(before: Option<&Look>, now: &Look) -> bool {
    let marked = now.marks.iter().any(|m| {
        matches!(
            m,
            Mark::LockBegun | Mark::Hidden | Mark::Hides | Mark::ModelBuilt
        )
    });
    let Some(before) = before else {
        return marked;
    };
    marked
        || before.page != now.page
        || before.sheets.len() > now.sheets.len()
        || before.sheets.iter().any(|s| !now.sheets.contains(s))
}

/// Looks for a secret the screen must not show; the step's name is for the failure.
pub fn check(before: Option<&Look>, now: &Look, step: &str) -> Result<(), String> {
    if !is_event(before, now) {
        return Ok(());
    }
    let forms = forms();
    for (text, _) in &now.texts {
        let flat = text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        for (form, whole) in &forms {
            let hit = if *whole {
                flat == *form
            } else {
                flat.contains(form.as_str())
            };
            if hit {
                return Err(format!(
                    "step {step}: a planted secret is drawn after the event: {form}"
                ));
            }
        }
    }
    if now.kept.iter().any(|k| k.secret) {
        return Err(format!(
            "step {step}: a kept image of a frame that showed a secret is still kept after the event"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::KeptLook;

    fn showing(text: &str, page: u32) -> Look {
        Look {
            texts: vec![(String::from(text), false)],
            page: (String::from("seed"), page),
            ..Look::default()
        }
    }

    /// Invariant: A planted secret's words still drawn at the frame after a page is left fail the run.
    /// Regression: The check reads no frame, or the frame before the event, so a page that keeps a secret on leaving passes.
    /// Owner: root
    #[test]
    fn a_secret_still_drawn_after_a_page_is_left_fails() {
        let words = test_seeds()[0].words;
        let before = showing(words, 1);
        let after = showing(words, 2);
        assert!(check(Some(&before), &after, "left").is_err());
        let clean = showing("Home", 2);
        assert!(check(Some(&before), &clean, "left").is_ok());
    }

    /// Invariant: The check reads a frame only after an event it knows: the same secret on a frame with no lock, hide or change of page is not a hit.
    /// Regression: The check fails every frame that shows a secret, so a seed on show fails the tour that shows it.
    /// Owner: root
    #[test]
    fn a_secret_on_show_with_no_event_is_not_a_hit() {
        let words = test_seeds()[0].words;
        let a = showing(words, 1);
        let b = showing(words, 1);
        assert!(check(Some(&a), &b, "shown").is_ok());
    }

    /// Invariant: A hiding press's frame is read: a secret still drawn after it, or a kept image that held one, fails.
    /// Regression: The hide mark is ignored, so an eye that closes but leaves the words or a frosted copy passes.
    /// Owner: root
    #[test]
    fn a_hide_that_leaves_a_secret_or_a_kept_image_fails() {
        let words = test_seeds()[1].words;
        let mut still = showing(words, 1);
        still.marks = vec![Mark::Hides];
        assert!(check(None, &still, "hide").is_err());
        let mut kept = showing("Home", 1);
        kept.marks = vec![Mark::Hides];
        kept.kept = vec![KeptLook { secret: true }];
        assert!(check(None, &kept, "hide").is_err());
    }
}
