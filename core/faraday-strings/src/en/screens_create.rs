//! Faraday's English words for the screens create area.

macro_rules! words_screens_create {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- create (faraday-core/src/screens/create.rs) -----
    /// Text in `create_screen`. `{}`: `c.m`, `c.n`.
    create_threshold: "{} of {}",
    /// Text in `create_screen`. `{}`: `filled`, `c.slots.len()`.
    create_of_chosen: "{} of {} chosen",
    /// Text in `create_screen`. `{}`: `filled - k`, `c.slots.len()`, `k`.
    create_slots_progress: "{} of {} chosen · {} to come",
    /// Text in `create_screen`. `{}`: `w.name`.
    create_first_addresses: "{} · first addresses",
    /// Text in `create_screen`.
    create_not_made_yet: "Not made yet",
    /// Text in `create_screen`.
    create_home: "Home",
    /// Text in `create_screen`.
    create_wallets: "Wallets",
    /// Text in `create_screen`.
    create_make_a_new_wallet: "Make a new wallet",
    /// Text in `create_body`.
    create_start_over: "Start over",
    /// Text in `create_body`.
    create_signatures_needed: "Signatures needed",
    /// Text in `create_body`.
    create_keys: "Keys",
    /// Text in `create_body`. `{}`: `c.n`.
    create_all_keys_sign: "All {} keys sign together, as one signature on chain. Losing any one key loses the money: there is no spare.",
    /// Text in `create_body`. `{}`: `c.m`, `c.n`, `c.n - c.m`, `if c.n - c.m == 1 { EN.create_key_inline }…`, `c.m`.
    create_any_of_the_keys: "Any {} of the {} keys can spend. Losing {} {} still leaves the money spendable; {} keys together can take it.",
    /// Text in `create_body`.
    create_key_inline: "key",
    /// Text in `create_body`.
    create_keys_inline: "keys",
    /// Line in `create_body`.
    create_locked_line: "Locked in",
    /// Text in `create_body`.
    create_not_chosen: "Not chosen",
    /// Text in `create_body`. `{}`: `fpt`, `name`.
    create_loaded_here: "{} · {} · loaded here",
    /// Text in `create_body`. `{}`: `&t[1..9.min(t.len())]`.
    create_a_cosigner_s: "{} · a cosigner's xpub",
    /// Text in `create_body`.
    create_a_cosigner_adds: "A cosigner adds it later · waiting for its xpub file",
    /// Text in `create_body`.
    create_new_key: "Make a new key",
    /// Text in `create_body`. `{}`: `name`.
    create_slot_source: "From {}",
    /// Text in `create_body`.
    create_scan_its_xpub: "Scan its xpub",
    /// Text in `create_body`.
    create_someone_else: "Someone else adds it later",
    // NEW-WALLET §16 (F45): keys from a vault in Create; a new wallet
    // from one held.
    /// Text in `create_body`: a slot left for a cosigner's xpub at
    /// another account. `{}`: the slot's number, the account.
    create_needs_xpub_account: "Needs Key {}'s xpub at account {}",
    /// Text in `create_body`: the same, when the keys are at BIP-87's
    /// path. `{}`: the slot's number, the account.
    create_needs_xpub_bip87_account: "Needs Key {}'s xpub at BIP-87, account {}",
    /// Label in `account_row`: the Account row and its stepper, at the
    /// foot of Quorum or the head of Keys (F98).
    create_account: "Account",
    /// Button in `account_row` and `path_choice`: opens the Account row's
    /// stepper (F98), or a multisig path row's two paths.
    create_account_change: "Change",
    /// Segment in `path_choice`: a multisig's own path, BIP-48's, the
    /// default.
    create_path_bip48: "BIP-48",
    /// Tooltip on the info button after BIP-48 on the multisig path row: a second explanation on a working screen, by the owner's decision (2026-11-18, F287).
    create_path_tip: "BIP-48 is the standard path for a multisig wallet, the one most wallets use. BIP-87 is a newer path, the same for every script type; fewer wallets support it.",
    /// Segment in `path_choice`: legacy multisig's own path.
    create_path_bip45: "m/45'",
    /// Segment in `path_choice`: BIP-87's multisig path, which leaves the
    /// script to the descriptor.
    create_path_bip87: "BIP-87",
    /// The closed line of the card that carries the Account row, and the
    /// proposed name of a wallet made from another. `{}`: the card's
    /// line, the kind's or the wallet's name; the account.
    create_at_account: "{} · account {}",
    /// Label in `create_body`: Check's row of the made wallet's
    /// derivation paths (`docs/WALLETS.md` §14).
    create_path: "Path",
    /// Button in `create_body`: Check's Path row, back to the Account
    /// row (F98).
    create_path_change: "Change",
    /// Text in Check's Path row: a key that came with no origin.
    create_path_not_given: "no path given",
    /// Button on a wallet's card.
    wallets_new_wallet_button: "New wallet from this one",
    /// Text in `create_body`.
    create_clear: "Clear",
    /// Text in `create_body`.
    create_show_xpub_qr: "Show xpub QR",
    /// Text in `create_body`. `{}`: `app.medium.for_the()`.
    create_xpub_png: "Xpub PNG {}",
    /// Text in `create_body`. `{}`: `app.medium.for_the()`.
    create_xpub_file: "Xpub file {}",
    /// Text in `create_body`.
    create_add_a_passphrase: "Add a passphrase",
    /// Button in `create_body`: ends the card's making for New key.
    create_other_ways: "Other ways",
    /// Button in `create_body`: drops the words made on the Keys card.
    create_make_again: "Make again",
    /// Label in `create_body`.
    create_passphrase: "Passphrase",
    /// Label in `create_body`.
    create_passphrase_again: "Passphrase again",
    /// Button in `create_body`.
    create_lock_button: "Lock in",
    /// Text in `create_body`.
    create_key: "Key",
    /// The pick's question in the pane at the left of Keys.
    create_which_key: "Which key for this slot?",
    /// A key's line in the pick when it stands in another slot. `{}`: that slot's number.
    hub_in_slot: "In slot {}",
    /// Button on a filled slot: opens its pick again.
    create_change_key: "Change",
    /// Text in `create_body`.
    create_continue: "Continue",
    /// The Keys row, off until pressed: brings in the stick of a vault the person already has, before any key exists.
    create_use_existing_vault: "Use a vault I already have on a stick",
    /// The Keys card's line while the vault's stick is still to be brought in and unlocked.
    create_vault_to_unlock: "Its vault is to be unlocked before the keys",
    /// Bring in's summary in Make a new wallet's setup. `{}`: the sticks inserted.
    create_sticks_inserted: "{} inserted",
    /// A stick read for a vault the person already has, holding no vault.
    create_no_vault_on_stick: "No vault on this stick. A new vault is added on the backup plan",
    /// Text in `create_body`. `{}`: `app.session.wallets[old].name`.
    create_move_the_money: "Move the money from {}",
    /// Line in `create_body`.
    create_make_the_wallet: "Make the wallet first",
    /// Create's passphrase pair, a line under it.
    create_pass_kept: "Kept nowhere until it is backed up. A different passphrase opens a different wallet.",
    /// Button in `create_body`: makes this multisig slot's key from new words on the card.
    create_new_words: "New words",
        }
    };
}
