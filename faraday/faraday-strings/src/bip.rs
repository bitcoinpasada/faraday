//! What faraday-bip refuses, in the strings file's words.
//!
//! faraday-bip's errors are kinds with their data and carry no wording;
//! each function here maps one kind to the sentence a person reads,
//! filled from that data. Every function is a lookup and a small fill.

use faraday_bip::{bip39, bsms, codex32, keys, musig, policy, slip39};

use faraday_text::{Num, Text};

use crate::EN;

/// A count or position as digits on the stack.
fn num(n: impl Into<u64>) -> Num {
    Text::num(n.into())
}

/// A BIP-39 refusal.
pub fn bip39_error(e: &bip39::Error) -> Text {
    use bip39::Error as E;
    match e {
        E::InvalidWordCount => Text::of(EN.bip_word_count),
        E::InvalidEntropyLength => Text::of(EN.bip_entropy_length),
        E::UnknownWord { position } => {
            Text::fill(EN.bip_word_not_in_list, &[&num(u32::from(*position) + 1)])
        }
        E::BadChecksum => Text::of(EN.bip_checksum_mismatch),
        E::PassphraseNotAscii => Text::of(EN.bip_passphrase_not_ascii),
        E::PassphraseTooLong => Text::fill(
            EN.bip_passphrase_too_long,
            &[&num(bip39::MAX_PASSPHRASE_BYTES as u64)],
        ),
    }
}

/// A BIP-129 (BSMS) refusal.
pub fn bsms_error(e: &bsms::Error) -> Text {
    use bsms::Error as E;
    Text::of(match e {
        E::Version => EN.bip_bsms_version,
        E::Lines => EN.bip_bsms_lines,
        E::Token => EN.bip_bsms_token,
        E::Encrypted => EN.bip_bsms_encrypted,
        E::Key => EN.bip_key_not_xpub,
        E::Description => EN.bip_bsms_description,
        E::Signature => EN.bip_bsms_signature,
        E::Paths => EN.bip_bsms_paths,
        E::Address => EN.bip_bsms_address,
        E::FirstAddress => EN.bip_bsms_first_address,
        E::Policy(p) => return policy_error(p),
    })
}

/// A MuSig2 refusal.
pub fn musig_error(e: &musig::Error) -> Text {
    use musig::Error as E;
    Text::of(match e {
        E::NoKeys => EN.bip_musig_no_keys,
        E::Infinity => EN.bip_musig_infinity,
        E::TweakOutOfRange => EN.bip_musig_tweak,
        E::Expression => EN.bip_musig_expression,
        E::Key => EN.bip_musig_key,
        E::Derivation => EN.bip_musig_derivation,
        E::IndexOutOfRange => EN.bip_index_out_of_range,
        E::PubNonce => EN.bip_musig_pub_nonce,
        E::AggNonce => EN.bip_musig_agg_nonce,
        E::SecNonce => EN.bip_musig_sec_nonce,
        E::PartialSig => EN.bip_musig_partial_sig,
        E::NotAParticipant => EN.bip_musig_not_participant,
        E::VerifyFailed => EN.bip_musig_verify,
    })
}

/// A wallet policy or descriptor refusal.
pub fn policy_error(e: &policy::Error) -> Text {
    use policy::Error as E;
    Text::of(match e {
        E::Template => EN.bip_policy_template,
        E::Key => EN.bip_key_not_xpub,
        E::RepeatedKey => EN.bip_policy_repeated_key,
        E::MixedNetwork => EN.bip_policy_mixed_network,
        E::Placeholder => EN.bip_policy_placeholder,
        E::Derivation => EN.bip_policy_derivation,
        E::Threshold => EN.bip_policy_threshold,
        E::Checksum => EN.bip_policy_checksum,
        E::IndexOutOfRange => EN.bip_index_out_of_range,
    })
}

/// A key-derivation refusal.
pub fn keys_error(e: &keys::Error) -> Text {
    Text::of(match e {
        keys::Error::IndexOutOfRange => EN.bip_index_out_of_range,
        keys::Error::NotMaster => EN.bip_keys_not_master,
    })
}

/// A codex32 refusal.
pub fn codex32_error(e: &codex32::Error) -> Text {
    use codex32::Error as E;
    match e {
        E::NotPrintable => Text::of(EN.bip_codex32_not_printable),
        E::MixedCase => Text::of(EN.bip_codex32_mixed_case),
        E::BadPrefix => Text::of(EN.bip_codex32_prefix),
        E::BadLength => Text::of(EN.bip_codex32_length),
        E::BadCharacter { position } => {
            Text::fill(EN.bip_codex32_character, &[&num(*position as u64 + 1)])
        }
        E::BadThreshold => Text::of(EN.bip_codex32_threshold),
        E::SecretIndexRequired => Text::of(EN.bip_codex32_secret_index),
        E::BadChecksum => Text::of(EN.bip_checksum_mismatch),
        E::WrongKind => Text::of(EN.bip_codex32_wrong_kind),
        E::DuplicateIndex => Text::of(EN.bip_codex32_duplicate_index),
        E::IdentifierMismatch => Text::of(EN.bip_codex32_identifier_mismatch),
        E::ThresholdMismatch => Text::of(EN.bip_codex32_threshold_mismatch),
        E::LengthMismatch => Text::of(EN.bip_codex32_length_mismatch),
        E::WrongShareCount => Text::of(EN.bip_codex32_share_count),
        E::BadShareIndex => Text::of(EN.bip_codex32_share_index),
        E::BadIdentifier => Text::of(EN.bip_codex32_identifier),
        E::BadSeedLength => Text::of(EN.bip_codex32_seed_length),
        E::BadSplit => Text::of(EN.bip_codex32_split),
        E::NotEnoughRandomness => Text::of(EN.bip_codex32_randomness),
    }
}

/// A SLIP-39 refusal.
pub fn slip39_error(e: &slip39::Error) -> Text {
    use slip39::Error as E;
    match e {
        E::InvalidWordCount => Text::of(EN.bip_slip39_word_count),
        E::UnknownWord { position } => {
            Text::fill(EN.bip_word_not_in_list, &[&num(u32::from(*position) + 1)])
        }
        E::BadChecksum => Text::of(EN.bip_checksum_mismatch),
        E::NonZeroPadding => Text::of(EN.bip_slip39_padding),
        E::InvalidSecretLength => Text::of(EN.bip_slip39_secret_length),
        E::NoShares => Text::of(EN.bip_slip39_no_shares),
        E::MismatchedIdentifiers => Text::of(EN.bip_slip39_identifiers),
        E::MismatchedExtendable => Text::of(EN.bip_slip39_extendable),
        E::MismatchedIterationExponents => Text::of(EN.bip_slip39_iteration_exponents),
        E::MismatchedGroupThresholds => Text::of(EN.bip_slip39_group_thresholds),
        E::MismatchedGroupCounts => Text::of(EN.bip_slip39_group_counts),
        E::MismatchedValueLengths => Text::of(EN.bip_slip39_value_lengths),
        E::MismatchedMemberThresholds => Text::of(EN.bip_slip39_member_thresholds),
        E::GroupThresholdExceedsCount => Text::of(EN.bip_slip39_group_threshold_exceeds),
        E::GroupIndexOutOfRange => Text::of(EN.bip_slip39_group_index),
        E::DuplicateMemberIndex => Text::of(EN.bip_slip39_duplicate_member),
        E::WrongGroupCount { needed, given } => {
            Text::fill(EN.bip_slip39_group_count, &[&num(*needed), &num(*given)])
        }
        E::WrongMemberCount {
            group_index,
            needed,
            given,
        } => Text::fill(
            EN.bip_slip39_member_count,
            &[
                &num(u32::from(*group_index) + 1),
                &num(*needed),
                &num(*given),
            ],
        ),
        E::BadDigest => Text::of(EN.bip_slip39_digest),
        E::PassphraseNotAscii => Text::of(EN.bip_passphrase_not_ascii),
        E::PassphraseTooLong => Text::fill(
            EN.bip_passphrase_too_long,
            &[&num(slip39::MAX_PASSPHRASE_BYTES as u64)],
        ),
        E::InvalidParameters => Text::of(EN.bip_slip39_parameters),
        E::IterationExponentTooHigh { exponent } => Text::fill(
            EN.bip_slip39_iteration_too_high,
            &[
                &num(*exponent),
                &num(u64::from(slip39::BASE_ITERATION_COUNT) << exponent),
                &num(slip39::MAX_READ_ITERATION_EXPONENT),
            ],
        ),
        E::OutputTooSmall => Text::of(EN.bip_slip39_output_too_small),
        E::MixedGroups => Text::of(EN.bip_slip39_mixed_groups),
        E::MemberIndexOutOfRange => Text::of(EN.bip_slip39_member_index),
        E::MemberIndexTaken => Text::of(EN.bip_slip39_member_taken),
        E::SingleMemberGroup => Text::of(EN.bip_slip39_single_member),
        E::NotExtendable => Text::of(EN.bip_slip39_not_extendable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_are_worded_with_their_data() {
        assert_eq!(
            bip39_error(&bip39::Error::UnknownWord { position: 4 }),
            "word 5 is not in the wordlist"
        );
        assert_eq!(
            slip39_error(&slip39::Error::WrongMemberCount {
                group_index: 0,
                needed: 3,
                given: 2
            }),
            "group 1 needs 3 shares, 2 given"
        );
        assert_eq!(
            slip39_error(&slip39::Error::IterationExponentTooHigh { exponent: 6 }),
            "iteration exponent 6 asks for 640000 PBKDF2 rounds; 5 is the most read"
        );
        assert_eq!(
            codex32_error(&codex32::Error::BadCharacter { position: 2 }),
            "character 3 is not a bech32 character"
        );
    }
}
