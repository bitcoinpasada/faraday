//! The source walker the costs lint and the crypto inventory share
//! (`docs/CRYPTO-LEARN.md` §4.1): the crate roots under `core/` and
//! `faraday/`, the modules each root reaches, and what is compiled for
//! tests left out. One walker, so both tools agree on what is shipped
//! code.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Files [`parse`] could not read as Rust, since the process began.
static PARSE_FAILURES: AtomicUsize = AtomicUsize::new(0);

/// How many files [`parse`] reported as not parsing.
pub fn parse_failures() -> usize {
    PARSE_FAILURES.load(Ordering::Relaxed)
}

/// A file a crate's roots reach: whether it is compiled for tests only,
/// and the `cfg` attributes (as text) that make it so.
pub struct ModFile {
    /// The file.
    pub path: PathBuf,
    /// Compiled for tests only.
    pub test: bool,
    /// The test `cfg`s on the `mod` declarations leading to it, joined
    /// with ` & `; empty when `test` is false.
    pub cfg: String,
}

/// `path` relative to the repository root, with `/`.
pub fn rel(root: &Path, path: &Path) -> String {
    let p = path.strip_prefix(root).unwrap_or(path);
    p.components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Every `src` directory of a crate in core and under `faraday/`.
pub fn crate_sources(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(root.join("core")) {
        for e in rd.flatten() {
            let src = e.path().join("src");
            if e.path().join("Cargo.toml").is_file() && src.is_dir() {
                out.push(src);
            }
        }
    }
    for m in manifests(&root.join("faraday")) {
        let src = m.parent().map(|d| d.join("src"));
        if let Some(src) = src
            && src.is_dir()
        {
            out.push(src);
        }
    }
    out.sort();
    out
}

/// Every `Cargo.toml` under `dir`, build output left out.
pub fn manifests(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(dir, &mut |p| {
        if p.file_name().is_some_and(|n| n == "Cargo.toml") {
            out.push(p.to_path_buf());
        }
    });
    out.sort();
    out
}

/// Every `.rs` file under `dir`, build output left out.
pub fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(dir, &mut |p| {
        if p.extension().is_some_and(|e| e == "rs") {
            out.push(p.to_path_buf());
        }
    });
    out.sort();
    out
}

/// Calls `f` on every file under `dir`, build output and dot-folders left
/// out.
pub fn walk(dir: &Path, f: &mut dyn FnMut(&Path)) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name();
        if name == "target" || name.to_string_lossy().starts_with('.') {
            continue;
        }
        if p.is_dir() {
            walk(&p, f);
        } else {
            f(&p);
        }
    }
}

/// `path` read as Rust, or `None` (and a note on stderr) when it does not
/// parse.
pub fn parse(path: &Path) -> Option<syn::File> {
    let text = fs::read_to_string(path).ok()?;
    match syn::parse_file(&text) {
        Ok(f) => Some(f),
        Err(e) => {
            eprintln!("costs-lint: {} does not parse: {e}", path.display());
            PARSE_FAILURES.fetch_add(1, Ordering::Relaxed);
            None
        }
    }
}

/// The files a crate's roots reach through `mod` declarations, each with
/// whether it is compiled for tests only.
pub fn module_files(root: &Path, src: &Path) -> Vec<(PathBuf, bool)> {
    module_files_cfg(root, src)
        .into_iter()
        .map(|m| (m.path, m.test))
        .collect()
}

/// [`module_files`], each with the `cfg` that makes it test-only.
pub fn module_files_cfg(root: &Path, src: &Path) -> Vec<ModFile> {
    let mut roots = Vec::new();
    for r in ["lib.rs", "main.rs"] {
        if src.join(r).is_file() {
            roots.push(src.join(r));
        }
    }
    if let Ok(rd) = fs::read_dir(src.join("bin")) {
        for e in rd.flatten() {
            if e.path().extension().is_some_and(|x| x == "rs") {
                roots.push(e.path());
            }
        }
    }
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for r in roots {
        let dir = r.parent().map(Path::to_path_buf).unwrap_or_default();
        reach(root, &r, dir, (false, String::new()), &mut out, &mut seen);
    }
    out
}

/// `file`, whose child modules live in `dir`, and the files it reaches.
fn reach(
    root: &Path,
    file: &Path,
    dir: PathBuf,
    test: (bool, String),
    out: &mut Vec<ModFile>,
    seen: &mut BTreeSet<PathBuf>,
) {
    if !seen.insert(file.to_path_buf()) {
        return;
    }
    out.push(ModFile {
        path: file.to_path_buf(),
        test: test.0,
        cfg: test.1.clone(),
    });
    let Some(ast) = parse(file) else {
        return;
    };
    let base = file.parent().map(Path::to_path_buf).unwrap_or_default();
    reach_items(root, &ast.items, &dir, &base, &test, out, seen);
}

fn reach_items(
    root: &Path,
    items: &[syn::Item],
    dir: &Path,
    base: &Path,
    test: &(bool, String),
    out: &mut Vec<ModFile>,
    seen: &mut BTreeSet<PathBuf>,
) {
    for item in items {
        let syn::Item::Mod(m) = item else { continue };
        let t = if is_test(&m.attrs) {
            let mut c = test.1.clone();
            if !c.is_empty() {
                c.push_str(" & ");
            }
            c.push_str(&cfg_text(&m.attrs));
            (true, c)
        } else {
            test.clone()
        };
        let name = m.ident.to_string();
        if let Some((_, inner)) = &m.content {
            reach_items(root, inner, &dir.join(&name), base, &t, out, seen);
            continue;
        }
        let file = if let Some(p) = path_attr(&m.attrs) {
            base.join(p)
        } else if dir.join(format!("{name}.rs")).is_file() {
            dir.join(format!("{name}.rs"))
        } else {
            dir.join(&name).join("mod.rs")
        };
        if !file.is_file() {
            continue;
        }
        let child_dir = if file.file_name().is_some_and(|n| n == "mod.rs") {
            file.parent().map(Path::to_path_buf).unwrap_or_default()
        } else {
            file.with_extension("")
        };
        reach(root, &file, child_dir, t, out, seen);
    }
}

fn path_attr(attrs: &[syn::Attribute]) -> Option<String> {
    for a in attrs {
        if a.path().is_ident("path")
            && let syn::Meta::NameValue(nv) = &a.meta
            && let syn::Expr::Lit(l) = &nv.value
            && let syn::Lit::Str(s) = &l.lit
        {
            return Some(s.value());
        }
    }
    None
}

/// The `cfg(…)` attributes of an item, as text.
pub fn cfg_text(attrs: &[syn::Attribute]) -> String {
    attrs
        .iter()
        .filter(|a| a.path().is_ident("cfg"))
        .map(|a| match &a.meta {
            syn::Meta::List(l) => format!("cfg({})", l.tokens),
            _ => "cfg".to_string(),
        })
        .collect::<Vec<_>>()
        .join(" & ")
}

/// Whether `attrs` compile the item for tests only: a `cfg` naming
/// `test` (alone, or in `all`/`any` with the test kit's feature) and no
/// `not`.
pub fn is_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        if !a.path().is_ident("cfg") {
            return false;
        }
        let syn::Meta::List(l) = &a.meta else {
            return false;
        };
        let words: Vec<String> = l.tokens.clone().into_iter().flat_map(flatten).collect();
        words.iter().any(|w| w == "test") && !words.iter().any(|w| w == "not")
    })
}

/// The identifiers of a token tree, groups opened.
pub fn flatten(t: proc_macro2::TokenTree) -> Vec<String> {
    match t {
        proc_macro2::TokenTree::Group(g) => g.stream().into_iter().flat_map(flatten).collect(),
        proc_macro2::TokenTree::Ident(i) => vec![i.to_string()],
        _ => Vec::new(),
    }
}

/// The last name of a path type.
pub fn last_ident(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
        syn::Type::Paren(p) => last_ident(&p.elem),
        _ => None,
    }
}
