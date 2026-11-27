//! The application kernel's derive crate: `Message`, `Public` and `AtLock` (03 §1, §2.3, §3).
//!
//! The derives read the item's name, generics and where clause from the token stream (the
//! workspace takes no outside dependency, so there is no `syn`) and write impls whose methods are
//! `todo!()`. `Public` writes `Clone` and `Debug` as well, since `Public` needs both and the
//! sketches derive it beside `Copy` alone (03 §3.3).

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

use proc_macro::{Delimiter, TokenStream, TokenTree};

/// What a derive needs of the item it is on: its name, its generics as written (bounds kept,
/// defaults dropped), the generics' arguments and its where clause.
struct Item {
    name: String,
    decl: String,
    args: String,
    wher: String,
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

fn parse(input: TokenStream) -> Item {
    let mut it = input.into_iter().peekable();
    let mut name = String::new();
    while let Some(t) = it.next() {
        if let TokenTree::Ident(i) = &t {
            let s = i.to_string();
            if s == "struct" || s == "enum" {
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
    for t in it {
        match &t {
            TokenTree::Ident(i) if i.to_string() == "where" => {
                in_where = true;
                continue;
            }
            TokenTree::Group(g) if in_where && g.delimiter() == Delimiter::Brace => break,
            TokenTree::Punct(p) if in_where && p.as_char() == ';' => break,
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
    }
}

/// `#[derive(Message)]`: every variant is public unless marked `#[withheld]` (03 §3.1).
#[proc_macro_derive(Message, attributes(withheld))]
pub fn message(input: TokenStream) -> TokenStream {
    let item = parse(input);
    let header = item.header("::faraday_kernel::Message", "");
    let code = format!(
        "{header} {{
            fn kind(&self) -> &'static str {{ todo!() }}
            fn public(&self) -> ::core::option::Option<&dyn ::core::fmt::Debug> {{ todo!() }}
        }}"
    );
    code.parse().unwrap_or_default()
}

/// `#[derive(Public)]`: `Public` and its sealing, with the `Clone` and `Debug` it needs (03 §3).
#[proc_macro_derive(Public)]
pub fn public(input: TokenStream) -> TokenStream {
    let item = parse(input);
    let sealed = item.header("::faraday_kernel::__private::Sealed", "");
    let public = item.header("::faraday_kernel::Public", "");
    let clone = item.header("::core::clone::Clone", "");
    let debug = item.header("::core::fmt::Debug", "");
    let code = format!(
        "{sealed} {{}}
        {public} {{}}
        {clone} {{ fn clone(&self) -> Self {{ todo!() }} }}
        {debug} {{
            fn fmt(&self, _f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {{ todo!() }}
        }}"
    );
    code.parse().unwrap_or_default()
}

/// `#[derive(AtLock)]`: `holds`, `hide` and `at_lock` from each field's `#[lock(..)]` mark
/// (03 §2.3).
#[proc_macro_derive(AtLock, attributes(lock))]
pub fn at_lock(input: TokenStream) -> TokenStream {
    let item = parse(input);
    let header = item.header("::faraday_kernel::AtLock", "");
    let code = format!(
        "{header} {{
            fn holds(&self) -> ::faraday_kernel::Holds {{ todo!() }}
            fn hide(&mut self) {{ todo!() }}
            fn at_lock(&mut self) {{ todo!() }}
        }}"
    );
    code.parse().unwrap_or_default()
}
