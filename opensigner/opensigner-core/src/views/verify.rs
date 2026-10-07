//! Verify › address, built from `docs/DESIGN.md` §5: the address Entry
//! and the Result that says whose the address is.
//!
//! The ways an address arrives are the scanner's rows (PLANNING
//! §16.88): the wallet page's row opens the scanner, and this screen is
//! what its "Type" row opens.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_ui::components;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Above, Action, Chrome, Entry};
use osk_ui::widgets::keyboard::{self, KeyboardKind};
use osk_ui::widgets::{Icon, Tone};

use crate::verify::{self, AddressResult, SEARCH_DEPTH, VerifyStage};
use crate::{OpenSigner, ids, strings, text};

impl OpenSigner {
    pub(crate) fn view_verify(&self) -> Node {
        self.with_chrome(Some(ids::BACK), |c| match self.verify.stage() {
            VerifyStage::Typing => self.verify_typing(c),
            VerifyStage::Result => self.verify_answer(c),
        })
    }

    /// §5 Entry, "Address": the typed address whole above the group, the
    /// field, the reserved caption line and the keyboard. ✓ is dead until
    /// the text is an address (§4.3).
    ///
    /// §4.3: "An address being typed (Verify) is the one value whose tail
    /// is not enough: the space above the entry group holds the typed
    /// address whole, as a comparison string at the largest size that
    /// fits, on every class, and the field under it keeps the tail in view
    /// as usual." This is the screen whose whole purpose is to stop a
    /// person accepting an address on its tail, so the head is on the
    /// screen from the first character; the block keeps the space's
    /// rectangle, so nothing moves when that character arrives.
    ///
    /// The keyboard is §4.3's address layout: bech32 on the QWERTY
    /// positions, base58 behind the shift, and every key that cannot
    /// continue an address on this network dimmed.
    fn verify_typing(&self, c: &Chrome<'_>) -> Node {
        let s = self.strings();
        let parses = self.verify.parses();
        let error = (!parses && !self.verify.input().is_empty())
            .then(|| String::from(s.verify_invalid_title));
        screens::entry(
            c,
            Entry {
                candidates: None,
                title: s.verify_title,
                value: String::from(self.verify.input()),
                mono: true,
                above: Above::Whole {
                    id: ids::VERIFY_TYPED,
                    text: String::from(self.verify.input()),
                },
                words: None,
                eye: None,
                keyboard: (ids::VERIFY_KEYBOARD, KeyboardKind::Address),
                enabled: verify::address_keys(self.verify.input(), self.network)
                    | if parses { 0 } else { keyboard::DONE_DISABLED },
                error,
            },
        )
    }

    /// §5 Result: whose the address is, the facts behind the answer, and
    /// the address itself as the reference row §4.5 gives every long
    /// string.
    fn verify_answer(&self, c: &Chrome<'_>) -> Node {
        let s = self.strings();
        let result = self
            .verify
            .result()
            .cloned()
            .unwrap_or(AddressResult::Invalid);
        let address = || {
            components::Record::reference(
                ids::VERIFY_ADDRESS,
                s.row_address,
                String::from(self.verify.input()),
            )
        };
        let (icon, tone, title, rows) = match &result {
            AddressResult::Yours {
                fingerprint,
                script,
                change,
                index,
            } => (
                Icon::Success,
                Tone::Success,
                s.verify_yours_title,
                vec![
                    components::Record::fingerprint(
                        s.sign_key_row,
                        text::fingerprint_hex(*fingerprint),
                    ),
                    components::Record::text(
                        s.row_path,
                        text::address_label(*script, *change, *index, s),
                        Tone::Text,
                    ),
                    address(),
                ],
            ),
            AddressResult::Wallet {
                wallet,
                change,
                index,
            } => {
                let policy = self.wallets.get(*wallet);
                let path = policy.map_or_else(String::new, |p| {
                    text::address_label(p.script_type(), *change, *index, s)
                });
                (
                    Icon::Success,
                    Tone::Success,
                    s.verify_yours_title,
                    vec![
                        components::Record::text(
                            s.wallet_title,
                            policy.map_or_else(String::new, |p| self.wallet_label(p)),
                            Tone::Text,
                        ),
                        components::Record::text(s.row_path, path, Tone::Text),
                        address(),
                    ],
                )
            }
            AddressResult::NotFound => (
                Icon::Warning,
                Tone::Caution,
                s.verify_not_found_title,
                vec![
                    components::Record::text(
                        s.verify_searched,
                        strings::fill1(s.verify_search_depth, &alloc::format!("{SEARCH_DEPTH}")),
                        Tone::Text,
                    ),
                    address(),
                ],
            ),
            AddressResult::WrongNetwork => (
                Icon::Error,
                Tone::Danger,
                s.verify_network_title,
                vec![components::Record::text(
                    s.confirm_network,
                    self.network.name(),
                    Tone::Text,
                )],
            ),
            // §16.113: a silent payment address cannot be checked
            // against a wallet, because nothing is ever paid to it. The
            // screen says what it is and shows it whole.
            AddressResult::Silent => (
                Icon::Info,
                Tone::Text,
                s.verify_silent_title,
                vec![address()],
            ),
            AddressResult::Invalid => (
                Icon::Error,
                Tone::Danger,
                s.verify_invalid_title,
                Vec::new(),
            ),
        };
        screens::result(
            c,
            screens::Result {
                caption: None,
                title: s.verify_entry_title,
                icon,
                tone,
                result: title,
                rows,
                actions: vec![Action::new(ids::VERIFY_CLEAR, s.verify_check_another)],
            },
        )
    }
}
