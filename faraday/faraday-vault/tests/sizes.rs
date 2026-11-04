//! `faraday-codec` tells a vault by its length, for the programs that
//! list files without opening them: every length Faraday writes counts.

use faraday_codec::vault_file;

#[test]
fn every_size_gives_a_file_the_codec_tells_as_a_vault() {
    for s in faraday_vault::file::SIZES {
        assert!(vault_file::is_vault_len(
            faraday_vault::file::file_len(s) as u64
        ));
    }
}
