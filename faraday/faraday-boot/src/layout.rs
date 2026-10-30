//! What makes a disk a Faraday stick or card, and what marks a boot
//! partition (F174, `docs/STICK-IDENTITY.md` §5.1, §5.2; F80 for the
//! sizes).
//!
//! A Faraday stick is known by its layout alone: a GPT disk with exactly
//! two partitions, an EFI system partition at sector 2048 of 64 MiB and a
//! basic data partition at sector 133120 of 128 MiB. A Pi card is an MBR
//! disk with two partitions of those sizes. Names, IDs and labels play no
//! part: they are an ordinary stick's (§3), and a person may name the
//! data partition (§6).
//!
//! A boot partition is kept back from every process by its name or its
//! label, whichever stick it is on: the names and labels an ordinary
//! stick's or card's boot partition has, and `FARADAYBOOT`, which a stick
//! made before F174 still carries (§5.4). Keeping back more than Faraday's
//! own fails closed: Faraday writes no boot partition (F128).

/// The unit `/sys` and a partition table count in.
pub const SECTOR: u64 = 512;

/// The stick's EFI system partition: its start and its size, in sectors.
pub const BOOT_START: u64 = 2048;
/// 64 MiB (F80).
pub const BOOT_SECTORS: u64 = (64 << 20) / SECTOR;
/// The stick's data partition: its start and its size, in sectors.
pub const DATA_START: u64 = 133_120;
/// 128 MiB (F80).
pub const DATA_SECTORS: u64 = (128 << 20) / SECTOR;

/// The EFI system partition's GPT type.
pub const ESP_TYPE: &str = "C12A7328-F81F-11D2-BA4B-00A0C93EC93B";
/// The basic data partition's GPT type.
pub const DATA_TYPE: &str = "EBD0A0A2-B9E5-4433-87C0-68B6B72699C7";

/// One partition as `/sys` or a partition table states it: its start
/// and its size in 512-byte sectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Its first sector.
    pub start: u64,
    /// How many sectors it has.
    pub size: u64,
}

/// Whether a disk's partitions, in order, are a Faraday stick's: exactly
/// two, at the starts and of the sizes of §5.1. The types are the
/// caller's to check where it can read them; `/sys` does not say them.
#[must_use]
pub fn is_stick(parts: &[Span]) -> bool {
    parts
        == [
            Span {
                start: BOOT_START,
                size: BOOT_SECTORS,
            },
            Span {
                start: DATA_START,
                size: DATA_SECTORS,
            },
        ]
}

/// Whether a disk's partitions, in order, are a Pi card's: exactly two,
/// of the stick's sizes, the second after the first. The card's
/// alignment sets where they start.
#[must_use]
pub fn is_card(parts: &[Span]) -> bool {
    matches!(parts, [boot, data]
        if boot.size == BOOT_SECTORS
            && data.size == DATA_SECTORS
            && boot.start > 0
            && data.start >= boot.start + boot.size)
}

/// The GPT names a boot partition is kept back by: Windows' name for an
/// EFI system partition, which a Faraday stick's has (§3), and the name a
/// stick made before F174 has.
pub const BOOT_NAMES: [&str; 2] = ["EFI system partition", "FARADAYBOOT"];

/// The FAT labels a boot partition is kept back by: the stick's (`EFI`),
/// the Pi card's (`bootfs`, Raspberry Pi OS's own), and the label a stick
/// or card made before F174 has.
pub const BOOT_LABELS: [&str; 3] = ["EFI", "bootfs", "FARADAYBOOT"];

/// Whether a GPT partition name is a boot partition's.
#[must_use]
pub fn is_boot_name(name: &str) -> bool {
    BOOT_NAMES.contains(&name)
}

/// Whether a FAT volume label, as a boot sector or a directory entry
/// holds it (padded with spaces) or as a person reads it, is a boot
/// partition's. Case is not looked at, so a label written in either case
/// is kept back.
#[must_use]
pub fn is_boot_label(label: &[u8]) -> bool {
    let end = label
        .iter()
        .rposition(|&b| b != b' ' && b != 0)
        .map_or(0, |i| i + 1);
    let label = &label[..end];
    BOOT_LABELS
        .iter()
        .any(|b| b.as_bytes().eq_ignore_ascii_case(label))
}
