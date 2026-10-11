//! The screens a row opens over the screen it is on (`docs/DESIGN.md`
//! §5): the Choice behind a value row and the Compare behind a
//! reference row.
//!
//! They are §5 screens wherever they are opened from, so one function
//! draws each of them and the screen underneath keeps its state: reading
//! an address whole is not leaving the transaction.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_bip::keys::ScriptType;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Action, Chrome, Item};

use crate::views::export::format_name;
use crate::views::message::{
    format_name as message_format_name, format_reason as message_format_reason,
};
use crate::{Comparison, ExportFormat, OpenSigner, Overlay, Picker, ids, strings, text};

impl OpenSigner {
    pub(crate) fn view_overlay(&self, o: &Overlay) -> Node {
        match o {
            Overlay::Setting(setting) => self.view_setting(*setting),
            // §5 Menu: the file list draws its own frame too.
            Overlay::Files(list) => self.view_files(list),
            _ => self.with_chrome(Some(ids::BACK), |c| match o {
                Overlay::Choice(Picker::Script | Picker::XpubScript, picked) => {
                    self.script_choice(c, *picked)
                }
                Overlay::Choice(Picker::Format, picked) => self.format_choice(c, *picked),
                Overlay::Choice(Picker::MessageKey(keys) | Picker::ConvertKey(keys), picked) => {
                    self.message_key_choice(c, *keys, *picked)
                }
                Overlay::Choice(Picker::MessageFormat, picked) => {
                    self.message_format_choice(c, *picked)
                }
                Overlay::Choice(Picker::SearchBy, picked) => self.search_by_choice(c, *picked),
                Overlay::Choice(Picker::ReadAs, picked) => self.read_as_choice(c, *picked),
                Overlay::Choice(Picker::FromUnit, picked) => self.unit_choice(c, *picked),
                Overlay::Choice(Picker::PolicyScript, picked) => {
                    self.policy_script_choice(c, *picked)
                }
                Overlay::Choice(Picker::KeepWallet, picked) => self.keep_wallet_choice(c, *picked),
                Overlay::Choice(Picker::Account, picked) => self.account_choice(c, *picked),
                Overlay::Choice(Picker::Bip85App, picked) => self.bip85_app_choice(c, *picked),
                Overlay::Choice(Picker::Bip85Bytes, picked) => self.bip85_bytes_choice(c, *picked),
                Overlay::Choice(Picker::Rescan, picked) => self.rescan_choice(c, *picked),
                Overlay::Choice(Picker::SilentForm, picked) => self.silent_form_choice(c, *picked),
                Overlay::Choice(Picker::LightningFrom, picked) => {
                    self.lightning_from_choice(c, *picked)
                }
                Overlay::Choice(Picker::LightningKey(keys), picked) => {
                    self.message_key_choice(c, *keys, *picked)
                }
                Overlay::Choice(Picker::VanityDial, picked) => self.vanity_dial_choice(c, *picked),
                Overlay::Compare(cmp) => self.compare_screen(c, cmp),
                Overlay::Setting(_) | Overlay::Files(_) => {
                    unreachable!("drawn above")
                }
            }),
        }
    }

    /// §5 Choice, "How?": which dial the vanity grinder turns, what one
    /// candidate costs on each under its name, and, on a device whose
    /// hardware makes a grind slow, that fact at the row's trailing
    /// edge (§4.8, `docs/PLANNING.md` §16.117).
    fn vanity_dial_choice(&self, c: &Chrome<'_>, picked: usize) -> Node {
        let s = self.strings();
        let key = match self.screen {
            crate::Screen::KeyDetail(key) => key,
            _ => 0,
        };
        let slow = self.tier_is_a();
        let items: Vec<Item> = crate::vanity::Dial::ALL
            .iter()
            .enumerate()
            .map(|(i, dial)| {
                let label = crate::views::vanity::dial_name(*dial, s);
                if !self.vanity_dial_available(key, *dial) {
                    return Item::dimmed(label, self.vanity_dial_reason(key));
                }
                Item {
                    id: Some(ids::at(ids::PICK_BASE, i)),
                    label: String::from(label),
                    reason: None,
                    chosen: i == picked,
                    subtitle: Some(String::from(crate::views::vanity::dial_cost(*dial, s))),
                    caution: slow.then(|| String::from(s.vanity_slow)),
                    icon: None,
                    mono: false,
                    mono_subtitle: false,
                }
            })
            .collect();
        screens::choice(
            c,
            s.vanity_how_title,
            items,
            Action::new(ids::PICK_CONTINUE, s.action_continue),
        )
    }

    /// §5 Choice, "Which script type?": the four types §4.6 names, the
    /// current one checked, and Continue.
    fn script_choice(&self, c: &Chrome<'_>, picked: usize) -> Node {
        let s = self.strings();
        let items: Vec<Item> = ScriptType::ALL
            .iter()
            .enumerate()
            .map(|(i, sc)| {
                Item::chosen(
                    ids::at(ids::PICK_BASE, i),
                    text::script_short(*sc, s),
                    i == picked,
                )
            })
            .collect();
        screens::choice(
            c,
            s.script_type_title,
            items,
            Action::new(ids::PICK_CONTINUE, s.action_continue),
        )
    }

    /// §5 Choice, "Keep this wallet on the device?": the two answers,
    /// "No" checked, and Continue. It follows "Add this wallet" for a
    /// wallet this device built over a passphrase key it holds
    /// (`docs/PLANNING.md` §16.104 rule 7).
    fn keep_wallet_choice(&self, c: &Chrome<'_>, picked: usize) -> Node {
        let s = self.strings();
        screens::choice(
            c,
            s.wallet_keep_title,
            vec![
                Item::chosen(ids::at(ids::PICK_BASE, 0), s.wallet_keep_no, picked == 0),
                Item::chosen(ids::at(ids::PICK_BASE, 1), s.wallet_keep_yes, picked == 1),
            ],
            Action::new(ids::PICK_CONTINUE, s.action_continue),
        )
    }

    /// §5 Choice, "Find a word by?": the four notations a word of the
    /// list can be looked up by, and the row that opens the list at its
    /// first word instead.
    fn search_by_choice(&self, c: &Chrome<'_>, picked: usize) -> Node {
        let s = self.strings();
        let mut items: Vec<Item> = crate::wordlist::SearchBy::ALL
            .iter()
            .enumerate()
            .map(|(i, by)| {
                Item::chosen(
                    ids::at(ids::PICK_BASE, i),
                    crate::views::wordlist::search_by_name(*by, s),
                    i == picked,
                )
            })
            .collect();
        items.push(Item::chosen(
            ids::at(ids::PICK_BASE, crate::wordlist::SearchBy::ALL.len()),
            s.wordlist_browse,
            picked == crate::wordlist::SearchBy::ALL.len(),
        ));
        screens::choice(
            c,
            s.wordlist_by_title,
            items,
            Action::new(ids::PICK_CONTINUE, s.action_continue),
        )
    }

    /// §5 Choice, "Read as": how the Hashes field is read, the current
    /// mode checked.
    /// §5 Choice, "Script": what a compiled policy is wrapped in.
    fn policy_script_choice(&self, c: &Chrome<'_>, picked: usize) -> Node {
        let s = self.strings();
        let items: Vec<Item> = osk_bip::compile::PolicyScript::ALL
            .iter()
            .enumerate()
            .map(|(i, script)| {
                Item::chosen(
                    ids::at(ids::PICK_BASE, i),
                    crate::views::calculators::policy_script_name(*script, s),
                    i == picked,
                )
            })
            .collect();
        screens::choice(
            c,
            s.tool_policy_script,
            items,
            Action::new(ids::PICK_CONTINUE, s.action_continue),
        )
    }

    fn read_as_choice(&self, c: &Chrome<'_>, picked: usize) -> Node {
        let s = self.strings();
        let items: Vec<Item> = osk_codec::encodings::ReadAs::ALL
            .iter()
            .enumerate()
            .map(|(i, r)| {
                Item::chosen(
                    ids::at(ids::PICK_BASE, i),
                    crate::views::calculators::read_as_name(*r, s),
                    i == picked,
                )
            })
            .collect();
        screens::choice(
            c,
            s.tool_read_as,
            items,
            Action::new(ids::PICK_CONTINUE, s.action_continue),
        )
    }

    /// §5 Choice, "From": which unit the Units field is typed in.
    fn unit_choice(&self, c: &Chrome<'_>, picked: usize) -> Node {
        let s = self.strings();
        let items: Vec<Item> = osk_ui::components::Denomination::ALL
            .iter()
            .enumerate()
            .map(|(i, u)| {
                Item::chosen(
                    ids::at(ids::PICK_BASE, i),
                    crate::views::calculators::unit_name(*u, s),
                    i == picked,
                )
            })
            .collect();
        screens::choice(
            c,
            s.tool_from,
            items,
            Action::new(ids::PICK_CONTINUE, s.action_continue),
        )
    }

    /// §5 Choice, "Which key?": the loaded keys, the one that will sign
    /// checked. §4.4 gives each row the fingerprint glyph, the
    /// fingerprint and what the key is made of.
    pub(crate) fn message_key_choice(&self, c: &Chrome<'_>, keys: usize, picked: usize) -> Node {
        let s = self.strings();
        let items: Vec<Item> = self
            .keys
            .iter()
            .take(keys)
            .enumerate()
            .map(|(i, k)| {
                Item::key(
                    ids::at(ids::PICK_BASE, i),
                    text::fingerprint_hex(k.fingerprint),
                    self.key_made_of(i),
                    i == picked,
                )
            })
            .collect();
        screens::choice(
            c,
            s.explore_using_title,
            items,
            Action::new(ids::PICK_CONTINUE, s.action_continue),
        )
    }

    /// §5 Choice, "Which format?": the two message formats, with the one
    /// this address form has no signature of dimmed and the reason
    /// beside it (§4.11).
    fn message_format_choice(&self, c: &Chrome<'_>, picked: usize) -> Node {
        let s = self.strings();
        let script = self
            .signing_message()
            .map_or(ScriptType::NativeSegwit, |f| f.script());
        let items: Vec<Item> = crate::message::FORMATS
            .iter()
            .enumerate()
            .map(|(i, f)| match message_format_reason(*f, script, s) {
                Some(reason) => Item::dimmed(message_format_name(*f, s), reason),
                None => Item::chosen(
                    ids::at(ids::PICK_BASE, i),
                    message_format_name(*f, s),
                    i == picked,
                ),
            })
            .collect();
        screens::choice(
            c,
            s.msg_format_row,
            items,
            Action::new(ids::PICK_CONTINUE, s.action_continue),
        )
    }

    /// §5 Choice, "Which format?": the forms a coordinator takes for this
    /// script type. A format the script type has no form of is not on the
    /// list at all, because no choice of this person's brings it back.
    fn format_choice(&self, c: &Chrome<'_>, picked: usize) -> Node {
        let s = self.strings();
        let script = self.export.script;
        let owner = self.export_owner();
        let items: Vec<Item> = ExportFormat::ALL
            .iter()
            .enumerate()
            .filter(|(_, f)| self.format_offered(**f, owner, script))
            .map(|(i, f)| Item::chosen(ids::at(ids::PICK_BASE, i), format_name(*f, s), i == picked))
            .collect();
        screens::choice(
            c,
            s.export_format_title,
            items,
            Action::new(ids::PICK_CONTINUE, s.action_continue),
        )
    }

    /// §5 Choice, "Which account?": the accounts a key exports, each
    /// with its path under the name for the network this device is on
    /// (`docs/PLANNING.md` §16.110 rule 1).
    fn account_choice(&self, c: &Chrome<'_>, picked: usize) -> Node {
        let s = self.strings();
        let items: Vec<Item> = crate::Account::ALL
            .iter()
            .enumerate()
            .map(|(i, account)| {
                Item::valued(
                    ids::at(ids::PICK_BASE, i),
                    account.name(s),
                    account.path(self.network),
                    i == picked,
                )
            })
            .collect();
        screens::choice(
            c,
            s.account_which_title,
            items,
            Action::new(ids::PICK_CONTINUE, s.action_continue),
        )
    }

    /// §5 Choice, "Which application?": the six applications BIP-85
    /// defines that this device derives, Words first because a child key
    /// is the one of them that becomes a key (§16.114).
    fn bip85_app_choice(&self, c: &Chrome<'_>, picked: usize) -> Node {
        let s = self.strings();
        let items: Vec<Item> = crate::Bip85App::ALL
            .iter()
            .enumerate()
            .map(|(i, app)| {
                Item::chosen(
                    ids::at(ids::PICK_BASE, i),
                    crate::views::bip85::app_name(*app, s),
                    i == picked,
                )
            })
            .collect();
        screens::choice(
            c,
            s.bip85_which_title,
            items,
            Action::new(ids::PICK_CONTINUE, s.action_continue),
        )
    }

    /// §5 Choice, "How many bytes?": the three lengths the hex
    /// application is asked for.
    fn bip85_bytes_choice(&self, c: &Chrome<'_>, picked: usize) -> Node {
        let s = self.strings();
        let items: Vec<Item> = crate::HEX_BYTE_COUNTS
            .iter()
            .enumerate()
            .map(|(i, bytes)| {
                Item::chosen(
                    ids::at(ids::PICK_BASE, i),
                    strings::fill1(s.bip85_bytes_row, &alloc::format!("{bytes}")),
                    i == picked,
                )
            })
            .collect();
        screens::choice(
            c,
            s.bip85_bytes_title,
            items,
            Action::new(ids::PICK_CONTINUE, s.action_continue),
        )
    }

    /// §5 Choice, "Rescan from?": where Bitcoin Core starts looking for
    /// the wallet's coins, with what each choice costs under its name
    /// (§16.114).
    fn rescan_choice(&self, c: &Chrome<'_>, picked: usize) -> Node {
        let s = self.strings();
        screens::choice(
            c,
            s.core_rescan_title,
            vec![
                Item::valued(
                    ids::at(ids::PICK_BASE, 0),
                    s.core_rescan_start,
                    String::from(s.core_rescan_start_value),
                    picked == 0,
                ),
                Item::valued(
                    ids::at(ids::PICK_BASE, 1),
                    s.core_rescan_now,
                    String::from(s.core_rescan_now_value),
                    picked == 1,
                ),
            ],
            Action::new(ids::PICK_CONTINUE, s.action_continue),
        )
    }

    /// §5 Compare: the whole string, chunked in fours, or a descriptor as
    /// the structure §4.5 shows instead.
    ///
    /// §5: the title names the kind and the label names the instance,
    /// where there is one; a kind with one instance carries no label, so
    /// nothing on the screen is written twice.
    fn compare_screen(&self, c: &Chrome<'_>, cmp: &Comparison) -> Node {
        let s = self.strings();
        let done = Action::new(ids::COMPARE_DONE, s.action_done);
        let label = cmp.label.as_deref();
        if cmp.descriptor {
            return screens::compare_descriptor(
                c,
                &self.layout_ctx(),
                screens::Compare {
                    title: &cmp.title,
                    label,
                    value: &cmp.value,
                    id: ids::COMPARE_TEXT,
                    done,
                    key: Some(ids::COMPARE_KEY),
                    copy: self.copy_row(),
                    caption: self.notice().map(String::from),
                },
            );
        }
        screens::compare(
            c,
            screens::Compare {
                title: &cmp.title,
                label,
                value: &cmp.value,
                id: ids::COMPARE_TEXT,
                done,
                key: None,
                copy: self.copy_row(),
                caption: self.notice().map(String::from),
            },
        )
    }
}
