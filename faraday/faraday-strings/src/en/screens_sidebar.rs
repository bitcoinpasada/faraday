//! Faraday's English words for the screens sidebar area.

macro_rules! words_screens_sidebar {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- sidebar (faraday-core/src/screens/sidebar.rs) -----
    /// Text in `sidebar`.
    sidebar_home: "Home",
    /// The Check destination, right under Home.
    sidebar_check: "Check",
    /// Text in `sidebar`.
    sidebar_vaults: "Vaults",
    /// Text in `sidebar`.
    sidebar_tools: "Tools",
    /// Text in `sidebar`.
    sidebar_transfer: "Transfer",
    /// Text in `sidebar`.
    sidebar_settings: "Settings",
    /// Text in `sidebar`. `{}`: `v`.
    sidebar_open_n: "{} open",
    /// Row in `sidebar`.
    sidebar_guide: "Guide",
    /// Text in `sidebar`.
    sidebar_bring_in_button: "Bring in",
    /// Text in `sidebar`.
    sidebar_open: "Open",
    /// Text in `sidebar`.
    sidebar_work: "Work",
    /// Text in `sidebar`.
    sidebar_write_out: "Write out",
    /// Text in `sidebar`.
    sidebar_passphrase: "passphrase",
    /// Text in `more`. `{}`: `n - i`.
    sidebar_more: "+ {} more",
    /// Text in `sidebar`.
    sidebar_keys: "Keys",
    /// Text in `sidebar`. `{}`: `m`, `n`.
    sidebar_count: "{} of {}",
    /// Text in `sidebar`.
    sidebar_single_key: "single key",
    /// A wallet's count in the sidebar when a key not of its own spends it alone (F132).
    sidebar_another_key: "another key",
    /// Text in `sidebar`.
    sidebar_wallets: "Wallets",
    /// Line in `sidebar`.
    sidebar_files: "Files",
    /// Text in `sidebar`.
    sidebar_import_waiting: "import waiting",
    /// Text in `sidebar`.
    sidebar_none: "none",
    /// Text in `sidebar`. `{}`: `k`.
    sidebar_attached: "{} attached",
    /// Button in `sidebar`.
    sidebar_lock: "Lock",
    /// A flow in progress's name, in `sidebar` and the small Home
    /// (`in_progress`).
    sidebar_flow_create: "Make a new wallet",
    /// A flow in progress's name (`in_progress`).
    sidebar_flow_restore: "Restore a wallet",
    /// A flow in progress's name (`in_progress`).
    sidebar_flow_backup: "Back up a wallet",
    /// A flow in progress's name (`in_progress`).
    sidebar_flow_spend: "Spend",
    /// A flow in progress's name (`in_progress`).
    sidebar_flow_message: "Sign a message",
    /// A flow in progress's name (`in_progress`).
    sidebar_flow_vault: "Create a vault",
    /// A wallet's backup flow in progress (`in_progress`). `{}`: the
    /// wallet's name.
    sidebar_flow_backup_wallet: "Backup of {}",
        }
    };
}
