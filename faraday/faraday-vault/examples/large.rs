//! Seals and opens one large vault and prints what it took, for measuring
//! how a vault near the ceiling behaves (`docs/VAULT.md` §8, §9).
//!
//! ```text
//! cargo run --release --locked -p faraday-vault --example large -- \
//!     [--attach-mib N] [--xml-mib N] [--argon-mib N]
//! cargo run --release --locked -p faraday-vault --example large -- \
//!     --dir DIR [--attach-mib N] [--entries-mib N] --phase build
//! cargo run --release --locked -p faraday-vault --example large -- \
//!     --dir DIR --phase open|read|seal
//! ```
//!
//! The second form is the memory measurement: `build` writes a file of
//! real-shaped entries (about 530 bytes of XML each, `--entries-mib`) and
//! one attachment into DIR, and each later phase, run as a process of its
//! own, prints the peak live bytes and the peak resident set of that step
//! alone: `open` (decrypt), `read` (`Document::read`) and `seal`
//! (`Document::write` and the seal); `fopen` and `fseal` do the same through
//! the vault layer (`file::open`, and `file::seal` of the opened vault).
//!
//! `--attach-mib` puts N MiB in one attachment, `--xml-mib` N MiB of notes
//! in the document (40,000 bytes each), and `--argon-mib` sets Argon2id's memory (default 8). The file is
//! sealed at the smallest size that holds it, then opened; the sizes and
//! the seconds of each are printed. The contents are filler and a throwaway
//! passphrase: this is a measuring tool, not a vector.
#![allow(clippy::disallowed_methods)]

use std::time::Instant;

use faraday_crypto::work::Compute;
use faraday_vault::Cost;
use faraday_vault::file::{STEP, file_len};
use faraday_vault::kdbx::items::{self, Body, Place};
use faraday_vault::kdbx::xml::{Document, Entry};
use faraday_vault::kdbx::{self, Binary, Contents, RANDOM_LEN};
use faraday_vault::records::{Record, field, kind};

fn status(field: &str) -> usize {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    s.lines()
        .find_map(|l| l.strip_prefix(field))
        .and_then(|v| {
            v.trim_start_matches(':')
                .trim()
                .split(' ')
                .next()?
                .parse()
                .ok()
        })
        .unwrap_or(0)
}

/// Runs `f`, printing the resident memory (MiB) before it and its peak.
fn measured<T>(what: &str, held: &str, f: impl FnOnce() -> T) -> T {
    let _ = std::fs::write("/proc/self/clear_refs", "5");
    let live = status("VmRSS") << 10;
    let started = Instant::now();
    let out = f();
    println!(
        "{what}: holding {held}: RSS at start {} MiB, peak RSS {} MiB, {:.1} s",
        live >> 20,
        status("VmHWM") >> 10,
        started.elapsed().as_secs_f64()
    );
    out
}

/// `n` bytes or so of entries as a KeePass app writes them, each element on
/// its own line (about 530 bytes an entry).
fn real_entries(bytes: usize) -> Document {
    let mut doc = Document::new([1; 16], "Large");
    for i in 0..bytes / 530 {
        let mut uuid = [0u8; 16];
        uuid[..8].copy_from_slice(&(i as u64 + 1).to_le_bytes());
        let mut e = Entry::new(uuid);
        for (k, v, p) in [
            ("Title", "A bank", false),
            ("UserName", "someone", false),
            ("Password", "correct horse battery", true),
            ("URL", "https://example.org/login", false),
            ("Notes", "", false),
        ] {
            e.set_field(k, v, p).expect("sets");
        }
        e.set_modified(1_700_000_000).expect("sets");
        doc.root_mut().push_entry(e).expect("pushes");
    }
    doc
}

#[expect(
    clippy::too_many_lines,
    reason = "one arm for each phase of a measuring tool"
)]
fn phases(args: &[String], dir: &str, phase: &str) {
    #[expect(
        clippy::disallowed_methods,
        reason = "a mint: a measuring tool that draws nothing (docs/COMPUTE.md §3)"
    )]
    let c = Compute::new(|_| {});
    let cost = Cost {
        memory_kib: 8 * 1024,
        passes: 1,
        lanes: 1,
    };
    let random: [u8; RANDOM_LEN] = std::array::from_fn(|i| (i as u8).wrapping_mul(7));
    let key = kdbx::inner_key_of(&random);
    let path = |n: &str| format!("{dir}/v.{n}");
    let attach = mib(args, "--attach-mib", 0) << 20;
    let binaries = |len: usize| {
        if len == 0 {
            vec![]
        } else {
            vec![Binary {
                protected: false,
                data: vec![0u8; len].into(),
            }]
        }
    };
    let seal = |document: &[u8], binaries: &[Binary]| {
        let size = (document.len() + attach + 4096).div_ceil(STEP) * STEP;
        kdbx::seal(
            &c,
            PASSPHRASE,
            cost,
            &random,
            &Contents {
                document,
                binaries: kdbx::Binaries::all(binaries),
                public_custom_data: None,
            },
            size,
        )
        .expect("seals")
    };
    match phase {
        "build" => {
            let doc = real_entries(mib(args, "--entries-mib", 0) << 20);
            let text = String::from_utf8(doc.write(key).to_vec()).expect("text");
            let mut text = text.replace("><", ">\n\t\t<");
            // The attachment is referred to by the first entry, else a seal
            // leaves it out as unused.
            if attach > 0 {
                let tag = text
                    .find("<AutoType")
                    .or_else(|| text.find("</Entry>"))
                    .expect("an entry to refer from (--entries-mib 1 or more)");
                text.insert_str(tag, "<Binary><Key>a</Key><Value Ref=\"0\"/></Binary>");
            }
            let text = text.into_bytes();
            drop(doc);
            let file = seal(&text, &binaries(attach));
            std::fs::write(path("xml"), &text).expect("writes");
            std::fs::write(path("kdbx"), &file).expect("writes");
            println!(
                "built: document {} bytes, file {} bytes",
                text.len(),
                file.len()
            );
        }
        "open" => {
            let file = std::fs::read(path("kdbx")).expect("reads");
            let opened = measured("open", "the file", || {
                kdbx::open(&c, &file, PASSPHRASE).expect("opens")
            });
            println!(
                "document {} bytes, {} attachments",
                opened.document.len(),
                opened.binaries.len()
            );
        }
        "read" => {
            let xml = std::fs::read(path("xml")).expect("reads");
            let doc = measured("read", "the document text", || {
                Document::read(&xml, key).expect("reads")
            });
            println!(
                "charged {} MiB for {} MiB",
                doc.charged() >> 20,
                xml.len() >> 20
            );
        }
        "seal" => {
            let xml = std::fs::read(path("xml")).expect("reads");
            let doc = Document::read(&xml, key).expect("reads");
            drop(xml);
            let bins = binaries(attach);
            let file = measured("seal", "the document model and the attachment", || {
                let text = doc.write(key);
                seal(&text, &bins)
            });
            println!("file {} bytes", file.len());
        }
        // The vault layer as the app runs it: `file::open` of the file (the
        // document, its items and the attachments), then a seal of it with no
        // change, which applies the changes to a copy, writes, reads back,
        // seals and reads the file back.
        "fopen" => {
            let file = std::fs::read(path("kdbx")).expect("reads");
            let opened = measured("fopen", "the file", || {
                faraday_vault::file::open(&c, &file, PASSPHRASE).expect("opens")
            });
            println!("opened {} bytes", opened.facts().len);
        }
        "fseal" => {
            let file = std::fs::read(path("kdbx")).expect("reads");
            let opened = faraday_vault::file::open(&c, &file, PASSPHRASE).expect("opens");
            drop(file);
            let out = measured("fseal", "the opened vault", || {
                faraday_vault::file::seal(&c, &opened, &[], &random, None).expect("seals")
            });
            println!("file {} bytes", out.len());
        }
        other => panic!("unknown phase {other}"),
    }
}

const PASSPHRASE: &str = "large vault, measured";
const NOTE_BYTES: usize = 40_000;

fn mib(args: &[String], name: &str, default: usize) -> usize {
    match args.iter().position(|a| a == name) {
        Some(i) => args
            .get(i + 1)
            .and_then(|v| v.parse().ok())
            .unwrap_or_else(|| panic!("{name} takes a whole number of MiB")),
        None => default,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(i) = args.iter().position(|a| a == "--phase") {
        let dir = args
            .iter()
            .position(|a| a == "--dir")
            .and_then(|d| args.get(d + 1))
            .expect("--dir DIR");
        phases(&args, dir, &args[i + 1]);
        return;
    }
    let attach = mib(&args, "--attach-mib", 0);
    let xml = mib(&args, "--xml-mib", 0);
    let argon = mib(&args, "--argon-mib", 8);
    #[expect(
        clippy::disallowed_methods,
        reason = "a mint: a measuring tool that draws nothing (docs/COMPUTE.md §3)"
    )]
    let c = Compute::new(|_| {});
    let cost = Cost {
        memory_kib: u32::try_from(argon * 1024).expect("memory fits"),
        passes: 1,
        lanes: 1,
    };
    let random: [u8; RANDOM_LEN] = std::array::from_fn(|i| (i as u8).wrapping_mul(7));

    let mut doc = Document::new([1; 16], "Large");
    items::stamp(&mut doc, 1, Some("2023-11-14T22:13:20Z")).expect("stamps");
    let text = "x".repeat(NOTE_BYTES);
    for n in 0..(xml << 20) / NOTE_BYTES {
        let body = Body::Record(Record::new(kind::NOTE).with(field::NOTE, text.as_bytes()));
        let at = Place {
            uuid: [
                (n % 256) as u8,
                (n / 256) as u8,
                1,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            group: "Notes",
            group_uuid: [9; 16],
            title: "Note",
            now: Some(1_700_000_000),
        };
        items::put(&c, &mut doc, &at, &body).expect("puts");
    }
    let document = doc.write(kdbx::inner_key_of(&random));
    let binaries = if attach == 0 {
        vec![]
    } else {
        vec![Binary {
            protected: false,
            data: vec![0u8; attach << 20].into(),
        }]
    };
    let plain = document.len() + (attach << 20) + 4096;
    let size = plain.div_ceil(STEP) * STEP;

    let started = Instant::now();
    let file = kdbx::seal(
        &c,
        PASSPHRASE,
        cost,
        &random,
        &Contents {
            document: &document,
            binaries: kdbx::Binaries::all(&binaries),
            public_custom_data: None,
        },
        size,
    )
    .expect("seals");
    println!(
        "sealed {} bytes (file_len of {size} is {}) in {:.1} s",
        file.len(),
        file_len(size),
        started.elapsed().as_secs_f64()
    );
    let started = Instant::now();
    let opened = kdbx::open(&c, &file, PASSPHRASE).expect("opens");
    println!(
        "opened: document {} bytes, {} attachments, in {:.1} s",
        opened.document.len(),
        opened.binaries.len(),
        started.elapsed().as_secs_f64()
    );
}
