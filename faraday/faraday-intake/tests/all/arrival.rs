//! What a whole transfer becomes when it is kept: a name when it brought
//! one, an extension, and the bytes.

use crate::common::interop;
use faraday_intake::arrival;
use faraday_qr::Arrived;

#[test]
fn a_psbt_that_arrived_is_kept_as_a_psbt_file_with_its_bytes() {
    let psbt = interop("bluewallet/quicklook-preview-sample.psbt").into_bytes();
    assert_eq!(arrival(Arrived::Psbt(psbt.clone())), (None, "psbt", psbt));
}

#[test]
fn a_finished_transaction_is_kept_as_the_hex_a_wallet_pastes() {
    // The sample's hex, as bytes in, the same hex out.
    let hex = interop("bluewallet/quicklook-preview-sample.txn");
    let hex = hex.trim();
    let raw = crate::common::unhex(hex);
    let (name, ext, data) = arrival(Arrived::Transaction(raw));
    assert_eq!(name, None);
    assert_eq!(ext, "txn");
    assert_eq!(data, hex.as_bytes());
}

#[test]
fn a_file_that_arrived_keeps_its_own_name_and_bytes() {
    let (name, ext, data) = arrival(Arrived::File(
        "wallet.json".to_string(),
        b"{}".to_vec(),
        "wallet".to_string(),
    ));
    assert_eq!(name.as_deref(), Some("wallet.json"));
    assert_eq!(ext, "");
    assert_eq!(data, b"{}");
}

#[test]
fn keys_a_descriptor_and_text_that_arrived_are_kept_as_text_files() {
    let descriptor = interop("bluewallet/coldcardmk4-descriptor.txt");
    for arrived in [
        Arrived::Descriptor(descriptor.clone()),
        Arrived::Keys(descriptor.clone()),
        Arrived::Text(descriptor.clone()),
    ] {
        assert_eq!(
            arrival(arrived),
            (None, "txt", descriptor.clone().into_bytes())
        );
    }
}
