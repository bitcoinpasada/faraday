//! An adapter: sticks as folders (the folder `Place` from `faraday-files`), `ToggleStick`,
//! Downloads, opening, the print folder, the camera list (09 §6).

use std::fs;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::Arc;

use faraday_files::{Place, Wiped, Zeroed};
use faraday_port::{Adapters, Effect, Event, Waker};
use faraday_strings::EN;
use faraday_text::Text;

/// The desktop's adapter: folder sticks and the host.
pub struct Desktop {
    _private: (),
}

impl Desktop {
    #[must_use]
    pub fn new(_sticks: Text, _downloads: Option<Text>, _waker: Arc<dyn Waker>) -> Self {
        todo!()
    }
}

impl Adapters for Desktop {
    fn carry(&mut self, _effect: Effect) {
        todo!()
    }

    fn poll(&mut self) -> Option<Event> {
        todo!()
    }
}

/// Why the computer refused a folder's file, in the strings file's
/// words: the operating system's own message is not shown.
fn io_text(e: std::io::Error) -> Text {
    use std::io::ErrorKind as K;
    let s = match e.kind() {
        K::NotFound => EN.fat_not_found,
        K::AlreadyExists => EN.fat_exists,
        K::StorageFull => EN.fat_full,
        K::PermissionDenied => EN.files_denied,
        K::ReadOnlyFilesystem => EN.files_read_only,
        _ => EN.fat_io,
    };
    Text::of(s)
}

/// A folder standing for a stick: `Place` over `std::fs`.
pub struct FolderPlace(pub PathBuf);

impl Place for FolderPlace {
    fn files(&mut self) -> Result<Vec<(Text, u64)>, Text> {
        Ok(fs::read_dir(&self.0)
            .map_err(io_text)?
            .filter_map(Result::ok)
            .filter_map(|e| {
                let meta = e.metadata().ok()?;
                meta.is_file()
                    .then(|| (Text::of(&e.file_name().to_string_lossy()), meta.len()))
            })
            .collect())
    }

    fn read(&mut self, name: &str, max: u64) -> Result<Vec<u8>, Text> {
        let path = self.0.join(name);
        let meta = fs::metadata(&path).map_err(io_text)?;
        if meta.len() > max {
            return Err(Text::fill(
                EN.files_larger_than,
                &[&Text::num(max / 1024 / 1024)],
            ));
        }
        fs::read(&path).map_err(io_text)
    }

    fn head(&mut self, name: &str, n: usize, max: u64) -> Result<Vec<u8>, Text> {
        use std::io::Read;
        let path = self.0.join(name);
        let meta = fs::metadata(&path).map_err(io_text)?;
        if meta.len() > max {
            return Err(Text::fill(
                EN.files_larger_than,
                &[&Text::num(max / 1024 / 1024)],
            ));
        }
        let mut out = Vec::new();
        fs::File::open(&path)
            .map_err(io_text)?
            .take(n as u64)
            .read_to_end(&mut out)
            .map_err(io_text)?;
        Ok(out)
    }

    fn read_part(
        &mut self,
        name: &str,
        from: u64,
        len: usize,
        max: u64,
    ) -> Result<(u64, Vec<u8>), Text> {
        use std::io::{Read, Seek, SeekFrom};
        let path = self.0.join(name);
        let meta = fs::metadata(&path).map_err(io_text)?;
        if meta.len() > max {
            return Err(Text::fill(
                EN.files_larger_than,
                &[&Text::num(max / 1024 / 1024)],
            ));
        }
        let mut file = fs::File::open(&path).map_err(io_text)?;
        file.seek(SeekFrom::Start(from.min(meta.len())))
            .map_err(io_text)?;
        let mut out = Vec::new();
        out.try_reserve_exact(len.min(meta.len() as usize))
            .map_err(|_| Text::of(EN.fat_io))?;
        file.take(len as u64)
            .read_to_end(&mut out)
            .map_err(io_text)?;
        Ok((meta.len(), out))
    }

    fn create(&mut self, name: &str, bytes: &[u8]) -> Result<(), Text> {
        let mut f = fs::File::create_new(self.0.join(name)).map_err(io_text)?;
        f.write_all(bytes).map_err(io_text)?;
        f.sync_all().map_err(io_text)
    }

    fn remove(&mut self, name: &str) -> Result<(), Text> {
        fs::remove_file(self.0.join(name)).map_err(io_text)
    }

    fn wipe(&mut self, name: &str) -> Result<Wiped, Text> {
        use std::io::{Read, Seek, SeekFrom};
        let path = self.0.join(name);
        let mut f = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(io_text)?;
        let len = f.metadata().map_err(io_text)?.len();
        let zeros = [0u8; 64 * 1024];
        let mut left = len;
        while left > 0 {
            let n = left.min(zeros.len() as u64) as usize;
            f.write_all(&zeros[..n]).map_err(io_text)?;
            left = left.saturating_sub(n as u64);
        }
        f.sync_all().map_err(io_text)?;
        f.seek(SeekFrom::Start(0)).map_err(io_text)?;
        let mut back = [0u8; 64 * 1024];
        loop {
            let n = f.read(&mut back).map_err(io_text)?;
            if n == 0 {
                break;
            }
            if back[..n].iter().any(|&b| b != 0) {
                return Err(Text::of(EN.fat_fault_unwritten));
            }
        }
        drop(f);
        fs::remove_file(path).map_err(io_text)?;
        Ok(Wiped::Zeroed)
    }

    // A folder standing for a stick has no free space of its own to
    // write over: the computer's disk holds it.
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
        for (name, _) in self.files()? {
            self.wipe(&name)?;
        }
        Ok(())
    }

    fn rename(&mut self, from: &str, to: &str) -> Result<(), Text> {
        if self.0.join(to).exists() {
            return Err(Text::fill(EN.files_already_there, &[to]));
        }
        fs::rename(self.0.join(from), self.0.join(to)).map_err(io_text)
    }
}
