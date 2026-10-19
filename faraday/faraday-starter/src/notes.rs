//! What to write down before the restart (`docs/STARTER.md` §3.4, F47):
//! this computer's boot menu key, BIOS key and where the Secure Boot
//! setting is, from the maker table (`faraday_learn::MAKERS`); on an
//! Intel Mac, the Mac's equivalents. The same as a sheet to print: the
//! rows, the steps in order and the line for a setting that cannot be
//! found. Nothing here is secret.

use faraday_core::pdf;
use faraday_core::strings::{EN, Strings};
use opensigner_core::strings::fill;

use crate::api::{Computer, Os};
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

/// The three things and the steps, for one computer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notes {
    /// The Start page's title.
    pub title: &'static str,
    /// The things to write down.
    pub rows: Vec<Row>,
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
        Some(Notes {
            title: S.starter_item_start,
            rows: vec![
                row(S.starter_notes_boot_key, maker.map(|m| m.boot_key)),
                row(S.starter_notes_bios_key, maker.map(|m| m.bios_key)),
                row(S.starter_notes_setting, maker.map(|m| m.secure_boot)),
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
            steps,
        }
    }

    /// Writes the sheet as a PDF (`faraday-core`'s writer): the title,
    /// the rows, a line to write on for a value not known, and the steps.
    pub fn pdf(&self) -> Vec<u8> {
        let rows: Vec<(String, Option<String>)> = self
            .rows
            .iter()
            .map(|r| (r.label.clone(), r.value.map(str::to_string)))
            .collect();
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
        pdf::notes(self.title, &rows, &steps)
    }
}
