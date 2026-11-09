//! What faraday-bip refuses, in the strings file's words.
//!
//! faraday-bip's errors are kinds with their data and carry no wording;
//! each function here maps one kind to the sentence a person reads,
//! filled from that data. Every function is a lookup and a small fill.

use faraday_bip::{bip39, bsms, codex32, keys, musig, policy, slip39};

use crate::{EN, fill};

/// A BIP-39 refusal.
pub fn bip39_error(e: &bip39::Error) -> String {
    use bip39::Error as E;
    match e {
        E::InvalidWordCount => EN.bip_word_count.into(),
        E::InvalidEntropyLength => EN.bip_entropy_length.into(),
        E::UnknownWord { position } => fill(
            EN.bip_word_not_in_list,
            &[&(u32::from(*position) + 1).to_string()],
        ),
        E::BadChecksum => EN.bip_checksum_mismatch.into(),
        E::PassphraseNotAscii => EN.bip_passphrase_not_ascii.into(),
        E::PassphraseTooLong => fill(
            EN.bip_passphrase_too_long,
            &[&bip39::MAX_PASSPHRASE_BYTES.to_string()],
        ),
    }
}

/// A BIP-129 (BSMS) refusal.
pub fn bsms_error(e: &bsms::Error) -> String {
    use bsms::Error as E;
    match e {
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
    }
    .into()
}

/// A MuSig2 refusal.
pub fn musig_error(e: &musig::Error) -> String {
    use musig::Error as E;
    match e {
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
    }
    .into()
}

/// A wallet policy or descriptor refusal.
pub fn policy_error(e: &policy::Error) -> String {
    use policy::Error as E;
    match e {
        E::Template => EN.bip_policy_template,
        E::Key => EN.bip_key_not_xpub,
        E::Placeholder => EN.bip_policy_placeholder,
        E::Derivation => EN.bip_policy_derivation,
        E::Threshold => EN.bip_policy_threshold,
        E::Checksum => EN.bip_policy_checksum,
        E::IndexOutOfRange => EN.bip_index_out_of_range,
    }
    .into()
}

/// A key-derivation refusal.
pub fn keys_error(e: &keys::Error) -> String {
    match e {
        keys::Error::IndexOutOfRange => EN.bip_index_out_of_range,
        keys::Error::NotMaster => EN.bip_keys_not_master,
    }
    .into()
}

/// A codex32 refusal.
pub fn codex32_error(e: &codex32::Error) -> String {
    use codex32::Error as E;
    match e {
        E::NotPrintable => EN.bip_codex32_not_printable.into(),
        E::MixedCase => EN.bip_codex32_mixed_case.into(),
        E::BadPrefix => EN.bip_codex32_prefix.into(),
        E::BadLength => EN.bip_codex32_length.into(),
        E::BadCharacter { position } => {
            fill(EN.bip_codex32_character, &[&(*position + 1).to_string()])
        }
        E::BadThreshold => EN.bip_codex32_threshold.into(),
        E::SecretIndexRequired => EN.bip_codex32_secret_index.into(),
        E::BadChecksum => EN.bip_checksum_mismatch.into(),
        E::WrongKind => EN.bip_codex32_wrong_kind.into(),
        E::DuplicateIndex => EN.bip_codex32_duplicate_index.into(),
        E::IdentifierMismatch => EN.bip_codex32_identifier_mismatch.into(),
        E::ThresholdMismatch => EN.bip_codex32_threshold_mismatch.into(),
        E::LengthMismatch => EN.bip_codex32_length_mismatch.into(),
        E::WrongShareCount => EN.bip_codex32_share_count.into(),
        E::BadShareIndex => EN.bip_codex32_share_index.into(),
        E::BadIdentifier => EN.bip_codex32_identifier.into(),
        E::BadSeedLength => EN.bip_codex32_seed_length.into(),
        E::BadSplit => EN.bip_codex32_split.into(),
        E::NotEnoughRandomness => EN.bip_codex32_randomness.into(),
    }
}

/// A SLIP-39 refusal.
pub fn slip39_error(e: &slip39::Error) -> String {
    use slip39::Error as E;
    match e {
        E::InvalidWordCount => EN.bip_slip39_word_count.into(),
        E::UnknownWord { position } => fill(
            EN.bip_word_not_in_list,
            &[&(u32::from(*position) + 1).to_string()],
        ),
        E::BadChecksum => EN.bip_checksum_mismatch.into(),
        E::NonZeroPadding => EN.bip_slip39_padding.into(),
        E::InvalidSecretLength => EN.bip_slip39_secret_length.into(),
        E::NoShares => EN.bip_slip39_no_shares.into(),
        E::MismatchedIdentifiers => EN.bip_slip39_identifiers.into(),
        E::MismatchedExtendable => EN.bip_slip39_extendable.into(),
        E::MismatchedIterationExponents => EN.bip_slip39_iteration_exponents.into(),
        E::MismatchedGroupThresholds => EN.bip_slip39_group_thresholds.into(),
        E::MismatchedGroupCounts => EN.bip_slip39_group_counts.into(),
        E::MismatchedValueLengths => EN.bip_slip39_value_lengths.into(),
        E::MismatchedMemberThresholds => EN.bip_slip39_member_thresholds.into(),
        E::GroupThresholdExceedsCount => EN.bip_slip39_group_threshold_exceeds.into(),
        E::GroupIndexOutOfRange => EN.bip_slip39_group_index.into(),
        E::DuplicateMemberIndex => EN.bip_slip39_duplicate_member.into(),
        E::WrongGroupCount { needed, given } => fill(
            EN.bip_slip39_group_count,
            &[&needed.to_string(), &given.to_string()],
        ),
        E::WrongMemberCount {
            group_index,
            needed,
            given,
        } => fill(
            EN.bip_slip39_member_count,
            &[
                &(u32::from(*group_index) + 1).to_string(),
                &needed.to_string(),
                &given.to_string(),
            ],
        ),
        E::BadDigest => EN.bip_slip39_digest.into(),
        E::PassphraseNotAscii => EN.bip_passphrase_not_ascii.into(),
        E::PassphraseTooLong => fill(
            EN.bip_passphrase_too_long,
            &[&slip39::MAX_PASSPHRASE_BYTES.to_string()],
        ),
        E::InvalidParameters => EN.bip_slip39_parameters.into(),
        E::IterationExponentTooHigh { exponent } => fill(
            EN.bip_slip39_iteration_too_high,
            &[
                &exponent.to_string(),
                &(u64::from(slip39::BASE_ITERATION_COUNT) << exponent).to_string(),
                &slip39::MAX_READ_ITERATION_EXPONENT.to_string(),
            ],
        ),
        E::OutputTooSmall => EN.bip_slip39_output_too_small.into(),
        E::MixedGroups => EN.bip_slip39_mixed_groups.into(),
        E::MemberIndexOutOfRange => EN.bip_slip39_member_index.into(),
        E::MemberIndexTaken => EN.bip_slip39_member_taken.into(),
        E::SingleMemberGroup => EN.bip_slip39_single_member.into(),
        E::NotExtendable => EN.bip_slip39_not_extendable.into(),
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
