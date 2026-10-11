//! Secure Boot keys Faraday makes (`PLAN.md` §8): an owner's PK, KEK and db
//! as RSA-2048 keys with self-signed certificates, the files a computer's
//! firmware setup enrols them from, Authenticode signatures over
//! `BOOTX64.EFI`, and checking one against the db certificate.
//!
//! The private keys live in a vault (`docs/VAULT.md` §7, type 8); only
//! certificates, signature lists, signed updates and signed images reach
//! the Outbox. Faraday never writes an EFI variable: enrolment happens in
//! the firmware's own setup screen.
//!
//! `no_std` + `alloc`.

#![no_std]

extern crate alloc;

pub mod der;
pub mod efi;
pub mod pe;
pub mod rng;
pub mod x509;

use alloc::string::String;
use alloc::vec::Vec;

pub use pe::Check;
pub use x509::Pair;

/// Why something could not be made or read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// A key could not be made, read or used.
    Key,
    /// Not a PE/COFF image this crate reads.
    Image,
}

impl Error {
    /// The reason in a few words.
    pub fn reason(self) -> &'static str {
        match self {
            Error::Key => "The key could not be made or read",
            Error::Image => "Not an EFI image this build reads",
        }
    }
}

/// An owner's three keys and the GUID they are listed under.
pub struct Keys {
    /// The owner GUID, as UEFI stores it.
    pub owner: [u8; 16],
    /// The platform key.
    pub pk: Pair,
    /// The key exchange key.
    pub kek: Pair,
    /// The signature database key, which signs images.
    pub db: Pair,
}

/// Years the certificates are valid for.
const YEARS: u32 = 30;

/// Makes the three keys from a seed of the system's entropy, named for
/// `owner` (the common names read "OWNER PK", "OWNER KEK", "OWNER db"),
/// valid from `now`.
pub fn make(seed: &[u8; 32], owner: &str, now: u64) -> Result<Keys, Error> {
    let mut ids = rng::Stream::new(seed, 0);
    let mut draw16 = || {
        let mut v = [0u8; 16];
        let _ = rand_core::TryRng::try_fill_bytes(&mut ids, &mut v);
        v
    };
    let mut guid = draw16();
    // A version-4 GUID.
    guid[7] = (guid[7] & 0x0f) | 0x40;
    guid[8] = (guid[8] & 0x3f) | 0x80;
    let serials = [draw16(), draw16(), draw16()];
    let owner = if owner.trim().is_empty() {
        "Faraday"
    } else {
        owner.trim()
    };
    let name = |what: &str| -> String { alloc::format!("{owner} {what}") };
    let mut keys_rng = rng::Stream::new(seed, 1);
    Ok(Keys {
        owner: guid,
        pk: x509::generate(&mut keys_rng, &name("PK"), &serials[0], now, YEARS)?,
        kek: x509::generate(&mut keys_rng, &name("KEK"), &serials[1], now, YEARS)?,
        db: x509::generate(&mut keys_rng, &name("db"), &serials[2], now, YEARS)?,
    })
}

/// Which certificates enrolment adds beside the owner's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    /// Microsoft's KEK CAs and Windows signing CAs too, so Windows and
    /// its updates still start (`PLAN.md` §8). Not the third-party CA.
    Windows,
    /// The owner's keys alone.
    OwnKeysOnly,
}

/// Which store a Microsoft certificate goes into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Store {
    /// KEK.
    Kek,
    /// db.
    Db,
}

/// One of Microsoft's public certificates, as Microsoft publishes it,
/// with the SHA-256 recorded here when it was taken.
pub struct Microsoft {
    /// Its common name.
    pub name: &'static str,
    /// Where it goes.
    pub store: Store,
    /// The certificate, DER.
    pub der: &'static [u8],
    /// Its SHA-256, hex.
    pub sha256: &'static str,
}

/// The certificates the Windows-compatible policy adds, downloaded from
/// Microsoft's links (go.microsoft.com/fwlink/?LinkId=321185 and 321192,
/// ?linkid=2239775 and 2239776) on 2026-10-05. The two 2011 certificates
/// passed their notAfter date in June 2026; firmware does not check the
/// dates of what db and KEK hold, and Windows still boots through them.
pub const MICROSOFT: [Microsoft; 4] = [
    Microsoft {
        name: "Microsoft Corporation KEK CA 2011",
        store: Store::Kek,
        der: include_bytes!("../certs/kek-ca-2011.der"),
        sha256: "a1117f516a32cefcba3f2d1ace10a87972fd6bbe8fe0d0b996e09e65d802a503",
    },
    Microsoft {
        name: "Microsoft Corporation KEK 2K CA 2023",
        store: Store::Kek,
        der: include_bytes!("../certs/kek-2k-ca-2023.der"),
        sha256: "3cd3f0309edae228767a976dd40d9f4affc4fbd5218f2e8cc3c9dd97e8ac6f9d",
    },
    Microsoft {
        name: "Microsoft Windows Production PCA 2011",
        store: Store::Db,
        der: include_bytes!("../certs/windows-pca-2011.der"),
        sha256: "e8e95f0733a55e8bad7be0a1413ee23c51fcea64b3c8fa6a786935fddcc71961",
    },
    Microsoft {
        name: "Windows UEFI CA 2023",
        store: Store::Db,
        der: include_bytes!("../certs/windows-uefi-ca-2023.der"),
        sha256: "076f1fea90ac29155ebf77c17682f75f1fdd1be196da302dc8461e350a9ae330",
    },
];

/// The enrolment files, by name: each store's signature list (`.esl`),
/// its signed update (`.auth`: PK signed by PK, KEK by PK, db by KEK) and
/// each of the owner's certificates (`.cer`), for firmware that enrols
/// from any of them.
pub fn enrolment(keys: &Keys, policy: Policy, now: u64) -> Result<Vec<(String, Vec<u8>)>, Error> {
    let extra = |store: Store| -> Vec<([u8; 16], &'static [u8])> {
        if policy == Policy::OwnKeysOnly {
            return Vec::new();
        }
        MICROSOFT
            .iter()
            .filter(|m| m.store == store)
            .map(|m| (efi::MICROSOFT_OWNER, m.der))
            .collect()
    };
    let pk = efi::signature_lists(&[(keys.owner, keys.pk.cert.as_slice())]);
    let mut kek_list = alloc::vec![(keys.owner, keys.kek.cert.as_slice())];
    kek_list.extend(extra(Store::Kek));
    let kek = efi::signature_lists(&kek_list);
    let mut db_list = alloc::vec![(keys.owner, keys.db.cert.as_slice())];
    db_list.extend(extra(Store::Db));
    let db = efi::signature_lists(&db_list);
    Ok(alloc::vec![
        (
            "PK.auth".into(),
            efi::auth("PK", &efi::GLOBAL_VARIABLE, &pk, now, &keys.pk)?
        ),
        (
            "KEK.auth".into(),
            efi::auth("KEK", &efi::GLOBAL_VARIABLE, &kek, now, &keys.pk)?
        ),
        (
            "db.auth".into(),
            efi::auth("db", &efi::IMAGE_SECURITY_DATABASE, &db, now, &keys.kek)?
        ),
        ("PK.esl".into(), pk),
        ("KEK.esl".into(), kek),
        ("db.esl".into(), db),
        ("PK.cer".into(), keys.pk.cert.clone()),
        ("KEK.cer".into(), keys.kek.cert.clone()),
        ("db.cer".into(), keys.db.cert.clone()),
    ])
}

/// A certificate's common name.
pub fn common_name(cert: &[u8]) -> Option<String> {
    x509::Cert::read(cert)?.cn
}
