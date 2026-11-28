use faraday_kernel::settings::{
    Appearance, Choice, Contrast, MotionSpeed, MotionStyle, Page, Settings as Choices, TextSize,
};
use faraday_kernel::{Line, Refused, SelfTest, Show, View, Worded};
use faraday_organisms::Offered;
use faraday_organisms::content::{Block, Content};
use faraday_organisms::nav::{Raise, SearchBarModel};
use faraday_organisms::{
    AppearanceModel, ButtonModel, ChipsModel, ChoiceModel, FieldModel, OptionRow, Segments, Shown,
    Style, TextSizeModel, ThemeChip, TimeZoneKind, TimeZoneModel,
};
use faraday_present::{Present, offer};
use faraday_strings::{Strings, fill};
use faraday_templates::Theme;
use faraday_templates::menu_page::{Body, MenuPageModel, Section};
use faraday_templates::screen::BarSearch;
use faraday_text::Text;

use crate::clock::{self, Focus, Source};
use crate::file::{self, IDLE_LOCK_CHOICES, IDLE_OFF_CHOICES, SCALES};
use crate::kept::ModeAsked;
use crate::rows::{self, Part};
use crate::{Model, Msg, Press, Refusal, Settings, State};

impl Worded for Refusal {
    fn words(&self, show: &Show<'_>) -> Line {
        let s = show.strings();
        Line::Fixed(match self {
            Refusal::ModeInUse => s.settings_mode,
            Refusal::NotReady => s.set_time_zone_not_set,
            Refusal::NoClock => s.set_time_not_known,
        })
    }
}

impl Refused for Refusal {}

/// The offered press: the one gate decides (04 V11).
fn gate(state: &State, view: &View<'_>, press: Press) -> Offered<Press> {
    offer::<Press, Settings>(state, &Msg::Press(press.clone()), press, view)
}

fn button<'s>(
    state: &State,
    view: &View<'_>,
    label: &'static str,
    style: Style,
    press: Press,
) -> ButtonModel<'s, Press> {
    ButtonModel {
        label: Shown::plain(label),
        style,
        offered: gate(state, view, press),
        chosen: false,
        icon: None,
    }
}

/// One labelled row of segmented choices; `dim` draws it dimmed, taking no press.
fn row<'s>(
    label: &'static str,
    items: Vec<(Shown<'s>, Press)>,
    chosen: Option<usize>,
    dim: bool,
) -> Block<'s, Press> {
    Block::Segmented(TextSizeModel {
        label: Shown::plain(label),
        sizes: Segments { items, chosen },
        label_w: None,
        dim,
    })
}

fn made<'s>(text: Text) -> Shown<'s> {
    Shown::made(text)
}

impl Present for Settings {
    type Model<'s> = Model<'s>;

    fn present<'s>(state: &'s State, view: &View<'s>, show: &Show<'s>) -> Model<'s> {
        match state.page() {
            Page::Settings => Model::Settings(Box::new(settings_page(state, view, show))),
            Page::SetTime => Model::SetTime(set_time_page(state, view, show)),
        }
    }
}

// ---- Settings ----

fn settings_page<'s>(
    state: &'s State,
    view: &View<'s>,
    show: &Show<'s>,
) -> MenuPageModel<'s, Press> {
    let s = show.strings();
    let find = state.find();
    let parts: Vec<Part> = if find.as_str().trim().is_empty() {
        rows::PARTS.to_vec()
    } else {
        rows::matching(find.as_str(), s)
    };
    let cards: Vec<Content<'s, Press>> = parts
        .into_iter()
        .map(|p| card(state, view, show, p))
        .collect();
    let search = BarSearch {
        field: SearchBarModel {
            typed: Shown::plain(find),
            hint: Shown::plain(s.home_search_hint),
            focused: state.finding(),
            here: Press::Find,
            clear: (Press::FindClear, Shown::plain(s.vault_search_clear)),
            raise: Some(Raise::Level),
        },
        ways: None,
    };
    MenuPageModel {
        scroll: crate::SCROLL,
        row: Some(search),
        most_columns: 2,
        fixed_columns: None,
        bar_still: false,
        panes: None,
        sections: vec![Section {
            label: None,
            body: Body::Cards(cards),
        }],
        above: None,
        results: None,
        details: None,
        on_placed: None,
        glide: None,
        tile_scroll: None,
        own: None,
    }
}

fn card<'s>(state: &State, view: &View<'s>, show: &Show<'s>, p: Part) -> Content<'s, Press> {
    let s = show.strings();
    let mut blocks = vec![Block::Heading(Shown::plain(rows::title(p, s)))];
    match p {
        Part::Mode => mode(view, s, &mut blocks, state),
        Part::Appearance => appearance(view.settings, s, &mut blocks),
        Part::Session => session(state, view, s, &mut blocks),
        Part::Time => time(state, view, s, &mut blocks),
        Part::Display => display(view, s, &mut blocks),
        Part::Remembered => remembered(view.settings, s, &mut blocks),
        Part::About => about(state, view, s, &mut blocks),
    }
    Content(blocks)
}

/// The modes this product offers, from S9: a Practice and no Mainnet online, a Mainnet and no
/// Practice on the device.
fn offered(online: bool) -> [ModeAsked; 4] {
    [
        if online {
            ModeAsked::Practice
        } else {
            ModeAsked::Mainnet
        },
        ModeAsked::Testnet,
        ModeAsked::Signet,
        ModeAsked::Regtest,
    ]
}

fn mode_name(s: &Strings, m: ModeAsked) -> &'static str {
    match m {
        ModeAsked::Practice => s.mode_demo,
        ModeAsked::Mainnet => s.wallet_mainnet,
        ModeAsked::Testnet => s.wallet_testnet,
        ModeAsked::Signet => s.wallet_signet,
        ModeAsked::Regtest => s.wallet_regtest,
    }
}

fn mode<'s>(view: &View<'s>, s: &Strings, blocks: &mut Vec<Block<'s, Press>>, state: &State) {
    let in_use = ModeAsked::of_index(view.mode.index());
    let all = offered(view.mode.online);
    let items = all
        .iter()
        .map(|&m| (Shown::plain(mode_name(s, m)), Press::ModeAsk(m)))
        .collect();
    let on = all.iter().position(|&m| m == in_use);
    blocks.push(row(s.mode_sheet_title, items, on, false));
    // A pick the Lock sheet is asking about, with its way out.
    if let Some(m) = state.mode_asked() {
        blocks.push(Block::Buttons(vec![ButtonModel {
            label: made(fill_text(s.mode_change_title, &[mode_name(s, m)])),
            style: Style::Tonal,
            offered: Offered::Yes(Press::ModeCancel),
            chosen: false,
            icon: None,
        }]));
    }
}

fn fill_text(template: &str, args: &[&str]) -> Text {
    Text::of(&fill(template, args))
}

fn theme_names(s: &Strings) -> [&'static str; 10] {
    [
        s.ui_cobalt,
        s.ui_bitcoin_orange,
        s.ui_lamplight,
        s.ui_mars,
        s.ui_forest,
        s.ui_frost,
        s.ui_dusk,
        s.ui_lavender,
        s.ui_rose,
        s.ui_monochrome,
    ]
}

fn appearance<'s>(c: &Choices, s: &Strings, blocks: &mut Vec<Block<'s, Press>>) {
    let a = c.appearance;
    let pick = |a: Appearance| Press::Choose(Choice::Appearance(a));
    let current = file::scheme(&a);
    let names = theme_names(s);
    let themes = Theme::all()
        .filter(|t| t.mode == current.mode && t.contrast == current.contrast)
        .map(|t| {
            let mut next = a;
            file::set_scheme(&mut next, t);
            ThemeChip {
                name: t.name,
                label: Shown::plain(names.get(t.row() / 6).copied().unwrap_or("")),
                press: pick(next),
            }
        })
        .collect();
    let modes = [(s.ui_light, false), (s.ui_dark, true)]
        .into_iter()
        .map(|(name, dark)| (Shown::plain(name), pick(Appearance { dark, ..a })))
        .collect();
    let contrasts = [
        (s.ui_contrast_standard, Contrast::Standard),
        (s.ui_contrast_medium, Contrast::Medium),
        (s.ui_contrast_high, Contrast::High),
    ]
    .into_iter()
    .map(|(name, contrast)| (Shown::plain(name), pick(Appearance { contrast, ..a })))
    .collect();
    blocks.push(Block::Appearance(AppearanceModel {
        labels: [
            Shown::plain(s.ui_theme_row),
            Shown::plain(s.ui_mode_row),
            Shown::plain(s.ui_contrast_row),
        ],
        themes: ChipsModel {
            current,
            items: themes,
            default_mark: Shown::plain(s.ui_default_theme),
        },
        modes: Segments {
            items: modes,
            chosen: Some(usize::from(a.dark)),
        },
        contrasts: Segments {
            items: contrasts,
            chosen: Some(match a.contrast {
                Contrast::Standard => 0,
                Contrast::Medium => 1,
                Contrast::High => 2,
            }),
        },
        label_w: None,
    }));
    let sizes = [
        (s.settings_text_standard, TextSize::Standard),
        (s.settings_text_large, TextSize::Large),
        (s.settings_text_larger, TextSize::Larger),
    ];
    blocks.push(row(
        s.settings_text_size,
        sizes
            .iter()
            .map(|&(n, text)| (Shown::plain(n), pick(Appearance { text, ..a })))
            .collect(),
        sizes.iter().position(|&(_, t)| t == a.text),
        false,
    ));
    let styles = [
        (
            s.settings_motion_smooth,
            s.settings_motion_smooth_line,
            MotionStyle::Smooth,
        ),
        (
            s.settings_motion_crossfade,
            s.settings_motion_crossfade_line,
            MotionStyle::Crossfade,
        ),
        (
            s.settings_motion_off,
            s.settings_motion_off_line,
            MotionStyle::Off,
        ),
    ];
    blocks.push(row(
        s.settings_motion,
        styles
            .iter()
            .map(|&(n, _, m)| {
                (
                    Shown::plain(n),
                    pick(Appearance {
                        motion: Some(m),
                        ..a
                    }),
                )
            })
            .collect(),
        styles.iter().position(|&(_, _, m)| Some(m) == a.motion),
        false,
    ));
    // What the chosen style does, as a fact.
    if let Some(&(_, line, _)) = styles.iter().find(|&&(_, _, m)| Some(m) == a.motion) {
        blocks.push(Block::Heading(Shown::plain(line)));
    }
    let speeds = [
        (s.settings_speed_quick, MotionSpeed::Quick),
        (s.settings_speed_medium, MotionSpeed::Medium),
        (s.settings_speed_relaxed, MotionSpeed::Relaxed),
    ];
    // Speed applies to Smooth motion and Crossfade: shown off with Off.
    blocks.push(row(
        s.settings_speed,
        speeds
            .iter()
            .map(|&(n, speed)| (Shown::plain(n), pick(Appearance { speed, ..a })))
            .collect(),
        speeds.iter().position(|&(_, v)| v == a.speed),
        a.motion == Some(MotionStyle::Off),
    ));
}

/// What an idle timer's choice of `m` minutes says ("Never" for 0).
fn minutes<'s>(s: &Strings, m: u16) -> Shown<'s> {
    if m == 0 {
        Shown::plain(s.settings_never)
    } else {
        made(fill_text(s.settings_min, &[&m.to_string()]))
    }
}

fn session<'s>(state: &State, view: &View<'s>, s: &Strings, blocks: &mut Vec<Block<'s, Press>>) {
    let n = view.session.keys().len();
    blocks.push(Block::Heading(made(fill_text(
        s.settings_loaded,
        &[
            &n.to_string(),
            if n == 1 {
                s.settings_key
            } else {
                s.settings_keys
            },
        ],
    ))));
    let online = view.mode.online;
    let mut buttons = Vec::new();
    if n > 0 {
        buttons.push(button(
            state,
            view,
            s.settings_lock,
            Style::Tonal,
            Press::Lock,
        ));
    }
    let leave = if online {
        s.settings_quit
    } else {
        s.settings_power_off_button
    };
    buttons.push(button(state, view, leave, Style::Tonal, Press::PowerAsk));
    blocks.push(Block::Buttons(buttons));
    if !online {
        blocks.push(Block::Heading(Shown::plain(s.app_panic_key_hint)));
    }
    let c = view.settings;
    blocks.push(row(
        s.settings_lock_after,
        IDLE_LOCK_CHOICES
            .iter()
            .map(|&m| (minutes(s, m), Press::Choose(Choice::IdleLock(m))))
            .collect(),
        IDLE_LOCK_CHOICES.iter().position(|&m| m == c.idle_lock_min),
        false,
    ));
    // The power-off timer only where the device can power off.
    if !online {
        blocks.push(row(
            s.settings_power_off,
            IDLE_OFF_CHOICES
                .iter()
                .map(|&m| (minutes(s, m), Press::Choose(Choice::IdleOff(m))))
                .collect(),
            IDLE_OFF_CHOICES.iter().position(|&m| m == c.idle_off_min),
            false,
        ));
    }
}

/// The zone as it reads: "UTC" and the offset, or that it is not set.
fn zone_text<'s>(s: &Strings, zone: Option<i16>) -> Shown<'s> {
    match zone {
        Some(min) => made(fill_text("UTC{}", &[&file::tz_offset_text(min)])),
        None => Shown::plain(s.set_time_zone_not_set),
    }
}

/// The live local date and time, seconds shown, or "Not known"; with no zone chosen the clock
/// reads in UTC, and says so.
fn now_text<'s>(view: &View<'_>, s: &Strings) -> Shown<'s> {
    let Some(t) = clock::now(&view.clock) else {
        return Shown::plain(s.set_time_not_known);
    };
    let zone = view.settings.zone_min;
    let local = clock::local_secs(t, zone.unwrap_or(0));
    let utc = if zone.is_some() { "" } else { " UTC" };
    made(Text::of(&format!(
        "{} {}{utc}",
        clock::date_text(local).as_str(),
        clock::time_text(local).as_str()
    )))
}

fn time<'s>(state: &State, view: &View<'s>, s: &Strings, blocks: &mut Vec<Block<'s, Press>>) {
    blocks.push(Block::Fact {
        label: Shown::plain(s.settings_now),
        value: now_text(view, s),
        mono: false,
    });
    blocks.push(Block::Zone(TimeZoneModel {
        value: zone_text(s, view.settings.zone_min),
        kind: TimeZoneKind::Row {
            label: Shown::plain(s.tz_row_time_zone),
            open: gate(state, view, Press::ZoneList(true)),
        },
    }));
    blocks.push(Block::Buttons(vec![button(
        state,
        view,
        s.settings_set_the_time,
        Style::Tonal,
        Press::OpenSetTime,
    )]));
}

fn display<'s>(view: &View<'s>, s: &Strings, blocks: &mut Vec<Block<'s, Press>>) {
    let c = view.settings;
    blocks.push(row(
        s.settings_scale,
        SCALES
            .iter()
            .map(|&pct| {
                let label = if pct == 100 {
                    Shown::plain(s.settings_auto)
                } else {
                    made(Text::of(&format!("{pct} %")))
                };
                (label, Press::Choose(Choice::Scale(pct)))
            })
            .collect(),
        SCALES.iter().position(|&pct| pct == c.scale_pct),
        false,
    ));
    // Rotation only where the shell can turn its frame.
    if view.facts.display.rotation_offered.is_some() {
        use faraday_port::values::Orientation;
        blocks.push(row(
            s.settings_rotation,
            vec![
                (
                    Shown::plain(s.settings_landscape),
                    Press::Choose(Choice::Rotation(Orientation::Landscape)),
                ),
                (
                    Shown::plain(s.settings_portrait),
                    Press::Choose(Choice::Rotation(Orientation::Portrait)),
                ),
            ],
            Some(usize::from(c.rotation == Orientation::Portrait)),
            false,
        ));
    }
}

fn remembered<'s>(c: &Choices, s: &Strings, blocks: &mut Vec<Block<'s, Press>>) {
    blocks.push(row(
        s.settings_amounts_signed,
        vec![
            (
                Shown::plain(s.settings_in_open_vault),
                Press::Choose(Choice::SealAmounts(true)),
            ),
            (
                Shown::plain(s.settings_until_power_off),
                Press::Choose(Choice::SealAmounts(false)),
            ),
        ],
        Some(usize::from(!c.seal_amounts)),
        false,
    ));
    blocks.push(row(
        s.settings_signatures_made,
        vec![
            (
                Shown::plain(s.settings_in_open_vault),
                Press::Choose(Choice::NonceHistory(true)),
            ),
            (
                Shown::plain(s.settings_nonces_off),
                Press::Choose(Choice::NonceHistory(false)),
            ),
        ],
        Some(usize::from(!c.nonce_history)),
        false,
    ));
}

fn about<'s>(state: &State, view: &View<'s>, s: &Strings, blocks: &mut Vec<Block<'s, Press>>) {
    let fact = |value: Shown<'s>| Block::Fact {
        label: Shown::plain(""),
        value,
        mono: false,
    };
    blocks.push(fact(made(fill_text(
        s.settings_faraday,
        &[&crate::version_label()],
    ))));
    blocks.push(fact(Shown::plain(s.settings_keys_are_typed)));
    blocks.push(fact(Shown::plain(s.settings_own_risk)));
    let result = match view.facts.selftest {
        SelfTest::Passed => made(fill_text(s.settings_selftest_passed, &["all"])),
        SelfTest::Failed(check) => made(fill_text(s.settings_selftest_failed, &[check])),
        SelfTest::NotRun => Shown::plain(s.settings_not_run),
    };
    blocks.push(Block::Fact {
        label: Shown::plain(s.settings_selftest),
        value: result,
        mono: false,
    });
    blocks.push(Block::Buttons(vec![button(
        state,
        view,
        s.settings_selftest_run,
        Style::Tonal,
        Press::SelfTest,
    )]));
    // The institute's name is a link only online; the link is not built, so it reads as text.
    blocks.push(fact(made(fill_text(
        s.institute_note_visit,
        &["nakamotoinstitute.org"],
    ))));
}

// ---- Set the time ----

/// "YYYY-MM-DD HH:MM" of Unix seconds.
fn both(t: u64) -> String {
    let secs = t % 86_400;
    format!(
        "{} {:02}:{:02}",
        clock::date_text(t).as_str(),
        secs / 3600,
        (secs / 60) % 60
    )
}

fn set_time_page<'s>(state: &'s State, view: &View<'s>, show: &Show<'s>) -> Content<'s, Press> {
    let s = show.strings();
    let form = state.form();
    let zone = view.settings.zone_min;
    if form.zones() {
        return zone_list(state, view, s, zone);
    }
    let mut blocks = vec![
        Block::Headline(Shown::plain(s.set_time_set_the_time)),
        Block::Fact {
            label: Shown::plain(s.set_time_now),
            value: now_text(view, s),
            mono: false,
        },
        Block::Zone(TimeZoneModel {
            value: zone_text(s, zone),
            kind: TimeZoneKind::Row {
                label: Shown::plain(s.tz_row_time_zone),
                open: gate(state, view, Press::ZoneList(true)),
            },
        }),
    ];
    // What the time starts from, when it is not this computer's clock.
    match clock::source(&view.clock) {
        Some(Source::CountedFromSet) => {
            blocks.push(Block::Heading(Shown::plain(s.set_time_counted_from_set)));
        }
        Some(Source::NoReading) => {
            blocks.push(Block::Heading(Shown::plain(s.set_time_no_reading)));
        }
        None => {}
    }
    for (label, text, focus) in [
        (s.set_time_date, form.date(), Focus::Date),
        (s.set_time_time, form.time(), Focus::Time),
    ] {
        blocks.push(Block::Field(FieldModel {
            label: Some(Shown::plain(label)),
            text: Shown::plain(text),
            focused: form.focus() == Some(focus),
            error: None,
            offered: Offered::Yes(Press::Focus(focus)),
        }));
    }
    // What Set would set, both ways, when Set is offered.
    if let (true, Some((local, _, _))) = (state.set_ready(view), form.typed_local()) {
        let tz = zone.unwrap_or(0);
        let local = u64::try_from(local).unwrap_or(0);
        let utc = clock::local_secs(local, tz.saturating_neg());
        blocks.push(Block::Heading(made(fill_text(
            s.set_time_result,
            &[&both(local), &file::tz_offset_text(tz), &both(utc)],
        ))));
    }
    let mut buttons = vec![button(
        state,
        view,
        s.set_time_set,
        Style::Filled,
        Press::Set,
    )];
    if clock::now(&view.clock).is_some() && zone.is_some() {
        buttons.push(button(
            state,
            view,
            s.set_time_keep_button,
            Style::Tonal,
            Press::Keep,
        ));
    }
    buttons.push(button(
        state,
        view,
        s.set_time_back,
        Style::Text,
        Press::SetTimeBack,
    ));
    blocks.push(Block::Buttons(buttons));
    Content(blocks)
}

/// The list of zones in place of the fields: one row for each offset in the table, then the
/// steppers for an offset it lacks, and Back to the fields.
fn zone_list<'s>(
    state: &State,
    view: &View<'s>,
    s: &Strings,
    zone: Option<i16>,
) -> Content<'s, Press> {
    let rows = faraday_strings::zones::ZONES
        .iter()
        .map(|&(min, clue)| OptionRow {
            name: made(fill_text("UTC{}", &[&file::tz_offset_text(min)])),
            line: Shown::plain(clue),
            on: zone == Some(min),
            press: Press::Zone(min),
            tip: None,
        })
        .collect();
    Content(vec![
        Block::Headline(Shown::plain(s.tz_row_time_zone)),
        Block::Choose(ChoiceModel {
            rows,
            more: None,
            enabled: true,
        }),
        Block::Heading(Shown::plain(s.set_time_other_offset)),
        Block::Zone(TimeZoneModel {
            value: zone_text(s, zone),
            kind: TimeZoneKind::Steps {
                minus: Offered::Yes(Press::ZoneStep(-15)),
                plus: Offered::Yes(Press::ZoneStep(15)),
            },
        }),
        Block::Buttons(vec![button(
            state,
            view,
            s.set_time_back,
            Style::Text,
            Press::ZoneList(false),
        )]),
    ])
}
