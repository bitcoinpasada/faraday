//! The file vault (`faraday_vault::file`, `docs/VAULT.md` §2 to §5, §7.1,
//! §8): made, opened, changed and sealed again, its count going up; the
//! four sizes' file lengths; a file a KeePass app saved opened and sealed
//! as Faraday writes a vault; a read-only vault refused; a seal that does
//! not fit named by the bytes over.

use std::cell::Cell;
use std::path::PathBuf;

use faraday_crypto::work::{Compute, Work};
use faraday_vault::file::{
    self, Change, DEFAULT_SIZE, Error, Facts, Opened, ReadOnly, Rekey, SIZES, STEPS_KIB, file_len,
};
use faraday_vault::kdbx::items::{Body, Format, Place};
use faraday_vault::kdbx::xml::Document;
use faraday_vault::kdbx::{self, RANDOM_LEN, items};
use faraday_vault::records::{field, kind};
use faraday_vault::{Cost, Record};

const PASSPHRASE: &str = "kdbx vector passphrase";
const ID: [u8; 16] = [0x5a; 16];
/// Argon2 at the least a reader accepts: 8 MiB, one pass, one lane.
const CHEAP: Cost = Cost {
    memory_kib: 8 * 1024,
    passes: 1,
    lanes: 1,
};
/// 2023-11-14T22:13:20Z.
const NOW: u64 = 1_700_000_000;

thread_local! {
    static STRETCHES: Cell<usize> = const { Cell::new(0) };
}

fn count(work: Work) {
    if matches!(work, Work::Kdf) {
        STRETCHES.set(STRETCHES.get() + 1);
    }
}

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-vault's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(count)
}

fn random(seed: u8) -> [u8; RANDOM_LEN] {
    std::array::from_fn(|i| (i as u8).wrapping_mul(7).wrapping_add(seed))
}

fn vector(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/vectors/kdbx")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn made(size: usize) -> Vec<u8> {
    file::create(
        &compute(),
        ID,
        "Savings",
        CHEAP,
        size,
        "correct horse",
        &random(1),
        Some(NOW),
    )
    .expect("creates")
}

fn open(bytes: &[u8], passphrase: &str) -> Opened {
    match file::open(&compute(), bytes, passphrase) {
        Ok(o) => o,
        Err(e) => panic!("does not open: {e:?}"),
    }
}

fn note(n: u8, text: &str) -> (Record, [u8; 16]) {
    (
        Record::new(kind::NOTE).with(field::NOTE, text.as_bytes()),
        [n; 16],
    )
}

fn place(uuid: [u8; 16]) -> Place<'static> {
    Place {
        uuid,
        group: "Notes",
        group_uuid: [200; 16],
        title: "Note",
        now: None,
    }
}

#[test]
fn a_vault_is_made_opened_and_sealed_again_with_its_count_going_up() {
    let first = made(DEFAULT_SIZE);
    let o = open(&first, "correct horse");
    assert_eq!(o.read().seals, 1);
    assert_eq!(o.read().sealed.as_deref(), Some("2023-11-14T22:13:20Z"));
    assert_eq!(o.read().label, "Savings");
    assert_eq!(o.vault_id(), *b"5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a");
    let second = file::seal(&compute(), &o, &[], &random(2), Some(NOW + 100)).expect("seals");
    let o = open(&second, "correct horse");
    assert_eq!(o.read().seals, 2);
    assert_eq!(o.read().sealed.as_deref(), Some("2023-11-14T22:15:00Z"));
    let third = file::seal(&compute(), &o, &[], &random(3), None).expect("seals");
    let o = open(&third, "correct horse");
    assert_eq!(o.read().seals, 3);
    assert_eq!(o.read().sealed, None);
    // The seals went to the step at or above the file's memory.
    assert_eq!(o.facts().header.cost.memory_kib, STEPS_KIB[0]);
    assert_eq!(o.facts().header.cost.passes, 3);
}

#[test]
fn the_clock_is_written_as_utc_across_a_leap_day() {
    let o = open(&made(SIZES[0]), "correct horse");
    for (unix, text) in [
        (951_782_400, "2000-02-29T00:00:00Z"),
        (951_868_799, "2000-02-29T23:59:59Z"),
        (951_868_800, "2000-03-01T00:00:00Z"),
        (1_767_225_599, "2025-12-31T23:59:59Z"),
    ] {
        let sealed = file::seal(&compute(), &o, &[], &random(2), Some(unix)).unwrap();
        let again = open(&sealed, "correct horse");
        assert_eq!(again.read().sealed.as_deref(), Some(text));
    }
}

#[test]
fn each_size_has_the_file_length_the_spec_gives() {
    for (size, len) in SIZES
        .into_iter()
        .zip([65_921, 262_529, 1_048_961, 4_194_797])
    {
        let bytes = made(size);
        assert_eq!(bytes.len(), len);
        assert_eq!(file_len(size), len);
        let facts = file::inspect(&bytes).expect("inspects");
        assert_eq!(facts.size, Some(size));
        assert_eq!(facts.len, len);
        assert!(!facts.other_settings());
    }
}

#[test]
fn a_size_that_is_not_one_is_refused() {
    let r = file::create(&compute(), ID, "x", CHEAP, 100_000, "p", &random(1), None);
    assert_eq!(r.err(), Some(Error::Size));
}

#[test]
fn two_seals_with_different_random_bytes_are_different_files() {
    let o = open(&made(SIZES[0]), "correct horse");
    let a = file::seal(&compute(), &o, &[], &random(10), Some(NOW)).unwrap();
    let b = file::seal(&compute(), &o, &[], &random(11), Some(NOW)).unwrap();
    assert_ne!(a, b);
    assert_eq!(a.len(), b.len());
}

#[test]
fn a_wrong_passphrase_is_named_and_the_two_forms_of_a_passphrase_are_one() {
    let bytes = file::create(
        &compute(),
        ID,
        "x",
        CHEAP,
        SIZES[0],
        "caf\u{e9}",
        &random(1),
        None,
    )
    .unwrap();
    assert_eq!(
        file::open(&compute(), &bytes, "cafe").err(),
        Some(Error::Kdbx(kdbx::Error::Passphrase))
    );
    assert_eq!(open(&bytes, "cafe\u{301}").read().seals, 1);
}

#[test]
fn a_seal_after_opening_with_the_decomposed_form_opens_with_the_composed_one() {
    let bytes = file::create(
        &compute(),
        ID,
        "x",
        CHEAP,
        SIZES[0],
        "caf\u{e9}",
        &random(1),
        None,
    )
    .unwrap();
    let o = open(&bytes, "cafe\u{301}");
    let sealed = file::seal(&compute(), &o, &[], &random(2), None).unwrap();
    assert_eq!(open(&sealed, "caf\u{e9}").read().seals, 2);
}

#[test]
fn a_compressed_file_the_length_of_a_size_does_not_state_that_size() {
    let mut bytes = vector("pykeepass-aes-argon2d-gzip.kdbx");
    let head = kdbx::head_len(&bytes).unwrap();
    // Two block frames follow a plaintext of the smallest size.
    let want = head + SIZES[0] + 72;
    assert!(bytes.len() < want);
    bytes.resize(want, 0);
    assert_eq!(file::inspect(&bytes).unwrap().size, None);
}

#[test]
fn a_file_a_keepass_app_saved_opens_and_is_sealed_as_faraday_writes_one() {
    let bytes = vector("pykeepass-aes-argon2d-gzip.kdbx");
    let facts = file::inspect(&bytes).unwrap();
    assert_eq!(facts.size, None);
    assert!(facts.other_settings());
    let o = open(&bytes, PASSPHRASE);
    assert_eq!(o.read().format, Format::Foreign);
    assert_eq!(o.read().seals, 0);
    assert_eq!(o.read().items.len(), 1);
    assert_eq!(o.next_size(&compute(), &[], Some(NOW)), Ok(SIZES[0]));
    let sealed = file::seal(&compute(), &o, &[], &random(5), Some(NOW)).unwrap();
    assert_eq!(sealed.len(), file_len(SIZES[0]));
    let facts = file::inspect(&sealed).unwrap();
    assert!(!facts.other_settings());
    assert_eq!(facts.size, Some(SIZES[0]));
    let o = open(&sealed, PASSPHRASE);
    assert_eq!(o.read().format, Format::Faraday);
    assert_eq!(o.read().seals, 1);
    assert_eq!(o.read().items.len(), 1);
}

#[test]
fn a_file_with_more_than_three_passes_keeps_them_and_a_small_memory_takes_the_first_step() {
    let cost = Cost {
        memory_kib: 8 * 1024,
        passes: 5,
        lanes: 1,
    };
    let bytes = file::create(&compute(), ID, "x", cost, SIZES[0], "p", &random(1), None).unwrap();
    let o = open(&bytes, "p");
    let next = o.next_cost();
    assert_eq!((next.memory_kib, next.passes, next.lanes), (65_536, 5, 4));
    let o = open(&made(SIZES[0]), "correct horse");
    assert_eq!(o.next_cost().passes, 3);
}

#[test]
fn history_and_the_recycle_bin_are_kept_at_the_seal() {
    let bytes = vector("pykeepass-chacha20-argon2id-plain-history.kdbx");
    let o = open(&bytes, PASSPHRASE);
    assert_eq!((o.read().history, o.read().recycle_bin), (1, 1));
    let sealed = file::seal(&compute(), &o, &[], &random(6), None).unwrap();
    let o = open(&sealed, PASSPHRASE);
    assert_eq!((o.read().history, o.read().recycle_bin), (1, 1));
}

#[test]
fn history_and_the_recycle_bin_count_toward_the_size() {
    let bytes = vector("pykeepass-chacha20-argon2id-plain-history.kdbx");
    let o = open(&bytes, PASSPHRASE);
    let held = kdbx::open(&compute(), &bytes, PASSPHRASE)
        .expect("opens")
        .document
        .len();
    let used = o.used(&compute(), &[]).unwrap();
    assert!(used >= held, "{used} bytes used of {held} in the document");
    // Notes that fill what is left after the history and the bin fit, and
    // more than fill it do not, at the largest size.
    let largest = SIZES[SIZES.len() - 1];
    let each = 4000;
    let room = (largest - used) / each;
    let try_notes = |n: usize| {
        let bodies: Vec<Body> = (0..n)
            .map(|_| Body::Record(note(0, &"n".repeat(each)).0))
            .collect();
        let changes: Vec<Change> = bodies
            .iter()
            .enumerate()
            .map(|(i, body)| Change::Put {
                place: place([
                    (i % 250) as u8 + 1,
                    (i / 250) as u8,
                    9,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                ]),
                body,
            })
            .collect();
        file::seal(&compute(), &o, &changes, &random(7), None)
    };
    let half = try_notes(room / 2);
    assert!(half.is_ok(), "{:?}", half.err());
    assert!(matches!(
        try_notes(room + 1),
        Err(Error::Full { over, next: None }) if over > 0
    ));
}

#[test]
fn a_note_in_the_recycle_bin_or_in_a_history_is_not_an_item() {
    let o = open(&made(SIZES[0]), "correct horse");
    let (r, u) = note(1, "in the bin");
    let body = Body::Record(r);
    let change = [Change::Put {
        place: place(u),
        body: &body,
    }];
    let sealed = file::seal(&compute(), &o, &change, &random(7), Some(NOW)).unwrap();
    assert_eq!(open(&sealed, "correct horse").read().items.len(), 1);

    let k = kdbx::open(&compute(), &sealed, "correct horse").expect("opens");
    let doc = Document::read(&k.document, &k.inner_key).unwrap();
    let random = random(9);
    let text = String::from_utf8(doc.write(kdbx::inner_key_of(&random)).to_vec()).unwrap();
    let seal = |xml: &str| {
        let contents = kdbx::Contents {
            document: xml.as_bytes(),
            binaries: &[],
            public_custom_data: None,
        };
        kdbx::seal(&compute(), "p", CHEAP, &random, &contents, SIZES[0]).unwrap()
    };
    // The group the note is in becomes the recycle bin.
    let name = text.find("<Name>Notes</Name>").unwrap();
    let from = text[..name].rfind("<UUID>").unwrap();
    let group = &text[from..name];
    let in_bin = text.replacen(
        "<RecycleBinUUID>AAAAAAAAAAAAAAAAAAAAAA==</RecycleBinUUID>",
        &format!(
            "<RecycleBinUUID>{}</RecycleBinUUID>",
            group
                .trim_start_matches("<UUID>")
                .trim_end_matches("</UUID>")
        ),
        1,
    );
    assert_ne!(in_bin, text);
    let o = open(&seal(&in_bin), "p");
    assert!(o.read().items.is_empty(), "a key in the bin loaded");
    assert_eq!(o.read().recycle_bin, 1);

    // The note's entry becomes a history version of a plain entry.
    let at = text.find("<Entry>").unwrap();
    let end = text.find("</Entry>").unwrap() + "</Entry>".len();
    let entry = &text[at..end];
    let in_history = format!(
        "{}<Entry><UUID>AwMDAwMDAwMDAwMDAwMDAw==</UUID><History>{entry}</History></Entry>{}",
        &text[..at],
        &text[end..]
    );
    let o = open(&seal(&in_history), "p");
    // The plain entry around it is the one item; the note in its history is not.
    assert_eq!(o.read().items.len(), 1, "a key in a history loaded");
    assert!(
        o.read().items.iter().all(|i| i.uuid != u),
        "the note in a history is an item"
    );
    assert_eq!(o.read().history, 1);
}

#[test]
fn what_does_not_fit_is_named_by_the_bytes_over_and_the_next_size_and_the_vault_stays_open() {
    let o = open(&made(SIZES[0]), "correct horse");
    let (a, ua) = note(1, &"a".repeat(40_000));
    let (b, ub) = note(2, &"b".repeat(40_000));
    let (body_a, body_b) = (Body::Record(a), Body::Record(b));
    let changes = [
        Change::Put {
            place: place(ua),
            body: &body_a,
        },
        Change::Put {
            place: place(ub),
            body: &body_b,
        },
    ];
    let used = o.used(&compute(), &changes).unwrap();
    assert!(used > SIZES[0]);
    let err = file::seal(&compute(), &o, &changes, &random(7), Some(NOW)).err();
    assert_eq!(
        err,
        Some(Error::Full {
            over: used - SIZES[0],
            next: Some(SIZES[1]),
        })
    );
    // The vault is as it was, and seals once the change is smaller.
    assert!(o.used(&compute(), &[]).unwrap() < SIZES[0]);
    let sealed = file::seal(&compute(), &o, &changes[..1], &random(7), Some(NOW)).unwrap();
    assert_eq!(open(&sealed, "correct horse").read().items.len(), 1);
}

/// A vault file whose document is `doc`, sealed here.
fn sealed_document(doc: &Document) -> Vec<u8> {
    let random = random(9);
    let xml = doc.write(kdbx::inner_key_of(&random));
    let contents = kdbx::Contents {
        document: &xml,
        binaries: &[],
        public_custom_data: None,
    };
    kdbx::seal(&compute(), "p", CHEAP, &random, &contents, SIZES[0]).unwrap()
}

#[test]
fn a_read_only_vault_refuses_every_change_for_either_reason() {
    let mut newer = Document::new(ID, "Newer");
    newer.custom_mut().set("faraday.format", "2").unwrap();
    let mut text = String::from_utf8(Document::new(ID, "Two").write(&[0; 64]).to_vec()).unwrap();
    let at = text.find("</DatabaseName>").unwrap() + "</DatabaseName>".len();
    text.insert_str(at, "<DatabaseName>second</DatabaseName>");
    let ambiguous = Document::read(text.as_bytes(), &[0; 64]).unwrap();

    let (r, u) = note(1, "x");
    let body = Body::Record(r);
    let plan = Record::new(kind::PLAN)
        .with(field::PLAN_PLACE, b"Home")
        .with(field::PLAN_ANSWERS, b"seeds 1");
    let signed = Record::new(kind::AMOUNTS)
        .with(field::AMOUNTS_TX, &[1; 32])
        .with(field::AMOUNTS_INPUT, &5u64.to_le_bytes());
    for (doc, is_newer) in [(&newer, true), (&ambiguous, false)] {
        let o = open(&sealed_document(doc), "p");
        match (o.read_only(), is_newer) {
            (Some(ReadOnly::Newer(f)), true) => assert_eq!(f, "2"),
            (Some(ReadOnly::Ambiguous(_)), false) => {}
            (other, _) => panic!("{other:?}"),
        }
        for change in [
            Change::Put {
                place: place(u),
                body: &body,
            },
            Change::Plan {
                n: 1,
                record: &plan,
            },
            Change::Signed(&signed),
        ] {
            let changes = [change];
            assert_eq!(
                file::seal(&compute(), &o, &changes, &random(1), None).err(),
                Some(Error::ReadOnly)
            );
            assert_eq!(o.used(&compute(), &changes).err(), Some(Error::ReadOnly));
        }
        assert_eq!(
            file::seal(&compute(), &o, &[], &random(1), None).err(),
            Some(Error::ReadOnly)
        );
    }
}

#[test]
fn a_new_plan_takes_the_next_number_in_an_opened_vault() {
    let o = open(&made(SIZES[0]), "correct horse");
    assert_eq!(items::next_plan(o.document()), Ok(1));
}

#[test]
fn measuring_runs_one_stretch_at_the_stated_cost() {
    let bytes = made(SIZES[0]);
    let facts: Facts = file::inspect(&bytes).unwrap();
    let before = STRETCHES.get();
    file::measure(&compute(), &facts).unwrap();
    assert_eq!(STRETCHES.get(), before + 1);
}

/// What pykeepass reads of `file` under `passphrase`: its output, or none
/// where pykeepass is not installed (said, not silent).
fn pykeepass_reads(file: &[u8], passphrase: &str) -> Option<String> {
    let dir = std::env::temp_dir().join(format!("faraday-file-rekey-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("vault.kdbx");
    std::fs::write(&path, file).unwrap();
    let out = std::process::Command::new("python3")
        .args(["-I", "-B"])
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/vectors/kdbx/reread.py"))
        .arg(&path)
        .arg(passphrase)
        .output();
    let _ = std::fs::remove_dir_all(&dir);
    let out = out.ok()?;
    let err = String::from_utf8_lossy(&out.stderr);
    if !out.status.success() && (err.contains("ModuleNotFoundError") || err.contains("ImportError"))
    {
        eprintln!("pykeepass is not installed: skipped");
        return None;
    }
    assert!(out.status.success(), "pykeepass: {err}");
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[test]
fn a_seal_under_a_new_passphrase_and_step_opens_under_the_new_and_not_the_old() {
    let old = made(SIZES[0]);
    let o = open(&old, "correct horse");
    let (record, uuid) = note(9, "kept");
    let body = Body::Record(record);
    let changes = [Change::Put {
        place: place(uuid),
        body: &body,
    }];
    let first = file::seal(&compute(), &o, &changes, &random(2), Some(NOW)).unwrap();
    let o = open(&first, "correct horse");
    let rekey = Rekey {
        passphrase: Some("battery staple"),
        memory_kib: Some(STEPS_KIB[1]),
    };
    let sealed = file::seal_with(&compute(), &o, &[], rekey, &random(3), Some(NOW + 5)).unwrap();
    // The header states the new cost; the count is one higher.
    let n = open(&sealed, "battery staple");
    assert_eq!(n.facts().header.cost.memory_kib, STEPS_KIB[1]);
    assert_eq!(n.facts().header.cost.passes, 3);
    assert_eq!(n.read().seals, o.read().seals + 1);
    assert_eq!(n.read().items.len(), o.read().items.len());
    assert_eq!(o.read().items.len(), 1);
    assert!(matches!(
        file::open(&compute(), &sealed, "correct horse"),
        Err(Error::Kdbx(kdbx::Error::Passphrase))
    ));
    // An older copy opens under the old passphrase.
    assert_eq!(open(&first, "correct horse").read().seals, o.read().seals);
    // Another implementation reads the new file under the new passphrase.
    if let Some(text) = pykeepass_reads(&sealed, "battery staple") {
        assert!(text.contains("Note"), "{text}");
    }
    // The opened file is as it was: a seal without a rekey keeps the old key.
    let again = file::seal(&compute(), &o, &[], &random(4), None).unwrap();
    assert_eq!(
        open(&again, "correct horse").facts().header.cost.memory_kib,
        STEPS_KIB[0]
    );
}

#[test]
fn a_seal_refuses_an_empty_passphrase_and_a_step_that_is_none_and_can_be_retried() {
    let o = open(&made(SIZES[0]), "correct horse");
    let empty = Rekey {
        passphrase: Some(""),
        memory_kib: None,
    };
    assert!(matches!(
        file::seal_with(&compute(), &o, &[], empty, &random(2), None),
        Err(Error::Kdbx(kdbx::Error::EmptyPassphrase))
    ));
    let odd = Rekey {
        passphrase: None,
        memory_kib: Some(100_000),
    };
    assert!(file::seal_with(&compute(), &o, &[], odd, &random(2), None).is_err());
    let retry = Rekey {
        passphrase: Some("new one"),
        memory_kib: None,
    };
    let sealed = file::seal_with(&compute(), &o, &[], retry, &random(2), None).unwrap();
    assert_eq!(open(&sealed, "new one").read().seals, 2);
}
