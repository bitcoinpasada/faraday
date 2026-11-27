//! An adapter: Ask::Storage, Keep, Forget, Memory, Host, Print; the disk-process client
//! (09 §6; 02 §8).
//!
//! What this carries as far as the device's path goes: the sticks through the disk process
//! (`Ask::Storage`), the carry across a lock in the RAM directory (`Keep`, `Forget`,
//! `Start::carried`) and the memory read (`Memory`). `Host` and `Print` are the desktops' and
//! answer `Unsupported`.

mod disk;
mod keep;
mod memory;
mod serve;
mod worker;

use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;

use faraday_port::bytes::Bytes;
use faraday_port::storage::{
    self, Carry, Codes, NameRefused, Room, Sha256, StickId, Unrenamed, Wiped, Written, Zeroing,
};
use faraday_port::{Adapters, Answer, Ask, AskId, Effect, Event, Failure, Waker};
use faraday_text::Text;

pub use serve::unsupported;

use crate::keep::Keeping;
use crate::worker::Worker;

/// The storage adapter: folders as sticks, or the disk process.
pub struct Storage {
    keeping: Keeping,
    worker: Worker,
    /// Answers made on the caller's thread, and the worker's, in the order they are taken.
    ready: VecDeque<Event>,
}

impl Storage {
    /// The device: the disk process over its FIFO, with a RAM directory.
    ///
    /// `fifo` is the directory of the disk process's two pipes, `requests` and `responses`;
    /// `ram_dir` is where the carry waits between processes.
    pub fn disk_process(fifo: Text, ram_dir: Text) -> Result<Self, Failure> {
        let dir: &Path = Path::new(fifo.as_str());
        let sticks = disk::DiskSticks::open(&dir.join("requests"), &dir.join("responses"))?;
        Ok(Self::over(Box::new(sticks), Keeping::dir(ram_dir.as_str())))
    }

    /// The desktops: sticks as given by `sticks`.
    #[must_use]
    pub fn folders(sticks: Box<dyn Sticks>) -> Self {
        Self::over(sticks, Keeping::memory())
    }

    fn over(sticks: Box<dyn Sticks>, keeping: Keeping) -> Self {
        Storage {
            keeping,
            worker: Worker::start(sticks),
            ready: VecDeque::new(),
        }
    }

    /// Lets the worker's thread wake the root's loop when it has an answer.
    pub fn wake_with(&mut self, waker: Arc<dyn Waker>) {
        self.worker.wake_with(waker);
    }

    /// What the previous process kept for this one (02 §4.2): moved out once, the files read
    /// and removed, so the adapter keeps no copy after handing it over.
    pub fn take_carried(&mut self) -> Option<Carry> {
        self.keeping.take()
    }
}

/// Where the sticks' files are.
pub trait Sticks: Send {
    /// Every stick present now.
    fn list(&mut self) -> Vec<storage::StickInfo>;
    /// The sticks the last listing found unusable: the id, and whether the reason is that the
    /// stick is read-only (else it is not FAT).
    fn problems(&mut self) -> Vec<(StickId, bool)> {
        Vec::new()
    }
    fn read(&mut self, ask: storage::Read) -> Result<Bytes, Failure>;
    fn read_codes(&mut self, ask: storage::ReadCodes) -> Result<Codes, Failure>;
    fn write(&mut self, ask: storage::Write) -> Result<Written, Failure>;
    fn wipe(&mut self, ask: storage::Wipe) -> Result<Wiped, Failure>;
    fn space(&mut self, ask: storage::Space) -> Result<Room, Failure>;
    fn zero_free(&mut self, ask: storage::ZeroFree) -> Result<Zeroing, Failure>;
    fn make_empty(&mut self, ask: storage::MakeEmpty) -> Result<(), Failure>;
    fn rename(&mut self, ask: storage::Rename) -> Result<(), Unrenamed>;
    fn name(&mut self, ask: storage::Name) -> Result<(), NameRefused>;
    fn hash(&mut self, ask: storage::Hash) -> Result<Sha256, Failure>;
    /// The stick a folder stands for, by id.
    fn has(&self, stick: &StickId) -> bool;
}

impl Adapters for Storage {
    fn carry(&mut self, effect: Effect) {
        let Effect::Ask(id, ask) = effect else {
            return;
        };
        match ask {
            Ask::Storage(ask) => self.worker.send(id, ask),
            Ask::Keep(carry) => {
                let answer = self.keeping.keep(carry);
                self.answer(id, Answer::Kept(answer));
            }
            Ask::Forget => {
                self.keeping.forget();
                self.answer(id, Answer::Forgotten(Ok(())));
            }
            Ask::Memory => self.answer(id, Answer::Memory(memory::available_mib())),
            Ask::Host(ask) => self.answer(id, Answer::Host(serve::host_unsupported(&ask))),
            Ask::Print(_) => self.answer(id, Answer::Printed(Err(Failure::Unsupported))),
            // Not this adapter's: the root routes these elsewhere.
            Ask::Camera(_) | Ask::Clipboard | Ask::Entropy | Ask::Settings(_) | Ask::System(_) => {}
        }
    }

    fn poll(&mut self) -> Option<Event> {
        if let Some(event) = self.ready.pop_front() {
            return Some(event);
        }
        self.worker.poll()
    }
}

impl Storage {
    fn answer(&mut self, id: AskId, answer: Answer) {
        self.ready.push_back(Event::Answer(id, answer));
    }
}

#[cfg(test)]
mod tests {
    use faraday_crypto::Secret;
    use faraday_port::ask::Request;
    use faraday_port::event;
    use faraday_port::storage::{StickInfo, StickMake};

    use super::*;

    /// A stick that holds one file.
    struct One;

    fn no<T>() -> Result<T, Failure> {
        Err(Failure::Unsupported)
    }

    impl Sticks for One {
        fn list(&mut self) -> Vec<StickInfo> {
            vec![StickInfo {
                id: StickId(Text::of("one")),
                label: Text::of("ONE"),
                boot: false,
                serial: None,
                files: vec![(Text::of("a.txt"), 3)],
                vaults: Vec::new(),
                make: StickMake::default(),
            }]
        }
        fn read(&mut self, ask: storage::Read) -> Result<Bytes, Failure> {
            if ask.name.as_str() == "a.txt" {
                Ok(Secret::new(b"abc".to_vec()))
            } else {
                Err(Failure::Gone)
            }
        }
        fn read_codes(&mut self, _: storage::ReadCodes) -> Result<Codes, Failure> {
            no()
        }
        fn write(&mut self, _: storage::Write) -> Result<Written, Failure> {
            no()
        }
        fn wipe(&mut self, _: storage::Wipe) -> Result<Wiped, Failure> {
            no()
        }
        fn space(&mut self, _: storage::Space) -> Result<Room, Failure> {
            no()
        }
        fn zero_free(&mut self, _: storage::ZeroFree) -> Result<Zeroing, Failure> {
            no()
        }
        fn make_empty(&mut self, _: storage::MakeEmpty) -> Result<(), Failure> {
            no()
        }
        fn rename(&mut self, _: storage::Rename) -> Result<(), Unrenamed> {
            Err(Unrenamed::Failed(Failure::Unsupported))
        }
        fn name(&mut self, _: storage::Name) -> Result<(), NameRefused> {
            Err(NameRefused::Failed(Failure::Unsupported))
        }
        fn hash(&mut self, _: storage::Hash) -> Result<Sha256, Failure> {
            no()
        }
        fn has(&self, stick: &StickId) -> bool {
            stick.0.as_str() == "one"
        }
    }

    fn id(n: u32) -> AskId {
        AskId { generation: 0, n }
    }

    fn next(storage: &mut Storage, want: AskId) -> Answer {
        for _ in 0..400 {
            while let Some(event) = storage.poll() {
                if let Event::Answer(got, answer) = event
                    && got == want
                {
                    return answer;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("no answer");
    }

    fn carry(names: &[&str]) -> Carry {
        Carry {
            inbox: Vec::new(),
            outbox: Vec::new(),
            kept: names
                .iter()
                .map(|n| (Text::of(n), Secret::new(n.as_bytes().to_vec())))
                .collect(),
        }
    }

    /// Invariant: a read of a stick's file is answered to the ask that made it, in the read's own variant.
    /// Regression: an answer that reaches another ask, or one in a variant the request does not take.
    /// Owner: adapter
    #[test]
    fn an_answer_reaches_the_ask_that_made_it() {
        let mut s = Storage::folders(Box::new(One));
        let read = |name: &str| {
            Ask::Storage(storage::StorageAsk::Read {
                stick: StickId(Text::of("one")),
                name: Text::of(name),
            })
        };
        s.carry(Effect::Ask(id(1), read("a.txt")));
        s.carry(Effect::Ask(id(2), read("missing")));
        // The asks are answered in the order given (P19), each to its own id.
        let first = next(&mut s, id(1));
        let second = next(&mut s, id(2));
        assert!(matches!(
            <storage::Read as Request>::take(first),
            Ok(Ok(bytes)) if bytes.expose() == b"abc"
        ));
        assert!(matches!(
            <storage::Read as Request>::take(second),
            Ok(Err(Failure::Gone))
        ));
    }

    /// Invariant: the sticks present are told to the app as a listing, once, and again when they change.
    /// Regression: an app that never learns which sticks are plugged in.
    /// Owner: adapter
    #[test]
    fn the_sticks_present_are_listed() {
        let mut s = Storage::folders(Box::new(One));
        for _ in 0..400 {
            if let Some(Event::Sticks(event::SticksFact::Listed(list))) = s.poll() {
                assert_eq!(list.len(), 1);
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("no listing");
    }

    /// Invariant: a keep is answered `Kept`, and what it kept is handed over once, whole and in its own order.
    /// Regression: a carry that is lost across a lock, handed over twice, or kept as a copy beside the hand-over.
    /// Owner: adapter
    #[test]
    fn a_kept_carry_is_handed_over_once() {
        let mut s = Storage::folders(Box::new(One));
        s.carry(Effect::Ask(id(3), Ask::Keep(carry(&["b", "a"]))));
        let answer = next(&mut s, id(3));
        assert!(matches!(
            <faraday_port::ask::Keep as Request>::take(answer),
            Ok(Ok(()))
        ));
        let carried = s.take_carried().expect("a carry");
        let names: Vec<&str> = carried.kept.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["b", "a"]);
        assert!(s.take_carried().is_none());
    }

    /// Invariant: a carry kept in the RAM directory goes through a file each, is read back once and leaves no file.
    /// Regression: carried files left in the RAM directory after the next process took them, or a bad name written.
    /// Owner: adapter
    #[test]
    fn the_ram_directory_hands_the_carry_over_and_is_left_empty() {
        let dir = std::env::temp_dir().join(format!("faraday-keep-{}", std::process::id()));
        let mut keeping = keep::Keeping::dir(dir.to_str().unwrap_or_default());
        assert!(keeping.keep(carry(&["gate", "flow"])).is_ok());
        assert!(dir.join("kept").join("gate").is_file());
        let got = keeping.take().expect("a carry");
        assert_eq!(got.kept.len(), 2);
        assert!(!dir.join("kept").join("gate").exists());
        assert!(keeping.take().is_none());
        // A name that cannot be a file fails the keep and the carry comes back, nothing left behind.
        let bad = keeping.keep(carry(&["fine", "../escape"]));
        let Err((why, back)) = bad else {
            panic!("a bad name is refused");
        };
        assert_eq!(why, Failure::Adapter);
        assert_eq!(back.kept.len(), 2);
        assert!(!dir.join("kept").join("fine").exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Invariant: forgetting removes every carried file and is answered `Forgotten`.
    /// Regression: a carry that outlives the app's end.
    /// Owner: adapter
    #[test]
    fn forgetting_empties_the_carry() {
        let mut s = Storage::folders(Box::new(One));
        s.carry(Effect::Ask(id(4), Ask::Keep(carry(&["x"]))));
        let _ = next(&mut s, id(4));
        s.carry(Effect::Ask(id(5), Ask::Forget));
        let answer = next(&mut s, id(5));
        assert!(matches!(
            <faraday_port::ask::Forget as Request>::take(answer),
            Ok(Ok(()))
        ));
        assert!(s.take_carried().is_none());
    }
}
