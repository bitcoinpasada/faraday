//! A part file on a stick, bytes of anyone's making: finishing an
//! interrupted vault write reads it whole and asks whether it is a whole
//! vault (`finish_vault_writes`, which runs the header's and the stream's
//! length checks over what a stick holds). It must not panic, and a part
//! file takes its name only when nothing else was there.

#![no_main]

use std::collections::BTreeMap;

use faraday_files::{Place, Zeroed, finish_vault_writes};
use faraday_text::Text;
use libfuzzer_sys::fuzz_target;

#[derive(Default)]
struct Memory(BTreeMap<String, Vec<u8>>);

impl Place for Memory {
    fn files(&mut self) -> Result<Vec<(Text, u64)>, Text> {
        Ok(self
            .0
            .iter()
            .map(|(n, b)| (Text::of(n), b.len() as u64))
            .collect())
    }
    fn read(&mut self, name: &str, _max: u64) -> Result<Vec<u8>, Text> {
        self.0.get(name).cloned().ok_or(Text::of("no such file"))
    }
    fn create(&mut self, name: &str, bytes: &[u8]) -> Result<(), Text> {
        self.0.insert(name.into(), bytes.to_vec());
        Ok(())
    }
    fn remove(&mut self, name: &str) -> Result<(), Text> {
        self.0.remove(name).map(|_| ()).ok_or(Text::of("no such file"))
    }
    fn wipe(&mut self, name: &str) -> Result<faraday_files::Wiped, Text> {
        self.remove(name).map(|()| faraday_files::Wiped::Zeroed)
    }
    fn rename(&mut self, from: &str, to: &str) -> Result<(), Text> {
        let b = self.0.remove(from).ok_or(Text::of("no such file"))?;
        self.0.insert(to.into(), b);
        Ok(())
    }
    fn space(&mut self) -> Result<(u64, u64), Text> {
        Ok((0, 0))
    }
    fn zero_free(&mut self, _from: u32) -> Result<Zeroed, Text> {
        Ok(Zeroed {
            next: 2,
            total: 0,
            done: true,
        })
    }
    fn make_empty(&mut self) -> Result<(), Text> {
        self.0.clear();
        Ok(())
    }
}

fuzz_target!(|data: &[u8]| {
    // Named for a vault of any name, and for a `.kdbx`.
    for part in [".faraday-v.kdbx.part", ".faraday-v.part"] {
        let mut place = Memory::default();
        place.0.insert(part.into(), data.to_vec());
        finish_vault_writes(&mut place);
        // Whatever stays of it is the same bytes.
        assert!(place.0.values().all(|b| b == data));
    }
});
