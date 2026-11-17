//! Faraday's English words for the gpg area.

macro_rules! words_gpg {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- gpg (faraday-core/src/gpg.rs) -----
    /// Error in `clock`.
    gpg_this_computer_s: "This computer's clock is not known",
    /// Error in `gpg_make`.
    gpg_a_key_needs_a_name: "A key needs a name or an email",
    /// Text in `gpg_code`. `{}`: `faraday_pgp::fingerprint_text(&key.fi…`.
    gpg_fingerprint: "Fingerprint {}",
    /// Error in `gpg_code`.
    gpg_no_gpg_key_is_selected: "No GPG key is selected",
    /// Text in `gpg_code`.
    gpg_gpg_public_key: "GPG public key",
    /// Text in `gpg_code`.
    gpg_public_checks_signatures: "Public: checks signatures, signs nothing",
    /// Text in `gpg_code`.
    gpg_gpg_revocation: "GPG revocation certificate",
    /// Text in `gpg_code`.
    gpg_revocation_warning: "Whoever has it can revoke the key",
    /// Error in `gpg_code`.
    gpg_file_gone: "That file is no longer in Files",
    /// Text in `gpg_code`. `{}`: `item.name`.
    gpg_gpg_signature: "GPG signature · {}",
    /// Text in `gpg_code`. `{}`: `item.name`.
    gpg_signs: "Signs {}",
    /// Text in `gpg_code`. `{}`: `who`.
    gpg_signer_line: "By {}",
    /// Error in `gpg_code`.
    gpg_not_a_gpg_file: "Not a GPG file",
    /// Toast in `gpg_renew`. `{}`: `name`, `self.medium.for_the()`.
    gpg_renewed_waits: "Renewed · {} waits {}",
    /// Text in `gpg_sign_ticked`. `{}`: `self.medium.for_the()`.
    gpg_1_signature_waits: "1 signature waits {}",
    /// Text in `gpg_sign_ticked`. `{}`: `n`, `self.medium.for_the()`.
    gpg_signatures_wait: "{} signatures wait {}",
    /// Text in `shown`.
    gpg_never: "Never",
    /// A span of one second, as GnuPG says how far in the future a key was made.
    gpg_second: "{} second",
    /// A span of seconds, as GnuPG says it.
    gpg_seconds: "{} seconds",
    /// A span of one day.
    gpg_day: "{} day",
    /// A span of days.
    gpg_days: "{} days",
    /// Error in `usable`. `{}`: the date the key expired.
    gpg_expired: "Unusable secret key: it expired on {}",
    /// Error in `usable`. `{}`: the date.
    gpg_revoked: "Unusable secret key: it was revoked on {}",
    /// Error in `usable`, GnuPG's words. `{}`: the key's ID, the span.
    gpg_future: "Key {} was created {} in the future (time warp or clock problem)",
    /// Error in `gpg_sign_ticked`, `gpg_export`. `{}`: the file's name.
    gpg_exists: "File exists: {}",
    /// Error in `gpg_sign_ticked`. `{}`: the file's name.
    gpg_changed: "{} changed after it was ticked, so it was not signed",
    /// Error in `gpg_renew`.
    gpg_choose_the_term: "Choose the new term first",
    /// Error in `gpg_renew`, after GnuPG's.
    gpg_time_conflict: "Time conflict: the clock is before the key's last signature",
    /// Error in `gpg_export_revocation`.
    gpg_no_revocation_kept: "No revocation certificate is kept with this key yet. Set the time and open the key again",
    /// Toast in `gpg_mark_revoked`.
    gpg_marked_revoked: "The key is marked revoked",
    // ----- A GPG key restored from its paperkey (docs/GPG-PAPERKEY.md §7.3), faraday-core -----
    /// Vault contents' GPG kind, beside Make a key.
    gpg_restore_action: "Restore from a paperkey",
    /// A public key's ⋮ in Files.
    gpg_restore_files_action: "Restore from its paperkey",
    /// The sheet's title and the flow's heading.
    gpg_restore_title: "Restore a GPG key",
    /// The sheet with no public key in Files.
    gpg_restore_none: "Copy its public key in on a stick visit first.",
    /// The sheet's button and the title of the scanner it opens.
    gpg_restore_scan: "Scan the public key",
    /// Step 4's button, while no line is checked, and the title of the
    /// scanner it opens.
    gpg_restore_scan_paperkey: "Scan the paperkey",
    /// The paperkey scanner, on a code that is neither of paperkey's forms.
    gpg_restore_not_paperkey: "Not a paperkey",
    /// The paperkey scanner, on a code that ends before the whole data's
    /// CRC: paperkey's words.
    gpg_restore_crc_missing: "CRC of secret is missing",
    /// The paperkey scanner, on a code with a line after the whole data's
    /// CRC.
    gpg_restore_after: "Line {} comes after the CRC of secret",
    /// The code box of line 1 at the top of the plan card.
    gpg_restore_line_one: "Line 1 of its paperkey",
    /// Step 4's card title.
    gpg_restore_card: "Type the paperkey",
    /// A line while the count is not known.
    gpg_restore_line: "Line {}",
    /// A line once the count is known.
    gpg_restore_line_of: "Line {} of {}",
    /// A line checked.
    gpg_restore_checked: "Line {} · checked",
    /// The button that ends a line.
    gpg_restore_next: "Next line",
    /// A checked line's ⋮.
    gpg_restore_again: "Type line {} again",
    /// Step 4's button once every line is checked.
    gpg_restore_button: "Restore the key",
    /// The Result, a row per vault.
    gpg_restore_into: "Restored into {}",
    /// The Result where signatures by other keys were set aside.
    gpg_restore_set_aside: "{} signatures by other keys are not kept",
    /// A character typed that a line does not take.
    gpg_restore_chars: "Only 0–9, A–F, spaces and the line's number",
    /// The number typed before the colon is another line's.
    gpg_restore_number: "This is line {}, not line {}",
    /// A line's CRC does not match.
    gpg_restore_crc: "CRC on line {} does not match",
    /// The whole data's CRC does not match.
    gpg_restore_crc_all: "CRC of secret does not match",
    /// Line 1 names another key.
    gpg_restore_other_key: "This paperkey is for key {}",
    /// The second entry names another subkey.
    gpg_restore_other_subkey: "This paperkey's subkey is {}",
    /// More or fewer keys than the certificate.
    gpg_restore_count: "The paperkey holds {} keys; the public key has 2",
    /// A paperkey format other than paperkey's 0.
    gpg_restore_format: "This paperkey is in format {}; paperkey writes format 0",
    /// A secret part with a passphrase.
    gpg_restore_protected: "Protected by a passphrase: Faraday restores keys with none",
    /// A secret part that does not read.
    gpg_restore_unreadable: "The paperkey's secret does not read",
    /// A secret that does not make the public key.
    gpg_restore_mismatch: "The secret does not match the public key",
    /// A signature of the key's that Faraday does not make the same.
    gpg_restore_not_ours: "Not a certificate Faraday made for this key",
    /// A key of another algorithm, named as `gpg --list-keys` names it.
    gpg_restore_algorithm: "{} keys are not restored here: Faraday's are Ed25519",
    /// A key of another OpenPGP version.
    gpg_restore_version: "Version {} keys are not restored here",
    /// A key of another shape.
    gpg_restore_shape: "Not a key of Faraday's shape: an Ed25519 key with one Ed25519 signing subkey",
    /// A certificate with no user ID.
    gpg_restore_no_user_id: "The public key has no user ID",
    /// A file that is not one armoured public key block.
    gpg_restore_not_public: "Not a public key Faraday reads",
    /// The key is in an open vault already.
    gpg_restore_in_vault: "This key is in {}",
    /// Button in `years_row`.
    vault_3_years: "3 years",
    /// Button and heading on a GPG key's page.
    vault_make_a_revocation: "Make a revocation",
    /// Button on a GPG key's page.
    vault_export_revocation: "Export the kept revocation",
    /// Label of the reason buttons.
    vault_reason: "Reason",
    /// GnuPG's menu entry 0.
    vault_reason_0: "No reason specified",
    /// GnuPG's menu entry 1.
    vault_reason_1: "Key has been compromised",
    /// GnuPG's menu entry 2.
    vault_reason_2: "Key is superseded",
    /// GnuPG's menu entry 3.
    vault_reason_3: "Key is no longer used",
    /// Button of the revocation form.
    vault_make_the_revocation: "Make the revocation",
    /// Label of the revocation form's field.
    vaults_description: "Description (optional)",
    /// Label on a GPG key's page.
    vault_status: "Status",
    /// Row on a GPG key's page. `{}`: the date.
    vault_revoked_date: "Revoked on {}",
    /// Row on a GPG key's page. `{}`: the date.
    vault_kept_with_the_key: "Kept with the key · made {}",
    /// Row on a GPG key's page.
    vault_none_kept: "None kept yet",
    /// Line on a GPG key's page.
    vault_term_never: "Term now: does not expire",
    /// Line on a GPG key's page. `{}`: the date.
    vault_term_until: "Term now: expires {}",
    /// Hold button on a GPG key's page.
    vault_hold_to_mark_revoked: "Hold to mark the key revoked",
    /// Line on the signing page. `{}`: the IDs, the date.
    vault_signing_key_ids: "Signing subkey ID {} · main key ID {} · created {}",
    /// Hold button on Sign with {key}.
    vault_hold_to_sign_1_file: "Hold to sign 1 file",
    /// Hold button on Sign with {key}. `{}`: the count.
    vault_hold_to_sign_files: "Hold to sign {} files",
    /// Title of the sheet Sign opens on the signing page (F63). `{}`:
    /// the key's user ID.
    vault_sign_sheet_title: "Sign with {}",
    /// Row on Sign with {key}: the time the signatures will carry.
    vault_signature_time: "Signature time",
        }
    };
}
