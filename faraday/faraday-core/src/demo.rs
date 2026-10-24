//! What Demo refuses (F20, F81). A session set up for the demo
//! ([`crate::wallet::Session::set_demo`], on Testnet) takes only the
//! samples (`crate::samples`).

/// The passphrase and password of Demo, all lowercase: every field that
/// takes one offers it in a chip (`docs/MODES.md` §6.2).
/// Generate with dice on Create a vault puts it in place of the rolled
/// words (`docs/DEMO.md` §5.2).
pub const PASSPHRASE: &str = "faraday";

use crate::backup_flow::steps::FlowAction as F;
use crate::grind::Dial;
use crate::vault::VaultAction as V;
use crate::{Action, Faraday, Screen};

impl Faraday {
    /// Whether this session makes secrets a person could take away: false
    /// in Demo (`Keys::demo`), true in every other mode, on the stick and
    /// on a test network alike (`docs/DEMO.md` §2).
    pub fn makes_secrets(&self) -> bool {
        !self.session.keys.demo()
    }

    /// Whether `action` is one the demo refuses as it stands now: an
    /// action of `docs/DEMO.md` §4. Its screen draws it dimmed and a press
    /// does nothing.
    pub fn demo_refuses(&self, action: Action) -> bool {
        if self.makes_secrets() {
            return false;
        }
        let slip39 = self.keygen.state().is_some_and(|k| k.slip39);
        match action {
            // Seed XOR parts and codex32 shares are fresh randomness; a
            // single codex32 string is the seed's own spelling.
            Action::BXor(_) => true,
            Action::BCodex32(k, _) => k >= 2,
            // New key as SLIP-39 shares: its sources and the deal.
            Action::KWay(_) | Action::KMix(_) => slip39,
            Action::KNext => self.keygen.state().is_some_and(|k| {
                k.slip39
                    && matches!(
                        k.open,
                        Some(crate::keygen::kstep::SOURCE | crate::keygen::kstep::ENTER)
                    )
            }),
            // With every sample of the length chosen loaded none is left to stand in.
            // A key typed is loaded as Add a key loads it.
            Action::KLock => self.keygen.state().is_some_and(|k| {
                !k.typed() && self.session.keys.next_handout(k.handout_words()).is_none()
            }),
            // Make the key, for a GPG key, or Secure Boot keys.
            Action::Flow(F::Make) => self
                .flows
                .on()
                .and_then(|i| self.flows.get(i))
                .is_some_and(|f| !f.kind.is_wallet()),
            // Every BIP-85 output, and the way to a vault to keep one.
            Action::PShow | Action::PVault | Action::PLoad | Action::POut => true,
            // A vanity passphrase is a new key.
            Action::Vanity(crate::vanity::VanityAction::Dial(i)) => {
                Dial::ALL.get(usize::from(i)) == Some(&Dial::Passphrase)
            }
            // A node's private key and a scan key leaving Faraday. An LND
            // cipher seed is open: `node_from_aezeed` takes only the
            // published one.
            Action::LShow | Action::LVault | Action::LOut => true,
            Action::SScanVault | Action::SScanOut => true,
            // The way to a vault to keep one of those.
            Action::Vault(V::CreateFrom(s) | V::OpenFrom(_, s) | V::ListFrom(s)) => {
                matches!(s, Screen::Bip85 | Screen::Lightning | Screen::Silent)
            }
            _ => false,
        }
    }

    /// Every action the frame draws dimmed now: those of
    /// [`Faraday::demo_refuses`] the screens can draw.
    pub(crate) fn demo_dimmed(&self) -> Vec<Action> {
        if self.makes_secrets() {
            return Vec::new();
        }
        let mut all = vec![
            Action::BXor(2),
            Action::BXor(3),
            Action::BCodex32(2, 3),
            Action::BCodex32(3, 5),
            Action::KNext,
            Action::KLock,
            Action::Flow(F::Make),
            Action::PShow,
            Action::PVault,
            Action::PLoad,
            Action::POut,
            Action::CNext(crate::cstep::KEYS),
            Action::LShow,
            Action::LVault,
            Action::LOut,
            Action::SScanVault,
            Action::SScanOut,
        ];
        all.extend((0..crate::keygen::WAYS.len() as u8).map(Action::KWay));
        all.extend((0..crate::keygen::MIX_SOURCES.len() as u8).map(Action::KMix));
        all.extend(
            (0..Dial::ALL.len() as u8)
                .map(|i| Action::Vanity(crate::vanity::VanityAction::Dial(i))),
        );
        for s in [Screen::Bip85, Screen::Lightning, Screen::Silent] {
            all.push(Action::Vault(V::CreateFrom(s)));
            all.push(Action::Vault(V::ListFrom(s)));
            all.extend((0..self.vault_files().len()).map(|k| Action::Vault(V::OpenFrom(k, s))));
        }
        all.retain(|&a| self.demo_refuses(a));
        all
    }
}

/// How a passphrase field draws (`docs/MODES.md` §6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Show {
    /// As dots; the eye shows it.
    Masked,
    /// In the clear; the eye hides it.
    Shown,
    /// Demo's [`PASSPHRASE`] in its code box, unmasked, with no eye: it is
    /// public, and a press in the field lets the person clear or replace
    /// it.
    Code,
}

/// A passphrase or password field that offers [`PASSPHRASE`] in Demo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassField {
    /// Add a key's BIP-39 passphrase.
    Entry,
    /// New key's passphrase (0) and its repeat (1).
    KeyGen(u8),
    /// The potential wallet's passphrase.
    Potential,
    /// A seed read's passphrase (0) and its repeat (1).
    SeedRead(u8),
    /// A Create slot's passphrase (0) and its repeat (1).
    Slot(u8),
    /// Create a vault's passphrase `i`, and its repeat.
    Phrase(u8, bool),
    /// The vault's unlock, the boot import's, a backup's.
    Unlock,
    /// A passphrase prompt's.
    Prompt,
    /// A vault entry's password field `k` of its form.
    Password(u8),
    /// Check my copy's passphrase typed back.
    Check,
    /// An aezeed's passphrase.
    Aezeed,
}

/// A passphrase field with an eye at its right end, as
/// [`secret_box`](crate::vault::screen::secret_box) draws it: in Demo what
/// holds `faraday` is in its code box, and an empty field not pressed in
/// offers its chip. `(shown, focused)`, `(action, eye)`.
pub(crate) fn pass_box(
    app: &crate::drawing::View,
    ui: &mut crate::ui::Ui,
    (x, y, w): (f32, f32, f32),
    b: &crate::vault::TextBox,
    (shown_eye, focused): (bool, bool),
    (action, eye): (Action, Action),
    field: PassField,
) {
    let h = faraday_draw::tokens::TEXT_FIELD;
    // No caret under the chip: it stands where the typing would start.
    let chip = app.pass_chip_shown(field, b.text.is_empty(), focused);
    crate::vault::screen::secret_box(
        ui,
        x,
        y,
        w,
        b,
        app.pass_shows(shown_eye, &b.text),
        focused && !chip,
        action,
        eye,
    );
    if focused && chip {
        ui.stroke(
            x,
            y,
            w,
            h,
            faraday_draw::tokens::FIELD_RADIUS,
            crate::ui::pal::Role::Primary,
        );
    }
    app.pass_chip_draw(ui, (x, y, w, h), (b.text.is_empty(), focused, false), field);
}

/// Whether pressing `a` puts the caret in a passphrase field: the chip of
/// an empty field is gone from then on, until another press.
pub(crate) fn presses_pass_field(a: Action) -> bool {
    crate::field_action(a) || matches!(a, Action::PotentialType | Action::LPassphrase)
}

impl crate::vault::Form {
    /// Whether field `k` is a vault entry's password, which Demo offers
    /// [`PASSPHRASE`] for.
    pub(crate) fn is_password(&self, k: usize) -> bool {
        use faraday_vault::records::{field, kind};
        self.kind == kind::ENTRY && self.fields.get(k).is_some_and(|f| f.0 == field::PASSWORD)
    }
}

impl Faraday {
    /// Whether this is Demo, where every passphrase field offers
    /// [`PASSPHRASE`] in a chip (`docs/MODES.md` §6.2). The one test every
    /// passphrase field asks.
    pub fn demo_passphrase(&self) -> bool {
        self.session.keys.demo()
    }

    /// Whether passphrase `i` of Create a vault is `faraday` in place of
    /// the words the dice rolled for it: its card says so
    /// (`docs/DEMO.md` §5.2). A lookup.
    pub(crate) fn dice_made_public(&self, c: &crate::vault::CreateForm, i: usize) -> bool {
        self.demo_passphrase()
            && c.from_dice
                .get(i)
                .and_then(|d| d.as_ref())
                .is_some_and(|(made, _, _)| made.as_str() == PASSPHRASE)
    }

    /// Whether `text` is Demo's public [`PASSPHRASE`], which is not
    /// measured for strength.
    pub(crate) fn pass_is_public(&self, text: &str) -> bool {
        self.demo_passphrase() && text == PASSPHRASE
    }

    /// How a field holding `text` draws that the eye masks or shows: in
    /// Demo what holds [`PASSPHRASE`] draws in its code box, unmasked.
    pub(crate) fn pass_shows(&self, shown: bool, text: &str) -> Show {
        if self.demo_passphrase() && text == PASSPHRASE {
            Show::Code
        } else if shown {
            Show::Shown
        } else {
            Show::Masked
        }
    }

    /// How field `k` of the entry form draws: masked or shown by the eye,
    /// and an entry's password holding [`PASSPHRASE`] in Demo in its code
    /// box.
    pub(crate) fn field_shows(&self, form: &crate::vault::Form, k: usize) -> Show {
        let text = form.fields.get(k).map_or("", |f| f.2.text.as_str());
        if form.is_password(k) {
            self.pass_shows(self.vaults.typed_shown, text)
        } else if self.vaults.typed_shown {
            Show::Shown
        } else {
            Show::Masked
        }
    }

    /// The press that fills `field` with [`PASSPHRASE`], in Demo only:
    /// what a field draws its chip for,
    /// and whether it is drawn now: not while the person has pressed in
    /// the field `focused`, or typed.
    pub(crate) fn pass_chip(&self, field: PassField, focused: bool) -> Option<(Action, bool)> {
        self.demo_passphrase().then_some((
            Action::PassChip(field),
            !(focused && self.sample_area.pass_pressed()),
        ))
    }

    /// Draws the `faraday` chip inside the empty, unfocused field `r`
    /// (x, y, w, h) of `field`, at its left; `right` puts it at the right
    /// end, beside the **No passphrase** pill of an optional field. The
    /// KeePass export's prompt has none: `faraday` is under its minimum.
    pub(crate) fn pass_chip_draw(
        &self,
        ui: &mut crate::ui::Ui,
        r: (f32, f32, f32, f32),
        (empty, focused, right): (bool, bool, bool),
        field: PassField,
    ) {
        if !self.pass_chip_shown(field, empty, focused) {
            return;
        }
        if let Some((a, _)) = self.pass_chip(field, focused) {
            crate::samples::put::pass_chip(ui, r, a, right);
        }
    }

    /// Whether the chip of `field` is on screen: Demo, the field `empty`,
    /// the person not pressed in it (`focused`) or typed, and the field
    /// offering one (the KeePass export's prompt does not). The caret is
    /// not drawn in the field while it is. A lookup.
    pub(crate) fn pass_chip_shown(&self, field: PassField, empty: bool, focused: bool) -> bool {
        empty
            && !(field == PassField::Prompt && !self.prompt_chip())
            && matches!(self.pass_chip(field, focused), Some((_, true)))
    }

    /// The chip of a vault entry's password field `k` of `form`, when it
    /// is one, empty and not pressed in.
    pub(crate) fn password_chip(
        &self,
        ui: &mut crate::ui::Ui,
        form: &crate::vault::Form,
        k: usize,
        (x, y, w): (f32, f32, f32),
        (b, focused): (&crate::vault::TextBox, bool),
    ) {
        if form.is_password(k) {
            let h = faraday_draw::tokens::TEXT_FIELD;
            let field = PassField::Password(k as u8);
            if focused && self.pass_chip_shown(field, b.text.is_empty(), focused) {
                ui.stroke(
                    x,
                    y,
                    w,
                    h,
                    faraday_draw::tokens::FIELD_RADIUS,
                    crate::ui::pal::Role::Primary,
                );
            }
            self.pass_chip_draw(ui, (x, y, w, h), (b.text.is_empty(), focused, false), field);
        }
    }

    /// Whether the caret draws in field `k` of `form`, which is `focused`:
    /// not under the chip of an empty password.
    pub(crate) fn field_caret(
        &self,
        form: &crate::vault::Form,
        k: usize,
        (b, focused): (&crate::vault::TextBox, bool),
    ) -> bool {
        let chip = form.is_password(k)
            && self.pass_chip_shown(PassField::Password(k as u8), b.text.is_empty(), focused);
        focused && !chip
    }

    /// A key is typed (or erased): the chip of a field in focus is gone.
    /// Returns false: the key goes on.
    pub(crate) fn pass_typed(&mut self, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key as K;
        if matches!(key, K::Char(_) | K::Backspace) {
            self.sample_area.pressed_past();
        }
        false
    }

    /// Whether the keyboard waits on a small panel: in Demo the field in
    /// focus is an empty passphrase the person has not pressed in, so its
    /// chip shows and the keyboard does not cover it. A lookup.
    pub(crate) fn pass_chip_waiting(&self) -> bool {
        use crate::vault::Focus as F;
        if !self.demo_passphrase() || self.sample_area.pass_pressed() {
            return false;
        }
        let v = &self.vaults;
        match v.focus {
            Some(F::Passphrase) => v.passphrase.text.is_empty(),
            Some(F::Prompt) => {
                self.prompt_chip() && v.prompt.as_ref().is_some_and(|p| p.text.text.is_empty())
            }
            Some(F::Phrase(i, again)) => v.create.as_ref().is_some_and(|c| {
                c.phrases
                    .get(i)
                    .is_some_and(|(a, b)| if again { b } else { a }.text.is_empty())
            }),
            _ => false,
        }
    }

    /// Whether the open prompt takes the `faraday` chip: not the KeePass
    /// export's.
    fn prompt_chip(&self) -> bool {
        !self
            .vaults
            .prompt
            .as_ref()
            .is_some_and(|p| matches!(p.purpose, crate::vault::Purpose::Kdbx))
    }

    /// A press on the `faraday` chip of `field`: the field holds
    /// [`PASSPHRASE`], and what depends on it is worked out as it is when
    /// typed. A field's repeat is its own chip.
    pub(crate) fn demo_pass_chip(&mut self, field: PassField) {
        if !self.demo_passphrase() {
            return;
        }
        match field {
            PassField::Entry => {
                self.entry.state_mut().passphrase.set(PASSPHRASE);
                self.entry.state_mut().error = None;
            }
            PassField::KeyGen(i) => {
                self.keygen.demo_passphrase(i, PASSPHRASE);
            }
            PassField::Potential => {
                if let Some(p) = self.inbox.potential_mut() {
                    p.passphrase.set(PASSPHRASE);
                    p.typing = true;
                    p.error = None;
                    self.potential_typed();
                }
            }
            PassField::SeedRead(i) => self.seed_read_filled(i),
            PassField::Slot(i) => {
                if let Some(c) = self.create.state_mut() {
                    let t = if i == 0 { &mut c.pass } else { &mut c.pass2 };
                    t.set(PASSPHRASE);
                    c.pass_focus = Some(i);
                    c.pass_note = None;
                }
            }
            PassField::Phrase(i, again) => {
                self.vaults.fill_phrase(usize::from(i), again, PASSPHRASE);
            }
            PassField::Unlock => {
                self.vaults.fill_unlock(PASSPHRASE);
            }
            PassField::Prompt => {
                self.vaults.fill_prompt(PASSPHRASE);
            }
            PassField::Password(k) => {
                self.vaults.fill_field(usize::from(k), PASSPHRASE);
            }
            PassField::Check => {
                if let Some(c) = self.backup.check_mut() {
                    c.typed.set(PASSPHRASE);
                }
            }
            PassField::Aezeed => {
                self.lightning.set_passphrase(PASSPHRASE);
            }
        }
    }
}
