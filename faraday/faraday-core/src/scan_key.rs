//! Scan a key (`docs/SCAN-TO-SIGN.md` §2): one way in for any key a
//! loaded wallet is short of. The camera takes any of the wallet's
//! missing keys and Faraday works out which; a seed whose key with no
//! passphrase is one of them loads at once, since the match proves it
//! has none, and one that matches nothing goes to the passphrase sheet
//! (`seeds::read`), which takes it only for a missing key of the wallet.

use crate::strings::EN;
use crate::strings::fill;
use faraday_bip::keys::Fingerprint;
use faraday_crypto::work::Compute;
use faraday_shell_api::Command;

use crate::plan::{At, What};
use crate::wallet::{Session, fp_text};
use crate::{Action, Faraday, ScanPurpose, ScanState, Sheet};

/// Costs what `add_words_with` costs. The key `words` make with `pass`,
/// if they are BIP-39 words.
pub(crate) fn key_of(c: &Compute, words: &str, pass: &str) -> Option<Fingerprint> {
    Session::default()
        .add_words_with(c, words, pass, "", None)
        .ok()
}

/// What a read or a choice found of a wallet's keys.
pub(crate) enum Found {
    /// Key n of the wallet, not here.
    Missing(usize),
    /// Key n of the wallet, here already.
    Here(usize),
    /// No key of the wallet.
    None,
}

impl Faraday {
    /// Whether loaded wallet `w` has enough keys here to sign.
    pub fn can_spend(&self, w: usize) -> bool {
        self.session
            .wallets
            .get(w)
            .is_some_and(|wl| crate::screens::sign_readiness(self, wl) == 2)
    }

    /// Keys loaded wallet `w` still needs here before it can spend.
    pub(crate) fn keys_short(&self, w: usize) -> usize {
        let Some(wl) = self.session.wallets.get(w) else {
            return 0;
        };
        let here = self
            .session
            .slots(wl)
            .iter()
            .filter(|s| s.held_by.is_some())
            .count();
        crate::wallet::needed(wl).saturating_sub(here)
    }

    /// Which of wallet `w`'s keys `fp` is.
    pub(crate) fn wallet_key_found(&self, w: usize, fp: Fingerprint) -> Found {
        let Some(wl) = self.session.wallets.get(w) else {
            return Found::None;
        };
        let slots = self.session.slots(wl);
        match slots.iter().position(|s| s.fingerprint == Some(fp)) {
            Some(n) if slots[n].held_by.is_some() => Found::Here(n),
            Some(n) => Found::Missing(n),
            None => Found::None,
        }
    }

    /// Costs what `plan_notes` costs. The name a key of wallet `w` loads
    /// under: who holds it, as the backup's plan keeps it, else none.
    pub(crate) fn wallet_key_label(&self, c: &Compute, w: usize, fp: Fingerprint) -> String {
        let Some(wl) = self.session.wallets.get(w) else {
            return String::new();
        };
        self.plan_notes(c, wl, None)
            .into_iter()
            .find_map(|n| match n {
                crate::plan::Note::Holder(f, name) if f == fp.0 => Some(name),
                _ => None,
            })
            .unwrap_or_default()
    }

    /// The line a seed that is no key of wallet `w` is refused with.
    pub(crate) fn not_a_wallet_key(&self, w: usize, fp: Fingerprint) -> String {
        let name = self
            .session
            .wallets
            .get(w)
            .map(|wl| wl.name.as_str())
            .unwrap_or_default();
        fill(EN.scan_key_is_not_one, &[&fp_text(fp).to_string(), name])
    }

    /// Costs what `of_now` costs. The keys the key scanner for wallet `w`
    /// takes, a line each: "Key 2 · 1A2B3C4D · Place 2", the place where
    /// the backup's plan keeps that seed.
    pub(crate) fn wallet_key_lines(&self, c: &Compute, w: usize) -> Vec<String> {
        let Some(wl) = self.session.wallets.get(w) else {
            return Vec::new();
        };
        let glance = crate::glance::of_now(c, self, w);
        let place = |n: usize| -> Option<String> {
            let g = glance.as_ref()?;
            let seed = g.keys.get(n)?.seed?;
            let crate::glance::Backup::Plan { nodes, .. } = &g.backup else {
                return None;
            };
            nodes
                .iter()
                .filter(|b| matches!(b.at, At::Place(_)))
                .find(|b| {
                    b.whats.iter().any(|x| {
                        matches!(*x, What::Words(i) | What::SeedQr(i) | What::Seed(i)
                            | What::SeedPassphrase(i) | What::SeedFile(i)
                            | What::SeedQrFile(i) if i == seed)
                    })
                })
                .map(|b| b.name.clone())
        };
        self.session
            .slots(wl)
            .iter()
            .enumerate()
            .filter(|(_, s)| s.held_by.is_none())
            .filter_map(|(n, s)| {
                let fp = s.fingerprint?;
                let line = fill(
                    EN.scan_key_key,
                    &[&(n + 1).to_string(), &fp_text(fp).to_string()],
                );
                Some(match place(n) {
                    Some(p) => format!("{line} · {p}"),
                    None => line,
                })
            })
            .collect()
    }

    /// A seed's words read by wallet `w`'s key scanner.
    pub(crate) fn wallet_key_scanned(&mut self, w: usize, words: &str) {
        let c = self.compute();
        let Some(fp) = key_of(&c, words, "") else {
            if let Some(scan) = self.scan.as_mut() {
                scan.note = Some(EN.scan_key_not_a_seedqr.to_string());
            }
            return;
        };
        match self.wallet_key_found(w, fp) {
            Found::Here(n) => {
                if let Some(scan) = self.scan.as_mut() {
                    scan.note = Some(fill(EN.scan_key_key_already, &[&(n + 1).to_string()]));
                }
            }
            Found::Missing(n) => {
                let label = self.wallet_key_label(&c, w, fp);
                match self.session.add_words_with(&c, words, "", &label, None) {
                    Ok(_) => self.wallet_key_added(w, n),
                    Err(e) => {
                        if let Some(scan) = self.scan.as_mut() {
                            scan.note = Some(e.text());
                        }
                    }
                }
            }
            // Perhaps a seed with a passphrase: the sheet asks for it.
            Found::None => {
                let reason = self.not_a_wallet_key(w, fp);
                self.seed_read_for_wallet(words, w, reason);
            }
        }
    }

    /// Key n of wallet `w` loaded from the key scanner or the passphrase
    /// sheet: the spend and the Spend tab take it, and the camera says
    /// how many more, or closes once the wallet can spend.
    pub(crate) fn wallet_key_added(&mut self, w: usize, n: usize) {
        self.refresh_spend();
        // Scanned from Add a key: back where Add a key was opened from.
        if self.screen == crate::Screen::Entry {
            let back = self.entry.back;
            self.entry = crate::EntryState::default();
            self.screen = match back {
                Some(s) => s,
                None if self.spend.is_some() => crate::Screen::Spend,
                None => crate::Screen::Wallets,
            };
        }
        if self.screen == crate::Screen::Family {
            self.family_key_added();
        }
        self.wallet_key_said(w, n);
    }

    /// What loading key n of wallet `w` says: with the camera on and the
    /// wallet still short, the camera's note; else a toast, the camera
    /// closed.
    pub(crate) fn wallet_key_said(&mut self, w: usize, n: usize) {
        let short = self.keys_short(w);
        if short > 0
            && let Some(scan) = self.scan.as_mut()
        {
            scan.note = Some(fill(
                EN.scan_key_key_added_more,
                &[&(n + 1).to_string(), &short.to_string()],
            ));
            return;
        }
        if self.scan.take().is_some() {
            self.commands.push_back(Command::CameraOff);
            if self.sheet == Some(Sheet::Scan) {
                self.sheet = None;
            }
        }
        if short > 0 {
            self.toast(&fill(
                EN.scan_key_key_added_more,
                &[&(n + 1).to_string(), &short.to_string()],
            ));
        } else {
            self.toast(&fill(EN.scan_key_key_added, &[&(n + 1).to_string()]));
        }
    }
}

impl Faraday {
    /// Handles the scan sheet's actions (`ScanPaste`, `ScanCamera`, `ScanPart`, `ScanSeed`, `ScanKey`, `Scan`).
    pub(crate) fn scan_act(&mut self, action: Action) {
        match action {
            Action::ScanPaste => self.scan_paste(),
            Action::ScanCamera(i) => {
                if let Some((id, _)) = self.cameras.get(usize::from(i)) {
                    self.camera = Some(id.clone());
                    self.camera_change = Some(id.clone());
                    if let Some(s) = self.scan.as_mut() {
                        s.seen = None;
                        s.too_fine = 0;
                    }
                }
            }
            Action::ScanPart => {
                if self.may_load_keys() {
                    let mut scan = ScanState::default();
                    scan.purpose = ScanPurpose::KeyPart;
                    self.scan = Some(scan);
                    self.sheet = Some(Sheet::Scan);
                    self.commands.push_back(Command::CameraOn);
                }
            }
            Action::ScanSeed => {
                if self.may_load_keys() {
                    let mut scan = ScanState::default();
                    scan.purpose = ScanPurpose::Seed;
                    self.scan = Some(scan);
                    self.sheet = Some(Sheet::Scan);
                    self.commands.push_back(Command::CameraOn);
                }
            }
            Action::ScanKey(w) => {
                if self.may_load_keys() && w < self.session.wallets.len() {
                    let mut scan = ScanState::default();
                    scan.purpose = ScanPurpose::WalletKey(w);
                    self.scan = Some(scan);
                    self.sheet = Some(Sheet::Scan);
                    self.commands.push_back(Command::CameraOn);
                }
            }
            Action::Scan => {
                let mut scan = ScanState::default();
                if let Some((decoder, assembler)) = self.scan_parts.take() {
                    scan.decoder = decoder;
                    scan.assembler = assembler;
                    scan.note = Some(EN.act_carrying_on_with.to_string());
                }
                self.scan = Some(scan);
                self.sheet = Some(Sheet::Scan);
                self.commands.push_back(Command::CameraOn);
            }
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }
}
