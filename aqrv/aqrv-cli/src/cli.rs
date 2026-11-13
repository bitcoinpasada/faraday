//! What the command line takes in and says back, which holds no secret: the
//! usage text, the arguments split into options and files, the exit code
//! and message of a failure, and the hexadecimal reader. The passphrase and
//! the item are never arguments, so nothing here sees them.

use std::io;

use faraday_text::{Text, text};

/// The envelope lengths `--size` accepts. A size the library adds is
/// named here; `Size::of_len` says whether it is one.
pub const SIZE_NUMBERS: [usize; 3] = [32, 53, 62];

pub const USAGE: &str = "\
usage:
  aqrv make    --kind words|codex32|passphrase|note|key [--size 32|53|62]
               [--memory 64|128|256|512|1024] [--out FILE.aqrv] [--png FILE.png]
               [--scale 8] [--passphrase-stdin]
  aqrv open    FILE|- | --text TEXT [--show] [--passphrase-stdin]
  aqrv inspect FILE|- [--json] [--png FILE.png]
  aqrv vectors [PATH/aqrv.json] [--case NAME] [--skip-large]
  aqrv from-sheet FILE.json --out FILE.aqrv";

/// An exit code and what is said with it.
pub struct Fail(pub u8, pub Text);

pub type Run = Result<(), Fail>;

/// The arguments after the program's name.
pub type Args = [String];

/// A failure with its code and message.
pub fn fail(code: u8, why: &str) -> Fail {
    Fail(code, Text::of(why))
}

pub fn usage(why: &str) -> Fail {
    Fail(64, text!("{why}\n{USAGE}"))
}

pub fn io_fail(what: &str, e: &io::Error) -> Fail {
    Fail(74, text!("{what}: {e}"))
}

/// The arguments after the program's name, each as text, or a usage error.
pub fn arguments() -> Result<Vec<String>, Fail> {
    std::env::args_os()
        .skip(1)
        .map(|a| {
            a.to_str()
                .map(str::to_owned)
                .ok_or_else(|| usage("an argument is not UTF-8"))
        })
        .collect()
}

/// Arguments split into what takes a value, what is a switch, and the
/// rest.
pub struct Parsed {
    pub values: Vec<(String, String)>,
    pub switches: Vec<String>,
    pub positional: Vec<String>,
}

impl Parsed {
    pub fn value(&self, name: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    pub fn has(&self, name: &str) -> bool {
        self.switches.iter().any(|s| s == name)
    }
}

pub fn parse(args: &Args, with_value: &[&str], switches: &[&str]) -> Result<Parsed, Fail> {
    let mut parsed = Parsed {
        values: Vec::new(),
        switches: Vec::new(),
        positional: Vec::new(),
    };
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        if with_value.contains(&arg.as_str()) {
            let Some(v) = it.next() else {
                return Err(usage(&format!("{arg} needs a value")));
            };
            if parsed.value(arg).is_some() {
                return Err(usage(&format!("{arg} is given twice")));
            }
            parsed.values.push((arg.clone(), v.clone()));
        } else if switches.contains(&arg.as_str()) {
            parsed.switches.push(arg.clone());
        } else if arg.starts_with("--") || (arg.starts_with('-') && arg != "-") {
            return Err(usage(&format!("{arg} is not an option here")));
        } else {
            parsed.positional.push(arg.clone());
        }
    }
    Ok(parsed)
}

pub fn number(p: &Parsed, name: &str) -> Result<Option<usize>, Fail> {
    p.value(name)
        .map(|v| {
            v.parse::<usize>()
                .map_err(|_| usage(&format!("{name} is a number")))
        })
        .transpose()
}

pub fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.is_ascii() || !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

pub fn numbers(all: &[usize]) -> String {
    all.iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}
