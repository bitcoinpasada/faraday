//! The written walk-through each step of a flow shows in Guided mode
//! (`docs/WALLETS.md` §6). Steps only (formerly Full) hides it.
//!
//! The rule for this text: two sentences, about 30 words and 40 at most,
//! saying what the card is for and what its press does. No reassurance
//! and no theory; Learn has the theory, one press away: each card that
//! has a Learn page names it in the `*_topic` function beside its text.

use crate::learn::Topic;
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
        step::CHECK if kind.is_none() => String::new(),
        step::CHECK => EN.guide_open_the_same.to_string(),
        step::TRANSACTION => EN.guide_read_each_line.to_string(),
        step::NONCES => EN.guide_musig2_signs.to_string(),
        step::PATH => EN.guide_this_wallet_can.to_string(),
        step::SIGNERS => match kind {
            Some(Kind::Miniscript) | Some(Kind::Tree) => EN.guide_this_wallet_has.to_string(),
            None => EN.guide_keys_signs_here.to_string(),
            _ => fill(
                EN.guide_keys_must_sign,
                &[
                    &needed.to_string(),
                    (if needed == 1 {
                        EN.guide_of_these
                    } else {
                        EN.guide_keys_must_sign_plural
                    }),
                ],
            ),
        },
        step::SIGN => {
            if !s.spend.signed_here.is_empty() {
                EN.guide_signed_each_signature.to_string()
            } else if kind == Some(Kind::MuSig) {
                EN.guide_spend_round_two.to_string()
            } else if cosigned {
                fill(
                    EN.guide_spend_adds_signatures,
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
        step::COLLECT => fill(EN.guide_get_the_other, &[m.a()]),
        step::FINISH => {
            if s.spend.finished.is_some() {
                EN.guide_there_are_two.to_string()
            } else {
                EN.guide_the_transaction.to_string()
            }
        }
        _ => String::new(),
    }
}

/// The Learn page under the walk-through of one step of a spend, if it
/// has one.
pub(crate) fn spend_topic(n: u8, kind: Option<Kind>, s: &SpendState) -> Option<Topic> {
    match n {
        step::WALLET | step::TRANSACTION => Some(Topic::Verifying),
        step::CHECK => kind.map(|_| Topic::Verifying),
        step::NONCES => Some(Topic::Nonces),
        step::PATH => Some(Topic::SpendPaths),
        step::SIGNERS => match kind {
            Some(Kind::Miniscript | Kind::Tree) => Some(Topic::SpendPaths),
            Some(_) => Some(Topic::Multisig),
            None => None,
        },
        step::SIGN if !s.spend.signed_here.is_empty() => None,
        step::SIGN => match kind {
            Some(Kind::MuSig) => Some(Topic::Nonces),
            Some(k) if k.has_cosigners() => Some(Topic::Multisig),
            _ => Some(Topic::AirGap),
        },
        step::COLLECT => Some(Topic::Multisig),
        step::FINISH if s.spend.finished.is_some() => Some(Topic::Verifying),
        _ => None,
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
        bstep::BLANK => EN.guide_print_the_blank.to_string(),
        n if bstep::is_copy(n) => {
            let lead = if seeds_here == 0 {
                EN.guide_backup_none_typed.to_string()
            } else {
                match (forms.words, forms.seedqr) {
                    (true, true) => EN.guide_copy_both,
                    (false, true) => EN.guide_copy_seedqr,
                    _ => EN.guide_copy_words,
                }
                .to_string()
            };
            if seeds_here == 0 {
                lead
            } else {
                fill(EN.guide_backup_paper, &[&lead])
            }
        }
        n if bstep::is_passphrase(n) => EN.guide_passphrase_on_paper.to_string(),
        n if bstep::is_codex32(n) => EN.c32_guide_strings_item.to_string(),
        bstep::PUBLIC => fill(EN.guide_backup_xpub_half, &[medium.for_the()]),
        bstep::SHEETS if split => fill(EN.guide_instead_of_one, &[medium.for_the()]),
        bstep::SHEETS => fill(EN.guide_backup_sheet, &[medium.for_the()]),
        bstep::FILES => fill(EN.guide_backup_files, &[medium.for_the(), medium.noun()]),
        bstep::SHOW => EN.guide_backup_show.to_string(),
        bstep::STICKS => fill(EN.guide_backup_sticks, &[medium.noun()]),
        n if n >= bstep::VAULT => fill(EN.guide_backup_vault, &[medium.noun()]),
        bstep::ENVELOPE => EN.guide_one_envelope.to_string(),
        _ => String::new(),
    }
}

/// The Learn page under the walk-through of one card of the backup.
pub(crate) fn backup_topic(n: u8, split: bool) -> Option<Topic> {
    use crate::bstep;
    match n {
        bstep::BLANK | bstep::FILES | bstep::ENVELOPE => Some(Topic::Backups),
        n if bstep::is_copy(n) => Some(Topic::Backups),
        n if bstep::is_codex32(n) => Some(Topic::OtherBackups),
        n if bstep::is_passphrase(n) => Some(Topic::Passphrases),
        bstep::PUBLIC | bstep::SHOW => Some(Topic::Xpubs),
        bstep::SHEETS if split => Some(Topic::Shares),
        bstep::SHEETS => Some(Topic::Xpubs),
        n if n >= bstep::VAULT => Some(Topic::Vaults),
        _ => None,
    }
}

/// The walk-through for one card of signing a message.
pub(crate) fn message(n: u8) -> String {
    use crate::mstep;
    match n {
        mstep::ADDRESS => EN.guide_a_signed_message.to_string(),
        mstep::TEXT => EN.guide_type_the_message.to_string(),
        mstep::FORMAT => EN.guide_message_formats.to_string(),
        mstep::SIGN => EN.guide_signing_moves.to_string(),
        _ => String::new(),
    }
}

/// The Learn page under the walk-through of one card of signing a
/// message.
pub(crate) fn message_topic(n: u8) -> Option<Topic> {
    use crate::mstep;
    matches!(n, mstep::ADDRESS | mstep::FORMAT).then_some(Topic::Message)
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
        cstep::KIND => EN.guide_create_kinds.to_string(),
        cstep::QUORUM => fill(
            EN.guide_of_every_spend,
            &[&m.to_string(), &keys.to_string(), &m.to_string()],
        ),
        cstep::KEYS => {
            if multi {
                EN.guide_fill_each_slot.to_string()
            } else {
                EN.guide_choose_the_key.to_string()
            }
        }
        cstep::CHECK => EN.guide_load_the_wallet.to_string(),
        cstep::WHERE => EN.guide_create_vaults_where.to_string(),
        cstep::HOW_MANY => fill(EN.guide_create_vaults_how_many, &[medium.noun()]),
        cstep::STICKS => fill(EN.guide_create_sticks, &[medium.noun(), medium.noun()]),
        cstep::VAULTS => EN.guide_create_vaults.to_string(),
        _ => String::new(),
    }
}

/// The Learn page under the walk-through of one card of creating a
/// wallet.
pub(crate) fn create_topic(n: u8, kind: crate::create::NewKind) -> Option<Topic> {
    use crate::cstep;
    match n {
        cstep::KIND => Some(Topic::WalletKinds),
        cstep::QUORUM => Some(Topic::Multisig),
        cstep::KEYS if kind.multi() => Some(Topic::Xpubs),
        cstep::CHECK => Some(Topic::Verifying),
        cstep::WHERE | cstep::VAULTS => Some(Topic::Vaults),
        _ => None,
    }
}

/// The walk-through for one card of restoring a wallet.
pub(crate) fn restore(n: u8, medium: Medium) -> String {
    use crate::rstep;
    match n {
        rstep::KIND => EN.guide_the_kind_of_wallet.to_string(),
        rstep::QUORUM => EN.guide_how_many_keys.to_string(),
        rstep::DESCRIPTION => EN.guide_the_descriptor.to_string(),
        rstep::SEEDS => EN.guide_one_slot_per.to_string(),
        rstep::CHECK => EN.guide_compare_these.to_string(),
        rstep::WHERE => EN.guide_create_vaults_where.to_string(),
        rstep::HOW_MANY => fill(EN.guide_create_vaults_how_many, &[medium.noun()]),
        _ => EN.guide_restore_session.to_string(),
    }
}

/// The Learn page under the walk-through of one card of restoring a
/// wallet.
pub(crate) fn restore_topic(n: u8) -> Option<Topic> {
    use crate::rstep;
    match n {
        rstep::KIND => Some(Topic::WalletKinds),
        rstep::QUORUM => Some(Topic::Multisig),
        rstep::DESCRIPTION => Some(Topic::Coordinators),
        rstep::SEEDS => Some(Topic::Passphrases),
        rstep::CHECK => Some(Topic::Verifying),
        rstep::WHERE => Some(Topic::Vaults),
        _ => None,
    }
}

/// The walk-through for one card of New key.
pub(crate) fn keygen(n: u8, k: &crate::keygen::KeyGen) -> String {
    use crate::keygen::{Source, kstep};
    let (active, slip39, by_die) = (k.active(), k.slip39, k.by_die);
    match n {
        kstep::START => EN.guide_keygen_start.to_string(),
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
            Some(Source::Dice | Source::Coins) if by_die => EN.guide_keygen_dice_flips.to_string(),
            Some(Source::Dice) if k.procedure().direct() => EN.guide_keygen_dice_words.to_string(),
            Some(Source::Coins) if !slip39 => EN.guide_flip_a_coin_and_enter.to_string(),
            Some(Source::Dice) => EN.guide_roll_one_die.to_string(),
            Some(Source::Coins) => EN.guide_keygen_coins.to_string(),
            Some(Source::Cards) => EN.guide_shuffle_a_full.to_string(),
            Some(Source::Hex) => EN.guide_type_the_hex.to_string(),
            Some(Source::Camera) => EN.guide_point_the_camera.to_string(),
            _ => EN.guide_this_device_asks.to_string(),
        },
        kstep::KEY if k.slot.is_some() => EN.guide_keygen_lock_step.to_string(),
        kstep::KEY => EN.guide_lock_in_adds.to_string(),
        kstep::CHECK => EN.guide_these_counts.to_string(),
        kstep::WORDS => EN.guide_write_the_words.to_string(),
        _ => EN.guide_answer_from_your.to_string(),
    }
}

/// The Learn page under the walk-through of one card of New key.
pub(crate) fn keygen_topic(n: u8, k: &crate::keygen::KeyGen) -> Option<Topic> {
    use crate::keygen::kstep;
    match n {
        kstep::LENGTH if k.slip39 => Some(Topic::OtherBackups),
        kstep::LENGTH => Some(Topic::Words),
        kstep::SOURCE | kstep::ENTER | kstep::CHECK => Some(Topic::Randomness),
        kstep::KEY if k.slot.is_some() => Some(Topic::Passphrases),
        kstep::WORDS | kstep::QUIZ if k.slip39 => Some(Topic::OtherBackups),
        kstep::WORDS | kstep::QUIZ => Some(Topic::Backups),
        _ => None,
    }
}
