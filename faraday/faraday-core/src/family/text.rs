//! The Spend tab's words (`docs/FAMILY.md` §4): the offline system is
//! Faraday itself, the wallet usually comes from a vault, and a stick is
//! read only after a lock. Each card has a lead, always shown, and a
//! longer More about this.
//!
//! The rule for this text is Faraday's: plain statements of fact. The
//! review's warnings are not here; they are the inspector's own words, as
//! Wallets shows them.

use crate::drawing::View;
use crate::family::{CardId, Route, page};
use crate::strings::EN;
use crate::strings::fill;
use crate::wallet::{Kind, step};
use crate::{Faraday, Medium, guide};

/// The map: where each stop happens, its name, and what is done there.
pub fn map(m: Medium) -> [(&'static str, &'static str, String); 6] {
    [
        (
            EN.family_paper,
            EN.family_what_you_have,
            fill(EN.family_the_faraday, &[m.noun()]),
        ),
        (
            EN.family_offline,
            EN.family_open_the_wallet_here,
            fill(EN.family_start_a_computer, &[m.noun()]),
        ),
        (
            EN.family_online,
            EN.family_check_the_balance,
            EN.family_put_the_wallet.to_string(),
        ),
        (
            EN.family_online,
            EN.family_write_the_payment,
            EN.family_make_the_transaction.to_string(),
        ),
        (
            EN.family_offline,
            EN.family_sign_it_here,
            EN.family_bring_the_psbt.to_string(),
        ),
        (
            EN.family_online,
            EN.family_title_send,
            EN.family_take_the_signed.to_string(),
        ),
    ]
}

/// The line under the map that is never folded away.
pub const WORDS_ARE_MONEY: &str = EN.family_the_words_are;

/// What do you have?'s link to the list of what each thing looks like.
pub const UNSURE: &str = EN.family_not_sure_what;

/// The hint with one seed in: what else may be in the envelope.
pub const ANYTHING_HINT: &str = EN.family_another_list;

/// The hint with several seeds in and no description: what one looks
/// like.
pub const DESCRIPTION_HINT: &str = EN.family_its_description;

/// What each thing in the envelope looks like: its look, then what it is.
pub fn looks(m: Medium) -> [(String, String); 8] {
    let looks: [(&str, &str); 7] = [
        (EN.family_a_square_black, EN.family_looks_qr_code),
        (EN.family_words_numbered, EN.family_the_seed_phrase),
        (EN.family_one_long_line, EN.family_a_descriptor),
        (EN.family_several_sheets, EN.family_one_description),
        (EN.family_a_word_or_sentence, EN.family_if_one_exists),
        (EN.family_a_short_line, EN.family_one_address),
        (EN.family_a_small_device, EN.family_a_hardware_wallet),
    ];
    let [a, b, c, d, e, f, g] = looks.map(|(look, what)| (look.to_string(), what.to_string()));
    let medium = (
        fill(EN.family_labelled_faraday, &[m.a_full_cap()]),
        fill(EN.family_looks_medium, &[m.noun()]),
    );
    [medium, a, b, c, d, e, f, g]
}

/// A card's title.
pub fn title(app: &Faraday, id: CardId) -> String {
    let s = match id {
        CardId::Page(p) => match p {
            page::MAP => EN.family_how_to_spend,
            page::SAFE => return fill(EN.family_title_start, &[app.medium.noun()]),
            page::HOLDING => EN.family_what_do_you,
            page::OPEN => EN.family_open_the_wallet,
            page::CHECK if app.family.from_card => EN.family_load_the_wallet,
            page::CHECK if app.family_guess().is_some() => EN.family_title_check_money,
            page::WRITE if app.family.from_card => EN.family_title_write_payment,
            page::CHECK => EN.family_check_the_money,
            page::WRITE => EN.family_write_the_payment,
            page::BRING => EN.family_bring_the_transaction,
            _ => EN.family_put_everything,
        },
        CardId::Step(n) | CardId::Later(n) => match n {
            step::TRANSACTION => EN.family_read_the_transaction,
            step::TXID => EN.family_the_transaction,
            step::PATH => EN.family_title_which_way,
            step::NONCES => EN.family_nonces,
            step::SIGNERS if app.family.route == Some(Route::Paper) => EN.family_add_the_seeds,
            step::SIGNERS => EN.family_who_signs,
            step::SIGN => EN.family_title_sign,
            step::COLLECT => EN.family_the_other_signatures,
            step::FINISH => EN.family_title_send,
            _ => "",
        },
    };
    s.to_string()
}

/// A card's lead: what to do on it, always shown while it is open.
pub fn lead(app: &View, id: CardId) -> String {
    let s: &str = match id {
        CardId::Page(page::MAP) => EN.family_six_stops_three,
        CardId::Page(page::SAFE) if app.online => {
            return fill(EN.family_lead_desktop_copy, &[app.medium.noun()]);
        }
        CardId::Page(page::SAFE) => {
            return fill(EN.family_lead_running_offline, &[app.medium.noun()]);
        }
        CardId::Page(page::HOLDING | page::OPEN) => EN.family_scan_or_type,
        // From the wallet's card: what to press, no more.
        CardId::Page(page::CHECK) if app.family.from_card => EN.family_in_sparrow_file_new,
        CardId::Page(page::WRITE) if app.family.from_card => EN.family_in_sparrow_s,
        CardId::Page(page::CHECK) if app.family_guess().is_some() => EN.family_words_alone,
        CardId::Page(page::CHECK) => EN.family_load_this_same,
        CardId::Page(page::WRITE) => EN.family_this_part_happens,
        CardId::Page(page::BRING) => {
            return fill(
                EN.family_two_ways_scanning,
                &[app.medium.a(), app.medium.a(), app.medium.noun()],
            );
        }
        CardId::Page(_) => {
            return fill(
                EN.family_lock_faraday,
                &[
                    app.medium.noun(),
                    app.medium.a(),
                    app.medium.noun(),
                    app.medium.noun(),
                ],
            );
        }
        CardId::Later(_) => EN.family_this_opens_once,
        CardId::Step(step::TRANSACTION) => EN.family_read_the_summary,
        CardId::Step(step::TXID) => EN.family_lead_write_down_id,
        CardId::Step(step::SIGNERS) if signers_missing(app) => EN.family_lead_every_key,
        CardId::Step(step::SIGNERS) if app.family.route == Some(Route::Paper) => {
            EN.family_lead_one_seed
        }
        CardId::Step(step::SIGN) => EN.family_signing_adds,
        CardId::Step(step::FINISH) => {
            return fill(EN.family_the_finished, &[app.medium.a()]);
        }
        CardId::Step(n) => {
            let s = app.spend.as_ref();
            let kind = s
                .and_then(|s| s.wallet)
                .and_then(|w| app.session.wallets.get(w))
                .map(|w| Kind::of(w.policy()));
            return match s {
                Some(s) => guide::spend(n, kind, app.spend_needed(), s, app.medium),
                None => String::new(),
            };
        }
    };
    s.to_string()
}

/// The spend's wallet needs more keys than are here.
fn signers_missing(app: &View) -> bool {
    let Some(s) = app.spend.as_ref() else {
        return false;
    };
    let here = s
        .inspection
        .participating_keys
        .iter()
        .filter(|fp| app.session.keys.key_label(**fp).is_some())
        .count();
    app.family.route != Some(Route::Paper) && here < app.spend_needed()
}

/// A card's More about this, as separate paragraphs.
pub fn more(app: &View, id: CardId) -> Vec<String> {
    let m = app.medium;
    let fixed: &[&str] = match id {
        CardId::Page(page::MAP) => {
            let mut paras = strings(&[
                EN.family_anyone_who_reads,
                EN.family_nothing_done,
                EN.family_more_no_deadline,
            ]);
            paras.push(fill(EN.family_more_stop_if_unsure, &[m.noun()]));
            return paras;
        }
        CardId::Page(page::SAFE) => {
            return vec![
                EN.family_shut_the_computer.to_string(),
                fill(EN.family_plug_the_faraday, &[m.noun()]),
                fill(EN.family_more_pick_boot_device, &[m.full()]),
                fill(EN.family_if_the_is_missing, &[m.noun(), m.a()]),
            ];
        }
        CardId::Page(page::HOLDING | page::OPEN) => &[
            EN.family_a_vault_unlocks,
            EN.family_more_words_order,
            EN.family_a_wallet_description,
            EN.family_split_sheets,
        ],
        CardId::Page(page::CHECK) => &[
            EN.family_in_sparrow_file,
            EN.family_more_trusted_software,
            EN.family_faraday_can,
            EN.family_on_a_phone_you,
        ],
        CardId::Page(page::WRITE) => &[EN.family_paying_into, EN.family_faraday_locks],
        CardId::Step(step::SIGN) => &[EN.family_if_the_envelope, EN.family_more_faraday_enough],
        CardId::Step(step::FINISH) => &[
            EN.family_any_of_these,
            EN.family_more_check_id,
            EN.family_the_first_confirmation,
        ],
        CardId::Page(page::AWAY) => {
            return vec![fill(EN.family_when_faraday, &[m.noun()])];
        }
        _ => &[],
    };
    strings(fixed)
}

fn strings(paras: &[&str]) -> Vec<String> {
    paras.iter().map(|p| p.to_string()).collect()
}
