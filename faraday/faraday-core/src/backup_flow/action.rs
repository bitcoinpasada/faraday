//! What a press in the backup flow can be: the cards' presses
//! ([`FlowAction`]), and those of a GPG key restored from its paperkey
//! ([`RestorePress`], `docs/GPG-PAPERKEY.md` §4); and what a kind's build
//! gives the flow ([`Built`]). Data only; what each press does is
//! `backup_flow/steps.rs` and `backup_flow/gpg_restore.rs`.

use crate::backup_flow::Kind;

/// Everything a press in the flow can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowAction {
    /// Open a card, or close it.
    Open(u8),
    /// A press on a wallet's plan cards, as Back up's
    /// (`docs/BACKUP-FLOW.md` §15).
    Plan(crate::PlanPress),
    /// Leave the flow, keeping its state.
    Back,
    /// Open the first card to change the plan.
    ChangePlan,
    /// Another place.
    AddPlace,
    /// Take a place out of the plan.
    RemovePlace(usize),
    /// The place keeps the stick of this vault (a new one when the index
    /// is the number of vaults).
    PlaceVault(usize, usize),
    /// The vault is an existing one, or a new one.
    VaultExisting(usize),
    /// Go on from the first card.
    Continue,
    /// Lock, then bring in the sticks.
    LockBring,
    /// Use the stick that is in the machine already, by its place among
    /// the sticks.
    UseBoot(usize),
    /// The vault is the file of this place in the stick's vault files.
    PickFile(usize, usize),
    /// The vault is a new one beside what the stick holds.
    PickBeside(usize),
    /// The copy of an existing vault the stick holds is the one to use
    /// (or the first copy read stays).
    UseCopy(bool),
    /// A tap: this is the stick for its place.
    Tap,
    /// A hold: this is the stick for its place, or a second stick.
    Hold,
    /// Leave this stick; ask for another.
    UseAnother,
    /// "I do not have its passphrase", for a vault.
    NoPass(usize),
    /// A new vault beside the one whose passphrase is not known.
    BesideIt(usize),
    /// Another stick for the vault.
    AnotherStick(usize),
    /// Unlock a vault of the plan.
    UnlockVault(usize),
    /// Make a vault of the plan.
    CreateVault(usize),
    /// Make the key.
    Make,
    /// The key is made: on to the lock.
    MadeContinue,
    /// Save the key into the vault added after it was made.
    CopyKey,
    /// Type the codex32 string Copy the key stopped on, from its copy.
    TypeString,
    /// Lock, then write.
    LockWrite,
    /// Unlock a vault of the plan once the sticks are written.
    UnlockDone(usize),
    /// The copy at this place was destroyed: it leaves the map.
    Destroyed(usize),
    /// A waiting action: open the flow of the key selected.
    BackUpFirst(Kind),
    /// Open the flow that made the key shown.
    Resume,
    /// Open flow `n`, from Home's lead.
    Show(usize),
    /// A place keeps several vaults on one stick, or on one each.
    Together(usize),
    /// Place `p` also keeps (or stops keeping) the stick of vault `v`.
    AlsoVault(usize, usize),
    /// The stick of this place carries the key's public files.
    PublicAt(usize),
    /// A wallet's answer to a question of the plan, as Back up's plan
    /// ticks it: the list and the row.
    Answer(u8, u8),
    /// The checklist's **Lock and write**: the wallet's flow takes the
    /// vaults and the files, and locks.
    WalletLock,
    /// Steps 1 to 3 are done: on to the wallet's keys.
    WalletMake,
    /// A press of a GPG key's restore from its paperkey.
    Restore(RestorePress),
}

/// Whether an action acts when held.
pub fn is_hold(a: FlowAction) -> bool {
    matches!(a, FlowAction::Hold)
}

/// A press of a GPG key's restore (`docs/GPG-PAPERKEY.md` §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestorePress {
    /// **Restore from a paperkey**: the sheet of the public keys in Files.
    Sheet,
    /// The flow opened on the public key that is this Inbox file.
    From(usize),
    /// The head card opened or closed.
    Head,
    /// The head card's **Continue**: on to where the key will be kept.
    HeadContinue,
    /// The line's field pressed: typing goes to it.
    Field,
    /// **Next line**: the line typed is handed in.
    Next,
    /// **Type line {n} again**: line `n` and those after it cleared.
    Again(u32),
    /// **Restore the key**.
    Restore,
}

/// What a kind's own build gives back (§4.4).
pub struct Built {
    /// The record the key is kept as, held: each vault takes a copy.
    pub record: faraday_keys::vault::MadeKey,
    /// The public value it is known by.
    pub subject: Vec<u8>,
    /// The name it is shown under.
    pub name: String,
    /// Files For the stick: the certificate or the public key.
    pub outbox: Vec<(String, Vec<u8>)>,
}
