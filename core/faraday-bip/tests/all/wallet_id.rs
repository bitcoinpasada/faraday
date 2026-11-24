//! A wallet is known by `WalletId`, a hash of its whole descriptor, not by
//! its 8-character checksum: the checksum is 40 bits, and two different
//! wallets can share one.
//!
//! The descriptors below share one checksum (public test keys; found
//! once by solving the checksum's linear form over free digits of one
//! cosigner's origin). `TWIN` changes the threshold and that origin;
//! `ALIAS` changes only that origin.

use faraday_bip::policy::{WalletId, WalletPolicy};
use faraday_crypto::work::Compute;

const FIRST: &str = concat!(
    "wsh(sortedmulti(2,[9a6a2580/48h/1h/0h/2h]tpubDFZk17LmJpm9HZWfxoggu7yC1KeUm26JA2gCHnCXdo9oV",
    "PnpGZb9ErVXmiRiDMBgTiQ9oNdC3TDE4PvVy7eMvubhZqkcrPyCkiZvzWBVqrr/<0;1>/*,[5d388376/48h/1h/0h",
    "/2h]tpubDFcsPThm5auRC835VVQsJ4U3cwqobwd2NUsMBHh7ngRtyRVFtKcWrWMUK5jQ6nFmUVfC5aLckrGaPSkRkx",
    "DYZL2Qxf3weKxbYb4uSvdc5fb/<0;1>/*,[cf2e083d/48h/1h/0h/2h]tpubDFUzvqxLm1V5MSw6HUgSCEWVUVfzF",
    "UtPD9hbnCg1uWKdUX9PqYpBqLvcBgDCWfDWZKUMQeczGQyccANPFYyK9nhTcszQGCZrKXDzVacHUzh/<0;1>/*))#6",
    "kjg6089",
);

const TWIN: &str = concat!(
    "wsh(sortedmulti(1,[9a6a2580/48h/1h/0h/2h]tpubDFZk17LmJpm9HZWfxoggu7yC1KeUm26JA2gCHnCXdo9oV",
    "PnpGZb9ErVXmiRiDMBgTiQ9oNdC3TDE4PvVy7eMvubhZqkcrPyCkiZvzWBVqrr/<0;1>/*,[5d388376/48h/1h/0h",
    "/2h]tpubDFcsPThm5auRC835VVQsJ4U3cwqobwd2NUsMBHh7ngRtyRVFtKcWrWMUK5jQ6nFmUVfC5aLckrGaPSkRkx",
    "DYZL2Qxf3weKxbYb4uSvdc5fb/<0;1>/*,[05764152/48h/1h/107727000h/2h]tpubDFUzvqxLm1V5MSw6HUgSC",
    "EWVUVfzFUtPD9hbnCg1uWKdUX9PqYpBqLvcBgDCWfDWZKUMQeczGQyccANPFYyK9nhTcszQGCZrKXDzVacHUzh/<0;",
    "1>/*))#6kjg6089",
);

const ALIAS: &str = concat!(
    "wsh(sortedmulti(2,[9a6a2580/48h/1h/0h/2h]tpubDFZk17LmJpm9HZWfxoggu7yC1KeUm26JA2gCHnCXdo9oV",
    "PnpGZb9ErVXmiRiDMBgTiQ9oNdC3TDE4PvVy7eMvubhZqkcrPyCkiZvzWBVqrr/<0;1>/*,[5d388376/48h/1h/0h",
    "/2h]tpubDFcsPThm5auRC835VVQsJ4U3cwqobwd2NUsMBHh7ngRtyRVFtKcWrWMUK5jQ6nFmUVfC5aLckrGaPSkRkx",
    "DYZL2Qxf3weKxbYb4uSvdc5fb/<0;1>/*,[75424404/48h/1h/141234000h/2h]tpubDFUzvqxLm1V5MSw6HUgSC",
    "EWVUVfzFUtPD9hbnCg1uWKdUX9PqYpBqLvcBgDCWfDWZKUMQeczGQyccANPFYyK9nhTcszQGCZrKXDzVacHUzh/<0;",
    "1>/*))#6kjg6089",
);

fn read(text: &str) -> WalletPolicy {
    WalletPolicy::parse_any(&Compute::unchecked(), text).expect("the descriptor reads")
}

#[test]
fn wallets_of_one_checksum_have_two_ids() {
    let (first, twin, alias) = (read(FIRST), read(TWIN), read(ALIAS));
    assert_eq!(first.checksum(), "6kjg6089");
    assert_eq!(twin.checksum(), first.checksum());
    assert_eq!(alias.checksum(), first.checksum());
    assert_ne!(first.id(), twin.id());
    assert_ne!(first.id(), alias.id());
    assert_ne!(twin.id(), alias.id());
}

#[test]
fn a_wallet_has_one_id_however_it_is_read() {
    let bare = FIRST.split_once('#').unwrap().0;
    assert_eq!(read(bare).id(), read(FIRST).id());
    let first = read(FIRST);
    assert_eq!(read(&first.to_descriptor_checksummed()).id(), first.id());
    assert_eq!(first.clone().id(), first.id());
}

#[test]
fn an_id_is_kept_as_64_hex_and_nothing_else_names_one() {
    let id = read(FIRST).id();
    let hex = id.to_hex();
    assert_eq!(hex.len(), 64);
    assert_eq!(WalletId::from_hex(&hex), Some(id));
    assert_eq!(WalletId::from_hex(&hex.to_uppercase()), None);
    assert_eq!(WalletId::from_hex("6kjg6089"), None);
    assert_eq!(WalletId::from_hex(&hex[..63]), None);
    assert_eq!(WalletId::from_hex(&format!("{}0", hex.as_str())), None);
}
