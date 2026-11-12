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

#[path = "../all/common.rs"]
mod common;
mod entries;
mod forms;
mod planted;
mod seed_load;

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
