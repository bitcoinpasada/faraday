//! The leak scan: it finds a planted secret in a live block, and passes a run without one.

use faraday_leak_scan::Leak;

#[global_allocator]
static LEAK: Leak = Leak::new();

const SECRET: &[u8] = b"leak scan planted secret 7f3a9c41";

/// Invariant: The scan finds a planted secret in a live block of an armed thread, and finds nothing once the block is freed or when nothing was planted.
/// Regression: The scan reports a freed block as live, or misses a live one.
/// Owner: app
#[test]
fn a_planted_secret_is_found_while_live_and_not_after() {
    Leak::arm();
    Leak::watch(SECRET);
    assert!(!Leak::live(), "nothing is planted yet");
    let planted = SECRET.to_vec();
    assert!(Leak::live(), "a live copy is found");
    drop(planted);
    assert!(!Leak::live(), "a freed copy is not a live block");
}

/// Invariant: `drop_checked` reports a form in the blocks the one value it drops freed, and in no other.
/// Regression: `drop_checked` reports a form in a block the dropped value did not own.
/// Owner: app
#[test]
fn a_drop_is_checked_for_its_own_blocks_only() {
    Leak::watch(SECRET);
    let other = SECRET.to_vec();
    assert!(!Leak::drop_checked(vec![1u8, 2, 3]));
    assert!(Leak::drop_checked(SECRET.to_vec()));
    drop(other);
}
