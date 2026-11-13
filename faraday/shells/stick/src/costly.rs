//! The costly work in the dev card's timings (`--timings`): which kind of
//! work an event did, by the counts the core keeps of it
//! (`faraday_core::work`), and how long the files written to a stick
//! took.
//!
//! The core counts each piece of costly work as it starts, through the
//! token every costly function takes, and keeps no clock. The shell
//! reads the counts before and after it hands the core an event, times
//! the call itself, and puts the time under the costliest kind the event
//! did ([`ORDER`]); a vault's key stretch is named by the note the core
//! keeps of it: what it was for, its strength's name, its Argon2id
//! memory and passes, and how it ended. Nothing here comes from what a
//! person typed or chose: no name, no passphrase, no file's name.
//!
//! A vault's stretch runs on the worker, so the main thread's time is only
//! the answer being applied. With `--timings` the worker times the stretch
//! itself, from its start to its end (`work::time_vaults`), and the note
//! carries it back with the answer: a second row, the same name with
//! [`ON_WORKER`] after it. A release card asks for nothing and the worker
//! reads no clock.

use std::time::{Duration, Instant};

use faraday_core::work::{self, Counts, VaultAct, VaultEnded, VaultWork, Work};
use faraday_storage::{PictureRead, Sticks, Wiped};

use crate::timings::Timings;

/// The kinds an event is put under, costliest first, each with the name
/// its row carries. The pixel work drawing does on every frame (a
/// sheet's backdrop, a fade) is the frame's own bucket, not a row here.
const ORDER: [(Work, &str); 20] = [
    (Work::Kdf, "key stretched"),
    (Work::Sign, "transaction signed"),
    (Work::PsbtInspect, "transaction checked"),
    (Work::PsbtParse, "transaction read"),
    (Work::QrDecode, "qr decoded"),
    (Work::PngDecode, "picture decoded"),
    (Work::Picture, "picture made"),
    (Work::QrEncode, "qr made"),
    (Work::Codex32, "codex32 worked"),
    (Work::MasterKey, "key made"),
    (Work::KeyDerive, "key derived"),
    (Work::WalletMatch, "wallet matched"),
    (Work::DescriptorParse, "wallet read"),
    (Work::Address, "address derived"),
    (Work::PlanMap, "plan mapped"),
    (Work::WordsParse, "words read"),
    (Work::FileHash, "file hashed"),
    (Work::Blind, "key blinded"),
    (Work::FileRead, "file head read"),
    (Work::TotpCode, "code made"),
];

/// What follows a vault row's name on the row of the worker's own time.
pub const ON_WORKER: &str = " (on the worker)";

/// What a vault stretch's row is called: what it was for, the strength
/// and its cost, and how it ended when it did not simply run.
pub fn vault_label(v: &VaultWork) -> String {
    let act = match v.act {
        VaultAct::Made => "vault made",
        VaultAct::Unlocked => "vault unlocked",
        VaultAct::Measured => "vault cost measured",
    };
    let ended = match v.ended {
        VaultEnded::Ran => "",
        VaultEnded::NoMemory => ", failed: no memory",
        VaultEnded::NotFree => ", not tried: too little memory free",
        VaultEnded::Refused => ", refused: header",
    };
    format!(
        "{act} {} {} MiB {} passes{ended}",
        v.strength(),
        v.memory_mib,
        v.passes
    )
}

/// The row an event that did `done` and noted `vaults` belongs on, or
/// `None` when it did no costly work. The vault notes name it when there
/// are any; otherwise the costliest kind in [`ORDER`] does.
pub fn label(done: &Counts, vaults: &[VaultWork]) -> Option<String> {
    if !vaults.is_empty() {
        let names: Vec<String> = vaults.iter().map(vault_label).collect();
        return Some(names.join(" + "));
    }
    ORDER
        .iter()
        .find(|(w, _)| done.of(*w) > 0)
        .map(|(_, name)| (*name).to_owned())
}

/// What is read before an event goes in, to time it and tell what it
/// did: taken only when `--timings` asked, so a release card reads no
/// clock.
pub struct Watch {
    before: Counts,
    start: Instant,
}

impl Watch {
    /// Starts watching, when the timings are kept. Cheap.
    pub fn start(timings: Option<&Timings>) -> Option<Watch> {
        timings.map(|_| Watch {
            before: work::snapshot(),
            start: Instant::now(),
        })
    }

    /// Ends the watch: the event's time, under its row, with `suffix`
    /// after the row's name. Cheap.
    pub fn end(self, timings: &mut Timings, suffix: &str) {
        let took = self.start.elapsed();
        let vaults = work::take_vault_work();
        for v in &vaults {
            if let Some(worker) = v.took {
                timings.work(&format!("{}{ON_WORKER}", vault_label(v)), worker);
            }
        }
        if let Some(name) = label(&work::since(&self.before), &vaults) {
            timings.work(&format!("{name}{suffix}"), took);
        }
    }
}

/// Where the sticks' files are, with each file written timed into the
/// timings when they are kept. Everything else is passed straight on.
pub struct Timed<'a> {
    pub sticks: &'a mut dyn Sticks,
    pub timings: Option<&'a mut Timings>,
}

impl Sticks for Timed<'_> {
    fn read(&mut self, stick: &str, name: &str) -> Result<Vec<u8>, faraday_text::Text> {
        self.sticks.read(stick, name)
    }

    fn write(
        &mut self,
        c: &faraday_crypto::work::Compute,
        stick: &str,
        name: &str,
        bytes: &[u8],
        copies: &[[u8; 32]],
        replace: bool,
        renamed: bool,
        on_taken: faraday_storage::OnTaken,
    ) -> Result<faraday_text::Text, faraday_storage::WriteRefused> {
        let start = self.timings.is_some().then(Instant::now);
        let wrote = self
            .sticks
            .write(c, stick, name, bytes, copies, replace, renamed, on_taken);
        if let (Some(start), Some(t)) = (start, self.timings.as_deref_mut()) {
            t.work(FILE_WRITTEN, start.elapsed());
        }
        wrote
    }

    fn transfer_later(
        &mut self,
        transfer: faraday_storage::Transfer,
    ) -> Result<(), faraday_storage::Transfer> {
        let written = match &transfer {
            faraday_storage::Transfer::Write { stick, name, .. } => {
                Some((stick.clone(), name.clone()))
            }
            faraday_storage::Transfer::Read { .. } => None,
        };
        let taken = self.sticks.transfer_later(transfer);
        // A file written in pieces is timed from here to its last piece,
        // as one file written (`transferred`).
        if let (Ok(()), Some((stick, name)), Some(t)) =
            (&taken, written, self.timings.as_deref_mut())
        {
            t.pieced_started(&stick, &name, Instant::now());
        }
        taken
    }

    fn transferred(&mut self) -> Vec<faraday_storage::Transferred> {
        let done = self.sticks.transferred();
        if let Some(t) = self.timings.as_deref_mut() {
            for d in &done {
                if let faraday_storage::Transferred::Write { stick, name, .. } = d
                    && let Some(took) = t.pieced_ended(stick, name)
                {
                    t.work(FILE_WRITTEN, took);
                }
            }
        }
        done
    }

    fn hash_later(&mut self, stick: &str, name: &str) -> bool {
        self.sticks.hash_later(stick, name)
    }

    fn hash_taken(&mut self, stick: &str, name: &str) -> bool {
        self.sticks.hash_taken(stick, name)
    }

    fn wipe(&mut self, stick: &str, name: &str) -> Result<Wiped, faraday_text::Text> {
        self.sticks.wipe(stick, name)
    }

    fn space(&mut self, stick: &str) -> Result<(u64, u64), faraday_text::Text> {
        self.sticks.space(stick)
    }

    fn zero_free(
        &mut self,
        stick: &str,
        from: u32,
    ) -> Result<(u32, u32, bool), faraday_text::Text> {
        self.sticks.zero_free(stick, from)
    }

    fn make_empty(&mut self, stick: &str) -> Result<(), faraday_text::Text> {
        self.sticks.make_empty(stick)
    }

    fn read_qr_later(&mut self, stick: &str, name: &str) {
        self.sticks.read_qr_later(stick, name);
    }

    fn pictures_read(&mut self) -> Vec<PictureRead> {
        self.sticks.pictures_read()
    }

    fn rename(
        &mut self,
        c: &faraday_crypto::work::Compute,
        stick: &str,
        from: &str,
        to: &str,
        vaults: &[Vec<[u8; 32]>],
    ) -> Result<(), faraday_storage::Unrenamed> {
        self.sticks.rename(c, stick, from, to, vaults)
    }

    fn name(&mut self, stick: &str, label: &str) -> Result<(), faraday_text::Text> {
        self.sticks.name(stick, label)
    }
}

/// The row a file the app wrote to a stick or the card is timed on.
pub const FILE_WRITTEN: &str = "file written";

/// The row the boot report's own write through the disk process is
/// timed on: how long it holds the loop every 30 seconds.
pub const DEV_FILE_WRITTEN: &str = "boot report written";

/// The row one of the scanner's passes over a camera frame is timed on.
pub const CAMERA_DECODE: &str = "qr decoded (camera pass)";

/// Times `f` onto `row` when the timings are kept.
pub fn timed<T>(timings: &mut Option<Timings>, row: &str, f: impl FnOnce() -> T) -> T {
    let start = timings.is_some().then(Instant::now);
    let out = f();
    if let (Some(start), Some(t)) = (start, timings.as_mut()) {
        t.work(row, start.elapsed());
    }
    out
}

/// Each camera pass's time, as the scanner gives them, onto its row.
pub fn camera_passes(timings: &mut Timings, passes: Vec<Duration>) {
    for took in passes {
        timings.work(CAMERA_DECODE, took);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault(act: VaultAct, memory_mib: u32, passes: u32, ended: VaultEnded) -> VaultWork {
        VaultWork {
            act,
            memory_mib,
            passes,
            ended,
            took: None,
        }
    }

    #[test]
    fn a_vault_row_names_its_strength_and_its_memory() {
        assert_eq!(
            vault_label(&vault(VaultAct::Unlocked, 64, 3, VaultEnded::Ran)),
            "vault unlocked Light 64 MiB 3 passes"
        );
        assert_eq!(
            vault_label(&vault(VaultAct::Made, 256, 3, VaultEnded::Ran)),
            "vault made Standard 256 MiB 3 passes"
        );
        assert_eq!(
            vault_label(&vault(VaultAct::Made, 384, 5, VaultEnded::Ran)),
            "vault made Custom 384 MiB 5 passes"
        );
    }

    #[test]
    fn a_vault_that_did_not_fit_in_memory_says_so() {
        assert_eq!(
            vault_label(&vault(VaultAct::Unlocked, 1024, 4, VaultEnded::NoMemory)),
            "vault unlocked Strong 1024 MiB 4 passes, failed: no memory"
        );
        assert_eq!(
            vault_label(&vault(VaultAct::Made, 512, 3, VaultEnded::NotFree)),
            "vault made Medium 512 MiB 3 passes, not tried: too little memory free"
        );
    }

    #[test]
    fn an_event_goes_on_the_row_of_its_costliest_work() {
        let before = work::snapshot();
        work::count(Work::KeyDerive);
        work::count(Work::Sign);
        work::count(Work::Address);
        let done = work::since(&before);
        assert_eq!(label(&done, &[]).as_deref(), Some("transaction signed"));
        let before = work::snapshot();
        work::count(Work::MasterKey);
        assert_eq!(
            label(&work::since(&before), &[]).as_deref(),
            Some("key made")
        );
    }

    #[test]
    fn an_event_with_no_costly_work_has_no_row() {
        let before = work::snapshot();
        work::count(Work::FullCanvas);
        assert_eq!(label(&work::since(&before), &[]), None);
    }

    #[test]
    fn a_vault_note_names_the_row_over_the_kinds_it_did() {
        let before = work::snapshot();
        work::count(Work::Kdf);
        work::count(Work::MasterKey);
        let v = [vault(VaultAct::Unlocked, 64, 3, VaultEnded::Ran)];
        assert_eq!(
            label(&work::since(&before), &v).as_deref(),
            Some("vault unlocked Light 64 MiB 3 passes")
        );
    }
}
