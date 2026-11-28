use faraday_guide::glossary;
use faraday_kernel::{Carried, Ctx, Effects, Feature, FrameSheet, Notice, Public, View, ViewOrder};

use crate::{GuideSheet, Msg, Out, Place, Press, SCROLL, State, TARGET};

/// Why a message is refused before `update` sees it. The guide refuses nothing: what does not
/// apply (a key typed off the contents, a control character, a Find at its limit) is left
/// unheard, as `main` did.
#[derive(Public)]
pub enum Refusal {}

type Fx = Effects<Msg, Out>;

/// The text scrolls to the top, and to the section the guide was sent to, where it has one.
fn goto(state: &State) -> Fx {
    let fx = Effects::none().view(ViewOrder::Top(SCROLL));
    if state.to().is_some() {
        fx.view(ViewOrder::Reveal(TARGET))
    } else {
        fx
    }
}

fn top() -> Fx {
    Effects::none().view(ViewOrder::Top(SCROLL))
}

/// Keeps the guide where the window and the page put it (`main`'s `guide_place`): the sheet
/// becomes the pane once the window holds one, the pane the sheet once it does not, and the
/// pane follows the page. The parent sends a follow only while no other sheet is over the page.
fn placed(state: &mut State, place: &Place) -> Fx {
    state.place(place);
    let key = place.key.as_str();
    let mut fx = Effects::none();
    if state.drop_how_if_gone() {
        fx = top();
    }
    let room = place.room;
    if state.pane_open() {
        if !room.wide() {
            state.move_to_sheet();
            return fx.and(top()).out(Out::Front(Some(FrameSheet::Guide)));
        }
        if room.place_free && state.follow(key) {
            return fx.and(goto(state));
        }
    } else if state.sheet() && room.pane() {
        state.move_to_pane(key);
        return fx.out(Out::Front(None));
    }
    fx
}

/// The guide opens at `at` as the sheet, which becomes the pane where the window holds one;
/// the pane open and shown is put away instead.
fn open(state: &mut State, place: &Place, at: Option<&str>) -> Fx {
    state.place(place);
    if state.pane_shown() {
        state.close_pane();
        return Effects::none();
    }
    state.open_at(at.unwrap_or(place.key.as_str()));
    let fx = goto(state);
    if place.room.pane() {
        state.move_to_pane(place.key.as_str());
        fx
    } else {
        fx.out(Out::Front(Some(FrameSheet::Guide)))
    }
}

fn close(state: &mut State) -> Fx {
    let was_sheet = state.sheet();
    state.close_sheet();
    state.close_pane();
    if was_sheet {
        Effects::none().out(Out::Front(None))
    } else {
        Effects::none()
    }
}

fn word(state: &mut State, place: &Place, i: u32) -> Fx {
    let Some(w) = glossary().get(i as usize) else {
        return Effects::none();
    };
    if state.open() {
        state.place(place);
        state.show_word(w.key);
        goto(state)
    } else {
        open(state, place, Some(w.key))
    }
}

impl Feature for GuideSheet {
    type State = State;
    type Msg = Msg;
    type Out = Out;
    type Refusal = Refusal;

    fn start(_carried: &mut Carried, _cx: &Ctx<'_>) -> (State, Fx) {
        (State::new(), Effects::none())
    }

    fn update(state: &mut State, msg: Msg, _cx: &Ctx<'_>) -> Fx {
        match msg {
            Msg::Open(place) => open(state, &place, None),
            Msg::OpenWord(place, i) => word(state, &place, i),
            Msg::Follow(place) if state.open() => placed(state, &place),
            Msg::Follow(_) => Effects::none(),
            Msg::Press(Press::TabGuide) => {
                state.tab_guide();
                goto(state)
            }
            Msg::Press(Press::TabHow) => {
                state.tab_how();
                top()
            }
            Msg::Press(Press::Contents | Press::ClearFind) => {
                state.show_contents();
                top()
            }
            Msg::Press(Press::Here) => {
                state.show_here();
                goto(state)
            }
            Msg::Press(Press::Page(n)) => {
                if state.show_page(n) {
                    goto(state)
                } else {
                    Effects::none()
                }
            }
            Msg::Press(Press::Word(i)) => match glossary().get(i as usize) {
                Some(w) => {
                    state.show_word(w.key);
                    goto(state)
                }
                None => Effects::none(),
            },
            Msg::Press(Press::Focus) => {
                state.set_find_focus(true);
                Effects::none()
            }
            Msg::Press(Press::Close) => close(state),
            Msg::Typed(c) => {
                if state.open() && state.contents() {
                    state.type_char(c);
                }
                Effects::none()
            }
            Msg::Erase => {
                if state.open() && state.contents() {
                    state.erase();
                }
                Effects::none()
            }
            Msg::Escape => {
                if state.open() && state.contents() && !state.find().is_empty() {
                    state.clear_find();
                } else if state.pane_open() && !state.sheet() {
                    state.close_pane();
                }
                Effects::none()
            }
            Msg::Cancel => {
                if state.open() && state.step_back() {
                    return goto(state);
                }
                close(state)
            }
        }
    }

    /// The guide keeps no secret and no screen: every notice leaves it as it is, open across
    /// pages until it is closed.
    fn notice(_state: &mut State, _notice: &Notice<'_>, _cx: &Ctx<'_>) -> Fx {
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
    use faraday_text::Text;

    use super::*;
    use crate::Room;

    fn place(key: &str, room: Room) -> Place {
        Place {
            key: Text::of(key),
            how_screen: None,
            release: Text::default(),
            room,
        }
    }

    const SMALL: Room = Room {
        extra_large: false,
        compact: false,
        held: false,
        place_free: true,
    };
    const WIDE: Room = Room {
        extra_large: true,
        ..SMALL
    };

    fn run(state: &mut State, msg: Msg) -> faraday_app_test::Seen<Msg, Out> {
        let view = TestView::device();
        seen(route::<GuideSheet>(state, msg, &view.ctx()))
    }

    fn sheet_up(out: &[Out]) -> bool {
        matches!(out, [Out::Front(Some(FrameSheet::Guide))])
    }

    fn contents(state: &mut State) {
        run(state, Msg::Open(place("nowhere", SMALL)));
        assert!(state.contents());
    }

    /// Invariant: Find takes at most 40 characters and no control character, Backspace deletes one, and Escape empties a Find with something in it.
    /// Regression: Find takes a 41st character or a tab, Backspace on an empty Find does harm, or Escape closes the guide while Find has text.
    /// Owner: app
    #[test]
    fn find_takes_forty_printing_characters_and_escape_empties_it() {
        let mut state = State::new();
        contents(&mut state);
        for _ in 0..45 {
            run(&mut state, Msg::Typed('a'));
        }
        run(&mut state, Msg::Typed('\t'));
        run(&mut state, Msg::Typed('\u{7f}'));
        assert_eq!(state.find().chars().count(), 40);
        run(&mut state, Msg::Erase);
        assert_eq!(state.find().chars().count(), 39);
        let out = run(&mut state, Msg::Escape).outs;
        assert!(out.is_empty());
        assert!(state.find().is_empty());
        assert!(state.open(), "Escape with a Find typed does not close");
    }

    /// Invariant: Cancel inside the guide goes back a step, from a page opened from the contents to the contents and from the contents to the place's page, and closes only from the place's own page.
    /// Regression: Cancel closes the guide from a page opened out of the contents, or never closes it.
    /// Owner: app
    #[test]
    fn cancel_steps_back_before_it_closes() {
        let mut state = State::new();
        run(&mut state, Msg::Open(place("home", SMALL)));
        run(&mut state, Msg::Press(Press::Contents));
        assert!(state.contents());
        run(&mut state, Msg::Cancel);
        assert!(
            state.sheet() && !state.contents(),
            "back to the place's page"
        );
        let out = run(&mut state, Msg::Cancel).outs;
        assert!(matches!(out.as_slice(), [Out::Front(None)]));
        assert!(!state.open());
    }

    /// Invariant: The side pane opens only on an extra-large window that is not compact or held and whose place is free; anywhere else the guide is the sheet.
    /// Regression: The pane opens on a window too small, held, or with its place taken by another pane.
    /// Owner: app
    #[test]
    fn the_pane_opens_only_where_the_window_and_the_place_hold_it() {
        for room in [
            SMALL,
            Room {
                compact: true,
                ..WIDE
            },
            Room { held: true, ..WIDE },
            Room {
                place_free: false,
                ..WIDE
            },
        ] {
            let mut state = State::new();
            let out = run(&mut state, Msg::Open(place("home", room))).outs;
            assert!(sheet_up(&out), "{room:?}");
            assert!(!state.pane_shown());
        }
        let mut state = State::new();
        let out = run(&mut state, Msg::Open(place("home", WIDE))).outs;
        assert!(out.is_empty());
        assert!(state.pane_shown());
    }

    /// Invariant: The Guide button puts a shown pane away, and Escape does the same only with no sheet over the page; a window that stops holding the pane turns it into the sheet.
    /// Regression: Guide opens a second guide over the pane, or Escape closes a sheet's page from under it.
    /// Owner: app
    #[test]
    fn the_button_and_escape_put_the_pane_away_and_a_narrow_window_makes_it_the_sheet() {
        let mut state = State::new();
        run(&mut state, Msg::Open(place("home", WIDE)));
        run(&mut state, Msg::Open(place("home", WIDE)));
        assert!(!state.open());
        run(&mut state, Msg::Open(place("home", WIDE)));
        run(&mut state, Msg::Escape);
        assert!(!state.open());
        run(&mut state, Msg::Open(place("home", WIDE)));
        let out = run(&mut state, Msg::Follow(place("home", SMALL))).outs;
        assert!(sheet_up(&out));
        assert!(state.sheet() && !state.pane_shown());
    }

    /// Invariant: An open pane follows the page: a new place opens its page, and a place that keeps the pane's place taken leaves it where it was.
    /// Regression: The pane stays on the first page, or follows a page that has taken its place.
    /// Owner: app
    #[test]
    fn the_pane_follows_the_page_unless_its_place_is_taken() {
        let mut state = State::new();
        run(&mut state, Msg::Open(place("home", WIDE)));
        run(&mut state, Msg::Follow(place("wallets", WIDE)));
        assert!(state.shown_page().is_some_and(|p| p.place == "wallets"));
        run(
            &mut state,
            Msg::Follow(place(
                "files",
                Room {
                    place_free: false,
                    ..WIDE
                },
            )),
        );
        assert!(state.shown_page().is_some_and(|p| p.place == "wallets"));
    }
}
