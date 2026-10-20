//! What a frame works out once.
//!
//! A frame asks the backup's checklist the same questions many times: the
//! items, whether each is done, the plan's shape, the seeds, the held keys
//! and the public file names, a stick's among them, the wallets an open
//! vault holds, what a lock keeps and wipes, and the sticks to write.
//! While [`crate::Faraday::render`] runs the answers are kept here;
//! nothing is kept outside it. Only drawing reads
//! them: a decision (a press gated, a hold released) computes from the
//! state as it is (`docs/SPEED.md` §2 rule 3).

use std::cell::{Cell, OnceCell, RefCell};

use osk_bip::keys::Fingerprint;

use crate::plan;

/// A wallet's seeds: a name each and the session key that holds it.
pub(crate) type SeedList = Vec<(String, Option<usize>)>;

/// A stick the wallet being backed up is written to: its place, the files
/// it will hold, and the stick's label once written
/// ([`crate::Faraday::wallet_stick_rows`]).
pub type StickRow = (String, Vec<String>, Option<String>);

/// The answers kept for one frame.
#[derive(Default)]
pub(crate) struct Drawing {
    /// True only while `render` runs.
    pub on: Cell<bool>,
    /// What `backup_items` returned.
    pub items: OnceCell<Vec<u8>>,
    /// What `backup_item_done` returned for each item asked.
    pub done: RefCell<Vec<(plan::Item, bool)>>,
    /// `plan_shape` per wallet.
    pub shapes: RefCell<Vec<(usize, plan::Shape)>>,
    /// `backup_seed_list` per wallet.
    pub seed_lists: RefCell<Vec<(usize, SeedList)>>,
    /// `held_keys`.
    pub held: OnceCell<Vec<Fingerprint>>,
    /// `backup_public_names_for` per wallet, for the backup's own answers.
    pub public_names: RefCell<Vec<(usize, Vec<String>)>>,
    /// `stick_file_names` per wallet and thing, for the backup's own
    /// answers.
    pub stick_names: RefCell<Vec<(usize, plan::What, Vec<String>)>>,
    /// The backup board as the frame draws it
    /// ([`crate::Faraday::board_view`], `docs/BACKUP-FLOW.md` §19).
    pub board: OnceCell<Option<std::rc::Rc<crate::board::BoardView>>>,
    /// What Create's Keys card offers from the vaults
    /// ([`crate::Faraday::create_vaults`], `docs/NEW-WALLET.md` §16.1).
    pub create_vaults: OnceCell<std::rc::Rc<crate::create_from::CreateVaults>>,
    /// The flows in progress ([`crate::Faraday::flows_in_progress`],
    /// `docs/FLOWS.md`, Flows in progress).
    pub flows: OnceCell<std::rc::Rc<Vec<crate::in_progress::Row>>>,
    /// The wallets each open vault holds, by its place among the open
    /// vaults (`vault_screens::vault_wallets`): public descriptors.
    pub vault_wallets: RefCell<Vec<(usize, std::rc::Rc<Vec<String>>)>>,
    /// What a lock keeps and wipes, by `power_off`
    /// ([`crate::Faraday::kept_wiped`]).
    pub kept_wiped: RefCell<Vec<(bool, crate::vaults::KeptWiped)>>,
    /// The sticks the wallet being backed up is written to
    /// ([`crate::Faraday::wallet_stick_rows`]).
    pub stick_rows: OnceCell<Vec<StickRow>>,
}

impl Drawing {
    /// Empties every cell and turns the memo off. Cheap.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Turns the memo on, empty.
    pub fn start(&mut self) {
        self.clear();
        self.on.set(true);
    }

    /// Whether the memo is in use: only while a frame is drawn.
    pub fn is_on(&self) -> bool {
        self.on.get()
    }
}
