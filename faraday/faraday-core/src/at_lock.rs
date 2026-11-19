//! What holds a secret: the one rule the clean state, the idle lock and
//! Lock's clears come from (`docs/AT-LOCK.md`).
//!
//! Every field of [`Faraday`] is written below, in [`faraday_areas!`],
//! as an area that answers [`AtLock`] or as [`Public`], with no rest
//! pattern: a new field does not compile until whoever adds it has said
//! what Lock does with it. An area that holds areas (`ToolsArea`) lists
//! its own fields the same way in its impl.

use crate::Faraday;

/// What an area holds, for the clean state, the idle lock and Lock.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
    pub(crate) const NOTHING: Self = Self {
        kept: false,
        typed: false,
        session: false,
    };

    /// Something typed and secret, when `typed`.
    pub(crate) const fn typed(typed: bool) -> Self {
        Self {
            kept: false,
            typed,
            session: false,
        }
    }

    /// Something of the session's, when `session`.
    pub(crate) const fn session(session: bool) -> Self {
        Self {
            kept: false,
            typed: false,
            session,
        }
    }

    /// Whether a lock would wipe anything.
    pub(crate) const fn any(self) -> bool {
        self.kept || self.typed || self.session
    }

    pub(crate) const fn or(self, o: Self) -> Self {
        Self {
            kept: self.kept || o.kept,
            typed: self.typed || o.typed,
            session: self.session || o.session,
        }
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
/// made where its type is written, and a reviewer reads it there. A
/// plain value (a number, a flag, a text) is public only where a listing
/// names its field public.
pub(crate) trait Public {}
impl Public for bool {}
impl Public for f32 {}
impl Public for u64 {}
impl Public for faraday_text::Text {}
impl<T: Public> Public for Option<T> {}
impl<T: Public> Public for Vec<T> {}
impl<T: Public + Copy> Public for std::cell::Cell<T> {}
// The display the shell reported.
impl Public for faraday_shell_api::DisplayInfo {}
// The self-test's outcome: which checks passed.
impl Public for faraday_selftest::Outcome {}

/// Says a field is [`Public`].
pub(crate) fn public<T: Public>(_: &T) {}

// The glyphs kept for frames: always filled, so nothing to count; a lock
// forgets them, since a secret on show was drawn with them.
impl AtLock for faraday_draw::Canvas {
    fn holds(&self) -> Holds {
        Holds::NOTHING
    }
    fn lock(&mut self) {
        self.forget_glyphs();
    }
}

impl<T: AtLock> AtLock for Option<T> {
    fn holds(&self) -> Holds {
        self.as_ref().map_or(Holds::NOTHING, AtLock::holds)
    }
    fn lock(&mut self) {
        if let Some(t) = self.as_mut() {
            t.lock();
        }
    }
}

/// Writes, from one list, the two exhaustive listings of [`Faraday`] (one
/// reading, one locking) and the [`Area`] names, so they cannot classify
/// a field differently.
macro_rules! faraday_areas {
    (public: [$($p:ident),* $(,)?],
     areas: [$($a:ident => $v:ident),* $(,)?],
     demo: [$($d:ident => $dv:ident),* $(,)?] $(,)?) => {
        impl Faraday {
            /// What every area holds.
            pub(crate) fn holds(&self) -> Holds {
                let Faraday { $($p,)* $($a,)* $(#[cfg(feature = "demo")] $d,)* } = self;
                $( public($p); )*
                let mut h = Holds::NOTHING;
                $( h = h.or(AtLock::holds($a)); )*
                $(
                    #[cfg(feature = "demo")]
                    {
                        h = h.or(AtLock::holds($d));
                    }
                )*
                h
            }

            /// Lock's clears, area by area.
            pub(crate) fn lock_areas(&mut self) {
                let Faraday { $($p,)* $($a,)* $(#[cfg(feature = "demo")] $d,)* } = self;
                $( let _ = $p; )*
                $( AtLock::lock($a); )*
                $(
                    #[cfg(feature = "demo")]
                    AtLock::lock($d);
                )*
            }
        }

        /// Each area of Faraday's state that answers what it holds, by
        /// name: for a leak tour that walks every one.
        #[cfg(feature = "testkit")]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Area {
            $( #[doc = concat!("`Faraday::", stringify!($a), "`.")] $v, )*
            $( #[doc = concat!("`Faraday::", stringify!($d), "`.")] #[cfg(feature = "demo")] $dv, )*
        }

        #[cfg(feature = "testkit")]
        impl Area {
            /// Every area, in the order of `Faraday`'s listing.
            pub const ALL: &[Area] = &[
                $( Area::$v, )*
                $( #[cfg(feature = "demo")] Area::$dv, )*
            ];
        }
    };
}

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
