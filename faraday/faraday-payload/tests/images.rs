//! The release's image list `IMAGES` (`docs/IMAGES.md` §4): a good one
//! read, a malformed one refused with its line and why, and what a file's
//! name says about it.

use faraday_payload::{ImageList, ImageListError, ImageListFault, NameClass, sha256_of};

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const SUM_A: &str = "d7e055e6f9f0d8a3c019d1e6b272b793dd607c3929a2a8f922cee3a5eae71c4f";
const BOOT: &str = "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8";
const SUM_B: &str = "c3a69d3c8e5599d5dc08dc6780f76b30a8e4d53082a85d40c6cd2effd28dfb0c";

fn header() -> String {
    format!("# Faraday 0.3.0 images, commit {COMMIT}\n")
}

fn pi_line() -> String {
    format!(
        "faraday-pi-0.3.0.img.xz\t{SUM_A}\t{SUM_B}\tpi3\tany\tPi 3B, Pi 3B+, Pi Zero 2 W\t4096\t0.3.0\t-\n"
    )
}

fn stick_line() -> String {
    format!(
        "faraday-stick-0.3.0-x86_64-uefi.img.xz\t{SUM_B}\t{SUM_A}\tx86_64-uefi\tefi-framebuffer\tPC\t8192\t0.3.0\t{BOOT}\n"
    )
}

fn refused(text: &str) -> ImageListError {
    ImageList::parse(text.as_bytes()).expect_err("refused")
}

#[test]
fn a_release_list_is_read() {
    let list = ImageList::parse(format!("{}{}{}", header(), stick_line(), pi_line()).as_bytes())
        .expect("good");
    assert_eq!((list.version(), list.commit()), ("0.3.0", COMMIT));
    let files: Vec<&str> = list.images().iter().map(|i| i.file.as_str()).collect();
    assert_eq!(
        files,
        [
            "faraday-stick-0.3.0-x86_64-uefi.img.xz",
            "faraday-pi-0.3.0.img.xz"
        ]
    );
    let pi = &list.images()[1];
    assert_eq!((pi.board.as_str(), pi.panel.as_str()), ("pi3", "any"));
    assert_eq!(pi.starts, ["Pi 3B", "Pi 3B+", "Pi Zero 2 W"]);
    assert_eq!(pi.xz_sha256.to_string(), SUM_A);
    assert_eq!(
        (pi.inner_size, pi.build_id.as_str(), pi.boot_sha256),
        (4096, "0.3.0", None)
    );
    let stick = &list.images()[0];
    assert_eq!(stick.inner_size, 8192);
    assert_eq!(
        stick.boot_sha256.map(|s| s.to_string()).as_deref(),
        Some(BOOT)
    );
}

/// A list from before the size, build and boot sum were added is refused
/// at its first image's line.
#[test]
fn a_list_without_the_write_fields_is_refused_with_its_line() {
    let old = format!(
        "{}faraday-pi-0.3.0.img.xz\t{SUM_A}\t{SUM_B}\tpi3\tany\tPi 3B\n",
        header()
    );
    let e = refused(&old);
    assert_eq!((e.line, e.fault), (2, ImageListFault::FieldCount(6)));
    assert_eq!(
        e.to_string(),
        "IMAGES line 2: not nine tab-separated fields"
    );
}

#[test]
fn a_bad_size_build_or_boot_sum_is_refused_on_its_line() {
    let at = |text: String| {
        let e = refused(&format!("{}{}{text}", header(), stick_line()));
        (e.line, e.fault)
    };
    let pi = pi_line();
    assert_eq!(at(pi.replace("4096", "0")), (3, ImageListFault::Size));
    assert_eq!(at(pi.replace("4096", "4k")), (3, ImageListFault::Size));
    assert_eq!(
        at(pi.replace("\t0.3.0\t", "\t\t")),
        (3, ImageListFault::BuildId)
    );
    // A Pi image has no boot sum; a stick image must have one.
    assert_eq!(
        at(pi.replace("\t-\n", &format!("\t{BOOT}\n"))),
        (3, ImageListFault::Sum)
    );
    let e = refused(&format!("{}{}", header(), stick_line().replace(BOOT, "-")));
    assert_eq!((e.line, e.fault), (2, ImageListFault::Sum));
}

#[test]
fn a_malformed_list_says_which_line_and_why() {
    let at = |e: ImageListError| (e.line, e.fault);
    assert_eq!(at(refused("")), (1, ImageListFault::Header));
    assert_eq!(
        at(refused("# Faraday 0.3.0 images\n")),
        (1, ImageListFault::Header)
    );
    assert_eq!(at(refused(&header())), (2, ImageListFault::NoImages));
    assert_eq!(
        at(refused(&format!(
            "{}{}\n{}",
            header(),
            stick_line(),
            pi_line()
        ))),
        (3, ImageListFault::EmptyLine)
    );
    assert_eq!(
        at(refused(&format!(
            "{}{}",
            header(),
            pi_line().replace('\t', " ")
        ))),
        (2, ImageListFault::FieldCount(1))
    );
    assert_eq!(
        at(refused(&format!(
            "{}{}",
            header(),
            pi_line().replace("pi-0.3.0", "pi-0.2.0")
        ))),
        (2, ImageListFault::Name)
    );
    assert_eq!(
        at(refused(&format!(
            "{}{}",
            header(),
            pi_line().replace(".img.xz", ".img")
        ))),
        (2, ImageListFault::Name)
    );
    assert_eq!(
        at(refused(&format!(
            "{}{}{}",
            header(),
            stick_line(),
            pi_line().replace(SUM_A, "abc")
        ))),
        (3, ImageListFault::Sum)
    );
    assert_eq!(
        at(refused(&format!(
            "{}{}",
            header(),
            pi_line().replace("\tany\t", "\t\t")
        ))),
        (2, ImageListFault::Field)
    );
    assert_eq!(
        at(refused(&format!("{}{}{}", header(), pi_line(), pi_line()))),
        (3, ImageListFault::Duplicate)
    );
    assert_eq!(
        refused(&format!(
            "{}{}",
            header(),
            pi_line().replace("faraday-pi", "../pi")
        ))
        .fault,
        ImageListFault::Name
    );
    assert_eq!(
        refused("").to_string(),
        "IMAGES line 1: not an image list header"
    );
}

#[test]
fn a_name_is_listed_of_another_release_or_unrelated() {
    let list = ImageList::parse(format!("{}{}{}", header(), stick_line(), pi_line()).as_bytes())
        .expect("good");
    assert!(
        matches!(list.classify("faraday-pi-0.3.0.img.xz"), NameClass::Listed(i) if i.board == "pi3")
    );
    assert!(matches!(
        list.classify("faraday-pi-0.2.1.img.xz"),
        NameClass::OtherRelease { found: "0.2.1", .. }
    ));
    assert!(matches!(
        list.classify("faraday-stick-0.3.1-rc1-x86_64-uefi.img.xz"),
        NameClass::OtherRelease {
            found: "0.3.1-rc1",
            ..
        }
    ));
    assert!(matches!(
        list.classify("faraday-pi-0.2.1-waveshare-28dpi.img.xz"),
        NameClass::OtherRelease {
            found: "0.2.1-waveshare-28dpi",
            ..
        }
    ));
    for other in [
        "faraday-pi-0.3.0.img",
        "faraday-pi-.img.xz",
        "faraday-pi-latest.img.xz",
        "notes.txt",
        "faraday-pi-0.2.1.img.xz.part",
    ] {
        assert_eq!(list.classify(other), NameClass::Unrelated, "{other}");
    }
}

#[test]
fn a_sum_is_what_sha256sum_gives() {
    assert_eq!(
        sha256_of(&b"abc"[..]).expect("reads").to_string(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
