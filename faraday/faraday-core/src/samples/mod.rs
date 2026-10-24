//! The shapes of Faraday's samples (F81, `docs/MODES.md` §5): public
//! dummies a person can load in Demo. The data is in `faraday-samples`,
//! a crate only the desktop app depends on; this module holds the types
//! it fills in and what the session does with them, so core never links
//! the data (`tools/lint-keys-tests.sh` holds that).
//!
//! Every field is public data of `&'static` text or bytes. Nothing here
//! parses or derives; the fingerprints are literals that the samples
//! crate's tests check against the code that derives them.
//!
//! The Load samples sheet (`screen.rs`) and what its presses do are here:
//! the sheet lists the samples, a person ticks some, and Load ticked puts
//! them in the session through the paths a typed or scanned key and wallet
//! take (`docs/MODES.md` §5, §10 M6).

pub(crate) mod put;
pub(crate) mod screen;

use crate::inbox::{Part, Secret, part_of};
use crate::strings::{EN, fill};
use crate::wallet::{Origin, read_wallet, same_wallet};
use crate::{Action, Faraday, Mode, Sheet};
use faraday_bip::keys::Fingerprint;
use faraday_crypto::work::Compute;
use faraday_keys::Handout;

/// The passphrase of every sample that has one: the backup, the vault, and
/// the second fingerprint of each key that takes a passphrase
/// (`docs/MODES.md` §6.2, §10 M6b). All lowercase.
pub const PASSPHRASE: &str = crate::demo::PASSPHRASE;

/// The form a key sample is read in (`docs/MODES.md` §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleForm {
    /// BIP-39 words, in any of the lists.
    Words,
    /// SLIP-39 shares.
    Slip39,
    /// codex32 strings: a secret, or the shares of a set.
    Codex32,
    /// Seed XOR parts, which make the key's words.
    Xor,
    /// A master seed, which has no words (a 64-byte hex seed).
    Seed,
    /// An LND cipher seed, for the Lightning node key only.
    Aezeed,
}

impl SampleForm {
    /// Whether a key of this form takes a passphrase (BIP-39, SLIP-39, and
    /// Seed XOR, whose result is BIP-39 words), so is public twice: with
    /// none and with `faraday`.
    pub fn takes_passphrase(self) -> bool {
        matches!(
            self,
            SampleForm::Words | SampleForm::Slip39 | SampleForm::Xor
        )
    }
}

/// How big a key sample is, as its type line says it (`docs/MODES.md`
/// §10 M6b): what the form counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleSize {
    /// BIP-39 words (or an LND cipher seed's).
    Words(u8),
    /// BIP-39 words of the Japanese list.
    JapaneseWords(u8),
    /// This many shares, typed or scanned.
    Shares(u8),
    /// SLIP-39 shares, `need` of the `total` the set was dealt in.
    Threshold {
        /// How many shares recover the key.
        need: u8,
        /// How many the set holds.
        total: u8,
    },
    /// SLIP-39 shares of a set in groups, this many of them.
    GroupShares(u8),
    /// A codex32 secret, which is the key itself.
    Secret,
    /// Seed XOR parts.
    Parts(u8),
    /// A master seed of this many bytes.
    Bytes(u8),
}

/// A sample key.
#[derive(Debug, Clone, Copy)]
pub struct SampleKey {
    /// The name of its string in `faraday-strings` (`sample_words`, ...).
    pub name: &'static str,
    /// For a words sample made of one word repeated: the word; else "".
    pub word: &'static str,
    /// For such a sample: how many words; else 0.
    pub count: u8,
    /// The form it is read in.
    pub form: SampleForm,
    /// How big it is: what the form counts.
    pub size: SampleSize,
    /// Whether the Load samples sheet lists it. A key a wallet needs and
    /// the table of §5.1 does not list (the 3 of 5's) is not.
    pub listed: bool,
    /// What is typed or scanned, one text per part: the phrase, the
    /// shares (as many as load the key), the codex32 strings, the Seed XOR
    /// parts' words, the hex master seed, the cipher seed's words.
    pub parts: &'static [&'static str],
    /// For Seed XOR: the words of the key the parts make; else "".
    pub combined: &'static str,
    /// The key's fingerprint with no passphrase; `None` for a cipher
    /// seed, which loads no key.
    pub plain: Option<[u8; 4]>,
    /// Its fingerprint with the passphrase `faraday`, for a form that
    /// takes one.
    pub with_passphrase: Option<[u8; 4]>,
    /// For a cipher seed: the Lightning node id it gives.
    pub node: Option<[u8; 33]>,
}

/// What a sample wallet is, apart from its script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleKind {
    /// One key.
    Single,
    /// `need` signatures of `total` keys.
    Multisig {
        /// Signatures required.
        need: u8,
        /// Keys.
        total: u8,
    },
    /// A MuSig2 aggregate of this many keys.
    MuSig(u8),
    /// A FROST group: `need` of `total` shares sign. Compiled out of the
    /// app (F109).
    #[cfg(feature = "unreleased")]
    Frost {
        /// Shares that sign.
        need: u8,
        /// Shares dealt.
        total: u8,
    },
    /// A key, and a second that spends after a delay.
    Timelock,
}

/// The script a sample wallet pays to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleScript {
    /// Pay to public key hash, or to script hash.
    Legacy,
    /// SegWit nested in script hash.
    NestedSegwit,
    /// Native SegWit.
    NativeSegwit,
    /// Taproot.
    Taproot,
    /// Taproot with a tree of scripts.
    TaprootTree,
}

/// A sample wallet (`docs/MODES.md` §5.2).
#[derive(Debug, Clone, Copy)]
pub struct SampleWallet {
    /// The name of its string in `faraday-strings` (`sample_savings`, ...).
    pub name: &'static str,
    /// What kind of wallet it is.
    pub kind: SampleKind,
    /// The script it pays to.
    pub script: SampleScript,
    /// What `Session::add_wallet` takes: its descriptor, or for the FROST
    /// wallet its record text.
    pub text: &'static str,
    /// Its unsigned spend (PSBT bytes); empty where there is none.
    pub spend: &'static [u8],
    /// The fingerprints of the sample keys it uses, in its own order:
    /// what the tick loads with it.
    pub keys: &'static [[u8; 4]],
}

/// The other inputs a flow takes (`docs/MODES.md` §5.3).
#[derive(Debug, Clone, Copy)]
pub struct SampleInputs {
    /// A vault file (`.ofv`) under the passphrase `faraday`.
    pub vault: &'static [u8],
    /// A signed message, in the text form a person pastes: made by sample
    /// Spending's first receive address.
    pub signed_message: &'static str,
    /// That address.
    pub address: &'static str,
    /// A TOTP setup code (`otpauth://` text).
    pub totp: &'static str,
}

/// Every sample, as the samples crate holds them.
#[derive(Debug, Clone, Copy)]
pub struct Samples {
    /// The keys, in the order the sheet lists them.
    pub keys: &'static [SampleKey],
    /// The wallets, in the order the sheet lists them.
    pub wallets: &'static [SampleWallet],
    /// The other inputs.
    pub inputs: SampleInputs,
}

/// The phrase of a words sample made of one word: `word` `count` times.
pub fn repeated(word: &str, count: u8) -> String {
    vec![word; usize::from(count)].join(" ")
}

impl Samples {
    /// What New key hands out: the words samples made of one word (any
    /// length), in the order of `keys`. Allocates a short list.
    pub fn handouts(&self) -> Vec<Handout> {
        self.keys
            .iter()
            .filter(|k| k.form == SampleForm::Words && !k.word.is_empty())
            .filter_map(|k| {
                Some(Handout {
                    name: k.word,
                    words: k.count,
                    plain: Fingerprint(k.plain?),
                    with_passphrase: Fingerprint(k.with_passphrase?),
                })
            })
            .collect()
    }

    /// Every fingerprint Demo takes: each key's with none, and each key
    /// that takes a passphrase with `faraday` too. Allocates a list.
    pub fn public(&self) -> Vec<Fingerprint> {
        self.keys
            .iter()
            .flat_map(|k| [k.plain, k.with_passphrase])
            .flatten()
            .map(Fingerprint)
            .collect()
    }

    /// The Lightning node ids Demo takes. Allocates a short list.
    pub fn nodes(&self) -> Vec<[u8; 33]> {
        self.keys.iter().filter_map(|k| k.node).collect()
    }
}

/// Which row of the Load samples sheet a press is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    /// The sample wallet at this place in [`Samples::wallets`].
    Wallet(u8),
    /// The sample key at this place in [`Samples::keys`].
    Key(u8),
}

/// One row of the sheet.
#[derive(Debug, Clone)]
pub(crate) struct Row {
    /// Its place in [`Samples::wallets`] or [`Samples::keys`].
    pub(crate) sample: usize,
    /// What the sample is, as its value: made once when the sheet opens
    /// from what the sample records, never while drawing.
    pub(crate) kind: String,
    /// Ticked by the person, to load.
    pub(crate) ticked: bool,
    /// Already in the session: shown ticked and dimmed.
    pub(crate) loaded: bool,
}

/// What the Load samples sheet keeps: its rows, with what is ticked and
/// what is loaded already. Kept in step with the session by being built
/// when the sheet opens and dropped when Load ticked has changed the
/// session, the only place the session's keys or wallets change while
/// the sheet is up (it is a sheet: nothing else is pressed), so drawing
/// reads it and works nothing out. A test changes the session, then opens
/// the sheet, and sees the change.
#[derive(Debug, Default)]
pub(crate) struct SheetState {
    /// The sample wallets, in the order of [`Samples::wallets`].
    pub(crate) wallets: Vec<Row>,
    /// The sample keys the sheet lists, in the order of [`Samples::keys`].
    pub(crate) keys: Vec<Row>,
    /// Demo's `faraday` chips: the person pressed in a passphrase field,
    /// or typed, and has done nothing else since, so a field in focus
    /// shows no chip (`docs/MODES.md` §6.2).
    pub(crate) pass_pressed: bool,
}

impl SheetState {
    /// Whether any row is ticked.
    pub(crate) fn any_ticked(&self) -> bool {
        self.wallets.iter().chain(&self.keys).any(|r| r.ticked)
    }

    fn row_mut(&mut self, pick: Pick) -> Option<&mut Row> {
        match pick {
            Pick::Wallet(i) => self.wallets.get_mut(usize::from(i)),
            Pick::Key(i) => self.keys.get_mut(usize::from(i)),
        }
    }
}

impl SampleWallet {
    /// What the wallet is: its kind and its script, "2-of-3 multisig ·
    /// native SegWit". A few strings put together.
    pub fn kind_label(&self) -> String {
        let en = &EN;
        let n = |n: u8| n.to_string();
        let kind = match self.kind {
            SampleKind::Single => en.sample_kind_single.to_string(),
            SampleKind::Multisig { need, total } => {
                fill(en.sample_kind_multisig, &[&n(need), &n(total)])
            }
            SampleKind::MuSig(keys) => fill(en.sample_kind_musig, &[&n(keys)]),
            #[cfg(feature = "unreleased")]
            SampleKind::Frost { need, total } => fill(en.sample_kind_frost, &[&n(need), &n(total)]),
            SampleKind::Timelock => en.sample_kind_timelock.to_string(),
        };
        let script = match self.script {
            SampleScript::Legacy => en.wallet_legacy,
            SampleScript::NestedSegwit => en.wallet_nested_segwit,
            SampleScript::NativeSegwit => en.wallet_native_segwit,
            SampleScript::Taproot => en.wallet_taproot,
            SampleScript::TaprootTree => en.sample_script_tree,
        };
        fill(en.sample_type, &[&kind, script])
    }
}

/// What a sample is called on screen, from the name its data carries: the
/// string of that name in `faraday-strings`. `None` for a name that is no
/// string there. The repeated-word samples take their word and its count.
pub fn label(name: &str, word: &str, count: u8) -> Option<String> {
    let en = &EN;
    Some(match name {
        "sample_words" => fill(en.sample_words, &[word, &count.to_string()]),
        "sample_abandon" => en.sample_abandon.to_string(),
        "sample_japanese" => en.sample_japanese.to_string(),
        "sample_slip39_one" => en.sample_slip39_one.to_string(),
        "sample_slip39_two_of_three" => en.sample_slip39_two_of_three.to_string(),
        "sample_slip39_groups" => en.sample_slip39_groups.to_string(),
        "sample_codex32_secret" => en.sample_codex32_secret.to_string(),
        "sample_codex32_shares" => en.sample_codex32_shares.to_string(),
        "sample_xor" => en.sample_xor.to_string(),
        "sample_aezeed" => en.sample_aezeed.to_string(),
        "sample_savings" => en.sample_savings.to_string(),
        "sample_spending" => en.sample_spending.to_string(),
        "sample_taproot" => en.sample_taproot.to_string(),
        "sample_nested" => en.sample_nested.to_string(),
        "sample_legacy" => en.sample_legacy.to_string(),
        "sample_nested_multisig" => en.sample_nested_multisig.to_string(),
        "sample_legacy_multisig" => en.sample_legacy_multisig.to_string(),
        "sample_taproot_multisig" => en.sample_taproot_multisig.to_string(),
        "sample_inheritance" => en.sample_inheritance.to_string(),
        "sample_tree" => en.sample_tree.to_string(),
        "sample_musig" => en.sample_musig.to_string(),
        #[cfg(feature = "unreleased")]
        "sample_frost" => en.sample_frost.to_string(),
        "sample_three_of_five" => en.sample_three_of_five.to_string(),
        _ => return None,
    })
}

impl SampleKey {
    /// What the key is: its form and its size, "BIP-39 · 24 words". A few
    /// strings put together.
    pub fn kind_label(&self) -> String {
        let en = &EN;
        let form = match self.form {
            SampleForm::Words => en.sample_form_bip39,
            SampleForm::Slip39 => en.sample_form_slip39,
            SampleForm::Codex32 => en.sample_form_codex32,
            SampleForm::Xor => en.sample_form_xor,
            SampleForm::Seed => en.sample_form_seed,
            SampleForm::Aezeed => en.sample_form_aezeed,
        };
        let n = |n: u8| n.to_string();
        let size = match self.size {
            SampleSize::Words(c) => fill(en.sample_size_words, &[&n(c)]),
            SampleSize::JapaneseWords(c) => fill(en.sample_size_words_japanese, &[&n(c)]),
            SampleSize::Shares(1) => en.sample_size_share.to_string(),
            SampleSize::Shares(c) => fill(en.sample_size_shares, &[&n(c)]),
            SampleSize::Threshold { need, total } => {
                fill(en.sample_size_threshold, &[&n(need), &n(total)])
            }
            SampleSize::GroupShares(c) => fill(en.sample_size_groups, &[&n(c)]),
            SampleSize::Secret => en.sample_size_secret.to_string(),
            SampleSize::Parts(c) => fill(en.sample_size_parts, &[&n(c)]),
            SampleSize::Bytes(c) => fill(en.sample_size_bytes, &[&n(c)]),
        };
        fill(en.sample_type, &[form, &size])
    }

    /// What the key is called on screen.
    pub(crate) fn label(&self) -> String {
        label(self.name, self.word, self.count).unwrap_or_default()
    }

    /// Whether the sheet lists it: a key that loads (a cipher seed loads
    /// no key) and that the table of `docs/MODES.md` §5.1 lists.
    pub(crate) fn tickable(&self) -> bool {
        self.listed && self.plain.is_some()
    }

    /// Whether the session holds it, with no passphrase or with
    /// `faraday`. A lookup.
    pub(crate) fn held(&self, keys: &faraday_keys::Keys) -> bool {
        [self.plain, self.with_passphrase]
            .into_iter()
            .flatten()
            .any(|fp| keys.position_of(fp).is_some())
    }

    /// Costs a stretched key and a master key; a backup, an Argon2id run.
    /// Adds the key the sample is to the session, with no passphrase,
    /// through the path a typed or scanned key of its form takes. False
    /// when the session refuses it (already there) or it loads no key.
    fn add(&self, c: &Compute, keys: &mut faraday_keys::Keys) -> bool {
        // Already there: looked up, not derived to be refused.
        if self.held(keys) {
            return false;
        }
        let label = self.label();
        let part = |i: usize| self.parts.get(i).copied().unwrap_or_default();
        let added = match self.form {
            SampleForm::Words => keys.add_words_with(c, part(0), "", &label, None),
            SampleForm::Xor => keys.add_words_with(c, self.combined, "", &label, None),
            SampleForm::Slip39 => {
                let shares = self
                    .parts
                    .iter()
                    .filter_map(|p| match part_of(p) {
                        Some(Part::Slip39(s)) => Some(s),
                        _ => None,
                    })
                    .collect();
                Secret::Slip39(shares).add(c, "", &label, keys)
            }
            SampleForm::Codex32 => {
                let parts: Vec<_> = self
                    .parts
                    .iter()
                    .filter_map(|p| match part_of(p) {
                        Some(Part::Codex32(s)) => Some(s),
                        _ => None,
                    })
                    .collect();
                match faraday_keys::codex32_seed(&parts) {
                    Ok(seed) => keys.add_seed(c, &seed, &label),
                    Err(_) => return false,
                }
            }
            SampleForm::Seed => keys.add_seed(c, &unhex(part(0)), &label),
            SampleForm::Aezeed => return false,
        };
        added.is_ok()
    }
}

/// The bytes of a hex text; a pair that is not hex ends it.
fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map_while(|i| u8::from_str_radix(s.get(2 * i..2 * i + 2)?, 16).ok())
        .collect()
}

/// What Load ticked says it loaded: "1 wallet", "3 keys" or "1 wallet and
/// 3 keys".
fn loaded_text(wallets: usize, keys: usize) -> String {
    let count = |n: usize, one: &str, many: &str| match n {
        0 => None,
        1 => Some(fill(one, &["1"])),
        n => Some(fill(many, &[&n.to_string()])),
    };
    let w = count(wallets, EN.samples_n_wallet, EN.samples_n_wallets);
    let k = count(keys, EN.samples_n_key, EN.samples_n_keys);
    match (w, k) {
        (Some(w), Some(k)) => fill(EN.samples_loaded_pair, &[&w, &k]),
        (w, k) => w.or(k).unwrap_or_default(),
    }
}

/// The samples area: the Load samples sheet's rows.
#[derive(Default)]
pub(crate) struct SamplesArea {
    sheet: SheetState,
}

impl SamplesArea {
    /// The Load samples sheet's rows.
    pub(crate) fn sheet(&self) -> &SheetState {
        &self.sheet
    }

    /// Whether any row of the sheet is ticked.
    pub(crate) fn any_ticked(&self) -> bool {
        self.sheet.any_ticked()
    }

    /// Whether the passphrase field's chip has been pressed past.
    pub(crate) fn pass_pressed(&self) -> bool {
        self.sheet.pass_pressed
    }

    /// Notes whether `action` presses a passphrase field, past its chip.
    pub(crate) fn note_press(&mut self, action: crate::Action) {
        self.sheet.pass_pressed = crate::demo::presses_pass_field(action);
    }

    /// Notes that the passphrase field's chip has been pressed past.
    pub(crate) fn pressed_past(&mut self) {
        self.sheet.pass_pressed = true;
    }
}

impl Faraday {
    /// Whether the Load samples pill and sheet are on: Demo, with the
    /// samples set by the shell (the desktop). The stick, the Pi and every
    /// other mode have none.
    pub fn samples_on(&self) -> bool {
        self.samples.is_some() && self.mode() == Mode::Demo
    }

    /// Costs a wallet read for each sample wallet, once a wallet is loaded.
    /// The sheet's rows as the session stands now, nothing ticked.
    fn samples_rows(&self, c: &Compute, set: &Samples) -> SheetState {
        let have: Vec<String> = self
            .session
            .wallets
            .iter()
            .map(|w| same_wallet(w.policy()))
            .collect();
        let wallets = set
            .wallets
            .iter()
            .enumerate()
            .map(|(sample, w)| Row {
                sample,
                kind: w.kind_label(),
                ticked: false,
                loaded: !have.is_empty()
                    && read_wallet(c, w.text).is_ok_and(|p| have.contains(&same_wallet(&p))),
            })
            .collect();
        let keys = set
            .keys
            .iter()
            .enumerate()
            .filter(|(_, k)| k.tickable())
            .map(|(sample, k)| Row {
                sample,
                kind: k.kind_label(),
                ticked: false,
                loaded: k.held(&self.session.keys),
            })
            .collect();
        SheetState {
            wallets,
            keys,
            ..Default::default()
        }
    }

    /// Handles the Load samples actions (`Samples`, `SampleTick`,
    /// `SamplesAll`, `SamplesLoad`). Nothing happens where the pill is not
    /// on.
    pub(crate) fn samples_act(&mut self, action: Action) {
        let Some(set) = self.samples.filter(|_| self.samples_on()) else {
            return;
        };
        match action {
            Action::Samples => {
                let c = self.compute();
                self.sample_area.sheet = self.samples_rows(&c, set);
                self.sheet = Some(Sheet::Samples);
            }
            Action::SampleTick(pick) => {
                if let Some(r) = self.sample_area.sheet.row_mut(pick)
                    && !r.loaded
                {
                    r.ticked = !r.ticked;
                }
            }
            Action::SamplesAll => {
                let on = !self.sample_area.sheet.any_ticked();
                let s = &mut self.sample_area.sheet;
                for r in s.wallets.iter_mut().chain(&mut s.keys) {
                    r.ticked = on && !r.loaded;
                }
            }
            Action::SamplesLoad => self.samples_load(set),
            Action::UseSample => self.use_sample(),
            Action::ScanSample => self.scan_use_sample(),
            Action::SamplePsbt => self.sample_psbt_use(),
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }

    /// Costs what the keys and wallets ticked cost: a stretched key and a
    /// master key for each key (an Argon2id run for the backup), a wallet
    /// read for each wallet. Loads what is ticked, wallets first, each with
    /// the sample keys it uses, then the keys, through the paths a typed or
    /// scanned key and wallet take. One already loaded is skipped, not
    /// shown as an error. The sheet closes and one toast counts what
    /// loaded.
    fn samples_load(&mut self, set: &'static Samples) {
        if !self.sample_area.sheet.any_ticked() || !self.may_load_keys() {
            return;
        }
        let c = self.compute();
        let sheet = std::mem::take(&mut self.sample_area.sheet);
        let (mut wallets, mut keys) = (0, 0);
        for r in sheet.wallets.iter().filter(|r| r.ticked && !r.loaded) {
            let w = &set.wallets[r.sample];
            let name = label(w.name, "", 0).unwrap_or_default();
            let before = self.session.wallets.len();
            let source = EN.samples_source;
            let added = self
                .session
                .add_wallet(&c, &name, w.text, source, Origin::Statement);
            wallets += usize::from(added.is_ok() && self.session.wallets.len() > before);
            for fp in w.keys {
                let sample = set.keys.iter().find(|k| k.plain == Some(*fp));
                keys += sample.map_or(0, |k| usize::from(k.add(&c, &mut self.session.keys)));
            }
        }
        for r in sheet.keys.iter().filter(|r| r.ticked && !r.loaded) {
            keys += usize::from(set.keys[r.sample].add(&c, &mut self.session.keys));
        }
        self.sheet = None;
        if wallets + keys > 0 {
            self.refresh_spend();
            self.toast(&fill(EN.samples_loaded, &[&loaded_text(wallets, keys)]));
        }
    }
}

impl Sheet {
    /// Whether this sheet scrolls its own region on a wide display, with
    /// the page's overlay scrollbar (`docs/MOTION.md` §3.8): the sheets
    /// whose long content sits under pinned buttons or a head.
    pub(crate) fn scrolls_with_bar(self) -> bool {
        matches!(
            self,
            Sheet::Learn
                | Sheet::WordList
                | Sheet::Import
                | Sheet::UnlockLoad
                | Sheet::Samples
                | Sheet::Mode
                | Sheet::Pull
        )
    }
}
