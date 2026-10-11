//! What is left of a key in this process's memory after it has been used
//! and wiped (`docs/PLANNING.md` §12; security review M3).
//!
//! The test loads a key, signs the demo transaction with it, wipes every
//! key from Settings, drops the application, and then reads its own
//! memory through `/proc/self/maps` and `/proc/self/mem` looking for two
//! things that must not survive: the middle of the 64-byte BIP-39 seed,
//! and the middle of the BIP-32 master private key derived from it.
//!
//! Sixteen bytes of each are looked for. That is far more than a
//! coincidence can produce in the few hundred megabytes a test process
//! maps, and short enough that a partial copy still matches.
//!
//! **The needles are stated, not derived.** A test that computed the
//! seed in order to look for it would leave the seed in the memory it
//! then scans, and would find itself. The two values below were computed
//! from the published "abandon … about" mnemonic with an empty
//! passphrase; the master key they belong to is the one whose
//! fingerprint is `73c5da0a`, which `osk_bip::keys` states. The only
//! copy this test makes is the one it decodes them into, whose address
//! is excluded from the count.
//!
//! Only writable anonymous regions, the heap and the stack are read.
//! File-backed mappings are the binary and the libraries, and `[vvar]`
//! and `[vsyscall]` are the kernel's; none of them can hold a secret
//! this process wrote.

#![cfg(target_os = "linux")]

mod common;

use std::fs::File;
use std::os::unix::fs::FileExt;

use common::{ABANDON, Harness, PANEL};
use opensigner_core::ScreenKind;
use opensigner_core::ids;
use opensigner_core::sign::{Stage, Step};
use osk_shell_api::{Event, FileKind};

const DEMO: &[u8] = include_bytes!("../../../tools/vectors/psbt/demo-regtest.psbt");

/// Bytes 24..40 of the 64-byte seed of "abandon … about" with no
/// passphrase.
const SEED_MIDDLE: &str = "811aaed6f6da5fc19a5ac40b389cd370";

/// Bytes 8..24 of that seed's BIP-32 master private key.
const MASTER_KEY_MIDDLE: &str = "11cda2b066151be2cfb48adf9e47b151";

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

/// A writable mapping worth reading.
struct Region {
    start: usize,
    end: usize,
    label: String,
}

/// The process's own writable anonymous and heap mappings.
fn regions() -> Vec<Region> {
    let maps = std::fs::read_to_string("/proc/self/maps").expect("/proc/self/maps");
    let mut out = Vec::new();
    for line in maps.lines() {
        let mut fields = line.split_whitespace();
        let (Some(range), Some(perms)) = (fields.next(), fields.next()) else {
            continue;
        };
        // offset, device, inode, then the pathname if there is one.
        let path = fields.nth(3).unwrap_or("");
        if !perms.starts_with("rw") {
            continue;
        }
        if !(path.is_empty() || path == "[heap]" || path == "[stack]") {
            continue;
        }
        let Some((from, to)) = range.split_once('-') else {
            continue;
        };
        let (Ok(start), Ok(end)) = (
            usize::from_str_radix(from, 16),
            usize::from_str_radix(to, 16),
        ) else {
            continue;
        };
        let label = if path.is_empty() { "anonymous" } else { path };
        out.push(Region {
            start,
            end,
            label: String::from(label),
        });
    }
    out
}

/// Where `needle` occurs in the readable writable memory of this
/// process, by region, ignoring anything inside `skip` (the needle's own
/// bytes and the buffer the scan reads into).
fn occurrences(needle: &[u8], skip: &[(usize, usize)]) -> Vec<(String, usize)> {
    let mem = File::open("/proc/self/mem").expect("/proc/self/mem");
    let mut buf: Vec<u8> = Vec::new();
    let mut found = Vec::new();
    for region in regions() {
        let len = region.end - region.start;
        // A region larger than this is a guard or an arena the allocator
        // has never touched; reading it would cost more than it can say.
        if len == 0 || len > 1 << 28 {
            continue;
        }
        buf.clear();
        buf.resize(len, 0);
        if mem.read_exact_at(&mut buf, region.start as u64).is_err() {
            continue;
        }
        let buf_start = buf.as_ptr() as usize;
        let mine = [(buf_start, buf_start + buf.len())];
        let mut count = 0;
        let mut i = 0;
        while i + needle.len() <= buf.len() {
            if &buf[i..i + needle.len()] == needle {
                let at = region.start + i;
                let inside = |&(from, to): &(usize, usize)| at < to && at + needle.len() > from;
                if !skip.iter().any(inside) && !mine.iter().any(inside) {
                    count += 1;
                }
                i += needle.len();
            } else {
                i += 1;
            }
        }
        if count > 0 {
            found.push((region.label.clone(), count));
        }
    }
    found
}

/// Loads the test key, signs the demo transaction with it, and leaves
/// the result screen on Home.
fn load_sign_and_wipe(h: &mut Harness) {
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.set_network(3);

    h.open_single_sig(0);
    h.tap(ids::WALLET_SIGN);
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: DEMO.to_vec(),
    });

    for _ in 0..10 {
        if h.app.sign_stage() == Some(Stage::Wizard(Step::Confirm)) {
            break;
        }
        if h.app.rect_of(ids::SIGN_ACK).is_some() {
            h.tap(ids::SIGN_ACK);
        }
        h.tap(ids::SIGN_CONTINUE);
    }
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Confirm)));
    h.hold(ids::SIGN_HOLD);
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Result)));
    // The result screen offers Done only once the file is saved; this
    // flow saves nothing, so it leaves the wizard by the chevron.
    while h.app.screen() != ScreenKind::Home {
        h.tap(ids::BACK);
    }

    h.open_settings();
    h.tap(ids::SETTINGS_WIPE_ROW);
    h.hold(ids::SETTINGS_WIPE);
    assert_eq!(h.app.screen(), ScreenKind::Wiped);
    assert!(h.app.fingerprints().is_empty(), "every key is gone");
}

/// After a signing flow and a wipe, neither the seed nor the master
/// private key is anywhere in this process's writable memory.
///
/// Ignored by default: it reads `/proc/self/mem`, so it runs on Linux
/// only, and what a moved-from copy inside `bitcoin` or `secp256k1`
/// leaves behind depends on the build profile rather than on this
/// code. Run it against the profile you care about:
///
/// ```text
/// cargo test -p opensigner-core --test memory -- --ignored --nocapture
/// ```
#[test]
#[ignore = "reads /proc/self/mem; Linux only, and profile-dependent"]
fn a_wiped_key_is_not_left_in_memory() {
    let mut h = Harness::new(PANEL);
    load_sign_and_wipe(&mut h);
    drop(h);

    // One heap copy of each needle, whose own address is not counted.
    let seed = unhex(SEED_MIDDLE);
    let master_key = unhex(MASTER_KEY_MIDDLE);
    let skip: Vec<(usize, usize)> = [&seed, &master_key]
        .iter()
        .map(|n| (n.as_ptr() as usize, n.as_ptr() as usize + n.len()))
        .collect();

    let mut report = Vec::new();
    for (what, needle) in [("seed", &seed), ("master private key", &master_key)] {
        for (region, count) in occurrences(needle, &skip) {
            report.push(format!("{what}: {count} in {region}"));
        }
    }
    assert!(
        report.is_empty(),
        "the key is still in memory after the wipe:\n  {}",
        report.join("\n  ")
    );
}
