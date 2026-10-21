//! What a frame works out once.
//!
//! A frame asks the backup's checklist the same questions many times: the
//! items, whether each is done, the plan's shape, the seeds, the held keys
//! and the public file names, a stick's among them, what a lock keeps and
//! wipes, and the sticks to write; and for every screen, what the Inbox
//! gives, every wallet known with its backup, and each open vault's rows.
//! While [`crate::Faraday::render`] runs the answers are kept here;
//! nothing is kept outside it. Only drawing reads
//! them: a decision (a press gated, a hold released) computes from the
//! state as it is (`docs/SPEED.md` §2 rule 3).
//!
//! What these answers are put together from is kept on the state, read
//! once from what it describes (a plan's map, a vault's wallet records,
//! an Inbox file's wallet, an address): a fill here puts parts together
//! and parses nothing again. Each fill runs in [`crate::work::filling`],
//! the only place, with what the state keeps ([`crate::work::keeping`]),
//! where costly work may run while a frame is drawn.

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
    /// What a lock keeps and wipes, by `power_off`
    /// ([`crate::Faraday::kept_wiped`]).
    pub kept_wiped: RefCell<Vec<(bool, crate::vaults::KeptWiped)>>,
    /// The sticks the wallet being backed up is written to
    /// ([`crate::Faraday::wallet_stick_rows`]).
    pub stick_rows: OnceCell<Vec<StickRow>>,
    /// The Load sheet an unlock shows ([`crate::Faraday::vault_load_view`],
    /// F62).
    pub load_sheet: OnceCell<std::rc::Rc<crate::vault_load::LoadView>>,
    /// The seeds open vaults keep without a passphrase
    /// ([`crate::Faraday::vault_passphrase_seeds`], F62).
    pub passphrase_seeds: OnceCell<std::rc::Rc<Vec<crate::vault_load::PassSeed>>>,
    /// Every wallet known and where its backup is
    /// ([`crate::Faraday::backups`]).
    pub backups: OnceCell<Vec<crate::backups::Entry>>,
    /// The keys and wallets each open vault holds, by its place among
    /// the open vaults ([`crate::Faraday::vault_rows`]).
    pub vault_rows: RefCell<Vec<(usize, Vec<crate::vaults::VaultRow>)>>,
    /// Everything the Inbox gives ([`crate::Faraday::inbox_found`]).
    pub inbox_found: OnceCell<crate::inbox::Found>,
    /// The session's keys and wallets no open vault holds
    /// ([`crate::Faraday::unsaved`]).
    pub unsaved: OnceCell<(Vec<String>, Vec<String>)>,
    /// What the boot stick brought, counted
    /// ([`crate::Faraday::import_count`]).
    pub import_count: OnceCell<Option<crate::boot_import::ImportCount>>,
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
