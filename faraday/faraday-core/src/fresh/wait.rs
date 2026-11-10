//! A press that needs the system's randomness before the seed has
//! arrived (`docs/OWN-RANDOMNESS.md` §3.5, F102): it waits, its button
//! reads "Waiting for randomness", and it goes on by itself once the
//! seed and the answers a press takes are in. A minute after the first
//! display with no seed, the press says the system gives no randomness
//! and does nothing.

use faraday_shell_api::Command;

use crate::{Action, Faraday, strings::EN, vault::VaultAction as V};

impl Faraday {
    /// Whether `action` is a press that draws from the system's
    /// randomness now (`docs/OWN-RANDOMNESS.md` §8.1's rows 2, 3, 5 to
    /// 13). A press that draws nothing in the state it is pressed in is
    /// not one: Next before the Enter step, Sign here without a MuSig2
    /// round.
    fn draws_system_randomness(&self, action: Action) -> bool {
        use crate::keygen::kstep;
        match action {
            Action::KNext => self
                .keygen
                .state()
                .is_some_and(|k| k.slip39 && k.open == Some(kstep::ENTER)),
            Action::Codex32(crate::codex32::C32::Suggest) => true,
            Action::BXor(_) => self.makes_secrets(),
            Action::SignHere => self
                .wallets
                .spend()
                .is_some_and(|s| crate::spend_opens_round(&s.inspection)),
            Action::Vault(V::CGo | V::SbEnrol | V::SbHoldSign) => true,
            Action::Vault(V::PromptGo) => self
                .vaults
                .prompt
                .as_ref()
                .is_some_and(|p| matches!(p.purpose, crate::vault::Purpose::Kdbx)),
            Action::Flow(crate::backup_flow::steps::FlowAction::Make) => self
                .flow()
                .is_some_and(|f| !f.kind.is_wallet() && !f.made && self.makes_secrets()),
            Action::BC32(crate::codex32_backup::Press::Set(
                crate::codex32_backup::SetPress::MakeNew,
            )) => true,
            _ => false,
        }
    }

    /// Run first by every press: a new press lets go of one that waits.
    /// Returns whether this press waits now (or was refused as one that
    /// can never be served), so nothing else is done with it.
    pub(crate) fn seed_gate(&mut self, action: Action) -> bool {
        self.fresh.wait_cancel();
        if self.seeded() || !self.draws_system_randomness(action) {
            return false;
        }
        if self.randomness_gives_none() {
            self.toast(EN.no_randomness);
        } else {
            let answers = self.answers_for(action);
            self.wait_for_randomness(action, answers);
        }
        true
    }

    /// The answers `action` takes from the pool, besides the seed.
    fn answers_for(&self, action: Action) -> usize {
        match action {
            Action::SignHere => 0,
            Action::Codex32(_) | Action::Vault(V::SbEnrol | V::SbHoldSign) => 1,
            Action::Flow(_) => 2,
            Action::BXor(n) => faraday_keys::seed_xor_random_len(n).div_ceil(32),
            Action::Vault(_) => 8,
            _ => crate::fresh::POOL,
        }
    }

    /// `action` waits for the seed and the answers; they are asked for.
    pub(crate) fn wait_for_randomness(&mut self, action: Action, answers: usize) {
        let now = self.now_ms;
        self.fresh.wait_on(action, now, answers);
        self.out.command(Command::RequestEntropy);
        self.out.redraw();
    }

    /// Whether `action` is the press that waits: its button reads
    /// "Waiting for randomness".
    pub fn waits_for_randomness(&self, action: Action) -> bool {
        self.fresh.waiting() == Some(action)
    }

    /// `label`, or "Waiting for randomness" while `action` waits.
    pub fn wait_label<'a>(&self, action: Action, label: &'a str) -> &'a str {
        if self.waits_for_randomness(action) {
            EN.waiting_for_randomness
        } else {
            label
        }
    }

    /// An answer arrived: the press that waits goes on once they are in.
    pub(crate) fn seed_replay(&mut self) {
        if let Some(action) = self.fresh.take_ready() {
            self.act(action);
        }
    }
}
