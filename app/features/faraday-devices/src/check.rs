//! The device check's state: what is held, what was let through, what the person has
//! understood (`main`'s `DeviceCheck`, `devices.rs` 284-474, and the facts' handling, 501-591).

use faraday_kernel::Public;
use faraday_port::devices::{Edid, UsbDevice};
use faraday_port::event::Machine;
use faraday_strings::EN;
use faraday_text::Text;

use crate::rules::{Concern, Kind, Reminder, screen_external, usb_kind, usb_name, virtual_machine};

/// The key of the screen's item, and of the machine's.
const SCREEN: &str = "screen";
const MACHINE: &str = "machine";

/// Holds nothing a lock wipes: the devices plugged in.
#[derive(Public, Default)]
pub struct DeviceCheck {
    /// The machine was read at start: the stick on a computer. Nothing here applies anywhere
    /// else.
    read: bool,
    /// The virtual machine Faraday runs in, if it does.
    virtual_machine: Option<Text>,
    /// The screen is a separate one, by the chassis type.
    external_screen: bool,
    /// The screen as its EDID names it, when it is a TV or a KVM's capture side.
    screen: Option<Concern>,
    /// The screen's EDID settled whether it may be a TV: no reminder.
    screen_settled: bool,
    /// The reminders the person has seen and gone on from, this session.
    understood: Vec<Reminder>,
    /// The USB devices of concern attached now.
    present: Vec<Concern>,
    /// What the sheet names: each item found and not yet checked again or let through.
    held: Vec<Concern>,
    /// What Continue anyway let through, by key.
    let_through: Vec<Text>,
    /// Continue anyway was pressed once: the sheet asks.
    asking: bool,
    /// The sheet is due once no other sheet is up.
    due: bool,
}

impl DeviceCheck {
    /// The machine as the adapter read it at start. `read` is whether this product checks at
    /// all (it holds back input: the device on a computer or a Pi with a screen).
    #[must_use]
    pub fn machine(read: bool, m: &Machine) -> DeviceCheck {
        if !read {
            return DeviceCheck::default();
        }
        // A Pi's HDMI screen has no chassis type: it is a separate one.
        let external = screen_external(m.chassis) || (m.chassis.is_none() && m.screen.is_some());
        let mut check = DeviceCheck {
            read,
            virtual_machine: virtual_machine(
                m.hypervisor,
                m.maker.as_ref().map(Text::as_str),
                m.product.as_ref().map(Text::as_str),
            )
            .map(Text::of),
            external_screen: external,
            due: true,
            ..DeviceCheck::default()
        };
        check.set_screen(m.screen.as_ref());
        check
    }

    /// Takes the screen from `edid`; none given leaves what was. A TV or a KVM is held by its
    /// name. A screen whose maker makes no TVs, or a computer's own panel, needs no reminder
    /// that it may be a TV; a TV maker's separate screen whose base block names no TV may still
    /// be one, and so may any screen when the firmware passed no EDID.
    fn set_screen(&mut self, edid: Option<&Edid>) {
        let Some(Edid(e)) = edid else {
            return;
        };
        let kind = faraday_paper::edid::display(e).map(|d| match d {
            faraday_paper::edid::Display::Tv => Kind::Tv,
            faraday_paper::edid::Display::Kvm => Kind::Kvm,
        });
        if let Some(kind) = kind {
            self.screen = Some(Concern {
                key: Text::of(SCREEN),
                name: faraday_paper::edid::edid_name(e)
                    .map_or_else(|| Text::of(EN.devices_this_screen), |n| Text::of(&n)),
                kind,
            });
        }
        // The chassis alone does not settle it: a laptop's HDMI output may carry a TV, and its
        // EDID then says HDMI.
        self.screen_settled = kind.is_some()
            || !faraday_paper::edid::tv_maker(e)
            || (!self.external_screen && !faraday_paper::edid::hdmi_or_dvi(e));
    }

    /// Everything of concern now: the machine, the screen as its EDID named it, and the USB
    /// devices; a receiver is a reminder and holds nothing.
    fn now(&self) -> Vec<Concern> {
        let mut out = Vec::new();
        if let Some(name) = &self.virtual_machine {
            out.push(Concern {
                key: Text::of(MACHINE),
                name: name.clone(),
                kind: Kind::VirtualMachine,
            });
        }
        out.extend(self.screen.clone());
        out.extend(self.present.iter().filter(|c| !c.kind.receiver()).cloned());
        out.retain(|c| !self.let_through.contains(&c.key));
        out
    }

    /// The reminders that apply now: the screen may be a TV, unless its EDID settled it; a
    /// wireless receiver is attached.
    #[must_use]
    pub fn reminders(&self) -> Vec<Reminder> {
        let mut out = Vec::new();
        if self.read && !self.screen_settled && self.screen.is_none() {
            out.push(Reminder::Tv);
        }
        out.extend(self.receiver_reminder());
        out
    }

    /// The reminders not yet gone on from this session.
    #[must_use]
    pub fn reminders_due(&self) -> Vec<Reminder> {
        let mut out = self.reminders();
        out.retain(|r| !self.understood.contains(r));
        out
    }

    /// The person went on from the reminders due.
    pub(crate) fn understand(&mut self) {
        let due = self.reminders_due();
        self.understood.extend(due);
    }

    /// Whatever shows or takes a secret waits: an item is held, or a reminder has not been
    /// seen.
    #[must_use]
    pub fn holding(&self) -> bool {
        self.read && (!self.held.is_empty() || !self.reminders_due().is_empty())
    }

    /// The reminder for a wireless receiver attached now: the warning for any receiver but
    /// Logitech's Bolt, the Bolt's own when it is the only kind attached.
    fn receiver_reminder(&self) -> Option<Reminder> {
        let kinds = || self.present.iter().map(|c| c.kind);
        if kinds().any(|k| k == Kind::Receiver) {
            Some(Reminder::Receiver)
        } else if kinds().any(|k| k == Kind::BoltReceiver) {
            Some(Reminder::Bolt)
        } else {
            None
        }
    }

    /// A wireless receiver other than Logitech's Bolt is attached now.
    #[must_use]
    pub fn receiver_marked(&self) -> bool {
        self.present.iter().any(|c| c.kind == Kind::Receiver)
    }

    /// A virtual machine is among the items: no Continue anyway.
    #[must_use]
    pub fn refuses(&self) -> bool {
        self.held.iter().any(|c| c.kind == Kind::VirtualMachine)
    }

    /// Whether the machine was read: the check applies.
    #[must_use]
    pub fn read(&self) -> bool {
        self.read
    }

    /// What the sheet names.
    #[must_use]
    pub fn held(&self) -> &[Concern] {
        &self.held
    }

    /// Continue anyway was pressed once.
    #[must_use]
    pub fn asking(&self) -> bool {
        self.asking
    }

    pub(crate) fn set_asking(&mut self, on: bool) {
        self.asking = on;
    }

    /// The sheet is due: an item came and the sheet has not been up for it. Clears it.
    pub(crate) fn take_due(&mut self) -> bool {
        self.read && !self.held.is_empty() && std::mem::take(&mut self.due)
    }

    /// Check again: what was pulled goes from the sheet. Whether anything is left.
    pub(crate) fn check_again(&mut self) -> bool {
        self.asking = false;
        self.held = self.now();
        !self.held.is_empty()
    }

    /// Every item the sheet names is let through for this session. Never for a virtual machine,
    /// and only once the sheet asks. Whether it was done.
    pub(crate) fn let_all_through(&mut self) -> bool {
        if self.refuses() || !self.asking {
            return false;
        }
        self.asking = false;
        let keys: Vec<Text> = self.held.drain(..).map(|c| c.key).collect();
        self.let_through.extend(keys);
        true
    }

    /// What the machine says of itself is held from the start. Whether anything is.
    pub(crate) fn refresh(&mut self) -> bool {
        self.concerns_changed(false)
    }

    /// The USB devices attached now, as the adapter read them. Whether something of concern
    /// arrived.
    pub(crate) fn usb(&mut self, devices: &[UsbDevice]) -> bool {
        self.present = devices
            .iter()
            .filter_map(|u| {
                usb_kind(u).map(|kind| Concern {
                    key: Text::of(&format!(
                        "{}#{:04x}:{:04x}",
                        u.port.as_str(),
                        u.vendor,
                        u.product
                    )),
                    name: usb_name(u),
                    kind,
                })
            })
            .collect();
        self.concerns_changed(false)
    }

    /// The screen's EDID as the adapter reads it now. A screen that is a TV or a KVM is held by
    /// its name and one that may be a TV is a reminder; it is judged again for each screen, so
    /// one swapped in mid-session that was let through before is held again. Whether something
    /// arrived.
    pub(crate) fn screen_changed(&mut self, edid: Option<&Edid>) -> bool {
        if !self.read {
            return false;
        }
        let before = !self.reminders_due().is_empty();
        self.let_through.retain(|k| k.as_str() != SCREEN);
        self.held.retain(|c| c.key.as_str() != SCREEN);
        self.understood.retain(|r| *r != Reminder::Tv);
        self.screen = None;
        self.set_screen(edid);
        let reminded = !before && !self.reminders_due().is_empty();
        self.concerns_changed(reminded)
    }

    /// New items are held and bring the sheet up; gone ones stay named until Check again.
    /// `reminded`: a reminder is newly due. Whether anything arrived (and so the secrets on
    /// show go off the screen).
    fn concerns_changed(&mut self, reminded: bool) -> bool {
        let mut arrived = reminded;
        for c in self.now() {
            if !self.held.iter().any(|h| h.key == c.key) {
                self.held.push(c);
                self.due = true;
                self.asking = false;
                arrived = true;
            }
        }
        arrived && self.read
    }
}
