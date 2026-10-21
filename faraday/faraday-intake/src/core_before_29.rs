//! Bitcoin Core's `importdescriptors` file for Core before 29, which
//! takes no multipath descriptor (`docs/INTEROP.md` §5 I4): two
//! requests, the receive descriptor (`"internal": false`) and the change
//! descriptor (`"internal": true`), each the wallet's multipath
//! descriptor with `<0;1>` as `0` or `1`, as miniscript's
//! `into_single_descriptors` splits it, and each with its own checksum.
//! The origins are kept as the wallet writes them, so a wallet read from
//! Core's own `listdescriptors` gives back Core's own two strings.

use osk_bip::json;
use osk_bip::policy::{Template, WalletPolicy};

/// The multipath step every wallet with a descriptor derives through.
const MULTIPATH: &str = "<0;1>";

/// The wallet's receive and change descriptors, each with its checksum;
/// `None` for MuSig2, FROST and silent payments wallets, whose
/// descriptors Core before 29 does not read, and any descriptor without
/// `<0;1>`. Costs two checksums over the descriptor.
pub fn descriptors(policy: &WalletPolicy) -> Option<[String; 2]> {
    if !writes(policy) {
        return None;
    }
    let d = policy.to_descriptor();
    if !d.contains(MULTIPATH) {
        return None;
    }
    let one = |chain: &str| {
        let single = d.replace(MULTIPATH, chain);
        let sum = osk_bip::descriptor::descriptor_checksum(&single)?;
        Some(format!("{single}#{}", String::from_utf8_lossy(&sum)))
    };
    Some([one("0")?, one("1")?])
}

/// Whether [`file`] writes a file for the wallet: any but a MuSig2,
/// FROST or silent payments wallet.
pub fn writes(policy: &WalletPolicy) -> bool {
    policy.silent().is_none()
        && policy.record().is_none()
        && !matches!(
            policy.template(),
            Template::MuSig | Template::Threshold { .. } | Template::Silent
        )
}

/// The `importdescriptors` argument for Core before 29, as a file: each
/// request active, over Core's default range, scanning from the genesis
/// block (`"timestamp": 0`). `None` where [`descriptors`] is.
pub fn file(policy: &WalletPolicy) -> Option<String> {
    let [receive, change] = descriptors(policy)?;
    let request = |desc: &str, internal: bool| {
        format!(
            "  {{\n    \"desc\": {},\n    \"active\": true,\n    \"internal\": {internal},\n    \"range\": [0, {}],\n    \"timestamp\": 0\n  }}",
            json::quote(desc),
            osk_bip::core_import::RANGE_END,
        )
    };
    Some(format!(
        "[\n{},\n{}\n]\n",
        request(&receive, false),
        request(&change, true)
    ))
}
