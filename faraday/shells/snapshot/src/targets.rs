//! The touch-target check over the snapshot tours (`docs/M3-CHECKS.md`
//! §2): every press target a frame offers is at least `TOUCH` square,
//! unless its control is cut by a clip or it is an exception of §3, and
//! what is under the floor today is held in `tools/touch-targets.counts`,
//! which only falls. The tool writes the counted targets of a run to
//! `targets.txt` beside its pictures.

use crate::tour::Small;
use std::path::Path;

/// The action's kind: its variant's name, its fields left off.
pub(crate) fn kind(action: &str) -> &str {
    action.split(['(', ' ', '{']).next().unwrap_or(action)
}

/// Whether a target of `action` (its `Debug` text) is an exception of
/// `docs/M3-CHECKS.md` §3 on a layout of the small class (a window under
/// 600 dp wide: the 240 × 320 and 480 × 640 panels), where it cannot be
/// larger.
pub(crate) fn excepted(action: &str, small: bool) -> bool {
    // The on-screen keyboard's and word entry's keys: ten keys across a
    // small panel (F152).
    if small
        && matches!(
            kind(action),
            "Osk" | "EntryKey" | "EntryKeyBack" | "EntryShift"
        )
    {
        return true;
    }
    // The codex32 worksheet's cells, a grid that must be read whole
    // (F152), and the backup board's chips, sharing the gap with their
    // neighbours (F108).
    action.starts_with("Codex32(Cell(") || action.starts_with("Board(Chip(")
}

/// The line `targets.txt` and the test's report carry for `s`, found in
/// `label` (the test and the tour, or the tour alone).
pub(crate) fn line(label: &str, s: &Small) -> String {
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

/// Writes `targets.txt` beside the pictures in `out`, one line a counted
/// target, as `cut.txt` is written. The guide's tours write none: their
/// folder holds pictures only.
pub(crate) fn write(out: &Path, tour: Option<&str>, found: &[Small]) -> Result<(), String> {
    if matches!(tour, Some("guide" | "screens")) {
        return Ok(());
    }
    let label = tour.unwrap_or("default");
    let mut text = String::new();
    for s in found {
        text.push_str(&line(label, s));
        text.push('\n');
    }
    let path = out.join("targets.txt");
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The ratchet's file, `tools/touch-targets.counts` (§2.5).
#[cfg(test)]
fn counts_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tools/touch-targets.counts")
}

/// Every tour at one size, collected: the counted targets, each with the
/// tour it was found in.
#[cfg(test)]
fn collect(size: (u16, u16), dpi: u16, work: &Path) -> Result<Vec<(String, Small)>, String> {
    let kit = work.join("kit");
    std::fs::create_dir_all(&kit).map_err(|e| e.to_string())?;
    for (name, bytes) in faraday_core::testkit::files()? {
        std::fs::write(kit.join(name), bytes).map_err(|e| e.to_string())?;
    }
    let compute = faraday_core::testkit::compute();
    let mut found = Vec::new();
    let tours = std::iter::once(None).chain(
        crate::TOURS
            .iter()
            .copied()
            // The pictures of the guide and the website are made for
            // 1280 × 800 on Mainnet; their frames are the named tours'.
            .filter(|t| !matches!(*t, "guide" | "screens" | "tour"))
            .map(Some),
    );
    for only in tours {
        let name = only.unwrap_or("default");
        let out = work.join(name);
        let small = crate::run(
            &compute,
            size,
            dpi,
            (&kit, &out),
            only,
            faraday_core::Medium::Stick,
            false,
        )
        .map_err(|e| format!("tour {name}: {e}"))?;
        found.extend(small.into_iter().map(|s| (name.to_string(), s)));
    }
    Ok(found)
}

/// A target's screen and action kind, the ratchet's key.
#[cfg(test)]
fn key_of(s: &Small) -> String {
    format!("{} {}", s.screen, kind(&format!("{:?}", s.action)))
}

/// Per screen and action kind, the distinct actions under the floor.
#[cfg(test)]
fn counted(found: &[(String, Small)]) -> std::collections::BTreeMap<String, usize> {
    let mut seen: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for (_, s) in found {
        let actions = seen.entry(key_of(s)).or_default();
        let a = format!("{:?}", s.action);
        if !actions.contains(&a) {
            actions.push(a);
        }
    }
    seen.into_iter().map(|(k, v)| (k, v.len())).collect()
}

/// The test at one size: collects, then holds the counts file.
#[cfg(test)]
fn at(name: &str, size: (u16, u16), dpi: u16) {
    use std::collections::BTreeMap;
    let started = std::time::Instant::now();
    let work = std::env::temp_dir().join(format!("faraday-targets-{name}-{}", std::process::id()));
    let found = collect(size, dpi, &work).unwrap_or_else(|e| panic!("{name}: {e}"));
    let _ = std::fs::remove_dir_all(&work);
    eprintln!("{name}: {} s", started.elapsed().as_secs());
    let now = counted(&found);
    let path = counts_path();
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
        // The four tests write one file: run them with `-j 1`.
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
            for (tour, s) in &found {
                if key_of(s) == *key {
                    fail.push(format!("    {}", line(&format!("{name} {tour}"), s)));
                }
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

#[cfg(test)]
#[test]
fn small_240x320() {
    at("small_240x320", (240, 320), 143);
}

#[cfg(test)]
#[test]
fn small_480x640() {
    at("small_480x640", (480, 640), 286);
}

#[cfg(test)]
#[test]
fn stick_1024x600() {
    at("stick_1024x600", (1024, 600), 170);
}

#[cfg(test)]
#[test]
fn desktop_1280x800() {
    at("desktop_1280x800", (1280, 800), 160);
}
