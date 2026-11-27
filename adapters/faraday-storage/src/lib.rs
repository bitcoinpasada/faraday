//! An adapter: Ask::Storage, Keep, Forget, Memory, Host, Print; the disk-process client
//! (09 §6; 02 §8).

use faraday_port::bytes::Bytes;
use faraday_port::storage::{
    self, Codes, NameRefused, Room, Sha256, StickId, Unrenamed, Wiped, Written, Zeroing,
};
use faraday_port::{Adapters, Effect, Event, Failure};
use faraday_text::Text;

/// The storage adapter: folders as sticks, or the disk process.
pub struct Storage {
    _private: (),
}

impl Storage {
    /// The device: the disk process over its FIFO, with a RAM directory.
    pub fn disk_process(_fifo: Text, _ram_dir: Text) -> Result<Self, Failure> {
        todo!()
    }

    /// The desktops: sticks as given by `sticks`.
    #[must_use]
    pub fn folders(_sticks: Box<dyn Sticks>) -> Self {
        todo!()
    }
}

/// Where the sticks' files are.
pub trait Sticks: Send {
    /// Every stick present now.
    fn list(&mut self) -> Vec<storage::StickInfo>;
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
    fn carry(&mut self, _effect: Effect) {
        todo!()
    }

    fn poll(&mut self) -> Option<Event> {
        todo!()
    }
}
