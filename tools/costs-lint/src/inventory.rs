//! The crypto inventory (`docs/CRYPTO-LEARN.md` §4, §5.1, §9.2): which
//! functions of the repository are cryptographic, what each one's body
//! hashes to, which functions of its crate it calls, which other crates
//! it calls, which `#[test]` functions exist, and where `faraday-core`
//! calls the cryptography of `faraday-keys` and `core/`. Printed as JSON
//! lines for the manifest's checker; it reads the same shipped code the
//! costs lint does, through the same walker ([`crate::walk`]).
//!
//! ```text
//! crypto-inventory [--root DIR] [--all]    JSON lines on stdout, a summary on stderr
//! crypto-inventory --self-test             the fixture crate's checks
//! ```
//!
//! A line is one of:
//!
//! - `{"kind":"fn", …}`: a function of a cryptographic crate with its
//!   `rules` (§4.2; empty for a function printed only because a
//!   cryptographic one calls it, directly or through others, which is
//!   the closure's input), its `hash` (§5.1), its same-crate `callees`
//!   (ids) and its `cross` crate calls. `--all` prints every function of
//!   a cryptographic crate.
//! - `{"kind":"test", …}`: a `#[test]` function of any crate.
//! - `{"kind":"call_site", …}`: a call in `faraday-core` of a
//!   cryptographic function of `faraday-keys` or `core/` (§9.2), `via`
//!   `path` (resolved through the file's `use` lines) or `method` (by
//!   name, taken only where the calling file names the callee's type).
//!
//! Calls are resolved by name (§4.4): a path call by its last two
//! segments, a method call by its name, every function of that name in
//! the crate taken. Costs one read and parse of every source file.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use crate::sha256;
use crate::walk;

/// The crates that compute (§4.1): a crate whose normal dependencies
/// include one is a cryptographic crate.
const PRIMITIVE_CRATES: &[&str] = &[
    "faraday-crypto",
    "bitcoin",
    "secp256k1",
    "sha1",
    "sha2",
    "hmac",
    "pbkdf2",
    "argon2",
    "chacha20",
    "chacha20poly1305",
    "ed25519-dalek",
    "rsa",
    "getrandom",
];

/// What `faraday-crypto` exports that computes nothing: the compute token
/// and the secret wrappers. A path into them is not a path into a
/// primitive (§4.2 rule 3 would otherwise take the 600 functions of
/// `faraday-core` that pass the token through).
const NOT_COMPUTING: &[&str] = &[
    "work",
    "Secret",
    "Zeroize",
    "ZeroizeOnDrop",
    "Zeroizing",
    "WipeOnFree",
    "Pinned",
    "pin_failure_to_report",
];

/// The `faraday/` crates that hold keys: with `core/`, where rules 1 and
/// 4 apply (§4.2).
const TOKEN_CRATES: &[&str] = &[
    "faraday-keys",
    "faraday-vault",
    "faraday-pgp",
    "faraday-sb",
    "faraday-payload",
    "faraday-qr",
];

/// Rule 2's crate.
const KEYS_CRATE: &str = "faraday-keys";

/// The crate whose call sites are listed (§9.2).
const APP_CRATE: &str = "faraday-core";

fn lib_name(package: &str) -> String {
    package.replace('-', "_")
}

/// One crate under `core/` or `faraday/`.
pub(crate) struct Crate {
    /// The package name.
    pub(crate) name: String,
    /// Its directory, relative to the root.
    dir: String,
    src: PathBuf,
    /// A normal dependency is a primitive crate.
    cryptographic: bool,
}

/// A function of a cryptographic crate.
pub(crate) struct Func {
    krate: usize,
    /// The file, relative to the root.
    pub(crate) file: String,
    /// `Type::name` for a method, `name` for a function.
    pub(crate) name: String,
    ident: String,
    ty: Option<String>,
    line: usize,
    public: bool,
    /// The rules of §4.2 it meets.
    pub(crate) rules: BTreeSet<u8>,
    /// §5.1's body hash.
    pub(crate) hash: String,
    /// Same-crate functions it calls (indexes).
    pub(crate) callees: BTreeSet<usize>,
    /// Calls into other workspace crates, `crate::path`.
    pub(crate) cross: BTreeSet<String>,
    calls: Vec<Call>,
    methods: Vec<(String, usize)>,
    /// Every name its paths use.
    idents: BTreeSet<String>,
}

/// A path used as an expression, after the file's `use` lines.
struct Call {
    segs: Vec<String>,
    called: bool,
    line: usize,
}

/// A `#[test]` function.
pub(crate) struct TestFn {
    krate: usize,
    file: String,
    pub(crate) name: String,
    line: usize,
    ignored: bool,
    cfg: String,
}

/// A call in the app of a cryptographic function elsewhere.
pub(crate) struct Site {
    caller: usize,
    callee: usize,
    line: usize,
    via: &'static str,
}

/// Everything the inventory found.
pub(crate) struct Inventory {
    pub(crate) crates: Vec<Crate>,
    pub(crate) funcs: Vec<Func>,
    pub(crate) tests: Vec<TestFn>,
    pub(crate) sites: Vec<Site>,
}

// ---------------------------------------------------------------- manifests

enum DepSection {
    No,
    Table,
    Named(String),
}

fn dep_section(s: &str) -> DepSection {
    if s.starts_with("workspace.") {
        return DepSection::No;
    }
    if s == "dependencies" || s.ends_with(".dependencies") {
        return DepSection::Table;
    }
    match s.find("dependencies.") {
        Some(i) if i == 0 || s[..i].ends_with('.') => {
            DepSection::Named(s[i + "dependencies.".len()..].trim_matches('"').to_string())
        }
        _ => DepSection::No,
    }
}

fn unquote(v: &str) -> String {
    v.trim().trim_matches('"').to_string()
}

/// A manifest's package name and its normal dependencies' package names
/// (no TOML crate: the few shapes the workspace uses).
fn read_manifest(path: &Path) -> (String, BTreeSet<String>) {
    let text = fs::read_to_string(path).unwrap_or_default();
    let mut section = String::new();
    let mut name = String::new();
    let mut deps = BTreeSet::new();
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with('#') || l.is_empty() {
            continue;
        }
        if l.starts_with('[') {
            section = l.trim_matches(|c| c == '[' || c == ']').trim().to_string();
            if let DepSection::Named(n) = dep_section(&section) {
                deps.insert(n);
            }
            continue;
        }
        let Some((k, v)) = l.split_once('=') else {
            continue;
        };
        let k = k.trim();
        if section == "package" && k == "name" {
            name = unquote(v);
        } else if matches!(dep_section(&section), DepSection::Table) {
            let mut dep = unquote(k.split('.').next().unwrap_or(k));
            if let Some(rest) = v.split("package").nth(1)
                && let Some((_, p)) = rest.split_once('=')
                && let Some(p) = p.trim().strip_prefix('"')
                && let Some((p, _)) = p.split_once('"')
            {
                dep = p.to_string();
            }
            deps.insert(dep);
        } else if matches!(dep_section(&section), DepSection::Named(_)) && k == "package" {
            deps.insert(unquote(v));
        }
    }
    (name, deps)
}

fn read_crates(root: &Path) -> Vec<Crate> {
    let mut crates = Vec::new();
    for src in walk::crate_sources(root) {
        let Some(dir) = src.parent() else { continue };
        let (name, deps) = read_manifest(&dir.join("Cargo.toml"));
        if name.is_empty() {
            continue;
        }
        crates.push(Crate {
            name,
            dir: walk::rel(root, dir),
            cryptographic: deps.iter().any(|d| PRIMITIVE_CRATES.contains(&d.as_str())),
            src,
        });
    }
    crates
}

// ------------------------------------------------------------------ aliases

/// What a file's `use` lines make a name mean.
type Aliases = BTreeMap<String, Vec<Vec<String>>>;

#[derive(Default)]
struct Uses {
    aliases: Aliases,
}

fn flat_use(tree: &syn::UseTree, prefix: &mut Vec<String>, out: &mut Aliases) {
    match tree {
        syn::UseTree::Path(p) => {
            prefix.push(p.ident.to_string());
            flat_use(&p.tree, prefix, out);
            prefix.pop();
        }
        syn::UseTree::Name(n) => {
            let id = n.ident.to_string();
            let mut full = prefix.clone();
            if id != "self" {
                full.push(id);
            }
            if let Some(alias) = full.last().cloned() {
                out.entry(alias).or_default().push(full);
            }
        }
        syn::UseTree::Rename(r) => {
            let id = r.ident.to_string();
            let alias = r.rename.to_string();
            let mut full = prefix.clone();
            if id != "self" {
                full.push(id);
            }
            if alias != "_" {
                out.entry(alias).or_default().push(full);
            }
        }
        syn::UseTree::Glob(_) => {}
        syn::UseTree::Group(g) => {
            for t in &g.items {
                flat_use(t, prefix, out);
            }
        }
    }
}

impl<'a> Visit<'a> for Uses {
    fn visit_item_use(&mut self, u: &'a syn::ItemUse) {
        flat_use(&u.tree, &mut Vec::new(), &mut self.aliases);
    }
}

// ----------------------------------------------------------------- scanning

/// What one function's signature and body mention.
#[derive(Default)]
struct Scan {
    /// Every path, in types, expressions and macro tokens.
    paths: Vec<Vec<String>>,
    /// Paths used as expressions.
    exprs: Vec<(Vec<String>, bool, usize)>,
    /// Method calls.
    methods: Vec<(String, usize)>,
    /// Every name the function binds.
    locals: BTreeSet<String>,
}

fn segs(p: &syn::Path) -> Vec<String> {
    p.segments.iter().map(|s| s.ident.to_string()).collect()
}

fn is_punct(t: Option<&TokenTree>, c: char) -> bool {
    matches!(t, Some(TokenTree::Punct(p)) if p.as_char() == c)
}

/// The index after the `>` closing the `<` at `i`.
fn skip_angle(toks: &[TokenTree], mut i: usize) -> usize {
    let mut depth = 0usize;
    while i < toks.len() {
        if is_punct(toks.get(i), '<') {
            depth += 1;
        } else if is_punct(toks.get(i), '>') {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return i + 1;
            }
        }
        i += 1;
    }
    i
}

impl Scan {
    /// The paths, calls and method calls in a macro's tokens, which `syn`
    /// does not parse.
    fn tokens(&mut self, ts: TokenStream) {
        let toks: Vec<TokenTree> = ts.into_iter().collect();
        let mut i = 0;
        while i < toks.len() {
            match &toks[i] {
                TokenTree::Group(g) => {
                    self.tokens(g.stream());
                    i += 1;
                }
                TokenTree::Ident(id) => {
                    let line = id.span().start().line;
                    let mut path = vec![id.to_string()];
                    let mut j = i + 1;
                    while is_punct(toks.get(j), ':') && is_punct(toks.get(j + 1), ':') {
                        match toks.get(j + 2) {
                            Some(TokenTree::Ident(n)) => {
                                path.push(n.to_string());
                                j += 3;
                            }
                            Some(TokenTree::Punct(p)) if p.as_char() == '<' => {
                                j = skip_angle(&toks, j + 2);
                            }
                            _ => break,
                        }
                    }
                    let called = matches!(
                        toks.get(j),
                        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis
                    );
                    if i > 0 && is_punct(toks.get(i - 1), '.') {
                        if called {
                            self.methods.push((path.swap_remove(0), line));
                        }
                    } else {
                        self.paths.push(path.clone());
                        if called || path.len() > 1 {
                            self.exprs.push((path, called, line));
                        }
                    }
                    i = j;
                }
                _ => i += 1,
            }
        }
    }
}

impl<'a> Visit<'a> for Scan {
    fn visit_path(&mut self, p: &'a syn::Path) {
        self.paths.push(segs(p));
        visit::visit_path(self, p);
    }

    fn visit_expr_path(&mut self, p: &'a syn::ExprPath) {
        self.exprs
            .push((segs(&p.path), false, p.path.span().start().line));
        visit::visit_expr_path(self, p);
    }

    fn visit_expr_call(&mut self, c: &'a syn::ExprCall) {
        if let syn::Expr::Path(p) = &*c.func {
            self.exprs
                .push((segs(&p.path), true, p.path.span().start().line));
            self.visit_path(&p.path);
        } else {
            self.visit_expr(&c.func);
        }
        for a in &c.args {
            self.visit_expr(a);
        }
    }

    fn visit_expr_method_call(&mut self, m: &'a syn::ExprMethodCall) {
        self.methods
            .push((m.method.to_string(), m.method.span().start().line));
        visit::visit_expr_method_call(self, m);
    }

    fn visit_macro(&mut self, m: &'a syn::Macro) {
        self.paths.push(segs(&m.path));
        self.tokens(m.tokens.clone());
    }

    fn visit_pat_ident(&mut self, p: &'a syn::PatIdent) {
        self.locals.insert(p.ident.to_string());
        visit::visit_pat_ident(self, p);
    }
}

// -------------------------------------------------------------- body hashes

fn starts_with_doc(ts: TokenStream) -> bool {
    matches!(ts.into_iter().next(), Some(TokenTree::Ident(i)) if i == "doc")
}

/// The tokens of `ts` as text pieces, `#[doc …]` and `#![doc …]` left out,
/// and a `,` that ends a group left out.
fn render(ts: TokenStream, out: &mut Vec<String>) {
    let mut toks: Vec<TokenTree> = ts.into_iter().collect();
    // A `,` just before the closing delimiter is rustfmt's to add or drop.
    if is_punct(toks.last(), ',') {
        toks.pop();
    }
    let mut it = toks.into_iter();
    while let Some(t) = it.next() {
        match t {
            TokenTree::Punct(p) if p.as_char() == '#' => {
                let mut look = it.clone();
                let mut next = look.next();
                if is_punct(next.as_ref(), '!') {
                    next = look.next();
                }
                if let Some(TokenTree::Group(g)) = &next
                    && g.delimiter() == Delimiter::Bracket
                    && starts_with_doc(g.stream())
                {
                    it = look;
                    continue;
                }
                out.push("#".to_string());
            }
            TokenTree::Punct(p) => out.push(p.as_char().to_string()),
            TokenTree::Group(g) => {
                let (open, close) = match g.delimiter() {
                    Delimiter::Parenthesis => ("(", ")"),
                    Delimiter::Brace => ("{", "}"),
                    Delimiter::Bracket => ("[", "]"),
                    Delimiter::None => ("", ""),
                };
                out.push(open.to_string());
                render(g.stream(), out);
                out.push(close.to_string());
            }
            other => out.push(other.to_string()),
        }
    }
}

/// §5.1: SHA-256 over the function's tokens with one space between them:
/// attributes but `#[doc]`, visibility, signature without its name, bounds
/// and body.
fn body_hash(
    attrs: &[syn::Attribute],
    vis: &syn::Visibility,
    sig: &syn::Signature,
    block: &syn::Block,
) -> String {
    let mut ts = TokenStream::new();
    for a in attrs.iter().filter(|a| !a.path().is_ident("doc")) {
        a.to_tokens(&mut ts);
    }
    vis.to_tokens(&mut ts);
    // The signature as `syn` prints it, less the name.
    sig.constness.to_tokens(&mut ts);
    sig.asyncness.to_tokens(&mut ts);
    sig.unsafety.to_tokens(&mut ts);
    sig.abi.to_tokens(&mut ts);
    sig.fn_token.to_tokens(&mut ts);
    sig.generics.to_tokens(&mut ts);
    sig.paren_token.surround(&mut ts, |t| {
        sig.inputs.to_tokens(t);
        sig.variadic.to_tokens(t);
    });
    sig.output.to_tokens(&mut ts);
    sig.generics.where_clause.to_tokens(&mut ts);
    block.to_tokens(&mut ts);
    let mut pieces = Vec::new();
    render(ts, &mut pieces);
    sha256::hex(pieces.join(" ").as_bytes())
}

// -------------------------------------------------------------------- rules

/// What a file is, for the rules.
struct FileCx {
    krate: usize,
    file: String,
    own_lib: String,
    aliases: Aliases,
    /// Rules 1 and 4 apply.
    token: bool,
    /// Rule 2 applies.
    reveal: bool,
    /// The crate is cryptographic: functions are recorded.
    record: bool,
    /// The whole file is compiled for tests only.
    test_file: bool,
    /// The `cfg` that makes it so.
    base_cfg: String,
}

impl FileCx {
    /// A path with its first name replaced by what a `use` line made it.
    fn resolve(&self, path: &[String]) -> Vec<Vec<String>> {
        match self.aliases.get(&path[0]) {
            Some(targets) => targets
                .iter()
                .map(|t| t.iter().chain(&path[1..]).cloned().collect())
                .collect(),
            None => vec![path.to_vec()],
        }
    }

    /// Whether `path` is a path into a primitive crate.
    fn primitive(&self, path: &[String]) -> bool {
        self.resolve(path).iter().any(|full| {
            primitive_lib(&full[0])
                && full[0] != self.own_lib
                && !(full[0] == "faraday_crypto"
                    && full
                        .get(1)
                        .is_some_and(|s| NOT_COMPUTING.contains(&s.as_str())))
        })
    }
}

fn has_ident(ty: &syn::Type, names: &[&str]) -> bool {
    let mut found = false;
    let mut stack = vec![ty.to_token_stream()];
    while let Some(ts) = stack.pop() {
        for t in ts {
            match t {
                TokenTree::Ident(i) if names.iter().any(|n| i == n) => found = true,
                TokenTree::Group(g) => stack.push(g.stream()),
                _ => {}
            }
        }
    }
    found
}

fn takes_compute(sig: &syn::Signature) -> bool {
    sig.inputs.iter().any(|i| match i {
        syn::FnArg::Typed(t) => match &*t.ty {
            syn::Type::Reference(r) => walk::last_ident(&r.elem).is_some_and(|n| n == "Compute"),
            _ => false,
        },
        syn::FnArg::Receiver(_) => false,
    })
}

fn reveals(sig: &syn::Signature) -> bool {
    let takes = sig.inputs.iter().any(|i| match i {
        syn::FnArg::Typed(t) => has_ident(&t.ty, &["Reveal"]),
        syn::FnArg::Receiver(_) => false,
    });
    let returns = match &sig.output {
        syn::ReturnType::Type(_, t) => has_ident(t, &["Revealed", "Held"]),
        syn::ReturnType::Default => false,
    };
    takes || returns
}

// ----------------------------------------------------------------- finding

struct Finder<'a> {
    cx: &'a FileCx,
    funcs: &'a mut Vec<Func>,
    tests: &'a mut Vec<TestFn>,
    mods: Vec<String>,
    cfgs: Vec<String>,
    self_ty: Option<String>,
}

impl Finder<'_> {
    fn test_only(&self) -> bool {
        self.cx.test_file || !self.cfgs.is_empty()
    }

    fn cfg_of(&self, attrs: &[syn::Attribute]) -> String {
        let own = walk::cfg_text(attrs);
        let mut parts: Vec<&str> = vec![&self.cx.base_cfg];
        parts.extend(self.cfgs.iter().map(String::as_str));
        parts.push(&own);
        parts
            .into_iter()
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join(" & ")
    }

    fn function(
        &mut self,
        attrs: &[syn::Attribute],
        vis: &syn::Visibility,
        sig: &syn::Signature,
        block: &syn::Block,
    ) {
        let line = sig.ident.span().start().line;
        if attrs.iter().any(|a| a.path().is_ident("test")) {
            let mut name = self.mods.join("::");
            if !name.is_empty() {
                name.push_str("::");
            }
            name.push_str(&sig.ident.to_string());
            self.tests.push(TestFn {
                krate: self.cx.krate,
                file: self.cx.file.clone(),
                name,
                line,
                ignored: attrs.iter().any(|a| a.path().is_ident("ignore")),
                cfg: self.cfg_of(attrs),
            });
            return;
        }
        if self.test_only() || walk::is_test(attrs) || !self.cx.record {
            return;
        }
        let mut scan = Scan::default();
        scan.visit_signature(sig);
        scan.visit_block(block);
        self.funcs
            .push(self.analyze(attrs, vis, sig, block, line, scan));
    }

    fn analyze(
        &self,
        attrs: &[syn::Attribute],
        vis: &syn::Visibility,
        sig: &syn::Signature,
        block: &syn::Block,
        line: usize,
        scan: Scan,
    ) -> Func {
        let cx = self.cx;
        let ident = sig.ident.to_string();
        let mut rules = BTreeSet::new();
        if cx.token && takes_compute(sig) {
            rules.insert(1);
        }
        if cx.reveal && reveals(sig) {
            rules.insert(2);
        }
        if scan.paths.iter().any(|p| cx.primitive(p)) {
            rules.insert(3);
        }
        let mut calls = Vec::new();
        for (path, called, line) in &scan.exprs {
            if path.len() == 1 && scan.locals.contains(&path[0]) {
                continue;
            }
            let mut path = path.clone();
            if path[0] == "Self" {
                match &self.self_ty {
                    Some(t) => path[0] = t.clone(),
                    None => continue,
                }
            }
            for segs in cx.resolve(&path) {
                calls.push(Call {
                    segs,
                    called: *called,
                    line: *line,
                });
            }
        }
        Func {
            krate: cx.krate,
            file: cx.file.clone(),
            name: match &self.self_ty {
                Some(t) => format!("{t}::{ident}"),
                None => ident.clone(),
            },
            ident,
            ty: self.self_ty.clone(),
            line,
            public: matches!(vis, syn::Visibility::Public(_)),
            rules,
            hash: body_hash(attrs, vis, sig, block),
            callees: BTreeSet::new(),
            cross: BTreeSet::new(),
            calls,
            idents: scan.paths.iter().flatten().cloned().collect(),
            methods: scan.methods,
        }
    }

    fn items(&mut self, items: &[syn::Item]) {
        for i in items {
            self.visit_item(i);
        }
    }
}

impl<'a> Visit<'a> for Finder<'_> {
    fn visit_item_mod(&mut self, m: &'a syn::ItemMod) {
        let Some((_, items)) = &m.content else { return };
        let test = walk::is_test(&m.attrs);
        self.mods.push(m.ident.to_string());
        if test {
            self.cfgs.push(walk::cfg_text(&m.attrs));
        }
        self.items(items);
        if test {
            self.cfgs.pop();
        }
        self.mods.pop();
    }

    fn visit_item_fn(&mut self, f: &'a syn::ItemFn) {
        let saved = self.self_ty.take();
        self.function(&f.attrs, &f.vis, &f.sig, &f.block);
        self.self_ty = saved;
    }

    fn visit_item_impl(&mut self, i: &'a syn::ItemImpl) {
        let test = walk::is_test(&i.attrs);
        if test {
            self.cfgs.push(walk::cfg_text(&i.attrs));
        }
        let saved = self.self_ty.replace(
            walk::last_ident(&i.self_ty).unwrap_or_else(|| i.self_ty.to_token_stream().to_string()),
        );
        for it in &i.items {
            if let syn::ImplItem::Fn(f) = it {
                self.function(&f.attrs, &f.vis, &f.sig, &f.block);
            }
        }
        self.self_ty = saved;
        if test {
            self.cfgs.pop();
        }
    }

    fn visit_item_trait(&mut self, t: &'a syn::ItemTrait) {
        let test = walk::is_test(&t.attrs);
        if test {
            self.cfgs.push(walk::cfg_text(&t.attrs));
        }
        let saved = self.self_ty.replace(t.ident.to_string());
        for it in &t.items {
            if let syn::TraitItem::Fn(f) = it
                && let Some(block) = &f.default
            {
                self.function(&f.attrs, &syn::Visibility::Inherited, &f.sig, block);
            }
        }
        self.self_ty = saved;
        if test {
            self.cfgs.pop();
        }
    }
}

// ------------------------------------------------------------------ linking

/// Whether `lib` is a primitive crate's library name (its package name
/// with `_` for `-`), without allocating: asked of every path.
fn primitive_lib(lib: &str) -> bool {
    PRIMITIVE_CRATES.iter().any(|p| {
        p.len() == lib.len()
            && p.bytes()
                .zip(lib.bytes())
                .all(|(a, b)| a == b || (a == b'-' && b == b'_'))
    })
}

fn external_head(head: &str) -> bool {
    matches!(head, "std" | "core" | "alloc") || primitive_lib(head)
}

/// Whether `f` is what a call by `segs` (a path) names: the last name, and
/// the segment before it if it is a type.
fn names(f: &Func, segs: &[String]) -> bool {
    let Some(last) = segs.last() else {
        return false;
    };
    if f.ident != *last {
        return false;
    }
    match segs.len().checked_sub(2).map(|i| &segs[i]) {
        None => true,
        Some(ty) => f.ty.as_deref() == Some(ty.as_str()) || f.ty.is_none(),
    }
}

/// Same-crate callees and cross-crate calls of every function.
fn link(inv: &mut Inventory) {
    let libs: HashMap<String, usize> = inv
        .crates
        .iter()
        .enumerate()
        .map(|(i, c)| (lib_name(&c.name), i))
        .collect();
    let mut by_name: HashMap<(usize, String), Vec<usize>> = HashMap::new();
    for (i, f) in inv.funcs.iter().enumerate() {
        by_name
            .entry((f.krate, f.ident.clone()))
            .or_default()
            .push(i);
    }
    for i in 0..inv.funcs.len() {
        let c = inv.funcs[i].krate;
        let mut callees = BTreeSet::new();
        let mut cross = BTreeSet::new();
        for call in &inv.funcs[i].calls {
            let head = call.segs[0].as_str();
            if let Some(&ci) = libs.get(head)
                && ci != c
            {
                if call.called {
                    cross.insert(format!(
                        "{}::{}",
                        inv.crates[ci].name,
                        call.segs[1..].join("::")
                    ));
                }
                continue;
            }
            if external_head(head) {
                continue;
            }
            let last = &call.segs[call.segs.len() - 1];
            for &j in by_name.get(&(c, last.clone())).into_iter().flatten() {
                if names(&inv.funcs[j], &call.segs) {
                    callees.insert(j);
                }
            }
        }
        for (m, _) in &inv.funcs[i].methods {
            callees.extend(by_name.get(&(c, m.clone())).into_iter().flatten());
        }
        callees.remove(&i);
        inv.funcs[i].callees = callees;
        inv.funcs[i].cross = cross;
    }
}

/// §4.2 rule 4: a public function of a token crate that, through calls in
/// its crate, reaches a function of rules 1 to 3.
fn public_reach(inv: &mut Inventory) {
    let mut reaching: Vec<bool> = inv.funcs.iter().map(|f| !f.rules.is_empty()).collect();
    loop {
        let mut changed = false;
        for i in 0..inv.funcs.len() {
            if !reaching[i] && inv.funcs[i].callees.iter().any(|&j| reaching[j]) {
                reaching[i] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for (i, f) in inv.funcs.iter_mut().enumerate() {
        if reaching[i] && f.public && f.rules.is_empty() && token_crate(&inv.crates[f.krate]) {
            f.rules.insert(4);
        }
    }
}

fn token_crate(c: &Crate) -> bool {
    c.cryptographic && (c.dir.starts_with("core/") || TOKEN_CRATES.contains(&c.name.as_str()))
}

/// §9.2: calls in the app of cryptographic functions of `faraday-keys` and
/// `core/`. A path call is resolved through the file's `use` lines. A
/// method call is taken by its name, for a method of a type the file
/// names (in a `use` line or a path): without types, the name alone would
/// take every `push` and `get`.
fn call_sites(inv: &mut Inventory, imports: &HashMap<String, BTreeSet<String>>) {
    let target = |c: &Crate| c.name == KEYS_CRATE || c.dir.starts_with("core/");
    let libs: HashMap<String, usize> = inv
        .crates
        .iter()
        .enumerate()
        .filter(|(_, c)| target(c))
        .map(|(i, c)| (lib_name(&c.name), i))
        .collect();
    let crypto: Vec<usize> = (0..inv.funcs.len())
        .filter(|&i| {
            !inv.funcs[i].rules.is_empty() && libs.values().any(|&c| c == inv.funcs[i].krate)
        })
        .collect();
    let mut mentioned: HashMap<&str, HashSet<&str>> = HashMap::new();
    for f in &inv.funcs {
        if inv.crates[f.krate].name == APP_CRATE {
            let set = mentioned.entry(&f.file).or_default();
            set.extend(f.idents.iter().map(String::as_str));
        }
    }
    for (file, set) in &mut mentioned {
        if let Some(names) = imports.get(*file) {
            set.extend(names.iter().map(String::as_str));
        }
    }
    let mut methods: HashMap<&str, Vec<usize>> = HashMap::new();
    for &j in &crypto {
        if inv.funcs[j].ty.is_some() {
            methods.entry(&inv.funcs[j].ident).or_default().push(j);
        }
    }
    let mut sites = Vec::new();
    for (i, f) in inv.funcs.iter().enumerate() {
        if inv.crates[f.krate].name != APP_CRATE {
            continue;
        }
        for call in f.calls.iter().filter(|c| c.called) {
            let Some(&ci) = libs.get(&call.segs[0]) else {
                continue;
            };
            let rest = &call.segs[1..];
            for &j in &crypto {
                let f = &inv.funcs[j];
                if f.krate == ci
                    && !rest.is_empty()
                    && (rest.len() > 1 || f.ty.is_none())
                    && names(f, rest)
                {
                    sites.push(Site {
                        caller: i,
                        callee: j,
                        line: call.line,
                        via: "path",
                    });
                }
            }
        }
        for (m, line) in &f.methods {
            for &j in methods.get(m.as_str()).into_iter().flatten() {
                let named = inv.funcs[j].ty.as_deref().is_some_and(|t| {
                    mentioned
                        .get(f.file.as_str())
                        .is_some_and(|s| s.contains(t))
                });
                if named {
                    sites.push(Site {
                        caller: i,
                        callee: j,
                        line: *line,
                        via: "method",
                    });
                }
            }
        }
    }
    inv.sites = sites;
}

// ----------------------------------------------------------------- scanning

/// What the files read so far hold.
#[derive(Default)]
struct Found {
    funcs: Vec<Func>,
    tests: Vec<TestFn>,
    /// Per file: the names its `use` lines bring in.
    imports: HashMap<String, BTreeSet<String>>,
}

fn parse_file(found: &mut Found, file: &Path, mut cx: FileCx, root: &Path) {
    let Some(ast) = walk::parse(file) else { return };
    let mut uses = Uses::default();
    uses.visit_file(&ast);
    cx.file = walk::rel(root, file);
    found
        .imports
        .insert(cx.file.clone(), uses.aliases.keys().cloned().collect());
    cx.aliases = uses.aliases;
    let mut f = Finder {
        cx: &cx,
        funcs: &mut found.funcs,
        tests: &mut found.tests,
        mods: Vec::new(),
        cfgs: Vec::new(),
        self_ty: None,
    };
    f.items(&ast.items);
}

/// Reads every crate under `root` (costs a parse of each source file).
pub(crate) fn scan(root: &Path) -> Inventory {
    let crates = read_crates(root);
    let mut found = Found::default();
    for (k, c) in crates.iter().enumerate() {
        let base = |test_file: bool, cfg: &str| FileCx {
            krate: k,
            file: String::new(),
            own_lib: lib_name(&c.name),
            aliases: Aliases::new(),
            token: token_crate(c),
            reveal: c.cryptographic && c.name == KEYS_CRATE,
            record: c.cryptographic,
            test_file,
            base_cfg: cfg.to_string(),
        };
        for m in walk::module_files_cfg(root, &c.src) {
            parse_file(&mut found, &m.path, base(m.test, &m.cfg), root);
        }
        if let Some(dir) = c.src.parent() {
            for f in walk::rust_files(&dir.join("tests")) {
                parse_file(&mut found, &f, base(true, ""), root);
            }
        }
    }
    let Found {
        mut funcs,
        mut tests,
        imports,
    } = found;
    funcs.sort_by(|a, b| {
        (a.krate, &a.file, a.line, &a.name).cmp(&(b.krate, &b.file, b.line, &b.name))
    });
    tests.sort_by(|a, b| (&a.file, a.line, &a.name).cmp(&(&b.file, b.line, &b.name)));
    let mut inv = Inventory {
        crates,
        funcs,
        tests,
        sites: Vec::new(),
    };
    link(&mut inv);
    public_reach(&mut inv);
    call_sites(&mut inv, &imports);
    inv
}

// ------------------------------------------------------------------- output

fn js(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn js_list<I: IntoIterator<Item = String>>(items: I) -> String {
    format!("[{}]", items.into_iter().collect::<Vec<_>>().join(","))
}

impl Inventory {
    /// `file::Type::name`, as the manifest writes a function.
    pub(crate) fn id(&self, i: usize) -> String {
        format!("{}::{}", self.funcs[i].file, self.funcs[i].name)
    }

    /// The functions to print: the cryptographic ones and everything they
    /// reach in their crate, or all of them.
    fn printed(&self, all: bool) -> Vec<usize> {
        let mut keep: HashSet<usize> = HashSet::new();
        let mut stack: Vec<usize> = Vec::new();
        for (i, f) in self.funcs.iter().enumerate() {
            if all || !f.rules.is_empty() {
                keep.insert(i);
                stack.push(i);
            }
        }
        while let Some(i) = stack.pop() {
            for &j in &self.funcs[i].callees {
                if keep.insert(j) {
                    stack.push(j);
                }
            }
        }
        let mut out: Vec<usize> = keep.into_iter().collect();
        out.sort_unstable();
        out
    }

    /// The JSON lines.
    pub(crate) fn lines(&self, all: bool) -> Vec<String> {
        let mut out = Vec::new();
        for i in self.printed(all) {
            let f = &self.funcs[i];
            out.push(format!(
                "{{\"kind\":\"fn\",\"crate\":{},\"file\":{},\"name\":{},\"id\":{},\"line\":{},\"public\":{},\"rules\":{},\"hash\":{},\"callees\":{},\"cross\":{}}}",
                js(&self.crates[f.krate].name),
                js(&f.file),
                js(&f.name),
                js(&self.id(i)),
                f.line,
                f.public,
                js_list(f.rules.iter().map(|r| r.to_string())),
                js(&f.hash),
                js_list(f.callees.iter().map(|&j| js(&self.id(j)))),
                js_list(f.cross.iter().map(|c| js(c))),
            ));
        }
        for t in &self.tests {
            out.push(format!(
                "{{\"kind\":\"test\",\"crate\":{},\"file\":{},\"name\":{},\"line\":{},\"ignored\":{},\"cfg\":{}}}",
                js(&self.crates[t.krate].name),
                js(&t.file),
                js(&t.name),
                t.line,
                t.ignored,
                js(&t.cfg),
            ));
        }
        for s in &self.sites {
            let f = &self.funcs[s.caller];
            out.push(format!(
                "{{\"kind\":\"call_site\",\"file\":{},\"module\":{},\"in\":{},\"line\":{},\"callee\":{},\"via\":{}}}",
                js(&f.file),
                js(&self.module_of(&f.file)),
                js(&self.id(s.caller)),
                s.line,
                js(&self.id(s.callee)),
                js(s.via),
            ));
        }
        out
    }

    /// A file's module path inside its crate: `create`, `keygen::words`.
    fn module_of(&self, file: &str) -> String {
        let after = file.split_once("/src/").map_or(file, |(_, rest)| rest);
        let m = after.trim_end_matches(".rs").trim_end_matches("/mod");
        m.replace('/', "::")
    }

    /// One text line per test function, for the self-test.
    pub(crate) fn test_lines(&self) -> Vec<String> {
        self.tests
            .iter()
            .map(|t| format!("{} {} ignored={} cfg={}", t.file, t.name, t.ignored, t.cfg))
            .collect()
    }

    /// One text line per call site, for the self-test.
    pub(crate) fn site_lines(&self) -> Vec<String> {
        self.sites
            .iter()
            .map(|s| format!("{} -> {} {}", self.id(s.caller), self.id(s.callee), s.via))
            .collect()
    }

    /// Counts of cryptographic functions, by rule and by crate.
    pub(crate) fn summary(&self) -> String {
        let mut by_rule: BTreeMap<u8, usize> = BTreeMap::new();
        let mut by_crate: BTreeMap<&str, usize> = BTreeMap::new();
        let mut total = 0;
        for f in self.funcs.iter().filter(|f| !f.rules.is_empty()) {
            total += 1;
            *by_crate.entry(&self.crates[f.krate].name).or_default() += 1;
            for r in &f.rules {
                *by_rule.entry(*r).or_default() += 1;
            }
        }
        let rules: Vec<String> = by_rule
            .iter()
            .map(|(r, n)| format!("rule {r}: {n}"))
            .collect();
        let crates: Vec<String> = by_crate
            .iter()
            .map(|(c, n)| format!("  {c}: {n}"))
            .collect();
        format!(
            "crypto-inventory: {total} cryptographic function(s) ({}); {} test(s); {} call site(s) in {APP_CRATE}\n{}",
            rules.join(", "),
            self.tests.len(),
            self.sites.len(),
            crates.join("\n"),
        )
    }
}

/// The `crypto-inventory` binary.
pub fn main() -> ExitCode {
    let mut root = PathBuf::from(".");
    let mut all = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--self-test" => return crate::selftest::run(),
            "--all" => all = true,
            "--root" => root = args.next().map(PathBuf::from).unwrap_or(root),
            other => {
                eprintln!("crypto-inventory: unknown argument {other}");
                return ExitCode::FAILURE;
            }
        }
    }
    let inv = scan(&root);
    for l in inv.lines(all) {
        println!("{l}");
    }
    eprintln!("{}", inv.summary());
    if walk::parse_failures() > 0 {
        eprintln!(
            "crypto-inventory: {} file(s) did not parse; the inventory is incomplete",
            walk::parse_failures()
        );
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
