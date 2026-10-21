//! The drawing memo's fills, on [`View`]: what a frame asks many times,
//! worked out the first time it is asked in the frame and looked up after
//! (`docs/SPEED.md` §3.3, `docs/COMPUTE.md` §4). Each is the answer of the
//! [`Faraday`] method of the same name, worked out by its `_now` form, the
//! one decisions use; the memo is emptied when the frame ends. Each fill
//! runs in [`work::filling`], the guard's exception for the memo, and is
//! handed the view's token (`View::filled`).
//!
//! The old `Faraday` forms ask these while a frame is drawn, so a screen
//! still drawn from `&Faraday` shares the one memo, until C3 removes them.

use std::rc::Rc;

use osk_bip::keys::Fingerprint;

use super::View;
use crate::backups::Entry;
use crate::board::BoardView;
use crate::boot_import::ImportCount;
use crate::create_from::CreateVaults;
use crate::in_progress::Row;
use crate::inbox::Found;
use crate::vault_load::{LoadView, PassSeed};
use crate::vaults::{KeptWiped, VaultRow};
use crate::{plan, work};

impl View<'_> {
    /// Every wallet known and where its backup is
    /// ([`crate::Faraday::backups`]), once a frame.
    pub fn backups(&self) -> Vec<Entry> {
        self.drawing
            .backups
            .get_or_init(|| self.filled(|c| self.app.backups_now(c)))
            .clone()
    }

    /// The files a stick of the backup's own answers holds for `what`
    /// ([`crate::Faraday::stick_file_names`]), once a frame each; for
    /// other answers worked out on each ask.
    pub(crate) fn stick_file_names(
        &self,
        w: usize,
        shape: &plan::Shape,
        a: &plan::Answers,
        what: plan::What,
    ) -> Vec<String> {
        // Kept for the frame only for the backup's own answers.
        let own = self
            .backup
            .as_ref()
            .is_some_and(|b| b.wallet == w && b.answers == *a);
        if !own {
            return self.app.stick_file_names_now(w, shape, a, what);
        }
        if let Some((_, _, names)) = self
            .drawing
            .stick_names
            .borrow()
            .iter()
            .find(|(k, x, _)| *k == w && *x == what)
        {
            return names.clone();
        }
        let names = self.filled(|_c| self.app.stick_file_names_now(w, shape, a, what));
        self.drawing
            .stick_names
            .borrow_mut()
            .push((w, what, names.clone()));
        names
    }

    /// The sticks the wallet being backed up is written to
    /// ([`crate::Faraday::wallet_stick_rows`]), once a frame.
    pub fn wallet_stick_rows(&self) -> Vec<super::StickRow> {
        self.drawing
            .stick_rows
            .get_or_init(|| self.filled(|c| self.app.wallet_stick_rows_now(c)))
            .clone()
    }

    /// What Create's Keys card offers from the vaults
    /// ([`View::create_vaults`]), once a frame.
    pub(crate) fn create_vaults(&self) -> Rc<CreateVaults> {
        self.drawing
            .create_vaults
            .get_or_init(|| self.filled(|c| self.app.create_vaults_now(c)))
            .clone()
    }

    /// The backup board as the frame draws it
    /// ([`crate::Faraday::board_view`]), once a frame.
    pub(crate) fn board_view(&self) -> Option<Rc<BoardView>> {
        self.drawing
            .board
            .get_or_init(|| self.filled(|c| self.app.board_view_now(c)))
            .clone()
    }

    /// What the boot stick brought, counted
    /// ([`crate::Faraday::import_count`]), once a frame.
    pub fn import_count(&self) -> Option<ImportCount> {
        *self
            .drawing
            .import_count
            .get_or_init(|| self.filled(|c| self.app.import_count_now(c)))
    }

    /// The keys here that are held ([`crate::Faraday::held_keys`]), once a
    /// frame.
    pub fn held_keys(&self) -> Vec<Fingerprint> {
        self.drawing
            .held
            .get_or_init(|| self.filled(|c| self.app.held_keys_now(c)))
            .clone()
    }

    /// Everything the Inbox gives ([`crate::Faraday::inbox_found`]), once a
    /// frame: the Inbox's kept copy ([`crate::inbox::InboxFiles`]), worked
    /// out again only when the files or the session changed.
    pub fn inbox_found(&self) -> Found {
        self.drawing
            .inbox_found
            .get_or_init(|| {
                self.inbox.found(&self.session, |items| {
                    self.kept(|c| work::keeping(|| self.app.found_in(c, items)))
                })
            })
            .clone()
    }

    /// Whether key `fp` is held ([`crate::Faraday::key_held`]), from the
    /// memo's [`View::held_keys`].
    pub fn key_held(&self, fp: Fingerprint) -> bool {
        self.held_keys().contains(&fp)
    }

    /// The keys of loaded wallet `w` here that are held
    /// ([`crate::Faraday::wallet_held`]), from the memo's held keys.
    pub fn wallet_held(&self, w: usize) -> Vec<Fingerprint> {
        self.kept(|c| self.app.wallet_held(c, w))
    }

    /// Whether a key of `wl` made here still waits for its backup
    /// ([`crate::Faraday::receive_held`]), from the memo's held keys.
    pub fn receive_held(&self, wl: &crate::wallet::Wallet) -> bool {
        self.kept(|c| self.app.receive_held(c, wl))
    }

    /// An address of `wl` as every screen shows it, and whether it is held
    /// back ([`crate::Faraday::receive_line`]), from the memo's held keys
    /// and the wallet's kept address.
    pub fn receive_line(
        &self,
        wl: &crate::wallet::Wallet,
        change: bool,
        index: u32,
    ) -> (String, bool) {
        self.kept(|c| self.app.receive_line(c, wl, change, index))
    }

    /// The line said in place of Make the checklist
    /// ([`crate::Faraday::backup_kept_nowhere`]), from the memo.
    pub fn backup_kept_nowhere(&self) -> Option<String> {
        self.kept(|c| self.app.backup_kept_nowhere(c))
    }

    /// The wallet card's backup line ([`crate::Faraday::backup_line`]),
    /// from the memo's [`View::backups`].
    pub fn backup_line(&self, i: usize) -> String {
        self.kept(|c| self.app.backup_line(c, i))
    }

    /// The wallet card's first button ([`crate::Faraday::spend_button`]),
    /// from the memo's held keys.
    pub fn spend_button(&self, w: usize) -> (String, crate::Action) {
        self.kept(|c| self.app.spend_button(c, w))
    }

    /// Where the money moving from loaded wallet `w` goes
    /// ([`crate::Faraday::moving_to`]), from the memo's held keys and the
    /// wallet's kept address.
    pub(crate) fn moving_to(&self, w: usize) -> Option<(String, String)> {
        self.kept(|c| self.app.moving_to(c, w))
    }

    /// The first checklist item not done ([`crate::Faraday::backup_waiting`]),
    /// from the memo's [`View::backup_items`] and [`View::backup_item_done`].
    pub fn backup_waiting(&self) -> Option<u8> {
        self.backup_items()
            .into_iter()
            .find(|&i| crate::bstep::item(i).is_some_and(|it| !self.backup_item_done(it)))
    }

    /// Whether Make a new wallet holds the idle lock
    /// ([`crate::Faraday::lock_held`]), from the memo's answers on the
    /// backup's checklist.
    pub fn lock_held(&self) -> bool {
        self.kept(|c| self.app.lock_held(c))
    }

    /// The wallet here, in the Inbox or loaded, whose key the potential
    /// wallet's seed with its passphrase is
    /// ([`crate::Faraday::potential_match`]), from the memo's
    /// [`View::inbox_found`].
    pub fn potential_match(&self) -> Option<String> {
        let with = self.potential.as_ref()?.with?;
        self.inbox_found()
            .wallets
            .into_iter()
            .find(|w| w.keys.contains(&with))
            .map(|w| w.name)
    }

    /// The flows in progress ([`crate::Faraday::flows_in_progress`]), once
    /// a frame.
    pub fn flows_in_progress(&self) -> Rc<Vec<Row>> {
        self.drawing
            .flows
            .get_or_init(|| self.filled(|c| self.app.flows_in_progress_now(c)))
            .clone()
    }

    /// Wallet `w`'s seeds ([`crate::Faraday::backup_seed_list`]), once a
    /// frame each.
    pub fn backup_seed_list(&self, w: usize) -> Vec<(String, Option<usize>)> {
        if let Some((_, list)) = self
            .drawing
            .seed_lists
            .borrow()
            .iter()
            .find(|(k, _)| *k == w)
        {
            return list.clone();
        }
        let list = self.filled(|_c| self.app.backup_seed_list_now(w));
        self.drawing.seed_lists.borrow_mut().push((w, list.clone()));
        list
    }

    /// What wallet `w`'s plan is for ([`crate::Faraday::plan_shape`]),
    /// once a frame each.
    pub fn plan_shape(&self, w: usize) -> plan::Shape {
        if let Some((_, shape)) = self.drawing.shapes.borrow().iter().find(|(k, _)| *k == w) {
            return shape.clone();
        }
        let shape = self.filled(|_c| self.app.plan_shape_now(w));
        self.drawing.shapes.borrow_mut().push((w, shape.clone()));
        shape
    }

    /// The backup's checklist ([`crate::Faraday::backup_items`]), once a
    /// frame.
    pub fn backup_items(&self) -> Vec<u8> {
        self.drawing
            .items
            .get_or_init(|| self.filled(|_c| self.app.backup_items_now()))
            .clone()
    }

    /// The public files of loaded wallet `wi` under answers `a`
    /// ([`crate::Faraday::backup_public_names_for`]): once a frame for the
    /// backup's own answers, on each ask for others.
    pub(crate) fn backup_public_names_for(&self, wi: usize, a: &plan::Answers) -> Vec<String> {
        // Kept only for the backup's own answers: another caller's
        // answers are another question.
        let own = self
            .backup
            .as_ref()
            .is_some_and(|b| b.wallet == wi && b.answers == *a);
        if !own {
            return self.filled(|c| self.app.backup_public_names_for_now(c, wi, a));
        }
        if let Some((_, names)) = self
            .drawing
            .public_names
            .borrow()
            .iter()
            .find(|(k, _)| *k == wi)
        {
            return names.clone();
        }
        let names = self.filled(|c| self.app.backup_public_names_for_now(c, wi, a));
        self.drawing
            .public_names
            .borrow_mut()
            .push((wi, names.clone()));
        names
    }

    /// Whether a checklist item is done
    /// ([`crate::Faraday::backup_item_done`]), once a frame each.
    pub fn backup_item_done(&self, item: plan::Item) -> bool {
        if let Some((_, done)) = self.drawing.done.borrow().iter().find(|(i, _)| *i == item) {
            return *done;
        }
        let done = self.filled(|c| self.app.backup_item_done_now(c, item));
        self.drawing.done.borrow_mut().push((item, done));
        done
    }

    /// The seeds open vaults keep without a passphrase
    /// ([`crate::Faraday::vault_passphrase_seeds`]), once a frame.
    pub fn vault_passphrase_seeds(&self) -> Rc<Vec<PassSeed>> {
        self.drawing
            .passphrase_seeds
            .get_or_init(|| self.filled(|c| self.app.vault_passphrase_seeds_now(c)))
            .clone()
    }

    /// The Load sheet an unlock shows ([`crate::Faraday::vault_load_view`]),
    /// once a frame.
    pub fn vault_load_view(&self) -> Option<Rc<LoadView>> {
        self.drawing
            .load_sheet
            .get_or_init(|| self.filled(|c| self.app.vault_load_view_now(c)))
            .clone()
    }

    /// The session's keys and wallets no open vault holds
    /// ([`crate::Faraday::unsaved`]), once a frame.
    pub fn unsaved(&self) -> (Vec<String>, Vec<String>) {
        self.drawing
            .unsaved
            .get_or_init(|| self.filled(|c| self.app.unsaved_now(c)))
            .clone()
    }

    /// What a lock keeps and wipes ([`crate::Faraday::kept_wiped`]), once a
    /// frame for each `power_off`.
    pub fn kept_wiped(&self, power_off: bool) -> KeptWiped {
        if let Some((_, kept)) = self
            .drawing
            .kept_wiped
            .borrow()
            .iter()
            .find(|(p, _)| *p == power_off)
        {
            return kept.clone();
        }
        let out = self.filled(|c| self.app.kept_wiped_now(c, power_off));
        self.drawing
            .kept_wiped
            .borrow_mut()
            .push((power_off, out.clone()));
        out
    }

    /// The keys and wallets open vault `v` holds
    /// ([`crate::Faraday::vault_rows`]), once a frame each.
    pub fn vault_rows(&self, v: usize) -> Vec<VaultRow> {
        if let Some((_, kept)) = self
            .drawing
            .vault_rows
            .borrow()
            .iter()
            .find(|(o, _)| *o == v)
        {
            return kept.clone();
        }
        let rows = self.filled(|c| self.app.vault_rows_now(c, v));
        self.drawing.vault_rows.borrow_mut().push((v, rows.clone()));
        rows
    }
}
