//! The secret-types check (`docs/SECRET-LEAKS.md` §6): a type that holds
//! a secret derives none of `Clone`, `PartialEq` or `Debug` and has no
//! `Display` impl, and a type with the name of a secret one derives no
//! `Debug`. The files read are those `tools/lint-secrets.sh` rules 1 and
//! 3 read: see [`debug_scope`] and [`type_scope`].
//!
//! A secret type is a `struct` or `enum` whose fields or generics name a
//! secret type (`tools/secret-names.txt`), that derives `Zeroize` or
//! `ZeroizeOnDrop`, or that has an `impl Drop`, `Zeroize` or
//! `ZeroizeOnDrop` in its file. One whose fields name a secret type of
//! its own crate (a name the crate defines once) and that has no `Drop`
//! of its own is secret too, repeated until nothing new is found.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use syn::visit::{self, Visit};

use crate::walk::{flatten, last_ident, parse, rel, rust_files};

/// A refusal: file, line, what.
pub type Hit = (String, usize, String);

/// Types named like a secret one that derive no `Debug`, unless listed in
/// [`NAMED_OK`].
const NAMED: &[&str] = &[
    "Secret",
    "Mnemonic",
    "MnemonicBytes",
    "Seed",
    "Entropy",
    "MasterKey",
    "DerivedKey",
    "PinEntry",
    "Session",
    "Key",
    "Item",
];

/// Types that carry the name of a secret type and hold none; each is public
/// data, and the exception is that file and name alone:
///
/// - slip132.rs `Key`: an extended public key, decoded
/// - fonts/mod.rs `Key`: a glyph cache key (family, size, code point)
/// - shell-api `Key`: a physical keyboard key
/// - backup_flow `Sealed`: the hash and name of a sealed vault file
/// - plan `Seed`: a seed as the plan sees it: name and flags
/// - plan `Item`: one line of the checklist
const NAMED_OK: &[(&str, &str)] = &[
    ("core/faraday-bip/src/slip132.rs", "Key"),
    ("core/faraday-draw/src/fonts/mod.rs", "Key"),
    ("core/faraday-shell-api/src/lib.rs", "Key"),
    ("faraday/faraday-core/src/backup_flow/mod.rs", "Sealed"),
    ("faraday/faraday-plan/src/lib.rs", "Seed"),
    ("faraday/faraday-plan/src/lib.rs", "Item"),
];

/// The types that keep a derive: file, type, the derives it excepts
/// (`Clone`, `PartialEq`, `Debug`, `Display`; any other derive on it is
/// still refused), and why. "Fixed in a later change" means the type's
/// callers copy, compare or print it and are not yet changed to a function
/// of the type that does.
const KEPT: &[(&str, &str, &str, &str)] = &[
    (
        "faraday/faraday-core/src/codex32/mod.rs",
        "DieRow",
        "Clone,PartialEq,Debug",
        "one die's two values, a Copy element of wiped buffers that the tour and the tests compare and print",
    ),
    (
        "core/faraday-shell-api/src/lib.rs",
        "EntropyBytes",
        "Clone",
        "Event is Clone and the snapshot and replay shells copy events",
    ),
    (
        "core/faraday-shell-api/src/lib.rs",
        "Event",
        "Clone,PartialEq",
        "its only wiping part is EntropyBytes, which wipes on drop and compares without a branch on a byte, so the derived copy and comparison are its own; the snapshot and replay shells copy events and the tests compare them",
    ),
    (
        "faraday/faraday-core/src/backup_flow/mod.rs",
        "Live",
        "Debug",
        "holds Writing, whose Debug is its own and prints the names and sizes of what it holds; its copy is its own",
    ),
    (
        "faraday/faraday-core/src/backup_flow/mod.rs",
        "BackupFlow",
        "Clone,Debug",
        "holds Live, whose copy is its own and whose Debug is Writing's own",
    ),
    (
        "faraday/faraday-core/src/backup_flow/mod.rs",
        "Reading",
        "Clone,Debug",
        "holds ReadFiles, whose Clone copies into wiping buffers and whose Debug prints a count only; a copy of a reading is taken to work on while the flow changes",
    ),
    (
        "faraday/faraday-core/src/dice_entry/screen.rs",
        "Ask",
        "Clone,Copy",
        "for it is Copy: it borrows the dice entry and holds no copy of it",
    ),
    (
        "faraday/faraday-core/src/transfer/mod.rs",
        "TransferArea",
        "Debug",
        "holds SendText, whose Debug is its own and prints lengths only",
    ),
    (
        "faraday/faraday-keys/src/vault/mod.rs",
        "Records",
        "Clone,Copy",
        "for it is Copy: a view of an open vault's records that holds references, not bytes",
    ),
    (
        "faraday/faraday-pgp/src/message.rs",
        "Message",
        "Debug",
        "holds Body, whose Debug is its own and prints a length only",
    ),
    (
        "faraday/faraday-vault/src/records.rs",
        "Record",
        "Debug",
        "holds Field, whose Debug is its own and prints a number and a length only",
    ),
    (
        "faraday/faraday-vault/src/contents.rs",
        "Refusal",
        "Debug,PartialEq",
        "holds KeyName, whose Debug and PartialEq are its own, written by hand over the name as text, not derived over its bytes",
    ),
    (
        "faraday/faraday-vault/src/qr.rs",
        "Error",
        "Debug,PartialEq",
        "holds Refusal, whose Debug and PartialEq come from KeyName's own",
    ),
];

/// The names in `tools/secret-names.txt`.
pub struct SecretNames {
    exact: BTreeSet<String>,
    prefixes: Vec<String>,
}

impl SecretNames {
    /// The names in the text of the list: one a line, `#` comments, a
    /// trailing `*` for a prefix.
    pub fn parse(text: &str) -> Self {
        let mut exact = BTreeSet::new();
        let mut prefixes = Vec::new();
        for line in text.lines() {
            let l = line.trim();
            if l.is_empty() || l.starts_with('#') {
                continue;
            }
            match l.strip_suffix('*') {
                Some(p) => prefixes.push(p.to_string()),
                None => {
                    exact.insert(l.to_string());
                }
            }
        }
        Self { exact, prefixes }
    }

    fn matches(&self, id: &str) -> bool {
        self.exact.contains(id)
            || self.prefixes.iter().any(|p| {
                id.strip_prefix(p.as_str())
                    .is_some_and(|rest| rest.chars().all(|c| c.is_ascii_alphabetic()))
            })
    }
}

/// Where rule 1's output macros and this check's `Debug` on a named type
/// are read: every file under `core/*/src/` and the source of faraday-core,
/// faraday-plan, faraday-paper, faraday-host and faraday-core-wallet.
pub fn debug_scope(r: &str) -> bool {
    (r.starts_with("core/") && r.contains("/src/"))
        || [
            "faraday/faraday-core/src/",
            "faraday/faraday-plan/src/",
            "faraday/faraday-paper/src/",
            "faraday/faraday-host/src/",
            "faraday/faraday-core-wallet/src/",
        ]
        .iter()
        .any(|p| r.starts_with(p))
}

/// Where a type that holds a secret is looked for: every source file under
/// `core/` and `faraday/` but tests, examples, benches, the Starter, the
/// shells, tools and fuzz.
pub fn type_scope(r: &str) -> bool {
    (r.starts_with("core/") || r.starts_with("faraday/"))
        && r.contains("/src/")
        && !["/tests/", "/examples/", "/benches/"]
            .iter()
            .any(|d| r.contains(d))
        && ![
            "faraday/faraday-starter",
            "faraday/shells/",
            "faraday/tools/",
            "faraday/fuzz/",
        ]
        .iter()
        .any(|p| r.starts_with(p))
}

/// The refusals over the tree under `root`.
pub fn check(root: &Path) -> Vec<Hit> {
    let names = match fs::read_to_string(root.join("tools/secret-names.txt")) {
        Ok(t) => SecretNames::parse(&t),
        Err(e) => {
            return vec![(
                "tools/secret-names.txt".into(),
                1,
                format!("cannot be read: {e}"),
            )];
        }
    };
    let mut files = Vec::new();
    let mut hits = Vec::new();
    for dir in ["core", "faraday"] {
        for p in rust_files(&root.join(dir)) {
            let r = rel(root, &p);
            if !debug_scope(&r) && !type_scope(&r) {
                continue;
            }
            match parse(&p) {
                Some(ast) => files.push((r, ast)),
                None => hits.push((r, 1, "does not parse as Rust".to_string())),
            }
        }
    }
    hits.extend(scan(&files, &names));
    hits
}

/// A `struct` or `enum` as the check sees it.
struct Ty {
    file: String,
    /// The line of its first derive, else of its name.
    line: usize,
    name: String,
    derives: Vec<String>,
    /// Holds a secret type, or derives `Zeroize`.
    direct: bool,
    /// Has an `impl Drop`, `Zeroize` or `ZeroizeOnDrop` in its file.
    drop: bool,
    /// The capitalised names in its fields and generics, in order.
    toks: Vec<String>,
}

/// What one file holds.
#[derive(Default)]
struct Found {
    types: Vec<Ty>,
    /// The types it has an `impl Drop`, `Zeroize` or `ZeroizeOnDrop` for.
    wiped: BTreeSet<String>,
    /// Its `impl Display for`: type and line.
    display: Vec<(String, usize)>,
}

/// The refusals over files already parsed: path relative to the root, and
/// the syntax.
pub fn scan(files: &[(String, syn::File)], names: &SecretNames) -> Vec<Hit> {
    let mut hits = Vec::new();
    let mut tys: Vec<Ty> = Vec::new();
    let mut display: Vec<(String, String, usize)> = Vec::new();
    for (file, ast) in files {
        let mut v = Walk {
            file,
            names,
            found: Found::default(),
        };
        v.visit_file(ast);
        let Found {
            types,
            wiped,
            display: d,
        } = v.found;
        for mut t in types {
            t.drop = wiped.contains(&t.name);
            if debug_scope(file)
                && NAMED.contains(&t.name.as_str())
                && t.derives.iter().any(|d| d.contains("Debug"))
                && !NAMED_OK.contains(&(file.as_str(), t.name.as_str()))
            {
                hits.push((
                    file.clone(),
                    t.line,
                    format!("`{}` is named like a secret type and derives Debug", t.name),
                ));
            }
            if type_scope(file) {
                tys.push(t);
            }
        }
        if type_scope(file) {
            display.extend(d.into_iter().map(|(n, l)| (file.clone(), n, l)));
        }
    }
    hits.extend(by_type(&tys, &display));
    hits
}

fn crate_of(file: &str) -> &str {
    file.split("/src/").next().unwrap_or(file)
}

fn excepts(file: &str, name: &str) -> &'static str {
    KEPT.iter()
        .find(|(f, n, _, _)| *f == file && *n == name)
        .map_or("", |k| k.2)
}

/// A type is secret when it holds a secret type directly, derives `Zeroize`
/// or has a `Drop` of its own; and, repeated until nothing new is found,
/// when its fields name a type already found secret and it has no `Drop` of
/// its own (the `Drop` is what wipes it). Types are matched by name within
/// a crate, and only a name the crate defines once, since two types of one
/// name cannot be told apart here.
fn by_type(tys: &[Ty], display: &[(String, String, usize)]) -> Vec<Hit> {
    let mut defs: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    let mut secret: BTreeSet<(&str, &str)> = BTreeSet::new();
    let mut sec = vec![false; tys.len()];
    let mut nested: Vec<Option<&str>> = vec![None; tys.len()];
    for (i, t) in tys.iter().enumerate() {
        let k = (crate_of(&t.file), t.name.as_str());
        *defs.entry(k).or_default() += 1;
        if t.direct || t.drop {
            sec[i] = true;
            secret.insert(k);
        }
    }
    loop {
        let mut changed = false;
        for (i, t) in tys.iter().enumerate() {
            if sec[i] || t.drop {
                continue;
            }
            let c = crate_of(&t.file);
            let hit = t.toks.iter().find(|w| {
                secret.contains(&(c, w.as_str()))
                    && defs.get(&(c, w.as_str())) == Some(&1)
                    && **w != t.name
            });
            if let Some(w) = hit {
                sec[i] = true;
                secret.insert((c, t.name.as_str()));
                nested[i] = Some(w.as_str());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let mut hits = Vec::new();
    let mut by_file: BTreeSet<(&str, &str)> = BTreeSet::new();
    for (i, t) in tys.iter().enumerate() {
        if !sec[i] {
            continue;
        }
        let ex: BTreeSet<&str> = excepts(&t.file, &t.name).split(',').collect();
        let bad: Vec<&str> = ["Clone", "PartialEq", "Debug"]
            .into_iter()
            .filter(|d| t.derives.iter().any(|x| x == d) && !ex.contains(d))
            .collect();
        if !bad.is_empty() {
            hits.push((
                t.file.clone(),
                t.line,
                format!(
                    "`{}` holds a secret and derives {}{}",
                    t.name,
                    bad.join(", "),
                    nested[i].map_or(String::new(), |n| format!(" (holds `{n}`)"))
                ),
            ));
        }
        by_file.insert((t.file.as_str(), t.name.as_str()));
    }
    for (file, name, line) in display {
        let ex: BTreeSet<&str> = excepts(file, name).split(',').collect();
        if by_file.contains(&(file.as_str(), name.as_str())) && !ex.contains("Display") {
            hits.push((
                file.clone(),
                *line,
                format!("`{name}` holds a secret and implements Display"),
            ));
        }
    }
    hits
}

struct Walk<'a> {
    file: &'a str,
    names: &'a SecretNames,
    found: Found,
}

/// The type names a part of an item mentions.
struct Mentions<'a> {
    names: &'a SecretNames,
    direct: bool,
    toks: Vec<String>,
}

impl<'a> Visit<'a> for Mentions<'_> {
    fn visit_path(&mut self, p: &'a syn::Path) {
        for s in &p.segments {
            let id = s.ident.to_string();
            if self.names.matches(&id) {
                self.direct = true;
            }
            if id.starts_with(|c: char| c.is_ascii_uppercase()) {
                self.toks.push(id);
            }
        }
        visit::visit_path(self, p);
    }

    fn visit_attribute(&mut self, _: &'a syn::Attribute) {}
}

/// The derives on an item with the line of the first: `derive` and the
/// `derive` inside a `cfg_attr`.
fn derives(attrs: &[syn::Attribute]) -> (Vec<String>, Option<usize>) {
    let mut out = Vec::new();
    let mut line = None;
    for a in attrs {
        let syn::Meta::List(l) = &a.meta else {
            continue;
        };
        let words: Vec<String> = l.tokens.clone().into_iter().flat_map(flatten).collect();
        let found: Vec<String> = if a.path().is_ident("derive") {
            words
        } else if a.path().is_ident("cfg_attr") {
            match words.iter().position(|w| w == "derive") {
                Some(i) => words[i + 1..].to_vec(),
                None => continue,
            }
        } else {
            continue;
        };
        line.get_or_insert(a.pound_token.spans[0].start().line);
        out.extend(found);
    }
    (out, line)
}

impl Walk<'_> {
    fn record(
        &mut self,
        attrs: &[syn::Attribute],
        ident: &syn::Ident,
        generics: &syn::Generics,
        fields: &[&syn::Fields],
    ) {
        let (derives, line) = derives(attrs);
        let mut m = Mentions {
            names: self.names,
            direct: derives.iter().any(|d| d.contains("Zeroize")),
            toks: Vec::new(),
        };
        m.visit_generics(generics);
        for f in fields {
            m.visit_fields(f);
        }
        self.found.types.push(Ty {
            file: self.file.to_string(),
            line: line.unwrap_or(ident.span().start().line),
            name: ident.to_string(),
            derives,
            direct: m.direct,
            drop: false,
            toks: m.toks,
        });
    }
}

impl<'a> Visit<'a> for Walk<'_> {
    fn visit_item_struct(&mut self, i: &'a syn::ItemStruct) {
        self.record(&i.attrs, &i.ident, &i.generics, &[&i.fields]);
        visit::visit_item_struct(self, i);
    }

    fn visit_item_enum(&mut self, i: &'a syn::ItemEnum) {
        let fields: Vec<&syn::Fields> = i.variants.iter().map(|v| &v.fields).collect();
        self.record(&i.attrs, &i.ident, &i.generics, &fields);
        visit::visit_item_enum(self, i);
    }

    fn visit_item_impl(&mut self, i: &'a syn::ItemImpl) {
        if let Some((_, tr, _)) = &i.trait_
            && let Some(t) = tr.segments.last().map(|s| s.ident.to_string())
            && let Some(name) = last_ident(&i.self_ty)
        {
            if ["Drop", "Zeroize", "ZeroizeOnDrop"].contains(&t.as_str()) {
                self.found.wiped.insert(name.clone());
            }
            if t == "Display" {
                self.found
                    .display
                    .push((name, i.impl_token.span.start().line));
            }
        }
        visit::visit_item_impl(self, i);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAMES: &str = "# names\nSecret\nZeroizing\nHeld*\nMasterKey\n";

    /// What the check refuses in one file at `path`, one line each.
    fn refused(path: &str, src: &str) -> Vec<String> {
        let ast = syn::parse_file(src).expect("fixture parses");
        scan(&[(path.to_string(), ast)], &SecretNames::parse(NAMES))
            .into_iter()
            .map(|(_, l, w)| format!("{l}: {w}"))
            .collect()
    }

    const CORE: &str = "core/faraday-x/src/a.rs";

    #[test]
    fn debug_on_a_type_named_like_a_secret_is_refused() {
        let r = refused(CORE, "#[derive(Debug)]\npub struct Seed(u8);\n");
        assert_eq!(r.len(), 1, "{r:?}");
        assert!(r[0].starts_with("1:") && r[0].contains("`Seed`"));
    }

    #[test]
    fn debug_on_a_named_public_type_listed_is_allowed() {
        let r = refused(
            "faraday/faraday-plan/src/lib.rs",
            "#[derive(Debug, Clone)]\npub struct Seed { pub name: u8 }\n",
        );
        assert!(r.is_empty(), "{r:?}");
    }

    #[test]
    fn derives_on_a_type_holding_a_secret_are_refused() {
        let r = refused(
            CORE,
            "#[derive(Clone, PartialEq)]\npub struct Wrap { s: Secret<u8> }\n",
        );
        assert_eq!(r.len(), 1, "{r:?}");
        assert!(r[0].contains("`Wrap`") && r[0].contains("Clone, PartialEq"));
    }

    #[test]
    fn a_held_type_and_a_generic_bound_count_as_secret() {
        assert_eq!(
            refused(CORE, "#[derive(Debug)] struct A(HeldSet);").len(),
            1
        );
        assert_eq!(
            refused(CORE, "#[derive(Debug)] struct B<T: Secret>(T);").len(),
            1
        );
    }

    #[test]
    fn a_type_holding_a_secret_type_through_nesting_is_refused() {
        let r = refused(
            CORE,
            "struct Inner { s: Zeroizing<u8> }\nimpl Drop for Inner { fn drop(&mut self) {} }\n\
             #[derive(Clone)]\nstruct Outer { i: Vec<Inner> }\n",
        );
        assert_eq!(r.len(), 1, "{r:?}");
        assert!(r[0].contains("`Outer`") && r[0].contains("holds `Inner`"));
    }

    #[test]
    fn a_type_with_a_drop_of_its_own_stops_the_nesting() {
        let r = refused(
            CORE,
            "struct Inner { s: Secret<u8> }\n\
             #[derive(Clone)]\nstruct Outer { i: Inner }\nimpl Drop for Outer { fn drop(&mut self) {} }\n",
        );
        assert_eq!(r.len(), 1, "{r:?}");
        assert!(r[0].contains("`Outer`") && !r[0].contains("(holds"));
    }

    #[test]
    fn a_zeroize_derive_or_a_drop_makes_a_type_secret() {
        assert_eq!(
            refused(CORE, "#[derive(Zeroize, Debug)] struct A(u8);").len(),
            1
        );
        assert_eq!(
            refused(
                CORE,
                "#[derive(PartialEq)] struct B(u8);\nimpl Drop for B { fn drop(&mut self) {} }"
            )
            .len(),
            1
        );
    }

    #[test]
    fn display_on_a_secret_type_is_refused() {
        let r = refused(
            CORE,
            "struct A { s: MasterKey }\n\
             impl std::fmt::Display for A { fn fmt(&self, f: &mut F) -> R { Ok(()) } }\n",
        );
        assert_eq!(r.len(), 1, "{r:?}");
        assert!(r[0].starts_with("2:") && r[0].contains("Display"));
    }

    #[test]
    fn display_on_a_type_holding_nothing_secret_is_allowed() {
        let r = refused(
            CORE,
            "#[derive(Clone, Debug)] struct A { n: u8 }\nimpl Display for A { fn fmt(&self) {} }\n",
        );
        assert!(r.is_empty(), "{r:?}");
    }

    #[test]
    fn a_kept_derive_is_allowed_and_another_on_it_is_refused() {
        let path = "faraday/faraday-vault/src/records.rs";
        assert!(refused(path, "#[derive(Debug)] struct Record { f: Secret<u8> }").is_empty());
        let r = refused(
            path,
            "#[derive(Debug, Clone)] struct Record { f: Secret<u8> }",
        );
        assert_eq!(r.len(), 1, "{r:?}");
        assert!(r[0].contains("derives Clone") && !r[0].contains("Debug"));
    }

    #[test]
    fn tests_the_starter_and_the_shells_are_not_read() {
        for p in [
            "core/faraday-x/tests/a.rs",
            "faraday/faraday-starter/src/a.rs",
            "faraday/shells/x/src/a.rs",
        ] {
            let r = refused(p, "#[derive(Clone)] struct A { s: Secret<u8> }");
            assert!(r.is_empty(), "{p}: {r:?}");
        }
    }

    #[test]
    fn a_name_the_crate_defines_twice_is_not_followed() {
        let r = refused(
            CORE,
            "mod a { struct Inner { s: Secret<u8> } }\nmod b { struct Inner(u8); }\n\
             #[derive(Clone)] struct Outer(Inner);",
        );
        assert!(r.is_empty(), "{r:?}");
    }

    #[test]
    fn a_secret_name_in_a_string_or_a_comment_is_not_a_field() {
        let r = refused(
            CORE,
            "#[derive(Clone)]\nstruct A { /* Secret<u8> */ s: u8 }\nconst X: &str = \"Secret\";",
        );
        assert!(r.is_empty(), "{r:?}");
    }
}
