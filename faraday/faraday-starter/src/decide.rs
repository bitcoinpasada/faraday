//! What the Starter decides from what the shell reports, with no drawing:
//! the disks offered and refused and their order (§3.2, §4.2), keep
//! data's rules (§3.2 step 2), and Home's checklist and its next step
//! (§3.1), on a Mac too (§3.1's "On a Mac", `docs/MAC.md` §6). The screens draw these; the tests check them through the
//! screens.

use faraday_core::upgrade::{Fit, compare};
use faraday_strings::EN;
use faraday_strings::fill;

use crate::api::{Bus, Computer, Disk, FaradayStick, InUse, Os};
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
        Some("maker-lenovo-ideapad" | "maker-framework") => EN.starter_boot_menu_efi_usb_device,
        Some("maker-hp") => EN.starter_boot_menu_hp,
        _ => EN.starter_boot_menu_default,
    };
    Some(fill(template, &[label.as_str()]))
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

/// The Faraday version `stick` holds, as Home and Which stick say it:
/// "version not read" when FARADAYBOOT was not read (Windows),
/// kept apart from a stick made before Faraday put its version there.
pub fn version_of(stick: &FaradayStick) -> String {
    if stick.read {
        faraday_core::upgrade::version(stick.release.as_deref())
    } else {
        EN.starter_version_not_read.to_string()
    }
}

/// An item of Home's checklist (§3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    /// Make a Faraday stick.
    MakeStick,
    /// Start this computer from the stick; on an Intel Mac, Start this
    /// Mac from the stick.
    Start,
    /// Turn Secure Boot back on.
    SecureBootOn,
}

/// The state an item shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// No Faraday stick attached.
    NoStick,
    /// A Faraday stick attached, with the Faraday version it holds.
    Stick(String),
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
    /// Its action can be taken now.
    pub offered: bool,
}

/// What Home's checklist is read from, every time it is drawn.
#[derive(Debug, Clone, Copy)]
pub struct Facts<'a> {
    /// This computer.
    pub computer: &'a Computer,
    /// The disks attached.
    pub disks: &'a [Disk],
    /// Another encryption product's driver is here and the person has
    /// not yet said they have its rescue disk or password (§6.4).
    pub other_open: bool,
}

impl Facts<'_> {
    /// The step back to Secure Boot on applies: Secure Boot is off now
    /// (§3.4 step 4). The Starter keeps no record, so it applies
    /// whenever Secure Boot is off.
    pub fn secure_boot_back(&self) -> bool {
        self.computer.secure_boot == Some(false)
    }

    /// Windows' Use a device (Restart to choose a device, Restart from
    /// the stick) is offered only while Windows reports Secure Boot off
    /// at this moment (F33); hidden when it reads on or cannot be read.
    pub fn use_a_device(&self) -> bool {
        self.computer.os == Os::Windows
            && self.computer.uefi
            && self.computer.secure_boot == Some(false)
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
            .map(version_of)
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
        rows
    }

    /// The next step: the item Home marks and its card's action opens:
    /// the first of Make a Faraday stick and Start this computer from the
    /// stick that is not done. Start is never ticked, so it is the next
    /// step once a stick is attached. Turn Secure Boot back on is never
    /// marked.
    pub fn next_step(&self) -> Option<Item> {
        self.checklist()
            .iter()
            .find(|r| !r.done && matches!(r.item, Item::MakeStick | Item::Start))
            .map(|r| r.item)
    }
}
