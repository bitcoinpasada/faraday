//! Which disks are Faraday sticks (`docs/STICK-IDENTITY.md` §5.1), on the
//! fixtures `faraday-grant`, the Starter and `stick-boot.sh` read too.

use faraday_boot::layout::{Span, is_stick, last_usable};

/// A 256 MB stick, in sectors.
const DISK: u64 = 524_288;

fn spans(parts: &[(u64, u64)]) -> Vec<Span> {
    parts
        .iter()
        .map(|&(start, size)| Span { start, size })
        .collect()
}

#[test]
fn a_stick_written_with_dd_and_one_made_for_its_size_are_both_known() {
    let end = last_usable(DISK);
    assert!(is_stick(
        &spans(&[(2048, 131_072), (133_120, 262_144)]),
        end
    ));
    assert!(is_stick(
        &spans(&[(2048, 131_072), (133_120, end - 133_120 + 1)]),
        end
    ));
}

#[test]
fn another_layout_is_not_a_stick() {
    let end = last_usable(DISK);
    let to_the_end = end - 133_120 + 1;
    for (name, parts) in [
        (
            "data under 128 MiB",
            vec![(2048, 131_072), (133_120, 262_143)],
        ),
        (
            "data starting elsewhere",
            vec![(2048, 131_072), (135_168, 262_144)],
        ),
        (
            "data past the last usable sector",
            vec![(2048, 131_072), (133_120, to_the_end + 1)],
        ),
        (
            "a third partition",
            vec![(2048, 131_072), (133_120, 262_144), (400_000, 2048)],
        ),
        (
            "a boot partition of another size",
            vec![(2048, 133_120), (135_168, 262_144)],
        ),
        ("one partition", vec![(2048, 131_072)]),
    ] {
        assert!(!is_stick(&spans(&parts), end), "{name}");
    }
}

#[test]
fn a_disk_whose_size_is_not_known_is_not_a_stick() {
    assert!(!is_stick(
        &spans(&[(2048, 131_072), (133_120, 262_144)]),
        last_usable(0)
    ));
}
