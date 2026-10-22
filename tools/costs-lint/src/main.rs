//! The costs lint (`docs/COMPUTE.md` §8): what the types cannot hold
//! about the compute token (`faraday_crypto::work::Compute`), checked over the
//! sources. Run from the repository root; `just lint` does.
//!
//! It reads `core/*/src` and every crate's `src` under `faraday/`, the
//! modules each crate root reaches, leaving out what is compiled for
//! tests only (`#[cfg(test)]`, and the test kit's
//! `#[cfg(any(test, feature = "testkit"))]`), and fails on:
//!
//! 1. a function taking `&Compute` whose doc's first line does not start
//!    with "Costs " (`CLAUDE.md`: a costly function says so first); a
//!    function that counts work (`c.count(…)`, `work::count(Work::…)`)
//!    and takes no `&Compute`;
//! 2. a mint (`Compute::new`) outside [`MINTS`], anywhere under
//!    `faraday/`, tests and tools included; `Compute::unchecked` anywhere
//!    under `faraday/` but its fuzz targets; either constructor in core's
//!    sources outside its tests; the `unchecked` feature named in a
//!    `faraday/**/Cargo.toml` but the fuzz crate's;
//! 3. a field, `static` or `thread_local!` holding a `Compute` (not a
//!    reference to one), but `View.fill`;
//! 4. a `.compute()` call or a `Compute::` path in a function named
//!    `render…` or in one taking a `View` (its methods included) or
//!    `&mut Ui`: drawing reaches the token only through `View`'s
//!    accessors.
//!
//! Each finding is printed as `path:line: rule: what`; the exit status is
//! 1 when there is any.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use syn::spanned::Spanned;
use syn::visit::{self, Visit};

/// Where a token may be minted: (file, function, why). The function is
/// the innermost named one around the mint, with its type for a method
/// (`Type::name`). `docs/COMPUTE.md` §3, with the test and tool mints the
/// gating batches added (each in a crate that cannot reach faraday-core's
/// test kit). Every site also carries clippy's
/// `#[expect(clippy::disallowed_methods, reason = "a mint: …")]`.
const MINTS: &[(&str, &str, &str)] = &[
    (
        "faraday/faraday-core/src/compute.rs",
        "Faraday::compute",
        "event code holding &mut Faraday",
    ),
    (
        "faraday/faraday-core/src/drawing.rs",
        "View::new",
        "View's own token, for its memo fills, kept values and Allowed entries",
    ),
    (
        "faraday/faraday-core/src/testkit.rs",
        "compute",
        "Faraday's tests and the snapshot tool's sessions",
    ),
    (
        "faraday/faraday-storage/src/lib.rs",
        "Pictures::ask",
        "the pictures' worker thread, which draws nothing (a token cannot cross threads)",
    ),
    (
        "faraday/faraday-disk/src/main.rs",
        "pictures",
        "the disk process's pictures thread, which draws nothing",
    ),
    (
        "faraday/faraday-disk/src/main.rs",
        "main",
        "the disk process's serving loop, which draws nothing",
    ),
    (
        "faraday/shells/snapshot/src/main.rs",
        "main",
        "the snapshot tool, which makes no drawing claim",
    ),
    (
        "faraday/tools/argon2-bench/src/main.rs",
        "main",
        "a benchmark tool that draws nothing",
    ),
    (
        "faraday/faraday-vault/examples/vectors.rs",
        "compute",
        "the vault's vector tool and the test that runs it",
    ),
    (
        "faraday/faraday-vault/src/tests.rs",
        "compute",
        "faraday-vault's tests, which cannot reach faraday-core's test kit",
    ),
    (
        "faraday/faraday-disk/tests/disks.rs",
        "compute",
        "faraday-disk's tests, which cannot reach faraday-core's test kit",
    ),
    (
        "faraday/faraday-files/tests/picture.rs",
        "compute",
        "faraday-files' tests, which cannot reach faraday-core's test kit",
    ),
    (
        "faraday/faraday-keys/tests/operations.rs",
        "compute",
        "faraday-keys' tests, which cannot reach faraday-core's test kit",
    ),
    (
        "faraday/faraday-keys/tests/vault.rs",
        "compute",
        "faraday-keys' tests, which cannot reach faraday-core's test kit",
    ),
    (
        "faraday/faraday-qr/tests/formats.rs",
        "compute",
        "faraday-qr's tests, which cannot reach faraday-core's test kit",
    ),
    (
        "faraday/faraday-qr/tests/interop.rs",
        "compute",
        "faraday-qr's tests, which cannot reach faraday-core's test kit",
    ),
    (
        "faraday/faraday-scanner/src/lib.rs",
        "compute",
        "faraday-scanner's unit tests, which cannot reach faraday-core's test kit",
    ),
    (
        "faraday/faraday-starter/tests/all/bitlocker.rs",
        "compute",
        "the Starter's tests, which draw nothing",
    ),
    (
        "faraday/faraday-starter/tests/all/notes.rs",
        "compute",
        "the Starter's tests, which draw nothing",
    ),
    (
        "faraday/faraday-starter/tests/all/template.rs",
        "compute",
        "the Starter's tests, which draw nothing",
    ),
    (
        "faraday/faraday-starter/examples/snap.rs",
        "compute",
        "the Starter's snapshot tool, which makes no drawing claim",
    ),
];

/// The one place core's sources make a token outside their tests: the
/// token that counts nothing is itself made with `new`.
const CORE_MINTS: &[(&str, &str)] = &[("core/faraday-crypto/src/work.rs", "Compute::unchecked")];

/// The one field that holds a token: `View`'s, private, handed only to
/// its accessors (`docs/COMPUTE.md` §4).
const FIELDS: &[(&str, &str)] = &[("faraday/faraday-core/src/drawing.rs", "View.fill")];

/// Directories under `faraday/` whose code may use the token that counts
/// nothing, and whose manifest may turn its feature on: the fuzz
/// targets, a workspace of their own with no app guarantee
/// (`docs/COMPUTE.md` §3).
const UNCHECKED_OK: &[&str] = &["faraday/fuzz/"];

fn main() -> ExitCode {
    let root = PathBuf::from(".");
    let mut out = Findings::default();

    // Rules 1, 3, 4, and rule 2 for core: the crate sources, test-only
    // code left out.
    for src in crate_sources(&root) {
        for (file, test) in module_files(&root, &src) {
            if test {
                continue;
            }
            check_source(&root, &file, &mut out);
        }
    }
    // Rule 2 under faraday/: every Rust file, tests and tools included.
    for file in rust_files(&root.join("faraday")) {
        check_mints_faraday(&root, &file, &mut out);
    }
    // Rule 2: the feature in Faraday's manifests.
    for file in manifests(&root.join("faraday")) {
        check_manifest(&root, &file, &mut out);
    }

    if out.lines.is_empty() {
        println!("costs-lint: ok");
        ExitCode::SUCCESS
    } else {
        for l in &out.lines {
            println!("{l}");
        }
        println!(
            "costs-lint: {} finding(s) (docs/COMPUTE.md §8)",
            out.lines.len()
        );
        ExitCode::FAILURE
    }
}

#[derive(Default)]
struct Findings {
    lines: BTreeSet<String>,
}

impl Findings {
    fn add(&mut self, file: &str, line: usize, rule: &str, what: String) {
        self.lines.insert(format!("{file}:{line}: {rule}: {what}"));
    }
}

/// `path` relative to the repository root, with `/`.
fn rel(root: &Path, path: &Path) -> String {
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
fn crate_sources(root: &Path) -> Vec<PathBuf> {
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
fn manifests(dir: &Path) -> Vec<PathBuf> {
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
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(dir, &mut |p| {
        if p.extension().is_some_and(|e| e == "rs") {
            out.push(p.to_path_buf());
        }
    });
    out.sort();
    out
}

fn walk(dir: &Path, f: &mut dyn FnMut(&Path)) {
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

fn parse(path: &Path) -> Option<syn::File> {
    let text = fs::read_to_string(path).ok()?;
    match syn::parse_file(&text) {
        Ok(f) => Some(f),
        Err(e) => {
            eprintln!("costs-lint: {} does not parse: {e}", path.display());
            None
        }
    }
}

/// The files a crate's roots reach through `mod` declarations, each with
/// whether it is compiled for tests only.
fn module_files(root: &Path, src: &Path) -> Vec<(PathBuf, bool)> {
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
        reach(root, &r, dir, false, &mut out, &mut seen);
    }
    out
}

/// `file`, whose child modules live in `dir`, and the files it reaches.
fn reach(
    root: &Path,
    file: &Path,
    dir: PathBuf,
    test: bool,
    out: &mut Vec<(PathBuf, bool)>,
    seen: &mut BTreeSet<PathBuf>,
) {
    if !seen.insert(file.to_path_buf()) {
        return;
    }
    out.push((file.to_path_buf(), test));
    let Some(ast) = parse(file) else {
        return;
    };
    let base = file.parent().map(Path::to_path_buf).unwrap_or_default();
    reach_items(root, &ast.items, &dir, &base, test, out, seen);
}

fn reach_items(
    root: &Path,
    items: &[syn::Item],
    dir: &Path,
    base: &Path,
    test: bool,
    out: &mut Vec<(PathBuf, bool)>,
    seen: &mut BTreeSet<PathBuf>,
) {
    for item in items {
        let syn::Item::Mod(m) = item else { continue };
        let t = test || is_test(&m.attrs);
        let name = m.ident.to_string();
        if let Some((_, inner)) = &m.content {
            reach_items(root, inner, &dir.join(&name), base, t, out, seen);
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

/// Whether `attrs` compile the item for tests only: a `cfg` naming
/// `test` (alone, or in `all`/`any` with the test kit's feature) and no
/// `not`.
fn is_test(attrs: &[syn::Attribute]) -> bool {
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

fn flatten(t: proc_macro2::TokenTree) -> Vec<String> {
    match t {
        proc_macro2::TokenTree::Group(g) => g.stream().into_iter().flat_map(flatten).collect(),
        proc_macro2::TokenTree::Ident(i) => vec![i.to_string()],
        _ => Vec::new(),
    }
}

/// The first line of an item's doc, trimmed.
fn doc_first_line(attrs: &[syn::Attribute]) -> Option<String> {
    for a in attrs {
        if !a.path().is_ident("doc") {
            continue;
        }
        if let syn::Meta::NameValue(nv) = &a.meta
            && let syn::Expr::Lit(l) = &nv.value
            && let syn::Lit::Str(s) = &l.lit
        {
            let v = s.value();
            if let Some(line) = v.lines().map(str::trim).find(|l| !l.is_empty()) {
                return Some(line.to_string());
            }
        }
    }
    None
}

/// Whether `ty` is `&Compute` (any path ending in `Compute`).
fn is_compute_ref(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Reference(r) => last_ident(&r.elem).is_some_and(|i| i == "Compute"),
        syn::Type::Paren(p) => is_compute_ref(&p.elem),
        _ => false,
    }
}

fn last_ident(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
        syn::Type::Paren(p) => last_ident(&p.elem),
        _ => None,
    }
}

/// Whether `ty` holds a `Compute` by value: anywhere but behind a
/// reference.
fn holds_compute(ty: &syn::Type) -> bool {
    struct V(bool);
    impl<'a> Visit<'a> for V {
        fn visit_type_reference(&mut self, _: &'a syn::TypeReference) {}
        fn visit_path_segment(&mut self, s: &'a syn::PathSegment) {
            if s.ident == "Compute" {
                self.0 = true;
            }
            visit::visit_path_segment(self, s);
        }
    }
    let mut v = V(false);
    v.visit_type(ty);
    v.0
}

/// What a function's signature says about drawing: whether it takes a
/// `View` (by reference, or as `self` in `View`'s own impl) or a
/// `&mut Ui`.
fn draws(sig: &syn::Signature, self_ty: Option<&str>) -> bool {
    sig.inputs.iter().any(|i| match i {
        syn::FnArg::Receiver(_) => self_ty == Some("View"),
        syn::FnArg::Typed(t) => match &*t.ty {
            syn::Type::Reference(r) => match last_ident(&r.elem).as_deref() {
                Some("View") => true,
                Some("Ui") => r.mutability.is_some(),
                _ => false,
            },
            _ => false,
        },
    })
}

/// The type an `impl` is for, its last name.
fn impl_name(i: &syn::ItemImpl) -> Option<String> {
    last_ident(&i.self_ty)
}

/// One crate source file: rules 1, 3, 4, and rule 2 in core.
fn check_source(root: &Path, file: &Path, out: &mut Findings) {
    let Some(ast) = parse(file) else { return };
    let r = rel(root, file);
    let mut v = Source {
        file: r.clone(),
        core: r.starts_with("core/"),
        out,
        self_ty: Vec::new(),
        trait_impl: Vec::new(),
        fns: Vec::new(),
        drawing: Vec::new(),
    };
    v.visit_file(&ast);
}

struct Source<'o> {
    file: String,
    core: bool,
    out: &'o mut Findings,
    /// The type of each `impl` around, innermost last.
    self_ty: Vec<Option<String>>,
    /// Whether each `impl` around is a trait's.
    trait_impl: Vec<bool>,
    /// Each function around: its name and whether it takes `&Compute`.
    fns: Vec<(String, bool, usize)>,
    /// Whether each function around draws (rule 4).
    drawing: Vec<bool>,
}

/// What a function body counts and touches.
#[derive(Default)]
struct Body {
    /// Line of the first count, if any.
    counts: Option<usize>,
}

impl Source<'_> {
    fn line(s: proc_macro2::Span) -> usize {
        s.start().line
    }

    fn fn_name(&self, ident: &syn::Ident) -> String {
        match self.self_ty.last() {
            Some(Some(t)) => format!("{t}::{ident}"),
            _ => ident.to_string(),
        }
    }

    fn enter_fn(&mut self, attrs: &[syn::Attribute], sig: &syn::Signature, check_doc: bool) {
        let takes = sig.inputs.iter().any(|i| match i {
            syn::FnArg::Typed(t) => is_compute_ref(&t.ty),
            syn::FnArg::Receiver(_) => false,
        });
        let name = self.fn_name(&sig.ident);
        if takes && check_doc {
            let ok = doc_first_line(attrs).is_some_and(|l| l.starts_with("Costs "));
            if !ok {
                self.out.add(
                    &self.file,
                    Self::line(sig.ident.span()),
                    "rule 1",
                    format!(
                        "`{name}` takes `&Compute`: its doc's first line says what it costs (\"Costs …\")"
                    ),
                );
            }
        }
        let self_ty = self.self_ty.last().cloned().flatten();
        let render = sig.ident.to_string().starts_with("render");
        let d = render || draws(sig, self_ty.as_deref());
        self.fns.push((name, takes, Self::line(sig.ident.span())));
        self.drawing.push(d);
    }

    fn leave_fn(&mut self, body: Body) {
        let (name, takes, _) = self.fns.pop().unwrap_or_default();
        self.drawing.pop();
        if let Some(line) = body.counts
            && !takes
            && !counter_itself(&self.file, &name)
        {
            self.out.add(
                &self.file,
                line,
                "rule 1",
                format!("`{name}` counts costly work and takes no `&Compute`"),
            );
        }
    }

    fn in_drawing(&self) -> bool {
        self.drawing.last().copied().unwrap_or(false)
    }

    fn mint(&mut self, what: &str, line: usize) {
        if !self.core {
            return; // rule 2 under faraday/ reads every file, below.
        }
        let name = self.fns.last().map(|f| f.0.clone()).unwrap_or_default();
        if CORE_MINTS
            .iter()
            .any(|(f, n)| *f == self.file && *n == name)
        {
            return;
        }
        self.out.add(
            &self.file,
            line,
            "rule 2",
            format!("`Compute::{what}` in core's sources: core passes its caller's token"),
        );
    }

    fn field(&mut self, name: &str, ty: &syn::Type, line: usize) {
        if !holds_compute(ty) {
            return;
        }
        if FIELDS.iter().any(|(f, n)| *f == self.file && *n == name) {
            return;
        }
        self.out.add(
            &self.file,
            line,
            "rule 3",
            format!("`{name}` keeps a `Compute`: a token is minted where it is used"),
        );
    }

    /// Walks a function body for counts (rule 1) and token reaches
    /// (rules 2 and 4).
    fn body(&mut self, block: &syn::Block) -> Body {
        let mut b = BodyWalk {
            src: self,
            body: Body::default(),
        };
        b.visit_block(block);
        b.body
    }
}

/// The counters themselves, which count for others: `Compute::count`
/// and Faraday's `work::count`, `core_count`.
fn counter_itself(file: &str, name: &str) -> bool {
    matches!(
        (file, name),
        ("core/faraday-crypto/src/work.rs", "Compute::count")
            | ("faraday/faraday-core/src/work.rs", "count" | "core_count")
    )
}

struct BodyWalk<'s, 'o> {
    src: &'s mut Source<'o>,
    body: Body,
}

impl<'a> Visit<'a> for BodyWalk<'_, '_> {
    // A nested function or impl is its own item: the source walker reads
    // it.
    fn visit_item(&mut self, i: &'a syn::Item) {
        self.src.visit_item(i);
    }

    fn visit_expr_method_call(&mut self, m: &'a syn::ExprMethodCall) {
        if m.method == "count"
            && m.args.len() == 1
            && quote::quote!(#m).to_string().contains("Work ::")
        {
            self.body
                .counts
                .get_or_insert(Source::line(m.method.span()));
        }
        if m.method == "compute" && m.args.is_empty() && self.src.in_drawing() {
            let name = self.src.fns.last().map(|f| f.0.clone()).unwrap_or_default();
            self.src.out.add(
                &self.src.file,
                Source::line(m.method.span()),
                "rule 4",
                format!("`{name}` draws and mints a token (`.compute()`): drawing reaches costly work through `View`'s accessors"),
            );
        }
        visit::visit_expr_method_call(self, m);
    }

    fn visit_expr_call(&mut self, c: &'a syn::ExprCall) {
        if let syn::Expr::Path(p) = &*c.func
            && p.path.segments.last().is_some_and(|s| s.ident == "count")
            && c.args.len() == 1
            && quote::quote!(#c).to_string().contains("Work ::")
        {
            self.body.counts.get_or_insert(Source::line(c.span()));
        }
        visit::visit_expr_call(self, c);
    }

    fn visit_path(&mut self, p: &'a syn::Path) {
        let segs: Vec<String> = p.segments.iter().map(|s| s.ident.to_string()).collect();
        if let Some(i) = segs.iter().position(|s| s == "Compute")
            && i + 1 < segs.len()
        {
            let line = Source::line(p.span());
            if self.src.in_drawing() {
                let name = self.src.fns.last().map(|f| f.0.clone()).unwrap_or_default();
                self.src.out.add(
                    &self.src.file,
                    line,
                    "rule 4",
                    format!("`{name}` draws and names `Compute::{}`: drawing reaches costly work through `View`'s accessors", segs[i + 1]),
                );
            }
            if segs[i + 1] == "new" || segs[i + 1] == "unchecked" {
                let what = segs[i + 1].clone();
                self.src.mint(&what, line);
            }
        }
        visit::visit_path(self, p);
    }

    fn visit_macro(&mut self, m: &'a syn::Macro) {
        // Macro bodies are tokens: `format!`, `assert!` and the like hold
        // expressions this walk would otherwise not see.
        if let Ok(args) = m.parse_body_with(
            syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated,
        ) {
            for a in &args {
                self.visit_expr(a);
            }
        }
        visit::visit_macro(self, m);
    }
}

impl<'a> Visit<'a> for Source<'_> {
    fn visit_item(&mut self, i: &'a syn::Item) {
        let attrs: &[syn::Attribute] = match i {
            syn::Item::Fn(f) => &f.attrs,
            syn::Item::Impl(x) => &x.attrs,
            syn::Item::Mod(x) => &x.attrs,
            syn::Item::Struct(x) => &x.attrs,
            syn::Item::Enum(x) => &x.attrs,
            syn::Item::Static(x) => &x.attrs,
            syn::Item::Trait(x) => &x.attrs,
            syn::Item::Macro(x) => &x.attrs,
            syn::Item::Const(x) => &x.attrs,
            _ => &[],
        };
        if is_test(attrs) {
            return;
        }
        // A file module is read as its own file.
        if let syn::Item::Mod(m) = i
            && m.content.is_none()
        {
            return;
        }
        visit::visit_item(self, i);
    }

    fn visit_item_impl(&mut self, i: &'a syn::ItemImpl) {
        self.self_ty.push(impl_name(i));
        self.trait_impl.push(i.trait_.is_some());
        for it in &i.items {
            self.visit_impl_item(it);
        }
        self.self_ty.pop();
        self.trait_impl.pop();
    }

    fn visit_impl_item(&mut self, i: &'a syn::ImplItem) {
        if let syn::ImplItem::Fn(f) = i {
            if is_test(&f.attrs) {
                return;
            }
            let check_doc = !self.trait_impl.last().copied().unwrap_or(false);
            self.enter_fn(&f.attrs, &f.sig, check_doc);
            let b = self.body(&f.block);
            self.leave_fn(b);
        } else {
            visit::visit_impl_item(self, i);
        }
    }

    fn visit_item_fn(&mut self, f: &'a syn::ItemFn) {
        // A free function: no impl around it, even nested in a method.
        self.self_ty.push(None);
        self.trait_impl.push(false);
        self.enter_fn(&f.attrs, &f.sig, true);
        let b = self.body(&f.block);
        self.leave_fn(b);
        self.self_ty.pop();
        self.trait_impl.pop();
    }

    fn visit_trait_item_fn(&mut self, f: &'a syn::TraitItemFn) {
        if is_test(&f.attrs) {
            return;
        }
        self.enter_fn(&f.attrs, &f.sig, true);
        let b = match &f.default {
            Some(block) => self.body(block),
            None => Body::default(),
        };
        self.leave_fn(b);
    }

    fn visit_item_trait(&mut self, t: &'a syn::ItemTrait) {
        self.self_ty.push(Some(t.ident.to_string()));
        self.trait_impl.push(false);
        visit::visit_item_trait(self, t);
        self.self_ty.pop();
        self.trait_impl.pop();
    }

    fn visit_item_struct(&mut self, s: &'a syn::ItemStruct) {
        for (n, f) in s.fields.iter().enumerate() {
            let name = match &f.ident {
                Some(i) => format!("{}.{i}", s.ident),
                None => format!("{}.{n}", s.ident),
            };
            self.field(&name, &f.ty, Source::line(f.span()));
        }
    }

    fn visit_item_enum(&mut self, e: &'a syn::ItemEnum) {
        for v in &e.variants {
            for (n, f) in v.fields.iter().enumerate() {
                let name = match &f.ident {
                    Some(i) => format!("{}::{}.{i}", e.ident, v.ident),
                    None => format!("{}::{}.{n}", e.ident, v.ident),
                };
                self.field(&name, &f.ty, Source::line(f.span()));
            }
        }
    }

    fn visit_item_static(&mut self, s: &'a syn::ItemStatic) {
        self.field(&s.ident.to_string(), &s.ty, Source::line(s.ident.span()));
    }

    fn visit_item_macro(&mut self, m: &'a syn::ItemMacro) {
        thread_local(self, &m.mac);
    }

    fn visit_stmt_macro(&mut self, m: &'a syn::StmtMacro) {
        thread_local(self, &m.mac);
    }
}

/// Rule 3 for a `thread_local!`: any `Compute` named in it.
fn thread_local(src: &mut Source<'_>, m: &syn::Macro) {
    if !m
        .path
        .segments
        .last()
        .is_some_and(|s| s.ident == "thread_local")
    {
        return;
    }
    let words: Vec<String> = m.tokens.clone().into_iter().flat_map(flatten).collect();
    if words.iter().any(|w| w == "Compute") {
        src.out.add(
            &src.file,
            Source::line(m.path.span()),
            "rule 3",
            "a `thread_local!` keeps a `Compute`: a token is minted where it is used".into(),
        );
    }
}

/// Rule 2 under `faraday/`: every mint in every file, test code included.
fn check_mints_faraday(root: &Path, file: &Path, out: &mut Findings) {
    let Some(ast) = parse(file) else { return };
    let r = rel(root, file);
    let mut v = Mints {
        file: r,
        out,
        names: Vec::new(),
    };
    v.visit_file(&ast);
}

struct Mints<'o> {
    file: String,
    out: &'o mut Findings,
    /// The named function around, with its type for a method; `None`
    /// pushed for an impl's type.
    names: Vec<(Option<String>, String)>,
}

impl Mints<'_> {
    fn current(&self) -> String {
        let fname = self
            .names
            .iter()
            .rev()
            .find(|(_, f)| !f.is_empty())
            .cloned();
        match fname {
            Some((Some(t), f)) => format!("{t}::{f}"),
            Some((None, f)) => f,
            None => String::new(),
        }
    }
}

impl<'a> Visit<'a> for Mints<'_> {
    fn visit_item_impl(&mut self, i: &'a syn::ItemImpl) {
        self.names.push((impl_name(i), String::new()));
        visit::visit_item_impl(self, i);
        self.names.pop();
    }

    fn visit_impl_item_fn(&mut self, f: &'a syn::ImplItemFn) {
        let t = self.names.last().and_then(|n| n.0.clone());
        self.names.push((t, f.sig.ident.to_string()));
        visit::visit_impl_item_fn(self, f);
        self.names.pop();
    }

    fn visit_item_fn(&mut self, f: &'a syn::ItemFn) {
        self.names.push((None, f.sig.ident.to_string()));
        visit::visit_item_fn(self, f);
        self.names.pop();
    }

    fn visit_path(&mut self, p: &'a syn::Path) {
        let segs: Vec<String> = p.segments.iter().map(|s| s.ident.to_string()).collect();
        if let Some(i) = segs.iter().position(|s| s == "Compute")
            && let Some(what) = segs.get(i + 1)
        {
            let line = p.span().start().line;
            let name = self.current();
            if what == "unchecked" && !UNCHECKED_OK.iter().any(|d| self.file.starts_with(d)) {
                self.out.add(
                    &self.file,
                    line,
                    "rule 2",
                    format!("`{name}` uses `Compute::unchecked`, which is never Faraday's"),
                );
            }
            if what == "new" && !MINTS.iter().any(|(f, n, _)| *f == self.file && *n == name) {
                self.out.add(
                    &self.file,
                    line,
                    "rule 2",
                    format!("`{name}` mints a token (`Compute::new`) and is not on the costs lint's list of mints (docs/COMPUTE.md §3)"),
                );
            }
        }
        visit::visit_path(self, p);
    }

    fn visit_macro(&mut self, m: &'a syn::Macro) {
        if let Ok(args) = m.parse_body_with(
            syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated,
        ) {
            for a in &args {
                self.visit_expr(a);
            }
        }
        visit::visit_macro(self, m);
    }
}

/// Rule 2: the `unchecked` feature named in a Faraday manifest.
fn check_manifest(root: &Path, file: &Path, out: &mut Findings) {
    let r = rel(root, file);
    if UNCHECKED_OK.iter().any(|d| r.starts_with(d)) {
        return;
    }
    let Ok(text) = fs::read_to_string(file) else {
        return;
    };
    for (n, line) in text.lines().enumerate() {
        let code = line.split('#').next().unwrap_or("");
        if code.contains("unchecked") {
            out.add(
                &r,
                n + 1,
                "rule 2",
                "names the `unchecked` feature: the token that counts nothing is never Faraday's"
                    .into(),
            );
        }
    }
}
