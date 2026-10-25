//! The scrolled regions a frame draws: which offset each moves
//! ([`Slot`]) and where each was drawn and how far it goes
//! ([`Scrolled`]), for the app to move the one under the pointer.

use faraday_draw::Rect;

/// Which offset a scrolled region moves. A screen can scroll more than
/// one region at once, each by its own offset; the wheel, a trackpad and
/// a finger move the one under the pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Slot {
    /// The screen's region, or the open sheet's when it scrolls.
    #[default]
    Page,
    /// Stick visit's Outbox list (`VisitState::out_offset`).
    VisitOut,
    /// The wallet card's body on a desktop (`Faraday::card_offset`).
    Card,
    /// Vault contents' item on a desktop (`VaultArea::item_offset`, `faraday-core`).
    VaultItem,
    /// Files' For the stick list on a desktop, beside From the stick
    /// (`Faraday::files_out_offset`).
    FilesOut,
    /// The list pane of a list and detail page, beside its detail
    /// (`docs/LAYOUT.md` §5; `Panes::list_offset`, `faraday-core`).
    ListPane,
    /// The backup board's list in the right-hand column on a desktop
    /// (`Faraday::board_list_offset`).
    BoardList,
    /// A codex32 checksum worksheet, which pans both ways
    /// (`SheetPan`, `faraday-core`).
    Worksheet,
    /// A codex32 checksum worksheet drawn whole: it moves no offset of
    /// its own, and says where it stands for the page to follow its work
    /// (`Codex32Make::follow_page`, `faraday-core`).
    WorksheetWhole,
    /// The codex32 entry's "Demo only" card, a supporting pane that
    /// scrolls within itself (`Codex32Make::pane_offset`, `faraday-core`).
    DemoPane,
    /// Where the codex32 dice sheet's work is while the step-through
    /// rolls: the marked die's track, or the tree on the character's
    /// step; and, in the tour of the shares made, the marked cells of a
    /// translation worksheet. It moves no offset of its own, and says where it stands for
    /// the page to follow (`Faraday::dice_follow`, `faraday-core`).
    DiceWork,
}

/// A scrolled region as drawn: where it is and how far it goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scrolled {
    /// The offset it moves.
    pub slot: Slot,
    /// Where it is seen on the panel, in pixels.
    pub view: Rect,
    /// How far its content scrolls, in design units.
    pub max: f32,
    /// It draws its own scrollbar.
    pub own_bar: bool,
    /// How far its content pans sideways, in design units: 0 for a
    /// region that only scrolls up and down.
    pub max_x: f32,
}
