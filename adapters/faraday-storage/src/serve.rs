//! One ask of the sticks, taken to the `Sticks` that has them and answered with its own answer
//! variant (P9).

use faraday_port::Failure;
use faraday_port::host::{HostAnswer, HostAsk};
use faraday_port::storage::{
    Hash, MakeEmpty, Name, NameRefused, Read, ReadCodes, Rename, Space, StorageAnswer, StorageAsk,
    Unrenamed, Wipe, Write, ZeroFree,
};

use crate::Sticks;

/// Takes `ask` to `sticks` and answers it.
pub fn serve(sticks: &mut dyn Sticks, ask: StorageAsk) -> StorageAnswer {
    match ask {
        StorageAsk::Read { stick, name } => StorageAnswer::Read(sticks.read(Read { stick, name })),
        StorageAsk::ReadCodes { stick, name } => {
            StorageAnswer::Codes(sticks.read_codes(ReadCodes { stick, name }))
        }
        StorageAsk::Write {
            stick,
            name,
            bytes,
            copies,
            replace,
            renamed,
            on_taken,
        } => StorageAnswer::Written(sticks.write(Write {
            stick,
            name,
            bytes,
            copies,
            replace,
            renamed,
            on_taken,
        })),
        StorageAsk::Wipe { stick, name } => StorageAnswer::Wiped(sticks.wipe(Wipe { stick, name })),
        StorageAsk::Space { stick } => StorageAnswer::Space(sticks.space(Space { stick })),
        StorageAsk::ZeroFree { stick, from } => {
            StorageAnswer::Zeroing(sticks.zero_free(ZeroFree { stick, from }))
        }
        StorageAsk::MakeEmpty { stick } => {
            StorageAnswer::Emptied(sticks.make_empty(MakeEmpty { stick }))
        }
        StorageAsk::Rename {
            stick,
            from,
            to,
            vaults,
        } => StorageAnswer::Renamed(sticks.rename(Rename {
            stick,
            from,
            to,
            vaults,
        })),
        StorageAsk::Name { stick, label } => {
            StorageAnswer::Named(sticks.name(Name { stick, label }))
        }
        StorageAsk::Hash { stick, name } => {
            StorageAnswer::Hashed(sticks.hash(Hash { stick, name }))
        }
    }
}

/// The answer of a product that has no sticks: `Unsupported`, in the variant the ask takes.
#[must_use]
pub fn unsupported(ask: &StorageAsk) -> StorageAnswer {
    let no = Failure::Unsupported;
    match ask {
        StorageAsk::Read { .. } => StorageAnswer::Read(Err(no)),
        StorageAsk::ReadCodes { .. } => StorageAnswer::Codes(Err(no)),
        StorageAsk::Write { .. } => StorageAnswer::Written(Err(no)),
        StorageAsk::Wipe { .. } => StorageAnswer::Wiped(Err(no)),
        StorageAsk::Space { .. } => StorageAnswer::Space(Err(no)),
        StorageAsk::ZeroFree { .. } => StorageAnswer::Zeroing(Err(no)),
        StorageAsk::MakeEmpty { .. } => StorageAnswer::Emptied(Err(no)),
        StorageAsk::Rename { .. } => StorageAnswer::Renamed(Err(Unrenamed::Failed(no))),
        StorageAsk::Name { .. } => StorageAnswer::Named(Err(NameRefused::Failed(no))),
        StorageAsk::Hash { .. } => StorageAnswer::Hashed(Err(no)),
    }
}

/// The answer of a product with no host files: `Unsupported`.
pub fn host_unsupported(ask: &HostAsk) -> HostAnswer {
    let no = Failure::Unsupported;
    match ask {
        HostAsk::Read { .. } => HostAnswer::Read(Err(no)),
        HostAsk::Save { .. } => HostAnswer::Saved(Err(no)),
        HostAsk::Open(_) => HostAnswer::Opened(Err(no)),
    }
}
