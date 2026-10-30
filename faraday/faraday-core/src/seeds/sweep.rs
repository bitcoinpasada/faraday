//! Sweep a private key (`docs/INTEROP.md` §5 I11, F171): a WIF scanned
//! on Home's Scan or the sheet's own scanner, or typed on the sheet,
//! never loaded as a key and never kept.
//!
//! The sheet names the key by its legacy address and shows the addresses
//! it pays (legacy, then native and nested SegWit for a compressed key),
//! each with its QR code, for an online wallet to watch. That wallet
//! builds the sweep's PSBT; the sheet's scanner reads it, and the review
//! shows what this key's inputs hold, the inputs it does not sign, and
//! each output with the loaded wallet whose address it is, or flagged
//! as none of theirs (their first hundred addresses on each chain, as
//! Check an address searches). A hold signs the key's inputs
//! (`faraday_keys::Keys::sweep_sign`); the signed PSBT, and the finished
//! transaction when every input is signed, go to the Outbox.
//!
//! The key lives in the session's key store (`Keys::sweep`) while the
//! sheet, or its scanner, is open: Done, a lock or leaving the sheet
//! erases it ([`crate::Faraday::sweep_tidy`]). What the sheet shows of
//! the PSBT is worked out once, when the PSBT is read, and kept here
//! ([`Review`]); drawing reads it.

use faraday_bip::keys::ScriptType;
use faraday_crypto::work::Compute;
use faraday_draw::tokens as T;
use faraday_draw::tokens::{BODY_M, BODY_S, FIELD_H, FIELD_PAD, LABEL_L, LABEL_M};
use faraday_draw::widgets::Icon;
use faraday_psbt::{Level, Psbt, WarningKind};

use crate::Action;
use crate::drawing::View;
use crate::secret_text::SecretText;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::wallet::Session;

/// The sheet's presses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepAction {
    /// The Tools tile: the sheet, with nothing read.
    Open,
    /// Typing goes to the key's field.
    Field,
    /// The key typed is read.
    Use,
    /// The camera: for the key, or once it is read for the PSBT.
    Scan,
    /// The sheet's scanner closed: back to the sheet.
    Back,
    /// The hold that signs.
    Hold,
    /// What was signed, as a QR code.
    ShowQr,
    /// The sheet closes and the key is erased.
    Done,
}

/// The sheet while it is open. The key itself is the store's
/// (`Keys::sweep`).
#[derive(Default)]
pub struct Sweep {
    /// The WIF typed, until **Use this key** reads it; wiped then.
    pub typed: SecretText,
    /// Typing goes to the field.
    pub typing: bool,
    /// Why the last key or PSBT was not taken.
    pub error: Option<String>,
    /// The PSBT read, and what the review shows of it.
    pub review: Option<Review>,
    /// What the hold signed.
    pub signed: Option<Signed>,
}

/// One output, as the review shows it.
pub struct Out {
    /// Its address, or its script named where it has none.
    pub address: String,
    /// Its amount, in bitcoin.
    pub amount: String,
    /// The loaded wallet whose address it is, with the chain and index;
    /// `None` flags it.
    pub wallet: Option<String>,
}

/// The PSBT the sweep signs, and what its review shows: worked out once
/// when it was read ([`review`]).
pub struct Review {
    /// The PSBT as read, in its version 0 form, which is what is signed.
    pub psbt: Psbt,
    /// The version 2 PSBT as read, when it came as one (BIP-370,
    /// `docs/INTEROP.md` §5 I10): the signed PSBT goes back out as it
    /// ([`out_bytes`]).
    pub v2: Option<faraday_psbt::v2::PsbtV2>,
    /// The line on the key's inputs: how many, and their total.
    pub from: String,
    /// Each of the key's inputs: the address it spends and its amount.
    pub inputs: Vec<(String, String)>,
    /// Inputs that do not spend the key: left unsigned.
    pub others: usize,
    /// The outputs.
    pub outputs: Vec<Out>,
    /// The fee, in bitcoin.
    pub fee: String,
    /// The warnings a danger or a block raised, in words.
    pub warnings: Vec<(Level, String)>,
    /// Why nothing signs, when something stops it.
    pub stop: Option<String>,
    /// The holdable dangers the hold signs past (F136): shown above it.
    pub accepted: Vec<WarningKind>,
}

/// What the hold signed.
pub struct Signed {
    /// What it did, in words.
    pub line: String,
    /// The signed PSBT.
    pub psbt: Vec<u8>,
    /// The finished transaction in hex, when every input is signed.
    pub hex: Option<String>,
}

/// The signed PSBT `signed` as it goes out, in the version it came in:
/// a version 2 PSBT as read (`v2`) with only the signing fields the
/// sweep added (`PsbtV2::merge_signed`), as `Spend::out_bytes` writes a
/// spend's. A version 2 PSBT is never answered with a version 0 one: where
/// the signatures do not go back into it, this refuses and the caller
/// writes nothing. Costs a PSBT's serialization.
pub fn out_bytes(
    v2: Option<&faraday_psbt::v2::PsbtV2>,
    signed: &Psbt,
) -> Result<Vec<u8>, faraday_psbt::v2::Error> {
    let Some(v2) = v2 else {
        return Ok(signed.to_bytes());
    };
    let mut out = v2.clone();
    out.merge_signed(signed)?;
    Ok(out.to_bytes())
}

/// An address's script type, in the sheet's words.
fn kind_text(t: ScriptType) -> &'static str {
    match t {
        ScriptType::Legacy => EN.sweep_legacy,
        ScriptType::NestedSegwit => EN.sweep_nested,
        ScriptType::NativeSegwit | ScriptType::Taproot => EN.sweep_native,
    }
}

/// The address the output input `i` of `psbt` spends pays, on `network`;
/// empty where it has none. A lookup and an address encoding.
fn spent_address(psbt: &Psbt, i: usize, network: faraday_bip::keys::Network) -> String {
    let inner = psbt.inner();
    let out = inner.inputs.get(i).and_then(|input| {
        input.witness_utxo.clone().or_else(|| {
            let vout = inner.unsigned_tx.input.get(i)?.previous_output.vout as usize;
            input.non_witness_utxo.as_ref()?.output.get(vout).cloned()
        })
    });
    out.and_then(|o| {
        faraday_bip::bitcoin::Address::from_script(
            &o.script_pubkey,
            faraday_bip::bitcoin::Network::from(network),
        )
        .ok()
    })
    .map(|a| a.to_string())
    .unwrap_or_default()
}

/// Costs an address derivation per address tried, up to two hundred per
/// wallet. The loaded wallet whose address `address` is, among each
/// wallet's first hundred on each chain, in words.
fn whose(c: &Compute, session: &Session, address: &str) -> Option<String> {
    let net = session.network();
    let a = address
        .parse::<faraday_bip::bitcoin::Address<faraday_bip::bitcoin::address::NetworkUnchecked>>()
        .ok()?
        .require_network(net.into())
        .ok()?;
    session.wallets.iter().find_map(|w| {
        let (change, index) = w.policy().find_address(c, net, &a, 100)?;
        let chain = if change {
            EN.address_note_change
        } else {
            EN.address_note_receive
        };
        Some(fill(
            EN.sweep_to_wallet,
            &[&w.name, chain, &index.to_string()],
        ))
    })
}

/// Costs what `inspect` costs and what `whose` costs per output. What
/// the review shows of `psbt` for the key the session sweeps: its
/// inputs, the others, each output's wallet, the fee, and what stops a
/// signature: a block, a danger the hold does not sign past, or no
/// input of the key.
pub(crate) fn review(
    c: &Compute,
    session: &Session,
    (psbt, v2): (Psbt, Option<faraday_psbt::v2::PsbtV2>),
) -> Review {
    let net = session.network();
    let inspection = session.inspect(c, &psbt);
    let mine = session
        .keys
        .sweep()
        .map(|k| k.inputs_of(&psbt))
        .unwrap_or_default();
    let mut total = 0u64;
    let inputs: Vec<(String, String)> = mine
        .iter()
        .map(|&i| {
            let sats = inspection
                .inputs
                .get(i)
                .and_then(|x| x.amount)
                .map_or(0, |a| a.to_sat());
            total = total.saturating_add(sats);
            (spent_address(&psbt, i, net), crate::decode::btc(sats))
        })
        .collect();
    let others = inspection
        .inputs
        .iter()
        .filter(|x| !x.finalized && !mine.contains(&x.index))
        .count();
    let outputs = inspection
        .outputs
        .iter()
        .map(|o| Out {
            address: o.address.clone(),
            amount: crate::decode::btc(o.amount.to_sat()),
            wallet: whose(c, session, &o.address),
        })
        .collect();
    let shown = |w: &&faraday_psbt::Warning| {
        matches!(w.level, Level::Danger | Level::Blocked)
            && w.kind != WarningKind::NoParticipatingKey
    };
    let warnings: Vec<(Level, String)> = inspection
        .warnings
        .iter()
        .filter(shown)
        .map(|w| {
            (
                w.level,
                crate::review::warning_text(&inspection, w).into_owned(),
            )
        })
        .collect();
    let accepted = inspection.holdable();
    let unheld = inspection
        .warnings
        .iter()
        .filter(shown)
        .any(|w| w.level == Level::Blocked || !w.kind.is_holdable());
    let stop = if mine.is_empty() {
        Some(EN.sweep_no_inputs.to_string())
    } else if unheld {
        warnings.first().map(|(_, t)| t.clone())
    } else {
        None
    };
    Review {
        from: fill(
            EN.sweep_from,
            &[&mine.len().to_string(), &crate::decode::btc(total)],
        ),
        inputs,
        others,
        outputs,
        fee: fill(
            EN.sweep_fee,
            &[&crate::decode::btc(inspection.fee.to_sat())],
        ),
        warnings,
        stop,
        accepted,
        psbt,
        v2,
    }
}

/// Why a WIF is not taken, in the sheet's words.
pub(crate) fn refusal_text(
    r: faraday_keys::sweep::Refusal,
    network: faraday_bip::keys::Network,
    medium: crate::Medium,
) -> String {
    use faraday_keys::sweep::Refusal as R;
    match r {
        R::NotAKey => EN.wif_not_a_key.to_string(),
        R::Network { mainnet: true } => fill(
            EN.sweep_mainnet_key,
            &[faraday_intake::wallet::network_name(network)],
        ),
        R::Network { mainnet: false } => EN.sweep_test_key.to_string(),
        R::StickAttached => crate::stick::keys_load(medium),
    }
}

/// The field the key is typed into: a dot for each character, never the
/// key. Returns its height.
fn key_field(ui: &mut Ui, (x, y, w): (f32, f32, f32), s: &Sweep) -> f32 {
    let mut cy = y;
    cy += ui.wrap_as((x, cy, w), LABEL_M, Role::OnSurfaceVariant, EN.sweep_field) + T::GAP_SMALL;
    ui.field_box((x, cy, w), s.typing, None);
    let dots = "•".repeat(s.typed.as_str().chars().count());
    let field = Action::Sweep(SweepAction::Field);
    ui.typed(
        x + FIELD_PAD,
        cy,
        FIELD_H,
        BODY_M.size,
        W::from(BODY_M.face),
        Role::OnSurface,
        &dots,
        w - 2.0 * FIELD_PAD,
        s.typing,
        field,
    );
    ui.press_box(x, cy, w, FIELD_H, field);
    cy += FIELD_H + T::GAP_SMALL;
    cy - y
}

/// The key's addresses, each with its type and its QR code. Returns the
/// height used.
fn addresses(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let Some(k) = app.session.keys.sweep() else {
        return 0.0;
    };
    let mut cy = y;
    let side = T::SWEEP_QR.min(w);
    for (kind, address) in k.addresses() {
        cy += ui.wrap_as((x, cy, w), LABEL_L, Role::OnSurface, kind_text(*kind)) + T::GAP_SMALL;
        if let Some(m) = app.public_qr(address) {
            faraday_ui::parts::qr_code(ui, &m, x, cy, side);
            cy += side + T::GAP_SMALL;
        }
        cy += ui.wrap_as((x, cy, w), BODY_S.mono(), Role::OnSurfaceVariant, address) + T::GAP;
    }
    cy - y
}

/// The review of the PSBT read: the key's inputs, the others, the
/// outputs with their wallets, the fee, the warnings, and the hold or
/// what stops it. Returns the height used.
fn review_body(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32), r: &Review) -> f32 {
    let mut cy = y;
    cy += ui.wrap_as((x, cy, w), LABEL_L, Role::OnSurface, &r.from) + T::GAP_SMALL;
    for (address, amount) in &r.inputs {
        cy += ui.wrap_as((x, cy, w), BODY_S.mono(), Role::OnSurfaceVariant, address);
        cy += ui.wrap_as((x, cy, w), BODY_M, Role::OnSurface, amount) + T::GAP_SMALL;
    }
    if r.others > 0 {
        let line = fill(EN.sweep_others, &[&r.others.to_string()]);
        cy += ui.wrap_as((x, cy, w), BODY_M, Role::Warning, &line) + T::GAP_SMALL;
    }
    cy += T::GAP_SMALL;
    cy += ui.wrap_as((x, cy, w), LABEL_L, Role::OnSurface, EN.sweep_to) + T::GAP_SMALL;
    for o in &r.outputs {
        cy += ui.wrap_as(
            (x, cy, w),
            BODY_S.mono(),
            Role::OnSurfaceVariant,
            &o.address,
        );
        cy += ui.wrap_as((x, cy, w), BODY_M, Role::OnSurface, &o.amount);
        cy += match &o.wallet {
            Some(name) => ui.wrap_as((x, cy, w), BODY_M, Role::Primary, name),
            None => ui.wrap_as((x, cy, w), BODY_M, Role::Error, EN.sweep_to_other),
        } + T::GAP_SMALL;
    }
    cy += ui.wrap_as((x, cy, w), BODY_M, Role::OnSurface, &r.fee) + T::GAP_SMALL;
    for (level, text) in &r.warnings {
        let tone = if *level == Level::Blocked {
            Role::Error
        } else {
            Role::Warning
        };
        cy += ui.wrap_as((x, cy, w), BODY_M, tone, text) + T::GAP_SMALL;
    }
    if let Some(why) = &r.stop {
        cy += ui.wrap_as((x, cy, w), LABEL_L, Role::Error, why) + T::GAP_SMALL;
    } else {
        cy += T::GAP_SMALL;
        crate::vault::screen::hold_button_in(
            app,
            ui,
            x,
            cy,
            w,
            EN.sweep_hold,
            Action::Sweep(SweepAction::Hold),
            faraday_ui::hold::HoldTone::Primary,
        );
        cy += T::CHIP_LINE + T::GAP_SMALL;
    }
    cy - y
}

/// The sheet (Material's dialog, as Faraday's other sheets): with no key
/// read, the field and Scan; with one, the key's addresses and Scan the
/// PSBT; with a PSBT, its review and the hold; once signed, what was
/// signed and Show as QR. Done closes it and erases the key.
pub(crate) fn sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(s) = app.seed_read.sweep() else {
        return;
    };
    let key = app.session.keys.sweep();
    let done = (
        EN.sweep_done,
        Style::Ghost,
        Action::Sweep(SweepAction::Done),
    );
    let actions: Vec<(&str, Style, Action)> = match (key, &s.review, &s.signed) {
        (None, ..) => vec![
            done,
            (
                EN.sweep_scan,
                Style::Secondary,
                Action::Sweep(SweepAction::Scan),
            ),
            (
                EN.sweep_use,
                Style::Primary.when(!s.typed.is_empty()),
                Action::Sweep(SweepAction::Use),
            ),
        ],
        (Some(_), _, Some(_)) => vec![
            done,
            (
                EN.sweep_show_qr,
                Style::Primary,
                Action::Sweep(SweepAction::ShowQr),
            ),
        ],
        (Some(_), None, None) => vec![
            done,
            (
                EN.sweep_scan_psbt,
                Style::Primary,
                Action::Sweep(SweepAction::Scan),
            ),
        ],
        (Some(_), Some(_), None) => vec![
            done,
            (
                EN.sweep_scan_psbt,
                Style::Secondary,
                Action::Sweep(SweepAction::Scan),
            ),
        ],
    };
    let body = |ui: &mut Ui, x: f32, y: f32, iw: f32| -> f32 {
        let mut cy = y;
        match key {
            None => {
                cy += key_field(ui, (x, cy, iw), s);
                cy += crate::screens::never_real_seed(app, ui, x, cy, iw);
            }
            Some(k) => {
                let name = fill(EN.sweep_key_of, &[k.name()]);
                cy += ui.wrap_as((x, cy, iw), BODY_M.mono(), Role::OnSurface, &name);
                cy += ui.wrap_as(
                    (x, cy, iw),
                    BODY_S,
                    Role::OnSurfaceVariant,
                    EN.sweep_not_kept,
                ) + T::GAP_SMALL;
                if !k.compressed() {
                    cy += ui.wrap_as((x, cy, iw), BODY_S, Role::Warning, EN.wif_uncompressed)
                        + T::GAP_SMALL;
                }
                cy += T::GAP_SMALL;
                match (&s.signed, &s.review) {
                    (Some(done), _) => {
                        cy += ui.wrap_as((x, cy, iw), LABEL_L, Role::Primary, &done.line)
                            + T::GAP_SMALL;
                    }
                    (None, Some(r)) => cy += review_body(app, ui, (x, cy, iw), r),
                    (None, None) => cy += addresses(app, ui, (x, cy, iw)),
                }
            }
        }
        if let Some(e) = &s.error {
            cy += ui.wrap_as((x, cy, iw), LABEL_L, Role::Error, e) + T::GAP_SMALL;
        }
        cy - y
    };
    let head = (
        Icon::Keys.into(),
        Role::Primary.paint(),
        EN.catalog_sweep_key,
    );
    crate::compact::dialog(
        ui,
        (w, h, T::DIALOG_MAX),
        head,
        &mut |ui, x, y, iw| body(ui, x, y, iw),
        &actions,
    );
}
