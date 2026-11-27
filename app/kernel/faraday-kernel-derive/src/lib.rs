//! The application kernel's derive crate: `Message`, `Public` and `AtLock` (03 §1, §2.3, §3).
//!
//! The derives read the item's name, generics and where clause from the token stream (the
//! workspace takes no outside dependency, so there is no `syn`) and write impls from the fields
//! and variants they read. `Public` writes `Clone` and `Debug` as well, since `Public` needs both
//! and the sketches derive it beside `Copy` alone (03 §3.3). `Message` writes a `Debug` that
//! names the variant and leaves its fields out, since a message's fields are not yet held to
//! `Public` (the check of 03 §3.1 is not built).

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

use proc_macro::{Delimiter, TokenStream, TokenTree};

/// What a derive needs of the item it is on: its name, its generics as written (bounds kept,
/// defaults dropped), the generics' arguments and its where clause.
/// One field of a struct or a variant: its name (none in a tuple) and its `#[lock(..)]` mark.
struct Field {
    name: Option<String>,
    lock: Option<String>,
}

/// What a struct or a variant holds.
enum Shape {
    Unit,
    Tuple(Vec<Field>),
    Named(Vec<Field>),
}

struct Variant {
    name: String,
    withheld: bool,
    shape: Shape,
}

enum Body {
    Struct(Shape),
    Enum(Vec<Variant>),
}

/// What a derive needs of the item it is on: its name, its generics as written (bounds kept,
/// defaults dropped), the generics' arguments, its where clause and its body.
struct Item {
    name: String,
    decl: String,
    args: String,
    wher: String,
    body: Body,
}

/// The text of an attribute group's inside, such as `lock(kept)`, without spaces.
fn attr_text(g: &proc_macro::Group) -> String {
    g.stream()
        .to_string()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// The attributes and the first name found in one comma-separated piece (a field or a variant).
struct Piece {
    attrs: Vec<String>,
    name: Option<String>,
    group: Option<(Delimiter, TokenStream)>,
}

/// Splits a body at its top-level commas (angle brackets count as a level) and reads each piece.
fn pieces(body: TokenStream) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut cur = Piece {
        attrs: Vec::new(),
        name: None,
        group: None,
    };
    let mut seen = false;
    let mut depth = 0i32;
    let mut after_dash = false;
    let mut it = body.into_iter().peekable();
    while let Some(t) = it.next() {
        match &t {
            TokenTree::Punct(p) if p.as_char() == '#' && cur.name.is_none() => {
                if let Some(TokenTree::Group(g)) = it.peek() {
                    if g.delimiter() == Delimiter::Bracket {
                        cur.attrs.push(attr_text(g));
                    }
                    it.next();
                }
                seen = true;
                continue;
            }
            TokenTree::Punct(p) => {
                match p.as_char() {
                    '<' => depth += 1,
                    '>' if !after_dash => depth -= 1,
                    ',' if depth == 0 => {
                        out.push(core::mem::replace(
                            &mut cur,
                            Piece {
                                attrs: Vec::new(),
                                name: None,
                                group: None,
                            },
                        ));
                        seen = false;
                        after_dash = false;
                        continue;
                    }
                    _ => {}
                }
                after_dash = p.as_char() == '-';
                seen = true;
                continue;
            }
            TokenTree::Ident(i) => {
                let s = i.to_string();
                if cur.name.is_none() && s != "pub" {
                    cur.name = Some(s);
                }
                seen = true;
            }
            TokenTree::Group(g) => {
                if cur.name.is_some() && cur.group.is_none() && depth == 0 {
                    cur.group = Some((g.delimiter(), g.stream()));
                }
                seen = true;
            }
            TokenTree::Literal(_) => seen = true,
        }
        after_dash = false;
    }
    if seen {
        out.push(cur);
    }
    out
}

fn lock_mark(attrs: &[String]) -> Option<String> {
    attrs.iter().find_map(|a| {
        a.strip_prefix("lock(")
            .and_then(|r| r.strip_suffix(')'))
            .map(String::from)
    })
}

/// The fields of a braced body (named) or a parenthesised one (tuple).
fn fields(body: TokenStream, named: bool) -> Vec<Field> {
    let mut out = Vec::new();
    for (n, piece) in pieces(body).into_iter().enumerate() {
        let name = if named {
            piece.name
        } else {
            Some(n.to_string())
        };
        out.push(Field {
            name,
            lock: lock_mark(&piece.attrs),
        });
    }
    out
}

fn shape_of(group: Option<(Delimiter, TokenStream)>) -> Shape {
    match group {
        Some((Delimiter::Parenthesis, s)) => Shape::Tuple(fields(s, false)),
        Some((Delimiter::Brace, s)) => Shape::Named(fields(s, true)),
        _ => Shape::Unit,
    }
}

fn variants(body: TokenStream) -> Vec<Variant> {
    pieces(body)
        .into_iter()
        .filter_map(|p| {
            Some(Variant {
                withheld: p.attrs.iter().any(|a| a == "withheld"),
                name: p.name?,
                shape: shape_of(p.group),
            })
        })
        .collect()
}

impl Item {
    /// `impl<..> Trait for Name<..> where ..` up to the body.
    fn header(&self, trait_path: &str, extra: &str) -> String {
        let decl = if self.decl.is_empty() {
            String::new()
        } else {
            format!("<{}>", self.decl)
        };
        let args = if self.args.is_empty() {
            String::new()
        } else {
            format!("<{}>", self.args)
        };
        let mut bounds: Vec<&str> = Vec::new();
        if !extra.is_empty() {
            bounds.push(extra);
        }
        if !self.wher.is_empty() {
            bounds.push(&self.wher);
        }
        let wher = if bounds.is_empty() {
            String::new()
        } else {
            format!("where {}", bounds.join(", "))
        };
        format!("impl{decl} {trait_path} for {}{args} {wher}", self.name)
    }
}

/// The pattern that matches a variant (or the struct) and binds each field to `f0`, `f1`, ..
/// (the binding of field `n`, named or not).
fn pattern(path: &str, shape: &Shape) -> String {
    match shape {
        Shape::Unit => path.to_string(),
        Shape::Tuple(fs) => {
            let b: Vec<String> = (0..fs.len()).map(|n| format!("f{n}")).collect();
            format!("{path}({})", b.join(", "))
        }
        Shape::Named(fs) => {
            let b: Vec<String> = fs
                .iter()
                .enumerate()
                .map(|(n, f)| format!("{}: f{n}", f.name.clone().unwrap_or_default()))
                .collect();
            format!("{path} {{ {} }}", b.join(", "))
        }
    }
}

/// The same value built from the bindings, each passed through `wrap`.
fn rebuild(path: &str, shape: &Shape, wrap: &dyn Fn(&str) -> String) -> String {
    match shape {
        Shape::Unit => path.to_string(),
        Shape::Tuple(fs) => {
            let b: Vec<String> = (0..fs.len()).map(|n| wrap(&format!("f{n}"))).collect();
            format!("{path}({})", b.join(", "))
        }
        Shape::Named(fs) => {
            let b: Vec<String> = fs
                .iter()
                .enumerate()
                .map(|(n, f)| format!("{}: {}", f.name.clone().unwrap_or_default(), wrap(&format!("f{n}"))))
                .collect();
            format!("{path} {{ {} }}", b.join(", "))
        }
    }
}

/// A `match self` over every shape of the item, one arm each, from `arm(path, name, shape)`.
fn match_self(item: &Item, bind: bool, arm: &dyn Fn(&str, &str, &Shape) -> String) -> String {
    let pat = |path: &str, shape: &Shape| {
        if bind {
            pattern(path, shape)
        } else {
            pattern_any(path, shape)
        }
    };
    match &item.body {
        Body::Struct(shape) => {
            let p = pat("Self", shape);
            format!("match self {{ {p} => {{ {} }} }}", arm("Self", &item.name, shape))
        }
        Body::Enum(vs) if vs.is_empty() => String::from("match *self {}"),
        Body::Enum(vs) => {
            let arms: Vec<String> = vs
                .iter()
                .map(|v| {
                    let path = format!("Self::{}", v.name);
                    format!("{} => {{ {} }}", pat(&path, &v.shape), arm(&path, &v.name, &v.shape))
                })
                .collect();
            format!("match self {{ {} }}", arms.join(", "))
        }
    }
}

fn parse(input: TokenStream) -> Item {
    let mut it = input.into_iter().peekable();
    let mut name = String::new();
    let mut is_enum = false;
    while let Some(t) = it.next() {
        if let TokenTree::Ident(i) = &t {
            let s = i.to_string();
            if s == "struct" || s == "enum" {
                is_enum = s == "enum";
                if let Some(TokenTree::Ident(n)) = it.next() {
                    name = n.to_string();
                }
                break;
            }
        }
    }
    let mut decl = String::new();
    let mut args: Vec<String> = Vec::new();
    if matches!(it.peek(), Some(TokenTree::Punct(p)) if p.as_char() == '<') {
        it.next();
        let mut depth = 1;
        let mut at_start = true;
        let mut in_default = false;
        let mut arg = String::new();
        for t in it.by_ref() {
            if let TokenTree::Punct(p) = &t {
                match p.as_char() {
                    '<' => depth += 1,
                    '>' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    ',' if depth == 1 => {
                        decl.push_str(", ");
                        args.push(core::mem::take(&mut arg));
                        at_start = true;
                        in_default = false;
                        continue;
                    }
                    '=' if depth == 1 => in_default = true,
                    _ => {}
                }
            }
            if in_default {
                continue;
            }
            if at_start {
                match &t {
                    TokenTree::Punct(p) if p.as_char() == '\'' => arg.push('\''),
                    TokenTree::Ident(i) if i.to_string() != "const" => {
                        arg.push_str(&i.to_string());
                        at_start = false;
                    }
                    _ => {}
                }
            }
            decl.push_str(&t.to_string());
            decl.push(' ');
        }
        if !arg.is_empty() {
            args.push(arg);
        }
    }
    let mut wher = String::new();
    let mut in_where = false;
    let mut body = Body::Struct(Shape::Unit);
    for t in it {
        match &t {
            TokenTree::Ident(i) if i.to_string() == "where" => {
                in_where = true;
                continue;
            }
            TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
                body = if is_enum {
                    Body::Enum(variants(g.stream()))
                } else {
                    Body::Struct(Shape::Named(fields(g.stream(), true)))
                };
                break;
            }
            TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis && !is_enum && !in_where => {
                body = Body::Struct(Shape::Tuple(fields(g.stream(), false)));
                continue;
            }
            TokenTree::Punct(p) if p.as_char() == ';' => break,
            _ => {}
        }
        if in_where {
            wher.push_str(&t.to_string());
            wher.push(' ');
        }
    }
    Item {
        name,
        decl,
        args: args.join(", "),
        wher: wher.trim().trim_end_matches(',').to_string(),
        body,
    }
}

/// The debug text of a shape: its name and, where `fields` is set, each field's `Debug`.
fn debug_arm(name: &str, shape: &Shape, with_fields: bool) -> String {
    match shape {
        Shape::Unit => format!("f.write_str(\"{name}\")"),
        _ if !with_fields => format!("f.debug_struct(\"{name}\").finish_non_exhaustive()"),
        Shape::Tuple(fs) => {
            let mut s = format!("f.debug_tuple(\"{name}\")");
            for n in 0..fs.len() {
                s.push_str(&format!(".field(f{n})"));
            }
            s.push_str(".finish()");
            s
        }
        Shape::Named(fs) => {
            let mut s = format!("f.debug_struct(\"{name}\")");
            for (n, fd) in fs.iter().enumerate() {
                s.push_str(&format!(
                    ".field(\"{}\", f{n})",
                    fd.name.clone().unwrap_or_default()
                ));
            }
            s.push_str(".finish()");
            s
        }
    }
}

/// `#[derive(Message)]`: every variant is public unless marked `#[withheld]` (03 §3.1).
#[proc_macro_derive(Message, attributes(withheld))]
pub fn message(input: TokenStream) -> TokenStream {
    let item = parse(input);
    let header = item.header("::faraday_kernel::Message", "");
    let debug = item.header("::core::fmt::Debug", "");
    let (kind, public, shown) = match &item.body {
        Body::Struct(_) => (
            format!("concat!(env!(\"CARGO_PKG_NAME\"), \".{}\")", item.name),
            String::from("::core::option::Option::Some(self)"),
            match_self(&item, false, &|_, name, shape| debug_arm(name, shape, false)),
        ),
        Body::Enum(vs) if vs.is_empty() => (
            String::from("match *self {}"),
            String::from("match *self {}"),
            String::from("match *self {}"),
        ),
        Body::Enum(vs) => {
            let kinds: Vec<String> = vs
                .iter()
                .map(|v| {
                    format!(
                        "{} => concat!(env!(\"CARGO_PKG_NAME\"), \".{}\")",
                        pattern_any(&format!("Self::{}", v.name), &v.shape),
                        v.name
                    )
                })
                .collect();
            let publics: Vec<String> = vs
                .iter()
                .map(|v| {
                    let out = if v.withheld {
                        "::core::option::Option::None"
                    } else {
                        "::core::option::Option::Some(self)"
                    };
                    format!("{} => {out}", pattern_any(&format!("Self::{}", v.name), &v.shape))
                })
                .collect();
            (
                format!("match self {{ {} }}", kinds.join(", ")),
                format!("match self {{ {} }}", publics.join(", ")),
                match_self(&item, false, &|_, name, shape| debug_arm(name, shape, false)),
            )
        }
    };
    let code = format!(
        "{header} {{
            fn kind(&self) -> &'static str {{ {kind} }}
            fn public(&self) -> ::core::option::Option<&dyn ::core::fmt::Debug> {{ {public} }}
        }}
        {debug} {{
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {{ {shown} }}
        }}"
    );
    code.parse().unwrap_or_default()
}

/// A pattern that matches the variant whatever it holds.
fn pattern_any(path: &str, shape: &Shape) -> String {
    match shape {
        Shape::Unit => path.to_string(),
        Shape::Tuple(_) => format!("{path}(..)"),
        Shape::Named(_) => format!("{path} {{ .. }}"),
    }
}

/// `#[derive(Public)]`: `Public` and its sealing, with the `Clone` and `Debug` it needs (03 §3).
#[proc_macro_derive(Public)]
pub fn public(input: TokenStream) -> TokenStream {
    let item = parse(input);
    let sealed = item.header("::faraday_kernel::__private::Sealed", "");
    let public = item.header("::faraday_kernel::Public", "");
    let clone = item.header("::core::clone::Clone", "");
    let debug = item.header("::core::fmt::Debug", "");
    let cloned = match_self(&item, true, &|path, _, shape| {
        rebuild(path, shape, &|b| format!("::core::clone::Clone::clone({b})"))
    });
    let shown = match_self(&item, true, &|_, name, shape| debug_arm(name, shape, true));
    let code = format!(
        "{sealed} {{}}
        {public} {{}}
        {clone} {{ fn clone(&self) -> Self {{ {cloned} }} }}
        {debug} {{
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {{ {shown} }}
        }}"
    );
    code.parse().unwrap_or_default()
}

/// `#[derive(AtLock)]`: `holds`, `hide` and `at_lock` from each field's `#[lock(..)]` mark
/// (03 §2.3). A marked field is itself an `AtLock` (a child's state); a `public` one is left.
#[proc_macro_derive(AtLock, attributes(lock))]
pub fn at_lock(input: TokenStream) -> TokenStream {
    let item = parse(input);
    let header = item.header("::faraday_kernel::AtLock", "");
    let marked: Vec<String> = match &item.body {
        Body::Struct(Shape::Named(fs)) | Body::Struct(Shape::Tuple(fs)) => fs
            .iter()
            .filter(|f| f.lock.as_deref().is_some_and(|m| m != "public"))
            .map(|f| format!("self.{}", f.name.clone().unwrap_or_default()))
            .collect(),
        _ => Vec::new(),
    };
    let mut holds = String::from(
        "let mut h = ::faraday_kernel::Holds { kept: false, typed: false, session: false, asks_first: ::core::option::Option::None };",
    );
    let mut hide = String::new();
    let mut wipe = String::new();
    for f in &marked {
        holds.push_str(&format!(
            "let c = ::faraday_kernel::AtLock::holds(&{f}); h.kept |= c.kept; h.typed |= c.typed; h.session |= c.session; h.asks_first = h.asks_first.or(c.asks_first);"
        ));
        hide.push_str(&format!("::faraday_kernel::AtLock::hide(&mut {f});"));
        wipe.push_str(&format!("::faraday_kernel::AtLock::at_lock(&mut {f});"));
    }
    let code = format!(
        "{header} {{
            fn holds(&self) -> ::faraday_kernel::Holds {{ {holds} h }}
            fn hide(&mut self) {{ {hide} }}
            fn at_lock(&mut self) {{ {wipe} }}
        }}"
    );
    code.parse().unwrap_or_default()
}
