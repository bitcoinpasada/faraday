//! The Companion's screens, drawn each frame from its state with
//! Faraday's own drawing (`faraday-ui`), as Faraday Starter's are
//! (`docs/COMPANION.md` §4): the wordmark with the Companion's name on
//! Home, a bar with the way back on every other page, Faraday's cards,
//! rows, buttons and sheets, at the touch sizes of the stick's small
//! panel. The scanner and the codes are drawn as the desktop app's Scan
//! and QR sheets are; what a scan brings is a sheet over the scanner, as
//! the desktop shows a scan's result. Nothing here changes the state.

use faraday_intake::Secret;
use faraday_qr::bitlocker::KeyText;
use faraday_qr::send::{Format, Made};
use faraday_strings::{EN, Strings, fill};
use faraday_ui::pal::*;
use faraday_ui::parts::{measured, qr_code};
use faraday_ui::{NameSuffix, Slot, Style, Theme, W, grouped};
use osk_ui::Color;
use osk_ui::widgets::Icon;
use osk_ui::widgets::keyboard::KeyboardKind;

use crate::kept;
use crate::store::{Kept, Kind};
use crate::{Act, Closed, Companion, Confirm, Doc, NotSent, Page, ScanLine, Shown, UnlockFor};

/// Faraday's drawing, a press being one of the Companion's [`Act`]s.
type Ui<'a> = faraday_ui::Ui<'a, Act>;

const S: &Strings = &EN;

/// The small panel's side margin.
const M: f32 = 12.0;
/// The bar across the top of every page but Home.
const BAR_H: f32 = 44.0;
/// The widest the column grows on a wide display.
const COL_W: f32 = 720.0;
/// A button's height at the small panel's touch size.
const BUTTON_H: f32 = 46.0;

// ---------------------------------------------------------------------
// What the pages list
// ---------------------------------------------------------------------

/// The `osk-learn` pages Faraday's apps show on their screens, in
/// `osk-learn`'s order: none that describes OpenSigner's own device
/// (Start here, the secure element) or that Faraday never shows.
fn osk_pages() -> Vec<usize> {
    let en = &osk_learn::EN;
    let shown: [&osk_learn::Page; 24] = [
        &en.words,
        &en.transactions,
        &en.randomness,
        &en.where_randomness,
        &en.backups,
        &en.encrypted_backups,
        &en.seed_xor,
        &en.passphrases,
        &en.verifying,
        &en.air_gap,
        &en.scams,
        &en.multisig,
        &en.spend_paths,
        &en.xpubs,
        &en.inheritance,
        &en.nonces,
        &en.message,
        &en.glossary,
        &en.tools,
        &en.wallet_kinds,
        &en.frost,
        &en.other_backups,
        &en.coordinators,
        &en.silent_payments,
    ];
    en.pages()
        .iter()
        .enumerate()
        .filter(|(_, p)| shown.iter().any(|s| std::ptr::eq(*s, **p)))
        .map(|(i, _)| i)
        .collect()
}

/// The pages Start from the stick lists: the boot guides, then the
/// makers' pages.
pub(crate) fn start_docs() -> Vec<Doc> {
    faraday_guides::BOOT
        .iter()
        .copied()
        .chain(faraday_learn::MAKERS.iter().map(|m| m.id))
        .map(Doc::Own)
        .collect()
}

/// Faraday's own pages that Start from the stick does not list, the
/// Companion's own page left out (it is Learn's first).
fn own_rest() -> Vec<Doc> {
    let start = start_docs();
    faraday_learn::ids()
        .chain(faraday_guides::ids())
        .map(Doc::Own)
        .filter(|d| !start.contains(d) && *d != COMPANION)
        .collect()
}

/// The Companion's own page, first in Learn.
const COMPANION: Doc = Doc::Own("companion");

/// Every page Learn lists (§4.7): the Companion's own page, then
/// Faraday's, the boot and maker pages before the rest, then
/// `osk-learn`'s that Faraday shows.
pub(crate) fn learn_docs() -> Vec<Doc> {
    std::iter::once(COMPANION)
        .chain(start_docs())
        .chain(own_rest())
        .chain(osk_pages().into_iter().map(Doc::Osk))
        .collect()
}

/// A page's text.
fn doc_page(d: Doc) -> &'static osk_learn::Page {
    match d {
        Doc::Osk(i) => {
            let pages = osk_learn::EN.pages();
            pages[i.min(pages.len() - 1)]
        }
        Doc::Own(id) => faraday_guides::page(id).expect("a known page"),
    }
}

// ---------------------------------------------------------------------
// Words
// ---------------------------------------------------------------------

/// What a kind of kept file is called.
pub(crate) fn kind_name(k: Kind) -> &'static str {
    match k {
        Kind::Psbt => S.companion_kind_psbt,
        Kind::Transaction => S.companion_kind_transaction,
        Kind::Description => S.companion_kind_description,
        Kind::File => S.companion_kind_file,
        Kind::Text => S.companion_kind_text,
    }
}

/// The line a refusal shows (`docs/COMPANION.md` §5).
pub(crate) fn refusal(s: Secret) -> &'static str {
    match s {
        Secret::Bip39Words => S.companion_refuse_words,
        Secret::SeedQr => S.companion_refuse_seedqr,
        Secret::CompactSeedQr => S.companion_refuse_compact_seedqr,
        Secret::Codex32 => S.companion_refuse_codex32,
        Secret::Slip39Share => S.companion_refuse_slip39,
        Secret::ExtendedPrivateKey => S.companion_refuse_xprv,
        Secret::Wif => S.companion_refuse_wif,
        Secret::UrSeed => S.companion_refuse_ur_seed,
        Secret::UrBip39 => S.companion_refuse_ur_bip39,
        Secret::UrPrivateKey => S.companion_refuse_ur_private_key,
        Secret::EncryptedBackup => S.companion_refuse_backup,
        Secret::KeePass => S.companion_refuse_keepass,
        Secret::Vault => S.companion_refuse_vault,
        Secret::ThresholdCarry => S.companion_refuse_carry,
        Secret::Totp => S.companion_refuse_totp,
        Secret::TotpMigration => S.companion_refuse_totp_migration,
        Secret::VaultEntries => S.companion_refuse_entries,
    }
}

/// A size as a person reads it.
fn size(n: usize) -> String {
    if n < 1024 {
        fill(S.companion_bytes, &[&n.to_string()])
    } else {
        let tenths = (n * 10).div_ceil(1024);
        let kib = format!("{}.{}", tenths / 10, tenths % 10);
        fill(S.companion_kib, &[&kib])
    }
}

/// A time kept, as the phone's clock shows it: `2026-10-12 14:02`.
fn when(unix_s: u64, offset_s: i32) -> String {
    if unix_s == 0 {
        return S.companion_time_unknown.to_string();
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

/// A kept file's line: kind, size, when.
fn file_line(app: &Companion, k: &Kept) -> String {
    format!(
        "{} · {} · {}",
        kind_name(k.kind),
        size(k.bytes.len()),
        when(k.time, app.offset_s())
    )
}

/// The line under the codes.
fn made_line(m: Made) -> String {
    let n = |k: usize| k.to_string();
    match m {
        Made::UrPsbtOne => S.companion_made_psbt_one.to_string(),
        Made::UrPsbtFrames(k) => fill(S.companion_made_psbt_frames, &[&n(k)]),
        Made::TextOne | Made::PartsOne => S.companion_made_text_one.to_string(),
        Made::KeyOne => S.companion_made_key_one.to_string(),
        Made::UrAccount => S.companion_made_account.to_string(),
        Made::UrBytesOne { file: false } => S.companion_made_bytes_one.to_string(),
        Made::UrBytesOne { file: true } => S.companion_made_file_one.to_string(),
        Made::UrBytesFrames {
            file: false,
            frames,
        } => fill(S.companion_made_bytes_frames, &[&n(frames)]),
        Made::UrBytesFrames { file: true, frames } => {
            fill(S.companion_made_file_frames, &[&n(frames)])
        }
        Made::BbqrOne(t) => fill(S.companion_made_bbqr_one, &[&t.to_string()]),
        Made::BbqrFrames(t, k) => fill(S.companion_made_bbqr_frames, &[&t.to_string(), &n(k)]),
        Made::PartsNumbered(k) => fill(S.companion_made_parts, &[&n(k)]),
    }
}

/// A kept file by its SHA-256.
fn kept<'a>(app: &'a Companion, sha: &[u8; 32]) -> Option<&'a Kept> {
    app.kept.iter().find(|k| &k.sha256 == sha)
}

// ---------------------------------------------------------------------
// The frame
// ---------------------------------------------------------------------

/// The page drawn under a sheet, and whether the page on show is one:
/// what a scan brings, a file not sent and a deletion are sheets over
/// the page they were reached from.
fn base(app: &Companion) -> (&Page, bool) {
    let under = || app.stack.last().unwrap_or(&Page::Home);
    match &app.page {
        // What another app shared is a sheet over Home.
        Page::Arrived { .. } | Page::Shown(_) | Page::Refused(_) | Page::NotRead if app.shared => {
            (&Page::Home, true)
        }
        Page::NotRead => (&Page::Home, true),
        Page::NotDescription(_) | Page::BackInWindows(_) | Page::Closed(_) => (under(), true),
        Page::Arrived { .. } | Page::Shown(_) => (&Page::Receive, true),
        Page::Refused(_) if under() == &Page::SendList => (&Page::SendList, true),
        Page::Refused(_) => (&Page::Receive, true),
        Page::NotSent(_) => (&Page::SendList, true),
        Page::Confirm(_) => (under(), true),
        p => (p, false),
    }
}

/// Draws the frame. Returns how far the page, or the sheet over it, can
/// scroll, units.
pub(crate) fn frame(app: &Companion, ui: &mut Ui) -> f32 {
    let (w, h) = (app.w, app.h);
    ui.clear(BG);
    ui.oy = app.inset_top;
    let (page, sheet) = base(app);
    let compact = app.compact;
    let top = if *page == Page::Home { 0.0 } else { BAR_H };
    let (x, cw) = if compact {
        (M, w - 2.0 * M)
    } else {
        let cw = (w - 96.0).min(COL_W);
        ((w - cw) / 2.0, cw)
    };
    let clip = ui.rect(0.0, top, w, h - top);
    ui.c.push_clip(clip);
    let pad = if compact { 12.0 } else { 28.0 };
    let offset = if sheet { 0.0 } else { app.offset };
    let y0 = top + pad - offset;
    let end = match page {
        Page::Home => home(app, ui, x, y0, cw),
        Page::Receive => receive(app, ui, x, y0, cw),
        Page::Descriptions => descriptions(app, ui, x, y0, cw),
        Page::Description(n) => description(app, ui, x, y0, cw, *n),
        Page::Rename(_) => rename(app, ui, x, y0, cw),
        Page::Keys => keys(app, ui, x, y0, cw),
        Page::Key(n) => key_record(app, ui, x, y0, cw, *n),
        Page::Unlocking(_) => unlocking(ui, x, y0, cw),
        Page::SendList => send_list(app, ui, x, y0, cw),
        Page::Send => send(app, ui, x, y0, cw),
        Page::Received => received(app, ui, x, y0, cw),
        Page::Record(sha) => record(app, ui, x, y0, cw, sha),
        Page::Learn => learn(ui, x, y0, cw),
        Page::Start => doc_rows(ui, x, y0, cw, &start_docs(), 0, Act::StartPage),
        Page::Doc(d) => document(ui, x, y0, cw, *d),
        Page::About => about(app, ui, x, y0, cw),
        _ => y0,
    };
    ui.c.pop_clip();
    if !sheet {
        let content = end - y0 + pad + 16.0;
        ui.report_scroll(clip, (content - (h - top)).max(0.0));
    }
    if *page != Page::Home {
        head(app, ui, page, w);
    }
    if sheet {
        ui.hits.clear();
        ui.in_sheet = true;
        ui.backdrop();
        match &app.page {
            Page::Arrived { sha256, again } => arrived_sheet(app, ui, w, h, sha256, *again),
            Page::Refused(s) => refused_sheet(app, ui, w, h, *s),
            Page::Shown(s) => shown_sheet(ui, w, h, s, app.shared),
            Page::NotSent(why) => not_sent_sheet(ui, w, h, *why),
            Page::Confirm(what) => confirm_sheet(app, ui, w, h, *what),
            Page::NotDescription(why) => not_description_sheet(ui, w, h, why),
            Page::BackInWindows(_) => back_in_windows_sheet(ui, w, h),
            Page::Closed(why) => closed_sheet(app, ui, w, h, *why),
            Page::NotRead => faraday_ui::sheet::kv_sheet(
                ui,
                w,
                h,
                (Icon::Warning, WARN, S.companion_shared_not_read),
                "",
                &[],
                &[(S.companion_done, Style::Primary, Act::Done)],
            ),
            _ => {}
        }
        ui.in_sheet = false;
    }
    if let Page::Rename(_) = page
        && !sheet
    {
        let kh = faraday_ui::keyboard::height(KeyboardKind::Passphrase);
        faraday_ui::keyboard::keyboard(
            ui,
            0.0,
            w,
            h - kh,
            KeyboardKind::Passphrase,
            app.keys,
            true,
            Act::Osk,
        );
    }
    ui.scrolled
        .iter()
        .find(|s| s.slot == Slot::Page)
        .map_or(0.0, |s| s.max)
}

/// A page's name, as its bar says it.
fn page_name(app: &Companion, page: &Page) -> String {
    match page {
        Page::Receive if app.scan_kept => S.companion_descriptions_add.to_string(),
        Page::Receive => S.companion_home_receive.to_string(),
        Page::Descriptions => S.companion_home_descriptions.to_string(),
        Page::Description(n) => kept_store(app)
            .and_then(|k| k.descriptions.get(*n))
            .map_or_else(
                || S.companion_home_descriptions.to_string(),
                |d| d.name.clone(),
            ),
        Page::Rename(_) => S.companion_rename.to_string(),
        Page::Keys => S.companion_home_bitlocker.to_string(),
        Page::Key(_) => S.companion_bitlocker_title.to_string(),
        Page::Unlocking(UnlockFor::Keys | UnlockFor::KeepKey) => {
            S.companion_home_bitlocker.to_string()
        }
        Page::Unlocking(_) => S.companion_home_descriptions.to_string(),
        Page::SendList => S.companion_home_send.to_string(),
        Page::Send => app
            .sending
            .as_ref()
            .map_or_else(|| S.companion_home_send.to_string(), |s| s.name.clone()),
        Page::Received => S.companion_home_received.to_string(),
        Page::Record(sha) => {
            kept(app, sha).map_or_else(|| S.companion_home_received.to_string(), |k| k.name.clone())
        }
        Page::Learn => S.companion_home_learn.to_string(),
        Page::Start => S.companion_home_start.to_string(),
        Page::Doc(d) => doc_page(*d).title.to_string(),
        Page::About => S.companion_home_about.to_string(),
        _ => S.companion_home.to_string(),
    }
}

/// The bar across the top: the way back, where it goes, and the page's
/// name, as the Starter's small-panel bar.
fn head(app: &Companion, ui: &mut Ui, page: &Page, w: f32) {
    let back = Act::Back;
    // Where Back goes: the page under this one.
    let below = if base(app).1 {
        app.stack.iter().rev().nth(1)
    } else {
        app.stack.last()
    };
    let back_to = match below {
        None | Some(Page::Home) => S.companion_home.to_string(),
        Some(p) => page_name(app, p),
    };
    let name = page_name(app, page);
    ui.fill(0.0, 0.0, w, BAR_H, 0.0, SIDEBAR);
    ui.fill(0.0, BAR_H - 1.0, w, 1.0, 0.0, LINE);
    if ui.is_pressed(back) {
        ui.fill(4.0, 4.0, BAR_H - 8.0, BAR_H - 8.0, 8.0, INNER);
    }
    ui.icon(4.0, 4.0, BAR_H - 8.0, Icon::ChevronLeft, 13.0, TEXT);
    ui.hit(0.0, 0.0, BAR_H + 8.0, BAR_H, back);
    ui.back(back);
    let x = BAR_H;
    let label = ui.fit(11.0, W::R, &back_to, w - x - M);
    ui.text(x, 5.0, 11.0, W::R, MUTED, &label);
    let name = ui.fit(16.0, W::S, &name, w - x - M);
    ui.text(x, 19.0, 16.0, W::S, TEXT, &name);
}

// ---------------------------------------------------------------------
// The pieces, as the Starter's
// ---------------------------------------------------------------------

/// A section's label. Returns its height.
fn label(ui: &mut Ui, x: f32, y: f32, s: &str) -> f32 {
    ui.text(x, y, 13.0, W::S, MUTED, s);
    24.0
}

/// A line of fact, wrapped. Returns its height with the gap under it.
fn line(ui: &mut Ui, x: f32, y: f32, w: f32, s: &str, tone: Color) -> f32 {
    ui.wrap(x, y, w, 13.0, W::R, tone, s) + 10.0
}

/// A label with its value under it, wrapped, and a rule. Returns its
/// height.
fn value_row(ui: &mut Ui, x: f32, y: f32, w: f32, key: &str, value: &str, tone: Color) -> f32 {
    ui.text(x, y, 12.0, W::R, MUTED, key);
    let vh = ui.wrap(x, y + 18.0, w, 14.0, W::R, tone, value);
    ui.rule(x, y + 24.0 + vh, w, INNER);
    vh + 32.0
}

/// A label over a long value in the mono face, wrapped (a SHA-256).
/// Returns its height.
fn mono_row(ui: &mut Ui, x: f32, y: f32, w: f32, key: &str, value: &str) -> f32 {
    ui.text(x, y + 8.0, 12.0, W::R, MUTED, key);
    let vh = ui.wrap(x, y + 28.0, w, 13.0, W::M, TEXT, value);
    ui.rule(x, y + 36.0 + vh, w, INNER);
    vh + 40.0
}

/// A row that opens something: an icon, its label, a second line, and a
/// value at the right. Returns its height with the gap under it.
#[allow(clippy::too_many_arguments)]
fn nav_row(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    icon: Icon,
    head: &str,
    sub: &str,
    tail: &str,
    action: Act,
) -> f32 {
    let h = 54.0;
    let pressed = ui.is_pressed(action);
    ui.fill(x, y, w, h, 10.0, if pressed { INNER } else { SURFACE });
    ui.stroke(x, y, w, h, 10.0, LINE);
    let ib = 28.0;
    ui.fill(
        x + 10.0,
        y + (h - ib) / 2.0,
        ib,
        ib,
        8.0,
        ACCENT.with_alpha(26),
    );
    ui.icon(x + 10.0, y + (h - ib) / 2.0, ib, icon, 14.0, ACCENT);
    let tx = x + 20.0 + ib;
    let tail_w = if tail.is_empty() {
        0.0
    } else {
        ui.measure(12.0, W::R, tail) + 12.0
    };
    let chevron = 22.0;
    let room = w - (tx - x) - 12.0 - tail_w - chevron;
    let head = ui.fit(14.0, W::S, head, room);
    if sub.is_empty() {
        ui.text_mid(tx, y, h, 14.0, W::S, TEXT, &head);
    } else {
        ui.text(tx, y + 9.0, 14.0, W::S, TEXT, &head);
        let sub = ui.fit(12.0, W::R, sub, w - (tx - x) - 12.0 - chevron);
        ui.text(tx, y + 30.0, 12.0, W::R, MUTED, &sub);
    }
    if !tail.is_empty() {
        ui.text_right(
            x + w - 12.0 - chevron,
            y,
            if sub.is_empty() { h } else { 30.0 },
            12.0,
            W::R,
            MUTED,
            tail,
        );
    }
    ui.icon(
        x + w - 28.0,
        y + (h - 20.0) / 2.0,
        20.0,
        Icon::ChevronRight,
        9.0,
        DIM,
    );
    ui.hit(x, y, w, h, action);
    h + 8.0
}

/// A warning: a tinted box with the line in it. Returns its height with
/// the gap under it.
fn warning(ui: &mut Ui, x: f32, y: f32, w: f32, s: &str, tone: Color) -> f32 {
    let th = measured(ui, |ui| {
        ui.wrap(x + 14.0, y + 10.0, w - 28.0, 13.0, W::S, tone, s)
    });
    let bh = th + 20.0;
    ui.fill(x, y, w, bh, 10.0, tone.with_alpha(26));
    ui.stroke(x, y, w, bh, 10.0, tone.with_alpha(110));
    ui.wrap(x + 14.0, y + 10.0, w - 28.0, 13.0, W::S, tone, s);
    bh + 12.0
}

/// Buttons side by side when every label fits its share, else one under
/// another, full width. Returns their height.
fn buttons(ui: &mut Ui, x: f32, y: f32, w: f32, items: &[(&str, Style, Act)]) -> f32 {
    if items.is_empty() {
        return 0.0;
    }
    let n = items.len() as f32;
    let share = (w - (n - 1.0) * 8.0) / n;
    let fits = items
        .iter()
        .all(|(l, ..)| ui.measure(15.0, W::S, l) + 24.0 <= share);
    if fits {
        for (i, &(l, s, a)) in items.iter().enumerate() {
            ui.button(
                x + i as f32 * (share + 8.0),
                y,
                Some(share),
                BUTTON_H,
                l,
                s,
                a,
            );
        }
        return BUTTON_H;
    }
    for (i, &(l, s, a)) in items.iter().enumerate() {
        let l = ui.fit(15.0, W::S, l, w - 24.0);
        ui.button(x, y + i as f32 * 54.0, Some(w), BUTTON_H, &l, s, a);
    }
    n * 54.0 - 8.0
}

/// A kept file as the desktop's Inbox shows one: its icon, its name, and
/// its kind, size and when, on a card; the whole card presses as
/// `action` when it has one. Returns its height.
fn file_card(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    name: &str,
    line: &str,
    action: Option<Act>,
) -> f32 {
    let h = 58.0;
    let pressed = action.is_some_and(|a| ui.is_pressed(a));
    ui.fill(x, y, w, h, 12.0, if pressed { INNER } else { SURFACE });
    ui.stroke(x, y, w, h, 12.0, LINE);
    let cy = y + 12.0;
    ui.fill(x + 12.0, cy, 32.0, 32.0, 8.0, ACCENT.with_alpha(26));
    ui.icon(x + 12.0, cy, 32.0, Icon::Download, 13.0, ACCENT);
    let chevron = if action.is_some() { 22.0 } else { 0.0 };
    let room = w - 56.0 - 12.0 - chevron;
    let n = ui.fit(14.0, W::M, name, room);
    ui.text(x + 56.0, cy, 14.0, W::M, TEXT, &n);
    let l = ui.fit(12.0, W::R, line, room);
    ui.text(x + 56.0, cy + 20.0, 12.0, W::R, MUTED, &l);
    if let Some(a) = action {
        ui.icon(
            x + w - 28.0,
            y + (h - 20.0) / 2.0,
            20.0,
            Icon::ChevronRight,
            9.0,
            DIM,
        );
        ui.hit(x, y, w, h, a);
    }
    h
}

// ---------------------------------------------------------------------
// §4.1 Home
// ---------------------------------------------------------------------

/// Home, as the Starter's: the wordmark, what is kept, then the rows.
fn home(app: &Companion, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    ui.mark(x, y, 20.0, 23.0);
    ui.wordmark_named(x + 28.0, y + 6.0, 11.0, TEXT, NameSuffix::Companion);
    y += 38.0;
    y += kept_card(app, ui, x, y, w) + 20.0;
    let count = app.kept.len().to_string();
    let (descriptions, keys) = app.kept_counts();
    let descriptions = descriptions.to_string();
    let key_count = keys.to_string();
    let mut rows: Vec<(Icon, &str, &str, Act)> = vec![
        (Icon::Scan, S.companion_home_receive, "", Act::Receive),
        (Icon::Qr, S.companion_home_send, "", Act::Send),
        (Icon::File, S.companion_home_received, &count, Act::Received),
        (
            Icon::Wallet,
            S.companion_home_descriptions,
            &descriptions,
            Act::Descriptions,
        ),
    ];
    // The BitLocker keys row only while one is kept (§4.1).
    if keys > 0 {
        rows.push((
            Icon::Lock,
            S.companion_home_bitlocker,
            &key_count,
            Act::Keys,
        ));
    }
    rows.extend([
        (Icon::Power, S.companion_home_start, "", Act::Start),
        (Icon::Learn, S.companion_home_learn, "", Act::Learn),
        (Icon::Info, S.companion_home_about, "", Act::About),
    ]);
    for (icon, head, tail, act) in rows {
        y += nav_row(ui, x, y, w, icon, head, "", tail, act);
    }
    y
}

/// What is kept on this phone: the count, the newest file, and a store
/// that could not be read. Returns its height.
fn kept_card(app: &Companion, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let pad = if app.compact { 12.0 } else { 18.0 };
    let iw = w - 2.0 * pad;
    let newest = app.kept.last();
    let draw = |ui: &mut Ui| {
        let mut y = y0 + pad;
        ui.text(x + pad, y, 12.0, W::S, MUTED, S.companion_on_phone);
        y += 20.0;
        let count = app.kept.len().to_string();
        y += fact(ui, x + pad, y, iw, S.companion_home_received, &count, TEXT);
        if let Some(k) = newest {
            y += fact(ui, x + pad, y, iw, S.companion_newest, &k.name, TEXT);
        }
        if app.unreadable {
            y += fact(
                ui,
                x + pad,
                y,
                iw,
                S.companion_unreadable,
                S.companion_unreadable_value,
                WARN,
            );
        }
        y + pad - 4.0 - y0
    };
    let h = measured(ui, draw);
    ui.card(x, y0, w, h, LINE);
    draw(ui);
    h
}

/// A label at the left and its value at the right, on one line. Returns
/// its height.
#[allow(clippy::too_many_arguments)]
fn fact(ui: &mut Ui, x: f32, y: f32, w: f32, key: &str, value: &str, tone: Color) -> f32 {
    let kw = ui.measure(13.0, W::R, key) + 16.0;
    ui.text_mid(x, y, 30.0, 13.0, W::R, MUTED, key);
    let v = ui.fit(14.0, W::S, value, w - kw);
    ui.text_right(x + w, y, 30.0, 14.0, W::S, tone, &v);
    30.0
}

// ---------------------------------------------------------------------
// §4.2 Scan a Faraday QR code
// ---------------------------------------------------------------------

/// The scanner, as the desktop's Scan sheet: the line, the camera's
/// picture in its box, the parts read, and Paste.
fn receive(app: &Companion, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let pad = 16.0;
    let iw = w - 2.0 * pad;
    let (state, tone) = match &app.scan_line {
        ScanLine::Waiting => (S.companion_scan_waiting, MUTED),
        ScanLine::Looking => (S.companion_scan_looking, MUTED),
        ScanLine::Part { .. } => (S.companion_scan_looking, MUTED),
        ScanLine::NoCamera => (S.companion_scan_no_camera, WARN),
        ScanLine::NotRead => (S.companion_scan_not_read, WARN),
        ScanLine::NothingToPaste => (S.companion_scan_nothing_to_paste, WARN),
    };
    let parts = match &app.scan_line {
        ScanLine::Part {
            have,
            total,
            missing,
        } => {
            let share = (*have as f32 / (*total).max(1) as f32).min(1.0);
            let line = if missing.is_empty() {
                fill(
                    S.companion_scan_part,
                    &[&have.to_string(), &total.to_string()],
                )
            } else {
                let list: Vec<String> = missing.iter().take(8).map(usize::to_string).collect();
                let more = if missing.len() > 8 { ", …" } else { "" };
                fill(
                    S.companion_scan_part_missing,
                    &[
                        &have.to_string(),
                        &total.to_string(),
                        &format!("{}{more}", list.join(", ")),
                    ],
                )
            };
            Some((line, share))
        }
        _ => None,
    };
    // The box as tall as it is wide, less what the page needs under it.
    let side_h = iw.min(app.h - BAR_H - 230.0).max(160.0);
    let parts_h = if parts.is_some() { 40.0 } else { 0.0 };
    let h = pad + 26.0 + side_h + 12.0 + parts_h + BUTTON_H + pad;
    ui.card(x, y0, w, h, LINE);
    let mut y = y0 + pad;
    let state = ui.fit(13.0, W::R, state, iw);
    ui.text(x + pad, y, 13.0, W::R, tone, &state);
    y += 26.0;
    let rect = ui.rect(x + pad, y, iw, side_h);
    ui.fill(x + pad, y, iw, side_h, 8.0, BG);
    match app.preview.as_ref() {
        Some(p) => {
            ui.c.luma(
                rect,
                p.width,
                p.height,
                &p.luma,
                p.chroma.as_deref().map(|c| &c[..]),
            );
            outline(app, ui, rect, p.width, p.height);
        }
        None => {
            let t = S.companion_scan_waiting;
            let tw = ui.measure(13.0, W::R, t);
            ui.text_mid(x + pad + (iw - tw) / 2.0, y, side_h, 13.0, W::R, DIM, t);
        }
    }
    y += side_h + 12.0;
    if let Some((line, share)) = parts {
        let line = ui.fit(13.0, W::S, &line, iw);
        ui.text(x + pad, y, 13.0, W::S, TEXT, &line);
        ui.fill(x + pad, y + 24.0, iw, 4.0, 2.0, INNER);
        ui.fill(x + pad, y + 24.0, iw * share, 4.0, 2.0, ACCENT);
        y += parts_h;
    }
    buttons(
        ui,
        x + pad,
        y,
        iw,
        &[(S.companion_paste, Style::Secondary, Act::Paste)],
    );
    y0 + h
}

/// The code the scanner last found, outlined where it is in the
/// preview, as the desktop's Scan sheet outlines it: green once read,
/// amber while it is found and not read, gone ten frames after the
/// scanner stops seeing it.
fn outline(app: &Companion, ui: &mut Ui, rect: osk_ui::Rect, fw: u16, fh: u16) {
    let Some((seen, at)) = app.seen else {
        return;
    };
    if app.frames.wrapping_sub(at) > 10 || seen.width == 0 || seen.height == 0 {
        return;
    }
    // The preview fills its box and crops the longer side, as
    // `Canvas::luma` draws it: the same scale and crop here.
    let (fw, fh) = (f32::from(fw), f32::from(fh));
    let scale = (rect.w as f32 / fw).max(rect.h as f32 / fh);
    let left = (fw - rect.w as f32 / scale) / 2.0;
    let top = (fh - rect.h as f32 / scale) / 2.0;
    let map = |(cx, cy): (u16, u16)| {
        let px = f32::from(cx) * fw / f32::from(seen.width);
        let py = f32::from(cy) * fh / f32::from(seen.height);
        (
            rect.x as f32 + (px - left) * scale,
            rect.y as f32 + (py - top) * scale,
        )
    };
    let color = ui.col(if seen.read { OK } else { WARN });
    let lw = (3.0 * ui.f).max(2.0);
    for i in 0..4 {
        let (a, b) = (map(seen.corners[i]), map(seen.corners[(i + 1) % 4]));
        ui.c.line(a.0, a.1, b.0, b.1, lw, color);
    }
}

/// A whole file arrived and is kept: named, with Open.
fn arrived_sheet(app: &Companion, ui: &mut Ui, w: f32, h: f32, sha: &[u8; 32], again: bool) {
    let file = kept(app, sha);
    let headline = if again {
        S.companion_already_kept
    } else {
        S.companion_kept
    };
    faraday_ui::sheet::sheet(ui, w, h, &mut |ui, x, y, iw| {
        let mut cy = y;
        cy += faraday_ui::sheet::sheet_head(ui, x, cy, iw, Icon::Done, OK, headline);
        if let Some(k) = file {
            cy += faraday_ui::sheet::kv(ui, x, cy, iw, S.companion_name, &k.name, TEXT);
            cy += faraday_ui::sheet::kv(ui, x, cy, iw, S.companion_kind, kind_name(k.kind), TEXT);
            cy +=
                faraday_ui::sheet::kv(ui, x, cy, iw, S.companion_size, &size(k.bytes.len()), TEXT);
        }
        cy += 8.0;
        let mut items = vec![if app.shared {
            (S.companion_done, Style::Secondary, Act::Done)
        } else {
            (S.companion_scan_again, Style::Secondary, Act::ScanAgain)
        }];
        if file.is_some() {
            items.push((S.companion_open, Style::Primary, Act::Open));
        }
        cy += faraday_ui::sheet::buttons(ui, x, cy, iw, &items);
        cy - y
    });
}

/// A secret refused: its one line, and nothing kept.
fn refused_sheet(app: &Companion, ui: &mut Ui, w: f32, h: f32, s: Secret) {
    let sending = app.stack.last() == Some(&Page::SendList) || app.shared;
    let items: Vec<(&str, Style, Act)> = if sending {
        vec![(S.companion_done, Style::Primary, Act::Done)]
    } else {
        vec![
            (S.companion_done, Style::Secondary, Act::Done),
            (S.companion_scan_again, Style::Primary, Act::ScanAgain),
        ]
    };
    faraday_ui::sheet::sheet(ui, w, h, &mut |ui, x, y, iw| {
        let mut cy = y;
        cy += faraday_ui::sheet::sheet_head(ui, x, cy, iw, Icon::Warning, ERR, refusal(s));
        cy += 8.0;
        cy += faraday_ui::sheet::buttons(ui, x, cy, iw, &items);
        cy - y
    });
}

/// Text, an address or a web address, shown and not kept.
fn shown_sheet(ui: &mut Ui, w: f32, h: f32, s: &Shown, shared: bool) {
    faraday_ui::sheet::sheet(ui, w, h, &mut |ui, x, y, iw| {
        let mut cy = y;
        let (again, last) = if shared {
            (
                (S.companion_done, Style::Secondary, Act::Done),
                (S.companion_done, Style::Primary, Act::Done),
            )
        } else {
            (
                (S.companion_scan_again, Style::Secondary, Act::ScanAgain),
                (S.companion_scan_again, Style::Primary, Act::ScanAgain),
            )
        };
        let items: Vec<(&str, Style, Act)> = match s {
            Shown::Text(t) => {
                cy += faraday_ui::sheet::sheet_head(
                    ui,
                    x,
                    cy,
                    iw,
                    Icon::File,
                    ACCENT,
                    S.companion_text_title,
                );
                for l in t.lines() {
                    cy += ui.wrap(x, cy, iw, 14.0, W::R, TEXT, l) + 6.0;
                }
                vec![again, (S.companion_keep, Style::Primary, Act::Keep)]
            }
            Shown::Address { text, silent } => {
                cy += faraday_ui::sheet::sheet_head(
                    ui,
                    x,
                    cy,
                    iw,
                    Icon::Bitcoin,
                    ACCENT,
                    S.companion_address_title,
                );
                let kind = if *silent {
                    S.companion_address_silent
                } else {
                    S.companion_address_bitcoin
                };
                ui.text(x, cy, 12.0, W::R, MUTED, kind);
                cy += 18.0;
                cy += ui.wrap(x, cy, iw, 15.0, W::M, TEXT, &grouped(text)) + 8.0;
                vec![last]
            }
            Shown::Web(t) => {
                cy += faraday_ui::sheet::sheet_head(
                    ui,
                    x,
                    cy,
                    iw,
                    Icon::Info,
                    ACCENT,
                    S.companion_web_title,
                );
                cy += ui.wrap(x, cy, iw, 13.0, W::M, TEXT, t) + 8.0;
                cy += ui.wrap(x, cy, iw, 13.0, W::R, MUTED, S.companion_web_not_opened) + 8.0;
                vec![last]
            }
        };
        cy += 8.0;
        cy += faraday_ui::sheet::buttons(ui, x, cy, iw, &items);
        cy - y
    });
}

// ---------------------------------------------------------------------
// A BitLocker recovery key
// ---------------------------------------------------------------------

/// The eight groups, two columns of four on a phone, each numbered, its
/// digits shown only when `shown`. Returns its height with the gap under
/// it.
fn key_panel(ui: &mut Ui, x: f32, y: f32, w: f32, k: &KeyText, shown: bool) -> f32 {
    let cols = if w >= 380.0 { 4 } else { 2 };
    let rows = 8 / cols;
    let size = 17.0;
    let pad = 14.0;
    let cw = (w - 2.0 * pad) / cols as f32;
    let rh = 18.0 + ui.line(size, W::M) + 8.0;
    let h = 2.0 * pad + rows as f32 * rh - 8.0;
    ui.fill(x, y, w, h, 10.0, BG);
    ui.stroke(x, y, w, h, 10.0, WARN);
    for i in 0..8 {
        let (c, r) = (i % cols, i / cols);
        let cx = x + pad + c as f32 * cw;
        let cy = y + pad + r as f32 * rh;
        ui.text(cx, cy, 11.0, W::R, MUTED, &(i + 1).to_string());
        if shown {
            ui.text(cx, cy + 16.0, size, W::M, TEXT, k.key.group(i));
        } else {
            ui.text(cx, cy + 16.0, size, W::M, MUTED, "••••••");
        }
    }
    h + 12.0
}

// ---------------------------------------------------------------------
// §4.3 Send to Faraday
// ---------------------------------------------------------------------

/// The file to send: the document picker, then what is in Received.
fn send_list(app: &Companion, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    y += buttons(
        ui,
        x,
        y,
        w,
        &[(S.companion_send_choose, Style::Primary, Act::Choose)],
    ) + 20.0;
    y += label(ui, x, y, S.companion_home_received);
    let files = app.received();
    if files.is_empty() {
        y += line(ui, x, y, w, S.companion_send_nothing, DIM);
    }
    for (i, k) in files.iter().enumerate() {
        y += file_card(
            ui,
            x,
            y,
            w,
            &k.name,
            &file_line(app, k),
            Some(Act::SendKept(i)),
        ) + 10.0;
    }
    y
}

/// The codes, as the desktop's QR sheet on a small panel: the code as
/// large as the width allows, its format and frame, the format, the
/// frame rate and Animated as buttons that step through the choices.
fn send(app: &Companion, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let Some(s) = app.sending.as_ref() else {
        return y0;
    };
    let mut y = y0;
    let n = s.frames.len();
    let mut sub = made_line(s.made);
    if n > 1 {
        let of = fill(
            S.companion_frame_of,
            &[&(s.frame + 1).to_string(), &n.to_string()],
        );
        sub = format!("{sub} · {of}");
    }
    let sub = ui.fit(12.0, W::S, &sub, w);
    ui.text(x, y, 12.0, W::S, MUTED, &sub);
    y += 22.0;
    if s.forced {
        let t = ui.fit(12.0, W::R, S.companion_too_dense, w);
        ui.text(x, y, 12.0, W::R, WARN, &t);
        y += 20.0;
    }
    let side = w.min(app.h - BAR_H - 230.0).max(160.0);
    if let Some(m) = s.frames.get(s.frame) {
        qr_code(ui, m, x + (w - side) / 2.0, y, side);
    }
    y += side + 12.0;
    let next = match s.next_format() {
        Format::Ur => S.companion_show_as_ur,
        Format::Bbqr => S.companion_show_as_bbqr,
        Format::Parts => S.companion_show_as_parts,
    };
    let mut cycles: Vec<(String, Style, Act)> = Vec::new();
    if s.formats().len() > 1 {
        cycles.push((next.to_string(), Style::Secondary, Act::Format));
    }
    if n > 1 {
        let per_second = (1000 + app.speed_ms / 2) / app.speed_ms.max(1);
        cycles.push((
            fill(S.companion_rate, &[&per_second.to_string()]),
            Style::Secondary,
            Act::Rate,
        ));
    }
    let items: Vec<(&str, Style, Act)> = cycles
        .iter()
        .map(|(l, s, a)| (l.as_str(), *s, *a))
        .collect();
    y += buttons(ui, x, y, w, &items) + 8.0;
    if s.can_animate && !s.forced {
        y += switch_row(ui, x, y, w, S.companion_animated, s.animated, Act::Animated);
    }
    y += 8.0;
    y += buttons(
        ui,
        x,
        y,
        w,
        &[(S.companion_done, Style::Primary, Act::Back)],
    );
    y
}

/// A row with a check box: its label, ticked or not. Returns its height.
fn switch_row(ui: &mut Ui, x: f32, y: f32, w: f32, label: &str, on: bool, action: Act) -> f32 {
    let rh = 44.0;
    if ui.is_pressed(action) {
        ui.fill(x, y + 2.0, w, rh - 4.0, 8.0, INNER);
    }
    ui.checkbox(x + 10.0, y + (rh - 18.0) / 2.0, on, true);
    let shown = ui.fit(14.0, W::R, label, w - 50.0);
    ui.text_mid(x + 40.0, y, rh, 14.0, W::R, TEXT, &shown);
    ui.hit(x, y, w, rh, action);
    rh
}

/// A file that could not go as codes.
fn not_sent_sheet(ui: &mut Ui, w: f32, h: f32, why: NotSent) {
    let reason = match why {
        NotSent::TooLarge => S.companion_send_too_large,
        NotSent::Empty => S.companion_send_empty,
        NotSent::NoCode => S.companion_send_no_code,
        NotSent::NoPicker => S.companion_send_no_picker,
    };
    faraday_ui::sheet::kv_sheet(
        ui,
        w,
        h,
        (Icon::Warning, WARN, S.companion_send_not_sent),
        "",
        &[(S.companion_reason, reason.to_string(), TEXT)],
        &[(S.companion_done, Style::Primary, Act::Done)],
    );
}

// ---------------------------------------------------------------------
// §4.4 Received
// ---------------------------------------------------------------------

/// Received, as the desktop's Inbox: newest first, each file on its
/// card, and Delete all.
fn received(app: &Companion, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    if app.unreadable {
        let l = format!(
            "{}: {}",
            S.companion_unreadable, S.companion_unreadable_value
        );
        y += warning(ui, x, y, w, &l, WARN);
        y += buttons(
            ui,
            x,
            y,
            w,
            &[(
                S.companion_unreadable_delete,
                Style::Secondary,
                Act::DeleteStore,
            )],
        ) + 16.0;
    }
    let files = app.received();
    let lw = ui.text(x, y, 15.0, W::S, MUTED, S.companion_home_received);
    ui.text(
        x + lw + 10.0,
        y + 2.0,
        13.0,
        W::R,
        DIM,
        &files.len().to_string(),
    );
    y += 28.0;
    if files.is_empty() {
        y += line(ui, x, y, w, S.companion_received_empty, DIM);
    }
    for (i, k) in files.iter().enumerate() {
        y += file_card(
            ui,
            x,
            y,
            w,
            &k.name,
            &file_line(app, k),
            Some(Act::Record(i)),
        ) + 10.0;
    }
    if !files.is_empty() {
        y += 8.0;
        y += buttons(
            ui,
            x,
            y,
            w,
            &[(S.companion_delete_all, Style::Secondary, Act::DeleteAll)],
        );
    }
    y
}

/// A kept file, as a file in the desktop's Inbox: its card, what it is,
/// and Show as QR, Save and Delete.
fn record(app: &Companion, ui: &mut Ui, x: f32, y0: f32, w: f32, sha: &[u8; 32]) -> f32 {
    let Some(k) = kept(app, sha) else {
        return y0;
    };
    let mut y = y0;
    y += file_card(ui, x, y, w, &k.name, kind_name(k.kind), None) + 16.0;
    y += value_row(ui, x, y, w, S.companion_size, &size(k.bytes.len()), TEXT);
    y += value_row(
        ui,
        x,
        y,
        w,
        S.companion_when,
        &when(k.time, app.offset_s()),
        TEXT,
    );
    y += mono_row(ui, x, y, w, S.companion_sha256, &grouped(&hex(&k.sha256)));
    if let Some((s, ok)) = app.saved
        && &s == sha
    {
        let (t, tone) = if ok {
            (S.companion_saved, OK)
        } else {
            (S.companion_not_saved, WARN)
        };
        y += value_row(ui, x, y, w, S.companion_save, t, tone);
    }
    y += 12.0;
    y += buttons(
        ui,
        x,
        y,
        w,
        &[(S.companion_show_as_qr, Style::Primary, Act::ShowAsQr)],
    ) + 8.0;
    if k.kind == Kind::Description {
        y += buttons(
            ui,
            x,
            y,
            w,
            &[(
                S.companion_keep_in_descriptions,
                Style::Secondary,
                Act::KeepInDescriptions,
            )],
        ) + 8.0;
    }
    y += buttons(
        ui,
        x,
        y,
        w,
        &[
            (S.companion_share, Style::Secondary, Act::Share),
            (S.companion_save, Style::Secondary, Act::Save),
        ],
    ) + 8.0;
    y += buttons(
        ui,
        x,
        y,
        w,
        &[(S.companion_delete, Style::Secondary, Act::Delete)],
    );
    y
}

/// A deletion, asked once.
fn confirm_sheet(app: &Companion, ui: &mut Ui, w: f32, h: f32, what: Confirm) {
    let (ask, rows) = match what {
        Confirm::One(sha) => (
            S.companion_delete_one,
            kept(app, &sha)
                .map(|k| vec![(S.companion_name, k.name.clone(), TEXT)])
                .unwrap_or_default(),
        ),
        Confirm::All => (
            S.companion_delete_all_ask,
            vec![(S.companion_files, app.kept.len().to_string(), TEXT)],
        ),
        Confirm::Store => (S.companion_delete_store_ask, Vec::new()),
        Confirm::Description(n) => (
            S.companion_delete_description_ask,
            kept_store(app)
                .and_then(|k| k.descriptions.get(n))
                .map(|d| vec![(S.companion_name, d.name.clone(), TEXT)])
                .unwrap_or_default(),
        ),
        Confirm::Key(_) => (S.companion_delete_key_ask, Vec::new()),
        Confirm::KeptStore => (S.companion_delete_kept_store_ask, Vec::new()),
    };
    // A key's Delete asks once, as Back in Windows does, without its
    // two lines (§4.6).
    let yes = match what {
        Confirm::Key(_) => S.companion_delete_the_key,
        _ => S.companion_delete,
    };
    faraday_ui::sheet::kv_sheet(
        ui,
        w,
        h,
        (Icon::Trash, WARN, ask),
        "",
        &rows,
        &[
            (S.companion_keep_it, Style::Secondary, Act::KeepIt),
            (yes, Style::Primary, Act::ConfirmDelete),
        ],
    );
}

// ---------------------------------------------------------------------
// §4.7 Start from the stick, and Learn
// ---------------------------------------------------------------------

/// Learn: Faraday's own pages, then the ones about Bitcoin.
fn learn(ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    let start = start_docs();
    let rest = own_rest();
    let osk: Vec<Doc> = osk_pages().into_iter().map(Doc::Osk).collect();
    let own = vec![COMPANION];
    let mut at = 0;
    for (title, docs) in [
        (S.companion_title, &own),
        (S.companion_home_start, &start),
        (S.companion_learn_faraday, &rest),
        (S.companion_learn_bitcoin, &osk),
    ] {
        y += label(ui, x, y, title);
        y = doc_rows(ui, x, y, w, docs, at, Act::LearnPage) + 12.0;
        at += docs.len();
    }
    y
}

/// One row per page, numbered from `first` for `act`. Returns where the
/// rows end.
fn doc_rows(
    ui: &mut Ui,
    x: f32,
    y0: f32,
    w: f32,
    docs: &[Doc],
    first: usize,
    act: fn(usize) -> Act,
) -> f32 {
    let mut y = y0;
    ui.rule(x, y, w, INNER);
    for (i, d) in docs.iter().enumerate() {
        let a = act(first + i);
        let rh = 48.0;
        if ui.is_pressed(a) {
            ui.fill(x - 6.0, y + 2.0, w + 12.0, rh - 4.0, 8.0, INNER);
        }
        let t = ui.fit(14.0, W::S, doc_page(*d).title, w - 36.0);
        ui.text_mid(x, y, rh, 14.0, W::S, TEXT, &t);
        ui.icon(
            x + w - 24.0,
            y + (rh - 20.0) / 2.0,
            20.0,
            Icon::ChevronRight,
            9.0,
            DIM,
        );
        ui.hit(x - 6.0, y, w + 12.0, rh, a);
        y += rh;
        ui.rule(x, y, w, INNER);
    }
    y
}

/// One page, as the desktop's Learn sheet shows it.
fn document(ui: &mut Ui, x: f32, y0: f32, w: f32, d: Doc) -> f32 {
    let page = doc_page(d);
    let mut y = y0;
    y += ui.wrap(x, y, w, 18.0, W::S, TEXT, page.title) + 16.0;
    for section in page.sections {
        if !section.heading.is_empty() {
            y += ui.wrap(x, y, w, 15.0, W::S, ACCENT, section.heading) + 8.0;
        }
        for para in section.paragraphs {
            // A firmware sketch's marker (DECISIONS F18): the sketch drawn
            // in its place, and the marker never printed as text.
            if faraday_learn::is_marker(para) {
                if let Some(s) = faraday_guides::sketch_of(para) {
                    y += faraday_ui::sketch(ui, s, x, y, w) + 12.0;
                }
                continue;
            }
            y += ui.wrap(x, y, w, 14.0, W::R, TEXT, para) + 12.0;
        }
        y += 10.0;
    }
    y
}

// ---------------------------------------------------------------------
// §4.8 About
// ---------------------------------------------------------------------

/// About, as the Starter's: the version and build, the signing
/// certificate, network access and permissions, how a release is
/// checked, and the theme.
fn about(app: &Companion, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    let s = app.system.as_ref();
    let not_reported = || S.companion_not_reported.to_string();
    y += value_row(ui, x, y, w, S.companion_version, crate::VERSION, TEXT);
    y += value_row(
        ui,
        x,
        y,
        w,
        S.companion_build,
        &s.map_or_else(not_reported, |s| s.build.clone()),
        TEXT,
    );
    match s {
        Some(s) => {
            y += mono_row(
                ui,
                x,
                y,
                w,
                S.companion_certificate,
                &grouped(&hex(&s.certificate_sha256)),
            );
        }
        None => {
            y += value_row(ui, x, y, w, S.companion_certificate, &not_reported(), TEXT);
        }
    }
    y += value_row(ui, x, y, w, S.companion_network, S.companion_none, TEXT);
    match s.map(|s| &s.permissions) {
        Some(p) if !p.is_empty() => {
            y += 8.0;
            y += label(ui, x, y, S.companion_permissions);
            for (name, granted) in p {
                let state = if *granted {
                    S.companion_granted
                } else {
                    S.companion_not_granted
                };
                y += value_row(ui, x, y, w, name, state, TEXT);
            }
        }
        Some(_) => {
            y += value_row(ui, x, y, w, S.companion_permissions, S.companion_none, TEXT);
        }
        None => {
            y += value_row(ui, x, y, w, S.companion_permissions, &not_reported(), TEXT);
        }
    }
    y += 16.0;
    y += label(ui, x, y, S.companion_check_title);
    y += line(ui, x, y, w, S.companion_about_check, TEXT);
    y += line(ui, x, y, w, S.companion_about_certificate, TEXT);
    y += 16.0;
    y += label(ui, x, y, S.companion_theme);
    y + themes(app, ui, x, y, w)
}

/// The themes as tiles in their own colours, the one on screen ringed.
/// Returns their height.
fn themes(app: &Companion, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let gap = 10.0;
    let min = ui.theme_tile_min();
    let cols = (((w + gap) / (min + gap)).floor() as usize).clamp(1, 4);
    let tw = (w - gap * (cols - 1) as f32) / cols as f32;
    let th = ui.theme_tile_h(tw);
    let mut y = y0 + 4.0;
    for (i, t) in Theme::ALL.into_iter().enumerate() {
        let (c, r) = (i % cols, i / cols);
        let tx = x + c as f32 * (tw + gap);
        let ty = y + r as f32 * (th + gap);
        ui.theme_tile(tx, ty, tw, th, t, t == app.theme, Act::Theme(i));
    }
    let rows = Theme::ALL.len().div_ceil(cols);
    y += rows as f32 * (th + gap);
    y - y0
}

// ---------------------------------------------------------------------
// §4.5 Wallet descriptions, §4.6 BitLocker keys, §7 the unlock
// ---------------------------------------------------------------------

/// The kept store, while it is open.
fn kept_store(app: &Companion) -> Option<&kept::Store> {
    app.kept_store.as_ref()
}

/// A list's head, as Received's: its name and how many.
fn list_head(ui: &mut Ui, x: f32, y: f32, name: &str, n: usize) -> f32 {
    let lw = ui.text(x, y, 15.0, W::S, MUTED, name);
    ui.text(x + lw + 10.0, y + 2.0, 13.0, W::R, DIM, &n.to_string());
    28.0
}

#[allow(clippy::too_many_arguments)]
/// A card as the desktop's Inbox draws a file, with its own icon: the
/// name, its line, and the whole card pressing as `action`. Returns its
/// height.
fn item_card(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    icon: Icon,
    name: &str,
    line: &str,
    action: Act,
) -> f32 {
    let h = 58.0;
    let pressed = ui.is_pressed(action);
    ui.fill(x, y, w, h, 12.0, if pressed { INNER } else { SURFACE });
    ui.stroke(x, y, w, h, 12.0, LINE);
    let cy = y + 12.0;
    ui.fill(x + 12.0, cy, 32.0, 32.0, 8.0, ACCENT.with_alpha(26));
    ui.icon(x + 12.0, cy, 32.0, icon, 13.0, ACCENT);
    let room = w - 56.0 - 12.0 - 22.0;
    let n = ui.fit(14.0, W::M, name, room);
    ui.text(x + 56.0, cy, 14.0, W::M, TEXT, &n);
    let l = ui.fit(12.0, W::R, line, room);
    ui.text(x + 56.0, cy + 20.0, 12.0, W::R, MUTED, &l);
    ui.icon(
        x + w - 28.0,
        y + (h - 20.0) / 2.0,
        20.0,
        Icon::ChevronRight,
        9.0,
        DIM,
    );
    ui.hit(x, y, w, h, action);
    h
}

/// What a description is, as its record and the list say it: the
/// script and the kind of wallet.
fn wallet_kind(p: &osk_bip::policy::WalletPolicy) -> String {
    use osk_bip::keys::ScriptType;
    use osk_bip::policy::Template;
    let script = match p.script_type() {
        ScriptType::Legacy => S.companion_script_legacy,
        ScriptType::NestedSegwit => S.companion_script_nested,
        ScriptType::NativeSegwit => S.companion_script_native,
        ScriptType::Taproot => S.companion_script_taproot,
    };
    let kind = match p.template() {
        Template::Single { .. } => S.companion_wallet_single,
        Template::Multi { .. } => S.companion_wallet_multisig,
        Template::MuSig => S.companion_wallet_musig,
        Template::Threshold { .. } => S.companion_wallet_threshold,
        Template::Silent => S.companion_wallet_silent,
        Template::Miniscript { .. } => S.companion_wallet_miniscript,
        _ => S.companion_wallet_tree,
    };
    format!("{script} · {kind}")
}

/// A description's keys, `k of n` where the wallet states how many
/// sign, else how many keys it has.
fn k_of_n(p: &osk_bip::policy::WalletPolicy) -> String {
    use osk_bip::policy::Template;
    let n = p.keys().len();
    let k = match p.template() {
        Template::Single { .. } => Some(1),
        Template::MuSig => Some(n),
        Template::Threshold {
            threshold,
            participants,
        } => {
            return fill(
                S.companion_k_of_n,
                &[&threshold.to_string(), &participants.to_string()],
            );
        }
        _ => p.quorum().or_else(|| p.tapscript_quorum()).map(|(k, _)| k),
    };
    match k {
        Some(k) => fill(S.companion_k_of_n, &[&k.to_string(), &n.to_string()]),
        None => fill(S.companion_n_keys, &[&n.to_string()]),
    }
}

/// Wallet descriptions, as Received: newest last, each on its card,
/// and Scan or paste.
fn descriptions(app: &Companion, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let Some(store) = kept_store(app) else {
        return y0;
    };
    let mut y = y0;
    y += buttons(
        ui,
        x,
        y,
        w,
        &[(
            S.companion_descriptions_add,
            Style::Primary,
            Act::AddDescription,
        )],
    ) + 20.0;
    y += list_head(
        ui,
        x,
        y,
        S.companion_home_descriptions,
        store.descriptions.len(),
    );
    if store.descriptions.is_empty() {
        y += line(ui, x, y, w, S.companion_descriptions_empty, DIM);
    }
    for (i, d) in store.descriptions.iter().enumerate() {
        let kind = faraday_intake::wallet::read_wallet(&d.text)
            .map(|p| format!("{} · {}", wallet_kind(&p), k_of_n(&p)))
            .unwrap_or_default();
        let sub = format!("{kind} · {}", when(d.time, app.offset_s()));
        y += item_card(
            ui,
            x,
            y,
            w,
            Icon::Wallet,
            &d.name,
            &sub,
            Act::DescriptionRecord(i),
        ) + 10.0;
    }
    y
}

/// A kept description's record, as a file in the desktop's Inbox: its
/// card, then what the reader finds in it, and Show as QR, Share,
/// Rename and Delete.
fn description(app: &Companion, ui: &mut Ui, x: f32, y0: f32, w: f32, n: usize) -> f32 {
    let Some(d) = kept_store(app).and_then(|k| k.descriptions.get(n)) else {
        return y0;
    };
    let mut y = y0;
    y += file_card(ui, x, y, w, &d.name, S.companion_kind_description, None) + 16.0;
    if let Ok(p) = faraday_intake::wallet::read_wallet(&d.text) {
        y += value_row(ui, x, y, w, S.companion_kind, &wallet_kind(&p), TEXT);
        y += value_row(ui, x, y, w, S.companion_keys, &k_of_n(&p), TEXT);
        for (i, key) in p.keys().iter().enumerate() {
            let fp = key.fingerprint().map_or_else(
                || S.companion_no_fingerprint.to_string(),
                |f| String::from_utf8_lossy(&f.to_hex()).into_owned(),
            );
            let label = fill(S.companion_key_n, &[&(i + 1).to_string()]);
            y += value_row(ui, x, y, w, &label, &fp, TEXT);
        }
        match faraday_intake::wallet::first_address(&p) {
            Some((net, a)) => {
                y += value_row(
                    ui,
                    x,
                    y,
                    w,
                    S.companion_wallet_network,
                    faraday_intake::wallet::network_name(net),
                    TEXT,
                );
                y += mono_row(ui, x, y, w, S.companion_first_address, &grouped(&a));
            }
            None => {
                y += value_row(
                    ui,
                    x,
                    y,
                    w,
                    S.companion_first_address,
                    S.companion_no_address,
                    TEXT,
                );
            }
        }
    }
    y += value_row(
        ui,
        x,
        y,
        w,
        S.companion_when,
        &when(d.time, app.offset_s()),
        TEXT,
    );
    y += 12.0;
    y += buttons(
        ui,
        x,
        y,
        w,
        &[(S.companion_show_as_qr, Style::Primary, Act::ShowAsQr)],
    ) + 8.0;
    y += buttons(
        ui,
        x,
        y,
        w,
        &[
            (S.companion_share, Style::Secondary, Act::Share),
            (S.companion_rename, Style::Secondary, Act::Rename),
        ],
    ) + 8.0;
    y += buttons(
        ui,
        x,
        y,
        w,
        &[(S.companion_delete, Style::Secondary, Act::Delete)],
    );
    y
}

/// Rename: the name in its field, typed on Faraday's keyboard docked at
/// the foot ([`frame`]), as the stick's touch panel types; and Save.
fn rename(app: &Companion, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    y += label(ui, x, y, S.companion_name);
    let fh = 46.0;
    ui.fill(x, y, w, fh, 10.0, INNER);
    ui.stroke(x, y, w, fh, 10.0, ACCENT);
    ui.typed(
        x + 12.0,
        y,
        fh,
        15.0,
        W::R,
        TEXT,
        &app.name,
        w - 24.0,
        true,
        Act::NameField,
    );
    ui.hit(x, y, w, fh, Act::NameField);
    y += fh + 16.0;
    let style = if app.name.trim().is_empty() {
        Style::Disabled
    } else {
        Style::Primary
    };
    y += buttons(ui, x, y, w, &[(S.companion_save, style, Act::SaveName)]);
    y
}

/// BitLocker keys, as Received: each key's computer and drive on its
/// card.
fn keys(app: &Companion, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let Some(store) = kept_store(app) else {
        return y0;
    };
    let mut y = y0;
    y += list_head(ui, x, y, S.companion_home_bitlocker, store.keys.len());
    if store.keys.is_empty() {
        y += line(ui, x, y, w, S.companion_keys_empty, DIM);
    }
    for (i, k) in store.keys.iter().enumerate() {
        let sub = fill(
            S.companion_drive_line,
            &[&k.text.drive, &when(k.time, app.offset_s())],
        );
        y += item_card(
            ui,
            x,
            y,
            w,
            Icon::Lock,
            &k.text.computer,
            &sub,
            Act::KeyRecord(i),
        ) + 10.0;
    }
    y
}

/// A kept key's record (§4.6): the computer, the drive, when it was
/// kept, the key as the Starter shows one (eight numbered groups of six)
/// masked until held, then Back in Windows and Delete. Nothing that
/// shares, saves or shows it as a code.
fn key_record(app: &Companion, ui: &mut Ui, x: f32, y0: f32, w: f32, n: usize) -> f32 {
    let Some(k) = kept_store(app).and_then(|s| s.keys.get(n)) else {
        return y0;
    };
    let mut y = y0;
    y += value_row(
        ui,
        x,
        y,
        w,
        S.companion_bitlocker_computer,
        &k.text.computer,
        TEXT,
    );
    y += value_row(
        ui,
        x,
        y,
        w,
        S.companion_bitlocker_drive,
        &k.text.drive,
        TEXT,
    );
    y += value_row(
        ui,
        x,
        y,
        w,
        S.companion_when,
        &when(k.time, app.offset_s()),
        TEXT,
    ) + 8.0;
    y += key_panel(ui, x, y, w, &k.text, app.held);
    y += buttons(
        ui,
        x,
        y,
        w,
        &[(S.companion_hold_to_show, Style::Secondary, Act::Hold)],
    ) + 20.0;
    y += buttons(
        ui,
        x,
        y,
        w,
        &[(
            S.companion_back_in_windows,
            Style::Primary,
            Act::BackInWindows,
        )],
    ) + 8.0;
    y += buttons(
        ui,
        x,
        y,
        w,
        &[(S.companion_delete, Style::Secondary, Act::Delete)],
    );
    y
}

/// Back in Windows (§4.6): the question, its two lines of fact, and
/// Delete the key or Keep it.
fn back_in_windows_sheet(ui: &mut Ui, w: f32, h: f32) {
    faraday_ui::sheet::sheet(ui, w, h, &mut |ui, x, y, iw| {
        let mut cy = y;
        cy += faraday_ui::sheet::sheet_head(
            ui,
            x,
            cy,
            iw,
            Icon::Trash,
            WARN,
            S.companion_delete_key_ask,
        );
        for l in [S.companion_key_not_needed, S.companion_key_shown_again] {
            cy += ui.wrap(x, cy, iw, 14.0, W::R, TEXT, l) + 10.0;
        }
        cy += 8.0;
        cy += faraday_ui::sheet::buttons(
            ui,
            x,
            cy,
            iw,
            &[
                (S.companion_keep_it, Style::Secondary, Act::KeepIt),
                (
                    S.companion_delete_the_key,
                    Style::Primary,
                    Act::ConfirmDelete,
                ),
            ],
        );
        cy - y
    });
}

/// Waiting for the phone's unlock.
fn unlocking(ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    y0 + line(ui, x, y0, w, S.companion_unlock_waiting, MUTED)
}

/// The kept store not opened, and why; a store that could not be read
/// can be deleted from here.
fn closed_sheet(app: &Companion, ui: &mut Ui, w: f32, h: f32, why: Closed) {
    let reason = match why {
        Closed::Cancelled => S.companion_closed_cancelled,
        Closed::NoScreenLock => S.companion_closed_no_lock,
        Closed::Unreadable => S.companion_unreadable_value,
    };
    let mut items = vec![(S.companion_done, Style::Primary, Act::Done)];
    if why == Closed::Unreadable && app.kept_unreadable {
        items.insert(
            0,
            (
                S.companion_delete_kept_store,
                Style::Secondary,
                Act::DeleteKeptStore,
            ),
        );
    }
    faraday_ui::sheet::kv_sheet(
        ui,
        w,
        h,
        (Icon::Lock, WARN, S.companion_closed_title),
        "",
        &[(S.companion_reason, reason.to_string(), TEXT)],
        &items,
    );
}

/// A description that did not read: the reader's line.
fn not_description_sheet(ui: &mut Ui, w: f32, h: f32, why: &str) {
    faraday_ui::sheet::kv_sheet(
        ui,
        w,
        h,
        (Icon::Warning, WARN, S.companion_not_description),
        "",
        &[(S.companion_reason, why.to_string(), TEXT)],
        &[(S.companion_done, Style::Primary, Act::Done)],
    );
}
