//! What the shell's storage side sends, and a stick arriving or leaving:
//! every `StorageEvent` is handled here.

use crate::Item;
use crate::Screen;
use crate::Seen;
use crate::Sheet;
use crate::StorageCommand;
use crate::StorageEvent;
use crate::strings::EN;
use crate::wallet::FileKind;
use crate::{Faraday, StickInfo};
use crate::{secrets, seeds, stick, vault};
use faraday_intake::seedqr_words;
use faraday_shell_api::Command;
use std::collections::BTreeSet;

pub(crate) fn storage_event(app: &mut Faraday, event: StorageEvent) {
    let c = app.compute();
    // A read's answer ends its being asked for, whichever handler takes it:
    // a flow's or the import's answer leaves the name free for Copy to ask
    // again.
    if let StorageEvent::Read { name, .. }
    | StorageEvent::QrRead { name, .. }
    | StorageEvent::ReadFailed { name, .. } = &event
    {
        app.visit.reading_done(name);
    }
    let event = match app.settings_event(event).and_then(|e| app.import_event(e)) {
        Some(e) => e,
        None => {
            app.out.redraw();
            app.out.commands.push_back(Command::Draw);
            return;
        }
    };
    match event {
        StorageEvent::Sticks(sticks) => {
            sticks_changed(app, sticks);
            app.zero_sticks_changed();
        }
        StorageEvent::Read { stick, name, bytes } if app.flow_wants_read(&stick, &name) => {
            app.flow_read_answer(&name, Some(bytes));
        }
        // A picture the write step wrote, read back for its codes.
        StorageEvent::QrRead {
            name,
            bytes,
            payloads,
            ..
        } if app.flow_wants_qr(&name) => {
            app.flow_qr_answer(&name, Some((bytes, payloads)));
        }
        StorageEvent::ReadFailed { name, .. } if app.flow_wants_qr(&name) => {
            app.flow_qr_answer(&name, None);
        }
        StorageEvent::ReadFailed { stick, name, .. } if app.flow_wants_read(&stick, &name) => {
            app.flow_read_answer(&name, None);
        }
        StorageEvent::StickSpace { stick, free, size } if app.flow_wants_space(&stick) => {
            app.flow_space_answer(free, size);
        }
        StorageEvent::StickProblem { stick, read_only } => {
            app.flow_stick_problem(&stick, read_only);
        }
        ev @ (StorageEvent::Written { .. } | StorageEvent::WriteFailed { .. })
            if app.flow_wants_write_event(&ev) =>
        {
            app.flow_write_event(ev);
        }
        ev @ StorageEvent::NameTaken { .. } => app.name_taken_event(ev),
        StorageEvent::Read { stick, name, bytes } => {
            // A file Faraday reads as nothing else still comes in, as
            // a File: to sign, or to send as codes.
            let mut item = Item::new(&c, &name, bytes);
            // A secret (seed, TOTP entries, wallet) the person did not
            // tick is not kept (its bytes wipe as they drop): the file
            // stays listed, unticked, as a secret named as one is.
            if !app.visit.inn_is_own(&name) && item.kind.exposure() == secrets::Exposure::Secret {
                app.visit.inn_unchoose(&name);
                app.visit.load_after_remove(&name);
                app.visit.reading_done(&name);
                app.visit.log_push((
                    faraday_text::Text::fill(EN.storage_event_seed_not_copied, &[&*name]),
                    true,
                ));
            } else {
                item.name = app.inbox_name_for(&name, &item.bytes);
                if item.kind == FileKind::Vault
                    && let Some(s) = app.stick.iter().find(|s| s.id == stick)
                {
                    let label = s.label.clone();
                    app.vault_read_from(&item.name, &item.bytes, &label);
                }
                if item.kind.exposure() == secrets::Exposure::Secret
                    && let Some(label) = app
                        .stick
                        .iter()
                        .find(|s| s.id == stick)
                        .map(|s| s.label.clone())
                {
                    app.visit.unprotected_add(&stick, &label, &name);
                }
                // The visit's rows say what came in; only a seed waiting
                // for the stick's removal is worth a line.
                let line = (item.kind == FileKind::Words
                    && app.visit.load_after.contains(&*item.name))
                .then(|| {
                    faraday_text::Text::fill(
                        EN.storage_event_copied_words,
                        &[&item.name, app.medium.noun()],
                    )
                });
                // The stick's own file name, not the Inbox name a rename
                // gave it.
                let sign = item.kind == FileKind::Psbt
                    && app.visit.sign_stick().is_some_and(|n| **n == *name);
                if *item.name == *name {
                    app.visit_same_mark(&stick, &name, item.bytes.len() as u64, false);
                }
                let sign_name = app.visit.sign_stick().cloned();
                let sign_from = app.visit.sign_from().cloned();
                app.inbox.push(item);
                if sign {
                    // "Import and sign": the review, not a trip through Files.
                    let k = app.inbox.len().saturating_sub(1);
                    app.act(crate::Action::StartSpend(k));
                    app.visit.set_sign_stick(sign_name);
                    app.visit.set_sign_from(sign_from);
                }
                if let Some(line) = line {
                    app.visit.log_push((line, true));
                }
                app.visit.inn_unchoose(&name);
                app.visit.reading_done(&name);
                app.save_boxes();
            }
        }
        StorageEvent::ReadFailed { name, reason, .. } => {
            app.visit.reading_done(&name);
            app.visit
                .log_push((faraday_text::text!("{}: {}", &*name, &*reason), false));
        }
        StorageEvent::Written {
            stick,
            name,
            wrote_as,
        } => {
            // A secret file written unprotected: the visit says so
            // while the stick is in (names only).
            let from = if app.visit.writing_inbox.contains(&*name) {
                &app.inbox
            } else {
                &app.outbox
            };
            let secret = from
                .iter()
                .any(|i| i.name == *name && i.kind.exposure() == secrets::Exposure::Secret);
            if secret
                && let Some(label) = app
                    .stick
                    .iter()
                    .find(|s| s.id == stick)
                    .map(|s| s.label.clone())
            {
                app.visit.unprotected_add(&stick, &label, &wrote_as);
            }
            // Written and read back: the same file in both places, under
            // the name the stick keeps it by (a Keep both's other name is
            // another file).
            let from_outbox = !app.visit.writing_inbox.contains(&*name);
            let written = app
                .outbox
                .iter()
                .filter(|_| from_outbox)
                .chain(app.inbox.iter().filter(|_| !from_outbox))
                .find(|i| i.name == *name)
                .map(|i| {
                    (
                        i.bytes.len() as u64,
                        !from_outbox || i.kind == FileKind::Vault,
                    )
                });
            if let Some((size, out_row)) = written
                && wrote_as.eq_ignore_ascii_case(&name)
            {
                app.visit_same_mark(&stick, &wrote_as, size, out_row);
            }
            app.visit_written(&name, &wrote_as);
            // A rename is carried by one write, to the stick written:
            // another stick keeps its own name for the same vault.
            app.visit.renamed_remove(&name);
            // A copy from the Inbox stays there, for the next stick.
            if app.visit.writing_inbox_done(&name) {
                app.visit.inbox_unpick(&name);
            } else {
                // Written, it leaves For the stick for the receipt. A
                // vault stays in Files as the stick now holds it, to
                // unlock again; its copy from before goes.
                if let Some(k) = app.outbox.iter().position(|i| i.name == *name) {
                    let item = app.outbox.remove(k);
                    app.inbox_vault_written(&c, &item);
                }
                app.visit.out_unchoose(&name);
            }
            let line = if wrote_as == name {
                faraday_text::Text::fill(EN.storage_event_wrote_read, &[&*name])
            } else {
                faraday_text::Text::fill(EN.storage_event_wrote, &[&*name, &*wrote_as])
            };
            app.visit.log_push((line, true));
            app.save_boxes();
        }
        StorageEvent::WriteFailed { name, reason, .. } => app.visit_write_failed(&name, &reason),
        StorageEvent::NotKept => app.not_kept(),
        StorageEvent::Wiped { stick, name } => app.wiped(&stick, &name, None, true),
        StorageEvent::WipedName { stick, name } => app.wiped(&stick, &name, None, false),
        e @ StorageEvent::Renamed { .. } => app.file_renamed(e),
        e @ StorageEvent::StickNamed { .. } => app.stick_named(e),
        StorageEvent::BootMedium(m) => app.stick.set_boot_medium(m),
        StorageEvent::Attached(a) => app.storage_attached(a),
        StorageEvent::Hashed { stick, name, sha } => app.file_hashed(&stick, &name, sha),
        StorageEvent::StickSpace { free, size, .. } => app.zero_space(free, size),
        StorageEvent::Zeroing {
            next, total, done, ..
        } => app.zero_step(next, total, done),
        StorageEvent::ZeroFailed { stick, reason } => {
            // The room on a stick could not be asked for: the flow's
            // reading of it ends, with the stick unusable.
            app.flow_space_failed(&stick);
            app.zero_failed(&reason)
        }
        StorageEvent::WipeFailed {
            stick,
            name,
            reason,
        } => app.wiped(&stick, &name, Some(&reason), false),
        StorageEvent::Printed { path } => {
            app.toast(&faraday_text::Text::fill(EN.storage_event_saved, &[&*path]))
        }
        StorageEvent::Clock { unix_secs } => {
            app.vaults.set_clock(unix_secs, app.now_ms);
            return;
        }
        StorageEvent::Memory { available_mib } => {
            let shown = app.vaults.memory_shown();
            app.vaults.set_memory_free(available_mib);
            // Drawn again only when the figure changes what the screen
            // shows, not at every reading.
            if !matches!(app.screen, Screen::CreateVault | Screen::Unlock)
                || shown == app.vaults.memory_shown()
            {
                return;
            }
        }
        StorageEvent::Cameras(list) => {
            // Sent as the shell looks again: drawn again only when it
            // changes, since what the copy item offers depends on it.
            if !app.camera.set_cameras(list) {
                return;
            }
        }
        StorageEvent::QrSeen {
            width,
            height,
            corners,
            module_tenths,
            read,
            whole,
        } => {
            let now_ms = app.now_ms;
            app.camera.qr_seen(
                now_ms,
                Seen {
                    width,
                    height,
                    corners,
                    read,
                },
                module_tenths,
                whole,
            );
        }
        StorageEvent::QrRead {
            name,
            mut bytes,
            mut payloads,
            ..
        } => {
            // A picture holding a SeedQR comes in only on the
            // person's own tick: another is dropped, left listed.
            let own = app.visit.inn_is_own(&name);
            app.visit.inn_unchoose(&name);
            app.visit.reading_done(&name);
            let dropped = !own
                && payloads
                    .iter()
                    .any(|p| !crate::scan_read::is_compact_text(p) && seedqr_words(p).is_some());
            if dropped {
                app.visit.load_after_remove(&name);
                app.visit.log_push((
                    faraday_text::Text::fill(EN.storage_event_seed_not_copied, &[&*name]),
                    true,
                ));
            } else {
                // The picture itself comes in as it is, beside what its
                // codes hold, to be written to another stick; it takes
                // the exposure of what they hold.
                let before = app.inbox.len();
                app.qr_read(&name, std::mem::take(&mut payloads));
                let holds = app.picture_holds(before);
                // Whatever its codes hold that is a secret (a share of a
                // key, a TOTP setup, secret text) comes in only on the
                // person's own tick as well.
                if !own && holds.exposure() == secrets::Exposure::Secret {
                    let mut at = 0;
                    app.inbox.retain(|_| {
                        at += 1;
                        at <= before
                    });
                    app.visit.load_after_remove(&name);
                    app.visit.log_push((
                        faraday_text::Text::fill(EN.storage_event_seed_not_copied, &[&*name]),
                        true,
                    ));
                } else {
                    // Under its own name, unless what its codes held took it.
                    let got = if app.inbox.iter().skip(before).any(|i| i.name == *name) {
                        app.free_inbox_name(&name)
                    } else {
                        app.inbox.retain(|i| i.name != *name);
                        faraday_text::Text::of(&name)
                    };
                    let mut item = Item::new(&c, &got, std::mem::take(&mut *bytes));
                    item.picture = Some(holds);
                    app.inbox.push(item);
                }
                app.save_boxes();
            }
        }
        StorageEvent::PrintFailed { reason } => app.toast(&faraday_text::Text::fill(
            EN.storage_event_not_printed,
            &[&*reason],
        )),
        #[cfg(feature = "demo")]
        StorageEvent::HostFiles { dir, files, opens } => {
            if !app.transfer_listed(dir, files, opens) {
                return;
            }
        }
        #[cfg(feature = "demo")]
        e @ (StorageEvent::Hovered(_)
        | StorageEvent::Dropped { .. }
        | StorageEvent::HostRead { .. }
        | StorageEvent::HostReadFailed { .. }
        | StorageEvent::Saved { .. }
        | StorageEvent::SaveFailed { .. }) => app.transfer_event(e),
        // Only Faraday Demo and the Starter hold Transfer, which these
        // events are for; the stick's shell never sends them.
        #[cfg(not(feature = "demo"))]
        StorageEvent::HostFiles { .. }
        | StorageEvent::Hovered(_)
        | StorageEvent::Dropped { .. }
        | StorageEvent::HostRead { .. }
        | StorageEvent::HostReadFailed { .. }
        | StorageEvent::Saved { .. }
        | StorageEvent::SaveFailed { .. } => {}
        e @ (StorageEvent::NewInput { .. }
        | StorageEvent::InputTyped { .. }
        | StorageEvent::InputTapped { .. }
        | StorageEvent::InputUsed { .. }
        | StorageEvent::InputTrust { .. }
        | StorageEvent::BootUsb { .. }
        | StorageEvent::InputGone { .. }) => app.input_event(e),
        StorageEvent::Restored {
            inbox,
            outbox,
            kept,
        } => {
            app.inbox.replace(
                inbox
                    .into_iter()
                    .map(|(n, b)| Item::new(&c, &n, b))
                    .collect(),
            );
            app.outbox = outbox
                .into_iter()
                .map(|(n, b)| Item::new(&c, &n, b))
                .collect();
            app.restore_kept(&kept);
        }
    }
    app.out.redraw();
    app.out.commands.push_back(Command::Draw);
}

pub(crate) fn sticks_changed(app: &mut Faraday, sticks: Vec<StickInfo>) {
    let had = !app.stick.is_empty();
    let ids_before: Vec<faraday_text::Text> = app.stick.iter().map(|s| s.id.clone()).collect();
    app.stick.set_sticks(sticks);
    // The keys follow: no way of adding one adds it with a stick in.
    let noun = app.stick.in_the_way().then(|| app.medium.noun());
    app.session.keys.set_stick_attached(noun);
    // What the shell found unusable comes with each listing: a stick
    // that no longer reports a problem has none.
    app.stick.clear_problems();
    // The medium the device started from goes by what it is, not by its volume label.
    let boot = app.medium.boot();
    app.stick.label_boot_sticks(boot);
    let ids_now: Vec<faraday_text::Text> = app.stick.iter().map(|s| s.id.clone()).collect();
    if app.visit.stick >= app.stick.len() {
        app.visit.set_stick(0);
    }
    let names: BTreeSet<faraday_text::Text> = app
        .stick
        .get(app.visit.stick)
        .map(|s| s.files.iter().map(|(n, _)| n.clone()).collect())
        .unwrap_or_default();
    app.visit.inn_retain(|n| names.contains(n));
    app.visit.unprotected_keep(&ids_now);
    app.visit.same.keep(&ids_now);
    app.visit_same_ask();
    app.flow_sticks_changed();
    app.name_taken_pulled();
    if app.stick.is_empty() {
        app.stick.set_held(false);
        app.stick.set_not_now(false);
        // With the kernel still showing storage (a card reader, now
        // empty) nothing is out of the way yet: `storage_attached(false)`
        // runs the step when it is.
        if !app.stick.in_the_way() {
            nothing_in_the_way(app, had);
        }
        return;
    }
    app.import_stick_gone();
    if ids_now == ids_before {
        app.release_held_stick();
        return;
    }
    // A stick arriving while the camera is on is held back, and
    // arrives when the camera is off (`docs/FLOWS.md`): then a clean
    // process visits it and any other asks to lock first. The
    // scanner is never replaced with its camera still on.
    if app.camera_on() {
        app.stick.set_held(true);
        return;
    }
    stick_arrived(app);
}

/// Nothing is in the way now: no stick is listed and the kernel shows no
/// storage. Run once, by whichever of the listing and the kernel's view
/// makes it so; what waited on the stick carries on. `had` is whether
/// something was in the way before.
pub(crate) fn nothing_in_the_way(app: &mut Faraday, had: bool) {
    // The read card's Scan again, asked for Lock while the stick was in,
    // carries on once it is out: the camera turns on as with no stick.
    let scan_again = app.sheet == Some(Sheet::Lock)
        && matches!(app.visit.pull(), Some(crate::Action::ReadAgain));
    if app.sheet == Some(Sheet::Lock) {
        app.sheet = None;
        app.visit.set_lock_to_write(None);
    }
    if scan_again {
        app.visit.clear_pull();
        app.act(crate::Action::ReadAgain);
    }
    // A passphrase waiting on the stick takes typing now.
    if app.screen == Screen::Unlock
        && app.vaults.focus.is_none()
        && app.vault_files().iter().any(|f| f.open.is_none())
    {
        app.vaults.set_focus(Some(vault::Focus::Passphrase));
    }
    app.visit_pulled();
    if had {
        let loaded = app.load_after_pull() + app.import_load_after_pull();
        let said = if loaded == 0 {
            faraday_text::Text::fill(EN.sticks_changed_removed_only, &[app.medium.cap()])
        } else {
            faraday_text::Text::fill(
                EN.sticks_changed_removed,
                &[
                    app.medium.cap(),
                    &faraday_text::Text::num(loaded as u64),
                    (if loaded == 1 {
                        EN.sticks_changed_key
                    } else {
                        EN.sticks_changed_keys
                    }),
                ],
            )
        };
        app.toast(&said);
    }
    app.import_stick_gone();
    // What was pressed with the stick in carries on, and the
    // reminders the sheet listed are seen.
    if app.sheet == Some(Sheet::Pull) {
        app.sheet = None;
        if let Some(a) = app.visit.take_pull() {
            if app.devices.held.is_empty() {
                app.devices.understand();
            }
            app.act(a);
        }
    }
}

/// What a stick in the way closes, whichever event shows it (the disk
/// process's listing or the kernel's view): the Krux passphrase sheet, the
/// seed read's passphrase pair, every passphrase field of `pass_field`, the
/// vaults' secret fields and the tools' secret fields.
pub(crate) fn secrets_close(app: &mut Faraday) {
    seeds::kef::kef_stick_arrived(app);
    app.seed_read_stick_arrived();
    app.lightning.stick_arrived();
    crate::pass_field::stick_arrived(app);
    if matches!(
        app.vaults.focus,
        Some(vault::Focus::Passphrase | vault::Focus::Phrase(..) | vault::Focus::Dice)
    ) {
        app.vaults.set_focus(None);
    }
    stick::tools_lose_secrets(app, false);
}

/// A stick arrived, or one held back is taken now.
pub(crate) fn stick_arrived(app: &mut Faraday) {
    let c = app.compute();
    // A stick arrived. Only a clean process takes it; any other locks
    // first (`docs/FLOWS.md`). No passphrase field takes typing while
    // it is attached.
    secrets_close(app);
    // The backup flow on screen takes the stick: never the visit, at any
    // step (`docs/BACKUP-FLOW.md` §5).
    if app.flow_stick_arrived() {
        return;
    }
    app.visit.log_clear();
    app.visit.set_pictures_scan(None);
    app.visit.set_settings(None);
    // The first look at the boot stick this power-on: what it holds is
    // copied into memory for the import (`boot_import`).
    let first_look = app.stick.settings().is_none();
    let boot = app.stick.iter().find(|s| s.boot).cloned();
    if !app.clean() {
        app.visit.set_write_waiting(None);
        app.stick.set_not_now(false);
        app.lock_sheet_replaces_devices();
    } else {
        {
            let v = app.visit_default_out(&c);
            app.visit.set_out(v);
        }
        {
            let v = app.visit_default_in();
            app.visit.set_inn(v);
        }
        if app.import_on_show() {
            app.sheet = None;
        }
        // The boot stick alone at its first look, or back for its
        // import, leaves Home on screen (`boot_import`).
        if !app.import_keeps_home(first_look, boot.as_ref()) {
            // A stick arriving in the middle of Create or Restore
            // brings the person back there when it is pulled.
            if matches!(
                app.screen,
                Screen::Create | Screen::Restore | Screen::SpendBackup
            ) {
                app.visit.set_after_visit(app.screen);
            }
            // A stick arriving on Add a key is for a key file: what is
            // imported loads at the pull, which returns to where Add a
            // key was opened from.
            if app.screen == Screen::Entry {
                let back = app.entry.state().back.unwrap_or(Screen::Home);
                app.visit.set_after_visit(back);
                app.visit.set_for_key(true);
            }
            app.screen = Screen::Files;
            app.visit.set_tab_stick(true);
        }
        // A boot stick with nothing but the settings file has nothing to
        // offer: its first look is its visit, and Home shows no tile.
        if first_look
            && let Some(b) = boot.as_ref()
            && b.files.iter().all(|(n, _)| stick::settings::is_file(n))
        {
            app.stick.mark_visited(&b.id);
        }
        // "Lock and write" was pressed before the lock: the files For the
        // stick are written to the stick that came, and read back.
        app.visit_write_waiting();
        if first_look
            && app.import.state().is_none()
            && let Some(b) = boot.as_ref()
            && b.files.iter().any(|(n, _)| !stick::settings::is_file(n))
        {
            app.import_start(b);
        }
        // The boot stick's settings, once a session (`stick_settings`).
        if app.stick.settings().is_none()
            && let Some(boot) = app.stick.iter().find(|s| s.boot)
        {
            match boot.files.iter().find(|(n, _)| stick::settings::is_file(n)) {
                Some((n, _)) => app.out.storage_out.push_back(StorageCommand::Read {
                    stick: boot.id.clone(),
                    name: n.clone(),
                }),
                None => app.settings_on_stick_known(),
            }
        }
    }
}
