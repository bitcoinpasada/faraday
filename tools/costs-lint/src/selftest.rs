//! `crypto-inventory --self-test`: the inventory over a small fixture
//! crate written to a temporary directory, as `tools/lint-guide.py
//! --self-test` does for the guide lint. It shows each rule of
//! `docs/CRYPTO-LEARN.md` §4.2 flagging what it should and nothing else,
//! the closure's input, the test functions and the call sites, and each
//! row of §5.1's table of what changes a body hash and what does not.
//! Standard library only.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::inventory::{self, Inventory};
use crate::sha256;

#[derive(Default)]
struct Checks {
    run: usize,
    failed: Vec<String>,
}

impl Checks {
    fn ok(&mut self, what: &str, cond: bool) {
        self.run += 1;
        if !cond {
            self.failed.push(what.to_string());
        }
    }

    fn eq<T: PartialEq + Debug>(&mut self, what: &str, got: T, want: T) {
        self.run += 1;
        if got != want {
            self.failed
                .push(format!("{what}: got {got:?}, wanted {want:?}"));
        }
    }
}

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Err(e) = fs::write(&path, text) {
        eprintln!("crypto-inventory: cannot write {}: {e}", path.display());
    }
}

fn manifest(name: &str, deps: &[&str]) -> String {
    let mut s = format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n\n[dependencies]\n");
    for d in deps {
        s.push_str(&format!("{d} = \"1\"\n"));
    }
    s.push_str("\n[dev-dependencies]\nbitcoin = \"1\"\n");
    s
}

// ------------------------------------------------------------------ vectors

fn sha256_vectors(c: &mut Checks) {
    let million = vec![b'a'; 1_000_000];
    let vectors: [(&str, &[u8], &str); 4] = [
        (
            "empty",
            b"",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ),
        (
            "abc",
            b"abc",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        ),
        (
            "two blocks",
            b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
        ),
        (
            "a million a",
            &million,
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
        ),
    ];
    for (name, data, want) in vectors {
        c.eq(
            &format!("SHA-256 of {name}"),
            sha256::hex(data).as_str(),
            want,
        );
    }
}

// -------------------------------------------------------------------- rules

const CRYPTO_LIB: &str = r"
pub mod work {
    pub struct Compute;
}
pub struct Secret;
pub fn sha256(data: &[u8]) -> [u8; 32] {
    sha2::Sha256::digest(data).into()
}
";

const BIP_LIB: &str = r"
use faraday_crypto::work::Compute;
use faraday_crypto::Secret;
use sha2::{Digest, Sha256};

mod inner;

pub fn token_only(_c: &Compute) -> u8 { 0 }
pub fn secret_only(_s: Secret) {}
pub fn alias_hash(data: &[u8]) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize().to_vec()
}
pub fn full_path_hash(data: &[u8]) {
    let _ = sha2::Sha512::digest(data);
}
pub fn macro_hash(data: &[u8]) -> bool {
    assert!(sha2::Sha256::digest(data).len() == 32);
    true
}
pub fn via_crypto(data: &[u8]) -> [u8; 32] {
    faraday_crypto::sha256(data)
}
pub fn token_and_hash(_c: &Compute) {
    let _ = Sha256::new();
}
pub fn wrapper(d: &[u8]) -> usize {
    Helper::run(d).len()
}
pub fn method_wrapper(h: &Helper) -> usize {
    h.run_method().len()
}
pub fn path_wrapper(d: &[u8]) -> usize {
    inner::deep(d)
}
fn private_wrapper(d: &[u8]) -> usize {
    alias_hash(d).len()
}
pub fn plain() -> u8 { 1 }
pub fn local_shadow(alias_hash: u8) -> u8 { alias_hash }
pub enum Reveal { A }
pub fn reveal_elsewhere(_w: Reveal) {}

pub struct Helper;
impl Helper {
    fn run(d: &[u8]) -> Vec<u8> { alias_hash(d) }
    pub fn run_method(&self) -> Vec<u8> { Self::run(&[]) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn passes() {}
    #[test]
    #[ignore]
    fn slow() {}
    fn helper_in_tests(_c: &Compute) {
        let _ = Sha256::new();
    }
}
";

const BIP_INNER: &str = r"
pub fn deep(d: &[u8]) -> usize { crate::alias_hash(d).len() }
";

const KEYS_LIB: &str = r"
pub enum Reveal { A }
pub struct Revealed;
pub struct Held<T>(pub T);
pub struct Keys;
impl Keys {
    pub fn takes_reveal(&self, _w: Reveal) {}
    pub fn returns_held(&self) -> Option<Held<u8>> { None }
    pub fn lend(&self) -> Revealed { Revealed }
    pub fn nothing(&self) {}
}
fn private_reveal(_w: Reveal) {}
pub fn hash_it(d: &[u8]) -> [u8; 32] { faraday_crypto::sha256(d) }
pub fn uses_reveal(k: &Keys) { k.takes_reveal(Reveal::A) }
";

const CORE_LIB: &str = r"
use faraday_crypto::work::Compute;
use faraday_keys::Keys;

mod other;

pub fn passes(_c: &Compute) {}
fn hashes_here() { let _ = sha2::Sha256::new(); }
pub fn wraps() { hashes_here() }
pub fn screen_path() { let _ = faraday_keys::hash_it(&[]); }
pub fn screen_bip() { let _ = fx_bip::alias_hash(&[]); }
pub fn screen_type(k: &Keys) { Keys::takes_reveal(k, faraday_keys::Reveal::A) }
pub fn screen_method(k: &Keys) { let _ = k.returns_held(); k.nothing(); }
";

const PLAIN_LIB: &str = r"
use faraday_crypto::work::Compute;
pub fn takes(_c: &Compute) { let _ = sha2::Sha256::new(); }
#[test]
fn plain_test() {}
";

fn write_fixture(root: &Path) {
    write(
        root,
        "core/faraday-crypto/Cargo.toml",
        &manifest("faraday-crypto", &["sha2"]),
    );
    write(root, "core/faraday-crypto/src/lib.rs", CRYPTO_LIB);
    write(
        root,
        "core/fx-bip/Cargo.toml",
        &manifest("fx-bip", &["sha2", "faraday-crypto"]),
    );
    write(root, "core/fx-bip/src/lib.rs", BIP_LIB);
    write(root, "core/fx-bip/src/inner.rs", BIP_INNER);
    write(
        root,
        "faraday/faraday-keys/Cargo.toml",
        &manifest("faraday-keys", &["faraday-crypto"]),
    );
    write(root, "faraday/faraday-keys/src/lib.rs", KEYS_LIB);
    write(
        root,
        "faraday/faraday-core/Cargo.toml",
        &manifest("faraday-core", &["faraday-crypto"]),
    );
    write(root, "faraday/faraday-core/src/lib.rs", CORE_LIB);
    // A method call in a file that names no type of that name: no site.
    write(
        root,
        "faraday/faraday-core/src/other.rs",
        "pub fn unnamed(x: u8) { let _ = x.lend(); }\n",
    );
    write(
        root,
        "faraday/faraday-plain/Cargo.toml",
        &manifest("faraday-plain", &[]),
    );
    write(root, "faraday/faraday-plain/src/lib.rs", PLAIN_LIB);
    write(
        root,
        "faraday/faraday-plain/tests/it.rs",
        "#[test]\nfn integration() {}\n",
    );
}

fn rules_by_id(inv: &Inventory) -> BTreeMap<String, Vec<u8>> {
    (0..inv.funcs.len())
        .filter(|&i| !inv.funcs[i].rules.is_empty())
        .map(|i| (inv.id(i), inv.funcs[i].rules.iter().copied().collect()))
        .collect()
}

fn expected_rules() -> BTreeMap<String, Vec<u8>> {
    let bip = "core/fx-bip/src/lib.rs";
    let keys = "faraday/faraday-keys/src/lib.rs";
    let want: Vec<(String, Vec<u8>)> = vec![
        ("core/faraday-crypto/src/lib.rs::sha256".into(), vec![3]),
        (format!("{bip}::token_only"), vec![1]),
        (format!("{bip}::alias_hash"), vec![3]),
        (format!("{bip}::full_path_hash"), vec![3]),
        (format!("{bip}::macro_hash"), vec![3]),
        (format!("{bip}::via_crypto"), vec![3]),
        (format!("{bip}::token_and_hash"), vec![1, 3]),
        (format!("{bip}::wrapper"), vec![4]),
        (format!("{bip}::method_wrapper"), vec![4]),
        (format!("{bip}::path_wrapper"), vec![4]),
        (format!("{bip}::Helper::run_method"), vec![4]),
        ("core/fx-bip/src/inner.rs::deep".into(), vec![4]),
        (format!("{keys}::Keys::takes_reveal"), vec![2]),
        (format!("{keys}::Keys::returns_held"), vec![2]),
        (format!("{keys}::Keys::lend"), vec![2]),
        (format!("{keys}::private_reveal"), vec![2]),
        (format!("{keys}::hash_it"), vec![3]),
        (format!("{keys}::uses_reveal"), vec![4]),
        (
            "faraday/faraday-core/src/lib.rs::hashes_here".into(),
            vec![3],
        ),
    ];
    want.into_iter().collect()
}

fn rule_checks(c: &mut Checks, root: &Path) {
    let inv = inventory::scan(root);
    let got = rules_by_id(&inv);
    let want = expected_rules();
    for (id, rules) in &want {
        c.eq(&format!("rules of {id}"), got.get(id), Some(rules));
    }
    let extra: Vec<&String> = got.keys().filter(|k| !want.contains_key(*k)).collect();
    c.eq(
        "functions flagged that no rule should flag",
        extra,
        Vec::new(),
    );

    let names: BTreeSet<&str> = inv.funcs.iter().map(|f| f.name.as_str()).collect();
    c.ok(
        "a function under #[cfg(test)] is left out",
        !names.contains("helper_in_tests"),
    );
    c.ok(
        "a private function that reaches a hash is not rule 4",
        !got.contains_key("core/fx-bip/src/lib.rs::private_wrapper"),
    );
    c.ok(
        "a local named like a function is not a call",
        !got.contains_key("core/fx-bip/src/lib.rs::local_shadow"),
    );

    // The closure's input: a private helper is printed with no rules, and a
    // function nothing cryptographic reaches is not.
    let printed = inv.lines(false).join("\n");
    let run_line = printed
        .lines()
        .find(|l| l.contains("\"id\":\"core/fx-bip/src/lib.rs::Helper::run\""));
    c.ok(
        "a callee of a cryptographic function is printed without rules",
        run_line.is_some_and(|l| l.contains("\"rules\":[]")),
    );
    c.ok(
        "a function nothing cryptographic reaches is not printed",
        !printed.contains("private_wrapper") && !printed.contains("\"name\":\"plain\""),
    );
    let wrapper = inv.funcs.iter().position(|f| f.name == "wrapper");
    let run = inv.funcs.iter().position(|f| f.name == "Helper::run");
    c.ok(
        "a path call is a callee by its last two segments",
        matches!((wrapper, run), (Some(w), Some(r)) if inv.funcs[w].callees.contains(&r)),
    );
    let mw = inv.funcs.iter().position(|f| f.name == "method_wrapper");
    let rm = inv
        .funcs
        .iter()
        .position(|f| f.name == "Helper::run_method");
    c.ok(
        "a method call is a callee by its name",
        matches!((mw, rm), (Some(a), Some(b)) if inv.funcs[a].callees.contains(&b)),
    );
    let core_fn = inv.funcs.iter().find(|f| f.name == "screen_path");
    c.ok(
        "a call into another workspace crate is listed",
        core_fn.is_some_and(|f| f.cross.contains("faraday-keys::hash_it")),
    );
}

fn test_checks(c: &mut Checks, root: &Path) {
    let inv = inventory::scan(root);
    let got: Vec<String> = inv.test_lines();
    let want = [
        "core/fx-bip/src/lib.rs tests::passes ignored=false cfg=cfg(test)",
        "core/fx-bip/src/lib.rs tests::slow ignored=true cfg=cfg(test)",
        "faraday/faraday-plain/src/lib.rs plain_test ignored=false cfg=",
        "faraday/faraday-plain/tests/it.rs integration ignored=false cfg=",
    ];
    c.eq(
        "test functions",
        got,
        want.iter().map(|s| s.to_string()).collect(),
    );
}

fn site_checks(c: &mut Checks, root: &Path) {
    let inv = inventory::scan(root);
    let got: BTreeSet<String> = inv.site_lines().into_iter().collect();
    let core = "faraday/faraday-core/src/lib.rs";
    let want: BTreeSet<String> = [
        format!("{core}::screen_path -> faraday/faraday-keys/src/lib.rs::hash_it path"),
        format!("{core}::screen_bip -> core/fx-bip/src/lib.rs::alias_hash path"),
        format!("{core}::screen_type -> faraday/faraday-keys/src/lib.rs::Keys::takes_reveal path"),
        format!(
            "{core}::screen_method -> faraday/faraday-keys/src/lib.rs::Keys::returns_held method"
        ),
    ]
    .into_iter()
    .collect();
    c.eq("call sites in faraday-core", got, want);
}

// --------------------------------------------------------------- body hashes

/// A crate `hx` with the given files, scanned; each function's body hash.
fn hashes(root: &Path, n: usize, files: &[(&str, &str)]) -> BTreeMap<String, String> {
    let dir = root.join(format!("hash{n}"));
    write(&dir, "core/hx/Cargo.toml", &manifest("hx", &["sha2"]));
    for (name, text) in files {
        write(&dir, &format!("core/hx/src/{name}"), text);
    }
    let inv = inventory::scan(&dir);
    inv.funcs
        .iter()
        .map(|f| (f.name.clone(), f.hash.clone()))
        .collect()
}

const BASE: &str =
    "pub fn body(a: u32) -> u32 {\n    let x = a + 1;\n    x * 2\n}\npub fn other() -> u32 { 7 }\n";

/// One row of §5.1's table: the crate's files, the function looked at, and
/// whether its hash is the base's.
struct Row {
    what: &'static str,
    files: &'static [(&'static str, &'static str)],
    name: &'static str,
    same: bool,
}

const ROWS: &[Row] = &[
    Row {
        what: "fmt, a line wrapped, blank lines",
        files: &[(
            "lib.rs",
            "pub fn   body( a : u32 )->u32{ let x=a\n+1;\n\n\n x*2 }\npub fn other()->u32{7}\n",
        )],
        name: "body",
        same: true,
    },
    Row {
        what: "a // comment, a /* */ comment, doc comments",
        files: &[(
            "lib.rs",
            "/// Doc.\n/// More.\npub fn body(a: u32) -> u32 {\n    // note\n    /// doc on a statement\n    let x = a + 1; /* note */\n    x * 2\n}\npub fn other() -> u32 { 7 }\n",
        )],
        name: "body",
        same: true,
    },
    Row {
        what: "moved within its file",
        files: &[(
            "lib.rs",
            "pub fn other() -> u32 { 7 }\npub fn body(a: u32) -> u32 {\n    let x = a + 1;\n    x * 2\n}\n",
        )],
        name: "body",
        same: true,
    },
    Row {
        what: "moved to another file",
        files: &[
            ("lib.rs", "mod moved;\npub fn other() -> u32 { 7 }\n"),
            (
                "moved.rs",
                "pub fn body(a: u32) -> u32 {\n    let x = a + 1;\n    x * 2\n}\n",
            ),
        ],
        name: "body",
        same: true,
    },
    Row {
        what: "the function renamed",
        files: &[(
            "lib.rs",
            "pub fn renamed(a: u32) -> u32 {\n    let x = a + 1;\n    x * 2\n}\npub fn other() -> u32 { 7 }\n",
        )],
        name: "renamed",
        same: true,
    },
    Row {
        what: "a trailing comma added or dropped",
        files: &[(
            "lib.rs",
            "pub fn body(a: u32,) -> u32 {\n    let x = a + 1;\n    x * 2\n}\npub fn other() -> u32 { 7 }\n",
        )],
        name: "body",
        same: true,
    },
    Row {
        what: "a local variable renamed",
        files: &[(
            "lib.rs",
            "pub fn body(a: u32) -> u32 {\n    let y = a + 1;\n    y * 2\n}\n",
        )],
        name: "body",
        same: false,
    },
    Row {
        what: "the body changed",
        files: &[(
            "lib.rs",
            "pub fn body(a: u32) -> u32 {\n    let x = a + 1;\n    x * 3\n}\n",
        )],
        name: "body",
        same: false,
    },
    Row {
        what: "an attribute other than doc added",
        files: &[(
            "lib.rs",
            "#[inline]\npub fn body(a: u32) -> u32 {\n    let x = a + 1;\n    x * 2\n}\n",
        )],
        name: "body",
        same: false,
    },
    Row {
        what: "the signature changed",
        files: &[(
            "lib.rs",
            "pub fn body(a: u64) -> u32 {\n    let x = a + 1;\n    x * 2\n}\n",
        )],
        name: "body",
        same: false,
    },
];

fn hash_rows(c: &mut Checks, root: &Path) {
    let base = hashes(root, 0, &[("lib.rs", BASE)]);
    let want = base.get("body").cloned();
    c.ok("the base function has a hash", want.is_some());
    for (n, row) in ROWS.iter().enumerate() {
        let got = hashes(root, n + 1, row.files).get(row.name).cloned();
        if row.same {
            c.eq(&format!("same hash: {}", row.what), got, want.clone());
        } else {
            c.ok(
                &format!("different hash: {}", row.what),
                got.is_some() && got != want,
            );
        }
    }
    callee_changed(c, root);
}

/// A callee changed: the caller's own hash stays, the callee's changes,
/// and the caller names the callee, so a closure over it sees both.
fn callee_changed(c: &mut Checks, root: &Path) {
    let calls = |n: usize, other: &str| {
        let src = format!(
            "pub fn body(a: u32) -> u32 {{ other() + a }}\npub fn other() -> u32 {{ {other} }}\n"
        );
        hashes(root, 100 + n, &[("lib.rs", &src)])
    };
    let (a, b) = (calls(0, "7"), calls(1, "8"));
    c.eq(
        "a callee changed: the caller's hash",
        a.get("body"),
        b.get("body"),
    );
    c.ok(
        "a callee changed: the callee's hash",
        a.get("other") != b.get("other"),
    );
    let inv = inventory::scan(&root.join("hash100"));
    let body = inv.funcs.iter().position(|f| f.name == "body");
    let other = inv.funcs.iter().position(|f| f.name == "other");
    c.ok(
        "a callee changed: the caller names it",
        matches!((body, other), (Some(x), Some(y)) if inv.funcs[x].callees.contains(&y)),
    );
}

// ---------------------------------------------------------------------- run

/// Runs every check; the exit status is 1 when one failed.
pub fn run() -> ExitCode {
    let root: PathBuf =
        std::env::temp_dir().join(format!("crypto-inventory-selftest-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let mut c = Checks::default();
    sha256_vectors(&mut c);
    let fixture = root.join("fixture");
    write_fixture(&fixture);
    rule_checks(&mut c, &fixture);
    test_checks(&mut c, &fixture);
    site_checks(&mut c, &fixture);
    hash_rows(&mut c, &root);
    let _ = fs::remove_dir_all(&root);
    if c.failed.is_empty() {
        println!("crypto-inventory self-test: ok ({} checks)", c.run);
        ExitCode::SUCCESS
    } else {
        for f in &c.failed {
            println!("crypto-inventory self-test: FAILED {f}");
        }
        println!(
            "crypto-inventory self-test: {} of {} checks failed",
            c.failed.len(),
            c.run
        );
        ExitCode::FAILURE
    }
}
