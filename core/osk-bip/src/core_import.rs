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
//!
//! Core before 29 takes no multipath descriptor, so
//! [`import_descriptors_pair`] writes the receive and the change
//! descriptor as two requests (Faraday's since the fork,
//! `docs/UPSTREAM.md` §3).

use alloc::string::String;
use core::fmt::Write;

use crate::policy::{Template, WalletPolicy};

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
        "[\n  {{\n    \"desc\": {},\n    \"active\": true,\n    \"range\": [0, {}],\n    \"timestamp\": {}\n  }}\n]\n",
        crate::json::quote(descriptor),
        RANGE_END,
        rescan.timestamp()
    );
    out
}

/// The multipath step every wallet with a descriptor derives through.
const MULTIPATH: &str = "<0;1>";

/// The wallet's receive and change descriptors for Bitcoin Core before
/// 29, which takes no multipath descriptor (`docs/INTEROP.md` §5 I4):
/// the wallet's descriptor with `<0;1>` as `0` or `1`, as miniscript's
/// `into_single_descriptors` splits it, each with its own checksum. The
/// origins are kept as the wallet writes them, so a wallet read from
/// Core's own `listdescriptors` gives back Core's own two strings.
/// `None` for MuSig2, FROST and silent payments wallets, whose
/// descriptors Core before 29 does not read, and any descriptor without
/// `<0;1>`. Costs two checksums over the descriptor.
pub fn pair_descriptors(policy: &WalletPolicy) -> Option<[String; 2]> {
    if !writes_pair(policy) {
        return None;
    }
    let d = policy.to_descriptor();
    if !d.contains(MULTIPATH) {
        return None;
    }
    let one = |chain: &str| {
        let single = d.replace(MULTIPATH, chain);
        let sum = crate::descriptor::descriptor_checksum(&single)?;
        let sum = core::str::from_utf8(&sum).ok()?;
        Some(alloc::format!("{single}#{sum}"))
    };
    Some([one("0")?, one("1")?])
}

/// Whether [`import_descriptors_pair`] writes a file for the wallet: any
/// but a MuSig2, FROST or silent payments wallet.
pub fn writes_pair(policy: &WalletPolicy) -> bool {
    policy.silent().is_none()
        && policy.record().is_none()
        && !matches!(
            policy.template(),
            Template::MuSig | Template::Threshold { .. } | Template::Silent
        )
}

/// The `importdescriptors` argument for Core before 29, as a file: two
/// requests, the receive descriptor (`"internal": false`) and the change
/// descriptor (`"internal": true`), each active, over Core's default
/// range, scanning from the genesis block (`"timestamp": 0`). `None`
/// where [`pair_descriptors`] is.
pub fn import_descriptors_pair(policy: &WalletPolicy) -> Option<String> {
    let [receive, change] = pair_descriptors(policy)?;
    let request = |desc: &str, internal: bool| {
        alloc::format!(
            "  {{\n    \"desc\": {},\n    \"active\": true,\n    \"internal\": {internal},\n    \"range\": [0, {}],\n    \"timestamp\": 0\n  }}",
            crate::json::quote(desc),
            RANGE_END,
        )
    };
    Some(alloc::format!(
        "[\n{},\n{}\n]\n",
        request(&receive, false),
        request(&change, true)
    ))
}
