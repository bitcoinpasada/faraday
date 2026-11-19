# What holds a secret: one rule for clean, the idle lock and Lock

**Kind:** Reference
**Status:** Built: the `AtLock` trait and `Public` marker (`faraday-core/src/at_lock.rs`), the exhaustive listing of `Faraday` and of `ToolsArea`, the clean state, the idle lock's test and Lock's clears derived from it (§4 to §6), and the leak tour that walks every area by name (§7).

## 1. The requirement

A new area of Faraday's state, or a new field of one, does not compile
until whoever adds it has said whether it holds a secret and what Lock
does with it (owner, 2026-11-19: a future tool must not pass without
being cleared at Lock). Three answers are derived from that one
statement, and from nothing else:

- **clean** (`docs/SECURITY-MODEL.md`): this process has never held or
  typed a secret, so a stick may be handed to the disk process
  (`Faraday::clean`, `clean_now`, `note_secrets`, `faraday-core/src/idle.rs`);
- **lockable**: a lock would wipe something, so the idle warning and the
  idle lock come, and the idle power-off waits (`Faraday::lockable`,
  `idle.rs`, read in `state.rs`'s idle tick);
- **the lock's clears**: what `lock_finish` (`faraday-core/src/vault/seal.rs`)
  wipes before the process ends.

Each is one line over `Faraday::holds()` (§4).

**Why the clears matter though the process ends.** Lock is two steps
(`docs/SECURITY-MODEL.md`, "What locking wipes"): the application drops
what it holds, then the process exits and the kernel zeroes its pages.
The first step is what the leak tests see: their scan of every live
block after Lock (`docs/SECRET-LEAKS.md` §1 point 2) runs in the test
harness, where nothing exits, and so does every test that locks and goes
on. `save_boxes_for_lock` writes, between the clears and the exit, what
the fresh process is given; a lock can also be stopped after it is asked
(`Faraday::not_kept`, `idle.rs`, which clears `restart` when the state
could not be kept), and then the same process goes on. The Demo shell
serves the saved state to a fresh app in the same process
(`shells/demo/src/lib.rs`, `Command::Exit` with `restart_requested`).

## 2. The three lists before the rule

Clean, lockable and Lock's clears were once three lists written by hand,
and they disagreed field by field. The rule replaced them; §4 says what
each area counts now. Git keeps the table.

## 3. The way chosen

One trait per area, called from an exhaustive destructuring of `Faraday`
with no rest pattern, so a new field is a compile error until it is
written there either as an area that answers the trait or as public.
Every area that holds areas destructures its own struct the same way
(`ToolsArea` holds the Krux code, Bitcoin Core wallet and seed check
tools). Resetting an area to its `Default`, keeping its public fields by
name, is the usual body of its `lock`. The `demo`-only `transfer` is
written in the pattern under its own `#[cfg(feature = "demo")]`.

## 4. The trait

One file, `faraday-core/src/at_lock.rs` (a name of its function, as
`docs/STRUCTURE-PASS.md` §7 asks).

```rust
/// What an area holds, for the clean state, the idle lock and Lock.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Holds {
    /// A key or an open vault the session keeps: what the screens read
    /// as `Faraday::holds_secret` (Home's fresh state, a stick's offers).
    pub kept: bool,
    /// A secret on its way: typed, rolled, scanned, shown or opened in a
    /// tool. Makes the process not clean; read by the fade as
    /// `secret_typed`.
    pub typed: bool,
    /// Something a lock wipes that is not secret, or not one the process
    /// is tainted by: a wallet of the session, a spend under way, a file
    /// the Inbox drops at Lock.
    pub session: bool,
}

impl Holds {
    pub const NOTHING: Self = Self { kept: false, typed: false, session: false };
    pub fn any(self) -> bool { self.kept || self.typed || self.session }
    pub fn or(self, o: Self) -> Self {
        Self { kept: self.kept || o.kept, typed: self.typed || o.typed, session: self.session || o.session }
    }
}

/// An area of Faraday's state that can hold a secret or the session's
/// own state. No method has a default body: writing the impl is the
/// decision.
pub(crate) trait AtLock {
    /// What it holds now.
    fn holds(&self) -> Holds;
    /// Lock: everything it holds for the session goes; a public setting
    /// it keeps across a lock stays. After it, `holds` answers nothing.
    fn lock(&mut self);
}

/// An area or value that holds nothing a lock wipes. Implemented by name
/// for each type, beside the type, so calling a field public is a claim
/// made where its type is written, and a reviewer reads it there.
pub(crate) trait Public {}
impl Public for bool {}
impl Public for f32 {}
impl Public for u64 {}
impl Public for faraday_text::Text {}
impl<T: Public> Public for Option<T> {}
impl<T: Public> Public for Vec<T> {}
impl<T: Public + Copy> Public for std::cell::Cell<T> {}
// … the other types the struct holds, each by name beside its type.

pub(crate) fn public<T: Public>(_: &T) {}
```

The listing of `Faraday`, one for reading (`Faraday::holds`) and one
for the lock (`Faraday::lock_areas`), both destructuring the struct with
no rest pattern, is written by one macro, `faraday_areas!`, from one
list, so they cannot classify a field differently. The same list writes
`pub enum Area` and `Area::ALL` (under the `testkit` feature, §7). An
area is written `field => Variant`, since `macro_rules!` cannot change a
name's case:

```rust
faraday_areas! {
    public: [f, w, h, compact, pointer, out, clock_drawn, restart, restart_computer,
             last_display, settings, set_time, idle, tainted, inputs, now_ms, screen,
             sheet, sheet_offset, panel, stick, pin_h, list_offset, scroll, guide,
             typing, trail, disclosed, fade, selftest, online, seed_line, held,
             shell_sticks, clipboard, medium, secure_boot, maker, maker_family,
             maker_model, devices, next_mode, samples, sample_area, in_progress,
             outbox, memory, backups_area, silent, later],
    areas: [canvas => Canvas, drawing => Drawing, kept => Kept, vaults => Vaults,
            fresh => Fresh, session => Session, decode => Decode, inbox => Inbox,
            entry => Entry, wallets => Wallets, visit => Visit, import => Import,
            glance => Glance, qr => Qr, spend_backup => SpendBackup, vanity => Vanity,
            backup => Backup, message => Message, create => Create, restore => Restore,
            keygen => KeyGen, wordlist => WordList, bip85 => Bip85, explore => Explore,
            lightning => Lightning, tools => Tools, secrets => Secrets, camera => Camera,
            seed_read => SeedRead, worker => Worker, flows => Flows],
    demo: [transfer => Transfer],
}
```

Where §2 left a row "to be decided by the area", the impl decides it,
failing closed (unsure means it counts and Lock clears it), except where
counting would keep every session unclean:

| Area | What it counts, and what its lock does | Why |
|---|---|---|
| `fresh` (own randomness, the session's seed) | session (lockable) only while the person has added randomness; Lock wipes it and the seed | the seed and pool fill in ordinary use; randomness added is mixed with the system's and is not a secret on its own |
| `vanity` (the address search) | typed while open; Lock clears it | a find names a passphrase |
| `drawing`, `kept`, `canvas` (what frames keep) | nothing; Lock empties `drawing`, drops `kept`'s charts and makes `canvas` forget its glyphs | always filled; `kept` keeps the public descriptions the save reads after the lock |
| `memory` (nonces and signed amounts) | public | the nonce history must reach a vault: wiping it would let a nonce be reused |
| `restore` (Restore a wallet) | nothing; Lock clears it | §2's "seed shares" are the names of Inbox share files, which are public and the Inbox's to answer for; the seeds typed are the session's keys, and what the restore keeps of them is fingerprints and cosigners' public keys. Counting it would hold back the sticks a restore's vaults are written to |
| `visit` (a stick visit) | nothing; Lock drops the name question | its files are the Inbox's and Outbox's |
| `outbox`, `later`, `backups_area`, `silent` | public | kept for the fresh process by design, or holding nothing secret |
| `worker` (background jobs) | typed while an opened Krux code waits for a screen to take it; Lock drops it | |
| `transfer` (Faraday Demo only) | nothing; Lock empties its text | it is the online side, holding nothing of the device's |
| `glance` (the PassCheck sheet) | typed while its passphrase is typed; Lock wipes it | it holds a typed passphrase, so it is an area, not public |
| `import` (what the boot stick brought) | session; Lock drops it | |
| `explore`, `qr`, `decode`, `message` | nothing; Lock clears them | they read a key or vault that already counts, or public contents |
| `wordlist` | typed while a word is searched for or marked | it may be a word of a seed |

The three answers, each one line:

```rust
pub fn holds_secret(&self) -> bool { self.holds().kept }
pub(crate) fn secret_typed(&self) -> bool { self.holds().typed }
pub(crate) fn lockable(&self) -> bool { self.holds().any() }
// clean_now: !tainted && !camera_on() && !(kept || typed). The camera
// area's `typed` counts its parts half read, a code carried to Unlock and
// a read card waiting under another sheet; camera_on stays beside it for
// a frame on the Scan sheet, since it reads the sheet and Faraday Demo's
// Transfer, which are not the camera area's.
```

Where one area's answer needs another's (Make a new wallet's Keys card
counts only once something is entered, which reads `entry`), the area
that owns the fact answers it: `entry` answers for what is typed in it,
`keygen` for its own rolls and card. No answer reads a sibling.

**What changes for a person.** Clean and lockable, derived from every
area, count what §2 marks **no**: Convert a seed, a new vault's
passphrases, a BIP-85 child, the calculator's typed
line, Spend from a backup. A stick is held back while one of them holds
a secret, and an idle session with only typed words or a tool's secret
warns and locks at the lock time rather than powering off at the
power-off time (and in Faraday Demo it locks rather than staying). This
is the intended change: it is what the requirement asks (owner,
2026-11-19).

## 5. Three areas in full

**`ToolsArea`, an area that holds areas** (`faraday-core/src/tools/mod.rs`).
Its own fields are destructured with no rest pattern, so a new tool is a
compile error here:

```rust
impl AtLock for ToolsArea {
    fn holds(&self) -> Holds {
        let ToolsArea { tools, catalog_find, home_find, home_focus, home_bar_y,
                        home_follow, convert, kef_tool, ots, seed_check, mscheck,
                        core_wallet, invoice, address_check } = self;
        public(catalog_find); public(home_find); public(home_focus);
        public(home_bar_y); public(home_follow); public(ots); public(invoice);
        public(address_check);
        let calc = Holds { typed: tools.as_ref().is_some_and(|t| !t.typed.is_empty()),
                           ..Holds::NOTHING };
        calc.or(convert.holds()).or(kef_tool.holds())
            .or(seed_check.holds()).or(mscheck.holds()).or(core_wallet.holds())
    }
    fn lock(&mut self) {
        let ToolsArea { tools, catalog_find: _, home_find: _, home_focus: _,
                        home_bar_y: _, home_follow: _, convert, kef_tool,
                        ots: _, seed_check, mscheck, core_wallet, invoice: _,
                        address_check: _ } = self;
        *tools = None;
        convert.lock(); kef_tool.lock(); seed_check.lock();
        mscheck.lock(); core_wallet.lock();
    }
}
```

**`LightningArea`, a plain secret area** (`faraday-core/src/lightning/mod.rs`):

```rust
impl AtLock for LightningArea {
    fn holds(&self) -> Holds {
        // Any state: the words, the passphrase or the node key.
        Holds { typed: self.state.is_some(), ..Holds::NOTHING }
    }
    fn lock(&mut self) { *self = Self::default(); }
}
```

**`InboxArea`, an area partly kept across Lock** (`faraday-core/src/inbox.rs`).
What is public or encrypted stays for the fresh process; the rest goes:

```rust
impl AtLock for InboxArea {
    fn holds(&self) -> Holds {
        Holds {
            kept: false,
            // A seed being made a wallet: its passphrase is typed.
            typed: self.potential().is_some_and(|p| !p.passphrase.is_empty()),
            // A seed file read from a stick: Lock drops it.
            session: self.potential().is_some()
                || self.files().iter().any(|i| i.dropped_at_lock()),
        }
    }
    fn lock(&mut self) {
        // A picture goes as what its codes hold, and as Other when none read.
        self.retain(|i| !i.dropped_at_lock());
        self.clear_potential();
    }
}
```

A file read from a stick that may hold a secret (a seed file, or text
Faraday cannot tell) counts toward the idle lock, so an idle session
holding one locks rather than powering off, and is dropped at Lock, but it does not make the process unclean: reading one does not
taint the process, and counting it as typed would hold back the stick it
is being copied to, which copying a file between sticks needs open
(`docs/FLOWS.md`, the visit's own exemption). A potential's typed
passphrase counts as typed.

## 6. `lock_finish` after the rule

`lock_finish` (`faraday-core/src/vault/seal.rs`) runs after the lock's
last seal, so its draws from own randomness come before the session's
seed is wiped. In order: `self.lock_areas()`; in a debug build, a check
that no area still holds what it reported (so every lock in the tests
checks each area's `lock` against its `holds`); the wallet list marked
changed; `more::drop_gone`; `save_boxes_for_lock`; then `restart` and
`Command::Exit` last. Nothing else in it is one area's: an area's clear
is the body of that area's `lock`, and a sub-area's (a tool's) is
reached through its holder's. Each `holds_secret` an area had (`seeds`,
`seed_check`, `kef_tool`, `core_wallet`, `wallet_backup`'s Krux form) is
that area's `holds`, widened to everything its `lock` wipes (the Core
wallet tool's file).

## 7. What holds the rule

**The compiler** holds that every field is decided: the pattern in
`faraday_areas!` and in each holder's impl, and the missing trait impl.
No test states it: a test does not state a design rule.

**The tests** hold what a person notices, and stay as they are:

- the leak flows that end in Lock, with the scan of every live block
  (`faraday-core/tests/leaks/`, `docs/SECRET-LEAKS.md` §1 point 2, §4);
- the tests that a lock wipes an area's state: own randomness's, the GPG restore's, the sidebar's, the
  visit's questions, a vault's kept and wiped lines (`tests/all/vault_seal.rs`)
  and Spend from a backup after Lock (`tests/all/spend_backup.rs`);
- the idle tests for lockable (`tests/all/idle.rs`);
- the clean tests: a text typed in the Krux code tool leaves the session
  not clean (`tests/all/kef_tool.rs`), a stick read and written only by a
  clean process (`tests/all/backup_flow.rs`), the visit's write on a clean
  visit (`tests/all/visit.rs`).

**The leak tour** (`faraday-core/tests/leaks/areas.rs`). The macro also
writes `pub enum Area { … }`, one name per field in `areas`, and
`Area::ALL` (under the `testkit` feature). `flows` in `areas.rs` matches on
`Area` with no wildcard arm, so a new area does not compile there until it
has a flow or an arm that says why it plants nothing. For each flow the
tour opens the area's screen through the testkit and plants the §3 secret of
`docs/SECRET-LEAKS.md` as a person would (typed, scanned or loaded), then
checks that the session is not clean and that the idle warning comes (not
in Make a new wallet, which holds the idle lock until its keys are kept),
presses Lock, the save failed (a fresh app is built over what the old one
carries) and again an ordinary lock, and scans every live block in the old
app before it is dropped and in the fresh one. An area that holds no secret
of its own has an arm that says so and plants nothing. The flows are in
§4's list of `docs/SECRET-LEAKS.md`, and each `Area` arm names its flows.
Every area that holds a secret has a flow but Worker, whose opened Krux
code is taken or dropped in the step its answer arrives, so no test can
catch one waiting.
