//! Tests of the spend's partial-signature tally.

use super::*;

fn fp(n: u8) -> Fingerprint {
    Fingerprint([n, 0, 0, 0])
}

#[test]
fn a_master_with_two_accounts_counts_only_when_both_do() {
    // The same master twice, as two participants of one wallet; the
    // first signed here, the second not yet.
    let first_only = Tally::of(&[
        (fp(1), Partial::Here),
        (fp(1), Partial::Open),
        (fp(2), Partial::Open),
    ]);
    assert!(first_only.counted.is_empty());
    assert_eq!(first_only.to_sign, vec![fp(1), fp(2)]);
    let both = Tally::of(&[
        (fp(1), Partial::Here),
        (fp(1), Partial::Here),
        (fp(2), Partial::Word),
    ]);
    assert_eq!(both.counted, vec![fp(1)]);
    assert_eq!(both.in_file, vec![fp(2)]);
    assert!(both.to_sign.is_empty());
}
