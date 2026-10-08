//! Silent payments (BIP-352), the receiving side: a loaded key's
//! `sp1…` address and its labelled ones, the public record a coordinator
//! keeps, the scan key a scanner needs, and a silent payments wallet among
//! the session's wallets, with a check of whether a transaction pays it
//! (`docs/OPENSIGNER-PARITY.md` plan C2, upstream §16.113). Every computation is `osk-bip::silent`'s and
//! `silent_wallet`'s, as OpenSigner's own wallet uses them. The address
//! and the record are public; the scan key shows every payment to the
//! address, so it is a secret: into the vault, or out only through the
//! secret sheet. Nothing here is kept; the keys are derived when used.

use faraday_vault::Record;
use faraday_vault::records::{field, kind};
use osk_bip::silent::Receiver;
use osk_bip::silent_wallet::{MAX_LABELS, SilentWallet};
use zeroize::Zeroizing;

use crate::wallet::fp_text;
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
    pub check: Option<(String, opensigner_core::silent::Check)>,
    /// Where the flow returns to.
    pub back: Screen,
}

impl Faraday {
    /// Opens the flow.
    pub(crate) fn silent_open(&mut self) {
        if self.session.keys.is_empty() {
            self.toast("Load a key first");
            return;
        }
        let key =
            (self.session.keys.len() == 1).then(|| self.session.keys[0].master.fingerprint().0);
        self.silent = Some(SilentState {
            open: Some(if key.is_some() {
                sstep::ADDRESS
            } else {
                sstep::KEY
            }),
            done: [key.is_some(), false, false, false],
            scroll: flow::Scroll::default(),
            key,
            label: 0,
            labels: 0,
            wallet: key.and_then(|fp| self.silent_wallet_of(fp)),
            check: None,
            back: self.screen,
        });
        if let Some(i) = self.silent.as_ref().and_then(|s| s.wallet)
            && let Some(r) = self.session.wallets[i].policy.silent()
            && let Some(s) = self.silent.as_mut()
        {
            s.labels = r.labels;
        }
        self.screen = Screen::Silent;
    }

    /// Opens the flow as the page of session wallet `i`, a silent
    /// payments wallet: its address shows whether or not its key is
    /// loaded; its labels, its scan key and a check need the key.
    pub(crate) fn silent_open_wallet(&mut self, i: usize) {
        let Some(r) = self.session.wallets.get(i).and_then(|w| w.policy.silent()) else {
            return;
        };
        self.silent = Some(SilentState {
            open: Some(sstep::ADDRESS),
            done: [true, false, false, false],
            scroll: flow::Scroll::default(),
            key: Some(r.fingerprint.0),
            label: 0,
            labels: r.labels,
            wallet: Some(i),
            check: None,
            back: Screen::Wallets,
        });
        self.screen = Screen::Silent;
    }

    /// The session's silent payments wallet over key `fp`, if there is
    /// one.
    fn silent_wallet_of(&self, fp: [u8; 4]) -> Option<usize> {
        self.session
            .wallets
            .iter()
            .position(|w| w.policy.silent().is_some_and(|r| r.fingerprint.0 == fp))
    }

    /// The record of the wallet the flow is the page of.
    pub fn silent_wallet(&self) -> Option<&SilentWallet> {
        let i = self.silent.as_ref()?.wallet?;
        self.session.wallets.get(i)?.policy.silent()
    }

    /// The scan and spend keys, derived from the loaded key: `None`
    /// while that key is not loaded, or where it is not the key the
    /// wallet was written from (a fingerprint is four bytes; the wallet
    /// is the two points).
    fn silent_receiver(&self) -> Option<(Receiver, [u8; 4])> {
        let s = self.silent.as_ref()?;
        let key = self
            .session
            .keys
            .iter()
            .find(|k| Some(k.master.fingerprint().0) == s.key)?;
        let r = Receiver::derive(&key.master, 0);
        if let Some(w) = self.silent_wallet()
            && w.spend != r.spend_public_key()
        {
            return None;
        }
        Some((r, key.master.fingerprint().0))
    }

    /// Whether the wallet's key is loaded, which labels, the scan key and
    /// a check need.
    pub fn silent_key_here(&self) -> bool {
        self.silent_receiver().is_some()
    }

    /// The address on show: unlabelled, or with the chosen label. The
    /// unlabelled one is the wallet's own, with or without its key; a
    /// labelled one needs the scan private key.
    pub fn silent_address(&self) -> Option<String> {
        let s = self.silent.as_ref()?;
        if s.label == 0
            && let Some(w) = self.silent_wallet()
        {
            return Some(w.address());
        }
        let (r, _) = self.silent_receiver()?;
        let secp = osk_bip::bitcoin::secp256k1::Secp256k1::new();
        let a = if s.label == 0 {
            r.address(&secp)
        } else {
            r.labelled_address(&secp, s.label)
        };
        a.ok().map(|t| t.as_str().to_string())
    }

    /// The public record a coordinator keeps: the network, the key's
    /// origin, the labels handed out and the address.
    pub fn silent_record(&self) -> Option<String> {
        if let Some(w) = self.silent_wallet() {
            return Some(w.to_text());
        }
        let s = self.silent.as_ref()?;
        let (r, fp) = self.silent_receiver()?;
        let secp = osk_bip::bitcoin::secp256k1::Secp256k1::new();
        Some(
            SilentWallet {
                network: r.network(),
                fingerprint: osk_bip::keys::Fingerprint(fp),
                account: 0,
                scan: r.scan_public_key(&secp),
                spend: r.spend_public_key(),
                labels: s.labels.max(s.label),
            }
            .to_text(),
        )
    }

    /// The scan key, BIP-392's `spscan1q…`.
    fn silent_scan_key(&self) -> Option<Zeroizing<String>> {
        let (r, _) = self.silent_receiver()?;
        let k = r.scan_key_text().ok()?;
        Some(Zeroizing::new(k.as_str().to_string()))
    }

    /// One press inside the flow.
    pub(crate) fn silent_act(&mut self, action: crate::Action) {
        use crate::Action as A;
        match action {
            A::Silent => return self.silent_open(),
            A::SQr(uri) => {
                if let Some(a) = self.silent_address() {
                    let text = if uri {
                        osk_bip::silent::uri(&a)
                            .map(|u| u.as_str().to_string())
                            .unwrap_or(a)
                    } else {
                        a
                    };
                    self.open_qr(QrView::of(
                        "Silent payments address",
                        QrSource::Text(text),
                        QrFormat::Ur,
                        crate::QR_PARTS[1],
                    ));
                }
                return;
            }
            A::SRecord => {
                let fp = self.silent.as_ref().and_then(|s| s.key);
                if let (Some(text), Some(fp)) = (self.silent_record(), fp) {
                    let name = format!("silent-{}.txt", fp_text(osk_bip::keys::Fingerprint(fp)));
                    self.put_outbox(&name, text.into_bytes());
                    self.toast(&format!("{name} is in the Outbox"));
                }
                return;
            }
            A::SScanVault => {
                if self.vaults.open.get(self.vaults.current).is_none() {
                    return self.toast("No vault is open");
                }
                let (Some(key), Some((_, fp))) = (self.silent_scan_key(), self.silent_receiver())
                else {
                    return;
                };
                let title = format!(
                    "Silent payments scan key · {}",
                    fp_text(osk_bip::keys::Fingerprint(fp))
                );
                let note = Zeroizing::new(format!("{title}\n{}", key.as_str()));
                self.vault_push(
                    Record::new(kind::NOTE).with(field::NOTE, note.as_bytes()),
                    &format!("{title} is in the vault"),
                );
                return;
            }
            A::SScanOut => {
                let (Some(key), Some((_, fp))) = (self.silent_scan_key(), self.silent_receiver())
                else {
                    return;
                };
                let name = format!("spscan-{}.txt", fp_text(osk_bip::keys::Fingerprint(fp)));
                self.offer_secret(crate::secrets::SecretOut {
                    name,
                    bytes: Zeroizing::new(format!("{}\n", key.as_str()).into_bytes()),
                    what: "A silent payments scan key",
                    gives: "Whoever has it sees every payment to this address; it cannot spend them",
                    round: None,
                });
                return;
            }
            A::SAddWallet => return self.silent_add_wallet(),
            A::SWallet(i) => return self.silent_open_wallet(i),
            A::SCheck(k) => return self.silent_check(k),
            _ => {}
        }
        let wallet = self.silent.as_ref().and_then(|s| s.wallet);
        let record = self.silent_wallet().cloned();
        let Some(s) = self.silent.as_mut() else {
            return;
        };
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
                    self.session.wallets[i].policy = osk_bip::policy::WalletPolicy::of_silent(r);
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
    }

    /// The key's silent payments wallet into the session's wallets: the
    /// two public keys, the key's origin and the labels handed out.
    fn silent_add_wallet(&mut self) {
        let Some(text) = self.silent_record() else {
            return self.toast("Load the key first");
        };
        let Some(fp) = self.silent.as_ref().and_then(|s| s.key) else {
            return;
        };
        let name = format!(
            "Silent payments {}",
            fp_text(osk_bip::keys::Fingerprint(fp))
        );
        match self.session.add_wallet(&name, &text, "Silent payments") {
            Ok(i) => {
                if let Some(s) = self.silent.as_mut() {
                    s.wallet = Some(i);
                }
                self.toast("Added to your wallets");
            }
            Err(e) => self.toast(&e.text()),
        }
    }

    /// Reads Inbox file `k` as the transaction to check, takes every
    /// previous transaction Files has for it, and works the answer out
    /// once every previous output is known.
    fn silent_check(&mut self, k: usize) {
        let Some(item) = self.inbox.get(k) else {
            return;
        };
        let Some(mut check) = opensigner_core::silent::Check::read(&item.bytes) else {
            return self.toast("Not a transaction");
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
        if let Some(answer) = self.silent_answer(&check) {
            check.set_result(answer);
        }
        if let Some(s) = self.silent.as_mut() {
            s.check = Some((name, check));
        }
    }

    /// What a check of `check` comes to, or `None` while a previous
    /// transaction is still missing. As OpenSigner works it out: an
    /// unsigned input is no answer, and the change label is always
    /// looked for.
    fn silent_answer(
        &self,
        check: &opensigner_core::silent::Check,
    ) -> Option<opensigner_core::silent::Checked> {
        use opensigner_core::silent::{Checked, Paid};
        let prevouts = check.prevouts()?;
        let tx = check.tx();
        for (i, input) in tx.input.iter().enumerate() {
            if osk_bip::silent::needs_signature_data(prevouts[i].as_bytes())
                && !osk_psbt::transaction::is_signed(&input.script_sig, &input.witness)
            {
                return Some(Checked::Unsigned);
            }
        }
        let Some((receiver, _)) = self.silent_receiver() else {
            return Some(Checked::KeyNotLoaded);
        };
        let labels_out = self
            .silent_wallet()
            .map_or(0, |w| w.labels)
            .max(self.silent.as_ref().map_or(0, |s| s.labels));
        let labels: Vec<u32> = (0..=labels_out).collect();
        let scripts: Vec<&[u8]> = prevouts.iter().map(|s| s.as_bytes()).collect();
        let secp = osk_bip::bitcoin::secp256k1::Secp256k1::new();
        match osk_bip::silent::find_payments(&secp, &receiver, tx, &scripts, &labels) {
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
