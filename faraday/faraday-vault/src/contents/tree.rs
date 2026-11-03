//! A TOML v1.0.0 document read into a tree of values (`docs/VAULT.md` §7).
//!
//! `toml_parser` lexes and parses the text into events and decodes each
//! scalar; this module is what it leaves out: the tree, and the rules
//! TOML puts across keys (a key given twice, a table defined twice, an
//! inline table or a static array reopened, dotted keys against headers).
//! The parser reads TOML 1.1; three things 1.1 adds are refused so that
//! what is read is 1.0.0: a line break or a trailing comma in an inline
//! table, and the `\x` and `\e` escapes of a basic string. (A time with no
//! seconds is refused where a date-time is read, in `contents`.)
//!
//! Every string value is kept in a buffer that is wiped when it drops.

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;

use toml_parser::decoder::{Encoding, ScalarKind};
use toml_parser::parser::{EventReceiver, RecursionGuard, parse_document};
use toml_parser::{ErrorSink, ParseError, Raw, Source, Span};
use zeroize::Zeroizing;

/// The deepest nesting of arrays and inline tables read.
const MAX_DEPTH: u32 = 16;

/// The most parts one dotted key or table header holds: a longer one
/// builds a table per part, which is nesting no vault needs.
const MAX_KEY_PARTS: usize = 8;

/// The most keys one table holds. No table of §7 has more than 13, and a
/// table is searched by key as it is built, so a bound keeps a document
/// of very many keys in one table from taking quadratic time: it is
/// refused instead.
const MAX_KEYS: usize = 64;

/// One TOML value.
pub(super) enum Val {
    /// A string.
    Str(Zeroizing<String>),
    /// An integer.
    Int(i64),
    /// A boolean.
    Bool(bool),
    /// A date-time, as written.
    Date(String),
    /// A float, which no key of the vault takes.
    Float,
    /// An array.
    Array(Arr),
    /// A table.
    Table(Table),
}

/// An array: written as one (`[…]`) or built by `[[…]]` headers.
pub(super) struct Arr {
    pub items: Vec<Val>,
    of_headers: bool,
}

/// How a table came to be, which decides what may be done to it later.
#[derive(Clone, Copy, PartialEq, Eq)]
enum How {
    /// Named only on the way to a deeper header.
    Implicit,
    /// Defined by its own header.
    Header,
    /// Made by a dotted key.
    Dotted,
    /// Written `{…}`: closed.
    Inline,
}

/// A table: its keys in the order written.
pub(super) struct Table {
    pub entries: Vec<(String, Val)>,
    how: How,
}

impl Table {
    fn new(how: How) -> Table {
        Table {
            entries: Vec::new(),
            how,
        }
    }

    fn full(&self) -> bool {
        self.entries.len() >= MAX_KEYS
    }

    fn at(&self, key: &str) -> Option<usize> {
        self.entries.iter().position(|(k, _)| k == key)
    }

    /// The value under `key`, taken out of the table.
    pub fn take(&mut self, key: &str) -> Option<Val> {
        let i = self.at(key)?;
        Some(self.entries.remove(i).1)
    }

    /// Adds `val` under the dotted `keys`; `Err` where a key is already
    /// there or a table on the way is not one a dotted key may extend.
    fn insert(&mut self, keys: &[String], val: Val) -> Result<(), ()> {
        let (last, via) = keys.split_last().ok_or(())?;
        let mut t = self;
        for k in via {
            let i = match t.at(k) {
                Some(i) => i,
                None => {
                    if t.full() {
                        return Err(());
                    }
                    t.entries
                        .push((k.clone(), Val::Table(Table::new(How::Dotted))));
                    t.entries.len() - 1
                }
            };
            match &mut t.entries[i].1 {
                Val::Table(next) if next.how == How::Dotted => t = next,
                _ => return Err(()),
            }
        }
        if t.at(last).is_some() || t.full() {
            return Err(());
        }
        t.entries.push((last.clone(), val));
        Ok(())
    }
}

/// The table `path` names under `root`, going into the last table of an
/// array of tables.
fn walk<'a>(root: &'a mut Table, path: &[String]) -> Option<&'a mut Table> {
    let mut t = root;
    for k in path {
        let i = t.at(k)?;
        t = match &mut t.entries[i].1 {
            Val::Table(next) => next,
            Val::Array(a) if a.of_headers => match a.items.last_mut()? {
                Val::Table(next) => next,
                _ => return None,
            },
            _ => return None,
        };
    }
    Some(t)
}

/// What is being built while the parser is inside it.
enum Open {
    Array(Vec<Val>),
    Inline {
        table: Table,
        keys: Vec<String>,
        done: bool,
        comma: bool,
    },
}

struct Builder<'i> {
    input: &'i str,
    root: Table,
    /// The table the current `[header]` opened.
    path: Vec<String>,
    /// The key (or header) being read at the top level.
    keys: Vec<String>,
    done: bool,
    open: Vec<Open>,
    /// The offset of the first thing refused.
    fail: Option<usize>,
}

impl Builder<'_> {
    fn refuse(&mut self, at: usize) {
        self.fail.get_or_insert(at);
    }

    fn raw(&self, span: Span, enc: Option<Encoding>) -> Raw<'_> {
        Raw::new_unchecked(&self.input[span.start()..span.end()], enc, span)
    }

    fn key_text(&mut self, span: Span, enc: Option<Encoding>) -> Option<String> {
        let mut out: Cow<str> = Cow::Borrowed("");
        let mut err: Option<ParseError> = None;
        if matches!(enc, Some(Encoding::BasicString))
            && has_1_1_escape(self.raw(span, enc).as_str())
        {
            self.refuse(span.start());
            return None;
        }
        self.raw(span, enc).decode_key(&mut out, &mut err);
        if err.is_some() {
            self.refuse(span.start());
            return None;
        }
        Some(out.into_owned())
    }

    fn scalar_val(&mut self, span: Span, enc: Option<Encoding>) -> Option<Val> {
        if matches!(enc, Some(Encoding::BasicString | Encoding::MlBasicString))
            && has_1_1_escape(self.raw(span, enc).as_str())
        {
            self.refuse(span.start());
            return None;
        }
        let mut out: Cow<str> = Cow::Borrowed("");
        let mut err: Option<ParseError> = None;
        let kind = self.raw(span, enc).decode_scalar(&mut out, &mut err);
        if err.is_some() {
            self.refuse(span.start());
            return None;
        }
        Some(match kind {
            ScalarKind::String => {
                // An owned string came from escapes and is wiped on drop;
                // a borrowed one is copied into a wiped buffer.
                Val::Str(Zeroizing::new(out.into_owned()))
            }
            ScalarKind::Boolean(b) => Val::Bool(b),
            ScalarKind::DateTime => Val::Date(out.into_owned()),
            ScalarKind::Float => Val::Float,
            ScalarKind::Integer(radix) => match int_of(&out, radix) {
                Some(n) => Val::Int(n),
                None => {
                    self.refuse(span.start());
                    return None;
                }
            },
        })
    }

    /// The key list a key being read goes in.
    fn keys_mut(&mut self) -> (&mut Vec<String>, &mut bool) {
        match self.open.last_mut() {
            Some(Open::Inline { keys, done, .. }) => (keys, done),
            _ => (&mut self.keys, &mut self.done),
        }
    }

    fn value(&mut self, val: Val, at: usize) {
        if self.fail.is_some() {
            return;
        }
        match self.open.last_mut() {
            Some(Open::Array(items)) => items.push(val),
            Some(Open::Inline {
                table,
                keys,
                done,
                comma,
            }) => {
                *comma = false;
                *done = true;
                if table.insert(keys, val).is_err() {
                    self.refuse(at);
                }
            }
            None => {
                self.done = true;
                let keys = core::mem::take(&mut self.keys);
                let ok =
                    walk(&mut self.root, &self.path).is_some_and(|t| t.insert(&keys, val).is_ok());
                self.keys = keys;
                if !ok {
                    self.refuse(at);
                }
            }
        }
    }

    /// Opens the table or the array element the header names.
    fn header(&mut self, array: bool, at: usize) {
        let keys = core::mem::take(&mut self.keys);
        let Some((last, via)) = keys.split_last() else {
            self.refuse(at);
            return;
        };
        let mut t = &mut self.root;
        for k in via {
            let i = match t.at(k) {
                Some(i) => i,
                None => {
                    if t.full() {
                        self.fail.get_or_insert(at);
                        return;
                    }
                    t.entries
                        .push((k.clone(), Val::Table(Table::new(How::Implicit))));
                    t.entries.len() - 1
                }
            };
            t = match &mut t.entries[i].1 {
                Val::Table(next) if next.how != How::Inline => next,
                Val::Array(a) if a.of_headers => match a.items.last_mut() {
                    Some(Val::Table(next)) => next,
                    _ => {
                        self.fail.get_or_insert(at);
                        return;
                    }
                },
                _ => {
                    self.fail.get_or_insert(at);
                    return;
                }
            };
        }
        if t.at(last).is_none() && t.full() {
            self.fail.get_or_insert(at);
            return;
        }
        let ok = match (t.at(last), array) {
            (None, false) => {
                t.entries
                    .push((last.clone(), Val::Table(Table::new(How::Header))));
                true
            }
            (None, true) => {
                t.entries.push((
                    last.clone(),
                    Val::Array(Arr {
                        items: alloc::vec![Val::Table(Table::new(How::Header))],
                        of_headers: true,
                    }),
                ));
                true
            }
            (Some(i), false) => match &mut t.entries[i].1 {
                Val::Table(x) if x.how == How::Implicit => {
                    x.how = How::Header;
                    true
                }
                _ => false,
            },
            (Some(i), true) => match &mut t.entries[i].1 {
                Val::Array(a) if a.of_headers => {
                    a.items.push(Val::Table(Table::new(How::Header)));
                    true
                }
                _ => false,
            },
        };
        if ok {
            self.path = keys;
        } else {
            self.refuse(at);
        }
    }
}

/// Whether a basic string's text holds `\x` or `\e`, which TOML 1.1 added.
fn has_1_1_escape(raw: &str) -> bool {
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c == '\\' && matches!(chars.next(), Some('x' | 'e')) {
            return true;
        }
    }
    false
}

/// The integer a decoded scalar spells.
fn int_of(text: &str, radix: toml_parser::decoder::IntegerRadix) -> Option<i64> {
    use toml_parser::decoder::IntegerRadix;
    let (neg, digits) = match text.as_bytes().first()? {
        b'-' => (true, &text[1..]),
        b'+' => (false, &text[1..]),
        _ => (false, text),
    };
    let (base, digits) = match radix {
        IntegerRadix::Dec => (10, digits),
        IntegerRadix::Hex => (16, digits.strip_prefix("0x").unwrap_or(digits)),
        IntegerRadix::Oct => (8, digits.strip_prefix("0o").unwrap_or(digits)),
        IntegerRadix::Bin => (2, digits.strip_prefix("0b").unwrap_or(digits)),
    };
    let clean: String = digits.chars().filter(|&c| c != '_').collect();
    let n = i128::from_str_radix(&clean, base).ok()?;
    i64::try_from(if neg { -n } else { n }).ok()
}

impl EventReceiver for Builder<'_> {
    fn std_table_open(&mut self, _: Span, _: &mut dyn ErrorSink) {
        self.keys.clear();
        self.done = false;
    }
    fn std_table_close(&mut self, span: Span, _: &mut dyn ErrorSink) {
        if self.fail.is_none() {
            self.header(false, span.start());
        }
    }
    fn array_table_open(&mut self, _: Span, _: &mut dyn ErrorSink) {
        self.keys.clear();
        self.done = false;
    }
    fn array_table_close(&mut self, span: Span, _: &mut dyn ErrorSink) {
        if self.fail.is_none() {
            self.header(true, span.start());
        }
    }
    fn inline_table_open(&mut self, _: Span, _: &mut dyn ErrorSink) -> bool {
        self.open.push(Open::Inline {
            table: Table::new(How::Inline),
            keys: Vec::new(),
            done: false,
            comma: false,
        });
        true
    }
    fn inline_table_close(&mut self, span: Span, _: &mut dyn ErrorSink) {
        if let Some(Open::Inline { table, comma, .. }) = self.open.pop() {
            if comma {
                self.refuse(span.start());
            }
            self.value(Val::Table(table), span.start());
        } else {
            self.refuse(span.start());
        }
    }
    fn array_open(&mut self, _: Span, _: &mut dyn ErrorSink) -> bool {
        self.open.push(Open::Array(Vec::new()));
        true
    }
    fn array_close(&mut self, span: Span, _: &mut dyn ErrorSink) {
        if let Some(Open::Array(items)) = self.open.pop() {
            self.value(
                Val::Array(Arr {
                    items,
                    of_headers: false,
                }),
                span.start(),
            );
        } else {
            self.refuse(span.start());
        }
    }
    fn simple_key(&mut self, span: Span, enc: Option<Encoding>, _: &mut dyn ErrorSink) {
        if self.fail.is_some() {
            return;
        }
        let Some(key) = self.key_text(span, enc) else {
            return;
        };
        let (keys, done) = self.keys_mut();
        if core::mem::replace(done, false) {
            keys.clear();
        }
        keys.push(key);
        if keys.len() > MAX_KEY_PARTS {
            self.refuse(span.start());
        }
    }
    fn scalar(&mut self, span: Span, enc: Option<Encoding>, _: &mut dyn ErrorSink) {
        if self.fail.is_some() {
            return;
        }
        if let Some(v) = self.scalar_val(span, enc) {
            self.value(v, span.start());
        }
    }
    fn value_sep(&mut self, _: Span, _: &mut dyn ErrorSink) {
        if let Some(Open::Inline { comma, .. }) = self.open.last_mut() {
            *comma = true;
        }
    }
    fn newline(&mut self, span: Span, _: &mut dyn ErrorSink) {
        if matches!(self.open.last(), Some(Open::Inline { .. })) {
            self.refuse(span.start());
        }
    }
    fn error(&mut self, span: Span, _: &mut dyn ErrorSink) {
        self.refuse(span.start());
    }
}

/// Reads `text` as TOML v1.0.0. `Err` is the line of the first thing
/// refused (counting from 1).
pub(super) fn parse(text: &str) -> Result<Table, usize> {
    let source = Source::new(text);
    let tokens = source.lex().into_vec();
    let mut builder = Builder {
        input: text,
        root: Table::new(How::Header),
        path: Vec::new(),
        keys: Vec::new(),
        done: false,
        open: Vec::new(),
        fail: None,
    };
    let mut err: Option<ParseError> = None;
    {
        let mut guard = RecursionGuard::new(&mut builder, MAX_DEPTH);
        parse_document(&tokens, &mut guard, &mut err);
    }
    let at = match (&err, builder.fail) {
        (Some(e), _) => Some(e.unexpected().or(e.context()).map_or(0, |s| s.start())),
        (None, Some(at)) => Some(at),
        (None, None) if !builder.open.is_empty() => Some(0),
        (None, None) => None,
    };
    match at {
        Some(at) => {
            let end = at.min(text.len());
            let end = (0..=end)
                .rev()
                .find(|&i| text.is_char_boundary(i))
                .unwrap_or(0);
            Err(1 + text[..end].bytes().filter(|&b| b == b'\n').count())
        }
        None => Ok(builder.root),
    }
}

#[cfg(test)]
mod toml_test;
