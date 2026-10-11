//! Bitcoin Core's `importdescriptors` file (`docs/PLANNING.md` §16.114).
//!
//! A wallet is registered with Core by handing `importdescriptors` an
//! array of requests. One multipath descriptor covers both chains, so
//! one request is the whole wallet: no second `internal: true` entry,
//! which is what a coordinator writes when it has only single-path
//! descriptors.
//!
//! ```
//! use osk_bip::core_import::{Rescan, import_descriptors};
//!
//! let file = import_descriptors("wpkh(xpub…/<0;1>/*)#checksum", Rescan::Now);
//! assert!(file.contains("\"active\": true"));
//! assert!(file.contains("\"timestamp\": \"now\""));
//! ```
//!
//! The descriptor carries its own checksum, which Core requires, and
//! the file holds nothing secret: it is the wallet's public form.

use alloc::string::String;
use core::fmt::Write;

/// The last address index the import covers. Core's default gap limit
/// is 1 000 addresses, so a range of `[0, 999]` is a wallet's first
/// thousand of each chain.
pub const RANGE_END: u32 = 999;

/// Where Core starts scanning the chain for the wallet's coins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rescan {
    /// From the genesis block: finds coins the wallet already has, and
    /// reads the whole chain to do it. Core's `timestamp: 0`.
    Start,
    /// From now: nothing before this moment is looked for. Core's
    /// `timestamp: "now"`.
    Now,
}

impl Rescan {
    /// What the request's `timestamp` field is written as.
    fn timestamp(self) -> &'static str {
        match self {
            Rescan::Start => "0",
            Rescan::Now => "\"now\"",
        }
    }
}

/// The `importdescriptors` argument for `descriptor`, as a file.
///
/// `descriptor` is a checksummed multipath output descriptor, which is
/// what [`crate::policy::WalletPolicy::to_descriptor_checksummed`]
/// writes.
pub fn import_descriptors(descriptor: &str, rescan: Rescan) -> String {
    let mut out = String::new();
    // Writing into a String cannot fail.
    let _ = write!(
        out,
        "[\n  {{\n    \"desc\": \"{}\",\n    \"active\": true,\n    \"range\": [0, {}],\n    \"timestamp\": {}\n  }}\n]\n",
        escape(descriptor),
        RANGE_END,
        rescan.timestamp()
    );
    out
}

/// JSON's two mandatory escapes. A descriptor holds neither, being
/// letters, digits and `[]()<>;,/'*#`, but a string written into JSON is
/// escaped whatever is in it.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out
}
