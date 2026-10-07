//! Silent payments (BIP-352), the receiving side: a loaded key's
//! `sp1…` address and its labelled ones, the public record a coordinator
//! keeps, and the scan key a scanner needs (`docs/OPENSIGNER-PARITY.md`
//! plan C2). Every computation is `osk-bip::silent`'s and
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
    /// How many cards.
    pub const COUNT: usize = 3;
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
            done: [key.is_some(), false, false],
            scroll: flow::Scroll::default(),
            key,
            label: 0,
            labels: 0,
            back: self.screen,
        });
        self.screen = Screen::Silent;
    }

    fn silent_receiver(&self) -> Option<(Receiver, [u8; 4])> {
        let s = self.silent.as_ref()?;
        let key = self
            .session
            .keys
            .iter()
            .find(|k| Some(k.master.fingerprint().0) == s.key)?;
        Some((Receiver::derive(&key.master, 0), key.master.fingerprint().0))
    }

    /// The address on show: unlabelled, or with the chosen label.
    pub fn silent_address(&self) -> Option<String> {
        let s = self.silent.as_ref()?;
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
                if let (Some(text), Some((_, fp))) = (self.silent_record(), self.silent_receiver())
                {
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
            _ => {}
        }
        let Some(s) = self.silent.as_mut() else {
            return;
        };
        match action {
            A::SStep(k) => {
                if s.open == Some(k) {
                    s.open = None;
                } else if (0..k).all(|i| s.done[usize::from(i)]) {
                    s.open = Some(k);
                    s.scroll.follow = true;
                }
            }
            A::SKey(fp) => {
                s.key = Some(fp);
                s.done[usize::from(sstep::KEY)] = true;
                s.open = Some(sstep::ADDRESS);
                s.scroll.follow = true;
            }
            A::SLabel(d) => {
                let next = i64::from(s.label) + i64::from(d);
                s.label = next.clamp(0, i64::from(MAX_LABELS)) as u32;
                s.labels = s.labels.max(s.label);
            }
            A::SNext => {
                if let Some(k) = s.open {
                    s.done[usize::from(k)] = true;
                    if k < sstep::SCAN {
                        s.open = Some(k + 1);
                        s.scroll.follow = true;
                    }
                }
            }
            _ => {}
        }
    }
}
