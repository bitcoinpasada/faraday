//! The Starter's press targets (`docs/M3-CHECKS.md` §2): every frame the
//! snapshot example draws offers presses at least as large as the touch
//! floor, unless a clip cut the control or it is an exception of §3, and
//! what is under the floor today is held in `tools/touch-targets.counts`
//! (the Starter's lines, `starter_<size>`), which only falls.

use std::collections::BTreeMap;
use std::path::Path;

#[path = "../../examples/snap.rs"]
#[expect(dead_code, reason = "the example's main and its other tours")]
mod snap;

use snap::{Small, Snap};

/// The action's kind: its variant's name, its fields left off.
fn kind(action: &str) -> &str {
    action.split(['(', ' ', '{']).next().unwrap_or(action)
}

/// Whether a target of `action` is an exception of `docs/M3-CHECKS.md` §3
/// on a window of `small` class (under 600 dp wide).
fn excepted(action: &str, small: bool) -> bool {
    (small
        && matches!(
            kind(action),
            "Osk" | "EntryKey" | "EntryKeyBack" | "EntryShift"
        ))
        || action.starts_with("Codex32(Cell(")
        || action.starts_with("Board(Chip(")
}

fn line(label: &str, s: &Small) -> String {
    let (x, y, w, h) = s.drawn;
    let sheet = s
        .sheet
        .as_ref()
        .map_or_else(|| "(no sheet)".to_string(), |s| format!("({s})"));
    format!(
        "{label} {} {} {sheet} {:?}: target {} × {} px, floor {}; drawn {w} × {h} at ({x}, {y})",
        s.step, s.screen, s.action, s.target.0, s.target.1, s.floor,
    )
}

fn key_of(s: &Small) -> String {
    format!("{} {}", s.screen, kind(&format!("{:?}", s.action)))
}

/// The frames of the example's default run at one size, held to the
/// counts file.
fn at(name: &str, size: (u16, u16), dpi: u16) {
    let started = std::time::Instant::now();
    // The example writes its PDFs beside its pictures: into a folder of
    // its own, removed after.
    let work = std::env::temp_dir().join(format!("faraday-targets-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&work).expect("a folder");
    let snap = Snap {
        out: &work,
        size,
        dpi,
        theme: faraday_ui::Theme::default(),
        text: None,
        pictures: false,
        targets: Some(std::cell::RefCell::new(Vec::new())),
    };
    let run = snap::run(&snap);
    let _ = std::fs::remove_dir_all(&work);
    run.unwrap_or_else(|e| panic!("{name}: {e}"));
    let small = f32::from(size.0) / (f32::from(dpi) / 160.0) < 600.0;
    let found: Vec<Small> = snap
        .targets
        .expect("collected")
        .into_inner()
        .into_iter()
        .filter(|s| !excepted(&format!("{:?}", s.action), small))
        .collect();
    eprintln!("{name}: {} s", started.elapsed().as_secs());
    let mut now: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for s in &found {
        let seen = now.entry(key_of(s)).or_default();
        let a = format!("{:?}", s.action);
        if !seen.contains(&a) {
            seen.push(a);
        }
    }
    let now: BTreeMap<String, usize> = now.into_iter().map(|(k, v)| (k, v.len())).collect();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/touch-targets.counts");
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let prefix = format!("{name} ");
    let mut other = Vec::new();
    let mut held: BTreeMap<String, usize> = BTreeMap::new();
    for l in text.lines().filter(|l| !l.is_empty()) {
        match l.strip_prefix(&prefix) {
            Some(rest) => {
                let (key, n) = rest.rsplit_once(' ').expect("a count");
                held.insert(key.to_string(), n.parse().expect("a number"));
            }
            None => other.push(l.to_string()),
        }
    }
    if std::env::var_os("FARADAY_TARGETS_WRITE").is_some() {
        // The tests of both crates write one file: run them with `-j 1`.
        let mut lines = other;
        lines.extend(now.iter().map(|(k, n)| format!("{prefix}{k} {n}")));
        lines.sort();
        let mut body = lines.join("\n");
        body.push('\n');
        std::fs::write(&path, body).expect("the counts file");
        return;
    }
    let mut fail = Vec::new();
    for (key, n) in &now {
        let limit = held.get(key).copied();
        if limit.is_none_or(|l| *n > l) {
            fail.push(limit.map_or_else(
                || format!("{key}: {n}, a new line"),
                |l| format!("{key}: {n}, held at {l}"),
            ));
            for s in found.iter().filter(|s| key_of(s) == *key) {
                fail.push(format!("    {}", line(name, s)));
            }
        }
    }
    for (key, l) in &held {
        let n = now.get(key).copied().unwrap_or(0);
        if n < *l {
            fail.push(format!(
                "{key}: {n}, held at {l}: lower the line (FARADAY_TARGETS_WRITE=1, with -j 1)"
            ));
        }
    }
    assert!(
        fail.is_empty(),
        "touch targets under 48 dp at {name} (docs/M3-CHECKS.md §2):\n{}",
        fail.join("\n")
    );
}

#[test]
fn starter_240x320() {
    at("starter_240x320", (240, 320), 143);
}

#[test]
fn starter_480x640() {
    at("starter_480x640", (480, 640), 286);
}

#[test]
fn starter_1024x600() {
    at("starter_1024x600", (1024, 600), 170);
}

#[test]
fn starter_1280x800() {
    at("starter_1280x800", (1280, 800), 160);
}
