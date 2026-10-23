//! Silent payments (BIP-352), the receiving side: a loaded key's
//! `sp1…` address and its labelled ones, the public record a coordinator
//! keeps, the scan key a scanner needs, and a silent payments wallet among
//! the session's wallets, with a check of whether a transaction pays it.
//! Every computation is `faraday-bip::silent`'s and
//! `silent_wallet`'s. The address
//! and the record are public; the scan key shows every payment to the
//! address, so it is a secret: into the vault, or out only through the
//! secret sheet. Nothing here is kept; the keys are derived when used.

pub mod check;
pub(crate) mod screen;

use crate::strings::EN;
use crate::strings::fill;
use faraday_bip::silent_wallet::{MAX_LABELS, SilentWallet};
use faraday_crypto::work::Compute;
use faraday_keys::silent::Receiver;
use faraday_vault::Record;
use faraday_vault::records::{field, kind};
use zeroize::Zeroizing;

use crate::wallet::{Session, fp_text};
use crate::{Faraday, QrFormat, QrSource, QrView, Screen, flow};

/// The cards of the flow.
pub mod sstep {
    /// Which loaded key.
    pub const KEY: u8 = 0;
    /// The address, and a label.
    pub const ADDRESS: u8 = 1;
    /// The scan key.
    pub const SCAN: u8 = 2;
    /// Whether a transaction pays the wallet.
    pub const CHECK: u8 = 3;
    /// How many cards.
    pub const COUNT: usize = 4;
}

/// The flow's state: a key and a label number. None of it is a secret.
pub struct SilentState {
    /// The open card.
    pub open: Option<u8>,
    /// Cards closed as done.
    pub done: [bool; sstep::COUNT],
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// The key, by its fingerprint.
    pub key: Option<[u8; 4]>,
    /// The label shown: 0 for none, else 1 to [`MAX_LABELS`].
    pub label: u32,
    /// Labels handed out so far, for the record.
    pub labels: u32,
    /// The session wallet this is the page of, once there is one.
    pub wallet: Option<usize>,
    /// A transaction being checked: the file it came from, and the check
    /// with its previous outputs as far as Files has them.
    pub check: Option<(String, crate::silent::check::Check)>,
    /// Where the flow returns to.
    pub back: Screen,
    /// What the page shows of the key, kept by what it is derived from
    /// ([`Faraday::silent_shown`]). Public: no key is kept.
    pub(crate) shown: std::cell::RefCell<Option<(SilentShownOf, SilentShown)>>,
}

/// What the Silent page shows that is derived from its key: whether the
/// wallet's key is loaded, and the address on show. Public.
#[derive(Clone, Default)]
pub struct SilentShown {
    /// Whether the wallet's key is loaded ([`Faraday::silent_key_here`]).
    pub key_here: bool,
    /// The address on show ([`Faraday::silent_address`]).
    pub address: Option<String>,
}

/// What [`SilentShown`] is derived from: the key, the label, the wallet,
/// the loaded key's id and the network.
pub(crate) type SilentShownOf = (
    Option<[u8; 4]>,
    u32,
    Option<usize>,
    Option<[u8; 32]>,
    faraday_bip::keys::Network,
);

/// The Silent payments flow's area: its state while it is open.
#[derive(Default)]
pub(crate) struct SilentArea {
    state: Option<SilentState>,
}

impl SilentArea {
    /// The flow's state, while it is open.
    pub(crate) fn state(&self) -> Option<&SilentState> {
        self.state.as_ref()
    }

    /// Forgets the flow's state.
    pub(crate) fn clear(&mut self) {
        self.state = None;
    }

    /// The column's scroll, while the flow is open.
    pub(crate) fn scroll_mut(&mut self) -> Option<&mut flow::Scroll> {
        self.state.as_mut().map(|s| &mut s.scroll)
    }

    /// Takes the column's scroll as the pass left it.
    pub(crate) fn set_scroll(&mut self, to: flow::Scroll) {
        if let Some(s) = self.state.as_mut() {
            s.scroll = to;
        }
    }

    /// Opens the flow over the session's keys, to return to `back`.
    pub(crate) fn open(&mut self, session: &Session, back: Screen) {
        let key = (session.keys.len() == 1).then(|| session.keys[0].fingerprint.0);
        let wallet = key.and_then(|fp| Self::wallet_of(session, fp));
        let labels = wallet
            .and_then(|i| session.wallets[i].policy().silent())
            .map_or(0, |r| r.labels);
        self.state = Some(SilentState {
            open: Some(if key.is_some() {
                sstep::ADDRESS
            } else {
                sstep::KEY
            }),
            done: [key.is_some(), false, false, false],
            scroll: flow::Scroll::default(),
            key,
            label: 0,
            labels,
            wallet,
            check: None,
            back,
            shown: Default::default(),
        });
    }

    /// Opens the flow as the page of session wallet `i`, a silent
    /// payments wallet: its address shows whether or not its key is
    /// loaded; its labels, its scan key and a check need the key. Returns
    /// whether it opened.
    pub(crate) fn open_wallet(&mut self, session: &Session, i: usize) -> bool {
        let Some(r) = session.wallets.get(i).and_then(|w| w.policy().silent()) else {
            return false;
        };
        self.state = Some(SilentState {
            open: Some(sstep::ADDRESS),
            done: [true, false, false, false],
            scroll: flow::Scroll::default(),
            key: Some(r.fingerprint.0),
            label: 0,
            labels: r.labels,
            wallet: Some(i),
            check: None,
            back: Screen::Wallets,
            shown: Default::default(),
        });
        true
    }

    /// The session's silent payments wallet over key `fp`, if there is
    /// one.
    fn wallet_of(session: &Session, fp: [u8; 4]) -> Option<usize> {
        session
            .wallets
            .iter()
            .position(|w| w.policy().silent().is_some_and(|r| r.fingerprint.0 == fp))
    }

    /// The record of the wallet the flow is the page of.
    pub(crate) fn wallet<'a>(&self, session: &'a Session) -> Option<&'a SilentWallet> {
        let i = self.state.as_ref()?.wallet?;
        session.wallets.get(i)?.policy().silent()
    }

    /// Costs what `derive` costs. The scan and spend keys, derived from the
    /// loaded key: `None` while that key is not loaded, or where it is not
    /// the key the wallet was written from (a fingerprint is four bytes;
    /// the wallet is the two points).
    fn receiver(&self, c: &Compute, session: &Session) -> Option<(Receiver, [u8; 4])> {
        let s = self.state.as_ref()?;
        let place = session.keys.position_of(s.key?)?;
        let r = session.keys.silent_receiver(c, place, 0)?;
        if let Some(w) = self.wallet(session)
            && w.spend != r.spend_public_key()
        {
            return None;
        }
        Some((r, session.keys[place].fingerprint.0))
    }

    /// Costs two key derivations and an address the first time. What the
    /// page shows of its key ([`SilentShown`]), kept on the flow's state by
    /// what it is derived from, by whoever asks first, a frame included: a
    /// lookup after.
    pub(crate) fn shown(&self, c: &Compute, session: &Session) -> SilentShown {
        let Some(s) = self.state.as_ref() else {
            return SilentShown::default();
        };
        let loaded = session
            .keys
            .all()
            .find(|k| Some(k.fingerprint.0) == s.key)
            .map(|k| k.id.0);
        let of = (s.key, s.label, s.wallet, loaded, session.network());
        if let Some((o, shown)) = s.shown.borrow().as_ref()
            && *o == of
        {
            return shown.clone();
        }
        let shown = SilentShown {
            key_here: self.key_here(c, session),
            address: self.address(c, session),
        };
        *s.shown.borrow_mut() = Some((of, shown.clone()));
        shown
    }

    /// Costs two key derivations: whether the wallet's key is loaded, which
    /// labels, the scan key and a check need.
    pub(crate) fn key_here(&self, c: &Compute, session: &Session) -> bool {
        self.receiver(c, session).is_some()
    }

    /// Costs two key derivations and an address, none for the wallet's own:
    /// the address on show, unlabelled or with the chosen label. The
    /// unlabelled one is the wallet's own, with or without its key; a
    /// labelled one needs the scan private key.
    pub(crate) fn address(&self, c: &Compute, session: &Session) -> Option<String> {
        let s = self.state.as_ref()?;
        if s.label == 0
            && let Some(w) = self.wallet(session)
        {
            return Some(w.address());
        }
        let (r, _) = self.receiver(c, session)?;
        r.address(s.label)
    }

    /// Costs what `receiver` costs. The public record a coordinator keeps:
    /// the network, the key's origin, the labels handed out and the
    /// address.
    pub(crate) fn record(&self, c: &Compute, session: &Session) -> Option<String> {
        if let Some(w) = self.wallet(session) {
            return Some(w.to_text());
        }
        let s = self.state.as_ref()?;
        let (r, fp) = self.receiver(c, session)?;
        Some(
            SilentWallet {
                network: r.network(),
                fingerprint: faraday_bip::keys::Fingerprint(fp),
                account: 0,
                scan: r.scan_public_key(),
                spend: r.spend_public_key(),
                labels: s.labels.max(s.label),
            }
            .to_text(),
        )
    }

    /// Costs what `receiver` costs. The scan key, BIP-392's `spscan1q…`,
    /// for `why`, and the key's fingerprint.
    pub(crate) fn scan_key(
        &self,
        c: &Compute,
        session: &Session,
        why: faraday_keys::Reveal,
    ) -> Option<(faraday_keys::Held<Zeroizing<String>>, [u8; 4])> {
        let (r, fp) = self.receiver(c, session)?;
        Some((r.scan_key(why)?, fp))
    }

    /// Costs what `address`, `record` and `of` cost. The QR view of the
    /// address on show (as it is, or as a `bitcoin:` link) or of the
    /// record, with the name and label of its picture.
    pub(crate) fn code(
        &self,
        c: &Compute,
        session: &Session,
        code: crate::Code,
    ) -> Result<QrView, String> {
        let s = self.state.as_ref().ok_or(EN.silent_not_open)?;
        let fp = fp_text(faraday_bip::keys::Fingerprint(
            s.key.ok_or(EN.silent_choose_a_key)?,
        ));
        let mut lines = vec![fill(EN.silent_silent_payment, &[&fp.to_string()])];
        let (title, text, name) = match code {
            crate::Code::Silent(uri) => {
                let a = self.address(c, session).ok_or(EN.silent_no_address)?;
                let mut name = format!("silent-{fp}");
                if s.label > 0 {
                    lines.push(fill(EN.silent_label, &[&s.label.to_string()]));
                    name.push_str(&format!("-label-{}", s.label));
                }
                let text = if uri {
                    name.push_str("-link");
                    faraday_bip::silent::uri(&a)
                        .map(|u| u.as_str().to_string())
                        .unwrap_or(a)
                } else {
                    a
                };
                lines.push(EN.silent_public_receives.to_string());
                (EN.silent_silent_payments_address, text, name)
            }
            _ => {
                let text = self.record(c, session).ok_or(EN.silent_no_record)?;
                lines.push(EN.silent_public_watch.to_string());
                (
                    EN.silent_silent_payments_record,
                    text,
                    format!("silent-{fp}-record"),
                )
            }
        };
        Ok(QrView::of(
            c,
            title,
            QrSource::Text(text),
            QrFormat::Ur,
            crate::QR_PARTS[1],
        )?
        .public(&format!("{name}.png"), lines))
    }

    /// A press that changes only this flow: a card, the key, the label.
    /// Returns the silent payments record a label handed out makes the
    /// session wallet `.0` carry, for the caller to put there.
    pub(crate) fn act(
        &mut self,
        action: crate::Action,
        session: &Session,
    ) -> Option<(usize, SilentWallet)> {
        use crate::Action as A;
        let wallet = self.state.as_ref().and_then(|s| s.wallet);
        let record = self.wallet(session).cloned();
        let s = self.state.as_mut()?;
        match action {
            A::SStep(k) => {
                // A wallet's page opens any card; the first time through,
                // the cards go in order.
                if s.open == Some(k) {
                    s.open = None;
                } else if s.wallet.is_some() || (0..k).all(|i| s.done[usize::from(i)]) {
                    s.open = Some(k);
                    s.scroll.follow = true;
                }
            }
            A::SKey(fp) => {
                s.key = Some(fp);
                s.check = None;
                s.done[usize::from(sstep::KEY)] = true;
                s.open = Some(sstep::ADDRESS);
                s.scroll.follow = true;
            }
            A::SLabel(d) => {
                let next = i64::from(s.label) + i64::from(d);
                s.label = next.clamp(0, i64::from(MAX_LABELS)) as u32;
                s.labels = s.labels.max(s.label);
                // A label shown is a label handed out: the wallet's record
                // lists it from now on.
                if let (Some(i), Some(mut r)) = (wallet, record)
                    && s.labels > r.labels
                {
                    r.labels = s.labels;
                    return Some((i, r));
                }
            }
            A::SNext => {
                if let Some(k) = s.open {
                    s.done[usize::from(k)] = true;
                    if k < sstep::CHECK {
                        s.open = Some(k + 1);
                        s.scroll.follow = true;
                    }
                }
            }
            _ => {}
        }
        None
    }

    /// The session wallet `i` is now the flow's page.
    pub(crate) fn set_wallet(&mut self, i: usize) {
        if let Some(s) = self.state.as_mut() {
            s.wallet = Some(i);
        }
    }

    /// The transaction checked, with the file it came from.
    pub(crate) fn set_check(&mut self, name: String, check: crate::silent::check::Check) {
        if let Some(s) = self.state.as_mut() {
            s.check = Some((name, check));
        }
    }

    /// Costs what `receiver` costs. What a check of `check` comes to, or
    /// `None` while a previous transaction is still missing. An unsigned
    /// input is no answer, and the change label is always looked for.
    pub(crate) fn answer(
        &self,
        c: &Compute,
        session: &Session,
        check: &crate::silent::check::Check,
    ) -> Option<crate::silent::check::Checked> {
        use crate::silent::check::{Checked, Paid};
        let prevouts = check.prevouts()?;
        let tx = check.tx();
        for (i, input) in tx.input.iter().enumerate() {
            if faraday_bip::silent::needs_signature_data(prevouts[i].as_bytes())
                && !faraday_psbt::transaction::is_signed(&input.script_sig, &input.witness)
            {
                return Some(Checked::Unsigned);
            }
        }
        let Some((receiver, _)) = self.receiver(c, session) else {
            return Some(Checked::KeyNotLoaded);
        };
        let labels_out = self
            .wallet(session)
            .map_or(0, |w| w.labels)
            .max(self.state.as_ref().map_or(0, |s| s.labels));
        let labels: Vec<u32> = (0..=labels_out).collect();
        let scripts: Vec<&[u8]> = prevouts.iter().map(|s| s.as_bytes()).collect();
        match receiver.payments_in(tx, &scripts, &labels) {
            Err(_) => Some(Checked::NoInputs),
            Ok(found) if found.is_empty() => Some(Checked::NotPaid),
            Ok(found) => Some(Checked::Paid(
                found
                    .into_iter()
                    .map(|p| Paid {
                        vout: p.vout,
                        amount: p.amount,
                        label: p.label,
                    })
                    .collect(),
            )),
        }
    }
}

impl Faraday {
    /// The flow's state, while it is open.
    pub fn silent(&self) -> Option<&SilentState> {
        self.silent.state()
    }

    /// Opens the flow.
    pub(crate) fn silent_open(&mut self) {
        if self.session.keys.is_empty() {
            self.toast(EN.silent_load_a_key_first);
            return;
        }
        self.silent.open(&self.session, self.screen);
        self.screen = Screen::Silent;
    }

    /// The record of the wallet the flow is the page of.
    pub fn silent_wallet(&self) -> Option<&SilentWallet> {
        self.silent.wallet(&self.session)
    }

    /// Costs two key derivations and an address the first time. What the
    /// page shows of its key ([`SilentShown`]), kept on the flow's state.
    pub(crate) fn silent_shown(&self, c: &Compute) -> SilentShown {
        self.silent.shown(c, &self.session)
    }

    /// Costs two key derivations: whether the wallet's key is loaded.
    pub fn silent_key_here(&self, c: &Compute) -> bool {
        self.silent.key_here(c, &self.session)
    }

    /// Costs two key derivations and an address, none for the wallet's own:
    /// the address on show.
    pub fn silent_address(&self, c: &Compute) -> Option<String> {
        self.silent.address(c, &self.session)
    }

    /// Costs two key derivations. The public record a coordinator keeps.
    pub fn silent_record(&self, c: &Compute) -> Option<String> {
        self.silent.record(c, &self.session)
    }

    /// Costs what `silent_address`, `silent_record` and `of` cost. The QR
    /// view of the address on show or of the record.
    pub(crate) fn silent_code(&self, c: &Compute, code: crate::Code) -> Result<QrView, String> {
        self.silent.code(c, &self.session, code)
    }

    /// One press inside the flow.
    pub(crate) fn silent_act(&mut self, action: crate::Action) {
        let c = self.compute();
        use crate::Action as A;
        match action {
            A::Silent => self.silent_open(),
            A::SQr(uri) => {
                if self.silent_address(&c).is_some() {
                    let view = self.silent_code(&c, crate::Code::Silent(uri));
                    self.open_qr(view);
                }
            }
            A::SRecord => {
                let fp = self.silent.state().and_then(|s| s.key);
                if let (Some(text), Some(fp)) = (self.silent_record(&c), fp) {
                    let name =
                        format!("silent-{}.txt", fp_text(faraday_bip::keys::Fingerprint(fp)));
                    self.put_outbox(&name, text.into_bytes());
                    self.toast_out(&name);
                }
            }
            A::SScanVault => {
                if self.vaults.open.get(self.vaults.current).is_none() {
                    return self.toast(EN.silent_no_vault_open);
                }
                let Some((key, fp)) = self.silent.scan_key(
                    &c,
                    &self.session,
                    faraday_keys::Reveal::VaultEntry(faraday_keys::Part::ScanKey),
                ) else {
                    return;
                };
                let title = fill(
                    EN.silent_silent_payments_scan,
                    &[&fp_text(faraday_bip::keys::Fingerprint(fp)).to_string()],
                );
                let note = Zeroizing::new(format!("{title}\n{}", key.as_str()));
                self.vault_push(
                    Record::new(kind::NOTE).with(field::NOTE, note.as_bytes()),
                    &fill(EN.silent_is_in_the_vault, &[&title.to_string()]),
                );
            }
            A::SScanOut => {
                let Some((key, fp)) = self.silent.scan_key(
                    &c,
                    &self.session,
                    faraday_keys::Reveal::OutboxFile(faraday_keys::Part::ScanKey),
                ) else {
                    return;
                };
                let name = format!("spscan-{}.txt", fp_text(faraday_bip::keys::Fingerprint(fp)));
                self.offer_secret(crate::secrets::SecretOut::new(
                    name,
                    Zeroizing::new(format!("{}\n", key.as_str()).into_bytes()),
                    EN.silent_a_silent_payments,
                    EN.silent_whoever_has,
                ));
            }
            A::SAddWallet => self.silent_add_wallet(),
            A::SWallet(i) => {
                if self.silent.open_wallet(&self.session, i) {
                    self.screen = Screen::Silent;
                }
            }
            A::SCheck(k) => self.silent_check(k),
            _ => {
                if let Some((i, r)) = self.silent.act(action, &self.session) {
                    self.session.wallets[i]
                        .set_policy(faraday_bip::policy::WalletPolicy::of_silent(r));
                }
            }
        }
    }

    /// The key's silent payments wallet into the session's wallets: the
    /// two public keys, the key's origin and the labels handed out.
    fn silent_add_wallet(&mut self) {
        let c = self.compute();
        let Some(text) = self.silent_record(&c) else {
            return self.toast(EN.silent_load_the_key);
        };
        let Some(fp) = self.silent.state().and_then(|s| s.key) else {
            return;
        };
        let name = fill(
            EN.silent_silent_payments_n,
            &[&fp_text(faraday_bip::keys::Fingerprint(fp)).to_string()],
        );
        match self.session.add_wallet(
            &c,
            &name,
            &text,
            EN.silent_silent_payments,
            crate::wallet::Origin::Named,
        ) {
            Ok(i) => {
                self.silent.set_wallet(i);
                self.toast(EN.silent_added_to_your);
            }
            Err(e) => self.toast(&e.text()),
        }
    }

    /// Reads Inbox file `k` as the transaction to check, takes every
    /// previous transaction Files has for it, and works the answer out
    /// once every previous output is known.
    fn silent_check(&mut self, k: usize) {
        let c = self.compute();
        let Some(item) = self.inbox.get(k) else {
            return;
        };
        let Some(mut check) = crate::silent::check::Check::read(&item.bytes) else {
            return self.toast(EN.silent_not_a_transaction);
        };
        let name = item.name.clone();
        // Each pass takes a transaction Files has for an input still
        // open; a pass that takes nothing ends it.
        while check.waiting_for().is_some() {
            let took = self.inbox.iter().any(|it| check.add_previous(&it.bytes));
            if !took {
                break;
            }
        }
        if let Some(answer) = self.silent.answer(&c, &self.session, &check) {
            check.set_result(answer);
        }
        self.silent.set_check(name, check);
    }
}
