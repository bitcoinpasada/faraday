//! Decode a Lightning invoice: a BOLT #11 invoice, typed in, pasted or
//! scanned, taken apart so the person can see what it asks for: the
//! network, the amount, what it is for, who is paid and by which payment
//! hash, when it was made and when it ends, and the rest of its fields.
//!
//! The reading is `faraday_codec::bolt11::decode`'s, which refuses what
//! BOLT #11 says to refuse. This module only arranges what it read, once,
//! when the text is decoded; drawing makes nothing. Read only: nothing is
//! paid, signed or stored, and leaving the screen drops the text.

pub(crate) mod screen;

use crate::strings::{EN, fill};
use crate::{Action, Faraday, Screen};
use faraday_codec::bolt11::{self, Error, Fallback, Invoice};

/// The most characters the field takes: more than any invoice is read at
/// (`bolt11::MAX_LEN`), so a text that is too long is refused by name.
const FIELD_MAX: usize = bolt11::MAX_LEN + 64;

/// A press on the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvoicePress {
    /// Decode the text in the field.
    Decode,
    /// Scan an invoice into the field.
    Scan,
    /// Empty the field.
    Clear,
}

/// One line of a decoded invoice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Its name.
    pub label: String,
    /// Its value.
    pub value: String,
    /// Whether the value is hex or an address, drawn in the mono face.
    pub mono: bool,
}

/// What the screen holds.
pub struct InvoiceState {
    /// The text in the field.
    text: String,
    /// What decoding it gave: the lines, or the refusal. `None` before
    /// the first decode and after the text changes.
    read: Option<Result<Vec<Row>, String>>,
    /// The screen its back link returns to.
    back: Screen,
}

impl InvoiceState {
    /// The text in the field.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// What decoding gave: the lines, or the refusal's wording.
    pub fn read(&self) -> Option<&Result<Vec<Row>, String>> {
        self.read.as_ref()
    }

    /// The screen its back link returns to.
    pub fn back(&self) -> Screen {
        self.back
    }
}

/// The screen's area: its state while it is open.
#[derive(Default)]
pub(crate) struct InvoiceArea {
    state: Option<InvoiceState>,
}

impl InvoiceArea {
    /// The screen's state, while it is open.
    pub(crate) fn state(&self) -> Option<&InvoiceState> {
        self.state.as_ref()
    }

    /// Opens the screen with `text` in the field, to return to `back`.
    fn open(&mut self, text: &str, back: Screen) {
        self.state = Some(InvoiceState {
            text: text.chars().take(FIELD_MAX).collect(),
            read: None,
            back,
        });
    }

    /// Forgets the text and what it read.
    pub(crate) fn clear(&mut self) {
        self.state = None;
    }

    /// Reads the text in the field, once.
    fn decode(&mut self) {
        if let Some(s) = self.state.as_mut()
            && !s.text.trim().is_empty()
        {
            s.read = Some(match bolt11::decode(&s.text) {
                Ok(i) => Ok(rows(&i)),
                Err(e) => Err(refusal(e)),
            });
        }
    }

    /// A key into the field: whether it was taken. A character other than
    /// white space, Backspace, and Enter, which decodes.
    fn key(&mut self, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key as K;
        let Some(s) = self.state.as_mut() else {
            return false;
        };
        match key {
            K::Char(c) if !c.is_control() && !c.is_whitespace() => {
                if s.text.chars().count() < FIELD_MAX {
                    s.text.push(c);
                }
                s.read = None;
            }
            K::Backspace => {
                s.text.pop();
                s.read = None;
            }
            K::Enter => self.decode(),
            _ => return false,
        }
        true
    }
}

/// Decode a Lightning invoice opened on its own, empty.
pub(crate) fn open(app: &mut Faraday) {
    let back = app.screen;
    app.tools.invoice_mut().open("", back);
    app.screen = Screen::Invoice;
}

/// Decode a Lightning invoice opened on an invoice scanned or carried,
/// decoded at once.
pub(crate) fn with(app: &mut Faraday, text: &str) {
    let back = match (app.screen, app.tools.invoice().state()) {
        (Screen::Invoice, Some(s)) => s.back,
        (screen, _) => screen,
    };
    let area = app.tools.invoice_mut();
    area.open(text, back);
    area.decode();
    app.screen = Screen::Invoice;
}

/// A read card's way over: the tool opened on the card's text. Home's
/// Scan filed it before the card, which then holds no code, so the text
/// is the file's (`filed`).
pub(crate) fn scanned(app: &mut Faraday, filed: Option<usize>, text: &str) {
    match filed.and_then(|at| app.inbox.get(at)) {
        Some(item) => {
            let text = String::from_utf8_lossy(&item.bytes).trim().to_string();
            with(app, &text);
        }
        None => with(app, text),
    }
}

/// One press on the screen.
pub(crate) fn act(app: &mut Faraday, press: InvoicePress) {
    let back = app
        .tools
        .invoice()
        .state()
        .map_or(Screen::Catalog, |s| s.back);
    match press {
        InvoicePress::Decode => app.tools.invoice_mut().decode(),
        InvoicePress::Scan => app.act(Action::ScanFor(crate::Ask::Invoice)),
        InvoicePress::Clear => app.tools.invoice_mut().open("", back),
    }
}

/// Typing into the field; Escape is the way back. Returns whether the key
/// was taken.
pub(crate) fn key(app: &mut Faraday, key: faraday_shell_api::Key) -> bool {
    if app.screen != Screen::Invoice || app.sheet.is_some() {
        return false;
    }
    if key == faraday_shell_api::Key::Escape {
        let back = app
            .tools
            .invoice()
            .state()
            .map_or(Screen::Catalog, |s| s.back);
        app.act(Action::Nav(back));
        return true;
    }
    app.tools.invoice_mut().key(key)
}

/// An amount in millisatoshis, in satoshis when whole.
fn amount(msat: u64) -> String {
    if msat.is_multiple_of(1000) {
        fill(EN.invoice_sat, &[&(msat / 1000).to_string()])
    } else {
        fill(EN.invoice_msat, &[&msat.to_string()])
    }
}

/// A Unix time as a UTC date and time.
fn when(unix: u64) -> String {
    fill(
        EN.invoice_utc,
        &[
            &crate::gpg::date(unix),
            &crate::vault::time_of_day_secs(unix),
        ],
    )
}

fn hex(bytes: &[u8]) -> String {
    faraday_crypto::hex::encode_string(bytes)
}

/// The lines an invoice shows, in the order they are drawn.
fn rows(i: &Invoice) -> Vec<Row> {
    let mut out = Vec::new();
    let mut put = |label: &str, value: String, mono: bool| {
        out.push(Row {
            label: label.to_string(),
            value,
            mono,
        });
    };
    put(EN.invoice_network, i.network.name().to_string(), false);
    put(
        EN.invoice_amount,
        i.amount_msat
            .map_or_else(|| EN.invoice_any_amount.to_string(), amount),
        false,
    );
    if let Some(d) = &i.description {
        put(EN.invoice_description, d.clone(), false);
    }
    if let Some(h) = &i.description_hash {
        put(EN.invoice_description_hash, hex(h), true);
    }
    put(EN.invoice_payee, hex(&i.payee), true);
    put(
        EN.invoice_payee_from,
        if i.payee_from_field {
            EN.invoice_payee_named
        } else {
            EN.invoice_payee_recovered
        }
        .to_string(),
        false,
    );
    put(EN.invoice_payment_hash, hex(&i.payment_hash), true);
    put(EN.invoice_payment_secret, hex(&i.payment_secret), true);
    put(EN.invoice_created, when(i.timestamp), false);
    put(
        EN.invoice_expires,
        when(i.timestamp.saturating_add(i.expiry)),
        false,
    );
    put(
        EN.invoice_final_cltv,
        fill(EN.invoice_blocks, &[&i.min_final_cltv_expiry.to_string()]),
        false,
    );
    for f in &i.fallbacks {
        let kind = match f {
            Fallback::P2pkh(_) => EN.invoice_fallback_p2pkh,
            Fallback::P2sh(_) => EN.invoice_fallback_p2sh,
            Fallback::Witness { .. } => EN.invoice_fallback_witness,
        };
        put(kind, f.address(i.network), true);
    }
    for (n, hint) in i.route_hints.iter().enumerate() {
        for (m, hop) in hint.iter().enumerate() {
            put(
                &fill(
                    EN.invoice_route_hop,
                    &[&(n + 1).to_string(), &(m + 1).to_string()],
                ),
                fill(
                    EN.invoice_hop,
                    &[
                        &hex(&hop.node_id),
                        &format!("{:016x}", hop.short_channel_id),
                        &hop.fee_base_msat.to_string(),
                        &hop.fee_proportional_millionths.to_string(),
                        &hop.cltv_expiry_delta.to_string(),
                    ],
                ),
                true,
            );
        }
    }
    if !i.features.is_empty() {
        let bits: Vec<String> = i.features.iter().map(u16::to_string).collect();
        put(EN.invoice_features, bits.join(", "), false);
    }
    if let Some(m) = &i.metadata {
        put(EN.invoice_metadata, hex(m), true);
    }
    for u in &i.unknown {
        put(
            &fill(EN.invoice_unknown, &[&u.tag.to_string()]),
            u.data
                .iter()
                .map(|v| format!("{v:02x}"))
                .collect::<Vec<_>>()
                .join(" "),
            true,
        );
    }
    out
}

/// One plain line for each reason an invoice is refused.
fn refusal(e: Error) -> String {
    match e {
        Error::TooLong => EN.invoice_err_too_long.to_string(),
        Error::MixedCase => EN.invoice_err_mixed_case.to_string(),
        Error::Bech32 => EN.invoice_err_bech32.to_string(),
        Error::Checksum => EN.invoice_err_checksum.to_string(),
        Error::Prefix => EN.invoice_err_prefix.to_string(),
        Error::Amount => EN.invoice_err_amount.to_string(),
        Error::TooShort => EN.invoice_err_too_short.to_string(),
        Error::Truncated => EN.invoice_err_truncated.to_string(),
        Error::Field(c) => fill(EN.invoice_err_field, &[&c.to_string()]),
        Error::Duplicate(c) => fill(EN.invoice_err_duplicate, &[&c.to_string()]),
        Error::NoPaymentHash => EN.invoice_err_no_payment_hash.to_string(),
        Error::NoPaymentSecret => EN.invoice_err_no_payment_secret.to_string(),
        Error::Description => EN.invoice_err_description.to_string(),
        Error::UnknownRequiredFeature(b) => fill(EN.invoice_err_feature, &[&b.to_string()]),
        Error::Signature => EN.invoice_err_signature.to_string(),
    }
}
