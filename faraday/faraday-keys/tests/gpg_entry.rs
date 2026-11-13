//! A GPG key as a KDBX vault's entry holds it (`docs/VAULT.md` §7) and as
//! the app's record: the one becomes the other and back, and a key that is
//! not in Faraday's own shape is not taken.
#![allow(clippy::disallowed_methods)]

use faraday_keys::gpg::{gpg_key_of, record_of};
use faraday_keys::vault::gpg_field as f;
use faraday_pgp::{Armor, Hash, Key, Reason, armor};
use faraday_text::Text;
use faraday_vault::Record;
use faraday_vault::kdbx::items::GpgKey;
use faraday_vault::records::kind;

const MADE: u32 = 1_780_000_000;

fn key(seed: u8) -> Key {
    Key {
        primary_seed: [seed; 32],
        created: MADE,
        subkey_seed: [seed + 1; 32],
        subkey_created: MADE,
        user_ids: vec![Text::of("Test Person <test@example.com>")],
        expiry: 31_536_000,
    }
}

fn record(k: &Key, revoked: Option<u32>) -> Record {
    let mut r = Record::new(kind::GPG)
        .with(f::PRIMARY, &k.primary_seed)
        .with(f::CREATED, &k.created.to_le_bytes())
        .with(f::SUBKEY, &k.subkey_seed)
        .with(f::SUBKEY_CREATED, &k.subkey_created.to_le_bytes())
        .with(f::USER_ID, k.user_ids[0].as_bytes())
        .with(f::EXPIRY, &k.expiry.to_le_bytes())
        .with(f::CERTIFICATE, &k.certificate(MADE))
        .with(f::REVOCATION, &k.revocation(MADE, Reason::Unspecified, ""));
    if let Some(at) = revoked {
        r.push(f::REVOKED, &at.to_le_bytes());
    }
    r
}

fn entry(k: &Key) -> GpgKey {
    GpgKey {
        title: k.user_ids[0].clone(),
        key: Text::from_utf8_lossy(&faraday_pgp::armor_secret(k, MADE)),
        public: Some(armor(Armor::PublicKey, &k.certificate(MADE))),
        revocation: Some(armor(
            Armor::PublicKey,
            &k.revocation(MADE, Reason::Unspecified, ""),
        )),
        revoked: None,
    }
}

fn same(a: &Record, b: &Record) -> bool {
    a.kind == b.kind
        && a.fields.len() == b.fields.len()
        && (1..=9).all(|n| a.field(n) == b.field(n))
}

#[test]
fn a_record_becomes_an_entry_and_the_entry_the_same_record() {
    let k = key(0x11);
    for revoked in [None, Some(1_790_000_000)] {
        let r = record(&k, revoked);
        let g = gpg_key_of(&r, MADE).expect("an entry");
        assert_eq!(g.title, "Test Person <test@example.com>");
        assert!(g.key.starts_with("-----BEGIN PGP PRIVATE KEY BLOCK-----"));
        assert_eq!(g.revoked.is_some(), revoked.is_some());
        if revoked.is_some() {
            assert_eq!(g.revoked.as_deref(), Some("2026-09-21T14:13:20Z"));
        }
        let back = record_of(&g).expect("a record");
        assert!(same(&back, &r));
    }
}

#[test]
fn an_entry_without_its_certificate_has_it_made_again() {
    let k = key(0x11);
    let mut g = entry(&k);
    g.public = None;
    g.revocation = None;
    let r = record_of(&g).expect("a record");
    assert_eq!(r.field(f::CERTIFICATE), Some(&k.certificate(MADE)[..]));
    assert!(r.field(f::REVOCATION).is_none());
}

#[test]
fn an_entry_that_is_not_faradays_own_key_makes_no_record() {
    let k = key(0x11);
    assert!(record_of(&entry(&k)).is_some());

    // The key block of another program: a changed byte, a public block.
    let mut g = entry(&k);
    let mut packets = faraday_pgp::dearmor(
        b"-----BEGIN PGP PRIVATE KEY BLOCK-----",
        b"-----END PGP PRIVATE KEY BLOCK-----",
        g.key.as_bytes(),
    )
    .expect("packets")
    .to_vec();
    packets[20] ^= 1;
    g.key = Text::of(&armor(Armor::PublicKey, &packets).replace("PUBLIC", "PRIVATE"));
    assert!(record_of(&g).is_none());
    let mut g = entry(&k);
    g.key = g.public.as_ref().unwrap().duplicate();
    assert!(record_of(&g).is_none());

    // A certificate of another key, or that does not read.
    let mut g = entry(&k);
    g.public = Some(armor(Armor::PublicKey, &key(0x33).certificate(MADE)));
    assert!(record_of(&g).is_none());
    let mut g = entry(&k);
    g.public = Some(Text::of(
        "-----BEGIN PGP PUBLIC KEY BLOCK-----\n\nAAAA\n-----END PGP PUBLIC KEY BLOCK-----\n",
    ));
    assert!(record_of(&g).is_none());

    // A revocation certificate that is not a signature: a user ID packet.
    let mut g = entry(&k);
    g.revocation = Some(armor(Armor::PublicKey, b"\xCD\x01x"));
    assert!(record_of(&g).is_none());
}

#[test]
fn a_revocation_is_the_keys_own_and_nothing_else() {
    let k = key(0x11);
    let with = |packets: Vec<u8>| {
        let mut g = entry(&k);
        g.revocation = Some(armor(Armor::PublicKey, &packets));
        record_of(&g)
    };
    // Its own, at another time, for another reason, with a text.
    assert!(with(k.revocation(MADE + 5, Reason::Retired, "no longer used")).is_some());
    // Another key's revocation, a certificate, a document signature.
    assert!(with(key(0x33).revocation(MADE, Reason::Unspecified, "")).is_none());
    assert!(with(k.certificate(MADE)).is_none());
    assert!(with(k.sign(b"a document", MADE, Hash::Sha512)).is_none());
    // The revocation and a second signature.
    let mut two = k.revocation(MADE, Reason::Unspecified, "");
    two.extend(k.revocation(MADE, Reason::Unspecified, ""));
    assert!(with(two).is_none());
}

#[test]
fn a_revocation_is_read_with_its_length_in_the_form_it_is_written_in() {
    let k = key(0x11);
    let with = |packets: &[u8]| {
        let mut g = entry(&k);
        g.revocation = Some(armor(Armor::PublicKey, packets));
        record_of(&g).is_some()
    };
    // The packet's length is one octet below 192 and two above; the
    // reason's own length is one octet only.
    for (n, ok) in [(0, true), (40, true), (60, true), (100, true), (189, true)] {
        let text = "r".repeat(n);
        assert_eq!(with(&k.revocation(MADE, Reason::Retired, &text)), ok, "{n}");
    }
    for n in [190, 191, 300] {
        let text = "r".repeat(n);
        assert!(!with(&k.revocation(MADE, Reason::Retired, &text)), "{n}");
    }
    // The same signature under a header of another form, or cut short.
    let own = k.revocation(MADE, Reason::Retired, "no longer used");
    let (head, body) = if own[1] < 192 {
        (2, &own[2..])
    } else {
        (3, &own[3..])
    };
    assert!(with(&own));
    let five = [&[own[0], 255][..], &(body.len() as u32).to_be_bytes(), body].concat();
    assert!(!with(&five));
    let partial = [&[own[0], 0xE9][..], body].concat();
    assert!(!with(&partial));
    assert!(!with(&own[..own.len() - 1]));
    assert!(!with(&[own.as_slice(), &[0]].concat()));
    assert!(!with(&own[..head - 1]));
    assert!(!with(&own[..head]));
}

#[test]
fn a_certificate_is_the_keys_own_at_its_expiry_and_nothing_else() {
    let k = key(0x11);
    let with = |packets: Vec<u8>| {
        let mut g = entry(&k);
        g.public = Some(armor(Armor::PublicKey, &packets));
        record_of(&g)
    };
    assert!(with(k.certificate(MADE + 60)).is_some());
    let mut more = key(0x11);
    more.user_ids.push(Text::of("Second <second@example.com>"));
    assert!(with(more.certificate(MADE)).is_none());
    let mut other = key(0x11);
    other.expiry = 63_072_000;
    assert!(with(other.certificate(MADE)).is_none());
}

#[test]
fn the_date_revoked_is_an_rfc_3339_time_in_utc() {
    let k = key(0x11);
    let read = |t: &str| {
        let mut g = entry(&k);
        g.revoked = Some(Text::of(t));
        record_of(&g).and_then(|r| {
            r.field(f::REVOKED)
                .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        })
    };
    assert_eq!(read("2026-10-28T14:02:00Z"), Some(1_793_196_120));
    assert_eq!(read("2024-02-29T00:00:00Z"), Some(1_709_164_800));
    for bad in [
        "2026-02-30T00:00:00Z",
        "2026-13-01T00:00:00Z",
        "2026-10-28T24:00:00Z",
        "2026-10-28 14:02:00Z",
        "2026-10-28T14:02:00+00:00",
        "2026-10-28T14:02:00",
        "1969-12-31T23:59:59Z",
        "2200-01-01T00:00:00Z",
    ] {
        assert_eq!(read(bad), None, "{bad}");
    }
}

#[test]
fn a_record_with_no_seeds_makes_no_entry() {
    let mut r = Record::new(kind::GPG);
    r.push(f::CREATED, &MADE.to_le_bytes());
    assert!(gpg_key_of(&r, MADE).is_none());
    assert!(gpg_key_of(&Record::new(kind::NOTE), MADE).is_none());
}
