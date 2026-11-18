//! What each key of a wallet of several keys is known by: a loaded key
//! gives it, the person ticked it as compared with its own device, or
//! nothing vouches for it. Worked out and stored when the wallet is added
//! and when a key is loaded or removed, never while drawing; the person's
//! ticks are kept apart from what the keys give so that both survive.

use super::Session;
use super::confirm::{Gives, KeyRow, Mark, key_row, key_rows};
use crate::strings::EN;
use crate::ui::Ui;
use crate::ui::pal::Role;
use faraday_bip::keys::Fingerprint;
use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::Compute;
use faraday_draw::tokens::{BODY_M, GAP, GAP_SMALL, ICON_GLYPH_BUTTON as SIDE};
use faraday_draw::widgets::Icon;
use faraday_text::Text;

/// What vouches for one key of a wallet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyCheck {
    /// A loaded key gives this extended key at its path.
    Yours,
    /// The person ticked it.
    Checked,
    /// Nothing vouches for it.
    Unchecked,
    /// Under a loaded key's fingerprint and not that key's.
    Wrong,
}

/// Where the ticks of a key block live: the press that toggles one says
/// which screen's rows to toggle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyBlockOf {
    /// The rows of the wallet waiting on the confirmation sheet.
    Asked,
    /// The rows of the register sheet.
    Register,
    /// An added wallet, by its place.
    Wallet(u16),
    /// The rows of Check a multisig setup's Result.
    Check,
}

/// A wallet's key states and the rows that show them.
#[derive(Clone, Debug, Default)]
pub(crate) struct KeyChecks {
    /// The rows with what the loaded keys give.
    given: Vec<KeyRow>,
    /// The person's ticks, one per key.
    ticks: Vec<bool>,
    /// The rows as they show, the ticks applied.
    shown: Vec<KeyRow>,
    /// The state of each key.
    states: Vec<KeyCheck>,
}

impl KeyChecks {
    /// Costs a key derivation per key under a loaded fingerprint that
    /// `memo` does not hold: the states of `policy`'s keys, none for a
    /// wallet of one key.
    pub(crate) fn of(
        session: &Session,
        c: &Compute,
        policy: &WalletPolicy,
        memo: &Gives,
    ) -> KeyChecks {
        let mut k = KeyChecks::default();
        if policy.keys().len() > 1 {
            k.ticks = vec![false; policy.keys().len()];
            k.given = key_rows(session, c, policy, memo);
            k.apply();
        }
        k
    }

    /// Costs a key derivation per key under a fingerprint in `new` that
    /// `memo` does not hold: what the loaded keys give worked out again
    /// after keys were loaded (`new`) or removed (not in `now`), the
    /// person's ticks kept.
    pub(crate) fn update(
        &mut self,
        session: &Session,
        c: &Compute,
        policy: &WalletPolicy,
        (now, new): (&[Fingerprint], &[Fingerprint]),
        memo: &Gives,
    ) {
        if self.ticks.len() != policy.keys().len() {
            return;
        }
        for (i, k) in policy.keys().iter().enumerate() {
            let Some(fp) = k.fingerprint() else { continue };
            if new.contains(&fp) {
                self.given[i] = key_row(session, c, memo, i, k);
            } else if !now.contains(&fp) {
                let row = &mut self.given[i];
                if matches!(row.mark, Mark::Yours(_) | Mark::Wrong(_)) {
                    row.mark = Mark::Unchecked;
                }
            }
        }
        self.apply();
    }

    fn apply(&mut self) {
        self.shown = self.given.clone();
        for (row, tick) in self.shown.iter_mut().zip(&self.ticks) {
            if *tick && row.mark == Mark::Unchecked {
                row.mark = Mark::Checked;
            }
        }
        self.states = self.shown.iter().map(|r| r.mark.state()).collect();
    }

    /// Sets each row's `waiting` as `waits` says.
    fn set_waiting(&mut self, waits: impl Fn(&KeyRow) -> bool) {
        for r in &mut self.given {
            r.waiting = waits(r);
        }
        self.apply();
    }

    pub(crate) fn states(&self) -> &[KeyCheck] {
        &self.states
    }

    pub(crate) fn rows(&self) -> &[KeyRow] {
        &self.shown
    }

    pub(crate) fn ticks(&self) -> Vec<bool> {
        self.ticks.clone()
    }

    /// `ticks` taken as the person's, unless it is not one per key.
    pub(crate) fn set_ticks(&mut self, ticks: &[bool]) {
        if ticks.len() == self.ticks.len() {
            self.ticks.copy_from_slice(ticks);
            self.apply();
        }
    }

    /// Merges the ticks a sheet took (`ticks`, one per key of `from`, the
    /// policy the sheet asked about) into the person's ticks of `policy`,
    /// the policy of the wallet that was added or was already loaded. A
    /// key is matched by itself (fingerprint, path and extended key all
    /// equal), never by its place, since the same wallet can list its keys
    /// in another order; a tick the wallet has stays, and a key of `from`
    /// that no key of `policy` equals ticks nothing.
    pub(crate) fn merge_ticks(
        &mut self,
        policy: &WalletPolicy,
        from: &WalletPolicy,
        ticks: &[bool],
    ) {
        if self.ticks.len() != policy.keys().len() || from.keys().len() != ticks.len() {
            return;
        }
        for (key, _) in from.keys().iter().zip(ticks).filter(|(_, t)| **t) {
            for (mine, tick) in policy.keys().iter().zip(self.ticks.iter_mut()) {
                if mine == key {
                    *tick = true;
                }
            }
        }
        self.apply();
    }

    /// Toggles the tick of key `i` where the person's tick decides it.
    pub(crate) fn tick(&mut self, i: usize) {
        if matches!(
            self.states.get(i),
            Some(KeyCheck::Unchecked | KeyCheck::Checked)
        ) {
            self.ticks[i] = !self.ticks[i];
            self.apply();
        }
    }

    pub(crate) fn unchecked(&self) -> usize {
        self.states
            .iter()
            .filter(|s| **s == KeyCheck::Unchecked)
            .count()
    }
}

impl Session {
    /// The keys here whose backup is not done are `held` (what
    /// `Faraday::held_keys_now` says): the rows of each added wallet and
    /// of each wallet waiting to be confirmed say so for those keys. A
    /// lookup; the rows made after this read it as they are made.
    pub(crate) fn set_waiting(&mut self, held: Vec<Fingerprint>) {
        let waits = |r: &KeyRow, now: &[Fingerprint]| {
            matches!(r.mark, Mark::Yours(_))
                && now
                    .iter()
                    .any(|f| r.fingerprint == crate::wallet::fp_text(*f))
        };
        for w in &mut self.wallets {
            w.key_checks.set_waiting(|r| waits(r, &held));
        }
        for a in &mut self.wallet_asked {
            for r in &mut a.rows.keys {
                r.waiting = waits(r, &held);
            }
        }
        self.waiting = held;
    }

    /// Whether any wallet's key rows are kept, or a wallet waits to be
    /// confirmed, or a key is marked as not backed up: the rows may need
    /// the words of `set_waiting` worked out again.
    pub(crate) fn shows_key_states(&self) -> bool {
        !self.waiting.is_empty()
            || !self.wallet_asked.is_empty()
            || self.wallets.iter().any(|w| !w.key_checks.rows().is_empty())
    }

    /// Toggles the person's tick of key `key` of wallet `wallet` where the
    /// key is unchecked or checked by them; nothing for a key a loaded key
    /// gives or one that is wrong. A lookup.
    pub fn tick_key(&mut self, wallet: usize, key: usize) {
        if let Some(w) = self.wallets.get_mut(wallet) {
            w.key_checks.tick(key);
        }
    }

    /// Costs what [`KeyChecks::update`] costs: the states of wallet `i`
    /// worked out again after keys were loaded or removed.
    pub(crate) fn refresh_key_checks(
        &mut self,
        c: &Compute,
        i: usize,
        fingerprints: (&[Fingerprint], &[Fingerprint]),
        memo: &Gives,
    ) {
        let mut checks = std::mem::take(&mut self.wallets[i].key_checks);
        checks.update(self, c, self.wallets[i].policy(), fingerprints, memo);
        self.wallets[i].key_checks = checks;
    }
}

/// The unchecked and wrong counts of `keys` as the line under them says.
fn line_of(keys: &[KeyRow]) -> Option<(Text, Role)> {
    if keys.len() < 2 {
        return None;
    }
    let unchecked = keys.iter().filter(|k| k.mark == Mark::Unchecked).count();
    let wrong = keys.iter().any(|k| matches!(k.mark, Mark::Wrong(_)));
    match unchecked {
        0 if wrong => None,
        0 => Some((Text::of(EN.keycheck_line_all), Role::Ok)),
        1 => Some((Text::of(EN.keycheck_line_one), Role::OnSurfaceVariant)),
        n => Some((
            Text::fill(EN.keycheck_line_many, &[&Text::num(n as u64)]),
            Role::OnSurfaceVariant,
        )),
    }
}

/// One key's state, as a mark and words. Returns the height.
fn state_row(ui: &mut Ui, (mark, waiting): (&Mark, bool), (x, y, w): (f32, f32, f32)) -> f32 {
    let (text, tone): (&str, Role) = match mark {
        Mark::Yours(true) if waiting => (EN.mscheck_yours_waiting_single, Role::Ok),
        Mark::Yours(false) if waiting => (EN.mscheck_yours_waiting, Role::Ok),
        Mark::Yours(true) => (EN.mscheck_yours_single, Role::Ok),
        Mark::Yours(false) => (EN.mscheck_yours, Role::Ok),
        Mark::Checked => (EN.keycheck_checked, Role::Ok),
        Mark::Unchecked => (EN.keycheck_unchecked, Role::OnSurfaceVariant),
        Mark::Wrong(line) => (line, Role::Error),
    };
    match mark {
        Mark::Unchecked => ui.checkbox(x, y, false, true),
        Mark::Wrong(_) => ui.icon(x, y, SIDE, Icon::Error, SIDE, tone.paint()),
        _ => ui.icon(x, y, SIDE, Icon::Done, SIDE, tone.paint()),
    }
    let inset = SIDE + GAP;
    ui.wrap_as((x + inset, y, w - inset), BODY_M, tone, text)
        .max(SIDE)
}

/// Draws each key of `keys` at `(x, y)`, `w` wide: its number,
/// fingerprint, path, the whole key and its state, a press on an entry
/// that the person's tick decides toggling the tick in `of`; a key whose
/// place is in `seeded` says it is the seed given; then the line that
/// counts the keys not checked. `states`: the keys' states show, for a
/// wallet of several keys. Returns the height.
pub(crate) fn draw_keys(
    keys: &[KeyRow],
    seeded: &[usize],
    of: KeyBlockOf,
    states: bool,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    use faraday_ui::sheet::kv;
    let mut cy = 0.0;
    for (i, k) in keys.iter().enumerate() {
        let top = cy;
        let head = faraday_text::text!("{} · {}", &*k.fingerprint, &*k.path);
        cy += kv(ui, x, y + cy, w, &k.number, &head, Role::OnSurface);
        cy += ui.wrap_as((x, y + cy, w), BODY_M.mono(), Role::OnSurface, &k.xpub);
        let shows = states || matches!(k.mark, Mark::Yours(_) | Mark::Wrong(_));
        if shows {
            cy += state_row(ui, (&k.mark, k.waiting), (x, y + cy, w));
        }
        if states
            && let Mark::Unchecked | Mark::Checked = k.mark
            && let Ok(at) = u8::try_from(i)
        {
            ui.press_box(x, y + top, w, cy - top, crate::Action::KeyTick(of, at));
        }
        if seeded.contains(&i) {
            cy += ui.wrap_as((x, y + cy, w), BODY_M, Role::Ok, EN.mscheck_this_seed);
        }
        cy += GAP;
    }
    if states && let Some((line, tone)) = line_of(keys) {
        cy += ui.wrap_as((x, y + cy, w), BODY_M, tone, &line) + GAP_SMALL;
    }
    cy
}

/// Draws the keys of an added wallet as the confirmation sheet draws
/// them, from the rows kept with it: nothing for a wallet of one key.
/// Returns the height.
#[allow(
    dead_code,
    reason = "the screens that draw an added wallet's keys call it"
)]
pub(crate) fn draw_key_block(
    wallet: &super::Wallet,
    of: KeyBlockOf,
    ui: &mut Ui,
    at: (f32, f32, f32),
) -> f32 {
    draw_keys(wallet.key_rows(), &[], of, true, ui, at)
}
