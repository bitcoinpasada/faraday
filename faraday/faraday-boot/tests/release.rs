//! A stick's version read out of its boot partition's bytes, as Faraday
//! Starter reads it: a PC stick's bzImage carrying the release string in
//! its setup code, a Pi card's build file, and a stick made before
//! Faraday put a version there.

use faraday_boot::{MAX_RELEASE, find_release};

const RELEASE: &str = "6.6.84-faraday-0.2.0+4d0680b1a2b3";
const MIB: usize = 1 << 20;

/// Bytes that are not text: no byte has its high bit clear, so no
/// release string turns up by chance.
fn noise(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|i| (i as u32).wrapping_mul(2_654_435_761).to_le_bytes()[3] ^ seed | 0x80)
        .collect()
}

/// A PC stick's boot partition: the bzImage's kernel_version string,
/// `6.6.84… (user@host) #1 …`, after a FAT and a PE header's worth of
/// other bytes.
fn stick_boot(release: &str) -> Vec<u8> {
    let mut b = noise(2 * MIB, 1);
    let at = 300_000;
    let text = format!("{release} (user@host) #1 SMP PREEMPT_DYNAMIC\0");
    b[at - 1] = 0;
    b[at..at + text.len()].copy_from_slice(text.as_bytes());
    b
}

/// A Pi card's boot partition: a compressed kernel and the build file
/// (`faraday-build.txt`: the release string and a newline).
fn pi_boot(release: &str) -> Vec<u8> {
    let mut b = noise(2 * MIB, 2);
    let at = MIB / 3;
    let text = format!("{release}\n");
    b[at..at + text.len()].copy_from_slice(text.as_bytes());
    b
}

#[test]
fn a_stick_s_kernel_names_its_faraday() {
    assert_eq!(find_release(&stick_boot(RELEASE)).as_deref(), Some(RELEASE));
}

#[test]
fn a_pi_card_s_build_file_names_its_faraday() {
    assert_eq!(find_release(&pi_boot(RELEASE)).as_deref(), Some(RELEASE));
}

#[test]
fn a_stick_made_before_faraday_named_its_version_has_none() {
    assert_eq!(find_release(&noise(2 * MIB, 3)), None);
    // A kernel of another project names no Faraday version.
    assert_eq!(find_release(&stick_boot("6.6.84-buildroot")), None);
}

#[test]
fn only_a_whole_release_string_is_taken() {
    // Cut off at the end of the bytes: not known to be whole.
    let cut = format!("\0{RELEASE}");
    assert_eq!(find_release(cut.as_bytes()), None);
    // Longer than the kernel allows.
    let long = format!("\0{RELEASE}{}\0", "a".repeat(MAX_RELEASE));
    assert_eq!(find_release(long.as_bytes()), None);
    // Not starting with a digit.
    assert_eq!(find_release(b"\0x-faraday-0.2.0\0"), None);
    // The first whole one, past one that is not.
    let two = format!("\0x-faraday-0.1.0\0{RELEASE}\0");
    assert_eq!(find_release(two.as_bytes()).as_deref(), Some(RELEASE));
}
