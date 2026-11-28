//! What a refused extended private key says, by name (`main`'s `one_line::xprv_refusal`).

use faraday_bip::xkey;
use faraday_keys::fp_text;
use faraday_keys::xprv::XprvRefusal;
use faraday_strings::EN;
use faraday_text::Text;

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
