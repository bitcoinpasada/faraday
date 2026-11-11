//! The create-vault form's cards: what a press on one changes in the form.
//! It touches the vault area alone; the glue calls it first.

use super::{
    CUSTOM, CUSTOM_MEMORY, CUSTOM_PASSES, Focus, MACHINES, SIZES, VaultAction, VaultArea, vstep,
};

impl VaultArea {
    /// A press on the create form's cards. `may_load` is whether typing
    /// may take the focus (`Faraday::may_load_keys`). False when `a` is
    /// not one of the form's.
    pub(crate) fn create_card_act(&mut self, a: VaultAction, may_load: bool) -> bool {
        use VaultAction as V;
        match a {
            V::CStep(k) => {
                if let Some(c) = self.state.create.as_mut() {
                    c.open = if c.open == Some(k) { None } else { Some(k) };
                    c.scroll.follow = true;
                    if c.open == Some(vstep::PHRASES) {
                        self.state.focus = Some(Focus::Phrase(0, false));
                    } else if matches!(self.state.focus, Some(Focus::Phrase(..) | Focus::Name)) {
                        // Typing goes nowhere on a card with no field.
                        self.state.focus = None;
                    }
                }
            }
            V::CNext(k) => {
                if let Some(c) = self.state.create.as_mut()
                    && usize::from(k) < vstep::COUNT
                {
                    c.done[k as usize] = true;
                    c.open = c.steps().iter().copied().find(|&i| !c.done[i as usize]);
                    c.scroll.follow = true;
                    c.error = None;
                    if c.open == Some(vstep::PHRASES) {
                        self.state.focus = Some(Focus::Phrase(0, false));
                    }
                }
            }
            V::CHere => {
                if let Some(c) = self.state.create.as_mut() {
                    c.not_here = !c.not_here;
                    c.error = None;
                }
            }
            V::CMachine(k) => {
                if let Some(c) = self.state.create.as_mut()
                    && k < MACHINES.len()
                {
                    c.on[k] = !c.on[k];
                    c.error = None;
                }
            }
            V::CPreset(k) => {
                if let Some(c) = self.state.create.as_mut() {
                    c.preset = Some(k.min(CUSTOM));
                    c.error = None;
                }
            }
            V::CSizeRow(k) => {
                if let Some(c) = self.state.create.as_mut()
                    && let Some(&(_, preset, on)) = SIZES.get(k)
                {
                    c.preset = Some(preset);
                    c.on = on;
                    c.not_here = false;
                    c.error = None;
                }
            }
            V::CCustomise => {
                if let Some(c) = self.state.create.as_mut() {
                    // The two cards open on the preset's values, to be
                    // walked through in order.
                    c.customise = true;
                    for k in [vstep::WHERE, vstep::COST] {
                        c.done[usize::from(k)] = false;
                    }
                    if matches!(self.state.focus, Some(Focus::Phrase(..) | Focus::Name)) {
                        self.state.focus = None;
                    }
                    c.open = Some(vstep::WHERE);
                    c.scroll.follow = true;
                    c.error = None;
                }
            }
            V::CMemory(k) => {
                if let Some(c) = self.state.create.as_mut() {
                    c.memory = CUSTOM_MEMORY[k.min(CUSTOM_MEMORY.len() - 1)];
                    c.preset = Some(CUSTOM);
                }
            }
            V::CPasses(k) => {
                if let Some(c) = self.state.create.as_mut() {
                    c.passes = CUSTOM_PASSES[k.min(CUSTOM_PASSES.len() - 1)];
                    c.preset = Some(CUSTOM);
                }
            }
            V::CFocus(i, second) => {
                self.state.focus = may_load.then_some(Focus::Phrase(i, second));
            }
            V::CName => self.state.focus = Some(Focus::Name),
            V::CShow => {
                if let Some(c) = self.state.create.as_mut() {
                    c.shown = !c.shown;
                }
            }
            _ => return false,
        }
        true
    }
}
