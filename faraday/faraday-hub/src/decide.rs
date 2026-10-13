//! What the hub decides from what the shell reports, with no drawing:
//! the disks offered and refused and their order (§3.2, §4.2), keep
//! data's rules (§3.2 step 2), and Home's checklist and its next step
//! (§3.1), on a Mac too (§3.1's "On a Mac", `docs/MAC.md` §6). The screens draw these; the tests check them through the
//! screens.

use faraday_core::strings::EN;
use faraday_core::upgrade::{Fit, compare};
use opensigner_core::strings::fill1;

use crate::api::{BitLocker, Bus, Computer, Disk, FaradayStick, InUse, Os, Protection};
use crate::macs;

/// What is being made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// A Faraday stick (§3.2).
    Stick,
    /// A Pi card (§3.3).
    Pi,
}

/// Why a disk is listed but cannot be chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The computer uses it.
    InUse(InUse),
    /// It is smaller than the image: the bytes the image needs.
    TooSmall(u64),
}

/// A disk as the list shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer<'a> {
    /// The disk.
    pub disk: &'a Disk,
    /// Why it cannot be chosen, when it cannot.
    pub refused: Option<Refusal>,
}

/// The disks Which stick (or Which card) lists for `target`, in order:
/// Faraday sticks first, then the others, each by size, the smallest
/// first. Only USB disks are listed, and for a Pi card the computer's own
/// SD card reader too. A disk the computer uses, or one smaller than
/// `image` bytes, is listed with its refusal. Nothing is chosen here: a
/// person chooses by a tap.
pub fn offers(disks: &[Disk], target: Target, image: Option<u64>) -> Vec<Offer<'_>> {
    let mut list: Vec<Offer<'_>> = disks
        .iter()
        .filter(|d| match d.bus {
            Bus::Usb => true,
            Bus::SdReader => target == Target::Pi,
            Bus::Other => false,
        })
        .map(|disk| {
            let refused = match (&disk.in_use, image) {
                (Some(why), _) => Some(Refusal::InUse(why.clone())),
                (None, Some(needs)) if disk.size < needs => Some(Refusal::TooSmall(needs)),
                _ => None,
            };
            Offer { disk, refused }
        })
        .collect();
    list.sort_by(|a, b| {
        (a.disk.faraday.is_none(), a.disk.size, &a.disk.id).cmp(&(
            b.disk.faraday.is_none(),
            b.disk.size,
            &b.disk.id,
        ))
    });
    list
}

/// An INQUIRY or USB descriptor string, trimmed and its interior
/// whitespace collapsed to single spaces (those fields are
/// space-padded).
fn clean(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Whether `vendor` (already [`clean`]ed) is one a boot menu entry
/// drops rather than repeat: empty, or one of the generic strings a
/// stick without its own vendor reports, ignoring case. Matched as
/// `faraday_learn::Maker`'s own `vendors` and `families` are (not
/// shown).
fn generic_vendor(vendor: &str) -> bool {
    let vendors: &[&str] = &["USB", "GENERIC", "MASS STORAGE"];
    vendor.is_empty() || vendors.contains(&vendor.to_ascii_uppercase().as_str())
}

/// What the computer's firmware is expected to list a stick as in its
/// boot menu (owner's idea, local research, read 2026-10-13, already
/// applied in the guides): the stick's own vendor and model
/// (`Disk::vendor`, `Disk::model`; trimmed and collapsed, since those
/// are space-padded INQUIRY or USB descriptor fields), under the
/// pattern this computer's maker family (`faraday_learn::maker`, §7.6)
/// is known to use:
///
/// - Lenovo's IdeaPad family (IdeaPad, Yoga, Legion) and Framework:
///   `EFI USB Device (<vendor> <model>)`.
/// - HP: `USB Hard Drive - <vendor> <model>`.
/// - Dell, ASUS, MSI, Acer, and the default for any other maker
///   (ThinkPad and Surface are not verified): `UEFI: <vendor> <model>,
///   Partition 1`.
///
/// A vendor that is empty or generic (`USB`, `Generic`, `Mass
/// Storage`) is dropped when the model already carries it, so the
/// entry does not repeat it. `None` when the stick reports neither a
/// vendor nor a model: nothing to show.
pub fn boot_menu_entry(vendor: &str, model: &str, computer: &Computer) -> Option<String> {
    let vendor = clean(vendor);
    let model = clean(model);
    let drop_vendor = generic_vendor(&vendor)
        && (vendor.is_empty()
            || model
                .to_ascii_uppercase()
                .contains(&vendor.to_ascii_uppercase()));
    let label = match (drop_vendor, model.is_empty()) {
        (true, _) => model,
        (false, true) => vendor,
        (false, false) => format!("{vendor} {model}"),
    };
    if label.is_empty() {
        return None;
    }
    let maker = faraday_learn::maker(&computer.maker, &computer.family, &computer.model);
    let template = match maker.map(|m| m.id) {
        Some("maker-lenovo-ideapad" | "maker-framework") => EN.hub_boot_menu_efi_usb_device,
        Some("maker-hp") => EN.hub_boot_menu_hp,
        _ => EN.hub_boot_menu_default,
    };
    Some(fill1(template, &label))
}

/// Keep data's rule for writing build `build` over `stick` (F2, F5, D15):
/// [`Fit::Same`] is refused; [`Fit::Newer`] and [`Fit::Dev`] are warned
/// about on Confirm; [`Fit::Fits`] is written. An image with no build ID
/// it can be compared by is written. The comparison is the stick's own
/// (`faraday_core::upgrade::compare`).
pub fn keep_data(stick: &FaradayStick, build: Option<&str>) -> Fit {
    let on_stick = stick
        .release
        .as_deref()
        .and_then(faraday_core::upgrade::build_id);
    match build {
        Some(b) => compare(on_stick, b),
        None => Fit::Fits,
    }
}

/// An item of Home's checklist (§3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    /// Make a Faraday stick.
    MakeStick,
    /// BitLocker (Windows, a protected drive).
    BitLocker,
    /// Start this computer from the stick; on an Intel Mac, Start this
    /// Mac from the stick.
    Start,
    /// Turn Secure Boot back on.
    SecureBootOn,
    /// Resume BitLocker.
    Resume,
}

/// The state an item shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// No Faraday stick attached.
    NoStick,
    /// A Faraday stick attached, with the Faraday version it holds.
    Stick(String),
    /// The drive is protected.
    Protected,
    /// The hub app's suspension stands.
    Suspended,
    /// Suspended, not by the hub app: the guard is still needed, as for
    /// a protected drive (§6.2).
    SuspendedOther,
    /// Protection was resumed through the hub app this session.
    Resumed,
    /// The person said they have the recovery key, this session (F17).
    KeyKept,
    /// Another encryption product, not yet confirmed (§6.4).
    Other,
    /// Secure Boot on, off, or not read.
    SecureBoot(Option<bool>),
    /// An Intel Mac's way to the stick: with the T2 chip's Startup
    /// Security first, or not; `None` for a Mac the table does not know.
    MacStart(Option<bool>),
}

/// One row of the checklist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The item.
    pub item: Item,
    /// What it shows.
    pub state: State,
    /// It is done.
    pub done: bool,
    /// Its action can be taken now (Resume needs Secure Boot as the
    /// marker recorded it).
    pub offered: bool,
}

/// What Home's checklist is read from, every time it is drawn.
#[derive(Debug, Clone, Copy)]
pub struct Facts<'a> {
    /// This computer.
    pub computer: &'a Computer,
    /// BitLocker, on Windows.
    pub bitlocker: Option<&'a BitLocker>,
    /// The disks attached.
    pub disks: &'a [Disk],
    /// Protection was resumed through the hub app this session.
    pub resumed: bool,
    /// The hub app suspended BitLocker in this session: the person has
    /// not been to the firmware yet, so the next step is still Start.
    pub suspended_here: bool,
    /// The person said they have the recovery key, this session: the
    /// default BitLocker step (F17). Nothing on the computer changed.
    pub key_kept: bool,
    /// Another encryption product's driver is here and the person has
    /// not yet said they have its rescue disk or password (§6.4).
    pub other_open: bool,
}

impl Facts<'_> {
    /// The hub app's suspension stands: its marker exists and protection
    /// is suspended (§6.3). Protection on again with the marker still
    /// there (a policy that undid it) does not count.
    pub fn suspension_stands(&self) -> bool {
        self.bitlocker
            .is_some_and(|b| b.marker.is_some() && b.protection == Protection::Suspended)
    }

    /// A drive is protected and the guard is needed (§6.2): on, or not
    /// readable. On Windows, no BitLocker information at all is "not
    /// readable": only a drive read as not encrypted lets the firmware
    /// steps through without the guard. A drive suspended by someone
    /// else counts as protected too: Control Panel's suspension ends at
    /// the next restart, which is the restart into the firmware, so only
    /// the hub app's own suspension (no restart count, its marker) is
    /// known to last.
    pub fn protected(&self) -> bool {
        self.computer.os == Os::Windows
            && self.bitlocker.is_none_or(|b| match b.protection {
                Protection::On | Protection::Unknown => true,
                Protection::Suspended => b.marker.is_none(),
                Protection::None => false,
            })
    }

    /// The BitLocker step is done: the recovery key is in hand (F17),
    /// or the extra-safe suspension stands, or it was resumed here with
    /// Secure Boot on.
    pub fn guard_done(&self) -> bool {
        self.key_kept || self.suspension_stands() || self.resumed
    }

    /// What the BitLocker item shows, on Home and on the Start page.
    pub fn bitlocker_state(&self) -> State {
        if self.other_open {
            State::Other
        } else if self.suspension_stands() {
            State::Suspended
        } else if self.resumed {
            State::Resumed
        } else if self.key_kept {
            State::KeyKept
        } else if self
            .bitlocker
            .is_some_and(|b| b.protection == Protection::Suspended)
        {
            State::SuspendedOther
        } else {
            State::Protected
        }
    }

    /// The step back to Secure Boot on applies: Secure Boot is off now,
    /// and the marker does not say it was already off when the hub app
    /// suspended BitLocker (§3.4 step 5). With no marker the hub app
    /// keeps no record, so it applies whenever Secure Boot is off.
    pub fn secure_boot_back(&self) -> bool {
        let was_off = self
            .bitlocker
            .and_then(|b| b.marker.as_ref())
            .is_some_and(|m| !m.secure_boot_was_on);
        self.computer.secure_boot == Some(false) && !was_off
    }

    /// Resume BitLocker can be offered: the suspension stands, Windows
    /// has started again since it, and Secure Boot is as the marker
    /// recorded it when the hub app suspended (§6.3).
    pub fn resume_offered(&self) -> bool {
        self.suspension_stands()
            && self
                .bitlocker
                .and_then(|b| b.marker.as_ref())
                .is_some_and(|m| {
                    !m.this_boot && self.computer.secure_boot == Some(m.secure_boot_was_on)
                })
    }

    /// This computer as a Mac (§3.1's "On a Mac"), or `None` when it is
    /// not one.
    pub fn mac(&self) -> Option<macs::Start> {
        macs::start(self.computer)
    }

    /// This computer can start the x86_64 stick at all: not an ARM
    /// computer, and not a Mac with 32-bit firmware or Apple silicon.
    pub fn can_start(&self) -> bool {
        !self.computer.arm
            && !matches!(
                self.mac(),
                Some(macs::Start::Firmware32 | macs::Start::NotYet)
            )
    }

    /// The first Faraday stick attached, when one is.
    pub fn stick(&self) -> Option<&Disk> {
        self.disks
            .iter()
            .find(|d| d.bus == Bus::Usb && d.faraday.is_some())
    }

    /// The Faraday version of the first Faraday stick attached, when one
    /// is.
    pub fn stick_version(&self) -> Option<String> {
        self.stick()
            .and_then(|d| d.faraday.as_ref())
            .map(|f| faraday_core::upgrade::version(f.release.as_deref()))
    }

    /// The checklist, in order, without the items that do not apply.
    pub fn checklist(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        let stick = self.stick_version();
        rows.push(Row {
            item: Item::MakeStick,
            done: stick.is_some(),
            state: stick.map_or(State::NoStick, State::Stick),
            offered: true,
        });
        if !self.can_start() {
            return rows;
        }
        let windows = self.computer.os == Os::Windows;
        if windows && (self.protected() || self.guard_done() || self.other_open) {
            rows.push(Row {
                item: Item::BitLocker,
                state: self.bitlocker_state(),
                done: self.guard_done() && !self.other_open,
                offered: true,
            });
        }
        let mac = self.mac();
        rows.push(Row {
            item: Item::Start,
            state: match mac {
                Some(macs::Start::Intel { t2 }) => State::MacStart(t2),
                _ => State::SecureBoot(self.computer.secure_boot),
            },
            done: false,
            offered: true,
        });
        // A Mac's steps put its Startup Security back themselves; its
        // firmware has no Secure Boot setting to turn back on.
        if mac.is_none() && self.secure_boot_back() {
            rows.push(Row {
                item: Item::SecureBootOn,
                state: State::SecureBoot(Some(false)),
                done: false,
                offered: true,
            });
        }
        if windows && self.suspension_stands() {
            rows.push(Row {
                item: Item::Resume,
                state: State::Suspended,
                done: false,
                offered: self.resume_offered(),
            });
        }
        rows
    }

    /// The next step: the item Home marks and its card's action opens.
    ///
    /// While the hub app's suspension stands, the person has made the
    /// stick and started from it already, so the next step is turning
    /// Secure Boot back on, and after that resuming BitLocker; but not
    /// in the session that suspended it, where the firmware is still
    /// ahead. Otherwise it is the first of Make a Faraday stick,
    /// BitLocker and Start this computer from the stick that is not
    /// done; Start is never ticked, so it is the next step once the
    /// others are done.
    pub fn next_step(&self) -> Option<Item> {
        let rows = self.checklist();
        let has = |i: Item| rows.iter().any(|r| r.item == i);
        if self.suspension_stands() && !self.suspended_here {
            if has(Item::SecureBootOn) {
                return Some(Item::SecureBootOn);
            }
            if has(Item::Resume) {
                return Some(Item::Resume);
            }
        }
        rows.iter()
            .find(|r| !r.done && matches!(r.item, Item::MakeStick | Item::BitLocker | Item::Start))
            .map(|r| r.item)
    }
}
