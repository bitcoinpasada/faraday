//! Faraday's English words for the bip refusal area.

macro_rules! words_bip_refusal {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- Why faraday-bip refuses what it reads or makes, worded by `faraday_strings::bip` (docs/STRUCTURE-PASS.md §7) -----
    /// A BIP-39, SLIP-39 or codex32 string whose checksum is wrong.
    bip_checksum_mismatch: "checksum does not match",
    /// A derivation index of 2^31 or more.
    bip_index_out_of_range: "index must be below 2^31",
    /// A BIP-39 mnemonic of another length.
    bip_word_count: "word count must be 12, 15, 18, 21 or 24",
    /// A word that is not in the wordlist; its position.
    bip_word_not_in_list: "word {} is not in the wordlist",
    /// A passphrase with a character outside printable ASCII.
    bip_passphrase_not_ascii: "passphrase must be printable ASCII",
    /// A passphrase over the limit; the limit.
    bip_passphrase_too_long: "passphrase exceeds {} bytes",
    /// BIP-39 entropy of another length.
    bip_entropy_length: "entropy must be 16, 20, 24, 28 or 32 bytes",
    /// A BSMS record without its version line.
    bip_bsms_version: "the first line is not BSMS 1.0",
    /// A BSMS record with missing lines.
    bip_bsms_lines: "the record does not have the lines it needs",
    /// A BSMS record with a bad token.
    bip_bsms_token: "the token is not 00 or a hex nonce",
    /// An encrypted BSMS record.
    bip_bsms_encrypted: "this record is encrypted",
    /// A key that is not written [fingerprint/path]xpub.
    bip_key_not_xpub: "key is not [fingerprint/path]xpub",
    /// A BSMS description over 80 characters.
    bip_bsms_description: "the description is longer than 80 characters",
    /// A BSMS signature that is not the key's.
    bip_bsms_signature: "the signature is not this key's",
    /// A BSMS record with other derivation paths.
    bip_bsms_paths: "the derivation paths are not /0/*,/1/*",
    /// A BSMS record whose fourth line is not an address.
    bip_bsms_address: "the fourth line is not an address",
    /// A BSMS first address that does not match the wallet.
    bip_bsms_first_address: "this is not the wallet's first address",
    /// MuSig2 aggregation with no keys.
    bip_musig_no_keys: "key aggregation needs at least one key",
    /// MuSig2 aggregate key at infinity.
    bip_musig_infinity: "the aggregate key is the point at infinity",
    /// A MuSig2 tweak out of range.
    bip_musig_tweak: "the tweak must be less than the curve order",
    /// A musig() expression a tr() cannot hold.
    bip_musig_expression: "not a musig() expression a tr() descriptor can hold",
    /// A MuSig2 participant key that is not public.
    bip_musig_key: "participant key is not a public key or an extended public key",
    /// A MuSig2 derivation that is hardened.
    bip_musig_derivation: "derivation must be unhardened, with the wildcard last",
    /// A MuSig2 public nonce that is malformed.
    bip_musig_pub_nonce: "public nonce is not two compressed points",
    /// A MuSig2 aggregate nonce that is malformed.
    bip_musig_agg_nonce: "aggregate nonce is not two points",
    /// A MuSig2 secret nonce not usable for the key.
    bip_musig_sec_nonce: "secret nonce is not a usable pair of scalars for this key",
    /// A MuSig2 partial signature out of range.
    bip_musig_partial_sig: "partial signature must be less than the curve order",
    /// A MuSig2 signer who is not a participant.
    bip_musig_not_participant: "the signer's key is not one of the session's keys",
    /// A MuSig2 partial signature that fails verification.
    bip_musig_verify: "partial signature does not verify",
    /// A descriptor template the wallet policy does not support.
    bip_policy_template: "unsupported descriptor template",
    /// A wallet policy whose key placeholders are out of order.
    bip_policy_placeholder: "key placeholders are not @0, @1, … in order",
    /// A wallet policy key with another derivation.
    bip_policy_derivation: "derivation must be /** or /<0;1>/*",
    /// A wallet policy threshold out of range.
    bip_policy_threshold: "threshold must be between 1 and the number of keys",
    /// A wallet that uses one key more than once.
    bip_policy_repeated_key: "a key is used more than once",
    /// A wallet whose keys are of more than one network.
    bip_policy_mixed_network: "keys are of more than one network",
    /// A descriptor whose checksum is wrong.
    bip_policy_checksum: "descriptor checksum does not match",
    /// An extended private key that is not a master key.
    bip_keys_not_master: "extended private key is not a master key",
    /// A codex32 string with a non-ASCII character.
    bip_codex32_not_printable: "string holds a character outside printable ASCII",
    /// A codex32 string in mixed case.
    bip_codex32_mixed_case: "string mixes upper and lower case",
    /// A codex32 string without the ms1 prefix.
    bip_codex32_prefix: "string does not start with ms1",
    /// A codex32 string of a length that is no master seed.
    bip_codex32_length: "string length does not encode a master seed",
    /// A codex32 character outside bech32; its position.
    bip_codex32_character: "character {} is not a bech32 character",
    /// A codex32 threshold out of range.
    bip_codex32_threshold: "threshold is not 0 or 2 to 9",
    /// A codex32 threshold 0 without index s.
    bip_codex32_secret_index: "threshold 0 needs the share index s",
    /// A codex32 string of the wrong kind.
    bip_codex32_wrong_kind: "string is the wrong kind for this operation",
    /// Two codex32 strings with one share index.
    bip_codex32_duplicate_index: "two strings carry the same share index",
    /// A codex32 set with differing identifiers.
    bip_codex32_identifier_mismatch: "identifiers differ across the set",
    /// A codex32 set with differing thresholds.
    bip_codex32_threshold_mismatch: "thresholds differ across the set",
    /// A codex32 set with differing lengths.
    bip_codex32_length_mismatch: "lengths differ across the set",
    /// A codex32 set that is not the threshold in size.
    bip_codex32_share_count: "the number of shares is not the threshold",
    /// A codex32 share index taken or outside the alphabet.
    bip_codex32_share_index: "share index is not free and in the alphabet",
    /// A codex32 identifier of the wrong form.
    bip_codex32_identifier: "identifier is not four bech32 characters",
    /// A codex32 master seed of another length.
    bip_codex32_seed_length: "master seed must be 16, 20, 24, 28, 32 or 64 bytes",
    /// A codex32 split with a threshold out of range.
    bip_codex32_split: "threshold must be 2 to 9 and at most the share count",
    /// A codex32 split given too few random bytes.
    bip_codex32_randomness: "too few random bytes for the shares asked",
    /// A SLIP-39 share of another length.
    bip_slip39_word_count: "a share is 20 or 33 words",
    /// A SLIP-39 share with padding bits set.
    bip_slip39_padding: "padding bits are not zero",
    /// A SLIP-39 share value of a bad length.
    bip_slip39_secret_length: "the share value must be an even number of bytes, at least 16",
    /// SLIP-39 recovery with no shares.
    bip_slip39_no_shares: "no shares given",
    /// SLIP-39 shares from different backups.
    bip_slip39_identifiers: "the shares are from different backups",
    /// SLIP-39 shares with differing extendable flags.
    bip_slip39_extendable: "the shares disagree about the extendable flag",
    /// SLIP-39 shares with differing iteration exponents.
    bip_slip39_iteration_exponents: "the shares disagree about the iteration exponent",
    /// SLIP-39 shares with differing group thresholds.
    bip_slip39_group_thresholds: "the shares disagree about how many groups must be present",
    /// SLIP-39 shares with differing group counts.
    bip_slip39_group_counts: "the shares disagree about how many groups there are",
    /// SLIP-39 shares with differing value lengths.
    bip_slip39_value_lengths: "the shares disagree about the secret's length",
    /// SLIP-39 shares of one group with differing thresholds.
    bip_slip39_member_thresholds: "one group's shares disagree about how many of them must be present",
    /// A SLIP-39 group threshold above the group count.
    bip_slip39_group_threshold_exceeds: "more groups must be present than the backup has",
    /// A SLIP-39 group index out of range.
    bip_slip39_group_index: "a group index is outside the group count",
    /// SLIP-39 shares of one group that repeat.
    bip_slip39_duplicate_member: "two shares of one group are the same share",
    /// SLIP-39 recovery with too few groups; needed, given.
    bip_slip39_group_count: "{} groups must be present, {} given",
    /// SLIP-39 recovery with too few shares in a group; group, needed, given.
    bip_slip39_member_count: "group {} needs {} shares, {} given",
    /// SLIP-39 shares whose digest fails.
    bip_slip39_digest: "the shares do not belong together",
    /// SLIP-39 parameters out of range.
    bip_slip39_parameters: "thresholds and counts are 1 to 16",
    /// A SLIP-39 iteration exponent above what is read; exponent, rounds, most.
    bip_slip39_iteration_too_high: "iteration exponent {} asks for {} PBKDF2 rounds; {} is the most read",
    /// SLIP-39 output buffer too small.
    bip_slip39_output_too_small: "not enough room for the shares",
    /// SLIP-39 shares of one group needed, several groups' given.
    bip_slip39_mixed_groups: "the shares are from more than one group",
    /// A SLIP-39 member asked for outside the sixteen a group holds.
    bip_slip39_member_index: "a group's shares are 1 to 16",
    /// A SLIP-39 member asked for that is among the shares given.
    bip_slip39_member_taken: "that share is among those given",
    /// A SLIP-39 share asked of a group of threshold 1.
    bip_slip39_single_member: "a group of threshold 1 has one share only",
    /// A SLIP-39 new identifier asked for a set whose extendable flag is clear.
    bip_slip39_not_extendable: "a set that is not extendable keeps its identifier",
        }
    };
}
