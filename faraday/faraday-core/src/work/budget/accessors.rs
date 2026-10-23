//! Every value kept on the state that drawing reads (`drawing/kept.rs`),
//! read on two frames in a row with nothing changed, does no counted
//! work the second time (`docs/SPEED.md` §3.3, `docs/COMPUTE.md` §4):
//! each accessor on its own, whichever screens draw it, in a session
//! where each has something to work out: the heavy session with its
//! backup, a spend, the test vault open, and Explore, Tools, Silent
//! payments and a Seeds piece open over a loaded key.

use faraday_bip::bip39::Language;
use faraday_vault::records::kind;

use super::walk;
use crate::Action;
use crate::create::NewKind;
use crate::drawing::View;
use crate::plan::{Answers, Preset};
use crate::seeds::SeedsState;
use crate::work::{KINDS, PIXELS, since, snapshot};
use crate::{Faraday, testkit};

/// One accessor read, by its name.
type Read = (&'static str, Box<dyn Fn(&View<'_>)>);

/// The heavy session, with every flow open that a kept value is read
/// for.
fn session() -> Faraday {
    let c = testkit::compute();
    let (mut app, w) = walk(&c, &mut |_, _| {});
    app.press(Action::Backup(w));
    let psbt = app
        .inbox
        .iter()
        .position(|i| i.name == "savings-unsigned.psbt")
        .expect("the spend is in the Inbox");
    app.press(Action::StartSpend(psbt));
    app.explore_open();
    app.silent_open();
    app.tools_open();
    if let Some(t) = testkit::tools_mut(&mut app) {
        t.tool = crate::tools::Tool::Descriptor;
        for ch in testkit::kits()[0].descriptor.chars() {
            t.typed.push(ch);
        }
    }
    assert!(app.backup().is_some(), "the backup is open");
    assert!(app.wallets.spend().is_some(), "the spend is open");
    assert!(!app.vaults.open.is_empty(), "the test vault is open");
    app
}

/// What each accessor is read with, from the session as it stands.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn reads(app: &Faraday) -> Vec<Read> {
    let net = app.session.network();
    let words = testkit::test_words(testkit::TEST_SEEDS[0].0);
    let mnemonic = faraday_keys::words::parse(&testkit::compute(), Language::English, &words)
        .expect("the test words");
    let descriptor = testkit::kits()[0].descriptor.clone();
    let w = app.backup().map_or(0, |b| b.wallet);
    let shape = app.plan_shape_now(w);
    let answers = Answers::preset(&shape, Preset::PaperVault);
    let seen = app.inbox.iter().find_map(|i| i.psbt().cloned());
    let psbt = app.wallets.spend().map(|s| s.spend.psbt.clone());
    let contents = &app.vaults.records(app.vaults.open[app.vaults.current].id);
    let key_record = contents
        .of(kind::KEY)
        .next()
        .map(|(i, _)| i)
        .expect("the test vault keeps a seed");
    let wallet_record = contents
        .of(kind::WALLET)
        .next()
        .map(|(i, _)| i)
        .expect("the test vault keeps a wallet");
    let file = app.vault_files().into_iter().next().expect("a vault file");
    let salt = app.vaults.open[app.vaults.current].header().salt;
    let slot = app.session.slots(&app.session.wallets[w]).remove(0);
    let seeds = SeedsState {
        keys: app.session.keys.all().map(|k| k.fingerprint.0).collect(),
        shaping: true,
        m: 2,
        n: app.session.keys.len(),
        kind: NewKind::Multi,
        ..SeedsState::default()
    };
    // The record itself, as its open vault keeps it: a copy is no open
    // vault's own, and is read afresh.
    fn record<'v>(v: &'v View<'_>, i: usize) -> faraday_keys::vault::RecordView<'v> {
        v.vaults
            .records(v.vaults.open[v.vaults.current].id)
            .get(i)
            .expect("the record is there")
    }
    let o = app.vaults.current;
    let mut out: Vec<Read> = vec![
        (
            "address_on",
            Box::new(move |v| {
                let _ = v.address_on(&v.session.wallets[w], net, false, 0);
            }),
        ),
        (
            "psbt_for",
            Box::new(move |v| {
                let _ = v.psbt_for(w);
            }),
        ),
        (
            "address_shown",
            Box::new(move |v| {
                let _ = v.address_shown(&v.session.wallets[w], false, 1);
            }),
        ),
        (
            "script_at",
            Box::new(move |v| {
                let _ = v.script_at(&v.session.wallets[w], true, 2);
            }),
        ),
        (
            "account_text",
            Box::new(|v| {
                let _ = v.account_text(0, NewKind::NativeSegwit, 1);
            }),
        ),
        (
            "text_at",
            Box::new(|v| {
                let _ = v.text_at(0, "m/84h/1h/2h");
            }),
        ),
        (
            "words_id",
            Box::new(|v| {
                let _ = v.words_id(0);
            }),
        ),
        (
            "entry_probed",
            Box::new(move |v| {
                let _ = v.entry_probed(&mnemonic);
            }),
        ),
        (
            "explore_reading",
            Box::new(|v| {
                let _ = v.explore_reading();
            }),
        ),
        (
            "explore_list",
            Box::new(|v| {
                let _ = v.explore_list();
            }),
        ),
        (
            "public_qr",
            Box::new(|v| {
                let _ = v.public_qr("bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq");
            }),
        ),
        (
            "read_kept",
            Box::new({
                let d = descriptor.clone();
                move |v| {
                    let _ = v.read_kept(&d);
                }
            }),
        ),
        (
            "plan_mapped",
            Box::new({
                let (s, a) = (shape.clone(), answers.clone());
                move |v| {
                    let _ = v.plan_mapped(&s, &a);
                }
            }),
        ),
        (
            "plan_check",
            Box::new({
                let (s, a) = (shape.clone(), answers.clone());
                move |v| {
                    let _ = v.plan_check(&s, &a);
                }
            }),
        ),
        (
            "copy_forms",
            Box::new(|v| {
                let _ = v.copy_forms(v.backup().expect("the backup"), 0);
            }),
        ),
        (
            "copy_checks",
            Box::new(|v| {
                let _ = v.copy_checks(v.backup().expect("the backup"), 0, &v.session.keys[0]);
            }),
        ),
        (
            "backup_questions",
            Box::new(|v| {
                let _ = v.backup_questions();
            }),
        ),
        (
            "message_check",
            Box::new(|v| {
                let _ = v.message_check("-----BEGIN BITCOIN SIGNED MESSAGE-----\nhello\n-----BEGIN SIGNATURE-----\nbc1q\nAAAA\n-----END BITCOIN SIGNED MESSAGE-----");
            }),
        ),
        (
            "record_described",
            Box::new(move |v| {
                let _ = v.record_described(record(v, wallet_record));
            }),
        ),
        (
            "restore_share_texts",
            Box::new(|v| {
                let _ = v.restore_share_texts();
            }),
        ),
        (
            "wallet_sticks_done",
            Box::new(|v| {
                let _ = v.wallet_sticks_done();
            }),
        ),
        (
            "described_of",
            Box::new(move |v| {
                let _ = v.described_of(v.vaults.open[o].id, kind::WALLET);
            }),
        ),
        (
            "vault_has_wallet",
            Box::new(move |v| {
                let _ = v.vault_has_wallet(o, &v.session.wallets[w]);
            }),
        ),
        (
            "record_word_count",
            Box::new(move |v| {
                let _ = v.record_word_count(record(v, key_record));
            }),
        ),
        (
            "gpg_chosen",
            Box::new(|v| {
                let _ = v.gpg_chosen();
            }),
        ),
        (
            "session_stage",
            Box::new(|v| {
                let _ = v.session_stage();
            }),
        ),
        (
            "vault_changed",
            Box::new(|v| {
                let _ = v.vault_changed();
            }),
        ),
        (
            "currency",
            Box::new(move |v| {
                let _ = v.currency(&file);
            }),
        ),
        (
            "apk_info",
            Box::new(|v| {
                if let Some(i) = v.inbox.iter().find(|i| i.name.ends_with(".apk")) {
                    let _ = v.apk_info(&i.name);
                }
            }),
        ),
        (
            "apk_listed",
            Box::new(|v| {
                let _ = v.apk_listed("app.faraday.test");
            }),
        ),
        (
            "apk_files_row",
            Box::new(|v| {
                let _ = v.apk_files_row(0);
            }),
        ),
        (
            "flow_files_copy_label",
            Box::new(move |v| {
                let _ = v.flow_files_copy_label(&salt);
            }),
        ),
        (
            "seeds_address",
            Box::new({
                let s = seeds.clone();
                move |v| {
                    let _ = v.seeds_address(&s);
                }
            }),
        ),
        (
            "gpg_files",
            Box::new(|v| {
                let _ = v.gpg_files();
            }),
        ),
        (
            "vault_key_for",
            Box::new(move |v| {
                let _ = v.vault_key_for(&slot);
            }),
        ),
        (
            "vault_wallets_with_keys",
            Box::new(move |v| {
                let _ = v.vault_wallets_with_keys(o);
            }),
        ),
        (
            "vault_key_wallet",
            Box::new(move |v| {
                let _ = v.vault_key_wallet(o, key_record);
            }),
        ),
        (
            "vault_seeds",
            Box::new(|v| {
                let _ = v.vault_seeds();
            }),
        ),
        (
            "glance_of_vault",
            Box::new(move |v| {
                let _ = v.glance_of_vault(o, wallet_record);
            }),
        ),
        (
            "wallet_key",
            Box::new(move |v| {
                let _ = v.wallet_key(w, 0);
            }),
        ),
        (
            "seeds_descriptor",
            Box::new({
                let s = seeds.clone();
                move |v| {
                    let _ = v.seeds_descriptor(&s);
                }
            }),
        ),
        (
            "vault_holds_seed",
            Box::new(move |v| {
                let _ = v.vault_holds_seed(o, 0, true);
            }),
        ),
        (
            "vault_has_key",
            Box::new(move |v| {
                let _ = v.vault_has_key(o, &v.session.keys[0]);
            }),
        ),
        (
            "backup_kept",
            Box::new(|v| {
                let _ = v.backup_kept();
            }),
        ),
        (
            "glance_of",
            Box::new(move |v| {
                let _ = v.glance_of(w);
            }),
        ),
        (
            "glance_sheet",
            Box::new(move |v| {
                let _ = v.glance_sheet(
                    crate::glance::Press::Loaded(w),
                    crate::glance::sheet::Target::Wallet,
                );
            }),
        ),
        (
            "backup_vault_taken",
            Box::new(move |v| {
                let _ = v.backup_vault_taken(0, o);
            }),
        ),
        (
            "item_title",
            Box::new(|v| {
                for n in v.backup_items() {
                    let _ = v.item_title(n);
                }
            }),
        ),
        (
            "wallet_key_lines",
            Box::new(move |v| {
                let _ = v.wallet_key_lines(w);
            }),
        ),
        (
            "tools_reading",
            Box::new(|v| {
                let _ = v.tools_reading();
            }),
        ),
        (
            "tools_keys",
            Box::new(|v| {
                let _ = v.tools_keys();
            }),
        ),
        (
            "silent_shown",
            Box::new(|v| {
                let _ = v.silent_shown();
            }),
        ),
        (
            "vault_key_fingerprint",
            Box::new(move |v| {
                let _ = v.vault_key_fingerprint(record(v, key_record));
            }),
        ),
    ];
    if let Some(seen) = seen {
        out.push((
            "spends_from",
            Box::new(move |v| {
                let _ = v.spends_from(&seen);
            }),
        ));
    }
    if let Some(psbt) = psbt {
        out.push((
            "nonce_check",
            Box::new(move |v| {
                let _ = v.nonce_check(&psbt);
            }),
        ));
    }
    out
}

#[test]
fn every_kept_value_read_on_two_frames_in_a_row_is_worked_out_once() {
    let mut app = session();
    let reads = reads(&app);
    assert!(reads.len() >= 50, "{} accessors read", reads.len());
    let mut failures = Vec::new();
    let mut worked = Vec::new();
    for (name, read) in &reads {
        // The frame before: whatever is not kept yet is worked out.
        let before = snapshot();
        read(&View::new(&app));
        app.drawing.clear();
        let first = since(&before);
        if KINDS.iter().any(|k| first.of(*k) > 0) {
            worked.push(*name);
        }
        // The frame after, nothing changed: a lookup.
        let before = snapshot();
        read(&View::new(&app));
        app.drawing.clear();
        let again = since(&before);
        for k in KINDS.iter().filter(|k| !PIXELS.contains(k)) {
            let n = again.of(*k);
            if n > 0 {
                failures.push(format!(
                    "{name}: read again with nothing changed, it did {k:?} {n} times: what it works out is kept (docs/SPEED.md §3.3)"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{}\n\nWorked something out the first time: {}",
        failures.join("\n"),
        worked.join(", ")
    );
}
