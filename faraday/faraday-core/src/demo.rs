//! What Demo refuses (F20, F81). A session set up for the demo
//! ([`crate::wallet::Session::set_demo`], on Testnet) takes only the
//! samples (`crate::samples`).

/// The one passphrase and password of Demo, all lowercase: every field
/// that takes one holds it and takes no typing (`docs/MODES.md` §6.2).
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
        let slip39 = self.keygen.as_ref().is_some_and(|k| k.slip39);
        match action {
            // Seed XOR parts and codex32 shares are fresh randomness; a
            // single codex32 string is the seed's own spelling.
            Action::BXor(_) => true,
            Action::BCodex32(k, _) => k >= 2,
            // New key as SLIP-39 shares: its sources and the deal.
            Action::KWay(_) | Action::KMix(_) => slip39,
            Action::KNext => self.keygen.as_ref().is_some_and(|k| {
                k.slip39
                    && matches!(
                        k.open,
                        Some(crate::keygen::kstep::SOURCE | crate::keygen::kstep::ENTER)
                    )
            }),
            // With every sample of the length chosen loaded none is left to stand in.
            // A key typed is loaded as Add a key loads it.
            Action::KLock => self.keygen.as_ref().is_some_and(|k| {
                !k.typed() && self.session.keys.next_handout(k.handout_words()).is_none()
            }),
            // Make the key, for a GPG key, Secure Boot keys or an APK key.
            Action::Flow(F::Make) => self
                .flow_on
                .and_then(|i| self.backup_flows.get(i))
                .is_some_and(|f| !f.kind.is_wallet()),
            // Every BIP-85 output, and the way to a vault to keep one.
            Action::PShow | Action::PVault | Action::PLoad | Action::POut => true,
            // A vanity passphrase is a new key.
            Action::Vanity(crate::vanity::VanityAction::Dial(i)) => {
                Dial::ALL.get(usize::from(i)) == Some(&Dial::Passphrase)
            }
            // The FROST deal: Keys' Continue on a threshold wallet.
            Action::CNext(k) => {
                k == crate::cstep::KEYS
                    && self
                        .create
                        .as_ref()
                        .is_some_and(|c| c.kind.threshold() && c.built.is_none())
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
    /// Demo's: [`PASSPHRASE`] in its code box, unmasked, with no eye, and
    /// no typing.
    Fixed,
}

impl crate::vault::Form {
    /// Whether field `k` is a vault entry's password, which Demo fixes.
    pub(crate) fn is_password(&self, k: usize) -> bool {
        use faraday_vault::records::{field, kind};
        self.kind == kind::ENTRY && self.fields.get(k).is_some_and(|f| f.0 == field::PASSWORD)
    }
}

impl Faraday {
    /// Whether this is Demo, where every passphrase and password is
    /// [`PASSPHRASE`] (`docs/MODES.md` §6.2). The one test every
    /// passphrase field asks, where it takes a key and where it draws.
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
                .is_some_and(|(made, _)| made.as_str() == PASSPHRASE)
    }

    /// How a field draws that the eye masks or shows: fixed in Demo.
    pub(crate) fn pass_shows(&self, shown: bool) -> Show {
        match (self.demo_passphrase(), shown) {
            (true, _) => Show::Fixed,
            (false, true) => Show::Shown,
            (false, false) => Show::Masked,
        }
    }

    /// How field `k` of the entry form draws: an entry's password is
    /// fixed in Demo, any other secret field masks or shows.
    pub(crate) fn field_shows(&self, form: &crate::vault::Form, k: usize) -> Show {
        match (
            self.demo_passphrase() && form.is_password(k),
            self.vaults.typed_shown,
        ) {
            (true, _) => Show::Fixed,
            (false, true) => Show::Shown,
            (false, false) => Show::Masked,
        }
    }

    /// Whether typing goes now to a passphrase field that holds
    /// [`PASSPHRASE`] in Demo: the keys do nothing there and no keyboard
    /// docks for it. A lookup.
    pub(crate) fn demo_pass_typing(&self) -> bool {
        use crate::vault::Focus as F;
        use crate::{Screen as S, Sheet};
        if !self.demo_passphrase() {
            return false;
        }
        let vault = || match self.vaults.focus {
            Some(F::Passphrase | F::Phrase(..) | F::Prompt) => true,
            Some(F::Field(k)) => self.vaults.form.as_ref().is_some_and(|f| f.is_password(k)),
            _ => false,
        };
        match self.sheet {
            Some(Sheet::Potential) => self.potential.as_ref().is_some_and(|p| p.typing),
            Some(Sheet::SeedRead) => self.seed_read.as_ref().is_some_and(|s| s.focus.is_some()),
            Some(Sheet::Import | Sheet::PassphraseSeeds) => vault(),
            _ => match self.screen {
                S::Unlock | S::CreateVault | S::VaultContents | S::BackupFlow | S::Family => {
                    vault()
                }
                S::Entry => self.entry.on_passphrase,
                S::KeyGen => self.keygen.as_ref().is_some_and(|k| {
                    !k.locked && k.open == Some(crate::keygen::kstep::KEY) && k.focus.is_some()
                }),
                S::Create => self.create.as_ref().is_some_and(|c| c.pass_focus.is_some()),
                S::Backup => self
                    .backup
                    .as_ref()
                    .and_then(|b| b.check.as_ref())
                    .is_some_and(|c| c.passphrase),
                S::Lightning => self.lightning.as_ref().is_some_and(|l| l.on_passphrase),
                _ => false,
            },
        }
    }

    /// Whether `key` would be typed into a Demo passphrase field, which
    /// takes no typing: swallowed. Enter, Escape and Tab still go on.
    pub(crate) fn demo_pass_swallows(&self, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key as K;
        matches!(key, K::Char(_) | K::Backspace | K::Delete) && self.demo_pass_typing()
    }

    /// Puts [`PASSPHRASE`] in every passphrase field in Demo that is not
    /// optional, or that the person has chosen to use, once an event is
    /// done: the vault's unlock, Create a vault's pairs, a prompt, Add a
    /// passphrase on a slot or a seed read, a backup's, Check my copy's.
    /// The optional ones (Add a key's, New key's, the potential wallet's,
    /// an aezeed's) are filled by a press on them
    /// ([`Faraday::demo_pass_toggle`]). A field already holding it is
    /// left; nothing is derived unless a field was empty.
    pub(crate) fn demo_pass_sync(&mut self) {
        if !self.demo_passphrase() {
            return;
        }
        let v = &mut self.vaults;
        if v.passphrase.text.is_empty() {
            v.passphrase.set(PASSPHRASE);
        }
        if let Some(c) = v.create.as_mut() {
            for (a, b) in &mut c.phrases {
                for t in [a, b] {
                    if t.text.as_str() != PASSPHRASE {
                        t.set(PASSPHRASE);
                    }
                }
            }
        }
        if let Some(p) = v.prompt.as_mut()
            && p.text.text.is_empty()
        {
            p.text.set(PASSPHRASE);
        }
        // A new entry's password; an entry edited keeps the one it has.
        if let Some(f) = v.form.as_mut().filter(|f| f.record.is_none()) {
            for k in 0..f.fields.len() {
                if f.is_password(k) && f.fields[k].2.text.is_empty() {
                    f.fields[k].2.set(PASSPHRASE);
                }
            }
        }
        if let Some(c) = self.create.as_mut()
            && c.pass_slot.is_some()
            && (c.pass.is_empty() || c.pass2.is_empty())
        {
            c.pass.set(PASSPHRASE);
            c.pass2.set(PASSPHRASE);
        }
        if let Some(c) = self.backup.as_mut().and_then(|b| b.check.as_mut())
            && c.passphrase
            && c.typed.is_empty()
        {
            c.typed.set(PASSPHRASE);
        }
        let asked = self.seed_read.as_mut().is_some_and(|s| {
            let fill = s.adding && s.pass.is_empty();
            if fill {
                s.pass.set(PASSPHRASE);
                s.pass2.set(PASSPHRASE);
            }
            fill
        });
        if asked {
            self.seed_read_typed();
        }
        let backup = self.potential.as_mut().is_some_and(|p| {
            let fill = p.backup.is_some() && p.passphrase.is_empty();
            if fill {
                p.passphrase.set(PASSPHRASE);
            }
            fill
        });
        if backup {
            self.potential_typed();
        }
    }

    /// A press on an optional passphrase field in Demo: the person
    /// chooses whether to use a passphrase, and when they do it is
    /// [`PASSPHRASE`]. Both fields of a pair are filled alike, or both
    /// emptied.
    pub(crate) fn demo_pass_toggle(
        a: &mut crate::secret_text::SecretText,
        b: Option<&mut crate::secret_text::SecretText>,
    ) {
        let text = if a.is_empty() { PASSPHRASE } else { "" };
        a.set(text);
        if let Some(b) = b {
            b.set(text);
        }
    }
}
