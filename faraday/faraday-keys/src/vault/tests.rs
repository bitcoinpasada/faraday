use super::*;

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: this crate's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

fn names() -> Names {
    let s = |t: &str| t.to_string();
    Names {
        group_keys: s("Seeds"),
        group_notes: s("Notes"),
        group_shares: s("Shares"),
        group_passphrases: s("Passphrases"),
        group_wallet_shares: s("Wallet shares"),
        title_key: s("Key {fingerprint}"),
        title_note: s("Note"),
        title_passphrase: s("Passphrase for {for}"),
        title_share: s("Codex32 share for {for}"),
        ..Names::default()
    }
}

/// Text that markup makes longest, and every character that is
/// escaped, left out or written as a reference.
fn hostile(n: usize) -> Vec<u8> {
    "&<>\"'\r\n\t\u{1}\u{3042}\u{1F600}x"
        .chars()
        .cycle()
        .scan(0, |len, ch| {
            *len += ch.len_utf8();
            (*len <= n).then_some(ch)
        })
        .collect::<String>()
        .into_bytes()
}

/// The largest plan the writer takes made of `item`s: the most items
/// before `write_plan` refuses it as too long.
fn largest_plan(item: &[u8]) -> Record {
    let plan_of = |n: usize| {
        let mut r = Record::new(kind::PLAN).with(field::PLAN_WALLET, b"w");
        for _ in 0..n {
            r.push(field::PLAN_PLACE, item);
        }
        r
    };
    let (mut low, mut high) = (1usize, 1usize);
    while fv::contents::write_plan(&plan_of(high)).is_ok() {
        low = high;
        high *= 2;
    }
    while high - low > 1 {
        let mid = (low + high) / 2;
        if fv::contents::write_plan(&plan_of(mid)).is_ok() {
            low = mid;
        } else {
            high = mid;
        }
    }
    plan_of(low)
}

/// One record of every kind the writer takes, each as large as its
/// rules let it be.
#[expect(clippy::too_many_lines, reason = "one record of each kind, listed")]
fn records_of_every_kind(mnemonic: &faraday_bip::bip39::Mnemonic) -> Vec<Record> {
    let text = |k: u8, fields: &[u8]| {
        let mut r = Record::new(k);
        for &n in fields {
            r.push(n, &b"&".repeat(20_000));
        }
        r
    };
    let sheet = fv::layouts::sheet_payload(&hostile(5_000), &hostile(5_000), &hostile(5_000))
        .expect("a sheet");
    // The kinds that are made, not typed: a key of GPG with a long user
    // ID, and an owner's three Secure Boot keys.
    let gpg = crate::gpg::create(
        &Zeroizing::new([0x31u8; 32]),
        &Zeroizing::new([0x42u8; 32]),
        1_700_000_000,
        String::from_utf8(hostile(2_000)).expect("text"),
        0,
    )
    .key
    .into_record();
    // The worst user ID: every character is written five times in the
    // title and again in the armor.
    let gpg_amps = crate::gpg::create(
        &Zeroizing::new([0x31u8; 32]),
        &Zeroizing::new([0x42u8; 32]),
        1_700_000_000,
        "&".repeat(2_000),
        0,
    )
    .key
    .into_record();
    // The most signatures for the least text: the armor holds a
    // self-signature for every user ID, however short. The record has
    // no certificate field, so no field of it grows with them.
    let gpg_many = {
        let mut r = Record::new(kind::GPG)
            .with(gpg_field::PRIMARY, &[0x31; 32])
            .with(gpg_field::CREATED, &1_700_000_000u32.to_le_bytes())
            .with(gpg_field::SUBKEY, &[0x42; 32])
            .with(gpg_field::SUBKEY_CREATED, &1_700_000_000u32.to_le_bytes());
        for _ in 0..1_000 {
            r.push(gpg_field::USER_ID, b"a");
        }
        r.push(gpg_field::EXPIRY, &0u32.to_le_bytes());
        r
    };
    let boot = crate::secure_boot::create(
        &[3u8; 32],
        &[4u8; 32],
        &String::from_utf8(hostile(200)).expect("text"),
        faraday_sb::Policy::OwnKeysOnly,
        1_700_000_000,
    )
    .expect("keys")
    .key
    .into_record();
    let mut records = vec![
        boot,
        text(kind::ENTRY, &[1, 2, 3, 4, 5, 6]),
        text(kind::NOTE, &[field::NOTE]),
        text(kind::WALLET, &[field::WALLET, field::WALLET_NAME]),
        text(
            kind::WALLET_SHARE,
            &[field::SHARE, field::SHARE_WALLET_NAME],
        ),
        text(kind::PASSPHRASE, &[field::PASSPHRASE]).with(field::PASSPHRASE_KEY, &[1; 4]),
        text(kind::SLOT_LABEL, &[field::LABEL]),
        Record::new(kind::KEY)
            .with(field::KEY, &fv::records::words_payload(mnemonic))
            .with(field::KEY_LABEL, &hostile(20_000))
            .with(field::KEY_PASSPHRASE, &b"&".repeat(20_000)),
        Record::new(kind::KEY).with(field::KEY, &fv::records::words_payload(mnemonic)),
        // A key that is a child seed of another, whose seed was split
        // into a set with every index known, and that has flags.
        Record::new(kind::KEY)
            .with(field::KEY, &fv::records::words_payload(mnemonic))
            .with(field::KEY_FLAGS, &[3])
            .with(
                field::KEY_CODEX32,
                &fv::codex32::set_payload(&fv::codex32::SetFacts {
                    threshold: 9,
                    identifier: *b"name",
                    length: 127,
                    unknown: 0,
                    strings: b"qpzry9x8gf2tvdw0s3jn54khce6mua7"
                        .iter()
                        .map(|&c| (c, fv::codex32::State::Lost))
                        .collect(),
                })
                .expect("a set"),
            )
            .with(
                field::KEY_CHILD,
                &fv::child::payload(&fv::child::ChildOf {
                    parent: [0xff; 4],
                    language: 0,
                    words: 12,
                    index: 0x7fff_ffff,
                })
                .expect("a child"),
            ),
        Record::new(kind::SHEET).with(field::SHEET, &sheet),
        gpg,
        gpg_amps,
        gpg_many,
        Record::new(kind::CODEX32_SHARE)
            .with(1, b"ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm")
            .with(2, b"name"),
    ];
    let mut amounts = Record::new(kind::AMOUNTS).with(1, &[0xff; 32]);
    for _ in 0..10_000 {
        amounts.push(2, &[0xff; 8]);
    }
    records.push(amounts);
    for item in [
        &b"x"[..],
        b"&",
        b"\\",
        b"\"",
        b"'",
        b"\t",
        &hostile(100)[..],
    ] {
        records.push(largest_plan(item));
    }
    records
}

/// A record never takes more of the document than its bound says.
#[test]
fn a_seal_never_writes_a_record_past_its_bound() {
    let c = compute();
    let file = create(
        &c,
        [7u8; 16],
        "Test",
        fv::Cost {
            memory_kib: 8192,
            passes: 1,
            lanes: 1,
        },
        "pass",
        &[7u8; SEAL_RANDOM_LEN],
        None,
    )
    .expect("a vault");
    let mut open = OpenVaults::default();
    open.set_names(names());
    let (id, _) = open.unlock(&c, file, "pass").expect("opens");
    let s = open.slot(id).expect("held");
    let base = s.used(&c, &open.names);
    let mnemonic = faraday_bip::bip39::Mnemonic::parse(
        &c,
        faraday_bip::bip39::Language::English,
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
         abandon abandon about",
    )
    .expect("words");
    let records = records_of_every_kind(&mnemonic);
    let mut written = 0;
    for r in &records {
        let add = [r];
        let ov = Overlay {
            remove: &[],
            add: &add,
            edit: &[],
        };
        let exact = s
            .used_with(&c, &open.names, &ov)
            .unwrap_or_else(|e| panic!("kind {} is not written: {e:?}", r.kind));
        written += 1;
        let bound = s.bound(&open.names, &ov);
        assert!(
            exact <= bound,
            "kind {}: the seal takes {exact}, the bound says {bound} (base {base})",
            r.kind
        );
    }
    assert_eq!(written, records.len());
}

#[test]
fn a_vaults_sealed_time_is_read_only_when_it_names_a_real_moment() {
    assert_eq!(unix_of_rfc3339("2026-10-28T14:02:00Z"), Some(1_793_196_120));
    assert_eq!(unix_of_rfc3339("2024-02-29T00:00:00Z"), Some(1_709_164_800));
    for bad in [
        "2026-02-31T00:00:00Z",
        "2025-02-29T00:00:00Z",
        "2026-10-28T-1:02:00Z",
        "2026-10-28T14:+2:00Z",
        "+026-10-28T14:02:00Z",
        "2026-1 -28T14:02:00Z",
        "2026-10-28T24:00:00Z",
        "2026-10-28T14:02:00",
    ] {
        assert_eq!(unix_of_rfc3339(bad), None, "{bad}");
    }
}
