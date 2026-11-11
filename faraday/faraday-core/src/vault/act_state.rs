//! The vault screens' presses that only move the vault area's own state:
//! which vault, kind and item is shown, what is ticked, which sheet's
//! fields are open. The glue calls them and does what else the press needs.

use super::{Focus, VaultAction, VaultArea};

impl VaultArea {
    /// A press that changes only this area's state. `may_load` is whether
    /// typing may take the focus (`Faraday::may_load_keys`). False when
    /// `a` is not one of them.
    pub(crate) fn state_act(&mut self, a: VaultAction, may_load: bool) -> bool {
        use VaultAction as V;
        match a {
            V::EachShown(v) => {
                if let Some(o) = self.state.open.get_mut(v) {
                    o.each_shown = !o.each_shown;
                }
            }
            V::Choose(v, r) => {
                if let Some(o) = self.state.open.get_mut(v)
                    && !o.skip.remove(&r)
                {
                    o.skip.insert(r);
                }
            }
            V::Pick(i) => {
                self.clear_unlock_parts();
                self.state.pick = i;
                self.state.unlock_error = None;
                self.state.focus = may_load.then_some(Focus::Passphrase);
            }
            V::FocusPassphrase => self.state.focus = may_load.then_some(Focus::Passphrase),
            V::FocusYubiKey => self.state.focus = may_load.then_some(Focus::YubiKey),
            V::SaveTick(k) => {
                if let Some(f) = self
                    .state
                    .stick_save
                    .as_mut()
                    .and_then(|s| s.files.get_mut(usize::from(k)))
                {
                    f.ticked = !f.ticked;
                }
            }
            V::ShowTyped => self.state.typed_shown = !self.typed_shown,
            V::PromptCancel => {
                self.state.prompt = None;
                self.state.focus = None;
            }
            V::GpgYears(y) => self.state.gpg_years = y,
            V::GpgReason(n) => self.state.revoke_reason = n.min(3),
            V::GpgRenewTerm(y) => self.state.renew_years = Some(y),
            V::SbOwnOnly(on) => self.state.sb_own_only = on,
            V::SbImages => {
                self.state.sb_images = !self.sb_images;
                self.state.sb_sign = None;
                self.state.sb_checked = None;
            }
            _ => return false,
        }
        true
    }

    /// Select or clear every row of vault `v` that is not loaded: all are
    /// chosen unless all already are, then all are cleared. `open` is the
    /// records of those rows.
    pub(crate) fn choose_all(&mut self, v: usize, open: Vec<usize>) {
        if let Some(o) = self.state.open.get_mut(v) {
            if open.iter().all(|r| !o.skip.contains(r)) {
                o.skip.extend(open);
            } else {
                for r in open {
                    o.skip.remove(&r);
                }
            }
        }
    }

    /// Vault `i` is shown in contents, nothing of it open.
    pub(crate) fn show_vault(&mut self, i: usize) {
        if i < self.open.len() {
            self.state.current = i;
            self.state.form = None;
            self.state.saving = false;
            self.state.item_open = false;
            self.set_item_offset(0.0);
        }
    }

    /// A kind's tab is shown, its list from the top.
    pub(crate) fn show_tab(&mut self, k: usize, list_offset: &mut f32, compact: bool) {
        self.show_category(k, list_offset, compact);
        self.set_item_offset(0.0);
        self.state.category_chosen = true;
        self.state.form = None;
        self.state.saving = false;
        self.state.item_open = false;
    }

    /// Item `k` of the shown kind is opened.
    pub(crate) fn select_item(&mut self, k: usize) {
        self.state.item[self.category] = k;
        self.set_item_offset(0.0);
        self.state.form = None;
        self.state.saving = false;
        self.state.item_open = true;
    }

    /// The open item is closed, and what hung off it.
    pub(crate) fn close_item(&mut self) {
        self.state.item_open = false;
        self.state.form = None;
        self.state.prompt = None;
        self.state.saving = false;
        self.state.sb_images = false;
        self.state.sb_sign = None;
        self.state.focus = None;
    }
}
