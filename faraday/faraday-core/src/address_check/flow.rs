//! Check an address as the flow's one card, where the window is not
//! compact (`docs/FLOW-CONTAINER.md` §2): the address's own pick (the
//! text files in Files and the ways in) at the left, the field and the
//! result on the card. While the scanner stands in its pane the page is
//! drawn with it (`pane`).

use super::AddressPress;
use crate::flow::{Contained, Pick, PickGroup, Row};
use crate::strings::{EN, fill};
use crate::trail::{StepFlow, Steps};
use crate::wallet::FileKind;
use crate::{Action, Faraday, Screen};
use faraday_draw::widgets::Icon;
use faraday_text::Text;

/// The step that has no answer yet: the address and the pick.
pub(crate) const PICK: u8 = 0;
/// The step that shows an answer, a file's list or what the check waits
/// for.
pub(crate) const RESULT: u8 = 1;

/// The card titles, by step.
const TITLES: [&str; 2] = [EN.address_check_field, EN.decode_card_result];

/// Whether Check an address is drawn as the flow's one card: the one place
/// every site asks. A small panel draws the page, and so does the window
/// while the scanner stands in its pane. A lookup.
pub(crate) fn contained(app: &Faraday) -> bool {
    app.screen == Screen::CheckAddress
        && !app.compact
        && app.address_check_state().is_some()
        && !super::pane::scanning(app)
}

/// The step open: the result once there is an answer or a list, or an
/// address waits for a wallet or key to be searched in.
fn open_step(app: &Faraday) -> u8 {
    let shows = app.address_check_state().is_some_and(|s| {
        s.answer().is_some() || s.list().is_some() || (s.load_first() && !s.typed().is_empty())
    });
    if shows { RESULT } else { PICK }
}

/// Check an address' steps, plugged into Back and Your choices. Only the
/// open step is in the order: Back leaves as the page's does.
pub(crate) struct CheckAddressSteps;

/// The one [`CheckAddressSteps`].
pub(crate) static ADDRESS_STEPS: CheckAddressSteps = CheckAddressSteps;

impl StepFlow for CheckAddressSteps {
    fn steps(&self, app: &Faraday) -> Option<Steps> {
        let k = open_step(app);
        Some(Steps::new(vec![k], Some(k)))
    }

    fn open(&self, app: &Faraday, k: u8) -> Action {
        if k == PICK {
            Action::AddressCheck(AddressPress::Another)
        } else {
            Action::Nav(
                app.address_check_state()
                    .map_or(Screen::Catalog, |s| s.back()),
            )
        }
    }

    fn name(&self, _: &Faraday, k: u8) -> &'static str {
        TITLES[usize::from(k.min(RESULT))]
    }

    fn contained(&self) -> Option<&dyn Contained> {
        Some(self)
    }
}

impl Contained for CheckAddressSteps {
    fn rows(&self, app: &Faraday) -> Vec<Row> {
        let Some(s) = app.address_check_state() else {
            return Vec::new();
        };
        if open_step(app) == PICK || s.typed().is_empty() {
            return Vec::new();
        }
        vec![Row {
            label: Text::of(TITLES[usize::from(PICK)]),
            value: Some(s.typed().clone()),
            mono: true,
            opens: PICK,
            current: false,
            mark: None,
            address: true,
        }]
    }

    fn hide_shown(&self, _: &mut Faraday) {
        // An address is public: no step shows a secret.
    }

    fn count_fixed(&self, _: &Faraday) -> bool {
        true
    }

    /// The only step in the order stands without a count.
    fn heads(&self, _: &Faraday) -> usize {
        1
    }

    fn pick(&self, app: &Faraday, _: u8) -> Option<Pick> {
        // Both steps: the pick stays at the left with the file read marked.
        Some(Pick {
            question: question(app),
            groups: pick_groups(app),
        })
    }
}

/// What the pick asks: which file, when text files wait.
fn question(app: &Faraday) -> &'static str {
    if super::file::waiting(app).is_empty() {
        EN.address_check_title
    } else {
        EN.address_check_choose_file
    }
}

/// The text files held in Files, then the ways in: Type it, Scan, Paste
/// and From a file; while no text file waits, Open Files and the stick
/// too. The file whose addresses are on show is marked.
pub(super) fn pick_groups(app: &Faraday) -> Vec<PickGroup> {
    use crate::pick::PickRow;
    let from = app.address_check_state().and_then(|s| s.from());
    let press = |p| Action::AddressCheck(p);
    let mut ways = vec![
        PickRow::way_in(
            Icon::Keyboard,
            EN.invoice_way_type,
            Action::TypeHere(crate::edit::Here::CheckAddress),
        ),
        PickRow::way_in(Icon::Scan, EN.address_check_scan, press(AddressPress::Scan)),
    ];
    if app.clipboard {
        ways.push(PickRow::way_in(
            Icon::Paste,
            EN.scan_paste,
            press(AddressPress::Paste),
        ));
    }
    if !super::file::files(app).is_empty() {
        ways.push(PickRow::way_in(
            Icon::File,
            EN.address_check_file,
            press(AddressPress::Files),
        ));
    }
    if super::file::waiting(app).is_empty() {
        ways.push(PickRow::way_in(
            Icon::File,
            EN.parts_open_files,
            Action::Nav(Screen::Files),
        ));
        if !app.stick.is_empty() && !app.holds_secret() {
            let label = &app.stick[app.visit.stick.min(app.stick.len() - 1)].label;
            ways.push(PickRow::way_in(
                Icon::Drive,
                &fill(EN.parts_bring_in_from, &[label]),
                Action::VisitFrom(Screen::CheckAddress),
            ));
        }
    }
    crate::pick::Decl::new(question(app))
        .files(app, |k, f| {
            (f.kind == FileKind::Text).then(|| {
                crate::pick::Take::of(from == Some(k), press(AddressPress::File(k as u16)))
            })
        })
        .ways(ways)
        .groups()
}
