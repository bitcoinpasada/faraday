//! The written walk-through each step of a flow shows in Guided mode
//! (`docs/WALLETS.md` §6). Steps only (formerly Full) hides it.
//!
//! The rule for this text: two sentences, about 30 words and 40 at most,
//! saying what the card is for and what its press does. No reassurance
//! and no theory; the guide has the theory, one press away, at the
//! section of the card.

use crate::strings::EN;
use crate::strings::fill;
use crate::wallet::{Kind, step};
use crate::{Medium, SpendState};

/// The walk-through for one step of a spend.
pub(crate) fn spend(n: u8, kind: Option<Kind>, needed: usize, s: &SpendState, m: Medium) -> String {
    let cosigned = kind.is_some_and(Kind::has_cosigners);
    match n {
        step::WALLET => match kind {
            Some(_) => EN.walkthrough_this_is_the_wallet.to_string(),
            None => EN.walkthrough_no_wallet_loaded.to_string(),
        },
        step::CHECK if kind.is_none() => String::new(),
        step::CHECK => EN.walkthrough_open_the_same.to_string(),
        step::TRANSACTION => EN.walkthrough_read_each_line.to_string(),
        step::NONCES => EN.walkthrough_musig2_signs.to_string(),
        step::PATH => EN.walkthrough_this_wallet_can.to_string(),
        step::SIGNERS => match kind {
            Some(Kind::Miniscript) | Some(Kind::Tree) => EN.walkthrough_this_wallet_has.to_string(),
            None => EN.walkthrough_keys_signs_here.to_string(),
            _ => fill(
                EN.walkthrough_keys_must_sign,
                &[
                    &needed.to_string(),
                    (if needed == 1 {
                        EN.walkthrough_of_these
                    } else {
                        EN.walkthrough_keys_must_sign_plural
                    }),
                ],
            ),
        },
        step::SIGN => {
            if !s.signed_keys().is_empty() {
                EN.walkthrough_signed_each_signature.to_string()
            } else if kind == Some(Kind::MuSig) {
                EN.walkthrough_spend_round_two.to_string()
            } else if cosigned {
                fill(
                    EN.walkthrough_spend_adds_signatures,
                    &[
                        &needed.to_string(),
                        (if needed == 1 {
                            EN.walkthrough_signature
                        } else {
                            EN.walkthrough_signatures
                        }),
                    ],
                )
            } else {
                fill(EN.walkthrough_signing_uses, &[m.a()])
            }
        }
        step::COLLECT => fill(EN.walkthrough_get_the_other, &[m.a()]),
        step::FINISH => {
            if s.spend.finished.is_some() {
                EN.walkthrough_there_are_two.to_string()
            } else {
                EN.walkthrough_the_transaction.to_string()
            }
        }
        _ => String::new(),
    }
}

/// The walk-through for one card of the backup. `split`: the plan splits
/// the wallet description into shares.
pub(crate) fn backup(
    n: u8,
    split: bool,
    seeds_here: usize,
    medium: Medium,
    forms: crate::plan::CopyForms,
) -> String {
    use crate::bstep;
    match n {
        bstep::BLANK => EN.walkthrough_print_the_blank.to_string(),
        n if bstep::is_copy(n) => {
            let lead = if seeds_here == 0 {
                EN.walkthrough_backup_none_typed.to_string()
            } else {
                match (forms.words, forms.seedqr) {
                    (true, true) => EN.walkthrough_copy_both,
                    (false, true) => EN.walkthrough_copy_seedqr,
                    _ => EN.walkthrough_copy_words,
                }
                .to_string()
            };
            if seeds_here == 0 {
                lead
            } else {
                fill(EN.walkthrough_backup_paper, &[&lead])
            }
        }
        n if bstep::is_passphrase(n) => EN.walkthrough_passphrase_on_paper.to_string(),
        n if bstep::is_codex32(n) => EN.c32_guide_strings_item.to_string(),
        bstep::PUBLIC => fill(EN.walkthrough_backup_xpub_half, &[medium.for_the()]),
        bstep::SHEETS if split => fill(EN.walkthrough_instead_of_one, &[medium.for_the()]),
        bstep::SHEETS => fill(EN.walkthrough_backup_sheet, &[medium.for_the()]),
        bstep::FILES => fill(
            EN.walkthrough_backup_files,
            &[medium.for_the(), medium.noun()],
        ),
        bstep::SHOW => EN.walkthrough_backup_show.to_string(),
        bstep::STICKS => fill(EN.walkthrough_backup_sticks, &[medium.noun()]),
        n if n >= bstep::VAULT => fill(EN.walkthrough_backup_vault, &[medium.noun()]),
        bstep::ENVELOPE => EN.walkthrough_one_envelope.to_string(),
        _ => String::new(),
    }
}

/// The walk-through for one card of signing a message.
pub(crate) fn message(n: u8) -> String {
    use crate::mstep;
    match n {
        mstep::ADDRESS => EN.walkthrough_a_signed_message.to_string(),
        mstep::TEXT => EN.walkthrough_type_the_message.to_string(),
        mstep::FORMAT => EN.walkthrough_message_formats.to_string(),
        mstep::SIGN => EN.walkthrough_signing_moves.to_string(),
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
    match n {
        cstep::KIND => EN.walkthrough_create_kinds.to_string(),
        cstep::QUORUM => fill(
            EN.walkthrough_of_every_spend,
            &[&m.to_string(), &keys.to_string(), &m.to_string()],
        ),
        cstep::KEYS => {
            if multi {
                EN.walkthrough_fill_each_slot.to_string()
            } else {
                EN.walkthrough_choose_the_key.to_string()
            }
        }
        cstep::CHECK => EN.walkthrough_load_the_wallet.to_string(),
        cstep::WHERE => EN.walkthrough_create_vaults_where.to_string(),
        cstep::HOW_MANY => fill(
            EN.walkthrough_create_vaults_how_many,
            &[medium.nouns(), medium.noun()],
        ),
        cstep::STICKS => fill(
            EN.walkthrough_create_sticks,
            &[medium.noun(), medium.noun()],
        ),
        cstep::VAULTS => EN.walkthrough_create_vaults.to_string(),
        _ => String::new(),
    }
}

/// The walk-through for one card of restoring a wallet.
pub(crate) fn restore(n: u8, medium: Medium) -> String {
    use crate::rstep;
    match n {
        rstep::KIND => EN.walkthrough_the_kind_of_wallet.to_string(),
        rstep::QUORUM => EN.walkthrough_how_many_keys.to_string(),
        rstep::DESCRIPTION => EN.walkthrough_the_descriptor.to_string(),
        rstep::SEEDS => EN.walkthrough_one_slot_per.to_string(),
        rstep::CHECK => EN.walkthrough_compare_these.to_string(),
        rstep::WHERE => EN.walkthrough_create_vaults_where.to_string(),
        rstep::HOW_MANY => fill(
            EN.walkthrough_create_vaults_how_many,
            &[medium.nouns(), medium.noun()],
        ),
        _ => EN.walkthrough_restore_session.to_string(),
    }
}

/// The walk-through for one card of New key.
pub(crate) fn keygen(n: u8, k: &crate::keygen::KeyGen) -> String {
    use crate::keygen::{Source, kstep};
    let (active, slip39, by_die) = (k.active(), k.slip39, k.by_die);
    match n {
        kstep::START => EN.walkthrough_keygen_start.to_string(),
        kstep::LENGTH if slip39 => EN.walkthrough_a_slip_39_key.to_string(),
        kstep::WORDS if slip39 => EN.walkthrough_write_each_share.to_string(),
        kstep::QUIZ if slip39 => EN.walkthrough_answer_from_your_papers.to_string(),
        kstep::LENGTH => EN.walkthrough_12_words_carry.to_string(),
        kstep::SOURCE if slip39 => EN.walkthrough_randomness_you_make.to_string(),
        kstep::SOURCE => EN.walkthrough_randomness_you.to_string(),
        kstep::ENTER => match active {
            Some(Source::Dice | Source::Coins) if by_die && slip39 => {
                EN.walkthrough_roll_one_die_at_a_time.to_string()
            }
            Some(Source::Dice | Source::Coins) if by_die => {
                EN.walkthrough_keygen_dice_flips.to_string()
            }
            Some(Source::Dice) if k.procedure().direct() => {
                EN.walkthrough_keygen_dice_words.to_string()
            }
            Some(Source::Coins) if !slip39 => EN.walkthrough_flip_a_coin_and_enter.to_string(),
            Some(Source::Dice) => EN.walkthrough_roll_one_die.to_string(),
            Some(Source::Coins) => EN.walkthrough_keygen_coins.to_string(),
            Some(Source::Cards) => EN.walkthrough_shuffle_a_full.to_string(),
            Some(Source::Hex) => EN.walkthrough_type_the_hex.to_string(),
            Some(Source::Camera) => EN.walkthrough_point_the_camera.to_string(),
            _ => EN.walkthrough_this_device_asks.to_string(),
        },
        kstep::KEY if k.slot.is_some() => EN.walkthrough_keygen_lock_step.to_string(),
        kstep::KEY => EN.walkthrough_lock_in_adds.to_string(),
        kstep::CHECK => EN.walkthrough_these_counts.to_string(),
        kstep::WORDS => EN.walkthrough_write_the_words.to_string(),
        _ => EN.walkthrough_answer_from_your.to_string(),
    }
}
