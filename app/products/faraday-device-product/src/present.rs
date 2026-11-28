use faraday_kernel::{Line, Refused, Show, View, Worded};
use faraday_organisms::nav::WaysModel;
use faraday_organisms::nav::{Badge, Dest, NavModel, Raise, SearchBarModel, TopBarModel};
use faraday_organisms::{ButtonModel, Icon, Shown, Style};
use faraday_port::values::Anchor;
use faraday_present::{Present, ScreenModel, offer};
use faraday_templates::screen::{BarSearch, FrameModel, Notice};

use crate::{App, AppPress, Body, DeviceApp, Msg, Over, Page, Press, Refusal};

impl Worded for Refusal {
    fn words(&self, show: &Show<'_>) -> Line {
        match self {
            Refusal::Device(r) => r.words(show),
        }
    }
}

impl Refused for Refusal {}

/// What `Root::frame` draws: the frame, the page the navigation shows and the sheets over it
/// (06 §4.4).
pub(crate) fn screen<'s>(
    app: &'s DeviceApp,
    settings: &'s faraday_settings::State,
    view: &View<'s>,
    show: &Show<'s>,
) -> ScreenModel<'s, Body<'s>, Over<'s>, Press> {
    let shown = app.frame().nav().shown();
    let (body, page) = match shown {
        Page::Device(page) => (
            Body::Device(Box::new(faraday_device::body(
                app.device(),
                page,
                view,
                show,
            ))),
            faraday_device::anchor(page),
        ),
        // The settings owner holds which of its two pages is shown (Settings or Set the time):
        // it is the one place that knows, so the page is drawn from it and the nav's
        // `Page::Settings(_)` only says that Settings is the destination.
        Page::Settings(_) => {
            let page = settings.page();
            (
                Body::Settings(<faraday_settings::Settings as Present>::present(
                    settings, view, show,
                )),
                faraday_settings::anchor(page),
            )
        }
        // The vaults' pages and the failed self-test page are not built into the frame yet.
        Page::Vaults(_) | Page::SelfTestFailed => todo!(),
    };
    let mut over: Vec<(Anchor, Over<'s>)> = faraday_device::over(app.device(), view, show)
        .into_iter()
        .map(|(anchor, sheet)| (anchor, Over::Device(sheet)))
        .collect();
    over.extend(
        faraday_device::frame::sheets(app.device(), app.frame().sheets(), view, show)
            .into_iter()
            .map(|(anchor, sheet)| (anchor, Over::Frame(sheet))),
    );
    ScreenModel {
        page,
        frame: frame_model(app, view, show),
        body,
        over,
        note: None,
    }
}

/// The page a destination opens.
fn dest_page(dest: Dest_) -> Page {
    use faraday_device::Page as D;
    match dest {
        Dest_::Home => Page::Device(D::Home),
        Dest_::Check => Page::Device(D::CHECK),
        Dest_::Wallets => Page::Device(D::WALLETS),
        Dest_::Files => Page::Device(D::FILES),
        Dest_::Vaults => Page::Vaults(faraday_vaults::Page::Vaults),
        Dest_::Tools => Page::Device(D::TOOLS),
        Dest_::Settings => Page::Settings(faraday_settings::Page::Settings),
    }
}

/// The destinations of the device app (06 §4.3), in the bar's order.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Dest_ {
    Home,
    Check,
    Wallets,
    Files,
    Vaults,
    Tools,
    Settings,
}

/// The destination a page belongs under, if it is under one.
fn under(page: Page) -> Option<Dest_> {
    use faraday_device::Page as D;
    match page {
        Page::Device(D::Home) => Some(Dest_::Home),
        Page::Device(D::Checks(_)) => Some(Dest_::Check),
        Page::Device(D::Wallets(_)) => Some(Dest_::Wallets),
        Page::Device(D::Files(_)) => Some(Dest_::Files),
        Page::Device(D::Tools(_)) => Some(Dest_::Tools),
        Page::Vaults(_) => Some(Dest_::Vaults),
        Page::Settings(_) => Some(Dest_::Settings),
        Page::Device(D::Keys(_) | D::Signing(_)) | Page::SelfTestFailed => None,
    }
}

/// Home's first actions beside the bar's search (`main`'s Home top row): the three tonal buttons
/// and the one Start button that stands for them where they do not fit.
fn ways<'s>(
    app: &'s DeviceApp,
    view: &View<'s>,
    s: &'static faraday_strings::Strings,
) -> WaysModel<'s, Press> {
    use faraday_home::Press as H;
    let press = |p: H| Press::App(AppPress::Device(faraday_device::Press::Home(p)));
    let offered = |p: H| {
        offer::<Press, App>(
            app,
            &Msg::from(AppPress::Device(faraday_device::Press::Home(p.clone()))),
            press(p),
            view,
        )
    };
    let button = |icon: Icon, label: &'static str, p: H| ButtonModel {
        label: Shown::plain(label),
        style: Style::Tonal,
        offered: offered(p),
        chosen: false,
        icon: Some(icon),
    };
    WaysModel {
        pills: vec![
            button(Icon::Flag, s.home_ways_new_short, H::Make),
            button(Icon::Wallet, s.home_ways_restore_short, H::Restore),
            button(Icon::Sign, s.home_ways_spend_short, H::SpendBackup),
        ],
        start: ButtonModel {
            label: Shown::plain(s.home_ways_start),
            style: Style::Filled,
            offered: offered(H::Make),
            chosen: false,
            icon: None,
        },
        pills_ok: true,
        raise: Some(Raise::Level),
    }
}

/// The navigation, the top app bar and, on the wide layout, the bar's search (06 §4.4). Home's
/// model holds the page's search row only, so the bar's is built here from the same state.
fn frame_model<'s>(app: &'s DeviceApp, view: &View<'s>, show: &Show<'s>) -> FrameModel<'s, Press> {
    let s = show.strings();
    let shown = app.frame().nav().shown();
    let here = under(shown);
    // The small panel's Home is the Pi's menu: its places are the page's, so no bottom bar.
    let small = faraday_home::small_panel(&view.facts.display);
    let dests: Vec<_> = if small && shown == Page::Device(faraday_device::Page::Home) {
        Vec::new()
    } else {
        vec![
            (Dest_::Home, Icon::House, s.compact_home),
            (Dest_::Check, Icon::Verify, s.compact_check),
            (Dest_::Wallets, Icon::Wallet, s.compact_wallets),
            (Dest_::Files, Icon::File, s.compact_files),
            (Dest_::Vaults, Icon::Vault, s.compact_vaults),
            (Dest_::Tools, Icon::Tools, s.compact_tools),
            (Dest_::Settings, Icon::Settings, s.compact_settings),
        ]
        .into_iter()
        .map(|(dest, icon, label)| {
            let press = Press::App(AppPress::Open(dest_page(dest)));
            Dest {
                icon: icon.into(),
                label: Shown::plain(label),
                offered: offer::<Press, App>(
                    app,
                    &Msg::from(AppPress::Open(dest_page(dest))),
                    press,
                    view,
                ),
                current: here == Some(dest),
                badge: Badge::None,
                flows: 0,
                // The bar keeps five destinations; Check and Settings are the rail's (`main`).
                bar: !matches!(dest, Dest_::Check | Dest_::Settings),
            }
        })
        .collect()
    };
    let title = match here {
        Some(Dest_::Home) => s.compact_home,
        Some(Dest_::Check) => s.compact_check,
        Some(Dest_::Wallets) => s.compact_wallets,
        Some(Dest_::Files) => s.compact_files,
        Some(Dest_::Vaults) => s.compact_vaults,
        Some(Dest_::Tools) => s.compact_tools,
        Some(Dest_::Settings) => s.compact_settings,
        None => "",
    };
    let search = (shown == Page::Device(faraday_device::Page::Home)).then(|| BarSearch {
        field: SearchBarModel {
            typed: Shown::plain(app.find()),
            hint: Shown::plain(s.home_search_hint),
            focused: app.find_focused(),
            here: Press::App(AppPress::Device(faraday_device::Press::Home(
                faraday_home::Press::Focus,
            ))),
            clear: (
                Press::App(AppPress::Device(faraday_device::Press::Home(
                    faraday_home::Press::Clear,
                ))),
                Shown::plain(s.vault_search_clear),
            ),
            raise: Some(Raise::Level),
        },
        ways: Some(ways(app, view, s)),
    });
    FrameModel {
        nav: NavModel {
            dests,
            many: Shown::plain(s.nav_badge_many),
        },
        top: TopBarModel {
            // The Back link's words are the page behind's name, which the memo will hold once a
            // page other than Home can be shown.
            back: None,
            title: Shown::plain(title),
            pill_w: 0.0,
            item_name: false,
            in_sheet: false,
            under: None,
            right: f32::MAX,
            power: Some((Press::App(AppPress::Power), Shown::plain(s.tip_power))),
        },
        busy: None,
        search,
        side: faraday_templates::screen::Side::None,
        keyboard: None,
        pills: None,
        tip: None,
        // The keyboards held back, named until a key is typed on a keyboard or the × is pressed.
        notices: faraday_device::notices(app.device(), show)
            .into_iter()
            .map(|(text, press)| Notice {
                text: Shown::made(text),
                close: Press::App(AppPress::Device(press)),
            })
            .collect(),
        keyboard_button: None,
        held: None,
    }
}

#[cfg(test)]
mod tests {
    use faraday_app_test::TestView;
    use faraday_kernel::{Carried, Feature};

    use super::*;

    /// Invariant: With the navigation on Settings the screen draws the settings owner's page, not a page of the device composite or an unbuilt one.
    /// Regression: The frame names Settings while the body is a todo, or the body is Home's.
    /// Owner: app
    #[test]
    fn the_screen_on_settings_draws_the_settings_page() {
        let view = TestView::device();
        let cx = view.ctx();
        let (mut app, _) = <App as Feature>::start(&mut Carried::empty(), &cx);
        let (settings, _) =
            <faraday_settings::Settings as Feature>::start(&mut Carried::empty(), &cx);
        let _ = app
            .parts()
            .1
            .nav_mut()
            .open(Page::Settings(faraday_settings::Page::Settings));
        view.showing(|view, show| {
            let model = screen(&app, &settings, view, show);
            assert!(matches!(model.body, Body::Settings(_)));
        });
    }
}
