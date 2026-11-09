//! The committed vectors (`tools/vectors/vault/`, `docs/VAULT.md` §10) are
//! what this build writes; each opens under its passphrase to what it was
//! made from, and each refused file is refused under its name.

#[path = "../examples/vectors.rs"]
mod vectors;

use faraday_vault::file::{self, Error, STEP, STEPS_KIB, file_len};
use faraday_vault::kdbx::items::Format;
use faraday_vault::kdbx::{self, Unsupported};
use faraday_vault::{Cost, Record};

fn committed(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/vectors/vault")
        .join(name);
    std::fs::read(path).expect("the committed vector")
}

fn open(name: &str, passphrase: &str) -> file::Opened {
    match file::open(&vectors::compute(), &committed(name), passphrase) {
        Ok(o) => o,
        Err(e) => panic!("{name} does not open: {e:?}"),
    }
}

#[test]
fn the_committed_vectors_are_what_this_build_writes() {
    for (name, bytes) in vectors::vectors() {
        assert!(
            committed(name) == bytes,
            "{name} differs from what this build writes"
        );
    }
}

#[test]
fn the_vault_of_every_kind_opens_to_what_it_was_made_from() {
    let o = open(vectors::ALL, vectors::PASSPHRASE);
    let read = o.read();
    assert_eq!(read.label, "Everything");
    assert_eq!(read.format, Format::Faraday);
    assert_eq!(
        (read.seals, read.sealed.as_deref()),
        (1, Some("2023-11-14T22:13:20Z"))
    );
    assert_eq!(o.vault_id(), *b"01010101010101010101010101010101");
    // Twelve items, a plan and signed amounts, none unreadable.
    assert_eq!(read.items.len(), 12);
    assert_eq!(read.custom.len(), 2);
    assert!(read.unreadable.is_empty());
    assert_eq!(read.history + read.recycle_bin, 0);
    // The size and cost the file states.
    let facts = o.facts();
    assert_eq!(facts.size, Some(vectors::BIG_SIZE));
    assert_eq!(facts.len, file_len(vectors::BIG_SIZE));
    assert_eq!(facts.header.cost, vectors::COST);
    // A wrong passphrase is named, and so is none.
    assert_eq!(
        file::open(&vectors::compute(), &committed(vectors::ALL), "not it").err(),
        Some(Error::Kdbx(kdbx::Error::Passphrase))
    );
    assert_eq!(
        file::open(&vectors::compute(), &committed(vectors::ALL), "").err(),
        Some(Error::Kdbx(kdbx::Error::EmptyPassphrase))
    );
}

#[test]
fn the_small_and_the_codex32_vaults_open() {
    let small = open(vectors::SMALL, vectors::PASSPHRASE);
    assert_eq!(small.facts().len, file_len(vectors::SMALL_SIZE));
    assert_eq!(small.read().label, "Small");
    assert_eq!(small.read().items.len(), 2);
    let shares = open(vectors::CODEX32, vectors::PASSPHRASE);
    assert_eq!(shares.read().label, "Shares");
    assert_eq!(shares.read().items.len(), 2);
    assert!(shares.read().unreadable.is_empty());
}

/// §3: made with the passphrase in NFD, the vector opens under it in NFC.
#[test]
fn the_nfd_vector_opens_under_either_form() {
    for p in [vectors::NFD_PASSPHRASE, vectors::NFD_AS_NFC] {
        let o = open(vectors::NFD, p);
        assert_eq!(o.read().label, "Accents");
        assert_eq!(o.read().seals, 1);
    }
}

#[test]
fn the_vault_sealed_through_file_seal_is_at_the_cost_a_seal_writes() {
    let o = open(vectors::SEALED, vectors::PASSPHRASE);
    assert_eq!(o.read().seals, 2);
    assert_eq!(o.read().sealed.as_deref(), Some("2023-11-14T22:14:20Z"));
    assert_eq!(o.read().label, "Sealed again");
    assert_eq!(o.read().items.len(), 2);
    assert_eq!(
        o.facts().header.cost,
        Cost {
            memory_kib: STEPS_KIB[0],
            passes: 3,
            lanes: 4
        }
    );
    assert_eq!(o.facts().size, Some(STEP));
}

#[test]
fn a_stream_that_ends_early_is_damage() {
    assert_eq!(
        file::open(
            &vectors::compute(),
            &committed(vectors::CUT),
            vectors::PASSPHRASE
        )
        .err(),
        Some(Error::Kdbx(kdbx::Error::Damaged))
    );
}

#[test]
fn each_refused_file_is_refused_under_its_name() {
    let refused = vectors::refused();
    assert!(refused.len() >= 30);
    for (name, bytes, why) in refused {
        assert!(
            committed(name) == bytes,
            "{name} is not what this build writes"
        );
        let got = file::open(&vectors::compute(), &bytes, vectors::PASSPHRASE).err();
        assert_eq!(got, Some(why), "{name}");
    }
}

/// The two foreign refusals the vectors name by reference
/// (`tools/vectors/kdbx/`): KeePassXC's KDBX 3.1 file and pykeepass's
/// Twofish file.
#[test]
fn the_foreign_files_of_an_older_version_and_of_twofish_are_refused() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/vectors/kdbx");
    let old = std::fs::read(dir.join("keepassxc-3.1-aeskdf.kdbx")).unwrap();
    assert_eq!(
        file::open(&vectors::compute(), &old, "pw").err(),
        Some(Error::Kdbx(kdbx::Error::Older { major: 3 }))
    );
    let twofish = std::fs::read(dir.join("pykeepass-twofish-argon2d-gzip.kdbx")).unwrap();
    assert_eq!(
        file::open(&vectors::compute(), &twofish, "kdbx vector passphrase").err(),
        Some(Error::Kdbx(kdbx::Error::Unsupported(Unsupported::Twofish)))
    );
}

/// §8: a vault is not made with an empty passphrase.
#[test]
fn an_empty_passphrase_is_refused_at_create() {
    let got = file::create(
        &vectors::compute(),
        [1; 16],
        "Empty",
        vectors::COST,
        "",
        &vectors::random(1),
        None,
    );
    assert_eq!(got.err(), Some(Error::Kdbx(kdbx::Error::EmptyPassphrase)));
}

/// One of every record the vectors hold reads back as the record it was
/// put in: the seed's fields and its two sets of facts.
#[test]
fn the_seed_reads_back_with_its_fields_and_facts() {
    use faraday_vault::records::{field, kind};
    let o = open(vectors::ALL, vectors::PASSPHRASE);
    let seed: &Record = o
        .read()
        .items
        .iter()
        .find_map(|i| match &i.body {
            faraday_vault::kdbx::items::Body::Record(r) if r.kind == kind::KEY => Some(r),
            _ => None,
        })
        .expect("a seed");
    assert_eq!(seed.text(field::KEY_LABEL), Some("Savings"));
    assert_eq!(seed.text(field::KEY_PASSPHRASE), Some("TREZOR"));
    assert!(seed.field(field::KEY_CODEX32).is_some());
    assert!(seed.field(field::KEY_CHILD).is_some());
}
