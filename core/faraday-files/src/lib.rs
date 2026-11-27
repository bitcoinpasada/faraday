//! The domain crate `faraday-files`: Faraday's own code, copied from `main` in a later step.
//! Everything a crate of the domain may do is in `docs/spec/01-layers.md` section 2.

extern crate alloc;

use alloc::vec::Vec;

use faraday_text::Text;

pub use faraday_fat::{Wiped, Zeroed};

// Placeholder until the domain copy: `main`'s `faraday/faraday-files/src/lib.rs`.
/// A stick's top level.
pub trait Place {
    fn files(&mut self) -> Result<Vec<(Text, u64)>, Text>;
    fn read(&mut self, name: &str, max: u64) -> Result<Vec<u8>, Text>;
    fn create(&mut self, name: &str, bytes: &[u8]) -> Result<(), Text>;
    fn remove(&mut self, name: &str) -> Result<(), Text>;
    fn wipe(&mut self, name: &str) -> Result<Wiped, Text>;
    fn rename(&mut self, from: &str, to: &str) -> Result<(), Text>;
    fn space(&mut self) -> Result<(u64, u64), Text>;
    fn zero_free(&mut self, from: u32) -> Result<Zeroed, Text>;
    fn make_empty(&mut self) -> Result<(), Text>;
}
