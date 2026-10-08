//! Widget ids. Every screen uses a distinct range so that an [`Action`]
//! can be dispatched on the id alone; ranges with a `_BASE` suffix are
//! offset by an index. Tests use these with
//! [`OpenSigner::rect_of`](crate::OpenSigner::rect_of) to compute tap
//! positions.
//!
//! [`Action`]: osk_ui::Action

pub use osk_ui::Id;

/// The scroll region of the current screen. One screen is on the display
/// at a time, so the application and [`osk_ui::screens`] name the same
/// region and a scroll offset survives a screen being rebuilt from the
/// design system.
pub const SCROLL: Id = osk_ui::screens::SCROLL;
/// The back control in every non-Home frame header.
pub const BACK: Id = Id(2);
/// The app bar's eye on a screen with a secret panel (Backup and Create
/// words, SeedQR, CompactSeedQR, Explore words, and word entry's "words
/// so far"): reveals the panel for 30 s without a finger on it
/// (`docs/DESIGN.md` §4.10).
pub const SECRET_EYE: Id = Id(15);
/// The app bar's info button, on a screen that has a Learn page
/// (`docs/PLANNING.md` §16.105). It sits in the same slot as the eye and
/// is drawn only where there is no eye, so one screen has one of the two
/// or neither.
pub const INFO: Id = Id(16);

/// The six tiles of the launcher on `Small` and `Mobile`, and the
/// `wide` sidebar's rows on every `Wide` screen, in
/// [`crate::HUB_TILES`] order. No screen draws both, so one area is one
/// id wherever its control is.
pub const HOME_TILE_BASE: u32 = 20;
/// Home: the Scan tile, which is [`HOME_TILE_BASE`] plus two. Scan is
/// the one way in for anything read, so it has a name of its own.
pub const HOME_SCAN: Id = Id(22);
/// `wide` sidebar: the whole sidebar, so a layout test can assert that
/// it is in the same place on every `Wide` screen. The same rectangle
/// [`osk_ui::screens`] reserves, so a rebuilt screen and one still
/// drawn from the older composites name one sidebar.
pub const SIDEBAR: Id = osk_ui::screens::SIDEBAR;

/// The text field of an entry or a pad screen
/// ([`osk_ui::screens::FIELD`]).
pub const FIELD: Id = osk_ui::screens::FIELD;
/// The rectangle a pad screen reserves for its pad
/// ([`osk_ui::screens::PAD_SLOT`]).
pub const PAD_SLOT: Id = osk_ui::screens::PAD_SLOT;
/// The pane a screen built from [`osk_ui::screens`] draws
/// ([`osk_ui::screens::SCREEN`]). A layout test asserts every rebuilt
/// screen has one.
pub const SCREEN: Id = osk_ui::screens::SCREEN;
/// The square a screen draws its QR in, so a layout test can assert it
/// is the class's side ([`osk_ui::screens::CODE`], §4.9).
pub const CODE: Id = osk_ui::screens::CODE;
/// The square a scanner screen draws its preview in
/// ([`osk_ui::screens::VIEWFINDER`]).
pub const VIEWFINDER: Id = osk_ui::screens::VIEWFINDER;
/// The state line inside that square ([`osk_ui::screens::VIEWFINDER_STATE`]).
pub const VIEWFINDER_STATE: Id = osk_ui::screens::VIEWFINDER_STATE;
/// The block §2.6 centres on a screen built from [`osk_ui::screens`]
/// ([`osk_ui::screens::BLOCK`]), so a layout test can measure a table
/// against the space it sits in.
pub const BLOCK: Id = osk_ui::screens::BLOCK;
/// The bottom group of §2.6 ([`osk_ui::screens::GROUP`]): the touch
/// controls a screen puts against its bottom edge, which on the
/// Scanner is the band of ways in.
pub const GROUP: Id = osk_ui::screens::GROUP;
/// The reserved caption line under a field
/// ([`osk_ui::screens::ERROR`]): "No word starts like that", "PINs
/// differed", "Wrong PIN · 4 left".
pub const INLINE_ERROR: Id = osk_ui::screens::ERROR;

// ----- The status line (UX.md §4 "Status") -----

/// Status line: the tier badge, which opens the tier explanation.
pub const STATUS_TIER: Id = Id(10);
/// Status line: the lock icon and countdown, which locks now.
pub const STATUS_LOCK: Id = Id(11);

/// Add a key: "Load a key", which is also Keys' first empty-state row.
pub const KEYS_LOAD: Id = Id(60);
/// Add a key: "Create a key", which is also Keys' second empty-state
/// row.
pub const KEYS_CREATE: Id = Id(61);
/// Add a key, while a wallet or a transaction is asking for a key:
/// "Open with passphrase", which asks which loaded key first.
pub const ADD_OPEN_PASSPHRASE: Id = Id(2200);
/// Add a key, under the same question: "Open BIP-85 child".
pub const ADD_OPEN_CHILD: Id = Id(2201);

// ----- Keys and Wallets (`docs/PLANNING.md` §16.104) -----

/// Add a wallet: "Load a wallet", which is also Wallets' second
/// empty-state row.
pub const WALLETS_LOAD: Id = Id(62);
/// Wallets: one row per registered policy, by its index in `wallets`.
/// Each opens that wallet's page.
pub const WALLETS_POLICY_ROW_BASE: u32 = 90;
/// Keys: the bottom action, which opens the Add a key menu.
pub const KEYS_ADD: Id = Id(2202);
/// Wallets: the bottom action, which opens the Add a wallet menu, and
/// Wallets' first empty-state row.
pub const WALLETS_ADD: Id = Id(2203);
/// Keys: one row per loaded key, by its index in `keys`.
pub const KEYS_ROW_BASE: u32 = 2210;
/// "Which key?": one row per loaded key, by its index in `keys`.
pub const PICK_KEY_BASE: u32 = 2240;
/// A wallet's Keys review: one row per member, by its place in the
/// policy's keys.
pub const WALLET_KEY_ROW_BASE: u32 = 2270;
/// The key page's row stating what the key is made of.
pub const KEY_MADE_OF: Id = Id(2206);
/// The key page's Forget row.
pub const KEY_FORGET: Id = Id(2207);

// ----- The wallet menu: a wallet in use, and one screen per row -----

/// Wallet: the "Sign a transaction" row, which opens the scanner
/// expecting a PSBT.
pub const WALLET_SIGN: Id = Id(814);
/// Wallet: the "Sign a message" row, on a wallet over one key.
pub const WALLET_SIGN_MESSAGE: Id = Id(815);
/// Wallet: the "Check an address" row, which opens the address check.
pub const WALLET_CHECK: Id = Id(816);
/// Wallet: the "Addresses" row.
pub const WALLET_ADDRESSES: Id = Id(810);
/// Wallet: the "Export" row.
pub const WALLET_EXPORT: Id = Id(811);
/// Wallet: the "Keys" row, which opens the wallet's own review, and on
/// a wallet over one key the "Key" row, which opens that key's menu.
/// One row, one id: a wallet draws whichever its kind has.
pub const WALLET_KEYS: Id = Id(812);
/// Wallet: the "Forget" row.
pub const WALLET_FORGET: Id = Id(813);
/// Wallet: the "Name" row, the first of the page, which opens the field
/// the name is typed in.
pub const WALLET_NAME: Id = Id(817);
/// The keyboard of that field.
pub const WALLET_NAME_KEYBOARD: Id = Id(818);
/// Wallet: the "Kept on this device" toggle, on a Tier B device that
/// keeps keys (`docs/PLANNING.md` §16.104 rule 7).
pub const WALLET_KEEP: Id = Id(819);

/// Load: the "Type the words" source row.
pub const LOAD_SOURCE_TYPE: Id = Id(100);
/// Load: the "Scan a SeedQR" source row.
pub const LOAD_SOURCE_SCAN: Id = Id(101);
/// Load: the "Encrypted backup" source row, which opens the scanner for
/// a backup rather than a seed code.
pub const LOAD_SOURCE_BACKUP: Id = Id(99);
/// Load: the "Seed XOR parts" source row, which combines parts back
/// into the key they were split from.
pub const LOAD_SOURCE_XOR: Id = Id(98);
/// Load: the "SLIP-39 shares" source row, which reads the shares of a
/// SLIP-39 backup one after another (`docs/PLANNING.md` §16.107).
pub const LOAD_SOURCE_SLIP39: Id = Id(97);
/// Load source: a codex32 string (`docs/PLANNING.md` §16.109).
pub const LOAD_SOURCE_CODEX32: Id = Id(96);
/// Load: the "Word numbers" source row, where each word is typed as its
/// 1-based number on the wordlist (`docs/PLANNING.md` §16.125 rule 2).
pub const LOAD_SOURCE_NUMBERS: Id = Id(95);
/// Load: the "Hex entropy" source row, where the key's entropy is typed
/// as hex digits and the words follow from it (§16.125 rule 3).
pub const LOAD_SOURCE_HEX: Id = Id(94);
/// Load: Continue on the source step, which opens the scanner when the
/// scan row is the checked one.
pub const LOAD_SOURCE_CONTINUE: Id = Id(102);
/// Load: Continue on the word-count step.
pub const LOAD_COUNT_CONTINUE: Id = Id(103);
/// Load: Continue on the language step.
pub const LOAD_LANG_CONTINUE: Id = Id(104);
/// Load and Create: Continue on the passphrase offer.
pub const LOAD_PASS_CONTINUE: Id = Id(105);
/// Load and Create: Continue on "Which key?".
pub const LOAD_WHICH_CONTINUE: Id = Id(106);
/// Load and Create: the "no passphrase" row of "Which key?".
pub const LOAD_WHICH_PLAIN: Id = Id(107);
/// Load and Create: the "with passphrase" row of "Which key?".
pub const LOAD_WHICH_PASSPHRASE: Id = Id(108);
/// Load: the "words so far" panel above the word entry, on the classes
/// that have the room for it. Drawn from the first accepted word, masked,
/// and revealed by a finger on it or by [`SECRET_EYE`] (§4.3, §4.10).
pub const LOAD_WORDS_PANEL: Id = Id(109);
/// Load: word-count buttons, in [`osk_entropy::WORD_COUNTS`] order.
pub const LOAD_COUNT_BASE: u32 = 110;
/// Load: language rows, in `Language::ALL` order.
pub const LOAD_LANG_BASE: u32 = 120;
/// Load: the "Language" row of the checksum result, which goes back to
/// the language step. A code does not name its language, so scanned
/// words keep it one tap away.
pub const LOAD_LANG_OTHER: Id = Id(139);
/// Load: the "Continue" button of steps that have one. Also the
/// passphrase-confirm "Continue" of the Create wizard, which shares the
/// finishing steps (`crate::finish`).
pub const LOAD_CONTINUE: Id = Id(130);
/// Load: the BIP-39 keyboard.
pub const LOAD_KEYBOARD: Id = Id(131);
/// The codex32 string drawn whole above the field, for the chunk
/// planner (`docs/DESIGN.md` §4.5).
pub const LOAD_CODEX32_TYPED: Id = Id(140);
/// Load: the candidate strip's first row.
pub const LOAD_CANDIDATES: Id = Id(132);
/// Load: the candidate strip's second row, on the classes that have one
/// (`docs/DESIGN.md` §4.3: two rows of four on `mobile`).
pub const LOAD_CANDIDATES_2: Id = Id(129);
/// Load: "Start over" after a checksum failure.
pub const LOAD_START_OVER: Id = Id(133);
/// Load and Create: the "No passphrase" row of the passphrase offer.
pub const LOAD_SKIP: Id = Id(134);
/// Load and Create: the "Add a passphrase" row of the passphrase offer.
pub const LOAD_ADD_PASSPHRASE: Id = Id(135);
/// Load and Create: the passphrase keyboard.
pub const LOAD_PASS_KEYBOARD: Id = Id(136);
/// Load and Create: the "Add key" button of the confirm step. Adding a
/// key is reversible, so it is a tap (`docs/PLANNING.md` §16.27).
pub const LOAD_HOLD: Id = Id(137);
/// Load: "Fix word N" rows and button, by 0-based word position.
pub const LOAD_FIX_BASE: u32 = 140;
/// Load and Create: the PIN pad of the session PIN steps.
pub const LOAD_PIN_KEYBOARD: Id = Id(138);

// ----- Key detail: a menu, and one screen per row (UX.md §7.2) -----

/// Key detail: the "Backup" row.
pub const DETAIL_BACKUP: Id = Id(202);
/// Key detail: the "Open with passphrase" row, on a key that has words,
/// which opens this key's passphrase entry (`docs/PLANNING.md` §16.127
/// rule 3).
pub const DETAIL_OPEN_PASSPHRASE: Id = Id(203);
/// Key detail: the "Add a wallet" row, on a key with a master, which
/// starts the wallet wizard with this key in hand (`docs/PLANNING.md`
/// §16.129 rule 1).
pub const DETAIL_ADD_WALLET: Id = Id(204);
/// Forget: the hold-to-confirm button.
pub const DETAIL_FORGET: Id = Id(205);

/// The rows of the Choice screen a value row opens (`docs/DESIGN.md`
/// §4.2): the script type and the wallet-export format share one list,
/// because one screen carries one value row at a time.
pub const PICK_BASE: u32 = 210;
/// Continue on that Choice screen, which applies the checked row.
pub const PICK_CONTINUE: Id = Id(209);
/// Done on the Compare screen a reference row opens (§4.5).
pub const COMPARE_DONE: Id = Id(208);
/// The whole string on that screen, chunked in fours.
pub const COMPARE_TEXT: Id = Id(207);
/// Addresses: the script-type value row.
pub const ADDR_SCRIPT: Id = Id(226);
/// Addresses: the Receive half of the chain pair.
pub const ADDR_RECEIVE: Id = Id(222);
/// Addresses: the Change half of the chain pair.
pub const ADDR_CHANGE: Id = Id(223);
/// Address detail: the address itself, whole and chunked.
pub const ADDR_TEXT: Id = Id(224);
/// Address detail: the QR code node.
pub const ADDR_QR: Id = Id(225);
/// Addresses: the row that extends the list by ten.
pub const ADDR_MORE: Id = Id(228);
/// Addresses: one row per address, by index in the list.
pub const ADDR_ROW_BASE: u32 = 1000;

/// Backup: the "Show words" row.
pub const BACKUP_WORDS: Id = Id(230);
/// Words screen (Backup and Create): the button that puts each word's
/// wordlist number beside it.
pub const WORDS_NUMBERS: Id = Id(231);
/// Backup: the "SeedQR" row.
pub const BACKUP_SEEDQR: Id = Id(232);
/// Backup: the "CompactSeedQR" row.
pub const BACKUP_COMPACT: Id = Id(233);
/// Backup: the "Verify backup" row.
pub const BACKUP_VERIFY: Id = Id(234);
/// Backup: the "Encrypted backup" row.
pub const BACKUP_ENCRYPTED: Id = Id(235);
/// Encrypted backup: the keyboard the backup passphrase is typed on,
/// both times.
pub const BACKUP_PASS_KEYBOARD: Id = Id(236);
/// Encrypted backup: "Save to file" on the result.
pub const BACKUP_SAVE: Id = Id(237);
/// Encrypted backup: "Show as QR" on the result.
pub const BACKUP_SHOW_QR: Id = Id(238);
/// Backup: the "Draw a SeedQR" row, which opens the code Choice.
pub const BACKUP_GRID: Id = Id(1930);
/// "Which code?": the SeedQR row.
pub const BACKUP_GRID_SEEDQR: Id = Id(1931);
/// "Which code?": the CompactSeedQR row.
pub const BACKUP_GRID_COMPACT: Id = Id(1932);
/// "Which code?": Continue, which opens the grid.
pub const BACKUP_GRID_CONTINUE: Id = Id(1933);
/// Backup: the "Numbers for steel" row.
pub const BACKUP_STEEL: Id = Id(1934);
/// The steel screen's "Print template" action, on `wide`.
pub const BACKUP_STEEL_TEMPLATE: Id = Id(1935);
/// Backup: the "Split into parts" row, which opens the part-count
/// Choice.
pub const BACKUP_XOR: Id = Id(1936);
/// "How many parts?": the rows, in [`crate::backup::XOR_COUNTS`] order.
pub const XOR_COUNT_BASE: u32 = 1940;
/// "How many parts?": Continue, which makes the split.
pub const XOR_COUNT_CONTINUE: Id = Id(1944);
/// Create: "Add another part" after a Seed XOR part has been typed.
pub const XOR_ADD_ANOTHER: Id = Id(1945);
/// Create: "Combine", which XORs the parts typed so far.
pub const XOR_COMBINE: Id = Id(1946);
/// "Random parts from?": the rows, in [`osk_entropy::SOURCE_ROWS`]
/// order.
pub const XOR_SOURCE_BASE: u32 = 1950;
/// "Random parts from?": Continue, which opens the first part's entry.
pub const XOR_SOURCE_CONTINUE: Id = Id(1957);

/// Open passphrase: the passphrase keyboard.
pub const OPEN_PASS_KEYBOARD: Id = Id(240);
/// Open BIP-85 child seed: Continue on the word-count choice.
pub const OPEN_CHILD_CONTINUE: Id = Id(241);
/// Open BIP-85 child seed: the word-count rows, in
/// [`crate::CHILD_COUNTS`] order.
pub const OPEN_CHILD_WORDS_BASE: u32 = 242;
/// Open BIP-85 child seed: the keyboard the index is typed on.
pub const OPEN_CHILD_KEYBOARD: Id = Id(246);
/// The Result after a key is opened: the action that opens its menu.
pub const OPENED_OPEN: Id = Id(247);

/// Export: the row that opens the wallet-export QR screen.
pub const EXPORT_SHOW: Id = Id(288);
/// Export: the "Animated" switch on the QR screen that row opens (§4.9).
pub const EXPORT_ANIMATED: Id = Id(289);
/// Export: the format value row, which opens the format Choice.
pub const EXPORT_FORMAT: Id = Id(285);
/// Export: the script-type value row, which opens the script Choice.
pub const EXPORT_SCRIPT: Id = Id(292);
/// Export: the exported string as a reference row, which opens Compare.
pub const EXPORT_TEXT: Id = Id(296);
/// The shortened extended key inside a descriptor on the Compare
/// screen, which opens the key whole on a Compare screen of its own.
/// One id, because one Compare screen is open at a time: the wallet
/// export's descriptor and a scanned one both reach it.
pub const COMPARE_KEY: Id = Id(298);

/// Settings: the rows of the network Choice, in `Network::ALL` order.
pub const SETTINGS_NET_BASE: u32 = 300;
/// Settings: the value row that opens the network Choice.
pub const SETTINGS_NETWORK_ROW: Id = Id(304);
/// Settings: the value row that opens the unit Choice.
pub const SETTINGS_UNIT_ROW: Id = Id(305);
/// Settings: the value row that opens the auto-lock Choice.
pub const SETTINGS_LOCK_AFTER_ROW: Id = Id(306);
/// Settings: the value row that opens the auto-wipe Choice.
pub const SETTINGS_WIPE_AFTER_ROW: Id = Id(307);
/// Settings: the row that opens About.
pub const SETTINGS_ABOUT_ROW: Id = Id(308);
/// About: the tier row, which opens the tiers document.
pub const ABOUT_TIER: Id = Id(317);
/// About: the core hash as a reference row, which opens Compare.
pub const ABOUT_HASH: Id = Id(318);
/// Wipe all keys: "Done" on the Result that says the keys are gone.
pub const SETTINGS_WIPED_DONE: Id = Id(319);
/// Wipe all keys: the hold-to-wipe button on its own screen.
pub const SETTINGS_WIPE: Id = Id(310);
/// Settings: the row that opens the wipe screen.
pub const SETTINGS_WIPE_ROW: Id = Id(311);
/// Settings: the row that opens the wipe-and-exit screen.
pub const SETTINGS_EXIT_ROW: Id = Id(312);
/// Settings: run the self-test again.
pub const SETTINGS_SELFTEST: Id = Id(313);
/// Settings: "Lock now".
pub const SETTINGS_LOCK: Id = Id(314);
/// Settings: the "Scramble PIN pad" toggle.
pub const SETTINGS_SCRAMBLE: Id = Id(315);
/// Wipe and exit: the hold button on its own screen.
pub const SETTINGS_EXIT: Id = Id(316);
/// Settings: auto-lock rows, in [`crate::session::LOCK_OPTIONS`] order.
pub const SETTINGS_LOCK_AFTER_BASE: u32 = 320;
/// Settings: auto-wipe rows, in [`crate::session::WIPE_OPTIONS`] order.
pub const SETTINGS_WIPE_AFTER_BASE: u32 = 330;
/// Settings: the rows of the unit Choice, in [`crate::UNITS`] order.
pub const SETTINGS_UNIT_BASE: u32 = 350;
/// Settings: the value row that opens the camera-rotation Choice.
pub const SETTINGS_CAMERA_ROTATION_ROW: Id = Id(309);
/// Settings: the rows of the camera-rotation Choice, in
/// [`crate::scan::CAMERA_ROTATIONS`] order.
pub const SETTINGS_CAMERA_ROTATION_BASE: u32 = 360;
/// Settings: the value row that opens the nonce Choice.
pub const SETTINGS_NONCE_ROW: Id = Id(370);
/// Settings: the rows of the nonce Choice, in [`crate::NONCES`] order.
pub const SETTINGS_NONCE_BASE: u32 = 372;
/// Settings: the value row that opens the Schnorr Choice.
pub const SETTINGS_SCHNORR_ROW: Id = Id(375);
/// Settings: the rows of the Schnorr Choice, in [`crate::SCHNORRS`]
/// order.
pub const SETTINGS_SCHNORR_BASE: u32 = 377;

/// Lock screen: the PIN pad. The only control while locked.
pub const LOCK_KEYBOARD: Id = Id(340);

/// Sign: "Show as QR" on the result step.
pub const SIGN_QR: Id = Id(402);
/// Sign: the Animated toggle on the QR screen, which switches between
/// one static code and the BC-UR parts.
pub const SIGN_QR_ANIMATED: Id = Id(407);
/// Sign: "Continue" on the review steps.
pub const SIGN_CONTINUE: Id = Id(410);
/// Sign: the "Back" button of a step that offers nothing else.
pub const SIGN_BACK: Id = Id(411);
/// Sign: the "I understand the risks" toggle, shown only with danger
/// warnings.
pub const SIGN_ACK: Id = Id(412);
/// Sign: the hold-to-sign button.
pub const SIGN_HOLD: Id = Id(413);
/// Sign: "Save to file" on the result step.
pub const SIGN_SAVE: Id = Id(414);
/// Sign: the "Signatures" row of the result, which opens the signature
/// menu.
pub const SIGN_SIGNATURES: Id = Id(415);
/// Sign: "Done" on the result step.
pub const SIGN_DONE: Id = Id(416);
/// Sign: the transaction id on the signature-bytes page.
pub const SIGN_TXID: Id = Id(417);
/// Sign: the recipient's address on the review and confirm screens,
/// which opens Compare.
pub const SIGN_TO: Id = Id(406);
/// Sign: signing-key chips on the confirm step, in
/// `Inspection::participating_keys` order.
pub const SIGN_KEY_BASE: u32 = 420;
/// Sign: output addresses, by output index, each a reference row that
/// opens Compare.
pub const SIGN_OUT_BASE: u32 = 440;
/// Sign: signature hex strings, by signed-input position.
pub const SIGN_SIG_BASE: u32 = 480;

/// Self-test failure: the only control, "Exit".
pub const SELFTEST_EXIT: Id = Id(500);
/// Boot refusal: the only control, "Exit".
pub const BOOT_EXIT: Id = Id(510);

/// The Tier A start screen: the only control, "Continue".
pub const NO_SECURE_BOOT_CONTINUE: Id = Id(501);

// ----- the first run (`docs/UX.md` §7.1, job A4) -----

/// Start here: "Continue", which opens the Add menu.
pub const START_HERE_CONTINUE: Id = Id(502);
/// Settings: the row that reopens Start here.
pub const SETTINGS_START_HERE_ROW: Id = Id(503);
/// The Result after the first created key: "Check an address", which
/// opens that key's wallet at Addresses.
pub const CREATED_CHECK: Id = Id(504);
/// The same Result: "Done", which goes to Home.
pub const CREATED_DONE: Id = Id(505);

/// Scan: the "Read a file" row under the viewfinder.
pub const SCAN_FILE: Id = Id(701);
/// Scan: "Try again" on the Result of a code that could not be used.
pub const SCAN_AGAIN: Id = Id(702);
/// Scan: "Read as a transaction" for an unknown code.
pub const SCAN_AS_PSBT: Id = Id(703);
/// Scan: "Read as text" for an unknown code.
pub const SCAN_AS_TEXT: Id = Id(704);
/// Scan: "Load" on the caution over words read in the clear.
pub const SCAN_CONTINUE: Id = Id(705);
/// Scan: the unknown code's bytes as hex, a reference row that opens
/// Compare.
pub const SCAN_HEX: Id = Id(706);
/// Scan: the keyboard the passphrase of a scanned encrypted backup is
/// typed on.
pub const SCAN_PASS_KEYBOARD: Id = Id(707);
/// Scan: the "Paste" row under the viewfinder.
pub const SCAN_PASTE: Id = Id(708);
/// Scan: the "Type" row under the viewfinder.
pub const SCAN_TYPE: Id = Id(709);
/// Scan: "Hash it" for an unknown code.
pub const SCAN_AS_HASHES: Id = Id(717);
/// Scan: "Show encodings" for an unknown code.
pub const SCAN_AS_ENCODINGS: Id = Id(718);
/// Scan: "Use a loaded key" on Convert key's scanner.
pub const SCAN_USE_KEY: Id = Id(719);
/// Copy: the row action that puts a public string on the clipboard. One
/// id, because one screen at a time carries it.
pub const COPY: Id = Id(696);
/// "Save as PNG": the row on a QR screen whose code is public, which
/// saves the code as a picture. One id, as Copy's is.
pub const SAVE_PNG: Id = Id(697);

/// Verify: the address keyboard.
pub const VERIFY_KEYBOARD: Id = Id(712);
/// Verify: the address on the result, which opens Compare.
pub const VERIFY_ADDRESS: Id = Id(713);
/// Verify: "Check another".
pub const VERIFY_CLEAR: Id = Id(714);
/// Verify: the typed address, whole, above the entry group (§4.3).
pub const VERIFY_TYPED: Id = Id(716);

/// Inspect: the descriptor, key or text as a reference row, which opens
/// the Compare screen.
pub const INSPECT_TEXT: Id = Id(720);
/// Inspect: "Done", which is the way out the chevron also is.
pub const INSPECT_DONE: Id = Id(721);
/// Inspect: "Use this wallet" on a wallet policy's review, which
/// registers it for the session.
pub const INSPECT_USE_WALLET: Id = Id(722);
/// Inspect: "Forget" on the review of a wallet already in use.
pub const INSPECT_FORGET_WALLET: Id = Id(723);

/// Tools: the "Key explorer" row, the one tool that is built.
pub const TOOLS_EXPLORER: Id = Id(1700);
/// Tools: the "Word list" row.
pub const TOOLS_WORD_LIST: Id = Id(1701);
/// Tools: the "Dice passphrase" row.
pub const TOOLS_DICE: Id = Id(1702);
/// Tools: the six calculator rows, in [`crate::tools::Tool::ALL`]
/// order — Hashes, Encodings, Descriptor checksum, Convert key, Units,
/// Miniscript.
pub const TOOLS_CALC_BASE: u32 = 1703;
/// Tools: the "Decode a transaction" row, which opens the scanner.
pub const TOOLS_DECODE: Id = Id(1709);
/// Tools: the "Compare transactions" row, which opens the scanner for
/// the first of the two (`docs/PLANNING.md` §16.111 rule 4).
pub const TOOLS_COMPARE: Id = Id(1714);
/// Compare transactions: the row that reads the second one.
pub const COMPARE_TX_SECOND: Id = Id(1715);
/// Compare transactions: Done on the result.
pub const COMPARE_TX_DONE: Id = Id(1716);

// ----- Tools > the calculators (`crate::tools`) -----

/// A calculator: the keyboard of its field, whichever kind it is.
pub const TOOL_KEYBOARD: Id = Id(1710);
/// A calculator: the mode row above its field — "Read as" on Hashes,
/// "From" on Units — which opens the Choice.
pub const TOOL_MODE: Id = Id(1711);
/// A calculator's Record: the reference rows of the strings it worked
/// out, in the order the Record lists them.
pub const TOOL_ROW_BASE: u32 = 1720;
/// Decode a transaction: "Done" on the last page of the review.
pub const DECODE_DONE: Id = Id(1712);
/// Miniscript: "Load as wallet" on a compiled policy that is one.
pub const TOOL_LOAD_WALLET: Id = Id(1713);

// ----- Tools > Word list -----

/// Word list: the language rows, in [`osk_bip::bip39::Language::ALL`]
/// order.
pub const WORDLIST_LANG_BASE: u32 = 850;
/// Word list: Continue on the language step.
pub const WORDLIST_LANG_CONTINUE: Id = Id(860);
/// Word list: the "Search by" mode row, which opens the Choice.
pub const WORDLIST_BY: Id = Id(861);
/// Word list: the keyboard of the search field, whichever notation it
/// types.
pub const WORDLIST_KEYBOARD: Id = Id(863);
/// Word list: the candidate strip's first row.
pub const WORDLIST_CANDIDATES: Id = Id(864);
/// Word list: the candidate strip's second row, on the classes that
/// have one.
pub const WORDLIST_CANDIDATES_2: Id = Id(865);
/// One word: the pager's previous word.
pub const WORD_PREV: Id = Id(866);
/// One word: the pager's next word.
pub const WORD_NEXT: Id = Id(867);

// ----- Tools > Dice passphrase -----

/// Dice passphrase: the list rows, in
/// [`osk_bip::diceware::List::ALL`] order.
pub const DICE_LIST_BASE: u32 = 880;
/// Dice passphrase: Continue on the list step.
pub const DICE_LIST_CONTINUE: Id = Id(883);
/// Dice passphrase: the word-count rows, in
/// [`crate::dice::WORD_COUNTS`] order.
pub const DICE_WORDS_BASE: u32 = 884;
/// Dice passphrase: Continue on the word-count step.
pub const DICE_WORDS_CONTINUE: Id = Id(890);
/// Dice passphrase: the dice pad.
pub const DICE_PAD: Id = Id(891);
/// Dice passphrase: Continue under the pad, live once every roll is in.
pub const DICE_CONTINUE: Id = Id(892);
/// Dice passphrase: the panel of words rolled so far, above the pad,
/// and the panel of the whole passphrase on the result.
pub const DICE_PANEL: Id = Id(893);
/// Dice passphrase: "Done" on the result, which returns to Tools.
pub const DICE_DONE: Id = Id(894);
/// Dice passphrase: "Discard" on the confirm that guards the rolls.
pub const DICE_DISCARD: Id = Id(895);
/// Dice passphrase: "Not now" on that confirm.
pub const DICE_KEEP: Id = Id(896);

/// Explore: the key-context row of the key menu, which opens the key
/// chooser.
pub const EXPLORE_USING: Id = Id(730);
/// Explore: "Type words" (the Load wizard's entry step as a sub-flow).
pub const EXPLORE_TYPE: Id = Id(731);
/// Explore: the "Typed words" row of the key chooser.
pub const EXPLORE_TYPED: Id = Id(732);
/// Explore: Continue on the key chooser.
pub const EXPLORE_CHOOSE_CONTINUE: Id = Id(733);
/// Explore: the "Words and bits" row of the key menu.
pub const EXPLORE_BITS: Id = Id(734);
/// Explore: the "Words" row, which opens the Words screen.
pub const EXPLORE_WORDS: Id = Id(735);
/// Explore: the passphrase row (typed words only).
pub const EXPLORE_PASSPHRASE: Id = Id(736);
/// Explore: the passphrase keyboard.
pub const EXPLORE_PASS_KEYBOARD: Id = Id(737);
/// Explore: the path row, which opens the path editor.
pub const EXPLORE_PATH: Id = Id(738);
/// Explore: the path keyboard.
pub const EXPLORE_PATH_KEYBOARD: Id = Id(739);
/// Explore: "Receive" of the path editor's Chain pair (§4.6).
pub const EXPLORE_RECEIVE: Id = Id(740);
/// Explore: the entropy row, which opens the Secret screen.
pub const EXPLORE_ENTROPY: Id = Id(741);
/// Explore: the seed row, which opens the Secret screen.
pub const EXPLORE_SEED: Id = Id(742);
/// Explore: the master private key row, which opens the Secret screen.
pub const EXPLORE_MASTER_XPRV: Id = Id(743);
/// Explore: the checksum-bits row, which opens the Secret screen.
pub const EXPLORE_CHECKSUM_BITS: Id = Id(744);
/// Explore: the account key as a reference row, which opens Compare.
pub const EXPLORE_ACCOUNT_XPUB: Id = Id(745);
/// Explore: the SLIP-132 form as a reference row, which opens Compare.
pub const EXPLORE_SLIP132: Id = Id(746);
/// Explore: "Done" on the Words screen.
pub const EXPLORE_WORDS_DONE: Id = Id(747);
/// Explore: "Load a key" on the empty menu.
pub const EXPLORE_LOAD: Id = Id(748);
/// Explore: the row into this key's Addresses screen.
pub const EXPLORE_ADDRESSES: Id = Id(749);
/// Explore: key-chooser rows, by key index.
pub const EXPLORE_KEY_BASE: u32 = 750;
/// Explore: "Change" of the path editor's Chain pair (§4.6).
pub const EXPLORE_CHANGE: Id = Id(772);
/// Explore: purpose preset rows of the path editor, in
/// `ScriptType::ALL` order.
pub const EXPLORE_PRESET_BASE: u32 = 780;
/// Explore: the extended public key at the applied path, as a reference
/// row.
pub const EXPLORE_XPUB: Id = Id(790);
/// Explore: the extended private key at the applied path, as a secret
/// row.
pub const EXPLORE_XPRV: Id = Id(800);
/// Explore: "Keep" on the discard confirm.
pub const EXPLORE_KEEP: Id = Id(823);
/// Explore: "Discard" on the discard confirm.
pub const EXPLORE_DISCARD: Id = Id(824);

/// Create: source rows, in [`osk_entropy::SOURCE_ROWS`] order.
pub const CREATE_SOURCE_BASE: u32 = 600;
/// Create: Continue on the source step.
pub const CREATE_SOURCE_CONTINUE: Id = Id(636);
/// Create: word-count rows, in [`osk_entropy::WORD_COUNTS`] order.
pub const CREATE_COUNT_BASE: u32 = 610;
/// Create: Continue on the word-count step.
pub const CREATE_COUNT_CONTINUE: Id = Id(638);
/// Create: language rows, in `Language::ALL` order.
pub const CREATE_LANG_BASE: u32 = 620;
/// Create: Continue on the language step.
pub const CREATE_LANG_CONTINUE: Id = Id(639);
/// Create: the "Continue" button of the steps that have one: the
/// entropy pad, the sanity result, the words, and Done on the math.
pub const CREATE_CONTINUE: Id = Id(630);
/// Create: the dice pad, coin pad or hex keyboard of the entropy step.
pub const CREATE_PAD: Id = Id(631);
/// Create: the masked run of rolls, flips or digits inside the pad
/// group ([`osk_ui::screens::ENTRIES`]).
pub const CREATE_ENTRIES: Id = osk_ui::screens::ENTRIES;
/// Create: "Roll again" on the sanity step (back to entry, cleared).
pub const CREATE_AGAIN: Id = Id(633);
/// Create: "The math" on the sanity step.
pub const CREATE_MATH: Id = Id(634);
/// The secret panel of the Words and Secret screens, wherever they are
/// shown: Create's words and math, Backup's words and seed codes, and
/// every one of Explore's secrets.
pub const CREATE_REVEAL: Id = Id(635);
/// Create: the math's "Entropy" row, which opens the entropy hex on the
/// Secret screen.
pub const CREATE_MATH_ENTROPY: Id = Id(632);
/// Create: the math's checksum-bits row, which opens the bits on the
/// Secret screen.
pub const CREATE_MATH_BITS: Id = Id(652);
/// Create: the math's "Word 1" row, which opens the first word on the
/// Secret screen.
pub const CREATE_MATH_WORD: Id = Id(653);
/// Create: the shutter of the camera-noise step.
pub const CREATE_SHUTTER: Id = Id(658);
/// Create: the toggle rows of "Mix which sources?", in
/// [`osk_entropy::MIX_SOURCES`] order.
pub const CREATE_MIX_BASE: u32 = 660;
/// Create: Continue on "Mix which sources?".
pub const CREATE_MIX_CONTINUE: Id = Id(665);

/// Words screen (Create and Backup): the pager's previous page.
pub const WORDS_PREV: Id = Id(656);
/// Words screen (Create and Backup): the pager's next page.
pub const WORDS_NEXT: Id = Id(657);

/// Quiz (Create step and Backup flow): the four candidate rows.
pub const QUIZ_CHOICE_BASE: u32 = 640;
/// Quiz: Continue, which confirms the checked candidate.
pub const QUIZ_CONTINUE: Id = Id(654);
/// Quiz: "Try again" after a wrong answer.
pub const QUIZ_RETRY: Id = Id(644);
/// Quiz: "Show the words" after a wrong answer, which opens the words
/// screen for this key and comes back to the same question.
pub const QUIZ_SHOW_WORDS: Id = Id(651);
/// Quiz: "Skip quiz" on the Create wizard's start screen.
pub const QUIZ_SKIP: Id = Id(645);
/// Quiz: "Skip anyway" on the skip caution.
pub const QUIZ_SKIP_CONFIRM: Id = Id(646);
/// Quiz: "Keep going" on the skip caution.
pub const QUIZ_SKIP_CANCEL: Id = Id(647);
/// Quiz: the "Helper mode" toggle on the start screen.
pub const QUIZ_HELPER: Id = Id(648);
/// Quiz: "Start quiz".
pub const QUIZ_START: Id = Id(649);
/// Backup: "Done" on the result, words and code screens.
pub const QUIZ_DONE: Id = Id(650);
/// Backup: "Run again" on a quiz result that did not pass.
pub const QUIZ_AGAIN: Id = Id(655);

/// Learn: the last row of a page whose subject has a flow, which opens
/// that flow.
pub const LEARN_TRY: Id = Id(849);
/// Learn: the heading a page opened from the info button starts at,
/// which the frame that placed it scrolls to the top of the view.
pub const LEARN_HEADING: Id = Id(848);
/// Learn: the row that opens page `i` of the list, in the order
/// [`osk_learn::Learn::pages`] gives them.
pub const LEARN_ROW_BASE: u32 = 850;

/// Files: the row that reads the file at position `i` of the listing
/// the shell answered a file request with, newest first.
pub const FILES_ROW_BASE: u32 = 1300;

/// Key detail: the row that keeps this key on the device, and the row
/// that says it is kept. One row, one id, since only one of the two is
/// ever drawn.
pub const KEEP_ROW: Id = Id(1400);
/// The Hold that keeps the key on the device.
pub const KEEP_HOLD: Id = Id(1401);
/// "Not now" beside that hold, on the offer that follows adding a key.
pub const KEEP_NOT_NOW: Id = Id(1402);
/// The PIN pad that opens the stored key: the first screen of a device
/// that keeps one.
pub const KEEP_PIN_KEYBOARD: Id = Id(1404);
/// Settings: "Duress PIN".
pub const KEEP_DURESS_ROW: Id = Id(1405);
/// The PIN pad that sets the duress PIN and repeats it.
pub const KEEP_DURESS_KEYBOARD: Id = Id(1406);
/// "Done" on the Result after the stored key was removed.
pub const KEEP_REMOVED_DONE: Id = Id(1409);

/// Tools: the "Verify a message" row.
pub const MSG_CHECK_ROW: Id = Id(1501);
/// Message: the key value row, which opens the key Choice.
pub const MSG_KEY: Id = Id(1502);
/// Message: the script-type value row, which opens the script Choice.
pub const MSG_SCRIPT: Id = Id(1503);
/// Message: the address, a reference row that opens Compare.
pub const MSG_ADDRESS: Id = Id(1504);
/// Message: the format value row, which opens the format Choice.
pub const MSG_FORMAT: Id = Id(1505);
/// Message: "Continue", on to the hold.
pub const MSG_CONTINUE: Id = Id(1507);
/// Message: the hold that signs.
pub const MSG_HOLD: Id = Id(1508);
/// Message: the signature, a reference row that opens Compare.
pub const MSG_SIGNATURE: Id = Id(1509);
/// Message: "Save to file" on the result.
pub const MSG_SAVE: Id = Id(1510);
/// Message: "Done" on the result, once the file is written.
pub const MSG_DONE: Id = Id(1511);
/// Message: "Show as QR" on the result.
pub const MSG_QR: Id = Id(1512);
/// Message: the Animated toggle of the QR screen, which a signature
/// short enough for one code leaves dimmed (§4.9).
pub const MSG_QR_ANIMATED: Id = Id(1513);
/// Message: the "Message" row of a check result, which opens the
/// message itself.
pub const MSG_READ: Id = Id(1514);
/// Sign a message: the keyboard the message is typed on.
pub const MSG_KEYBOARD: Id = Id(1515);

/// `base + index` as an id.
pub const fn at(base: u32, index: usize) -> Id {
    Id(base + index as u32)
}

/// The index of `id` within `base..base + len`, if any.
pub fn index_in(id: Id, base: u32, len: usize) -> Option<usize> {
    let i = id.0.checked_sub(base)? as usize;
    (i < len).then_some(i)
}

// ----- The wallet builder (`crate::build`) -----

/// Add a wallet: "New wallet", which starts the wizard.
pub const BUILD_NEW: Id = Id(63);
/// The wizard: the kinds, in `crate::build::WalletKind::ALL` order.
pub const BUILD_KIND_BASE: u32 = 1940;
/// The wizard: Continue on the kind Choice.
pub const BUILD_KIND_CONTINUE: Id = Id(1812);
/// Builder: Continue on the script-type Choice.
pub const BUILD_SCRIPT_CONTINUE: Id = Id(1800);
/// Builder: Continue on the Keys step, dead below the kind's count.
pub const BUILD_CONTINUE: Id = Id(1802);
/// Builder: the keyboard the "Type" row's cosigner key is typed on.
pub const BUILD_TYPE_KEYBOARD: Id = Id(1811);
/// Builder: "Scan a key", the last row of the Keys step.
pub const BUILD_WHICH_SCAN: Id = Id(1803);
/// Builder: Continue on the "How many must sign?" Choice.
pub const BUILD_THRESHOLD_CONTINUE: Id = Id(1805);
/// Builder: "Add this wallet" on the review.
pub const BUILD_ADD_WALLET: Id = Id(1808);
/// Builder: "Discard" on the confirm that guards the keys on the way
/// out.
pub const BUILD_DISCARD: Id = Id(1809);
/// Builder: "Keep" beside it.
pub const BUILD_KEEP: Id = Id(1810);
/// Builder: the script types, in `ScriptType::ALL` order.
pub const BUILD_SCRIPT_BASE: u32 = 1820;
/// Builder: one row per cosigner scanned into the wallet, by its place
/// among them; a tap takes it back out.
pub const BUILD_KEY_ROW_BASE: u32 = 1840;
/// Builder: the loaded keys the Keys step lists, by their index among
/// the loaded keys; a tap checks one and the next unchecks it.
pub const BUILD_WHICH_BASE: u32 = 1880;
/// Builder: the thresholds, 1 to the number of keys.
pub const BUILD_THRESHOLD_BASE: u32 = 1920;
/// Builder: Continue on the recovery path's Keys step.
pub const BUILD_LATER_CONTINUE: Id = Id(2300);
/// Builder: Continue on "How many must sign later?".
pub const BUILD_LATER_THRESHOLD_CONTINUE: Id = Id(2301);
/// Builder: Continue on "After how long?".
pub const BUILD_DELAY_CONTINUE: Id = Id(2302);
/// Builder: the recovery path's thresholds, 1 to the number of its keys.
pub const BUILD_LATER_THRESHOLD_BASE: u32 = 2310;
/// Builder: the waits, in `crate::build::DELAYS` order.
pub const BUILD_DELAY_BASE: u32 = 2330;
/// Builder: "Type a number of days", the row under those waits.
pub const BUILD_DELAY_TYPE: Id = Id(2303);
/// Builder: the pad that number is typed on.
pub const BUILD_DAYS_PAD: Id = Id(2304);
/// Builder: "Another path later?", its No row.
pub const BUILD_ANOTHER_NO: Id = Id(2305);
/// The same Choice's Yes.
pub const BUILD_ANOTHER_YES: Id = Id(2306);
/// Continue on that Choice.
pub const BUILD_ANOTHER_CONTINUE: Id = Id(2307);
/// Builder, from a key's page: "Passphrase?", its No row.
pub const BUILD_PASSPHRASE_NO: Id = Id(2308);
/// The same Choice's Yes.
pub const BUILD_PASSPHRASE_YES: Id = Id(2309);
/// Continue on that Choice.
pub const BUILD_PASSPHRASE_CONTINUE: Id = Id(1813);

// ----- FROST wallets (`crate::threshold`, `docs/PLANNING.md` §16.103,
// §16.104 rule 3) -----

/// "How many keys?": one row per count in `threshold::COUNTS`.
pub const THRESHOLD_COUNT_BASE: u32 = 2010;
/// Continue on that Choice.
pub const THRESHOLD_COUNT_CONTINUE: Id = Id(2019);
/// "How many must sign?": row `i` is a threshold of `i + 2`.
pub const THRESHOLD_QUORUM_BASE: u32 = 2020;
/// Continue on that Choice.
pub const THRESHOLD_QUORUM_CONTINUE: Id = Id(2029);
/// The group record screen: "Save".
pub const THRESHOLD_RECORD_SAVE: Id = Id(2051);
/// The group record screen: Continue.
pub const THRESHOLD_RECORD_CONTINUE: Id = Id(2053);
/// The Words screen's Continue, once per computed key.
pub const THRESHOLD_WORDS_CONTINUE: Id = Id(2055);
/// Sign: the chip of one member of a FROST wallet this device holds a
/// key of, by that member's identifier (`docs/PLANNING.md` §16.103).
pub const SIGN_SHARE_BASE: u32 = 2080;
/// Sign: the "Then with" chips at the first location, by the
/// identifier of the member each offers.
pub const SIGN_OTHER_BASE: u32 = 2100;

// ----- SLIP-39 shares this device writes (`crate::shares`,
// `docs/PLANNING.md` §16.107 rules 4 and 5) -----

/// Add a key: "Create SLIP-39 shares", the flow that makes a key whose
/// only written form is shares.
pub const ADD_CREATE_SLIP39: Id = Id(2400);
/// Backup: the "SLIP-39 shares" row of a SLIP-39 key's Backup menu.
pub const BACKUP_SLIP39: Id = Id(2401);
/// "How many groups?": row `i` is `i + 1` groups.
pub const SHARE_GROUPS_BASE: u32 = 2410;
/// Continue on that Choice.
pub const SHARE_GROUPS_CONTINUE: Id = Id(2402);
/// "How many groups must be present?": row `i` is `i + 1`.
pub const SHARE_QUORUM_BASE: u32 = 2430;
/// Continue on that Choice.
pub const SHARE_QUORUM_CONTINUE: Id = Id(2403);
/// "How many shares?": row `i` is `i + 1`.
pub const SHARE_COUNT_BASE: u32 = 2450;
/// Continue on that Choice.
pub const SHARE_COUNT_CONTINUE: Id = Id(2404);
/// "How many must be present?": row `i` is `i + 1`.
pub const SHARE_THRESHOLD_BASE: u32 = 2470;
/// Continue on that Choice.
pub const SHARE_THRESHOLD_CONTINUE: Id = Id(2405);
/// "Random shares from?": the rows, in [`osk_entropy::SOURCE_ROWS`]
/// order.
pub const SHARE_SOURCE_BASE: u32 = 2490;
/// Continue on that Choice.
pub const SHARE_SOURCE_CONTINUE: Id = Id(2406);

// ----- codex32 strings this device writes (`crate::codex32::Codex32Plan`,
// `docs/PLANNING.md` §16.109 rules 4 and 5) -----

/// Add a key: "Create Codex32 shares", the flow that makes a key whose
/// only written form is codex32 strings.
pub const ADD_CREATE_CODEX32: Id = Id(2500);
/// Backup: the "Codex32" row, on every key's Backup menu.
pub const BACKUP_CODEX32: Id = Id(2501);
/// "Split into shares?": the two rows.
pub const CODEX32_SPLIT_NO: Id = Id(2502);
/// The same Choice's "Yes".
pub const CODEX32_SPLIT_YES: Id = Id(2503);
/// Continue on that Choice.
pub const CODEX32_SPLIT_CONTINUE: Id = Id(2504);
/// "How many shares?": row `i` is `i + 1` shares.
pub const CODEX32_COUNT_BASE: u32 = 2540;
/// Continue on that Choice.
pub const CODEX32_COUNT_CONTINUE: Id = Id(2505);
/// "How many must be present?": row `i` is `i + 1`.
pub const CODEX32_THRESHOLD_BASE: u32 = 2580;
/// Continue on that Choice.
pub const CODEX32_THRESHOLD_CONTINUE: Id = Id(2506);
/// "Random shares from?": the rows, in [`osk_entropy::SOURCE_ROWS`]
/// order.
pub const CODEX32_SOURCE_BASE: u32 = 2620;
/// Continue on that Choice.
pub const CODEX32_SOURCE_CONTINUE: Id = Id(2507);
/// The keyboard "Type it back" is typed on.
pub const CODEX32_KEYBOARD: Id = Id(2508);
/// The string being typed back, whole, above the entry group.
pub const CODEX32_TYPED: Id = Id(2509);

// ----- A key's account for a coordinator (`docs/PLANNING.md` §16.110) -----

/// The key page's "Account key" row, which opens "Which account?".
pub const DETAIL_ACCOUNT: Id = Id(2700);
/// The keyboard the BSMS session token is typed on.
pub const BSMS_TOKEN_KEYBOARD: Id = Id(2702);
/// The preset row above that field, which writes `00`.
pub const BSMS_TOKEN_NONE: Id = Id(2703);
/// The keyboard the record's description is typed on.
pub const BSMS_DESCRIPTION_KEYBOARD: Id = Id(2704);
/// The Sign review's key context row, which opens the transaction's
/// Keys review (§16.110 rule 3).
pub const SIGN_KEYS: Id = Id(2705);
/// That review's rows: row `i` is the `i`th key the transaction names.
pub const SIGN_KEY_ROW_BASE: u32 = 2780;

// ----- Notes, recovery sheets and the form an export takes
// (`docs/PLANNING.md` §16.112) -----

/// Tools › Notes.
pub const TOOLS_NOTES: Id = Id(2900);
/// Notes › "New note".
pub const NOTES_NEW: Id = Id(2901);
/// Notes › "Read an encrypted backup".
pub const NOTES_READ_FILE: Id = Id(2902);
/// The keyboard a note is typed on.
pub const NOTE_KEYBOARD: Id = Id(2903);
/// The note document's "Export".
pub const NOTE_EXPORT: Id = Id(2904);
/// The note document's "Forget".
pub const NOTE_FORGET: Id = Id(2905);
/// A wallet page's "Recovery sheet" row.
pub const WALLET_SHEET: Id = Id(2906);
/// The sheet's "Write the note" row, which opens the same entry a note
/// is typed on.
pub const SHEET_NOTE: Id = Id(2907);
/// The sheet document's "Export".
pub const SHEET_EXPORT: Id = Id(2908);
/// An opened sheet's "Add this wallet".
pub const SHEET_ADD_WALLET: Id = Id(2909);
/// "Which form?": row `i` is [`crate::Form::ALL`]`[i]`.
pub const FORM_BASE: u32 = 2940;
/// Continue on that Choice.
pub const FORM_CONTINUE: Id = Id(2910);
/// The keyboard a note's or a sheet's passphrase is typed on.
pub const SEAL_PASS_KEYBOARD: Id = Id(2911);
/// The sealed file's "Save to file".
pub const SEAL_SAVE: Id = Id(2912);
/// The sealed file's "Show QR".
pub const SEAL_SHOW_QR: Id = Id(2913);
/// Done on the sealed file's Result and on its QR.
pub const SEAL_DONE: Id = Id(2914);
/// The note document's "Keep on this device" toggle (pass E3).
pub const NOTE_KEEP: Id = Id(2916);
/// The same toggle on a wallet's recovery sheet.
pub const SHEET_KEEP: Id = Id(2917);
/// The sheet's "Read an encrypted backup" row, which opens the scanner
/// Notes opens.
pub const SHEET_READ: Id = Id(2918);
/// Notes: row `i` is the `i`th kept note.
pub const NOTES_KEPT_BASE: u32 = 2980;
/// Notes: row `i` is the `i`th kept sheet whose wallet is not in use.
pub const NOTES_SHEET_BASE: u32 = 2990;
/// Settings › "Backup memory", which opens the Choice of costs.
pub const SETTINGS_BACKUP_MEMORY_ROW: Id = Id(2915);
/// That Choice: row `i` is [`crate::BACKUP_MEMORY`]`[i]`.
pub const SETTINGS_BACKUP_MEMORY_BASE: u32 = 2960;

// ----- BIP-85's other applications, and the Bitcoin Core import file
// (`docs/PLANNING.md` §16.114) -----

/// The key page's "BIP-85" row, which opens "Which application?".
pub const DETAIL_BIP85: Id = Id(3000);
/// The digits pad a password's length is typed on.
pub const BIP85_LENGTH_PAD: Id = Id(3001);
/// The digits pad the index is typed on.
pub const BIP85_INDEX_PAD: Id = Id(3002);
/// The derived value's panel, revealed by a finger on it or by
/// [`SECRET_EYE`].
pub const BIP85_REVEAL: Id = Id(3003);
/// Done on that Secret screen.
pub const BIP85_DONE: Id = Id(3004);
/// The export's "Save to file", on the format that writes one.
pub const EXPORT_SAVE: Id = Id(3005);

// ----- Silent payments (`docs/PLANNING.md` §16.113) -----

/// A silent payments wallet's "Address" row.
pub const SILENT_ADDRESS: Id = Id(3100);
/// Its "Labels" row.
pub const SILENT_LABELS: Id = Id(3101);
/// Its "Check a payment" row.
pub const SILENT_CHECK: Id = Id(3102);
/// The Address screen's Format value row, which opens the Choice of the
/// form the address is shown in.
pub const SILENT_FORM: Id = Id(3103);
/// That Choice: row `i` is [`crate::silent::AddressForm::ALL`]`[i]`.
pub const SILENT_FORM_BASE: u32 = 3140;
/// The Labels list's "Add a label" row.
pub const SILENT_ADD_LABEL: Id = Id(3104);
/// The Labels list: row `i` is label `i + 1`.
pub const SILENT_LABEL_BASE: u32 = 3160;
/// Check a payment's way in, which opens the scanner.
pub const SILENT_READ: Id = Id(3105);
/// The row that reads the previous transaction an input still needs.
pub const SILENT_PREVIOUS: Id = Id(3106);
/// Done on a check's Result.
pub const SILENT_DONE: Id = Id(3107);
/// The keyboard the DNS record's user name and domain are typed on.
pub const SILENT_DNS_KEYBOARD: Id = Id(3108);
/// The scan descriptor's panel on the export's Secret screen, revealed
/// by a finger on it or by [`SECRET_EYE`].
pub const SILENT_REVEAL: Id = Id(3109);
/// The export's row that opens the scan descriptor's Secret screen.
pub const SILENT_SECRET_ROW: Id = Id(3110);

// ----- The dice procedures (`docs/PLANNING.md` §16.115) -----

/// Create: the rows of "Which procedure?", in
/// `osk_entropy::DiceProcedure::ALL` order.
pub const CREATE_PROCEDURE_BASE: u32 = 3200;
/// Create: Continue on "Which procedure?".
pub const CREATE_PROCEDURE_CONTINUE: Id = Id(3210);

// ----- Tools › Lightning node key (`docs/PLANNING.md` §16.116) -----

/// The Tools row, which opens "From?".
pub const TOOLS_LIGHTNING: Id = Id(3400);
/// The passphrase keyboard of an LND cipher seed.
pub const LIGHTNING_PASS_KEYBOARD: Id = Id(3401);
/// The Result's reference row, which opens the node's public key whole.
pub const LIGHTNING_NODE_KEY: Id = Id(3402);
/// "Show secret" on the Result.
pub const LIGHTNING_SECRET: Id = Id(3403);
/// Done, on the Result and on the Secret screen.
pub const LIGHTNING_DONE: Id = Id(3404);
/// The Secret screen's panel, revealed by a finger on it or by
/// [`SECRET_EYE`].
pub const LIGHTNING_REVEAL: Id = Id(3405);
/// The Secret screen's action that shows the cipher seed's entropy.
pub const LIGHTNING_ENTROPY: Id = Id(3406);
/// The action beside it that shows the node private key again.
pub const LIGHTNING_NODE_SECRET: Id = Id(3407);

// ----- The vanity address grinder (`docs/PLANNING.md` §16.117) -----

/// The key page's "Vanity address" row, which opens "How?".
pub const DETAIL_VANITY: Id = Id(3500);
/// "Which script type?": row `i` is `ScriptType::ALL[i]`.
pub const VANITY_SCRIPT_BASE: u32 = 3520;
/// Continue on that Choice.
pub const VANITY_SCRIPT_CONTINUE: Id = Id(3501);
/// The address keyboard the prefix is typed on.
pub const VANITY_KEYBOARD: Id = Id(3502);
/// Stop, on the screen that states how far the grind has gone.
pub const VANITY_STOP: Id = Id(3503);
/// The Result's reference row, which opens the address whole.
pub const VANITY_ADDRESS: Id = Id(3504);
/// "Use it" on the Result.
pub const VANITY_USE: Id = Id(3505);
/// The action beside it that shows the passphrase counter.
pub const VANITY_SHOW: Id = Id(3506);
/// That Secret screen's panel, revealed by a finger on it or by
/// [`SECRET_EYE`].
pub const VANITY_REVEAL: Id = Id(3507);
/// Done, on that Secret screen.
pub const VANITY_DONE: Id = Id(3508);
