//! The touch-target check over the snapshot tours (`docs/M3-CHECKS.md`
//! §2): every press target a frame offers is at least the touch floor square (44 dp on the
//! compact and short layout, 48 dp elsewhere, F188),
//! unless its control is cut by a clip or it is an exception of §3, and
//! what is under the floor today is held in `tools/touch-targets.counts`,
//! which only falls. The ratchet runs in `just lint-touch-targets`. The tool writes the
//! counted targets of a run to
//! `targets.txt` beside its pictures.

use crate::tour::Small;
use std::path::Path;

/// The action's kind: its variant's name, its fields left off.
pub(crate) fn kind(action: &str) -> &str {
    action.split(['(', ' ', '{']).next().unwrap_or(action)
}

/// Whether target `t`, `g` before any neighbour cut it back, is counted
/// as under the `floor` (`docs/M3-CHECKS.md` §2.2): it is when a side is
/// short and a neighbour trimmed that side; a side only its clip trims
/// is not counted.
pub(crate) fn short(t: faraday_draw::geom::Rect, g: faraday_draw::geom::Rect, floor: i32) -> bool {
    (t.w + 1 < floor && t.w < g.w) || (t.h + 1 < floor && t.h < g.h)
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
    // (F152).
    action.starts_with("Codex32(Cell(")
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
/// target, as `cut.txt` is written.
pub(crate) fn write(out: &Path, tour: Option<&str>, found: &[Small]) -> Result<(), String> {
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

/// The four sizes of the ratchet: the name its lines carry in
/// `tools/touch-targets.counts`, the window and its dpi.
#[cfg(test)]
const SIZES: [(&str, (u16, u16), u16); 4] = [
    ("small_240x320", (240, 320), 143),
    ("small_480x640", (480, 640), 286),
    ("stick_1024x600", (1024, 600), 170),
    ("desktop_1280x800", (1280, 800), 160),
];

/// One tour (or the default run, `None`) at one size, the counted
/// targets, each with the tour it was found in.
#[cfg(test)]
fn collect_tour(
    size: (u16, u16),
    dpi: u16,
    work: &Path,
    only: Option<&str>,
) -> Result<Vec<Small>, String> {
    let kit = work.join("kit");
    std::fs::create_dir_all(&kit).map_err(|e| e.to_string())?;
    for (name, bytes) in faraday_core::testkit::files().map_err(|e| String::from(&*e))? {
        std::fs::write(kit.join(name), bytes).map_err(|e| e.to_string())?;
    }
    let compute = faraday_core::testkit::compute();
    let out = work.join(only.unwrap_or("default"));
    crate::run(
        &compute,
        size,
        dpi,
        (&kit, &out),
        only,
        faraday_core::Medium::Stick,
        false,
    )
}

/// The tours the ratchet walks: the default run and every named tour but
/// the website's (made for 1280 × 800 on Mainnet; its frames are the named
/// tours').
#[cfg(test)]
fn tours() -> impl Iterator<Item = Option<&'static str>> {
    std::iter::once(None).chain(
        crate::tours::TOURS
            .iter()
            .map(|(n, _)| *n)
            .filter(|t| *t != "tour")
            .map(Some),
    )
}

/// Every tour at one size, collected: the counted targets, each with the
/// tour it was found in.
#[cfg(test)]
fn collect(size: (u16, u16), dpi: u16, work: &Path) -> Result<Vec<(String, Small)>, String> {
    let mut found = Vec::new();
    let mut failed = Vec::new();
    for only in tours() {
        let name = only.unwrap_or("default");
        match collect_tour(size, dpi, work, only) {
            Ok(small) => found.extend(small.into_iter().map(|s| (name.to_string(), s))),
            Err(e) => failed.push(format!("tour {name}: {e}")),
        }
    }
    if !failed.is_empty() {
        return Err(failed.join("\n"));
    }
    Ok(found)
}

/// A target's screen and action kind, the ratchet's key.
#[cfg(test)]
fn key_of(s: &Small) -> String {
    format!("{} {}", s.screen, kind(&format!("{:?}", s.action)))
}

/// The folder the walks leave their results in for the ratchet:
/// `FARADAY_TARGETS_DIR`, set by `just lint-touch-targets`.
#[cfg(test)]
fn results_dir() -> std::path::PathBuf {
    std::env::var_os("FARADAY_TARGETS_DIR")
        .expect("FARADAY_TARGETS_DIR: run these tests with `just lint-touch-targets`")
        .into()
}

/// One tour at the four sizes, as `just lint-touch-targets` runs it: the
/// counted targets are left in `FARADAY_TARGETS_DIR`, one file a size,
/// a line a target (key, action, the report line), for `ratchet` to hold
/// against the counts file.
#[cfg(test)]
fn walk(only: Option<&str>) {
    let tour = only.unwrap_or("default");
    for (name, size, dpi) in SIZES {
        let work = std::env::temp_dir().join(format!(
            "faraday-targets-{name}-{tour}-{}",
            std::process::id()
        ));
        let found = collect_tour(size, dpi, &work, only);
        let _ = std::fs::remove_dir_all(&work);
        let found = found.unwrap_or_else(|e| panic!("{name} tour {tour}: {e}"));
        let mut text = String::new();
        for s in &found {
            text.push_str(&format!(
                "{}\t{:?}\t{}\n",
                key_of(s),
                s.action,
                line(&format!("{name} {tour}"), s)
            ));
        }
        let dir = results_dir().join(name);
        std::fs::create_dir_all(&dir).expect("a folder");
        std::fs::write(dir.join(format!("{tour}.tsv")), text).expect("a result");
    }
}

/// The counted targets of every tour at one size, read from the walks'
/// results: (key, action, report line), in the order the tours run.
/// Fails for a tour whose walk left nothing.
#[cfg(test)]
fn gathered(name: &str) -> Result<Vec<(String, String, String)>, String> {
    let mut found = Vec::new();
    for only in tours() {
        let tour = only.unwrap_or("default");
        let path = results_dir().join(name).join(format!("{tour}.tsv"));
        let text = std::fs::read_to_string(&path).map_err(|e| {
            format!("tour {tour} at {name}: no result ({e}): is it one of the walks?")
        })?;
        for l in text.lines() {
            let mut f = l.splitn(3, '\t');
            let (Some(k), Some(a), Some(r)) = (f.next(), f.next(), f.next()) else {
                return Err(format!("{}: a line of three fields", path.display()));
            };
            found.push((k.to_string(), a.to_string(), r.to_string()));
        }
    }
    Ok(found)
}

/// Per screen and action kind, the distinct actions under the floor.
#[cfg(test)]
fn counted(found: &[(String, String, String)]) -> std::collections::BTreeMap<String, usize> {
    let mut seen: std::collections::BTreeMap<String, Vec<&str>> = std::collections::BTreeMap::new();
    for (key, action, _) in found {
        let actions = seen.entry(key.clone()).or_default();
        if !actions.contains(&action.as_str()) {
            actions.push(action);
        }
    }
    seen.into_iter().map(|(k, v)| (k, v.len())).collect()
}

/// The ratchet: the walks' results at the four sizes, held to the counts
/// file; with `FARADAY_TARGETS_WRITE` set, written to it instead.
#[cfg(test)]
#[test]
#[ignore = "the ratchet: `just lint-touch-targets` runs it after the walks"]
fn ratchet() {
    use std::collections::BTreeMap;
    let path = counts_path();
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let write = std::env::var_os("FARADAY_TARGETS_WRITE").is_some();
    // The lines of the other tests (the Starter's), kept as they are.
    let mut lines: Vec<String> = text
        .lines()
        .filter(|l| !l.is_empty() && !SIZES.iter().any(|(n, ..)| l.starts_with(&format!("{n} "))))
        .map(String::from)
        .collect();
    let mut fail = Vec::new();
    for (name, ..) in SIZES {
        let found = gathered(name).unwrap_or_else(|e| panic!("{name}: {e}"));
        let now = counted(&found);
        let prefix = format!("{name} ");
        if write {
            lines.extend(now.iter().map(|(k, n)| format!("{prefix}{k} {n}")));
            continue;
        }
        let mut held: BTreeMap<String, usize> = BTreeMap::new();
        for l in text.lines().filter_map(|l| l.strip_prefix(&prefix)) {
            let (key, n) = l.rsplit_once(' ').expect("a count");
            held.insert(key.to_string(), n.parse().expect("a number"));
        }
        let mut here = Vec::new();
        for (key, n) in &now {
            let limit = held.get(key).copied();
            if limit.is_none_or(|l| *n > l) {
                here.push(limit.map_or_else(
                    || format!("{key}: {n}, a new line"),
                    |l| format!("{key}: {n}, held at {l}"),
                ));
                for (k, _, report) in &found {
                    if k == key {
                        here.push(format!("    {report}"));
                    }
                }
            }
        }
        for (key, l) in &held {
            let n = now.get(key).copied().unwrap_or(0);
            if n < *l {
                here.push(format!(
                    "{key}: {n}, held at {l}: lower the line (FARADAY_TARGETS_WRITE=1 NEXTEST_TEST_THREADS=1 just lint-touch-targets)"
                ));
            }
        }
        if !here.is_empty() {
            fail.push(format!(
                "touch targets under the floor at {name} (docs/M3-CHECKS.md §2):\n{}",
                here.join("\n")
            ));
        }
    }
    if write {
        lines.sort();
        let mut body = lines.join("\n");
        body.push('\n');
        std::fs::write(&path, body).expect("the counts file");
        return;
    }
    assert!(fail.is_empty(), "{}", fail.join("\n"));
}

/// One walk test a tour, so that nextest runs them side by side. The list
/// is `tours::TOURS` less the website's `tour`, each name once: `ratchet`
/// fails for a tour with no walk, and a walk of a name not in the table
/// fails itself.
#[cfg(test)]
macro_rules! walks {
    ($($test:ident = $tour:expr,)*) => {
        $(
            #[test]
            #[ignore = "the ratchet's walk: `just lint-touch-targets` runs it"]
            fn $test() {
                walk($tour);
            }
        )*
    };
}

#[cfg(test)]
walks! {
    walk_default = None,
    walk_spend = Some("spend"),
    walk_check = Some("check"),
    walk_check_address = Some("check-address"),
    walk_open_tools = Some("open-tools"),
    walk_convert = Some("convert"),
    walk_invoice = Some("invoice"),
    walk_devices = Some("devices"),
    walk_toasts = Some("toasts"),
    walk_home_search = Some("home-search"),
    walk_themes = Some("themes"),
    walk_tooltips = Some("tooltips"),
    walk_compact = Some("compact"),
    walk_seeds = Some("seeds"),
    walk_keys = Some("keys"),
    walk_visit = Some("visit"),
    walk_files_wide = Some("files-wide"),
    walk_copy = Some("copy"),
    walk_scan = Some("scan"),
    walk_public = Some("public"),
    walk_qr = Some("qr"),
    walk_lightning_cln = Some("lightning-cln"),
    walk_seedfile = Some("seedfile"),
    walk_vaultway = Some("vaultway"),
    walk_vaultstates = Some("vaultstates"),
    walk_kef = Some("kef"),
    walk_kef_menu = Some("kef-menu"),
    walk_kef_ways = Some("kef-ways"),
    walk_kef_make = Some("kef-make"),
    walk_kef_step = Some("kef-step"),
    walk_ots = Some("ots"),
    walk_core_wallet = Some("core-wallet"),
    walk_kept = Some("kept"),
    walk_again = Some("again"),
    walk_quick = Some("quick"),
    walk_plan = Some("plan"),
    walk_board = Some("board"),
    walk_boardunsorted = Some("boardunsorted"),
    walk_menu = Some("menu"),
    walk_rowdetails = Some("rowdetails"),
    walk_band = Some("band"),
    walk_seed_pick = Some("seed-pick"),
    walk_m3 = Some("m3"),
    walk_inbox = Some("inbox"),
    walk_silent = Some("silent"),
    walk_keypath = Some("keypath"),
    walk_register = Some("register"),
    walk_multisig_check = Some("multisig-check"),
    walk_nonce = Some("nonce"),
    walk_signer = Some("signer"),
    walk_transfer = Some("transfer"),
    walk_glance = Some("glance"),
    walk_chartedit = Some("chartedit"),
    walk_oldparts = Some("oldparts"),
    walk_fromvault = Some("fromvault"),
    walk_first = Some("first"),
    walk_restore = Some("restore"),
    walk_osk = Some("osk"),
    walk_flow = Some("flow"),
    walk_flowbeside = Some("flowbeside"),
    walk_flowguide = Some("flowguide"),
    walk_gpg_restore = Some("gpg-restore"),
    walk_bootplace = Some("bootplace"),
    walk_passseeds = Some("passseeds"),
    walk_power = Some("power"),
    walk_pills = Some("pills"),
    walk_components = Some("components"),
    walk_host = Some("host"),
    walk_walletflow = Some("walletflow"),
    walk_demo = Some("demo"),
    walk_modes = Some("modes"),
    walk_vaultshow = Some("vaultshow"),
    walk_vaultsheets = Some("vaultsheets"),
    walk_addkey = Some("addkey"),
    walk_keygen = Some("keygen"),
    walk_dicewords = Some("dicewords"),
    walk_flipwords = Some("flipwords"),
    walk_codex32correct = Some("codex32correct"),
    walk_createfrom = Some("createfrom"),
    walk_inprogress = Some("inprogress"),
    walk_cancel = Some("cancel"),
    walk_busy = Some("busy"),
    walk_how = Some("how"),
    walk_std = Some("std"),
    walk_parts = Some("parts"),
    walk_panes = Some("panes"),
    walk_roller = Some("roller"),
    walk_randomness = Some("randomness"),
    walk_dice5 = Some("dice5"),
    walk_codex32 = Some("codex32"),
    walk_codex32form = Some("codex32form"),
    walk_createshares = Some("createshares"),
    walk_createslots = Some("createslots"),
    walk_codex32backup = Some("codex32backup"),
}

/// Every tour runs to its end at the smallest size, nothing counted: the
/// ratchet above runs in `just lint-touch-targets`.
#[cfg(test)]
#[test]
fn every_tour_runs_to_its_end() {
    let work = std::env::temp_dir().join(format!("faraday-targets-smoke-{}", std::process::id()));
    let run = collect((240, 320), 143, &work);
    let _ = std::fs::remove_dir_all(&work);
    run.unwrap_or_else(|e| panic!("{e}"));
}
