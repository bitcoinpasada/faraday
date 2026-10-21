//! What the last stick visit wrote, as a receipt.

use crate::strings::{EN, fill};
use crate::vault;
use faraday_crypto::work::Compute;

/// What the last stick visit wrote (`docs/history/SIMPLIFY.md` §4.3): kept on
/// [`Faraday`], across a lock in the kept state, gone at power-off, and
/// replaced by the next write. Nothing on it is written again.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Receipt {
    /// The stick's label.
    pub label: String,
    /// The stick's identity: its FAT volume serial in hex, else the place
    /// the shell found it (`docs/VAULT.md` §6, `docs/APK-SIGNING.md` §3).
    pub stick: String,
    /// When the write was answered, Unix seconds, when the clock is known.
    pub time: Option<u64>,
    /// One row per file sent.
    pub files: Vec<ReceiptFile>,
    /// The backup flow wrote it. Its bytes are the flow's sealings, which
    /// no ordinary visit writes over a copy of the same vault.
    pub by_flow: bool,
}

/// One file on a [`Receipt`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptFile {
    /// Its name.
    pub name: String,
    /// SHA-256 of the bytes written.
    pub hash: [u8; 32],
    /// Written, read back and matched.
    pub verified: bool,
    /// Why it was not, when it was not.
    pub failure: Option<String>,
}

impl Receipt {
    /// "Written to STICK at 14:02", or without the time when the clock
    /// is not known. The time is local (`docs/TOTP.md` §2.3).
    pub fn heading(&self, tz_min: i16) -> String {
        match self.time {
            Some(t) => fill(
                EN.heading_written_to_at,
                &[
                    &self.label.to_string(),
                    &vault::time_of_day(vault::local_secs(t, tz_min)).to_string(),
                ],
            ),
            None => fill(EN.heading_written_to, &[&self.label.to_string()]),
        }
    }

    /// The file written and matched under `name` with bytes of this
    /// SHA-256.
    pub fn holds(&self, name: &str, hash: &[u8; 32]) -> bool {
        self.files
            .iter()
            .any(|f| f.verified && f.name == name && f.hash == *hash)
    }

    /// Whether a file with these bytes was written and matched.
    pub fn holds_hash(&self, hash: &[u8; 32]) -> bool {
        self.files.iter().any(|f| f.verified && f.hash == *hash)
    }

    /// Whether `name` was written and matched.
    pub fn wrote(&self, name: &str) -> bool {
        self.files.iter().any(|f| f.verified && f.name == name)
    }

    /// As the kept state holds it: a `receipt` line, then a `file` line
    /// per file, fields split by tabs.
    pub(crate) fn encode(&self) -> Vec<u8> {
        let field = |s: &str| s.replace(['\t', '\n', '\r'], " ");
        let mut out = format!(
            "receipt\t{}\t{}\t{}\t{}\n",
            field(&self.label),
            self.time.map(|t| t.to_string()).unwrap_or_default(),
            field(&self.stick),
            u8::from(self.by_flow)
        );
        for f in &self.files {
            let hash: String = f.hash.iter().map(|b| format!("{b:02x}")).collect();
            out.push_str(&format!(
                "file\t{}\t{hash}\t{}\t{}\n",
                field(&f.name),
                u8::from(f.verified),
                field(f.failure.as_deref().unwrap_or(""))
            ));
        }
        out.into_bytes()
    }

    /// Read back; `None` when it does not read.
    pub(crate) fn decode(bytes: &[u8]) -> Option<Receipt> {
        let text = String::from_utf8_lossy(bytes);
        let mut out: Option<Receipt> = None;
        for line in text.lines() {
            let f: Vec<&str> = line.split('\t').collect();
            match f[..] {
                ["receipt", label, time]
                | ["receipt", label, time, _]
                | ["receipt", label, time, _, _] => {
                    out = Some(Receipt {
                        label: label.to_string(),
                        stick: f.get(3).map(|s| s.to_string()).unwrap_or_default(),
                        time: time.parse().ok(),
                        files: Vec::new(),
                        by_flow: f.get(4) == Some(&"1"),
                    });
                }
                ["file", name, hash, verified, failure] => {
                    let bytes: Option<Vec<u8>> = (0..32)
                        .map(|i| {
                            hash.get(i * 2..i * 2 + 2)
                                .and_then(|h| u8::from_str_radix(h, 16).ok())
                        })
                        .collect();
                    let Some(hash) = bytes.and_then(|b| <[u8; 32]>::try_from(b).ok()) else {
                        continue;
                    };
                    if let Some(r) = out.as_mut() {
                        r.files.push(ReceiptFile {
                            name: name.to_string(),
                            hash,
                            verified: verified == "1",
                            failure: (!failure.is_empty()).then(|| failure.to_string()),
                        });
                    }
                }
                _ => {}
            }
        }
        out
    }
}

/// Costs a SHA-256 pass over every byte: the SHA-256 of `bytes`, a
/// whole file's or an id's, by `faraday_crypto::sha256_whole`.
pub(crate) fn sha256_of(c: &Compute, bytes: &[u8]) -> [u8; 32] {
    faraday_crypto::sha256_whole(c, bytes)
}
