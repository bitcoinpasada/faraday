//! Faraday's English words for the screens visit area.

macro_rules! words_screens_visit {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- visit (faraday-core/src/screens/visit.rs) -----
    /// Text in `visit_compact`.
    visit_unselect_all: "Unselect all",
    /// Line in `visit_compact`.
    visit_select_all: "Select all",
    /// Text in `visit`.
    visit_in_faraday: "In Faraday",
    /// Text in `visit`.
    visit_public: "Public",
    /// Text in `visit`. `{}`: `app.medium.for_the()`.
    visit_nothing_waits: "Nothing waits {}",
    /// Text in `visit`. `{}`: `nout`, `if nout == 1 { EN.visit_file } else {…`.
    visit_write: "Write {} {}",
    /// Text in `visit`.
    visit_file_too_large: "Larger than 18 MB: not read",
    /// Text in `visit`.
    visit_file_memory_unknown: "Not read: this computer does not say how much memory it has free",
    /// Text in `visit`.
    visit_settings: "Settings",
    /// Text in `visit`: a file asked for and not yet read. `{}`: its size.
    visit_being_read: "Being read · {}",
    /// A toast on the frame drawn before a press's pictures and PDFs are
    /// made (`docs/SPEED.md` §9).
    working_files: "Making the files",
    /// A toast on the frame drawn before an aezeed is stretched for its
    /// node key.
    working_node_key: "Working out the node key",
    /// A toast on the frame drawn before My seed makes my wallet stretches
    /// a key for a passphrase or for Keep this key.
    working_seed_key: "Working out the key",
    /// Toast while an aezeed is read for Convert.
    working_aezeed: "Reading the aezeed",
    /// The line of the job that signs a transaction.
    working_signing: "Signing the transaction",
    /// The line of the job that adds the signatures of a file to a PSBT.
    working_collecting: "Adding the signatures",
    /// The line of the job that hashes a large file in Check a timestamp.
    working_hashing: "Hashing the file",
    /// The line of the job that compares a key, an address or a wallet with a seed's.
    working_seed_compare: "Comparing",
    /// The line of the job that reads a Bitcoin Core wallet.
    working_core_wallet: "Reading the wallet",
    /// The line of the job that reads a PSBT and checks it.
    working_reading_psbt: "Reading the transaction",
    /// The line of the job that loads the ticked samples.
    working_samples: "Loading the samples",
    /// The line of the job that loads the seeds ticked on the Load sheet.
    working_load_seeds: "Loading the seeds",
    /// The line of the job that fingerprints a vault's marked records of words.
    working_check_marks: "Checking the keys",
    /// The line of the job that opens a vault.
    working_vault_open: "Unlocking the vault",
    /// The line of the job that makes a vault.
    working_vault_make: "Making the vault",
    /// Text in `visit`. `{}`: `app.medium.noun()`.
    visit_no_files: "No files on this {}",
    /// Button in `visit`.
    visit_delete_files: "Delete files",
    /// Line in `visit`, error tone: the stick holds secret files this
    /// session wrote or read. `{}`: the stick's label, the file names.
    unprotected_one: "{} holds 1 unprotected secret: {}. Do not plug it into an online computer.",
    /// Line in `visit`, error tone. `{}`: the stick's label, how many,
    /// the file names.
    unprotected_many: "{} holds {} unprotected secrets: {}. Do not plug it into an online computer.",
    /// Button in `visit`: Delete files with those secrets chosen.
    unprotected_delete: "Delete unprotected secrets",
    /// Text in `visit`.
    visit_import_and_load: "Import and load",
    /// Button on a stick visit that holds one PSBT. `{}`: its name.
    visit_import_and_sign: "Import and sign {}",
    /// Text in `visit`. `{}`: `nin`, `if nin == 1 { EN.visit_file } else { …`.
    visit_import: "Import {} {}",
    /// Text in `visit`.
    visit_file: "file",
    /// Text in `visit`.
    visit_files: "files",
    // ----- visit_inbox_line (faraday-core/src/screens/visit.rs) -----
    /// Text in `visit_inbox_line`. `{}`: `kind_line(FileKind::Other, item.bytes…`.
    visit_inbox_line_png_no_qr: "PNG, no QR code read · {}",
    /// Text in `visit_inbox_line`. `{}`: `kind_line(k, item.bytes.len())`.
    visit_inbox_line_png: "PNG · {}",
    // ----- visit_inbox_tag (faraday-core/src/screens/visit.rs) -----
    /// Text in `visit_inbox_tag`.
    visit_inbox_tag_sealed: "Sealed",
    /// Text in `visit_inbox_tag`.
    visit_inbox_tag_public: "Public",
    /// Text in `visit_inbox_tag`.
    visit_inbox_tag_may: "May be a secret",
    /// Text in `visit_inbox_tag`.
    visit_inbox_tag_secret: "Secret",
    // ----- the Inbox summary's fold (faraday-core/src/screens/files.rs) -----
    /// Text on the folded Inbox summary: opens it.
    visit_summary_show: "Show",
    /// Text on the open long Inbox summary: folds it.
    visit_summary_hide: "Hide",
    // ----- visit_select_all (faraday-core/src/screens/visit.rs) -----
    /// Text in `visit_select_all`.
    visit_select_all_select: "Select all",
    // ----- visit_unselect_all (faraday-core/src/screens/visit.rs) -----
    /// Text in `visit_unselect_all`.
    visit_unselect_all_unselect: "Unselect all",
    // ----- visit_list_bar (faraday-core/src/screens/visit.rs) -----
    /// Text in `visit_list_bar`. `{}`: `(below / ROW).ceil() as usize`.
    visit_list_bar_more: "{} more · scroll to see them",
        }
    };
}
