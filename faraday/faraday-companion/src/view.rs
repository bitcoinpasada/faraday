//! The Companion's screens, built each frame from its state out of
//! `osk-ui`'s §5 screens (`docs/COMPANION.md` §4), as `opensigner-core`'s
//! views are. Nothing here changes the state.

use std::rc::Rc;

use faraday_qr::send::{Format, Made};
use faraday_strings::{EN, fill};
use osk_shell_api::DisplayInfo;
use osk_ui::Id;
use osk_ui::components::{self, Network, Preview};
use osk_ui::geom::Metrics;
use osk_ui::layout::Node;
use osk_ui::organisms::TileSpec;
use osk_ui::screens::{self, Action, ActionRow, Chrome, Row, Section};
use osk_ui::widgets::{Icon, Tone, Widget};

use crate::store::Kind;
use faraday_intake::Secret;

use crate::{Act, Companion, Confirm, Doc, NotSent, Page, ScanLine, Shown};

/// The value of a shown string, which opens nothing.
const VALUE: Id = Id(90);
/// The recovery key's panel.
const PANEL: Id = Id(91);

/// Every page Learn lists: `osk-learn`'s, then Faraday's own.
pub(crate) fn learn_docs() -> Vec<Doc> {
    (0..osk_learn::PAGES)
        .map(Doc::Osk)
        .chain(faraday_learn::ids().map(Doc::Own))
        .collect()
}

/// The pages Start from the stick lists: the boot guides, then the
/// makers' pages.
pub(crate) fn start_docs() -> Vec<Doc> {
    faraday_learn::BOOT
        .iter()
        .copied()
        .chain(faraday_learn::MAKERS.iter().map(|m| m.id))
        .map(Doc::Own)
        .collect()
}

/// A page's text.
fn doc_page(d: Doc) -> &'static osk_learn::Page {
    match d {
        Doc::Osk(i) => {
            let pages = osk_learn::EN.pages();
            pages[i.min(pages.len() - 1)]
        }
        Doc::Own(id) => faraday_learn::known(id),
    }
}

/// What a kind of kept file is called.
pub(crate) fn kind_name(k: Kind) -> &'static str {
    match k {
        Kind::Psbt => EN.companion_kind_psbt,
        Kind::Transaction => EN.companion_kind_transaction,
        Kind::Description => EN.companion_kind_description,
        Kind::File => EN.companion_kind_file,
        Kind::Text => EN.companion_kind_text,
    }
}

/// The line a refusal shows (`docs/COMPANION.md` §5).
pub(crate) fn refusal(s: Secret) -> &'static str {
    match s {
        Secret::Bip39Words => EN.companion_refuse_words,
        Secret::SeedQr => EN.companion_refuse_seedqr,
        Secret::CompactSeedQr => EN.companion_refuse_compact_seedqr,
        Secret::Codex32 => EN.companion_refuse_codex32,
        Secret::Slip39Share => EN.companion_refuse_slip39,
        Secret::ExtendedPrivateKey => EN.companion_refuse_xprv,
        Secret::Wif => EN.companion_refuse_wif,
        Secret::UrSeed => EN.companion_refuse_ur_seed,
        Secret::UrBip39 => EN.companion_refuse_ur_bip39,
        Secret::UrPrivateKey => EN.companion_refuse_ur_private_key,
        Secret::EncryptedBackup => EN.companion_refuse_backup,
        Secret::KeePass => EN.companion_refuse_keepass,
        Secret::Vault => EN.companion_refuse_vault,
        Secret::ThresholdCarry => EN.companion_refuse_carry,
        Secret::Totp => EN.companion_refuse_totp,
        Secret::TotpMigration => EN.companion_refuse_totp_migration,
        Secret::VaultEntries => EN.companion_refuse_entries,
    }
}

/// A size as a person reads it.
fn size(n: usize) -> String {
    if n < 1024 {
        fill(EN.companion_bytes, &[&n.to_string()])
    } else {
        let tenths = (n * 10).div_ceil(1024);
        let kib = format!("{}.{}", tenths / 10, tenths % 10);
        fill(EN.companion_kib, &[&kib])
    }
}

/// A time kept, as the phone's clock shows it: `2026-10-12 14:02`.
fn when(unix_s: u64, offset_s: i32) -> String {
    if unix_s == 0 {
        return EN.companion_time_unknown.to_string();
    }
    let t = unix_s as i64 + i64::from(offset_s);
    let days = t.div_euclid(86_400);
    let secs = t.rem_euclid(86_400);
    // Howard Hinnant's days-from-civil, inverted.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}",
        secs / 3600,
        (secs % 3600) / 60
    )
}

/// Bytes as lower-case hex.
fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// The line under the codes.
fn made_line(m: Made) -> String {
    let n = |k: usize| k.to_string();
    match m {
        Made::UrPsbtOne => EN.companion_made_psbt_one.to_string(),
        Made::UrPsbtFrames(k) => fill(EN.companion_made_psbt_frames, &[&n(k)]),
        Made::TextOne | Made::PartsOne => EN.companion_made_text_one.to_string(),
        Made::KeyOne => EN.companion_made_key_one.to_string(),
        Made::UrAccount => EN.companion_made_account.to_string(),
        Made::UrBytesOne { file: false } => EN.companion_made_bytes_one.to_string(),
        Made::UrBytesOne { file: true } => EN.companion_made_file_one.to_string(),
        Made::UrBytesFrames {
            file: false,
            frames,
        } => fill(EN.companion_made_bytes_frames, &[&n(frames)]),
        Made::UrBytesFrames { file: true, frames } => {
            fill(EN.companion_made_file_frames, &[&n(frames)])
        }
        Made::BbqrOne(t) => fill(EN.companion_made_bbqr_one, &[&t.to_string()]),
        Made::BbqrFrames(t, k) => fill(EN.companion_made_bbqr_frames, &[&t.to_string(), &n(k)]),
        Made::PartsNumbered(k) => fill(EN.companion_made_parts, &[&n(k)]),
    }
}

/// The screen for the state as it is.
pub(crate) fn build(app: &Companion, display: &DisplayInfo) -> Node {
    let m = Metrics::of(display);
    let c = Chrome {
        m: &m,
        nav: &[],
        back: (app.page != Page::Home).then_some(Act::Back.id()),
        dimmed: false,
        info: None,
    };
    let c = &c;
    match &app.page {
        Page::Home => home(app, c),
        Page::Receive => receive(app, c),
        Page::Arrived { sha256, again } => arrived(app, c, sha256, *again),
        Page::Refused(s) => refused(app, c, *s),
        Page::Shown(s) => shown(c, s),
        Page::BitLocker => bitlocker(app, c),
        Page::SendList => send_list(app, c),
        Page::Send => send(app, c),
        Page::NotSent(why) => not_sent(c, *why),
        Page::Received => received(app, c),
        Page::Record(sha) => record(app, c, sha),
        Page::Confirm(what) => confirm(app, c, *what),
        Page::Learn => doc_list(c, EN.companion_home_learn, &learn_docs(), Act::LearnPage),
        Page::Start => doc_list(c, EN.companion_home_start, &start_docs(), Act::StartPage),
        Page::Doc(d) => document(c, *d),
        Page::About => about(app, c),
    }
}

fn menu_row(act: Act, icon: Icon, label: &str, value: Option<String>) -> Row {
    Row::Menu {
        id: act.id(),
        icon: Some(icon),
        label: label.to_string(),
        value,
        tone: Tone::Text,
    }
}

/// §4.1 Home.
fn home(app: &Companion, c: &Chrome) -> Node {
    let rows = vec![
        menu_row(Act::Receive, Icon::Scan, EN.companion_home_receive, None),
        menu_row(Act::Send, Icon::Qr, EN.companion_home_send, None),
        menu_row(
            Act::Received,
            Icon::File,
            EN.companion_home_received,
            Some(app.kept.len().to_string()),
        ),
        menu_row(Act::Start, Icon::Power, EN.companion_home_start, None),
        menu_row(Act::Learn, Icon::Learn, EN.companion_home_learn, None),
        menu_row(Act::About, Icon::Info, EN.companion_home_about, None),
    ];
    screens::menu(c, EN.companion_title, None, rows, Vec::new())
}

/// §4.2 Receive from Faraday: the Scanner, with Paste and no Type.
fn receive(app: &Companion, c: &Chrome) -> Node {
    let (state, parts) = match &app.scan_line {
        ScanLine::Waiting => (EN.companion_scan_waiting.to_string(), None),
        ScanLine::Looking => (EN.companion_scan_looking.to_string(), None),
        ScanLine::NoCamera => (EN.companion_scan_no_camera.to_string(), None),
        ScanLine::NotRead => (EN.companion_scan_not_read.to_string(), None),
        ScanLine::NothingToPaste => (EN.companion_scan_nothing_to_paste.to_string(), None),
        ScanLine::Part {
            have,
            total,
            missing,
        } => {
            let share = (*have as f32 / (*total).max(1) as f32).min(1.0);
            let line = if missing.is_empty() {
                fill(
                    EN.companion_scan_part,
                    &[&have.to_string(), &total.to_string()],
                )
            } else {
                let list: Vec<String> = missing.iter().take(8).map(usize::to_string).collect();
                let more = if missing.len() > 8 { ", …" } else { "" };
                fill(
                    EN.companion_scan_part_missing,
                    &[
                        &have.to_string(),
                        &total.to_string(),
                        &format!("{}{more}", list.join(", ")),
                    ],
                )
            };
            (line, Some(share))
        }
    };
    let preview = app.preview.as_ref().map(|p| Preview {
        width: p.width,
        height: p.height,
        pixels: Rc::clone(&p.pixels),
        chroma: p.chroma.as_ref().map(Rc::clone),
    });
    screens::scanner(
        c,
        screens::Scanner {
            title: EN.companion_home_receive,
            state: &state,
            parts,
            ways: vec![TileSpec {
                id: Act::Paste.id(),
                icon: Icon::Paste,
                label: EN.companion_paste.to_string(),
                enabled: true,
                badge: None,
            }],
            preview,
            action: None,
        },
    )
}

/// A whole file arrived: named, with Open.
fn arrived(app: &Companion, c: &Chrome, sha: &[u8; 32], again: bool) -> Node {
    let file = app.kept.iter().find(|k| &k.sha256 == sha);
    let rows = file.map_or_else(Vec::new, |k| {
        vec![
            components::Record::text(EN.companion_name, k.name.clone(), Tone::Text),
            components::Record::text(EN.companion_kind, kind_name(k.kind), Tone::Text),
            components::Record::text(EN.companion_size, size(k.bytes.len()), Tone::Text),
        ]
    });
    let mut actions = vec![Action::new(Act::ScanAgain.id(), EN.companion_scan_again)];
    if file.is_some() {
        actions.push(Action::new(Act::Open.id(), EN.companion_open));
    }
    screens::result(
        c,
        screens::Result {
            title: EN.companion_home_receive,
            icon: Icon::Check,
            tone: Tone::Success,
            result: if again {
                EN.companion_already_kept
            } else {
                EN.companion_kept
            },
            rows,
            actions,
            caption: None,
        },
    )
}

/// A secret refused: its one line, and nothing kept.
fn refused(app: &Companion, c: &Chrome, s: Secret) -> Node {
    let sending = app.stack.last() == Some(&Page::SendList);
    let (title, actions) = if sending {
        (
            EN.companion_home_send,
            vec![Action::new(Act::Done.id(), EN.companion_done)],
        )
    } else {
        (
            EN.companion_home_receive,
            vec![
                Action::new(Act::Done.id(), EN.companion_done),
                Action::new(Act::ScanAgain.id(), EN.companion_scan_again),
            ],
        )
    };
    screens::result(
        c,
        screens::Result {
            title,
            icon: Icon::Error,
            tone: Tone::Danger,
            result: refusal(s),
            rows: Vec::new(),
            actions,
            caption: None,
        },
    )
}

/// Text, an address or a web address, shown and not kept.
fn shown(c: &Chrome, s: &Shown) -> Node {
    match s {
        Shown::Text(t) => screens::reading(
            c,
            screens::Reading {
                title: EN.companion_text_title,
                rows: Vec::new(),
                paragraphs: t.lines().map(str::to_string).collect(),
                actions: vec![
                    Action::new(Act::ScanAgain.id(), EN.companion_scan_again),
                    Action::new(Act::Keep.id(), EN.companion_keep),
                ],
            },
        ),
        Shown::Address { text, silent } => screens::compare(
            c,
            screens::Compare {
                title: EN.companion_address_title,
                label: Some(if *silent {
                    EN.companion_address_silent
                } else {
                    EN.companion_address_bitcoin
                }),
                value: text,
                id: VALUE,
                done: Action::new(Act::ScanAgain.id(), EN.companion_scan_again),
                key: None,
                copy: None,
                caption: None,
            },
        ),
        Shown::Web(t) => screens::reading(
            c,
            screens::Reading {
                title: EN.companion_web_title,
                rows: Vec::new(),
                paragraphs: vec![t.clone(), EN.companion_web_not_opened.to_string()],
                actions: vec![Action::new(Act::ScanAgain.id(), EN.companion_scan_again)],
            },
        ),
    }
}

/// A BitLocker recovery key: masked, kept nowhere.
fn bitlocker(app: &Companion, c: &Chrome) -> Node {
    let Some(k) = app.bitlocker.as_ref() else {
        return home(app, c);
    };
    screens::secret(
        c,
        screens::Secret {
            title: EN.companion_bitlocker_title,
            value: screens::Value::Text(k.key.as_str()),
            revealed: app.revealed,
            panel: PANEL,
            eye: Some(Act::Eye.id()),
            remaining: None,
            rows: vec![
                components::Record::text(
                    EN.companion_bitlocker_computer,
                    k.computer.clone(),
                    Tone::Text,
                ),
                components::Record::text(EN.companion_bitlocker_drive, k.drive.clone(), Tone::Text),
            ],
            action: Some(Action::new(Act::Done.id(), EN.companion_done)),
            secondary: None,
            pager: None,
        },
    )
}

/// §4.3: the file to send, from Received or the document picker.
fn send_list(app: &Companion, c: &Chrome) -> Node {
    let mut rows = vec![Row::Action {
        id: Act::Choose.id(),
        icon: Icon::File,
        label: EN.companion_send_choose.to_string(),
    }];
    let kept = app.received();
    if kept.is_empty() {
        rows.push(Row::Dimmed {
            icon: None,
            label: EN.companion_send_nothing.to_string(),
            reason: None,
        });
    }
    for (i, k) in kept.iter().enumerate() {
        rows.push(Row::Menu {
            id: Act::SendKept(i).id(),
            icon: None,
            label: k.name.clone(),
            value: Some(kind_name(k.kind).to_string()),
            tone: Tone::Text,
        });
    }
    screens::menu(c, EN.companion_home_send, None, rows, Vec::new())
}

/// §4.3: the codes, with Animated, the format and the frame rate.
fn send(app: &Companion, c: &Chrome) -> Node {
    let Some(s) = app.sending.as_ref() else {
        return send_list(app, c);
    };
    let Some(matrix) = s.frames.get(s.frame).cloned() else {
        return send_list(app, c);
    };
    let n = s.frames.len();
    let label = made_line(s.made);
    let next = match s.next_format() {
        Format::Ur => EN.companion_show_as_ur,
        Format::Bbqr => EN.companion_show_as_bbqr,
        Format::Parts => EN.companion_show_as_parts,
    };
    let per_second = (1000 + app.speed_ms / 2) / app.speed_ms.max(1);
    let actions = if n > 1 {
        vec![Action::new(
            Act::Rate.id(),
            fill(EN.companion_rate, &[&per_second.to_string()]),
        )]
    } else {
        Vec::new()
    };
    screens::qr_with_actions(
        c,
        screens::Qr {
            title: &s.name,
            matrix,
            label: &label,
            toggle: (s.can_animate).then_some((Act::Animated.id(), EN.companion_animated)),
            animated: s.animated,
            forced: s.forced.then(|| EN.companion_too_dense.to_string()),
            progress: (n > 1).then(|| {
                (
                    (s.frame + 1) as f32 / n as f32,
                    fill(
                        EN.companion_frame_of,
                        &[&(s.frame + 1).to_string(), &n.to_string()],
                    ),
                )
            }),
            save: Some(ActionRow {
                id: Act::Format.id(),
                icon: Icon::Swap,
                label: next.to_string(),
                reason: None,
            }),
            caption: None,
        },
        actions,
    )
}

/// A file that could not go as codes.
fn not_sent(c: &Chrome, why: NotSent) -> Node {
    let reason = match why {
        NotSent::TooLarge => EN.companion_send_too_large,
        NotSent::Empty => EN.companion_send_empty,
        NotSent::NoCode => EN.companion_send_no_code,
        NotSent::NoPicker => EN.companion_send_no_picker,
    };
    screens::result(
        c,
        screens::Result {
            title: EN.companion_home_send,
            icon: Icon::Error,
            tone: Tone::Danger,
            result: EN.companion_send_not_sent,
            rows: vec![components::Record::text(
                EN.companion_reason,
                reason,
                Tone::Text,
            )],
            actions: vec![Action::new(Act::Done.id(), EN.companion_done)],
            caption: None,
        },
    )
}

/// §4.4 Received: newest first, name, kind, size, when.
fn received(app: &Companion, c: &Chrome) -> Node {
    let mut rows = Vec::new();
    if app.unreadable {
        rows.push(Row::Fact {
            label: EN.companion_unreadable.to_string(),
            value: EN.companion_unreadable_value.to_string(),
            mono: false,
            tone: Tone::Caution,
        });
        rows.push(Row::Action {
            id: Act::DeleteStore.id(),
            icon: Icon::Trash,
            label: EN.companion_unreadable_delete.to_string(),
        });
    }
    let kept = app.received();
    if kept.is_empty() && !app.unreadable {
        rows.push(Row::Dimmed {
            icon: None,
            label: EN.companion_received_empty.to_string(),
            reason: None,
        });
    }
    let offset = app.offset_s();
    for (i, k) in kept.iter().enumerate() {
        rows.push(Row::Menu {
            id: Act::Record(i).id(),
            icon: None,
            label: k.name.clone(),
            value: Some(format!(
                "{} · {} · {}",
                kind_name(k.kind),
                size(k.bytes.len()),
                when(k.time, offset)
            )),
            tone: Tone::Text,
        });
    }
    let actions = if kept.is_empty() {
        Vec::new()
    } else {
        vec![Action::new(Act::DeleteAll.id(), EN.companion_delete_all)]
    };
    screens::menu(c, EN.companion_home_received, None, rows, actions)
}

/// A kept file's Record: Show as QR, Save, Delete.
fn record(app: &Companion, c: &Chrome, sha: &[u8; 32]) -> Node {
    let Some(k) = app.kept.iter().find(|k| &k.sha256 == sha) else {
        return received(app, c);
    };
    let mut rows = vec![
        components::Record::text(EN.companion_kind, kind_name(k.kind), Tone::Text),
        components::Record::text(EN.companion_size, size(k.bytes.len()), Tone::Text),
        components::Record::text(EN.companion_when, when(k.time, app.offset_s()), Tone::Text),
        components::Record::mono(EN.companion_sha256, hex(&k.sha256)),
    ];
    if let Some((s, ok)) = app.saved
        && &s == sha
    {
        rows.push(components::Record::text(
            EN.companion_save,
            if ok {
                EN.companion_saved
            } else {
                EN.companion_not_saved
            },
            if ok { Tone::Success } else { Tone::Caution },
        ));
    }
    rows.push(components::Record::action(
        Act::ShowAsQr.id(),
        Icon::Qr,
        EN.companion_show_as_qr,
    ));
    rows.push(components::Record::action(
        Act::Save.id(),
        Icon::File,
        EN.companion_save,
    ));
    rows.push(components::Record::action(
        Act::Delete.id(),
        Icon::Trash,
        EN.companion_delete,
    ));
    screens::record(
        c,
        screens::Record {
            title: &k.name,
            key: None,
            network: Network::Mainnet,
            rows,
            warnings: Vec::new(),
            pager: None,
            action: None,
        },
    )
}

/// A deletion, asked once.
fn confirm(app: &Companion, c: &Chrome, what: Confirm) -> Node {
    let (ask, rows) = match what {
        Confirm::One(sha) => (
            EN.companion_delete_one,
            app.kept
                .iter()
                .find(|k| k.sha256 == sha)
                .map(|k| {
                    vec![components::Record::text(
                        EN.companion_name,
                        k.name.clone(),
                        Tone::Text,
                    )]
                })
                .unwrap_or_default(),
        ),
        Confirm::All => (
            EN.companion_delete_all_ask,
            vec![components::Record::text(
                EN.companion_files,
                app.kept.len().to_string(),
                Tone::Text,
            )],
        ),
        Confirm::Store => (EN.companion_delete_store_ask, Vec::new()),
    };
    screens::result(
        c,
        screens::Result {
            title: EN.companion_delete,
            icon: Icon::Warning,
            tone: Tone::Caution,
            result: ask,
            rows,
            actions: vec![
                Action::new(Act::KeepIt.id(), EN.companion_keep_it),
                Action::new(Act::ConfirmDelete.id(), EN.companion_delete),
            ],
            caption: None,
        },
    )
}

/// Learn, or Start from the stick: one row per page.
fn doc_list(c: &Chrome, title: &str, docs: &[Doc], act: fn(usize) -> Act) -> Node {
    let rows = docs
        .iter()
        .enumerate()
        .map(|(i, d)| Row::Menu {
            id: act(i).id(),
            icon: None,
            label: doc_page(*d).title.to_string(),
            value: None,
            tone: Tone::Text,
        })
        .collect();
    screens::menu(c, title, None, rows, Vec::new())
}

/// One page, as the device shows it.
fn document(c: &Chrome, d: Doc) -> Node {
    let page = doc_page(d);
    let sections = page
        .sections
        .iter()
        .map(|s| Section {
            id: None,
            heading: s.heading.to_string(),
            paragraphs: s.paragraphs.iter().map(|p| p.to_string()).collect(),
        })
        .collect();
    screens::document(c, page.title, sections)
}

/// §4.8 About.
fn about(app: &Companion, c: &Chrome) -> Node {
    let fact = |label: &str, value: String, mono: bool| Row::Fact {
        label: label.to_string(),
        value,
        mono,
        tone: Tone::Text,
    };
    let not_reported = || EN.companion_not_reported.to_string();
    let s = app.system.as_ref();
    let permissions = s.map_or_else(not_reported, |s| {
        if s.permissions.is_empty() {
            EN.companion_none.to_string()
        } else {
            s.permissions.join(", ")
        }
    });
    let rows = vec![
        fact(EN.companion_version, crate::VERSION.to_string(), false),
        fact(
            EN.companion_build,
            s.map_or_else(not_reported, |s| s.build.clone()),
            false,
        ),
        fact(
            EN.companion_certificate,
            s.map_or_else(not_reported, |s| hex(&s.certificate_sha256)),
            s.is_some(),
        ),
        fact(EN.companion_network, EN.companion_none.to_string(), false),
        fact(EN.companion_permissions, permissions, false),
    ];
    screens::reading(
        c,
        screens::Reading {
            title: EN.companion_home_about,
            rows,
            paragraphs: vec![
                EN.companion_about_check.to_string(),
                EN.companion_about_certificate.to_string(),
            ],
            actions: Vec::new(),
        },
    )
}

/// Every string `node` draws, in tree order: copied from
/// `opensigner-core`'s `OpenSigner::texts` (upstream `d26b84e`).
pub(crate) fn texts(node: &Node, out: &mut Vec<String>) {
    match node.as_widget() {
        Some(Widget::ChunkedString { masked: true, .. }) => {}
        Some(
            Widget::Text { text, .. }
            | Widget::ChunkedString { text, .. }
            | Widget::Button { label: text, .. }
            | Widget::HoldButton { label: text, .. }
            | Widget::Chip { label: text, .. }
            | Widget::Badge { label: text, .. },
        ) => out.push(text.clone()),
        Some(Widget::ListRow {
            title,
            subtitle,
            trailing,
            ..
        }) => {
            out.push(title.clone());
            for t in [subtitle, trailing].into_iter().flatten() {
                out.push(t.clone());
            }
        }
        Some(Widget::LabelledValue { label, value, .. }) => {
            out.push(label.clone());
            out.push(value.clone());
        }
        Some(Widget::Tile { label, badge, .. }) => {
            out.push(label.clone());
            if let Some(badge) = badge {
                out.push(badge.clone());
            }
        }
        _ => {}
    }
    for c in &node.children {
        texts(c, out);
    }
}
