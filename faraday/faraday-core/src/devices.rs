//! What else can capture what Faraday shows or what is typed
//! (`docs/STARTER.md` §3.1a), and the stick's check at start.
//!
//! The rules are pure functions over what the machine says of itself: a
//! USB device's identity as `/sys/bus/usb/devices` holds it (the same
//! attributes `faraday-grant` reads before authorising a device; nothing
//! new is bound), the CPU's flags, and the firmware's DMI strings. The
//! Starter can call the same functions on what its platform reads.
//!
//! On the stick, the shell reads the machine once at start
//! ([`DeviceCheck::machine`]) and the USB devices whenever they change
//! ([`Faraday::usb_devices`]). Each item found is named on a sheet with
//! what it can do. While one is attached and not let through, whatever
//! would put a secret on the screen or have one typed opens the sheet
//! instead ([`Faraday::device_holds`]). A virtual machine is never let
//! through.
//!
//! What may capture a secret but was not found is a reminder
//! ([`DeviceCheck::reminders`], `docs/DECISIONS.md` F24): the screen
//! may be a TV, unless its EDID settled it, and a wireless receiver is
//! attached. Nothing shows them at start or on Home. The first time a
//! secret would go in, the same sheet lists them with I understand, or
//! the sheet asking for a stick to be pulled lists them and pulling it
//! is the answer. They come back only when one is new.

use crate::strings::EN;

use crate::{Action, Faraday, Screen, Sheet};

/// One interface of a USB device: `bInterfaceClass`, `bInterfaceSubClass`
/// and `bInterfaceProtocol`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Interface {
    /// The class: 3 HID, 8 mass storage.
    pub class: u8,
    /// The subclass: 1 for a HID boot device.
    pub subclass: u8,
    /// The protocol: 1 for a HID boot keyboard, 2 for a mouse.
    pub protocol: u8,
}

/// A USB device as `/sys` describes it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UsbDevice {
    /// Where it is on the bus (`1-2`): what tells two alike apart.
    pub port: String,
    /// `idVendor`.
    pub vendor: u16,
    /// `idProduct`.
    pub product: u16,
    /// The `manufacturer` string, empty when it gives none.
    pub manufacturer: String,
    /// The `product` string, empty when it gives none.
    pub name: String,
    /// Its interfaces.
    pub interfaces: Vec<Interface>,
}

/// What an item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A KVM switch or an IP-KVM: it can see the screen and type.
    Kvm,
    /// A wireless keyboard or mouse receiver: keys travel over the air.
    /// A notice on Home; it holds nothing.
    Receiver,
    /// Logitech's Bolt receiver, whose link is encrypted: a line on Home
    /// that says so, in place of the receiver's.
    BoltReceiver,
    /// Faraday is running inside a virtual machine.
    VirtualMachine,
    /// The screen is a TV: its EDID says so.
    Tv,
}

impl Kind {
    /// A wireless receiver of either kind.
    pub fn receiver(self) -> bool {
        matches!(self, Kind::Receiver | Kind::BoltReceiver)
    }

    /// What it can do, in the words the sheet uses after its name.
    pub fn what(self) -> &'static str {
        match self {
            Kind::Kvm => EN.devices_kvm,
            Kind::Receiver => EN.devices_receiver_notice,
            Kind::BoltReceiver => EN.devices_bolt_notice,
            Kind::VirtualMachine => EN.devices_virtual_machine,
            Kind::Tv => EN.devices_tv,
        }
    }
}

/// One thing named on the sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Concern {
    /// What tells it apart from others: its port and identity, or the
    /// machine or the screen.
    pub key: String,
    /// Its name.
    pub name: String,
    /// What it is.
    pub kind: Kind,
}

impl Concern {
    /// The sheet's line: "Logitech USB Receiver: a wireless keyboard's
    /// keys travel over the air".
    pub fn line(&self) -> String {
        format!("{}: {}", self.name, self.kind.what())
    }
}

/// The Linux Foundation's USB vendor ID, which the kernel's USB gadget
/// framework uses by default: PiKVM, TinyPilot and other IP-KVMs built on
/// it present a keyboard and a disk under it (`usb.ids`: `1d6b Linux
/// Foundation`, `0104 Multifunction Composite Gadget`).
const LINUX_FOUNDATION: u16 = 0x1d6b;

const HID: u8 = 0x03;
const STORAGE: u8 = 0x08;

/// The common wireless keyboard and mouse receivers, by vendor and
/// product ID. Logitech's are the kernel's own names for them in
/// `drivers/hid/hid-ids.h` (`USB_DEVICE_ID_LOGITECH_*RECEIVER*`,
/// `USB_DEVICE_ID_S510_RECEIVER*`, `USB_DEVICE_ID_MX3000_RECEIVER`), as
/// is Rapoo's (`USB_DEVICE_ID_RAPOO_2_4G_RECEIVER`); Microsoft's and
/// Rapoo's others are `usb.ids` (2026.06.26): `045e 071f Mouse/Keyboard
/// 2.4GHz Transceiver V2.0`, `0745 Nano Transceiver v1.0`, `07a5 Wireless
/// Receiver 1461C`, `07b2 2.4GHz Transceiver v8.0`, `07fd Nano
/// Transceiver 1.1`, `00f9 Wireless Desktop Receiver 3.1`; `24ae 2001
/// 5 GHz Wireless Receiver`, `2003 5GHz Wireless Transceiver`.
pub const RECEIVERS: &[(u16, u16)] = &[
    // Logitech (USB_VENDOR_ID_LOGITECH 0x046d).
    (0x046d, 0xc101), // RECEIVER
    (0x046d, 0xc50c), // S510_RECEIVER
    (0x046d, 0xc513), // MX3000_RECEIVER
    (0x046d, 0xc517), // S510_RECEIVER_2
    (0x046d, 0xc51b), // 27MHZ_MOUSE_RECEIVER
    (0x046d, 0xc52b), // UNIFYING_RECEIVER
    (0x046d, 0xc52f), // NANO_RECEIVER
    (0x046d, 0xc531), // G700_RECEIVER
    (0x046d, 0xc532), // UNIFYING_RECEIVER_2
    (0x046d, 0xc534), // NANO_RECEIVER_2
    (0x046d, 0xc539), // NANO_RECEIVER_LIGHTSPEED_1
    (0x046d, 0xc53a), // NANO_RECEIVER_POWERPLAY
    (0x046d, 0xc53f), // NANO_RECEIVER_LIGHTSPEED_1_1
    (0x046d, 0xc543), // NANO_RECEIVER_LIGHTSPEED_1_2
    (0x046d, 0xc545), // NANO_RECEIVER_LIGHTSPEED_1_5
    (0x046d, 0xc547), // NANO_RECEIVER_LIGHTSPEED_1_3
    (0x046d, BOLT_RECEIVER.1),
    (0x046d, 0xc54d), // NANO_RECEIVER_LIGHTSPEED_1_4
    // Microsoft (0x045e), from usb.ids.
    (0x045e, 0x00f9),
    (0x045e, 0x071f),
    (0x045e, 0x0745),
    (0x045e, 0x07a5),
    (0x045e, 0x07b2),
    (0x045e, 0x07fd),
    // Rapoo (0x24ae).
    (0x24ae, 0x2001),
    (0x24ae, 0x2003),
    (0x24ae, 0x2015), // RAPOO_2_4G_RECEIVER
];

/// Logitech's Bolt receiver: `USB_VENDOR_ID_LOGITECH` and
/// `USB_DEVICE_ID_LOGITECH_BOLT_RECEIVER` in Linux's
/// `drivers/hid/hid-ids.h` (read 2026-10-12). Its link to a Bolt keyboard
/// is Bluetooth Low Energy in Secure Connections Only mode, encrypted
/// (Logitech's Logi Bolt white paper).
pub const BOLT_RECEIVER: (u16, u16) = (0x046d, 0xc548);

fn names_any(d: &UsbDevice, words: &[&str]) -> bool {
    let s = format!("{} {}", d.manufacturer, d.name).to_ascii_lowercase();
    words.iter().any(|w| s.contains(w))
}

/// What a USB device is, when it is one the stick warns about: a KVM
/// (its strings name one, or the Linux gadget vendor with a keyboard and
/// a disk), or a wireless receiver (a known receiver's ID, or a HID
/// device whose strings name a receiver, a dongle or wireless), Logitech's
/// Bolt receiver told apart by its ID.
pub fn usb_kind(d: &UsbDevice) -> Option<Kind> {
    let has = |class: u8| d.interfaces.iter().any(|i| i.class == class);
    let keyboard = d
        .interfaces
        .iter()
        .any(|i| i.class == HID && i.protocol == 1);
    if names_any(d, &["kvm", "tinypilot"])
        || (d.vendor == LINUX_FOUNDATION && keyboard && has(STORAGE))
    {
        return Some(Kind::Kvm);
    }
    if (d.vendor, d.product) == BOLT_RECEIVER {
        return Some(Kind::BoltReceiver);
    }
    if RECEIVERS.contains(&(d.vendor, d.product))
        || (has(HID) && names_any(d, &["receiver", "dongle", "wireless"]))
    {
        return Some(Kind::Receiver);
    }
    None
}

/// A USB device's name: its maker and product strings, the maker left
/// out when the product already starts with it; its IDs when it gives
/// neither.
pub fn usb_name(d: &UsbDevice) -> String {
    let (m, p) = (d.manufacturer.trim(), d.name.trim());
    match (m.is_empty(), p.is_empty()) {
        (true, true) => {
            EN.devices_unnamed
                .replacen("{}", &format!("{:04x}:{:04x}", d.vendor, d.product), 1)
        }
        (true, false) => p.to_string(),
        (false, true) => m.to_string(),
        (false, false) if p.to_ascii_lowercase().starts_with(&m.to_ascii_lowercase()) => {
            p.to_string()
        }
        (false, false) => {
            // "Logitech, Inc." reads as "Logitech".
            let m = m.split(',').next().unwrap_or(m).trim();
            format!("{m} {p}")
        }
    }
}

/// Whether Faraday runs inside a virtual machine, and its name: from the
/// CPU's `hypervisor` flag (`/proc/cpuinfo`) and the firmware's maker
/// and product (`/sys/class/dmi/id/sys_vendor`, `product_name`). A
/// hypervisor whose maker is not one of these is still one, named
/// [`EN.devices_virtual_machine_unknown`].
pub fn virtual_machine(cpuinfo: &str, sys_vendor: &str, product_name: &str) -> Option<String> {
    let (v, p) = (sys_vendor.trim(), product_name.trim());
    let named = if v.contains("QEMU") || p.contains("QEMU") {
        Some(EN.vm_qemu)
    } else if v.contains("VMware") || p.contains("VMware") {
        Some(EN.vm_vmware)
    } else if v.contains("innotek") || p.contains("VirtualBox") {
        Some(EN.vm_virtualbox)
    } else if v.starts_with("Microsoft") && p == "Virtual Machine" {
        Some(EN.vm_hyper_v)
    } else if v.contains("Xen") || p.contains("HVM domU") {
        Some(EN.vm_xen)
    } else if v.contains("Parallels") || p.contains("Parallels") {
        Some(EN.vm_parallels)
    } else {
        None
    };
    let flag = cpuinfo.lines().any(|l| {
        l.split_once(':').is_some_and(|(k, rest)| {
            k.trim() == "flags" && rest.split_whitespace().any(|f| f == "hypervisor")
        })
    });
    match (named, flag) {
        (Some(n), _) => Some(n.to_string()),
        (None, true) => Some(EN.devices_virtual_machine_unknown.to_string()),
        (None, false) => None,
    }
}

/// Whether the computer's screen is a separate one, from its DMI chassis
/// type (`/sys/class/dmi/id/chassis_type`, the SMBIOS System Enclosure
/// type in decimal): a desktop (3), low-profile desktop (4), pizza box
/// (5), mini tower (6), tower (7), space-saving (15), lunch box (16),
/// sealed-case PC (24), mini PC (35) or stick PC (36). Laptops,
/// notebooks, all-in-ones, tablets, convertibles and anything unknown
/// have a panel of their own, or are not asked.
pub fn screen_external(chassis_type: &str) -> bool {
    matches!(
        chassis_type.trim().parse::<u8>(),
        Ok(3..=7 | 15 | 16 | 24 | 35 | 36)
    )
}

/// The device check's state.
#[derive(Debug, Clone, Default)]
pub struct DeviceCheck {
    /// The shell read the machine at start: the stick on a computer.
    /// Nothing here applies anywhere else.
    pub read: bool,
    /// The virtual machine Faraday runs in, if it does.
    pub virtual_machine: Option<String>,
    /// The screen is a separate one, by the chassis type.
    pub external_screen: bool,
    /// The screen as its EDID names it, when it is a TV or a KVM's
    /// capture side.
    screen: Option<Concern>,
    /// The screen's EDID settled whether it may be a TV: no reminder.
    screen_settled: bool,
    /// The reminders the person has seen and gone on from, this session.
    understood: Vec<&'static str>,
    /// The boot import's sheet was open when the hold came up over it:
    /// it comes back when the hold ends.
    pub(crate) over_import: bool,
    /// The USB devices of concern attached now.
    present: Vec<Concern>,
    /// What the sheet names: each item found and not yet checked again
    /// or let through.
    pub held: Vec<Concern>,
    /// What Continue anyway let through, by key.
    let_through: Vec<String>,
    /// Continue anyway was pressed once: the sheet asks.
    pub asking: bool,
    /// The sheet is due once no other sheet is up.
    due: bool,
    /// What was pressed while held, done once the hold ends.
    pub(crate) after: Option<Action>,
}

impl DeviceCheck {
    /// The machine as the shell read it once at start: `/proc/cpuinfo`,
    /// `sys_vendor`, `product_name` and `chassis_type`.
    pub fn machine(cpuinfo: &str, sys_vendor: &str, product_name: &str, chassis: &str) -> Self {
        DeviceCheck {
            read: true,
            virtual_machine: virtual_machine(cpuinfo, sys_vendor, product_name),
            external_screen: screen_external(chassis),
            due: true,
            ..DeviceCheck::default()
        }
    }

    /// The screen's EDID base block as the firmware passed it at start
    /// ([`crate::edid::from_boot_params`]), when it did. A TV or a KVM is
    /// held by its name. A screen whose maker makes no TVs, or a
    /// computer's own panel, needs no reminder that it may be a TV. A TV
    /// maker's separate screen whose base block names no TV may still be
    /// one (the base block has none of the CEA-861 extension the Starter
    /// reads), and so may any screen when the firmware passed no EDID.
    pub fn with_edid(mut self, edid: Option<&[u8]>) -> Self {
        let Some(e) = edid else {
            return self;
        };
        let kind = crate::edid::display(e).map(|d| match d {
            crate::edid::Display::Tv => Kind::Tv,
            crate::edid::Display::Kvm => Kind::Kvm,
        });
        if let Some(kind) = kind {
            self.screen = Some(Concern {
                key: "screen".into(),
                name: crate::edid::edid_name(e).unwrap_or_else(|| EN.devices_this_screen.into()),
                kind,
            });
        }
        self.screen_settled = kind.is_some() || !crate::edid::tv_maker(e) || !self.external_screen;
        self
    }

    /// Everything of concern now: the machine, the screen as its EDID
    /// named it, and the USB devices.
    fn now(&self) -> Vec<Concern> {
        let mut out = Vec::new();
        if let Some(name) = &self.virtual_machine {
            out.push(Concern {
                key: "machine".into(),
                name: name.clone(),
                kind: Kind::VirtualMachine,
            });
        }
        out.extend(self.screen.clone());
        // A receiver is a reminder and holds nothing.
        out.extend(self.present.iter().filter(|c| !c.kind.receiver()).cloned());
        out.retain(|c| !self.let_through.contains(&c.key));
        out
    }

    /// The reminders that apply now: the screen may be a TV, on the
    /// stick, unless its EDID settled it; a wireless receiver is
    /// attached.
    pub fn reminders(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.read && !self.screen_settled && self.screen.is_none() {
            out.push(EN.devices_tv_line);
        }
        out.extend(self.receiver_line());
        out
    }

    /// The reminders not yet gone on from this session.
    pub fn reminders_due(&self) -> Vec<&'static str> {
        let mut out = self.reminders();
        out.retain(|r| !self.understood.contains(r));
        out
    }

    /// The person went on from the reminders due.
    pub(crate) fn understand(&mut self) {
        let due = self.reminders_due();
        self.understood.extend(due);
    }

    /// Whatever shows or takes a secret waits: an item is held, or a
    /// reminder has not been seen.
    pub fn holding(&self) -> bool {
        !self.held.is_empty() || !self.reminders_due().is_empty()
    }

    /// The line for a wireless receiver attached now: the warning for
    /// any receiver but Logitech's Bolt, the Bolt's own line when it is
    /// the only kind attached; nothing without one.
    pub fn receiver_line(&self) -> Option<&'static str> {
        let kinds = || self.present.iter().map(|c| c.kind);
        if kinds().any(|k| k == Kind::Receiver) {
            Some(Kind::Receiver.what())
        } else if kinds().any(|k| k == Kind::BoltReceiver) {
            Some(Kind::BoltReceiver.what())
        } else {
            None
        }
    }

    /// A virtual machine is among the items: no Continue anyway.
    pub fn refuses(&self) -> bool {
        self.held.iter().any(|c| c.kind == Kind::VirtualMachine)
    }
}

impl Faraday {
    /// The USB devices attached now, as the shell read them: at start and
    /// whenever they change. A new item of concern brings the sheet up.
    pub fn usb_devices(&mut self, devices: &[UsbDevice]) {
        let d = &mut self.devices;
        d.present = devices
            .iter()
            .filter_map(|u| {
                usb_kind(u).map(|kind| Concern {
                    key: format!("{}#{:04x}:{:04x}", u.port, u.vendor, u.product),
                    name: usb_name(u),
                    kind,
                })
            })
            .collect();
        // Gone ones stay named until Check again; new ones join.
        for c in d.now() {
            if !d.held.iter().any(|h| h.key == c.key) {
                d.held.push(c);
                d.due = true;
                d.asking = false;
            }
        }
        self.device_sheet();
        self.dirty = true;
        self.commands.push_back(faraday_shell_api::Command::Draw);
    }

    /// The sheet comes up when an item is held, it is due and no other
    /// sheet is. Reminders alone wait for a secret.
    pub(crate) fn device_sheet(&mut self) {
        if !self.devices.read || self.sheet.is_some() {
            return;
        }
        let d = &mut self.devices;
        if !d.held.is_empty() && d.due {
            d.due = false;
            self.sheet = Some(Sheet::Devices);
        }
    }

    /// Whether `action` shows a secret, loads or unlocks one, or has one
    /// typed: what waits while an item is held. Loading a key, scanning
    /// and unlocking are what a stick attached holds too
    /// ([`Faraday::pull_what`]); showing a seed, a key or a vault's
    /// contents are added here.
    pub fn device_holds(&self, action: Action) -> bool {
        use crate::vault::VaultAction as V;
        self.devices.holding()
            && (self.pull_what(action).is_some()
                || matches!(
                    action,
                    Action::BKey(_)
                        | Action::BReveal
                        | Action::BFile
                        | Action::KShow
                        | Action::LShow
                        | Action::LOut
                        | Action::SScanOut
                        | Action::SecretUnprotected
                        | Action::Bip85
                        | Action::Vault(V::Unlock | V::Show(_) | V::FocusPassphrase)
                        | Action::Nav(Screen::Unlock | Screen::VaultContents | Screen::Codes)
                ))
    }

    /// `action` was held: the sheet comes up, and it is done once the
    /// hold ends.
    pub(crate) fn device_hold(&mut self, action: Action) {
        self.devices.after = Some(action);
        self.devices.asking = false;
        self.devices.over_import = self.sheet == Some(Sheet::Import);
        self.sheet = Some(Sheet::Devices);
    }

    /// The hold ended: what was pressed carries on once nothing holds
    /// it, or comes back to the sheet with the reminders due.
    fn device_released(&mut self) {
        self.sheet = std::mem::take(&mut self.devices.over_import).then_some(Sheet::Import);
        if let Some(a) = self.devices.after.take() {
            self.act(a);
        }
    }

    /// The reminders the sheet shows: only while a secret waits, never
    /// at start.
    pub fn device_reminders(&self) -> Vec<&'static str> {
        if self.devices.after.is_some() {
            self.devices.reminders_due()
        } else {
            Vec::new()
        }
    }

    /// I understand: the reminders are seen, and what waited carries on.
    pub(crate) fn device_understood(&mut self) {
        if !self.devices.held.is_empty() {
            return;
        }
        self.devices.understand();
        self.device_released();
    }

    /// Check again: what was pulled goes from the sheet, and the sheet
    /// with it once nothing is left.
    pub(crate) fn device_check(&mut self) {
        let d = &mut self.devices;
        d.asking = false;
        d.held = d.now();
        if d.held.is_empty() {
            self.device_released();
        } else {
            self.toast(EN.devices_still);
        }
    }

    /// Continue anyway: the first press asks, the second lets every item
    /// through for this session. Never for a virtual machine.
    pub(crate) fn device_continue(&mut self) {
        let d = &mut self.devices;
        if d.refuses() {
            return;
        }
        if !d.asking {
            d.asking = true;
            return;
        }
        d.asking = false;
        let keys: Vec<String> = d.held.drain(..).map(|c| c.key).collect();
        d.let_through.extend(keys);
        // What the sheet listed with them is seen too.
        if d.after.is_some() {
            d.understand();
        }
        self.device_released();
    }
}

impl Faraday {
    /// Handles the device checks', network and input-device actions (`Devices*`, `Network`, `AirgapUnderstood`, `Input*`).
    pub(crate) fn devices_act(&mut self, action: Action) {
        match action {
            Action::InputUse(id) => self.input_believe(id),
            Action::InputIgnore(id) => self.input_ignore(id),
            Action::Network(n)
                if self.online
                    && !n.is_mainnet()
                    && !self.airgap_warned
                    && self.session.network().is_mainnet()
                    && self.session.wallets.is_empty()
                    && !self.session.keys.network_fixed() =>
            {
                self.test_asked = Some(n);
                self.sheet = Some(Sheet::NotAirgapped);
            }
            Action::AirgapUnderstood => {
                self.airgap_warned = true;
                self.sheet = None;
                if let Some(n) = self.test_asked.take() {
                    self.act(Action::Network(n));
                }
                // What waited under the start warning comes up now: a new
                // device first, then the boot import (F61).
                self.input_sheet();
                if self.sheet.is_none()
                    && self.import.is_some()
                    && std::mem::take(&mut self.import_under_input)
                {
                    self.sheet = Some(Sheet::Import);
                }
            }
            Action::Network(n) => match self.session.set_network(n) {
                Ok(()) => {
                    self.sheet = None;
                    self.refresh_spend();
                }
                Err(e) => self.toast(&e.text()),
            },
            Action::DevicesCheck => self.device_check(),
            Action::DevicesContinue => self.device_continue(),
            Action::DevicesUnderstood => self.device_understood(),
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }
}
