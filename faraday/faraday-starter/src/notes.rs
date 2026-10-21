//! What to write down before the restart (`docs/STARTER.md` §3.4, F47):
//! this computer's boot menu key, BIOS key and where the Secure Boot
//! setting is, from the maker table (`faraday_learn::MAKERS`); on an
//! Intel Mac, the Mac's equivalents. The same as a sheet to print: the
//! rows, the steps in order and the line for a setting that cannot be
//! found, and on the sheet a line each to circle the boot menu key and
//! the BIOS key that worked. On Windows with BitLocker on, and only when
//! the person ticks it, the sheet also carries the system drive's
//! recovery key (F49): the one secret here, wiped once the sheet is made.

use faraday_core::pdf;
use faraday_core::strings::fill;
use faraday_core::strings::{EN, Strings};

use crate::api::{Computer, Os, RecoveryKey};
use crate::macs;

const S: &Strings = &EN;

/// The sheet's file name, in the system's temporary folder.
pub const FILE_NAME: &str = "faraday-start-from-the-stick.pdf";

/// One thing to write down: its label, and its value, `None` when this
/// computer's maker has no page (a line to write on).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// What it is.
    pub label: String,
    /// Its value, or `None` when not known.
    pub value: Option<&'static str>,
}

/// One step of the sheet: what to do, and its lines of how.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// What to do.
    pub head: &'static str,
    /// How, a line each.
    pub lines: Vec<&'static str>,
}

/// A line of the sheet to circle the key that worked (F47): its label,
/// and the keys, this computer's maker's first, then every other key the
/// maker table names, then a line to write on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Circle {
    /// What it is.
    pub label: &'static str,
    /// The keys, in order.
    pub keys: Vec<&'static str>,
}

impl Circle {
    /// The line's value as the sheet prints it: the keys apart, then
    /// Other with a line to write on.
    pub fn text(&self) -> String {
        let mut parts: Vec<&str> = self.keys.clone();
        parts.push(S.starter_sheet_other_key);
        parts.join(" · ")
    }
}

/// The single keys a maker table phrase names, in its order: "Esc, then
/// F10" names Esc and F10, "F2 (or Fn+F2)" F2 and Fn+F2, "F2, held" F2.
pub fn keys_of(phrase: &'static str) -> Vec<&'static str> {
    phrase
        .split([',', '(', ')'])
        .map(str::trim)
        .map(|k| k.strip_prefix("then ").unwrap_or(k))
        .map(|k| k.strip_prefix("or ").unwrap_or(k))
        .filter(|k| !k.is_empty() && *k != "held")
        .collect()
}

/// Where a key goes in the fixed order: Esc, the function keys by
/// number, Fn with a function key, then the rest as the table has them.
fn rank(key: &str) -> u32 {
    let f = |k: &str| k.strip_prefix('F').and_then(|n| n.parse::<u32>().ok());
    if key == S.starter_sheet_key_esc {
        0
    } else if let Some(n) = f(key) {
        n
    } else if let Some(n) = key.strip_prefix("Fn+").and_then(f) {
        50 + n
    } else {
        100
    }
}

/// The keys to circle: `own`'s first (this maker's phrase, when its maker
/// has a page), then every key of `all` (the table's phrases) not yet
/// named, in the fixed order.
pub fn circle_keys(own: Option<&'static str>, all: &[&'static str]) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = own.map(keys_of).unwrap_or_default();
    let mut rest: Vec<&'static str> = Vec::new();
    for phrase in all {
        for k in keys_of(phrase) {
            if !out.contains(&k) && !rest.contains(&k) {
                rest.push(k);
            }
        }
    }
    rest.sort_by_key(|k| rank(k));
    out.extend(rest);
    out
}

/// The three things and the steps, for one computer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notes {
    /// The Start page's title.
    pub title: &'static str,
    /// The things to write down.
    pub rows: Vec<Row>,
    /// The sheet's lines to circle the key that worked: the boot menu
    /// key's and the BIOS key's; none on a Mac, whose keys are fixed.
    pub circles: Vec<Circle>,
    /// The steps, in order.
    pub steps: Vec<Step>,
}

/// The line for a Secure Boot setting that cannot be found, for the
/// system this computer runs.
pub fn not_found(os: Os) -> &'static str {
    if os == Os::Windows {
        S.starter_not_found_windows
    } else {
        S.starter_not_found
    }
}

impl Notes {
    /// The notes for this computer, or `None` for one that cannot start
    /// the stick (an ARM computer, a Mac with 32-bit firmware or Apple
    /// silicon).
    pub fn for_computer(c: &Computer) -> Option<Notes> {
        if let Some(mac) = macs::start(c) {
            return match mac {
                macs::Start::Intel { t2 } => Some(Notes::mac(t2)),
                macs::Start::Firmware32 | macs::Start::NotYet => None,
            };
        }
        if c.arm {
            return None;
        }
        let maker = faraday_learn::maker(&c.maker, &c.family, &c.model);
        let row = |label: &str, value: Option<&'static str>| Row {
            label: label.to_string(),
            value,
        };
        let windows = c.os == Os::Windows;
        let boot: Vec<&'static str> = faraday_learn::MAKERS.iter().map(|m| m.boot_key).collect();
        let bios: Vec<&'static str> = faraday_learn::MAKERS.iter().map(|m| m.bios_key).collect();
        Some(Notes {
            title: S.starter_item_start,
            rows: vec![
                row(S.starter_notes_boot_key, maker.map(|m| m.boot_key)),
                row(S.starter_notes_bios_key, maker.map(|m| m.bios_key)),
                row(S.starter_notes_setting, maker.map(|m| m.secure_boot)),
            ],
            circles: vec![
                Circle {
                    label: S.starter_sheet_boot_key_used,
                    keys: circle_keys(maker.map(|m| m.boot_key), &boot),
                },
                Circle {
                    label: S.starter_sheet_bios_key_used,
                    keys: circle_keys(maker.map(|m| m.bios_key), &bios),
                },
            ],
            steps: vec![
                Step {
                    head: S.starter_secure_boot_turn_off,
                    lines: vec![S.starter_sheet_off_how, not_found(c.os)],
                },
                Step {
                    head: S.starter_boot_from_stick,
                    lines: vec![if windows {
                        S.starter_sheet_boot_how_windows
                    } else {
                        S.starter_sheet_boot_how
                    }],
                },
                Step {
                    head: if windows {
                        S.starter_afterwards_windows
                    } else {
                        S.starter_afterwards
                    },
                    lines: vec![S.starter_sheet_back_how],
                },
            ],
        })
    }

    /// An Intel Mac's (§3.4's Mac steps): Option at power-on; on a T2
    /// Mac, macOS Recovery and its Startup Security Utility too, marked
    /// as for a T2 Mac only when the table does not know the Mac.
    fn mac(t2: Option<bool>) -> Notes {
        let t2_steps = t2 != Some(false);
        let marked = |label: &str| {
            if t2.is_none() {
                fill(S.starter_notes_t2_only, &[label])
            } else {
                label.to_string()
            }
        };
        let only = |mut lines: Vec<&'static str>| {
            if t2.is_none() {
                lines.insert(0, S.starter_mac_t2_only);
            }
            lines
        };
        let mut rows = vec![Row {
            label: S.starter_notes_boot_key.to_string(),
            value: Some(S.starter_notes_mac_boot_value),
        }];
        let mut steps = Vec::new();
        if t2_steps {
            rows.push(Row {
                label: marked(S.starter_notes_mac_recovery),
                value: Some(S.starter_notes_mac_recovery_value),
            });
            rows.push(Row {
                label: marked(S.starter_notes_mac_setting),
                value: Some(S.starter_notes_mac_setting_value),
            });
            steps.push(Step {
                head: S.starter_mac_lower,
                lines: only(vec![S.starter_mac_recovery, S.starter_mac_utility_lower]),
            });
        }
        steps.push(Step {
            head: S.starter_boot_from_stick,
            lines: vec![S.starter_mac_option, S.starter_mac_firmware_password],
        });
        if t2_steps {
            steps.push(Step {
                head: S.starter_mac_afterwards,
                lines: only(vec![S.starter_mac_utility_full]),
            });
        }
        Notes {
            title: S.starter_item_start_mac,
            rows,
            circles: Vec::new(),
            steps,
        }
    }

    /// Writes the sheet as a PDF (`faraday-core`'s writer): the title,
    /// the rows, a line to write on for a value not known, the lines to
    /// circle the keys that worked, and the steps. Writing a PDF.
    pub fn pdf(&self) -> Vec<u8> {
        self.sheet(None)
    }

    /// The same sheet with the system drive's BitLocker recovery key
    /// after the rows (F49): its drive and its eight groups, four to a
    /// line. Nothing else of BitLocker. The rows' copy of the key is
    /// wiped once the PDF is written; the PDF is the caller's to wipe.
    /// Writing a PDF.
    pub fn pdf_with_key(&self, key: &RecoveryKey) -> Vec<u8> {
        self.sheet(Some(key))
    }

    fn sheet(&self, key: Option<&RecoveryKey>) -> Vec<u8> {
        use zeroize::Zeroize;
        let mut rows: Vec<(String, Option<String>)> = self
            .rows
            .iter()
            .map(|r| (r.label.clone(), r.value.map(str::to_string)))
            .collect();
        for c in &self.circles {
            rows.push((c.label.to_string(), Some(c.text())));
        }
        if let Some(k) = key {
            // Four groups to a line: the value column holds about 44
            // characters, and a line breaks only at a space.
            let at = 4 * (crate::api::KEY_DIGITS + 1);
            let (a, b) = k.as_str().split_at(at);
            let mut value = String::with_capacity(k.as_str().len() + 1);
            value.push_str(a);
            value.push(' ');
            value.push_str(b);
            rows.push((
                fill(S.starter_sheet_recovery_key, &[k.drive()]),
                Some(value),
            ));
        }
        let steps: Vec<(String, Vec<String>)> = self
            .steps
            .iter()
            .map(|s| {
                (
                    s.head.to_string(),
                    s.lines.iter().map(|l| l.to_string()).collect(),
                )
            })
            .collect();
        let out = pdf::notes(self.title, &rows, &steps);
        for (label, value) in &mut rows {
            label.zeroize();
            value.zeroize();
        }
        out
    }
}
