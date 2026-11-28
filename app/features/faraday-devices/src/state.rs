use faraday_kernel::{AtLock, Gate};

use crate::check::DeviceCheck;
use crate::inputs::Inputs;

/// The device check and the held-back inputs. A lock wipes the held devices' codes, places and
/// what they typed; the counts carry.
#[derive(AtLock)]
pub struct State {
    #[lock(public)]
    check: DeviceCheck,
    /// The devices held back; their codes are secret.
    #[lock(kept)]
    inputs: Inputs,
    /// The Devices sheet is up.
    #[lock(public)]
    devices_up: bool,
    /// The New input sheet is up.
    #[lock(public)]
    input_up: bool,
    /// The press that waits for the check: one slot, a second replaces the first.
    #[lock(public)]
    waiting: Option<Gate>,
    /// The pointer whose targets the adapter's arrow aims at.
    #[lock(public)]
    aimed: Option<u32>,
    /// Devices whose draw is on its way.
    #[lock(public)]
    asked: Vec<u32>,
}

impl State {
    pub(crate) fn new(check: DeviceCheck, inputs: Inputs) -> State {
        State {
            check,
            inputs,
            devices_up: false,
            input_up: false,
            waiting: None,
            aimed: None,
            asked: Vec::new(),
        }
    }

    /// The device check.
    #[must_use]
    pub fn check(&self) -> &DeviceCheck {
        &self.check
    }

    pub(crate) fn check_mut(&mut self) -> &mut DeviceCheck {
        &mut self.check
    }

    /// The input devices area.
    #[must_use]
    pub fn inputs(&self) -> &Inputs {
        &self.inputs
    }

    pub(crate) fn inputs_mut(&mut self) -> &mut Inputs {
        &mut self.inputs
    }

    /// The Devices sheet is up.
    #[must_use]
    pub fn devices_up(&self) -> bool {
        self.devices_up
    }

    pub(crate) fn set_devices_up(&mut self, on: bool) {
        self.devices_up = on;
    }

    /// The New input sheet is up.
    #[must_use]
    pub fn input_up(&self) -> bool {
        self.input_up
    }

    pub(crate) fn set_input_up(&mut self, on: bool) {
        self.input_up = on;
    }

    /// The press that waits for the check.
    #[must_use]
    pub fn waiting(&self) -> Option<Gate> {
        self.waiting
    }

    pub(crate) fn wait(&mut self, gate: Option<Gate>) {
        self.waiting = gate;
    }

    pub(crate) fn aimed(&self) -> Option<u32> {
        self.aimed
    }

    pub(crate) fn aim(&mut self, id: Option<u32>) {
        self.aimed = id;
    }

    /// Whether a draw for device `id` is on its way.
    pub(crate) fn is_asked(&self, id: u32) -> bool {
        self.asked.contains(&id)
    }

    pub(crate) fn ask(&mut self, id: u32, on: bool) {
        self.asked.retain(|a| *a != id);
        if on {
            self.asked.push(id);
        }
    }
}
