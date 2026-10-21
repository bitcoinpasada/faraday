//! The written walk-through each step of a flow shows in Guided mode
//! (`docs/WALLETS.md` §6). Steps only (formerly Full) hides it.
//!
//! The rule for this text: say what is on the screen, what to compare it
//! with, and what happens next. No reassurance and no theory; Learn has
//! the theory.

use crate::strings::EN;
use crate::strings::fill;
use crate::wallet::{Kind, step};
use crate::{Medium, SpendState};

/// The walk-through for one step of a spend.
pub(crate) fn spend(n: u8, kind: Option<Kind>, needed: usize, s: &SpendState, m: Medium) -> String {
    let cosigned = kind.is_some_and(Kind::has_cosigners);
    match n {
        step::WALLET => match kind {
            Some(_) => EN.guide_this_is_the_wallet.to_string(),
            None => EN.guide_no_wallet_loaded.to_string(),
        },
        step::CHECK => EN.guide_open_the_same.to_string(),
        step::TRANSACTION => EN.guide_read_each_line.to_string(),
        step::NONCES => EN.guide_musig2_signs.to_string(),
        step::PATH => EN.guide_this_wallet_can.to_string(),
        step::TXID => EN.guide_this_is_the_transaction.to_string(),
        step::SIGNERS => match kind {
            Some(Kind::Threshold) => fill(
                EN.guide_any_of_these,
                &[
                    &needed.to_string(),
                    &(if needed == 2 {
                        EN.guide_one_other.to_string()
                    } else {
                        fill(EN.guide_others, &[&(needed - 1).to_string()])
                    })
                    .to_string(),
                ],
            ),
            Some(Kind::Miniscript) | Some(Kind::Tree) => EN.guide_this_wallet_has.to_string(),
            _ => fill(
                EN.guide_keys_must_sign,
                &[
                    &needed.to_string(),
                    (if needed == 1 {
                        EN.guide_of_these
                    } else {
                        EN.guide_of_the
                    }),
                ],
            ),
        },
        step::SIGN => {
            if !s.spend.signed_here.is_empty() {
                EN.guide_signed_each_signature.to_string()
            } else if kind == Some(Kind::Threshold) {
                fill(EN.guide_signing_as_a, &[m.noun()])
            } else if kind == Some(Kind::MuSig) {
                EN.guide_round_two_this.to_string()
            } else if cosigned {
                fill(
                    EN.guide_this_adds_the,
                    &[
                        &needed.to_string(),
                        (if needed == 1 {
                            EN.guide_signature
                        } else {
                            EN.guide_signatures
                        }),
                    ],
                )
            } else {
                fill(EN.guide_signing_uses, &[m.a()])
            }
        }
        step::COLLECT => fill(EN.guide_get_the_other, &[m.a(), m.a()]),
        step::FINISH => {
            if s.spend.finished.is_some() {
                EN.guide_there_are_two.to_string()
            } else if kind == Some(Kind::Threshold) && (s.carry_out.is_some() || s.out_signed) {
                fill(EN.guide_keep_the_carry, &[m.a()])
            } else {
                EN.guide_the_transaction.to_string()
            }
        }
        _ => String::new(),
    }
}

/// The walk-through for one card of the backup.
pub(crate) fn backup(
    n: u8,
    m: usize,
    keys: usize,
    seeds_here: usize,
    medium: Medium,
    forms: crate::plan::CopyForms,
) -> String {
    use crate::bstep;
    match n {
        bstep::BLANK => fill(EN.guide_print_the_blank, &[medium.for_the()]),
        n if bstep::is_copy(n) => {
            let lead = if seeds_here == 0 {
                EN.guide_none_of_this.to_string()
            } else {
                match (forms.words, forms.seedqr) {
                    (true, true) => EN.guide_copy_both,
                    (false, true) => EN.guide_copy_seedqr,
                    _ => EN.guide_copy_words,
                }
                .to_string()
            };
            let further = if seeds_here == 0 {
                String::new()
            } else {
                format!(
                    " {}",
                    fill(
                        EN.guide_paper_comes_first,
                        &[medium.for_the(), medium.noun()]
                    )
                )
            };
            fill(
                EN.guide_a_printer_is,
                &[&lead.to_string(), &further.to_string()],
            )
        }
        bstep::PUBLIC => fill(EN.guide_this_half_is, &[medium.for_the()]),
        bstep::SHEETS if m > 1 => fill(
            EN.guide_instead_of_one,
            &[
                &keys.to_string(),
                &m.saturating_sub(1).to_string(),
                &m.to_string(),
                &m.to_string(),
                medium.for_the(),
            ],
        ),
        bstep::ENVELOPE => EN.guide_one_envelope.to_string(),
        _ => String::new(),
    }
}

/// The walk-through for one card of signing a message.
pub(crate) fn message(n: u8) -> String {
    use crate::mstep;
    match n {
        mstep::ADDRESS => EN.guide_a_signed_message.to_string(),
        mstep::TEXT => EN.guide_type_the_message.to_string(),
        mstep::FORMAT => EN.guide_bip_322_is_the.to_string(),
        mstep::SIGN => EN.guide_signing_moves.to_string(),
        _ => String::new(),
    }
}

/// The walk-through for one card of creating a wallet.
pub(crate) fn create(
    n: u8,
    kind: crate::create::NewKind,
    m: usize,
    keys: usize,
    medium: Medium,
) -> String {
    use crate::cstep;
    let multi = kind.multi();
    if kind.threshold() {
        match n {
            cstep::QUORUM => {
                return fill(EN.guide_any_of_shares, &[&m.to_string(), &keys.to_string()]);
            }
            cstep::KEYS => {
                return fill(
                    EN.guide_choose_loaded,
                    &[&m.to_string(), &(keys - m).to_string()],
                );
            }
            _ => {}
        }
    }
    match n {
        cstep::KIND => EN.guide_choose_how_the.to_string(),
        cstep::QUORUM => fill(
            EN.guide_of_every_spend,
            &[&m.to_string(), &keys.to_string(), &m.to_string()],
        ),
        cstep::KEYS => {
            if multi {
                fill(EN.guide_fill_each_slot, &[medium.a()])
            } else {
                EN.guide_choose_the_key.to_string()
            }
        }
        cstep::CHECK => EN.guide_load_the_wallet.to_string(),
        cstep::BACKUP => EN.guide_the_plan_says.to_string(),
        _ => String::new(),
    }
}

/// The walk-through for one card of restoring a wallet.
pub(crate) fn restore(n: u8) -> String {
    use crate::rstep;
    match n {
        rstep::KIND => EN.guide_the_kind_of_wallet.to_string(),
        rstep::QUORUM => EN.guide_how_many_keys.to_string(),
        rstep::DESCRIPTION => EN.guide_the_descriptor.to_string(),
        rstep::SEEDS => EN.guide_one_slot_per.to_string(),
        rstep::CHECK => EN.guide_compare_these.to_string(),
        _ => EN.guide_the_wallet_is.to_string(),
    }
}

/// The walk-through for one card of New key.
pub(crate) fn keygen(n: u8, k: &crate::keygen::KeyGen) -> String {
    use crate::keygen::{Source, kstep};
    let (active, slip39, by_die) = (k.active(), k.slip39, k.by_die);
    match n {
        kstep::LENGTH if slip39 => EN.guide_a_slip_39_key.to_string(),
        kstep::WORDS if slip39 => EN.guide_write_each_share.to_string(),
        kstep::QUIZ if slip39 => EN.guide_answer_from_your_papers.to_string(),
        kstep::LENGTH => EN.guide_12_words_carry.to_string(),
        kstep::SOURCE if slip39 => EN.guide_randomness_you_make.to_string(),
        kstep::SOURCE => EN.guide_randomness_you.to_string(),
        kstep::ENTER => match active {
            Some(Source::Dice | Source::Coins) if by_die && slip39 => {
                EN.guide_roll_one_die_at_a_time.to_string()
            }
            Some(Source::Dice | Source::Coins) if by_die => EN.guide_roll_one_die_at_a.to_string(),
            Some(Source::Dice) if k.procedure().direct() => fill(
                EN.guide_roll_one_die_at,
                &[&k.last_bits().to_string(), &(k.words / 3).to_string()],
            ),
            Some(Source::Coins) if !slip39 => EN.guide_flip_a_coin_and_enter.to_string(),
            Some(Source::Dice) => EN.guide_roll_one_die.to_string(),
            Some(Source::Coins) => EN.guide_flip_a_coin_and.to_string(),
            Some(Source::Cards) => EN.guide_shuffle_a_full.to_string(),
            Some(Source::Hex) => EN.guide_type_the_hex.to_string(),
            Some(Source::Camera) => EN.guide_point_the_camera.to_string(),
            _ => EN.guide_this_device_asks.to_string(),
        },
        kstep::KEY if k.slot.is_some() => EN.guide_lock_in_adds_the.to_string(),
        kstep::KEY => EN.guide_lock_in_adds.to_string(),
        kstep::CHECK => EN.guide_these_counts.to_string(),
        kstep::WORDS => EN.guide_write_the_words.to_string(),
        _ => EN.guide_answer_from_your.to_string(),
    }
}
