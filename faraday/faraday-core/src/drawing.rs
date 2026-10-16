//! What a frame works out once.
//!
//! A frame asks the backup's checklist the same questions many times: the
//! items, whether each is done, the plan's shape, the seeds, the held keys
//! and the public file names, a stick's among them. While [`crate::Faraday::render`] runs the
//! answers are kept here; nothing is kept outside it. Only drawing reads
//! them: a decision (a press gated, a hold released) computes from the
//! state as it is (`docs/SPEED.md` §2 rule 3).

use std::cell::{Cell, OnceCell, RefCell};

use osk_bip::keys::Fingerprint;

use crate::plan;

/// A wallet's seeds: a name each and the session key that holds it.
pub(crate) type SeedList = Vec<(String, Option<usize>)>;

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
