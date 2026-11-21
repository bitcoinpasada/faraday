//! The two forms read from one line of text, an extended private key and a
//! master seed in hex (`docs/KEY-FORMS.md` §6): what each refusal of the key
//! boundary says. The reading itself is `faraday_keys::xprv`.

use crate::strings::EN;
use crate::wallet::fp_text;
use faraday_bip::xkey;
use faraday_keys::xprv::{HexSeedRefusal, XprvError, XprvRefusal};
use faraday_text::Text;

/// What a refused extended private key says, by name.
pub(crate) fn xprv_refusal(r: &XprvRefusal) -> Text {
    match r {
        XprvRefusal::Unreadable(e) => Text::of(match e {
            xkey::Error::Base58 => EN.forms_xprv_base58,
            xkey::Error::Length(n) => {
                return Text::fill(EN.forms_xprv_length, &[&Text::num(*n as u64)]);
            }
            xkey::Error::UnknownVersion(_) => EN.forms_xprv_version,
            xkey::Error::ZeroDepth => EN.forms_xprv_zero_depth,
            xkey::Error::Pad(_) => EN.forms_xprv_pad,
            xkey::Error::Key(_) => EN.forms_xprv_key_range,
        }),
        XprvRefusal::Slip132(name) => Text::fill(EN.forms_xprv_slip132, &[&Text::of(name)]),
        XprvRefusal::WrongNetwork { found, wanted } => Text::fill(
            EN.forms_xprv_wrong_network,
            &[&Text::of(found), &Text::of(wanted)],
        ),
        XprvRefusal::NotMaster {
            depth,
            parent,
            index,
            hardened,
        } => Text::fill(
            EN.forms_xprv_not_master,
            &[
                &Text::num(u64::from(*depth)),
                &fp_text(*parent),
                &if *hardened {
                    faraday_text::text!("{}'", index)
                } else {
                    faraday_text::text!("{}", index)
                },
            ],
        ),
        XprvRefusal::OriginUnreadable => Text::of(EN.forms_xprv_origin_unreadable),
        XprvRefusal::OriginPath => Text::of(EN.forms_xprv_origin_path),
        XprvRefusal::OriginDiffers { named, own } => Text::fill(
            EN.forms_xprv_origin_differs,
            &[&fp_text(*named), &fp_text(*own)],
        ),
    }
}

/// What an extended private key the session did not add says.
pub(crate) fn xprv_error(e: &XprvError) -> Text {
    match e {
        XprvError::Read(r) => xprv_refusal(r),
        XprvError::Session(s) => s.text(),
    }
}

/// What a refused hex seed says, with its length where it has one.
pub(crate) fn hex_refusal(r: HexSeedRefusal) -> Text {
    match r {
        HexSeedRefusal::NotHex => Text::of(EN.forms_hex_not_hex),
        HexSeedRefusal::OddDigits(n) => Text::fill(EN.forms_hex_odd, &[&Text::num(n as u64)]),
        HexSeedRefusal::Length(n) => Text::fill(EN.forms_hex_length, &[&Text::num(n as u64)]),
    }
}

/// Whether a file's whole text is one key of the two one-line forms: an
/// extended private key (of any version, loaded or refused by the form
/// when it is opened), or a master seed in hex of a BIP-32 length. A hex
/// file may be a hash, which is why a listed file loads only when it is
/// opened (`docs/KEY-FORMS.md` §6.3).
pub(crate) fn line_form(
    c: &faraday_crypto::work::Compute,
    text: &str,
) -> Option<crate::forms::Form> {
    let t = text.trim();
    if t.is_empty() || t.split_whitespace().nth(1).is_some() {
        return None;
    }
    if matches!(
        faraday_intake::secret_text(c, t),
        Some(faraday_intake::TextSecret::ExtendedPrivateKey)
    ) {
        return Some(crate::forms::Form::Xprv);
    }
    faraday_keys::xprv::parse_hex_seed(t)
        .is_ok()
        .then_some(crate::forms::Form::HexSeed)
}
