//! Everything a press can do.

use crate::{
    Code, Column, Screen, backup_flow, board, boot_import, compact, explore, family, file_menu,
    glance, in_progress, nav, seeds, set_time, ui, vanity, vault, wordlist,
};

/// Everything a press can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Go to a screen.
    Nav(Screen),
    /// Back: the screen's own way back for what it lets go, then the
    /// screen seen before this one, as it was seen (F77).
    Back,
    /// Back to a flow in progress, as it was left: its row in the
    /// sidebar, its tile on the small Home (`docs/FLOWS.md`, Flows in
    /// progress).
    Resume(in_progress::Going),
    /// **Cancel** at the top of a flow that can be in progress: it ends
    /// at once, or asks first when something would be lost
    /// (`docs/FLOWS.md`, Flows in progress, Cancel; F51).
    CancelFlow,
    /// **Cancel the flow** on the sheet Cancel brought up.
    CancelFlowNow,
    /// Open Backups on loaded wallet `i`.
    BackupsOf(usize),
    /// Open Back up a wallet on loaded wallet `i` at its checklist, from
    /// the plan its chart shows (`docs/history/NEW-WALLET.md` §6.2). Nothing is
    /// made For the stick by opening it: each item offers what it makes.
    BackupChecklist(usize),
    /// Finish the backup first (`docs/history/NEW-WALLET.md` §14.3): wallet `i`'s
    /// checklist, or its plan when none is made.
    BackupFirst(usize),
    /// Open, or close, the wallet's chart as its own page on a small
    /// panel.
    Glance(bool),
    /// A press on a stick's pill across the top of the window: 0 the boot
    /// stick, `n` Stick `n`. The shell inserts the stick or pulls it.
    StickPill(u8),
    /// A top app bar's ⋮ menu (`docs/LAYOUT.md` §3): the bar's windows'
    /// menu, or a compact bar's overflow.
    NavMenu(nav::MenuPress),
    /// A press on the wallet's chart or one of its sheets
    /// (`docs/history/NEW-WALLET.md` §9).
    Chart(glance::sheet::ChartAction),
    /// A press in the backup flow (`docs/BACKUP-FLOW.md`).
    Flow(backup_flow::steps::FlowAction),
    /// Show the Backups screen's wallet `i`, a page each on a small panel.
    BackupsPage(usize),
    /// A vault screen's action.
    Vault(vault::VaultAction),
    /// Open seed entry, for a slot's fingerprint or for any key.
    Entry(Option<[u8; 4]>),
    /// Add the typed key.
    EntryAdd,
    /// Type the words from another list, by its place in
    /// `forms::LANGUAGES`.
    EntryLanguage(u8),
    /// A key of the on-screen keyboard a list's words are typed on.
    EntryKey(char),
    /// That keyboard's backspace.
    EntryKeyBack,
    /// That keyboard's shift (Korean's doubled consonants).
    EntryShift,
    /// Take the word offered at this place among the candidates.
    EntryCandidate(u8),
    /// Show the lists other than English, or hide them.
    EntryLanguages,
    /// Type another form of key, by its place in `forms::Form::ALL`.
    EntryForm(u8),
    /// Shows the SLIP-39, codex32 and Seed XOR chips.
    EntryOtherForms,
    /// The typed share, string or part, collected.
    EntryPart,
    /// The collected parts put together as a key.
    EntryRecover,
    /// Clear the typed words.
    EntryClear,
    /// How many words the key has: 12, 15, 18, 21 or 24 boxes shown.
    EntryWordCount(u8),
    /// Choose a wallet on the Wallets tab.
    PickWallet(usize),
    /// Choose a key in no wallet on the Wallets tab, by fingerprint.
    PickKey([u8; 4]),
    /// Remove a wallet from the session.
    RemoveWallet(usize),
    /// Remove every key from the session.
    RemoveKeys,
    /// Start signing the Inbox PSBT at this index.
    StartSpend(usize),
    /// Open or close a step card.
    Step(u8),
    /// Close a step card as done and open the next.
    StepNext(u8),
    /// Review's hold past a holdable danger (F136): the dangers are
    /// accepted for signing and Review closes as done.
    ReviewHold,
    /// Show or hide the full transaction table.
    ToggleTable,
    /// Sign with every session key the transaction names.
    SignHere,
    /// Add the signatures in this Inbox file.
    Collect(usize),
    /// Make the signed PSBT For the stick again, after a Remove. Finish
    /// makes it the moment the transaction is finished.
    SignedToOutbox,
    /// Make the finished transaction For the stick again, after a Remove.
    TxToOutbox,
    /// The summary panel's action.
    Primary,
    /// Load the wallet in this Inbox file.
    LoadWallet(usize),
    /// Remove an Inbox file.
    InboxRemove(usize),
    /// Remove an Outbox file.
    OutboxRemove(usize),
    /// Show this stick on the visit screen.
    VisitStick(usize),
    /// Choose an Outbox file to write.
    VisitOut(usize),
    /// Tick or untick the settings file on a stick visit.
    VisitSettings,
    /// Choose a stick file to copy.
    VisitIn(usize),
    /// Unselect all: none of the stick's files comes in.
    VisitInAll,
    /// Choose an Inbox file to write to the stick, by its index in the
    /// Inbox: one that may be a secret opens the secret sheet first.
    VisitInbox(usize),
    /// Choose the settings and every Outbox file but an unprotected
    /// secret, or none when all of those are chosen.
    VisitOutAll,
    /// A stick visit column's scrollbar: held and dragged.
    VisitBar(Column),
    /// Write the chosen Outbox files: first the Already on the stick
    /// sheet, when a name is taken there.
    VisitWrite,
    /// That sheet's Replace: the taken files written over the old ones.
    VisitReplace,
    /// That sheet's Keep both: the taken files written beside the old
    /// ones, under the names the sheet shows.
    VisitKeepBoth,
    /// Copy the chosen stick files.
    VisitCopy,
    /// Copy the chosen stick files, and load as keys any that are once
    /// the stick is pulled.
    VisitCopyAndLoad,
    /// Load the key an Inbox file's words spell.
    LoadKey(usize),
    /// Scan a SeedQR into Add a key.
    ScanSeed,
    /// Scan any missing key of loaded wallet n.
    ScanKey(usize),
    /// Add a key for loaded wallet n: its words typed, for any of its
    /// missing keys.
    EntryWallet(usize),
    /// Scan one part of a key into Add a key's other forms: a SLIP-39
    /// share, a codex32 string, or a Seed XOR part's SeedQR or words.
    ScanPart,
    /// Type the BIP-39 passphrase in Add a key, or go back to the words.
    EntryPassphrase,
    /// A press in Add a key's empty field: focuses it, and the Use a
    /// sample chip is gone for the visit.
    EntryField,
    /// Seal the signed-amount memory into an open vault on lock, or not.
    SealAmounts(bool),
    /// Lock: wipe the session and start a fresh process.
    Lock,
    /// Lock with the open vaults' changes not sealed, asked for when the
    /// system has given no randomness to seal with: they are lost.
    LockUnsealed,
    /// Lock, first listing what is not saved in a vault when one is open.
    LockAsk,
    /// Leave the stick untouched and keep working.
    NotNow,
    /// Ask to power off.
    PowerAsk,
    /// The device check's Check again.
    DevicesCheck,
    /// The device check's Continue anyway: asks first.
    DevicesContinue,
    /// The hold under Continue anyway's question: lets the items through
    /// for this session (F136).
    DevicesHold,
    /// I understand, under the reminders before a secret.
    DevicesUnderstood,
    /// Power off.
    PowerOff,
    /// Restart the computer: Power off in everything but its end.
    Restart,
    /// Power off (or Quit), held: the power sheet's while files wait for
    /// the stick or a vault has unsaved changes (decision 5).
    HoldPowerOff,
    /// Restart, held, as [`Action::HoldPowerOff`].
    HoldRestart,
    /// The key path sheet's hold: the waiting wallet is added (F132).
    KeyPathHold,
    /// The key path sheet's Cancel: the waiting wallet is not added.
    KeyPathCancel,
    /// The Spend review's **Register this wallet**: the sheet opens on
    /// the wallet the PSBT defines.
    RegisterOpen,
    /// The register sheet's name field: typing goes to it.
    RegisterField,
    /// The register sheet's hold: the wallet is checked against the
    /// spend's scripts and added.
    RegisterHold,
    /// The register sheet's Cancel: nothing is registered.
    RegisterCancel,
    /// The register sheet's choice of reading where the spend allows
    /// two: 0 `sortedmulti`, 1 `multi`.
    RegisterReading(u8),
    /// Minutes without input before locking; 0 for never.
    IdleLock(u16),
    /// Minutes without input before powering off; 0 for never.
    IdleOff(u16),
    /// Believe a new pointer, said with input already believed.
    InputUse(u32),
    /// Keep a new device out until it is unplugged.
    InputIgnore(u32),
    /// The target a new touchscreen taps to be believed; pressed with
    /// input already believed, it believes the device as
    /// [`Action::InputUse`] does.
    InputTarget(u32),
    /// One of wallet n's public files to the Outbox, as `public_out`
    /// numbers them: what the backup's public files item offers.
    PublicOut(usize, u8),
    /// Remove from For the stick the files [`Action::PublicOut`] makes
    /// for the same wallet and number.
    PublicRemove(usize, u8),
    /// Show the public files of wallet n that are under More options
    /// (F64), until the item is left.
    PublicMore(usize),
    /// The account key in wallet n's slot k, held here, to the Outbox
    /// for the cosigners.
    WalletKeyOut(usize, u8),
    /// That key as a BIP 129 key record, signed, to the Outbox.
    WalletKeyBsms(usize, u8),
    /// Open the stick visit from a flow, which it returns to when the
    /// stick is pulled.
    VisitFrom(Screen),
    /// Restore the wallet in record r of open vault v, with the vault's
    /// keys for its slots.
    RFromVault(usize, usize),
    /// Choose this mode: a lock, and a fresh session in the mode (F81).
    Mode(crate::modes::Mode),
    /// Open the mode sheet: the pill, or Settings' Mode row.
    ModeSheet,
    /// Open Load samples: its pill, in Practice with samples set (F81).
    Samples,
    /// Tick or untick a row of the Load samples sheet.
    SampleTick(crate::samples::Pick),
    /// Untick every row while any is ticked, else tick every one not
    /// loaded.
    SamplesAll,
    /// Load what the Load samples sheet has ticked.
    SamplesLoad,
    /// Use a sample: the chip inside an empty field where a key is typed
    /// (Practice, with samples set).
    UseSample,
    /// The `faraday` chip inside an empty passphrase field, in Practice: fills
    /// that field with it.
    PassChip(crate::demo::PassField),
    /// The scanner's Sample tile: reads a sample as the camera would.
    ScanSample,
    /// Load sample PSBT, in a step that asks for the transaction: the
    /// sample spend comes in as a scan of it would bring it.
    SamplePsbt,
    /// Close a sheet.
    Cancel,
    /// Set the display scale, in percent.
    Scale(u16),
    /// Show the app this way round on a panel that can turn
    /// (Settings › Display › Rotation, `docs/ORIENTATION.md` §6).
    Rotation(faraday_shell_api::Orientation),
    /// Settings' section `k` opened as its own page from the list of
    /// sections on a compact window, or none: back to the list
    /// (`docs/LAYOUT.md` §17).
    SettingsSection(Option<u8>),
    /// Draw in this scheme: a theme, mode and contrast (`docs/THEMES.md` §12).
    Theme(ui::Theme),
    /// Draw text at this size (Settings › Appearance, `docs/TEXT-SIZE.md`
    /// §2).
    TextSize(ui::TextSize),
    /// Reduce motion on or off.
    ReduceMotion(bool),
    /// Guided mode on or off.
    Guided(bool),
    /// Steps the time zone by this many minutes, stopping at the ends
    /// (`docs/TOTP.md` §2.1). Settings' row and Set the time's own row.
    TzStep(i16),
    /// Opens Set the time from the current screen, which it returns to.
    OpenSetTime,
    /// Where typing goes on Set the time.
    SetTimeFocus(set_time::Focus),
    /// Set the time's Set button.
    SetTimeSet,
    /// Set the time's Keep this time button.
    SetTimeKeep,
    /// Set the time's Back.
    SetTimeBack,
    /// Opens the TOTP codes screen (`docs/TOTP.md` §5.2): no vault open
    /// leads to Create or Unlock first, and the clock not checked this
    /// power-on opens Set the time first.
    OpenCodes,
    /// A codes screen row pressed: the vault and the entry's position
    /// among it, as `vaults.item` holds it.
    CodesOpen(usize, usize),
    /// Add an entry from the codes screen.
    CodesAdd,
    /// On a small panel, a step's walk-through, or a page's, shown or
    /// put away: the step's place, or [`ABOUT_PAGE`] for the page.
    About(u8),
    /// Show the spend's signed PSBT as a QR code.
    QrSigned,
    /// Show the spend's PSBT as it stands, for a cosigner.
    QrPart,
    /// Show a wallet's descriptor as a QR code.
    QrWallet(usize),
    /// Show an Outbox file as a QR code.
    QrOutbox(usize),
    /// Transfer: send file n of the Downloads list as codes.
    TransferSend(usize),
    /// Transfer: receive by camera into Downloads.
    TransferReceive,
    /// Transfer: open Downloads in the file manager.
    TransferOpenFolder,
    /// About: open nakamotoinstitute.org in this computer's browser; the
    /// online app only.
    OpenInstitute,
    /// The sheet of a seed the camera read.
    SeedRead(seeds::read::Choice),
    /// The Sweep sheet: a private key swept (`seeds::sweep`, F171).
    Sweep(seeds::sweep::SweepAction),
    /// The stick visit's Delete files: the sheet that chooses what.
    DeleteFiles,
    /// The sheet's choice: chosen files, the free space, the whole
    /// partition, by place.
    DeleteWhat(u8),
    /// A file of the stick ticked or unticked on the sheet, by place.
    DeletePick(usize),
    /// One pass or two, for the whole partition.
    DeletePasses(u8),
    /// The sheet's button: delete or zero what is chosen.
    DeleteGo,
    /// Stop zeroing after the step under way.
    ZeroStop,
    /// The QR view as BBQr (`true`) or UR.
    QrFormat(u8),
    /// The QR view's bytes of data a frame.
    QrPartSize(usize),
    /// Milliseconds each frame of an animated code stays up.
    QrSpeed(u64),
    /// The QR sheet's code as a labelled picture, to the Outbox.
    QrPng,
    /// A public file a flow makes, shown as a QR code.
    ShowCode(Code),
    /// A public file a flow makes, as a labelled picture of its code to
    /// the Outbox: what the QR sheet's PNG makes, without the sheet.
    CodePng(Code),
    /// Open a wallet's card.
    OpenWallet(usize),
    /// Start backing up a wallet.
    Backup(usize),
    /// Open or close the list of loaded wallets under the backup's
    /// wallet chip.
    BWallets,
    /// Open or close an item of the backup's checklist, by [`bstep`].
    BStep(u8),
    /// Open the checklist item after this one; on the envelopes, close
    /// them as done.
    BNext(u8),
    /// Open or close a question of the backup's plan, by [`qstep`].
    BQ(u8),
    /// Open the plan's question after this one.
    BQNext(u8),
    /// Tick or untick a row of the plan's Software card (the software, the
    /// form; the wallet's flow's own plan asks the rest): the list by
    /// [`qrow`], then the row.
    BAnswer(u8, u8),
    /// Start or stop typing place n's name (only with a vault open).
    BName(u8),
    /// A press on the backup board (`docs/BACKUP-FLOW.md` §17).
    Board(board::BoardPress),
    /// The plan made: on to the checklist, the plan saved into the open
    /// vault.
    BChecklist,
    /// Back from the checklist to the plan.
    BPlan,
    /// Show the seed of this session key.
    BKey(usize),
    /// Show or hide the words.
    BReveal,
    /// Standard or compact SeedQR.
    BCompact(bool),
    /// Mark this block of the SeedQR grid, or this row where it is marked
    /// by rows.
    BPin(usize),
    /// Mark the SeedQR grid by rows (`true`) or by blocks.
    BRows(bool),
    /// Start or stop typing the digits back.
    BCheck,
    /// Scan the copy of the seed on screen.
    BScan,
    /// Clear the typed digits.
    BCheckClear,
    /// Keep the words only: the open copy item's SeedQR copies become
    /// copies of the words (`docs/history/NEW-WALLET.md` §14.7 item 3).
    BKeepWords,
    /// Vault `v`'s item: save into the open vault, in one press,
    /// everything the plan puts in that vault: its seeds, each passphrase
    /// the plan keeps there, and the wallet where the plan puts it there.
    BVaultSave(u8),
    /// Vault `v`'s item: turn **Load at unlock** off or on for the
    /// wallet's record in that vault (F62).
    BVaultLoad(u8),
    /// Make the checklist's public files again: the template, the sheet
    /// or the shares, the files for the software.
    BFilesMake,
    /// A vault's item, the open vault holding another vault's seed: lock
    /// that vault and make a new one, back to the item.
    BNewVault(u8),
    /// The seed shown as a file: the secret sheet for it.
    BFile,
    /// The template's word count.
    BWords(usize),
    /// Keys left off each split sheet.
    BOmit(usize),
    /// Start signing a message.
    SignMessage,
    /// Open or close a message card.
    MStep(u8),
    /// Close a message card as done and open the next.
    MNext(u8),
    /// The wallet to sign with.
    MWallet(usize),
    /// Move the address index by this much.
    MIndex(i32),
    /// Start or stop typing the message.
    MType,
    /// BIP-322 (true) or BIP-137.
    MFormat(bool),
    /// Sign the message.
    MSign,
    /// Put the signed message in the Outbox.
    MOut,
    /// Show the signed message as a QR code.
    MQr,
    /// Check the signed message in this Inbox file.
    CheckMessage(usize),
    /// Start creating a wallet.
    CreateWallet,
    /// Open or close a create card.
    CStep(u8),
    /// Close a create card as done and open the next.
    CNext(u8),
    /// The kind, by index into `NewKind::ALL`.
    CKind(u8),
    /// Expands the Kind card's other eight kinds.
    CMoreKinds,
    /// The info button after "codex32" on the Kind card: shows its
    /// tooltip at once (F112).
    CKindAbout,
    /// Answers the question of vaults on Make a new wallet: yes or no.
    CVaults(bool),
    /// The same on Restore.
    RVaults(bool),
    /// Sets how many vaults Make a new wallet sets up.
    CVaultN(u16),
    /// The same on Restore.
    RVaultN(u16),
    /// **Add a passphrase** on the plan's check lines: Create opens on
    /// its Keys with the key in its slot and the fields open
    /// (`docs/BACKUP-FLOW.md` §10.3).
    PlanAddPass,
    /// Change the signatures needed.
    CM(i8),
    /// Change the number of keys.
    CN(i8),
    /// Fill a slot with a key loaded here.
    CSlotHere(u8, [u8; 4]),
    // NEW-WALLET §16 (F45): keys from a vault in Create; a new wallet
    // from one held.
    /// Fill slot s with key record r of open vault v, loading that key
    /// (`docs/history/NEW-WALLET.md` §16.1).
    CSlotVault(u8, usize, usize),
    /// The Account row (F98): 0 its **Change**, opening the stepper;
    /// −1 and +1 the stepper (`docs/history/NEW-WALLET.md` §16.2, §17).
    CAccount(i8),
    /// Check's Path row **Change**: the made wallet taken back, the
    /// Account row's card open on its stepper (F98).
    CPathChange,
    /// A wallet card's **New wallet from this one**: Create filled from
    /// wallet i (`docs/history/NEW-WALLET.md` §16.2).
    NewWalletFrom(usize),
    /// The secret on its way out into the open vault.
    SecretVault,
    /// **Hold to save without encryption**: the secret, a plan's file or a
    /// received file saved as a plain file (F129). Acts when held.
    SecretHold,
    /// The secret sheet's **Unprotected**: asks for the hold.
    SecretUnprotected,
    /// The form the secret's file takes, by index.
    SecretForm(u8),
    /// Open BIP-85.
    Bip85,
    /// Open silent payments.
    Silent,
    /// Open Explore.
    Explore,
    /// Open Explore on this loaded seed, by fingerprint.
    ExploreKey([u8; 4]),
    /// Open the Lightning node key.
    Lightning,
    /// Open Tools.
    Tools,
    /// A calculator, by its place in `Tool::ALL`.
    TTool(u8),
    /// The calculator's mode row.
    TMode(u8),
    /// Empty the calculator's field.
    TClear,
    /// The node key from this loaded key, as ldk-node reads it.
    LKey([u8; 4]),
    /// The node key from an aezeed typed here.
    LAezeed,
    /// Type into the aezeed's passphrase, or back into its words.
    LPassphrase,
    /// Read the aezeed.
    LResolve,
    /// Show or hide the private key.
    LShow,
    /// The private key into the open vault.
    LVault,
    /// The private key out unprotected, through the secret sheet.
    LOut,
    /// Explore this key, by fingerprint.
    XKey([u8; 4]),
    /// A path preset, by its place in `explore::PRESETS`.
    XPreset(u8),
    /// The script for a path that names none.
    XScript(u8),
    /// The path's last index up or down.
    XIndex(i32),
    /// A press on Explore's address list or its search (F76).
    X(explore::XAction),
    /// Open or close a silent payments card.
    SStep(u8),
    /// The key, by fingerprint.
    SKey([u8; 4]),
    /// The label up or down.
    SLabel(i8),
    /// The address as a QR code: as it is, or as a `bitcoin:` link.
    SQr(bool),
    /// The public record to the Outbox.
    SRecord,
    /// The scan key into the open vault.
    SScanVault,
    /// The scan key out unprotected, through the secret sheet.
    SScanOut,
    /// Done with this silent payments card.
    SNext,
    /// Add the key's silent payments wallet to the session's wallets.
    SAddWallet,
    /// Open session wallet n, a silent payments wallet, on its page.
    SWallet(usize),
    /// Check whether Inbox file n, a transaction, pays the wallet.
    SCheck(usize),
    /// Open or close a BIP-85 card.
    PStep(u8),
    /// The key BIP-85 derives from, by fingerprint.
    PKey([u8; 4]),
    /// The application, by its place in `bip85::APPS`.
    PApp(u8),
    /// The length up or down a step.
    PLength(i8),
    /// The index up or down.
    PIndex(i32),
    /// Done with this BIP-85 card.
    PNext,
    /// Show or hide the value.
    PShow,
    /// The value into the open vault.
    PVault,
    /// Child words as a key of this session.
    PLoad,
    /// **Record it as a child seed?** answered: recorded (true) or not.
    PRecord(bool),
    /// The value out unprotected, through the secret sheet.
    POut,
    /// Split the key's words into this many Seed XOR parts.
    BXor(u8),
    /// Put the other paper form away.
    BPaperHide,
    /// Read with this camera, by its place in the shell's list.
    ScanCamera(u8),
    /// The scanner's Paste: the clipboard's text, read as a QR.
    ScanPaste,
    /// Paste words, on Faraday Demo: the clipboard's text as the whole of
    /// the words form's words (F75).
    PasteWords,
    /// Open the Learn pages for the screen on show.
    Learn,
    /// The docked keyboard of a small panel.
    Osk(compact::OskPress),
    /// Learn's supporting pane's close button: puts the pane away
    /// (`docs/LAYOUT.md` §6).
    LearnClose,
    /// Learn's supporting pane's pin: kept open across screens while on,
    /// put away at the next screen while off (`docs/LAYOUT.md` §6).
    LearnPin,
    /// Show the Learn sheet's page, or its How tab for
    /// [`crate::learn::HOW_TAB`].
    LearnPage(u8),
    /// **How Faraday does it** on the top app bar: Learn on the How tab
    /// (`docs/CRYPTO-LEARN.md` §8.2).
    How,
    /// Tick step `n` of the guide page on show in the Learn sheet, or
    /// clear its tick if it is ticked already.
    LearnStep(u32),
    /// **Spending, step by step** above Learn's tabs on the Spend tab:
    /// closes Learn over the tab's steps.
    LearnSpend,
    /// **What is a share?** beside the backup plan's In shares: the
    /// Learn sheet on the page about shares (`docs/history/NEW-WALLET.md` §4.2).
    LearnShares,
    /// The Learn page a Guided card links to, under its text.
    LearnTopic(crate::learn::Topic),
    /// The word-list sheet, or a link to it.
    WordList(wordlist::WordListAction),
    /// Make a new key, for this Create slot or for the session; for the
    /// session it is Add a key, opening on Start (`docs/history/NEW-WALLET.md`
    /// §15.3).
    KeyGen(Option<u8>),
    /// Open or close a new-key card.
    KStep(u8),
    /// A choice on New key's Start card, by its place in
    /// `keygen::STARTS`.
    KStart(u8),
    /// The Start card's Other forms, shown or hidden.
    KOthers,
    /// A word count pressed on the Length card: taken, and Randomness
    /// opens; SLIP-39's Continue presses the share count chosen.
    KWords(u8),
    /// The Length card's Other: the menu of the counts besides 12 and 24.
    KOtherLengths,
    /// Choose an option of the Randomness card, by its place in
    /// `keygen::WAYS`.
    KWay(u8),
    /// Open or close a group of the Randomness card: 0 the options
    /// verifiable by hand, 1 the ones the device computes.
    KGroup(u8),
    /// Take a source into a mix or out of it, by its place in
    /// `keygen::Source::MIXABLE`.
    KMix(u8),
    /// A die's face.
    KRoll(u8),
    /// A press on the one-die roller beside New key's dice pad (Faraday
    /// Desktop, `docs/CODEX32.md` §5.6).
    KRoller(crate::fresh::roller::Act),
    /// A press on Add randomness, or one that opens it
    /// (`docs/OWN-RANDOMNESS.md` §2.2).
    Own(crate::fresh::page::Act),
    /// A coin: heads or tails.
    KFlip(bool),
    /// A die's face, read as a coin flip: 1 to 3 tails, 4 to 6 heads.
    KDie(u8),
    /// Coin flips entered as a die's faces (true) or as a coin's sides.
    KByDie(bool),
    /// Entries typed as one string into a box (true), or pressed one at
    /// a time.
    KTyping(bool),
    /// A card's rank.
    KRank(u8),
    /// A card's suit.
    KSuit(u8),
    /// A hex digit's value.
    KHex(u8),
    /// Take the camera's picture.
    KFrame,
    /// Take back the last entry.
    KUndo,
    /// Forget this source's entries.
    KClear,
    /// Done with this card.
    KNext,
    /// Run the self-test again (Settings › About).
    SelfTestRun,
    /// Show or hide the words.
    KShow,
    /// The Quiz's last button: loads SLIP-39 shares' key, or leaves the
    /// flow once a BIP-39 key is locked in.
    KAdd,
    /// Lock in, on New key's Key card: adds the key.
    KLock,
    /// Typing goes to the Key card's passphrase field: 0 the first, 1
    /// the second.
    KPassField(u8),
    /// The quiz's candidate in this slot.
    KQuiz(u8),
    /// Ask the word that was answered wrongly again.
    KQuizRetry,
    /// Skip the quiz: the first press asks, the second skips.
    KSkip,
    /// Throw the entries away and start again.
    KAgain,
    /// A SLIP-39 share's word count, chosen without leaving the card.
    KSlipWords(u8),
    /// Shares needed, one more or one fewer.
    KSlipM(i8),
    /// Shares dealt, one more or one fewer.
    KSlipN(i8),
    /// The next or the previous share on screen.
    KShare(i8),
    /// Open New key making SLIP-39 shares, for the session.
    KeyGenSlip39,
    /// A Spend tab action.
    Family(family::FamilyAction),
    /// A vanity screen action.
    Vanity(vanity::VanityAction),
    /// Fill a slot with the account key in an Inbox file.
    CSlotFile(u8, usize),
    /// Empty a slot.
    CSlotClear(u8),
    /// Open or close the passphrase fields under a slot whose key was
    /// loaded from BIP-39 words without one.
    CPassOpen(u8),
    /// Typing goes to that passphrase field: 0 the first, 1 the second.
    CPassField(u8),
    /// Lock in the slot's key with the passphrase typed.
    CPassLock,
    /// Leave a slot for a cosigner's key, to add later.
    CSlotLater(u8),
    /// This slot's account key to the Outbox, for the cosigners.
    CKeyOut(u8),
    /// This slot's account key as a QR code.
    CKeyQr(u8),
    /// This slot's key as a BIP 129 key record, signed, to the Outbox.
    CKeyBsms(u8),
    /// Throw away the creation under way and start again.
    CreateOver,
    /// Check's **Back up wallet** in Make a new wallet: Back up on the
    /// wallet just made, the board open on The standard way
    /// (`docs/BACKUP-FLOW.md` §17.21 item 2).
    CBackup,
    /// Start restoring a wallet.
    RestoreWallet,
    /// Open or close a restore card.
    RStep(u8),
    /// Close a restore card as done and open the next.
    RNext(u8),
    /// Restore the wallet in this Inbox file.
    RUse(usize),
    /// Take or leave the share in this Inbox file.
    RShare(usize),
    /// Rebuild the wallet from the shares chosen.
    RRebuild,
    /// Description's **I have the seed** (a single key) or **I do not
    /// have it** (several): the wallet is made from the
    /// seeds and the cosigners' xpubs at the kind's standard path.
    RSeeds,
    /// Restore's Kind, by its place in `NewKind::ALL`
    /// (`docs/history/NEW-WALLET.md` §12.1).
    RKind(u8),
    /// Restore's Kind card shows every kind.
    RMoreKinds,
    /// The shortcut's **From Files**: the wallet description in Files.
    RFromFiles,
    /// A key of the description whose seed is not here, or here after
    /// all, by its place in the wallet.
    RAbsent(u8),
    /// The open slot's **Type the words**: Add a key, which asks the
    /// passphrase.
    RSlotWords,
    /// The open slot's **Scan a SeedQR**, whose sheet asks the
    /// passphrase.
    RSlotScan,
    /// A wallet from a loaded key in no wallet, by fingerprint: Restore's
    /// Seeds card with that key in its first slot, with this many keys
    /// (1 for a wallet of that key alone, 2 to add another).
    KeyWallet([u8; 4], u8),
    /// A wallet made from the seeds in hand (`seeds.rs`).
    Seeds(seeds::SeedsAction),
    /// A slider pressed, dragged or stepped: which slider, the value.
    Slide(u8, u8),
    /// Rebuild the wallet from every share in the Inbox, and open it.
    RestoreShares,
    /// Read QR codes with the camera into Files.
    Scan,
    /// Type a new name for the chosen wallet.
    Rename,
    /// Make a PDF of an Inbox sheet (online only).
    PdfInbox(usize),
    /// Make a PDF of an Outbox sheet (online only).
    PdfOutbox(usize),
    /// Make a backup file For the stick again: 0 template, 1 descriptor,
    /// 2 multisig config, 3 backup sheet, 4 split shares, 5 wallet .json.
    /// Making the checklist makes the ones it calls for.
    BOut(u8),
    /// Decode the transaction this spend finished.
    DecodeFinished,
    /// Open the Tools page's tile n.
    Catalog(u8),
    /// Load the wallets the Inbox describes, with the seeds here.
    InboxLoad,
    /// Leave out, or take back, the Inbox's wallet n in its next load.
    InboxChoose(usize),
    /// Make the seed with this fingerprint a wallet.
    PotentialOpen([u8; 4]),
    /// Make the Inbox's account xpub n a watch-only wallet.
    XpubOpen(usize),
    /// The potential wallet's kind, by its place in `inbox::KINDS`.
    PotentialKind(u8),
    /// Show or mask the potential wallet's passphrase.
    PotentialShow,
    /// Typing goes to the potential wallet's passphrase.
    PotentialType,
    /// Make the potential wallet, or open the backup.
    PotentialMake,
    /// Decode the transaction in this Inbox file.
    DecodeInbox(usize),
    /// A press on the nonce check of a transaction brought in
    /// (`Screen::NonceCheck`).
    NonceCheck(crate::decode::CheckPress),
    /// Show or hide the decoded transaction's hex.
    DecodeHex,
    /// The decoded transaction as a QR code.
    QrDecoded,
    /// Show or hide the finished transaction's hex on the spend.
    SpendHex,
    /// Write the Outbox to a stick: the visit now when nothing secret was
    /// held, else what a lock wipes and keeps first.
    WriteAsk,
    /// The boot import's sheet.
    Import(boot_import::ImportAction),
    /// A press on a screen the app holding this one draws with
    /// [`ui::Ui`] (the Starter, `docs/STARTER.md` §2), in that app's own
    /// numbering. This app does nothing with it.
    Host(u32),
    /// A file row's ⋮, its menu, Rename and Details (`docs/FLOWS.md`
    /// decision 61).
    File(file_menu::FilePress),
    /// A press inside Make codex32 shares (`docs/CODEX32.md` §6).
    Codex32(crate::codex32::C32),
    /// A press on the codex32 form's error correction
    /// (`docs/CODEX32.md` §8.4).
    Codex32Correct(crate::codex32_correct::Press),
    /// A press on the codex32 form's own rows: a string at an index
    /// already in, Check a string (`docs/CODEX32.md` §8.3, §8.5).
    Codex32Form(crate::forms::C32Press),
    /// A press on the backup checklist's codex32 item
    /// (`docs/CODEX32.md` §9.2, §9.4, §9.7).
    BC32(crate::codex32_backup::Press),
    /// A mark on a backup board chip that is no button: it only owns the
    /// plain tooltip that names it, and is never pressed.
    Mark(crate::board::Chip, MarkTip),
}

/// Which mark of a backup board chip a tooltip names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkTip {
    /// The warning mark.
    Warning,
    /// The link mark.
    Link,
}

/// Whether an action acts when held long enough rather than on release.
pub fn is_hold(a: Action) -> bool {
    matches!(
        a,
        Action::Vault(
            crate::vault::VaultAction::HoldDelete
                | crate::vault::VaultAction::HoldWipe
                | crate::vault::VaultAction::SbHoldSign
                | crate::vault::VaultAction::GpgHoldSign
                | crate::vault::VaultAction::GpgHoldRevoke
        ) | Action::Flow(crate::backup_flow::steps::FlowAction::Hold)
            | Action::HoldPowerOff
            | Action::HoldRestart
            | Action::SecretHold
            | Action::KeyPathHold
            | Action::Sweep(seeds::sweep::SweepAction::Hold)
            | Action::RegisterHold
            | Action::ReviewHold
            | Action::DevicesHold
    )
}
