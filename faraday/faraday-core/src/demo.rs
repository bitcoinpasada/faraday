//! The demo's public seeds (F20). A session set up for the demo
//! ([`crate::wallet::Session::set_demo`]) takes only these on mainnet;
//! on a test network it takes any seed.

/// The public seeds, as their words: each of these words, twenty-four
/// times, is a valid BIP-39 phrase.
pub const SEEDS: [&str; 11] = [
    "bacon", "flag", "gas", "great", "slice", "solution", "summer", "they", "trade", "trap",
    "zebra",
];

/// The demo's public passphrase: six words of the EFF large list run
/// together, as Faraday joins the words the dice make. Generate with dice
/// on Create a vault puts it in place of the rolled words (`docs/DEMO.md`
/// §5.2); the website's note publishes it.
pub const PASSPHRASE: &str = "brightharborcalmbreezequietshelter";

/// How many EFF large-list words [`PASSPHRASE`] is.
pub const PASSPHRASE_WORDS: usize = 6;

/// A public seed's phrase: its word twenty-four times.
pub fn words(word: &str) -> String {
    vec![word; 24].join(" ")
}

use crate::backup_flow::steps::FlowAction as F;
use crate::grind::Dial;
use crate::vault::VaultAction as V;
use crate::{Action, Faraday, Screen};

impl Faraday {
    /// Whether this session makes secrets a person could take away: false
    /// while the demo is on (`Session::demo`), true otherwise, on the
    /// stick and on a test network alike (`docs/DEMO.md` §2).
    pub fn makes_secrets(&self) -> bool {
        !self.session.demo()
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
            // With every public seed loaded none is left to stand in.
            // A key typed is loaded as Add a key loads it.
            Action::KLock => {
                !self
                    .keygen
                    .as_ref()
                    .is_some_and(crate::keygen::KeyGen::typed)
                    && self.session.next_handout().is_none()
            }
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
            // A node's private key and a scan key leaving Faraday; an LND
            // cipher seed typed would be someone's real node.
            Action::LShow | Action::LVault | Action::LOut | Action::LAezeed => true,
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
            Action::LAezeed,
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
