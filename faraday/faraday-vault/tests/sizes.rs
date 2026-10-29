//! `faraday-vault` keeps the vault sizes as a literal list
//! (`docs/crypto/check.py` reads it there); `faraday-codec` keeps them
//! for the programs that tell a vault by its length. They are the same.

use faraday_codec::vault_file;

#[test]
fn the_vault_and_the_codec_agree_on_the_sizes_and_the_file_length() {
    assert_eq!(faraday_vault::SLOT_SIZES, vault_file::SLOT_SIZES);
    for s in faraday_vault::SLOT_SIZES {
        assert_eq!(faraday_vault::file_len(s), vault_file::file_len(s));
        assert!(vault_file::is_vault_len(faraday_vault::file_len(s) as u64));
    }
}
