//! The ways a key reaches a screen: one component for every screen that
//! takes a key (`docs/KEY-FORMS.md` §4).
//!
//! The table gives it the forms; an [`Opener`] says which forms its screen
//! takes and which ways show first; everything the opener takes that is not
//! shown first is under Other forms, worked out here so no screen lists it
//! by hand.

pub(crate) mod screen;

use crate::Action;
use crate::decode::check::SignerPress;
use crate::drawing::View;
use crate::forms::Form;
use crate::seed_check::SeedAction;
use crate::strings::EN;
use crate::ui::Style;
use faraday_text::Text;

/// A way a key reaches a screen. A Form way is any form of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Way {
    /// Scan a code: it switches form itself (§3.3).
    Scan,
    /// A SeedQR: a writing of BIP-39 words, with a tool of its own.
    SeedQr,
    /// One form of the table.
    Form(Form),
    /// A loaded key, by its place among the loaded keys.
    Holder(usize),
    /// An LND aezeed, read by Lightning's tool.
    Aezeed,
    /// Add a key: its screen takes every form.
    AddAKey,
}

/// What a screen takes and what it shows first.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Opener {
    /// The forms the screen takes.
    pub forms: &'static [Form],
    /// The ways shown first, in order.
    pub up_front: &'static [Way],
    /// Ways under Other forms that are not forms of the table.
    pub other_ways: &'static [Way],
}

/// Add a key: every form; Scan, the word grid and codex32 first.
pub(crate) const ADD_A_KEY: Opener = Opener {
    forms: &Form::ALL,
    up_front: &[Way::Scan, Way::Form(Form::Words), Way::Form(Form::Codex32)],
    other_ways: &[],
};

/// My seed makes my wallet: every form; Type, Scan and codex32 first.
pub(crate) const SEED_MAKES_WALLET: Opener = Opener {
    forms: &Form::ALL,
    up_front: &[Way::Form(Form::Words), Way::Scan, Way::Form(Form::Codex32)],
    other_ways: &[],
};

/// Check a signer's Load the seed: every form; Type, Scan and codex32
/// first.
pub(crate) const CHECK_A_SIGNER: Opener = Opener {
    forms: &Form::ALL,
    up_front: &[Way::Form(Form::Words), Way::Scan, Way::Form(Form::Codex32)],
    other_ways: &[],
};

/// Spend's sign card: every form; Type, Scan and codex32 first.
pub(crate) const SPEND_SIGN: Opener = Opener {
    forms: &Form::ALL,
    up_front: &[Way::Form(Form::Words), Way::Scan, Way::Form(Form::Codex32)],
    other_ways: &[],
};

/// Which opener drew an Other forms control: its press and its sheet
/// carry it, so every opener's rows come from its own constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opening {
    /// Add a key.
    AddAKey,
    /// My seed makes my wallet: the Seed card.
    SeedMakesWallet,
    /// Check a multisig setup's Check a seed against it: the Seed card,
    /// with no loaded key offered.
    SeedAgainst,
    /// Check a signer: Load the seed.
    CheckSigner,
    /// Spend's sign card.
    SpendSign,
}

/// One row of a way: its label, style and press.
pub(crate) type Item = (Text, Style, Action);

impl Opening {
    /// The opener this is.
    pub(crate) fn opener(self) -> &'static Opener {
        match self {
            Opening::AddAKey => &ADD_A_KEY,
            Opening::SeedMakesWallet | Opening::SeedAgainst => &SEED_MAKES_WALLET,
            Opening::CheckSigner => &CHECK_A_SIGNER,
            Opening::SpendSign => &SPEND_SIGN,
        }
    }

    /// Whether Other forms is a sheet on every panel: the opener is a
    /// dialog, with no page to open its rows in.
    pub(crate) fn sheet_only(self) -> bool {
        self == Opening::CheckSigner
    }

    /// Whether the loaded keys are ways of this opener, under Other
    /// forms: the screen does not know which key it wants.
    fn holders(self) -> bool {
        self == Opening::SeedMakesWallet
    }

    /// One way as a row: its label and its press. Add a key's rows show
    /// the form chosen.
    pub(crate) fn item(self, app: &View, way: Way) -> Item {
        let me = if self == Opening::SeedAgainst {
            Opening::SeedMakesWallet
        } else {
            self
        };
        let (words, scan) = match me {
            Opening::AddAKey => (Form::Words.name(), EN.entry_scan_a_seedqr),
            Opening::SeedMakesWallet | Opening::SeedAgainst => {
                (EN.entry_type_the_words, EN.seed_check_scan)
            }
            Opening::CheckSigner => (EN.entry_type_the_words, EN.keygen_scan_a_seedqr),
            Opening::SpendSign => (EN.entry_type_the_words, EN.step_scan_a_seedqr),
        };
        let wi = app.wallets.wallet();
        match way {
            Way::Scan => (
                Text::of(scan),
                Style::Secondary,
                match me {
                    Opening::AddAKey | Opening::SeedMakesWallet | Opening::SeedAgainst => {
                        Action::ScanSeed
                    }
                    Opening::CheckSigner => Action::SignerCheck(SignerPress::SeedScan),
                    Opening::SpendSign => Action::ScanKey(wi),
                },
            ),
            Way::Form(Form::Words) if me != Opening::AddAKey => (
                Text::of(words),
                Style::Secondary,
                match me {
                    Opening::CheckSigner => Action::SignerCheck(SignerPress::SeedWords),
                    Opening::SpendSign => Action::EntryWallet(wi),
                    _ => Action::SeedCheck(SeedAction::Type),
                },
            ),
            Way::Form(f) => (
                Text::of(f.name()),
                Style::chosen(me == Opening::AddAKey && f == app.entry.state().form),
                match me {
                    Opening::AddAKey => Action::EntryForm(f.code()),
                    Opening::CheckSigner => Action::SignerCheck(SignerPress::SeedForm(f.code())),
                    Opening::SpendSign => Action::EntryWalletForm(wi, f.code()),
                    _ => crate::seed_check::FORMS
                        .iter()
                        .position(|x| *x == f)
                        .map_or(Action::SeedCheck(SeedAction::Type), |i| {
                            Action::SeedCheck(SeedAction::Form(i as u8))
                        }),
                },
            ),
            Way::Holder(i) => (
                app.session
                    .keys
                    .all()
                    .nth(i)
                    .map_or_else(Text::default, |key| {
                        let line =
                            crate::wallet::key_line(key.fingerprint, &crate::bip85::key_name(key));
                        if key.has_passphrase {
                            Text::formatted(format_args!(
                                "{} · {}",
                                line.as_str(),
                                EN.seed_check_with_passphrase_mark
                            ))
                        } else {
                            line
                        }
                    }),
                Style::Secondary,
                Action::SeedCheck(SeedAction::Loaded(i)),
            ),
        }
    }

    /// The ways shown first, as rows.
    pub(crate) fn up_front(self, app: &View) -> Vec<Item> {
        self.opener()
            .up_front
            .iter()
            .map(|w| self.item(app, *w))
            .collect()
    }

    /// The ways under Other forms, as rows: the forms the opener takes
    /// less those shown first, then the loaded keys where it lists them.
    pub(crate) fn others(self, app: &View) -> Vec<Item> {
        let mut rows: Vec<Item> = self
            .opener()
            .other_forms()
            .into_iter()
            .map(|f| self.item(app, Way::Form(f)))
            .collect();
        if self.holders() {
            let n = app.session.keys.all().count();
            rows.extend((0..n).map(|i| self.item(app, Way::Holder(i))));
        }
        rows
    }
}

/// Restore's open slot: every form; Type the words, Scan and codex32
/// first.
pub(crate) const RESTORE_SLOT: Opener = Opener {
    forms: &Form::ALL,
    up_front: &[Way::Form(Form::Words), Way::Scan, Way::Form(Form::Codex32)],
    other_ways: &[],
};

/// Check's backup menu: the copies Check reads back; the SeedQR and
/// codex32 first.
pub(crate) const CHECK_BACKUP: Opener = Opener {
    forms: &[Form::Slip39, Form::Xor, Form::Codex32, Form::Krux],
    up_front: &[Way::SeedQr, Way::Form(Form::Codex32)],
    other_ways: &[],
};

/// Convert's From: the keys held, Add a key (which takes every form); a
/// SLIP-39 set and an aezeed, the two sources Add a key does not read,
/// under Other forms.
pub(crate) const CONVERT_FROM: Opener = Opener {
    forms: &[Form::Slip39],
    up_front: &[Way::AddAKey],
    other_ways: &[Way::Aezeed],
};

impl Opener {
    /// The forms shown first, in order.
    pub(crate) fn up_front_forms(&self) -> impl Iterator<Item = Form> + '_ {
        self.up_front.iter().filter_map(|w| match w {
            Way::Form(f) => Some(*f),
            Way::Scan | Way::Holder(_) | Way::SeedQr | Way::Aezeed | Way::AddAKey => None,
        })
    }

    /// The forms under Other forms: the forms the opener takes less those
    /// shown first.
    pub(crate) fn other_forms(&self) -> Vec<Form> {
        self.forms
            .iter()
            .copied()
            .filter(|f| !self.up_front.contains(&Way::Form(*f)))
            .collect()
    }
}
