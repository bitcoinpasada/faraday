//! The kit's atoms level: new: #[derive(Pressed)] and #[hides] (04 §2, §11.1).
//!
//! The derive reads the item's name, generics and where clause from the token stream (the
//! workspace takes no outside dependency, so there is no `syn`) and writes an impl whose method
//! is a `match` over the variants marked `#[hides]`.
//!
//! The impl names `::faraday_organisms::Pressed`: a feature's crate depends on the organisms and
//! templates crates, which re-export the trait (K7), and on no atoms crate. Section 09 §3.2 does
//! not say which path the derive writes.

use proc_macro::{Delimiter, TokenStream, TokenTree};

/// What a derive needs of the item it is on: its name, its generics as written (bounds kept,
/// defaults dropped), the generics' arguments and its where clause.
struct Item {
    name: String,
    decl: String,
    args: String,
    wher: String,
    /// The names of the variants marked `#[hides]`, each with the pattern that matches it
    /// whatever it holds.
    hiding: Vec<String>,
    /// Every variant of an enum: its name and what it holds.
    variants: Vec<(String, Holds)>,
}

/// What a variant holds, as far as the kind of a press needs.
enum Holds {
    Nothing,
    /// One value in parentheses: it may itself be a press.
    One,
    /// Anything else.
    Other,
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

    /// The type itself, with its arguments.
    fn ty(&self) -> String {
        if self.args.is_empty() {
            self.name.clone()
        } else {
            format!("{}<{}>", self.name, self.args)
        }
    }

    fn generic(&self) -> bool {
        !self.args.is_empty()
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
    let mut hiding = Vec::new();
    let mut variants = Vec::new();
    for t in it {
        match &t {
            TokenTree::Ident(i) if i.to_string() == "where" => {
                in_where = true;
                continue;
            }
            TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
                if is_enum {
                    hiding = hiding_variants(g.stream());
                    variants = all_variants(g.stream());
                }
                break;
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
        hiding,
        variants,
    }
}

/// The patterns of the variants of an enum's body marked `#[hides]`: `Self::V`, `Self::V(..)`
/// or `Self::V { .. }`, by what the variant holds.
fn hiding_variants(body: TokenStream) -> Vec<String> {
    let mut out = Vec::new();
    let mut marked = false;
    let mut name: Option<String> = None;
    let mut pattern = String::new();
    let mut it = body.into_iter().peekable();
    while let Some(t) = it.next() {
        match &t {
            // An attribute: `#` and a bracket group. Only `hides` is read.
            TokenTree::Punct(p) if p.as_char() == '#' && name.is_none() => {
                if let Some(TokenTree::Group(g)) = it.peek() {
                    if g.delimiter() == Delimiter::Bracket
                        && g.stream().to_string().trim() == "hides"
                    {
                        marked = true;
                    }
                    it.next();
                }
            }
            TokenTree::Punct(p) if p.as_char() == ',' => {
                if let (true, Some(n)) = (marked, name.take()) {
                    out.push(format!("Self::{n}{pattern}"));
                }
                name = None;
                marked = false;
                pattern.clear();
            }
            TokenTree::Ident(i) if name.is_none() => name = Some(i.to_string()),
            TokenTree::Group(g) if name.is_some() && pattern.is_empty() => match g.delimiter() {
                Delimiter::Parenthesis => pattern = "(..)".to_string(),
                Delimiter::Brace => pattern = " { .. }".to_string(),
                _ => {}
            },
            _ => {}
        }
    }
    if let (true, Some(n)) = (marked, name) {
        out.push(format!("Self::{n}{pattern}"));
    }
    out
}

/// Every variant of an enum's body, with what it holds.
fn all_variants(body: TokenStream) -> Vec<(String, Holds)> {
    let mut out = Vec::new();
    let mut name: Option<String> = None;
    let mut holds = Holds::Nothing;
    let mut it = body.into_iter().peekable();
    while let Some(t) = it.next() {
        match &t {
            TokenTree::Punct(p) if p.as_char() == '#' && name.is_none() => {
                if matches!(it.peek(), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Bracket)
                {
                    it.next();
                }
            }
            TokenTree::Punct(p) if p.as_char() == ',' => {
                if let Some(n) = name.take() {
                    out.push((n, core::mem::replace(&mut holds, Holds::Nothing)));
                }
            }
            TokenTree::Ident(i) if name.is_none() => name = Some(i.to_string()),
            TokenTree::Group(g) if name.is_some() => {
                holds = match g.delimiter() {
                    Delimiter::Parenthesis if one_field(g.stream()) => Holds::One,
                    Delimiter::Parenthesis | Delimiter::Brace => Holds::Other,
                    _ => Holds::Nothing,
                };
            }
            _ => {}
        }
    }
    if let Some(n) = name {
        out.push((n, holds));
    }
    out
}

/// Whether a parenthesised field list holds exactly one field: no comma outside angle brackets
/// but a trailing one.
fn one_field(fields: TokenStream) -> bool {
    let mut depth = 0i32;
    let mut after_dash = false;
    let mut seen = false;
    let mut commas = 0;
    let mut tail = false;
    for t in fields {
        tail = false;
        if let TokenTree::Punct(p) = &t {
            match p.as_char() {
                '<' => depth += 1,
                '>' if !after_dash => depth -= 1,
                ',' if depth == 0 => {
                    commas += 1;
                    tail = true;
                }
                _ => {}
            }
            after_dash = p.as_char() == '-';
        } else {
            after_dash = false;
        }
        seen = true;
    }
    seen && (commas == 0 || (commas == 1 && tail))
}

/// `#[derive(Pressed)]`: `faraday_atoms::Pressed` for a press type. A variant marked `#[hides]`
/// is one that hides a secret (04 §11.1).
#[proc_macro_derive(Pressed, attributes(hides))]
pub fn pressed(input: TokenStream) -> TokenStream {
    let item = parse(input);
    let extra = if item.generic() {
        format!("{}: Clone + PartialEq + Send + 'static", item.ty())
    } else {
        String::new()
    };
    let header = item.header("::faraday_organisms::Pressed", &extra);
    let body = if item.hiding.is_empty() {
        "false".to_string()
    } else {
        format!("matches!(self, {})", item.hiding.join(" | "))
    };
    let kind = if item.variants.is_empty() {
        String::new()
    } else {
        let arms: Vec<String> = item
            .variants
            .iter()
            .map(|(name, holds)| {
                let own = format!("concat!(env!(\"CARGO_PKG_NAME\"), \".{name}\")");
                match holds {
                    Holds::Nothing => format!("Self::{name} => {own}"),
                    Holds::Other => format!("Self::{name} {{ .. }} => {own}"),
                    Holds::One => format!(
                        "Self::{name}(inner) => {{ use ::faraday_organisms::kind_probe::{{Deeper as _, Leaf as _}}; (&::faraday_organisms::kind_probe::Probe(inner)).kind_of({own}) }}"
                    ),
                }
            })
            .collect();
        format!(
            "fn kind(&self) -> &'static str {{ match self {{ {} }} }}",
            arms.join(", ")
        )
    };
    let code = format!("{header} {{ fn hides(&self) -> bool {{ {body} }} {kind} }}");
    code.parse().unwrap_or_default()
}
