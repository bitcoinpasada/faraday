//! The descriptor and the split shares as labelled QR pictures: what a
//! wallet's public files put in the Outbox as PNGs reads back, through
//! the disk process's picture reader, as the same descriptor or share,
//! and the label under the code names the wallet and the checksum.

use faraday_core::plan::{self, Item};
use faraday_core::testkit;
use faraday_core::wallet::FileKind;
use faraday_core::{Action, Faraday, Screen, StickInfo, StorageEvent, bstep, qrow};
use faraday_draw::canvas::InkKind;
use faraday_shell_api::{App, Event, Key};

const STICK: &str = "S1";

/// A started app on Stick visit, with a stick in.
fn visiting() -> Faraday {
    let mut app = testkit::started();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        serial: None,
        id: STICK.to_string(),
        label: "TESTSTICK".to_string(),
        boot: false,
        files: Vec::new(),
    }]));
    app.press(Action::Nav(Screen::Visit));
    app
}

/// The picture `name` read from the stick, as the disk process reads a
/// PNG, and handed to the app.
fn scan(app: &mut Faraday, name: &str, png: &[u8]) {
    let c = faraday_core::testkit::compute();
    let payloads = faraday_files::qr_in_png(&c, png).expect("a PNG");
    assert!(!payloads.is_empty(), "{name}: no code read");
    app.storage(StorageEvent::QrRead {
        name: name.to_string(),
        bytes: png.to_vec(),
        payloads,
    });
}

/// The public file `name` written for the wallet `descriptor`.
fn file(files: &[(String, Vec<u8>)], name: &str) -> Vec<u8> {
    files
        .iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| {
            panic!(
                "no {name} in {:?}",
                files.iter().map(|(n, _)| n).collect::<Vec<_>>()
            )
        })
        .1
        .clone()
}

fn first_address(text: &str) -> String {
    let c = faraday_core::testkit::compute();
    let mut s = testkit::session();
    let i = s
        .add_wallet(&c, "w", text, "test", faraday_core::wallet::Origin::Named)
        .expect("a wallet");
    s.address(&c, &s.wallets[i], false, 0)
}

fn checksum(text: &str) -> String {
    text.trim()
        .rsplit_once('#')
        .expect("a checksum")
        .1
        .to_string()
}

/// The wallet `descriptor` loaded under `name` in an app of its own.
fn holding(name: &str, descriptor: &str) -> (Faraday, usize) {
    let c = faraday_core::testkit::compute();
    let mut app = Faraday::new();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    app.session = testkit::session();
    let w = app
        .session
        .add_wallet(
            &c,
            name,
            descriptor,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .expect("a wallet");
    (app, w)
}

/// The text the label of a picture shows.
fn label(p: &faraday_core::picture::Labelled) -> Vec<String> {
    p.draw()
        .ink()
        .iter()
        .filter_map(|i| match &i.kind {
            InkKind::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// The descriptor's picture of `kit`, scanned on a visit, comes back as
/// the same descriptor and loads as the same wallet.
fn descriptor_picture_reads_back(id: &str) {
    let kit = testkit::kits().into_iter().find(|k| k.id == id).unwrap();
    let files = testkit::public_files(kit.name, &kit.descriptor).unwrap();
    let stem = faraday_core::file_stem(kit.name);
    let want = String::from_utf8(file(&files, &format!("{stem}-descriptor.txt"))).unwrap();
    let png = file(&files, &format!("{stem}-descriptor.png"));
    let mut app = visiting();
    scan(&mut app, &format!("{stem}-descriptor.png"), &png);
    let item = app
        .inbox()
        .iter()
        .find(|i| i.kind == FileKind::Wallet)
        .unwrap_or_else(|| panic!("no wallet in {:?}", app.visit().log));
    let got = String::from_utf8_lossy(&item.bytes).into_owned();
    assert_eq!(checksum(&got), checksum(&want));
    assert_eq!(first_address(&got), first_address(&want));
}

#[test]
fn a_2_of_3_descriptor_picture_scans_back_as_the_wallet() {
    descriptor_picture_reads_back("savings");
}

#[test]
fn a_single_key_descriptor_picture_scans_back_as_the_wallet() {
    descriptor_picture_reads_back("spending");
}

#[test]
fn the_descriptor_picture_is_labelled_with_the_wallet_and_its_checksum() {
    let c = faraday_core::testkit::compute();
    let (app, w) = holding("Savings", &testkit::savings());
    let pictures = app.public_pictures(&c, w, 8);
    assert_eq!(pictures.len(), 1);
    let text = label(&pictures[0]);
    let sum = checksum(&testkit::savings());
    assert!(text.iter().any(|t| t == "Savings"), "{text:?}");
    assert!(
        text.iter()
            .any(|t| *t == format!("Descriptor checksum {sum}")),
        "{text:?}"
    );
}

#[test]
fn a_descriptor_past_one_code_goes_as_parts_that_assemble_to_it() {
    let c = faraday_core::testkit::compute();
    // A 2-of-15: three seeds at five accounts each.
    let keys: Vec<String> = (0..5)
        .flat_map(|a| (0..3).map(move |n| testkit::key(n, &format!("m/48'/1'/{a}'/2'"))))
        .map(|k| format!("{k}/<0;1>/*"))
        .collect();
    let descriptor = format!("wsh(sortedmulti(2,{}))", keys.join(","));
    let (app, w) = holding("Big", &descriptor);
    let want = app.session.wallets[w].policy().to_descriptor_checksummed();
    let pictures = app.public_pictures(&c, w, 8);
    let k = pictures.len();
    assert!(k > 1, "one code");
    for (j, p) in pictures.iter().enumerate() {
        assert_eq!(p.name, format!("big-descriptor-{}-of-{k}.png", j + 1));
        let text = label(p);
        assert!(
            text.iter().any(|t| *t == format!("Part {} of {k}", j + 1)),
            "{text:?}"
        );
    }
    let mut reader = visiting();
    for p in &pictures {
        scan(&mut reader, &p.name, &p.png(&c));
    }
    let item = reader
        .inbox()
        .iter()
        .find(|i| i.kind == FileKind::Wallet)
        .unwrap_or_else(|| panic!("not assembled: {:?}", reader.visit().log));
    let got = String::from_utf8_lossy(&item.bytes).into_owned();
    assert_eq!(got.trim(), want);
}

#[test]
fn any_two_share_pictures_of_a_2_of_3_rebuild_the_wallet() {
    shares_rebuild("savings");
}

#[test]
fn any_two_share_pictures_of_a_taproot_2_of_3_rebuild_the_wallet() {
    shares_rebuild("taproot-multisig");
}

/// Any two of the three share pictures of `id`, scanned on a visit,
/// rebuild the wallet with Restore.
fn shares_rebuild(id: &str) {
    let c = faraday_core::testkit::compute();
    let kit = testkit::kits().into_iter().find(|k| k.id == id).unwrap();
    let files = testkit::public_files(kit.name, &kit.descriptor).unwrap();
    let want = first_address(&kit.descriptor);
    let name = |k: usize| format!("{id}-share-{k}-of-3.png");
    for pair in [[1, 2], [2, 3], [1, 3]] {
        let mut app = visiting();
        for k in pair {
            scan(&mut app, &name(k), &file(&files, &name(k)));
        }
        let shares = app
            .inbox()
            .iter()
            .filter(|i| i.kind == FileKind::Share)
            .count();
        assert_eq!(shares, 2, "{pair:?}: {:?}", app.visit().log);
        app.storage(StorageEvent::Sticks(Vec::new()));
        app.press(Action::RestoreShares);
        assert!(
            app.session
                .wallets
                .iter()
                .any(|w| app.session.address(&c, w, false, 0) == want),
            "{pair:?} did not rebuild"
        );
    }
}

#[test]
fn a_share_picture_names_what_it_holds_and_leaves_off() {
    let c = faraday_core::testkit::compute();
    let (app, w) = holding("Savings", &testkit::savings());
    let pictures = app.public_pictures(&c, w, 4);
    assert_eq!(pictures.len(), 3);
    let fps: Vec<String> = app.session.wallets[w]
        .policy()
        .keys()
        .iter()
        .map(|k| faraday_core::wallet::fp_text(k.fingerprint().unwrap()))
        .collect();
    for p in &pictures {
        let text = label(p);
        let off = text
            .iter()
            .find_map(|t| t.strip_prefix("Leaves off "))
            .unwrap_or_else(|| panic!("{text:?}"));
        assert!(fps.iter().any(|f| f == off), "{off} is not a key");
        assert!(text.iter().any(|t| t == "Not a wallet on its own"));
    }
}

/// The backup of the wallet `descriptor`, with test key 1 typed in and
/// the checklist made for the descriptor as a public file.
fn checklist_for(name: &str, descriptor: &str) -> (Faraday, usize) {
    let c = faraday_core::testkit::compute();
    let mut app = testkit::started();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    let w = app
        .session
        .add_wallet(
            &c,
            name,
            descriptor,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .expect("a wallet");
    app.press(Action::Backup(w));
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVaultSoftware);
    let answers = &app.backup().unwrap().answers;
    // The descriptor as a picture and as text.
    if !answers.software[plan::software::NOT_SURE] {
        app.press(Action::BAnswer(
            qrow::SOFTWARE,
            plan::software::NOT_SURE as u8,
        ));
    }
    for (row, on) in [(plan::form::QR, true), (plan::form::TEXT, true)] {
        if app.backup().unwrap().answers.form[row] != on {
            app.press(Action::BAnswer(qrow::FORM, row as u8));
        }
    }
    app.press(Action::BChecklist);
    if app.backup_items_now(&c).contains(&bstep::BLANK) {
        app.press(Action::BNext(bstep::BLANK));
    }
    (app, w)
}

/// The pictures written for the stick are the ones the descriptor's
/// public file is named by: the item is done once they are there, and
/// not once the picture it names is gone.
fn public_item_lists_the_pictures_written(name: &str, descriptor: &str, parts: bool) {
    let c = testkit::compute();
    let (mut app, w) = checklist_for(name, descriptor);
    let pictures: Vec<String> = app
        .public_pictures(&c, w, 8)
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert_eq!(pictures.len() > 1, parts, "{pictures:?}");
    let written = |app: &Faraday| -> Vec<String> {
        app.outbox
            .iter()
            .filter(|i| i.name.contains("-descriptor") && i.name.ends_with(".png"))
            .map(|i| i.name.clone())
            .collect()
    };
    let mut have = written(&app);
    have.sort();
    let mut want = pictures.clone();
    want.sort();
    assert_eq!(have, want);
    assert!(app.backup_items_now(&c).contains(&bstep::PUBLIC));
    assert!(app.backup_item_done_now(&c, Item::PublicFiles));
    // Taking the descriptor's picture (its first part, past one code) off
    // the stick makes the item not done again.
    let one = pictures.first().unwrap().clone();
    app.outbox.retain(|i| i.name != one);
    assert!(!app.backup_item_done_now(&c, Item::PublicFiles));
}

#[test]
fn the_public_files_item_follows_the_picture_written_for_a_descriptor_that_fits_one_code() {
    public_item_lists_the_pictures_written("Spending", &testkit::savings(), false);
}

#[test]
fn the_public_files_item_follows_the_pictures_written_for_a_descriptor_that_needs_parts() {
    // A 2-of-15: three seeds at five accounts each.
    let keys: Vec<String> = (0..5)
        .flat_map(|a| (0..3).map(move |n| testkit::key(n, &format!("m/48'/1'/{a}'/2'"))))
        .map(|k| format!("{k}/<0;1>/*"))
        .collect();
    let descriptor = format!("wsh(sortedmulti(2,{}))", keys.join(","));
    public_item_lists_the_pictures_written("Big", &descriptor, true);
}
