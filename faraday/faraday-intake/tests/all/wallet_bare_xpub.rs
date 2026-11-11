//! A wallet whose key is a bare extended public key, with no origin, is
//! refused: nothing says which seed or path it is.

use faraday_intake::wallet::read_wallet;

#[test]
fn a_single_key_descriptor_with_a_bare_xpub_is_refused() {
    let c = crate::common::compute();
    let xpub = "xpub661MyMwAqRbcFtXgS5sYJABqqG9YLmC4Q1Rdap9gSE8NqtwybGhePY2gZ29ESFjqJoCu1Rupje8YtGqsefD265TMg7usUDFdp6W1EGMcet8";
    let read = read_wallet(&c, &format!("wpkh({xpub}/<0;1>/*)"));
    assert!(read.is_err(), "read as a wallet");
}
