//! Which medium the device started from, as init records it in
//! `/etc/faraday/exchange` (`docs/STICK-IDENTITY.md` §5.2), and as
//! `faraday-grant`, the disk process and the stick shell read it.
//!
//! The image writes the file. On the Pi it names the board's own card's
//! data partition, `/dev/mmcblk0p2`, which is always that node; at start
//! `faraday-grant --find-boot` writes in its place the node and the
//! sequence number the kernel gave the card's disk (`/dev/mmcblk0p2 3`),
//! so a card put into the slot later is another disk. A record with a
//! node and no number leaves no disk out of what counts as attached. On a PC
//! it says `layout`: at start `faraday-grant --find-boot` looks for the
//! stick, and writes in its place the data partition's node and the
//! sequence number the kernel gave its disk (`/dev/sdb2 7`), so a stick
//! put in later under the same node is another disk; or `none` or `two`,
//! when no stick of this build, or more than one, was found. Until then
//! nothing is handed out (fail closed), and a file that says anything
//! else reads as not yet found.

/// What the image writes on a medium found by its layout.
pub const LAYOUT: &str = "layout";
/// No stick of this build was found.
pub const NONE: &str = "none";
/// More than one stick of this build was found.
pub const TWO: &str = "two";

/// What the record says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Record {
    /// Not found yet, or the record cannot be read: nothing is handed out.
    Waiting,
    /// The data partition of the medium the machine started from.
    Found {
        /// Its node's name under `/dev`: `sdb2`, `mmcblk0p2`.
        partition: String,
        /// The kernel's sequence number for its disk, where the node is
        /// not fixed; `None` where it could not be read, or in the
        /// image's own record on a Pi before start has added it.
        diskseq: Option<String>,
    },
    /// No USB disk with a Faraday stick's layout holds this build.
    NotFound,
    /// More than one does: neither data partition is mounted (§11).
    Two,
}

impl Record {
    /// Reads the record's text. A lookup.
    #[must_use]
    pub fn parse(text: &str) -> Record {
        let mut words = text.split_whitespace();
        let (Some(first), rest) = (words.next(), words.next()) else {
            return Record::Waiting;
        };
        if words.next().is_some() {
            return Record::Waiting;
        }
        match (first, rest) {
            (NONE, None) => Record::NotFound,
            (TWO, None) => Record::Two,
            (node, seq) => match node.strip_prefix("/dev/") {
                Some(p) if !p.is_empty() && !p.contains('/') && p.is_ascii() => {
                    let seq =
                        seq.filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()));
                    if rest.is_some() && seq.is_none() {
                        return Record::Waiting;
                    }
                    Record::Found {
                        partition: p.to_string(),
                        diskseq: seq.map(str::to_string),
                    }
                }
                _ => Record::Waiting,
            },
        }
    }

    /// The Pi's own card as start finds it: the image's record `baked`
    /// (a node, no number) with the sequence number `diskseq` the kernel
    /// gave the card's disk. Without a readable number it is the node
    /// alone, which fails closed both ways: the grant helper and the disk
    /// process keep the disk of that name back ([`Record::is_boot_disk`]),
    /// and the attached check leaves no disk out, so the card counts as
    /// attached. Anything but a node of the card's own is
    /// [`Record::Waiting`].
    #[must_use]
    pub fn card(baked: &str, diskseq: &str) -> Record {
        match Record::parse(baked) {
            Record::Found {
                partition,
                diskseq: None,
            } if partition.starts_with("mmcblk") => {
                let seq = diskseq.trim();
                let ok = !seq.is_empty() && seq.bytes().all(|b| b.is_ascii_digit());
                Record::Found {
                    partition,
                    diskseq: ok.then(|| seq.to_string()),
                }
            }
            _ => Record::Waiting,
        }
    }

    /// The record's text, as [`Record::parse`] reads it back; nothing for
    /// [`Record::Waiting`], which is never written.
    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Record::Waiting => String::new(),
            Record::Found {
                partition,
                diskseq: Some(seq),
            } => format!("/dev/{partition} {seq}\n"),
            Record::Found {
                partition,
                diskseq: None,
            } => format!("/dev/{partition}\n"),
            Record::NotFound => format!("{NONE}\n"),
            Record::Two => format!("{TWO}\n"),
        }
    }

    /// Whether a disk is the one the machine started from: the disk that
    /// holds `partition`, by its node's name and, where the record has
    /// one, its sequence number. `children` are the names of the disk's
    /// partitions.
    #[must_use]
    pub fn is_boot_disk<'a>(
        &self,
        children: impl IntoIterator<Item = &'a str>,
        diskseq: &str,
    ) -> bool {
        let Record::Found {
            partition,
            diskseq: want,
        } = self
        else {
            return false;
        };
        want.as_deref().is_none_or(|w| w == diskseq.trim())
            && children.into_iter().any(|c| c == partition)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pis_card_is_recorded_with_its_number() {
        let r = Record::card("/dev/mmcblk0p2\n", "5\n");
        assert_eq!(r.text(), "/dev/mmcblk0p2 5\n");
        assert!(r.is_boot_disk(["mmcblk0p2"], "5"));
        assert!(!r.is_boot_disk(["mmcblk0p2"], "6"));
    }

    #[test]
    fn a_card_whose_number_cannot_be_read_is_recorded_without_one() {
        assert_eq!(
            Record::card("/dev/mmcblk0p2\n", "").text(),
            "/dev/mmcblk0p2\n"
        );
        assert_eq!(
            Record::card("/dev/mmcblk0p2\n", "x").text(),
            "/dev/mmcblk0p2\n"
        );
    }

    #[test]
    fn only_the_pis_own_card_is_recorded_this_way() {
        assert_eq!(Record::card("layout\n", "5"), Record::Waiting);
        assert_eq!(Record::card("/dev/sdb2 7\n", "5"), Record::Waiting);
    }
}
