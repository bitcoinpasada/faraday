use faraday_home::Home;
use faraday_kernel::{Line, Refused, Show, View, Worded};
use faraday_port::values::Anchor;
use faraday_present::Present;

use crate::{Body, Over, Page, Refusal, State};

impl Worded for Refusal {
    fn words(&self, show: &Show<'_>) -> Line {
        match self {
            Refusal::Devices(r) => r.words(show),
        }
    }
}

impl Refused for Refusal {}

/// What `present` works out for the page the frame's navigation shows.
pub fn body<'s>(state: &'s State, page: Page, view: &View<'s>, show: &Show<'s>) -> Body<'s> {
    match page {
        Page::Home => Body::Home(Box::new(Home::present(&state.home, view, show))),
        // The areas' pages are not built.
        Page::Keys(_)
        | Page::Wallets(_)
        | Page::Signing(_)
        | Page::Checks(_)
        | Page::Tools(_)
        | Page::Files(_) => todo!(),
    }
}

/// The sheets over the page, each with its anchor, the lowest first. The scan sheet and the
/// areas' own sheets come up only with the features that show them, which are not built.
pub fn over<'s>(_state: &'s State, _view: &View<'s>, _show: &Show<'s>) -> Vec<(Anchor, Over<'s>)> {
    Vec::new()
}

/// The page's anchor: the motion store's scope and the page turn's.
#[must_use]
pub fn anchor(page: Page) -> Anchor {
    match page {
        Page::Home => faraday_home::ANCHORS[0],
        // The areas' pages are not built.
        Page::Keys(_)
        | Page::Wallets(_)
        | Page::Signing(_)
        | Page::Checks(_)
        | Page::Tools(_)
        | Page::Files(_) => todo!(),
    }
}
