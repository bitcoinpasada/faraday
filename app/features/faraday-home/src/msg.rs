use faraday_kernel::Message;
use faraday_present::Pressed;

#[derive(Message)]
pub enum Msg {
    Press(Press),
    /// A key typed on a menu page goes to the search field.
    Typed(char),
    /// Backspace in the search field.
    Erase,
}

impl From<Press> for Msg {
    fn from(press: Press) -> Msg {
        Msg::Press(press)
    }
}

/// What the feature tells its parent.
#[derive(Message)]
pub enum Out {
    /// Begin Make a new wallet (06 §4.3).
    CreateWallet,
    /// Begin Restore an existing wallet.
    RestoreWallet,
    /// Begin Spend from a backup.
    SpendBackup,
    /// Open the guide at the glossary word in this place of `faraday_guide::glossary`.
    GuideWord(u32),
    /// The words typed in the search and whether the typing goes to it: the bar shows them.
    Search {
        typed: faraday_text::Text,
        focused: bool,
    },
}

/// What a press on this feature's page sends.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {
    /// The search field is pressed: the typing goes to it.
    Focus,
    /// The search field's x.
    Clear,
    Make,
    Restore,
    SpendBackup,
    /// A guide word found by the search, by its place in the glossary.
    Word(u32),
}
