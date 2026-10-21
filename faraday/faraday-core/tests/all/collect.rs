//! What collecting signatures asks for: the count still needed, and
//! "more" only once a signature is in.

#[test]
fn collecting_says_more_only_once_a_signature_is_in() {
    use faraday_core::wallet::collect_line;
    assert_eq!(collect_line(2, 0), "Collect 2 signatures");
    assert_eq!(collect_line(1, 1), "Collect 1 more signature");
    assert_eq!(collect_line(2, 1), "Collect 2 more signatures");
}
