//! OpenTimestamps proofs, `faraday_ots` (`docs/decisions/F230.md`): a proof
//! read into the paths from the file's digest to each attestation, a
//! header's checks, and a proof walked against a header.

use faraday_crypto::work::Compute;
use faraday_ots::{Attestation, Entry, Header, Network, Node, Op, Outcome, Proof};
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes, hex};

fn err<E: core::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

fn op_text(op: &Op) -> String {
    match op {
        Op::Append(a) => format!("append {}", hex(a)),
        Op::Prepend(a) => format!("prepend {}", hex(a)),
        Op::Hash(h) => h.name().to_string(),
        Op::Reverse => "reverse".to_string(),
        Op::Hexlify => "hexlify".to_string(),
    }
}

fn attestation_text(a: &Attestation) -> String {
    match a {
        Attestation::Pending { uri } => format!("pending {uri}"),
        Attestation::Bitcoin { height } => format!("bitcoin {height}"),
        Attestation::Litecoin { height } => format!("litecoin {height}"),
        Attestation::Ethereum { height } => format!("ethereum {height}"),
        Attestation::Unknown { tag, payload } => {
            format!("unknown {} {}", hex(tag), hex(payload))
        }
    }
}

/// Every attestation with the operations that lead to it, and whether a
/// node holds the same attestation twice.
fn leaves(node: &Node, path: &mut Vec<String>, out: &mut Vec<String>, repeated: &mut bool) {
    for (i, entry) in node.entries.iter().enumerate() {
        if matches!(entry, Entry::Attestation(_)) && node.entries[..i].contains(entry) {
            *repeated = true;
        }
        match entry {
            Entry::Attestation(a) => {
                let mut line = path.join(" | ");
                if !line.is_empty() {
                    line.push(' ');
                }
                out.push(format!("{line}=> {}", attestation_text(a)));
            }
            Entry::Op(op, next) => {
                path.push(op_text(op));
                leaves(next, path, out, repeated);
                path.pop();
            }
        }
    }
}

fn outcome(o: &Outcome) -> Value {
    match o {
        Outcome::BitcoinMatched {
            block_hash,
            height,
            time,
        } => json!({"kind": "bitcoin", "height": height, "result": "match",
                    "time": time, "block_hash": hex(block_hash)}),
        Outcome::BitcoinMismatch { height } => {
            json!({"kind": "bitcoin", "height": height, "result": "mismatch"})
        }
        Outcome::BitcoinNotSha256 {
            height,
            op,
            reaches_header,
        } => json!({"kind": "bitcoin", "height": height, "result": "not-sha256",
                    "op": op.name(), "reaches": reaches_header}),
        Outcome::Pending { uri } => json!({"kind": "pending", "uri": uri}),
        Outcome::Litecoin { height } => json!({"kind": "litecoin", "height": height}),
        Outcome::Ethereum { height } => json!({"kind": "ethereum", "height": height}),
        Outcome::Unknown { tag } => json!({"kind": "unknown", "tag": hex(tag)}),
    }
}

/// Handles `ots.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    let c = Compute::unchecked();
    match name {
        // The proof's file hash, its digest and each path to an attestation.
        "read" => {
            let proof = Proof::parse(&c, &bytes(req, "proof")).map_err(err)?;
            let mut out = Vec::new();
            let mut repeated = false;
            leaves(proof.tree(), &mut Vec::new(), &mut out, &mut repeated);
            Ok(json!({
                "file_op": proof.file_op().name(),
                "digest": hex(proof.file_digest()),
                "leaves": out,
                "repeated_attestation": repeated,
            }))
        }
        // A header read: its hash, merkle root and time.
        "header" => {
            let h = Header::parse(Network::Regtest, &bytes(req, "header")).map_err(err)?;
            Ok(json!({
                "block_hash": hex(&h.block_hash()),
                "merkle_root": hex(&h.merkle_root()),
                "time": h.time(),
            }))
        }
        // What each attestation of a proof shows against a header.
        "verify" => {
            let proof = Proof::parse(&c, &bytes(req, "proof")).map_err(err)?;
            let header = Header::parse(Network::Regtest, &bytes(req, "header")).map_err(err)?;
            let report = proof.verify(&c, &bytes(req, "digest"), &header);
            let paths: Vec<Value> = report.paths.iter().map(outcome).collect();
            Ok(json!({
                "file_matches": report.file_matches,
                "committed": report.committed(),
                "paths": paths,
            }))
        }
        _ => Err(format!("unknown ots op {name}")),
    }
}
