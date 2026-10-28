//! A wallet of several keys registered from the spend that needs it.
//!
//! An input spent from a script of several keys is signed only for a
//! registered wallet (`faraday_psbt::WarningKind::WalletNotRegistered`).
//! Where the PSBT carries every cosigner's extended key, its inspection
//! holds the wallet it defines (`faraday_psbt::Registration`), and the
//! Spend review offers **Register this wallet**: a dialog with the
//! wallet's shape, its cosigners and which of them is yours, a name, and
//! a hold. The hold checks the descriptor against the spend's scripts
//! (`faraday_psbt::register`) and adds the wallet as a wallet file is
//! added; this module holds the sheet's state and draws it, and
//! `wallet.rs` holds its actions.

use faraday_psbt::{RegisterRefusal, Registration, Wrapper};
use faraday_shell_api::Key as KeyIn;

use crate::Action;
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wallet::fp_text;
use faraday_draw::tokens as T;
use faraday_draw::tokens::{BODY_M, BODY_S, FIELD_H, FIELD_PAD, LABEL_M};
use faraday_draw::widgets::Icon;

/// The register sheet while it is open: the name, whether typing goes to
/// it, and why the last hold registered nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterAsk {
    /// The wallet's name as typed; the PSBT's file name at first.
    pub name: String,
    /// The name field has focus.
    pub typing: bool,
    /// Why the last hold registered nothing.
    pub refused: Option<String>,
    /// The reading chosen, by its place in the offer's readings: the
    /// first, `sortedmulti`, until the person picks the other.
    pub reading: usize,
}

impl RegisterAsk {
    /// The sheet as it opens for the PSBT named `file`.
    pub(crate) fn new(file: &str) -> Self {
        Self {
            name: name_of_file(file),
            typing: false,
            refused: None,
            reading: 0,
        }
    }

    /// A key typed while the name field has focus. Whether it took it.
    pub(crate) fn key(&mut self, key: KeyIn) -> bool {
        if !self.typing {
            return false;
        }
        match key {
            KeyIn::Char(c) if !c.is_control() => self.name.push(c),
            KeyIn::Backspace => {
                self.name.pop();
            }
            // Enter, or the keyboard's Done, ends the typing: the hold
            // is what registers.
            KeyIn::Enter => {
                self.typing = false;
                return true;
            }
            _ => return false,
        }
        self.refused = None;
        true
    }

    /// The name the wallet is added under: what is typed, trimmed, or
    /// `fallback` when nothing is.
    pub(crate) fn name_or(&self, fallback: &str) -> String {
        let typed = self.name.trim();
        if typed.is_empty() {
            fallback.to_string()
        } else {
            typed.to_string()
        }
    }
}

/// A wallet's name from a file's: its stem less `-wallet`, its words
/// split at `-` and `_` and each begun in capitals (`savings-unsigned.psbt`
/// is "Savings Unsigned").
pub(crate) fn name_of_file(file: &str) -> String {
    let stem = crate::stem(file);
    stem.strip_suffix("-wallet")
        .unwrap_or(&stem)
        .split(['-', '_'])
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Why a hold registered nothing, as the sheet says it.
pub(crate) fn refusal_text(r: RegisterRefusal, offer: &Registration) -> String {
    match r {
        RegisterRefusal::Descriptor => EN.register_refused_descriptor.to_string(),
        RegisterRefusal::Input(i) => fill(EN.register_refused_input, &[&i.to_string()]),
        RegisterRefusal::Output(i) => fill(EN.register_refused_output, &[&i.to_string()]),
        RegisterRefusal::NotYours => EN.register_refused_not_yours.to_string(),
        RegisterRefusal::NotYourKey(i) => offer.cosigners.get(i).map_or_else(
            || EN.register_refused_not_yours.to_string(),
            |k| {
                fill(
                    EN.register_refused_not_your_key,
                    &[&fp_text(k.fingerprint), &path_text(&k.path)],
                )
            },
        ),
    }
}

/// "2 of 3, P2WSH, sorted": the wallet's threshold, its count of keys,
/// its wrapper and whether its keys are sorted in reading `reading`.
pub(crate) fn shape_line(r: &Registration, reading: usize) -> String {
    let wrapper = match r.wrapper {
        Wrapper::P2sh => EN.register_p2sh,
        Wrapper::P2wsh => EN.register_p2wsh,
        Wrapper::P2shP2wsh => EN.register_p2sh_p2wsh,
    };
    let order = if r.readings.get(reading).is_none_or(|x| x.sorted) {
        EN.register_sorted
    } else {
        EN.register_in_order
    };
    fill(
        EN.register_shape,
        &[
            &r.threshold.to_string(),
            &r.cosigners.len().to_string(),
            wrapper,
            order,
        ],
    )
}

/// A path as a descriptor writes it, hardened steps with `h`, no `m/`.
fn path_text(path: &faraday_psbt::bitcoin::bip32::DerivationPath) -> String {
    path.into_iter()
        .map(|c| match c {
            faraday_psbt::bitcoin::bip32::ChildNumber::Hardened { index } => format!("{index}h"),
            faraday_psbt::bitcoin::bip32::ChildNumber::Normal { index } => index.to_string(),
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// The register sheet (`docs/FLOWS.md`, Spend): DESIGN §4.13's dialog
/// with the wallet's shape (left out over the small panel's keyboard, so
/// the field shows above it), the name field, the refusal when the last hold found one, a
/// line per cosigner (fingerprint and path in the mono face, "Yours"
/// where a loaded key is that cosigner), and DESIGN §4.13's hold; Cancel
/// is the dialog's action. Reads the offer the inspection
/// keeps and the sheet's state; works nothing out.
pub(crate) fn sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(s) = app.wallets.spend() else {
        return;
    };
    let (Some(offer), Some(ask)) = (s.inspection.register.as_ref(), s.register.as_ref()) else {
        return;
    };
    let shape = shape_line(offer, ask.reading);
    crate::compact::dialog(
        ui,
        (w, h, T::DIALOG_MAX),
        (Icon::Keys.into(), Role::Primary.paint(), EN.register_head),
        &mut |ui, x, y, iw| {
            let mut cy = y;
            // Over the small panel's keyboard the field comes first.
            if !(ui.compact && ask.typing) {
                cy += ui.wrap_as((x, cy, iw), BODY_M, Role::OnSurface, &shape) + T::GAP_SMALL;
                // The spend cannot tell the two readings apart: the
                // person chooses, sorted first.
                if offer.readings.len() > 1 {
                    cy += ui.wrap_as(
                        (x, cy, iw),
                        BODY_S,
                        Role::OnSurfaceVariant,
                        EN.register_reading_line,
                    ) + T::GAP_SMALL;
                    let bw = (iw - T::GAP_SMALL) / 2.0;
                    for (k, label) in [EN.register_reading_sorted, EN.register_reading_in_order]
                        .into_iter()
                        .enumerate()
                    {
                        let style = if ask.reading == k {
                            Style::Selected
                        } else {
                            Style::Secondary
                        };
                        let at = x + (bw + T::GAP_SMALL) * k as f32;
                        let pick = Action::RegisterReading(u8::try_from(k).unwrap_or(0));
                        ui.button(at, cy, Some(bw), T::BUTTON_H, label, style, pick);
                    }
                    cy += T::BUTTON_H + T::GAP_SMALL;
                }
            }
            cy += ui.wrap_as(
                (x, cy, iw),
                LABEL_M,
                Role::OnSurfaceVariant,
                EN.register_name,
            ) + T::GAP_SMALL;
            ui.field_box((x, cy, iw), ask.typing, None);
            ui.typed(
                x + FIELD_PAD,
                cy,
                FIELD_H,
                BODY_M.size,
                W::from(BODY_M.face),
                Role::OnSurface,
                &ask.name,
                iw - 2.0 * FIELD_PAD,
                ask.typing,
                Action::RegisterField,
            );
            ui.hit(x, cy, iw, FIELD_H, Action::RegisterField);
            cy += FIELD_H + T::GAP_SMALL;
            if let Some(why) = &ask.refused {
                cy += ui.wrap_as((x, cy, iw), BODY_M, Role::Error, why) + T::GAP_SMALL;
            }
            cy += T::GAP_SMALL;
            for k in &offer.cosigners {
                let line = fill(
                    if k.ours {
                        EN.register_cosigner_yours
                    } else {
                        EN.register_cosigner
                    },
                    &[&fp_text(k.fingerprint), &path_text(&k.path)],
                );
                let tone = if k.ours {
                    Role::Primary
                } else {
                    Role::OnSurfaceVariant
                };
                cy += ui.wrap_as((x, cy, iw), BODY_M.mono(), tone, &line);
            }
            cy += T::GAP;
            crate::vault::screen::hold_button_in(
                app,
                ui,
                x,
                cy,
                iw,
                EN.register_hold,
                Action::RegisterHold,
                faraday_ui::hold::HoldTone::Primary,
            );
            cy += T::CHIP_LINE + T::GAP_SMALL;
            cy - y
        },
        &[(EN.register_cancel, Style::Secondary, Action::RegisterCancel)],
    );
}

/// One entry of the session's kept extended public keys: the network,
/// the key's `master_id`, the path, and the key there.
pub(crate) type KeyXpub = (
    faraday_bip::keys::Network,
    [u8; 32],
    faraday_psbt::bitcoin::bip32::DerivationPath,
    faraday_psbt::bitcoin::bip32::Xpub,
);

/// Costs a key derivation of at most `MAX_PATH_STEPS` steps per path a
/// wallet names under a loaded fingerprint that no account of the key
/// is at, the first time it is named; kept after on the session
/// (`Session::key_xpubs`). Adds to each of `refs` the extended
/// public keys its key gives at its wallets' paths, once per path
/// however many wallets name it: what says a wallet holds it and which
/// of its keys are the person's (`faraday_psbt::Context::holds`).
pub(crate) fn wallet_xpubs(
    session: &crate::wallet::Session,
    c: &faraday_crypto::work::Compute,
    refs: &mut [faraday_psbt::KeyRef],
) {
    let network = session.network();
    for r in refs {
        let Some(i) = session.keys.position(r.fingerprint) else {
            continue;
        };
        let Some(id) = session.keys.find(r.fingerprint).map(|k| k.id.0) else {
            continue;
        };
        for key in session.wallets.iter().flat_map(|w| w.policy().keys()) {
            let Some(path) = key.path() else { continue };
            if key.fingerprint() != Some(r.fingerprint)
                || r.xpub_at(path).is_some()
                || r.extra.iter().any(|(p, _)| p == path)
            {
                continue;
            }
            let kept = session
                .key_xpubs
                .borrow()
                .iter()
                .find(|(n, k, p, _)| *n == network && *k == id && p == path)
                .map(|(.., x)| *x);
            let xpub = kept.or_else(|| {
                let x = session.keys.xpub_at(c, i, path)?;
                session
                    .key_xpubs
                    .borrow_mut()
                    .push((network, id, path.clone(), x));
                Some(x)
            });
            if let Some(xpub) = xpub {
                r.extra.push((path.clone(), xpub));
            }
        }
    }
}
