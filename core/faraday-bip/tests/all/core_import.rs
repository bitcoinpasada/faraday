//! Bitcoin Core's `importdescriptors` file for Core before 29, against
//! what Core itself lists for a wallet (`listdescriptors`, in
//! `faraday/faraday-core/tests/vectors/interop/core/`): the receive and
//! change descriptors the file writes are Core's own strings, checksums
//! and all.

use faraday_bip::core_import::{import_descriptors_pair, pair_descriptors, writes_pair};
use faraday_bip::json::{self, Value};
use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::Compute;

const LISTED: &str = include_str!(
    "../../../../faraday/faraday-core/tests/vectors/interop/core/listdescriptors.json"
);

/// Core's descriptors, receive then change, for each of its wallets.
fn listed() -> Vec<(String, String)> {
    let v = json::parse(LISTED).unwrap();
    let all: Vec<(String, bool)> = v
        .get("descriptors")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .map(|d| {
            (
                d.get("desc").and_then(Value::as_str).unwrap().to_string(),
                d.get("internal") == Some(&Value::Bool(true)),
            )
        })
        .collect();
    all.iter()
        .filter(|(_, internal)| !internal)
        .map(|(receive, _)| {
            let change = receive.split('#').next().unwrap().replace("/0/*", "/1/*");
            let change = all
                .iter()
                .find(|(d, internal)| *internal && d.starts_with(&change))
                .unwrap()
                .0
                .clone();
            (receive.clone(), change)
        })
        .collect()
}

#[test]
fn the_pair_for_core_before_29_is_cores_own_receive_and_change_descriptors() {
    let c = Compute::unchecked();
    let pairs = listed();
    assert_eq!(pairs.len(), 4);
    for (receive, change) in pairs {
        let multipath = receive
            .split('#')
            .next()
            .unwrap()
            .replace("/0/*", "/<0;1>/*");
        let policy = WalletPolicy::parse_any(&c, &multipath).unwrap();
        assert!(writes_pair(&policy));
        assert_eq!(
            pair_descriptors(&policy),
            Some([receive.clone(), change.clone()])
        );
        let file = import_descriptors_pair(&policy).unwrap();
        let requests = json::parse(&file).unwrap();
        let requests = requests.as_array().unwrap();
        assert_eq!(requests.len(), 2);
        for (r, (desc, internal)) in requests.iter().zip([(&receive, false), (&change, true)]) {
            assert_eq!(r.get("desc").and_then(Value::as_str), Some(desc.as_str()));
            assert_eq!(r.get("internal"), Some(&Value::Bool(internal)));
            assert_eq!(r.get("active"), Some(&Value::Bool(true)));
            assert_eq!(r.get("timestamp").and_then(Value::as_u64), Some(0));
        }
    }
}
