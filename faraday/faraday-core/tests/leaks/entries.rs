//! What a flow's run found, set against the allowed entries
//! (`tools/leak-entries.toml`, `docs/SECRET-LEAKS.md` §7 and §9): a freed
//! block that held a form passes only when an entry of the flow names the
//! form and the place that freed it; an entry the flow no longer hits
//! fails; a block still live after Lock never passes.

use faraday_leak_scan::{Hit, Report};

const FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/leak-entries.toml");

/// One allowed entry.
pub struct Entry {
    pub flow: String,
    pub form: String,
    pub place: String,
    pub reason: String,
    pub fix: String,
}

/// The entries of the file, read as `[[entry]]` tables of `key = "value"`
/// lines.
pub fn entries() -> Vec<Entry> {
    let text = std::fs::read_to_string(FILE).expect("tools/leak-entries.toml is there");
    let mut out: Vec<Entry> = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[[entry]]" {
            out.push(Entry {
                flow: String::new(),
                form: String::new(),
                place: String::new(),
                reason: String::new(),
                fix: String::new(),
            });
            continue;
        }
        let (key, value) = line.split_once('=').expect("a key = \"value\" line");
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .expect("a quoted value")
            .replace("\\\"", "\"");
        let e = out.last_mut().expect("a [[entry]] first");
        match key.trim() {
            "flow" => e.flow = value,
            "form" => e.form = value,
            "place" => e.place = value,
            "reason" => e.reason = value,
            "fix" => e.fix = value,
            other => panic!("an entry has no field {other}"),
        }
    }
    for e in &out {
        assert!(
            [&e.flow, &e.form, &e.place, &e.reason, &e.fix]
                .iter()
                .all(|f| !f.is_empty()),
            "an entry names its flow, form, place, reason and fix: {}",
            e.place
        );
        assert!(
            e.reason.contains("at every lock"),
            "an entry's reason names what bounds the copy, the restart at every lock (F181 point 2): {}",
            e.place
        );
    }
    out
}

/// The symbols of a backtrace, nearest the free first.
fn frames(trace: &str) -> Vec<String> {
    trace
        .lines()
        .filter_map(|l| {
            let (n, symbol) = l.trim().split_once(": ")?;
            n.chars().all(|c| c.is_ascii_digit()).then(|| {
                let symbol = symbol.trim();
                match symbol.rsplit_once("::h") {
                    Some((s, h)) if h.len() == 16 => s.to_string(),
                    _ => symbol.to_string(),
                }
            })
        })
        .collect()
}

/// The frames that are Faraday's or a library's: those after the
/// allocator's own and the standard library's drop glue.
fn owners(trace: &str) -> Vec<String> {
    // The standard library's, and the drop glue, the deallocation and the
    // allocator's own frames between a free and the code that made it.
    const SKIP: [&str; 28] = [
        "std::",
        "core::",
        "alloc::",
        "<std::",
        "<core::",
        "<alloc::",
        "drop_in_place",
        "drop<",
        "drop_elements",
        "drop_inner_table",
        "dealloc",
        "deallocate",
        "try_with",
        "<hashbrown::",
        "<bool>::",
        "<&mut alloc::",
        "<faraday_leak_scan::",
        "map_or_else",
        "format",
        "{closure#",
        "drop_glue",
        "realloc",
        "grow",
        "finish_grow",
        "do_reserve",
        "reserve",
        "extend",
        "spec_extend",
    ];
    let frames = frames(trace);
    let from = frames
        .iter()
        .position(|f| f.starts_with("<faraday_leak_scan::LeakScan as"))
        .map_or(0, |i| i + 1);
    frames
        .into_iter()
        .skip(from)
        .filter(|f| {
            !SKIP.iter().any(|s| f.starts_with(s))
                && !f.starts_with("__")
                && !f.starts_with("faraday_leak_scan::")
        })
        .collect()
}

/// A frame's name cut to what a line can hold.
fn short(frame: &str) -> &str {
    match frame.char_indices().nth(70) {
        Some((i, _)) => &frame[..i],
        None => frame,
    }
}

/// Fails unless what `flow` found is what its entries allow: `report` is
/// the freed blocks, `live` the blocks live after Lock.
pub fn verdict(flow: &str, report: Report, live: &[Hit]) {
    let mut problems: Vec<String> = Vec::new();
    for h in live {
        problems.push(format!(
            "kept after Lock, which no entry passes: {} at {} of {} bytes",
            h.form, h.offset, h.size
        ));
    }
    if report.dropped > 0 {
        problems.push(format!("{} more hits than are kept", report.dropped));
    }
    let entries: Vec<Entry> = entries().into_iter().filter(|e| e.flow == flow).collect();
    let mut hit = vec![false; entries.len()];
    let mut seen: Vec<(String, String)> = Vec::new();
    for h in &report.freed {
        let trace = h.backtrace();
        let frames = owners(&trace);
        let top = frames.first().cloned().unwrap_or_default();
        let matched = entries
            .iter()
            .position(|e| e.form == h.form && frames.iter().take(6).any(|f| f.contains(&e.place)));
        match matched {
            Some(i) => hit[i] = true,
            None => {
                let key = (h.form.clone(), top);
                if !seen.contains(&key) {
                    let top: Vec<&str> = frames.iter().take(8).map(|f| short(f)).collect();
                    problems.push(format!(
                        "freed with no entry: {} at {} of {} bytes, in {}",
                        h.form,
                        h.offset,
                        h.size,
                        top.join(" < ")
                    ));
                    seen.push(key);
                }
            }
        }
    }
    for (e, hit) in entries.iter().zip(hit) {
        if !hit {
            problems.push(format!(
                "an entry no run hits any more (remove it): {} in {}",
                e.form, e.place
            ));
        }
    }
    assert!(problems.is_empty(), "{flow}:\n{}", problems.join("\n"));
}
