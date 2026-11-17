//! The leak tests (`docs/SECRET-LEAKS.md`): flows run in the core's
//! headless harness under an allocator that looks for a test secret in
//! every block the flow frees and, after Lock, in every block it still
//! holds. A binary of its own: a binary has one global allocator, and the
//! frame budgets' (`src/work/budget.rs`) and `tests/all`'s are others.
//!
//! A flow plants its secret (a published test vector, `forms.rs`), arms
//! the allocator with every form of it, runs, and ends in [`finish`]. A
//! copy found is a failure unless `tools/leak-entries.toml` names it
//! (`entries.rs`).

#[global_allocator]
static ALLOC: faraday_leak_scan::LeakScan = faraday_leak_scan::LeakScan;

mod backup;
mod canvas_text;
mod codex32;
#[path = "../all/common.rs"]
mod common;
mod entries;
mod forms;
mod planted;
mod readers;
mod scan;
mod seed_load;
mod sign;
mod stick_visit;
mod storage_events;
mod ur_message;
mod vault_names;
mod wallet_files;

use faraday_leak_scan::{Armed, Form};

/// Arms this thread, and the worker thread of a job it starts, with
/// `forms`. Build what the flow keeps (the secret's text, the files)
/// before this call.
fn arm(forms: &[Form]) -> Armed {
    let armed = faraday_leak_scan::arm(forms);
    let handle = armed.handle();
    faraday_core::testkit::on_worker_start(Some(Box::new(move || {
        let handle = handle.clone();
        Box::new(move || handle.arm_this_thread())
    })));
    armed
}

/// Ends a flow: when it ended in Lock (`locked`), every block still live
/// is searched; the run's freed blocks are set against the flow's
/// entries.
fn finish(flow: &str, armed: Armed, locked: bool) {
    faraday_core::testkit::on_worker_start(None);
    let live = if locked {
        armed.scan_live()
    } else {
        Vec::new()
    };
    let report = armed.finish();
    entries::verdict(flow, report, &live);
}

/// Ends a flow that left its screen without Lock (the arrow, Escape, the
/// mouse's back, finishing): every block still live is searched, and a
/// copy of a form not named in `kept` fails. `kept` names what the
/// session rightly holds after the screen is done (a loaded key's seed).
/// Freed blocks are not looked at here: the flow's lock test does that.
fn finish_left(flow: &str, armed: Armed, kept: &[&str]) {
    faraday_core::testkit::on_worker_start(None);
    let live: Vec<_> = armed
        .scan_live()
        .into_iter()
        .filter(|h| !kept.contains(&h.form.as_str()))
        .collect();
    let _ = armed.finish();
    assert!(
        live.is_empty(),
        "{flow}: a secret is still live after its screen was left: {live:?}"
    );
}
