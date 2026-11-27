use faraday_kernel::{Carried, Ctx, Effects, Feature, Notice, Public, View};

use crate::{Home, Msg, Out, Press, State};

/// Why a message is refused before `update` sees it. Home refuses nothing: every press on it
/// opens something or edits the search.
#[derive(Public)]
pub enum Refusal {}

/// The places in the glossary that hold every word typed; none for nothing typed.
fn found(find: &str) -> Vec<u32> {
    if find.split_whitespace().next().is_none() {
        return Vec::new();
    }
    faraday_guide::glossary()
        .iter()
        .enumerate()
        .filter(|(_, word)| words_in(word.word, find))
        .filter_map(|(at, _)| u32::try_from(at).ok())
        .collect()
}

/// Letters, digits and spaces, lower-cased: "bip85" and "bip-85" search alike.
fn fold(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn words_in(text: &str, find: &str) -> bool {
    let hay = fold(text);
    find.split_whitespace()
        .all(|word| hay.contains(&fold(word)))
}

impl Feature for Home {
    type State = State;
    type Msg = Msg;
    type Out = Out;
    type Refusal = Refusal;

    fn start(_carried: &mut Carried, _cx: &Ctx<'_>) -> (State, Effects<Msg, Out>) {
        (State::new(), Effects::none())
    }

    fn update(state: &mut State, msg: Msg, _cx: &Ctx<'_>) -> Effects<Msg, Out> {
        match msg {
            Msg::Press(Press::Focus) => {
                state.focus();
                Effects::none()
            }
            Msg::Press(Press::Clear) => {
                state.clear();
                Effects::none()
            }
            Msg::Press(Press::Make) => Effects::none().out(Out::CreateWallet),
            Msg::Press(Press::Restore) => Effects::none().out(Out::RestoreWallet),
            Msg::Press(Press::SpendBackup) => Effects::none().out(Out::SpendBackup),
            Msg::Press(Press::Word(at)) => Effects::none().out(Out::GuideWord(at)),
            Msg::Typed(c) => {
                state.type_char(c);
                state.set_found(found(state.typed()));
                Effects::none()
            }
            Msg::Erase => {
                state.erase();
                state.set_found(found(state.typed()));
                Effects::none()
            }
        }
    }

    /// Nothing on Home is a secret or needs a screen: every notice leaves it as it is. The
    /// words typed stay while a result is open and when Home is left.
    fn notice(_state: &mut State, _notice: &Notice<'_>, _cx: &Ctx<'_>) -> Effects<Msg, Out> {
        Effects::none()
    }

    fn refuses(_state: &State, _msg: &Msg, _view: &View<'_>) -> Option<Refusal> {
        None
    }
}

#[cfg(test)]
mod tests {
    use faraday_app_test::{TestView, seen};
    use faraday_kernel::route;

    use super::*;

    fn press(state: &mut State, press: Press) -> faraday_app_test::Seen<Msg, Out> {
        let view = TestView::device();
        seen(route::<Home>(state, Msg::Press(press), &view.ctx()))
    }

    fn type_in(state: &mut State, text: &str) {
        let view = TestView::device();
        for c in text.chars() {
            let _ = route::<Home>(state, Msg::Typed(c), &view.ctx());
        }
    }

    /// Invariant: Each of Home's three first actions begins its own flow and nothing else.
    /// Regression: Restore or Spend from a backup is wired to Make a new wallet, so a person who asks for one starts another.
    /// Owner: app
    #[test]
    fn each_first_action_begins_its_own_flow() {
        let mut state = State::new();
        let out = press(&mut state, Press::Make).outs;
        assert!(matches!(out.as_slice(), [Out::CreateWallet]));
        let out = press(&mut state, Press::Restore).outs;
        assert!(matches!(out.as_slice(), [Out::RestoreWallet]));
        let out = press(&mut state, Press::SpendBackup).outs;
        assert!(matches!(out.as_slice(), [Out::SpendBackup]));
    }

    /// Invariant: A word typed in the search shows the guide words that hold it, and a word that matches none shows none.
    /// Regression: The search lists every guide word whatever is typed, or lists none.
    /// Owner: app
    #[test]
    fn a_typed_word_finds_its_guide_words_and_a_non_match_finds_none() {
        let first = faraday_guide::glossary()[0].word;
        let mut state = State::new();
        type_in(&mut state, first);
        assert!(state.found().contains(&0));
        assert!(state.found().len() < faraday_guide::glossary().len());

        let mut state = State::new();
        type_in(&mut state, "qzxwvk");
        assert!(state.found().is_empty());
    }

    /// Invariant: With nothing typed the search finds nothing; clearing it, or erasing what was typed, takes the words found away with it.
    /// Regression: The words found stay on Home after the search is cleared.
    /// Owner: app
    #[test]
    fn clearing_or_erasing_the_search_takes_its_results_away() {
        let first = faraday_guide::glossary()[0].word;
        let mut state = State::new();
        type_in(&mut state, first);
        assert!(!state.found().is_empty());
        let _ = press(&mut state, Press::Clear);
        assert!(state.found().is_empty());
        assert!(state.typed().is_empty());

        let view = TestView::device();
        type_in(&mut state, "q");
        let _ = route::<Home>(&mut state, Msg::Erase, &view.ctx());
        assert!(state.found().is_empty());
        assert!(state.typed().is_empty());
    }

    /// Invariant: Pressing a guide word found by the search opens the guide at that word and keeps the words typed for when the guide is closed.
    /// Regression: The press opens the guide at the first word, or clears the search first.
    /// Owner: app
    #[test]
    fn a_found_word_opens_the_guide_at_it_and_keeps_the_search() {
        let first = faraday_guide::glossary()[0].word;
        let mut state = State::new();
        type_in(&mut state, first);
        let at = *state.found().last().expect("a found word");
        let out = press(&mut state, Press::Word(at)).outs;
        assert!(matches!(out.as_slice(), [Out::GuideWord(w)] if *w == at));
        assert_eq!(state.typed(), first);
    }
}
