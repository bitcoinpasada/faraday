//! Every file under `tests/vectors/interop/` gives what it gave when its
//! readers moved into `core/` (`docs/history/UPSTREAM.md` §3, 2026-10-20): the
//! same kind in Files, the same wallet with the same first address or the
//! same refusal, the same keys for Create's slots, the same name. The
//! outcomes are `tests/vectors/interop/outcomes.txt`; a reader that
//! changes one fails here. When a change is meant (a reader made
//! stricter, which F65 allows), run this test with
//! `FARADAY_WRITE_OUTCOMES=1` and read the difference before keeping it.

use faraday_core::create::{NewKind, key_for, read_keys};
use faraday_core::wallet::{classify, read_wallet, same_wallet};

/// What Faraday makes of one file, a line per question.
fn outcome(path: &str, bytes: &[u8]) -> String {
    let c = faraday_core::testkit::compute();
    let text = String::from_utf8_lossy(bytes);
    let mut out = format!("== {path}\n");
    out.push_str(&format!("kind {:?}\n", classify(&c, path, bytes)));
    match read_wallet(&c, &text) {
        Ok(p) => {
            out.push_str(&format!("wallet {}\n", same_wallet(&p)));
            match faraday_intake::wallet::first_address(&c, &p) {
                Some((net, a)) => out.push_str(&format!("first {a} {net:?}\n")),
                None => out.push_str("first none\n"),
            }
        }
        Err(e) => out.push_str(&format!("refused {}\n", e.text())),
    }
    for k in read_keys(&text) {
        out.push_str(&format!("key {k}\n"));
    }
    for kind in NewKind::ALL {
        if let Some(k) = key_for(kind, &text) {
            out.push_str(&format!("slot {kind:?} {k}\n"));
        }
    }
    if let Some(l) = faraday_intake::wallet::json_label(&text) {
        out.push_str(&format!("label {l}\n"));
    }
    if let Ok((_, Some(n))) = osk_bip::multisig_config::parse_named(&c, &text) {
        out.push_str(&format!("config name {n}\n"));
    }
    out.push_str(&format!(
        "rebuilt {}\n",
        faraday_core::restore::readable(&c, &text)
    ));
    out
}

/// Every file, sorted by path.
fn all() -> String {
    let root = format!("{}/tests/vectors/interop", env!("CARGO_MANIFEST_DIR"));
    let mut paths = Vec::new();
    let mut dirs = vec![std::path::PathBuf::from(&root)];
    while let Some(d) = dirs.pop() {
        for e in std::fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                dirs.push(p);
            } else {
                paths.push(p);
            }
        }
    }
    paths.sort();
    let mut out = String::new();
    for p in paths {
        let rel = p
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        if rel == "README.md" || rel == "outcomes.txt" {
            continue;
        }
        out.push_str(&outcome(&rel, &std::fs::read(&p).unwrap()));
    }
    out
}

#[test]
fn every_interop_file_reads_as_it_did_before_its_readers_moved_into_core() {
    let path = format!(
        "{}/tests/vectors/interop/outcomes.txt",
        env!("CARGO_MANIFEST_DIR")
    );
    let now = all();
    if std::env::var_os("FARADAY_WRITE_OUTCOMES").is_some() {
        std::fs::write(&path, &now).unwrap();
        return;
    }
    let then = std::fs::read_to_string(&path).unwrap();
    for (a, b) in then.lines().zip(now.lines()) {
        assert_eq!(a, b, "first difference from outcomes.txt");
    }
    assert_eq!(then, now);
}
