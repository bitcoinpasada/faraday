//! What the Inbox holds, put together: every wallet its files describe,
//! once each however many forms it came in, every seed its backups give,
//! once each, and which seeds belong to which wallets.
//!
//! A wallet description comes as a descriptor, a wallet .json, a BIP 129
//! record, a multisig config, Bitcoin Core's import file, or split sheets
//! that add up. A seed comes as words in a text file (a SeedQR or
//! CompactSeedQR picture read at a stick visit arrives as one), as SLIP-39
//! shares or codex32 strings that add up, or out of an OpenSigner backup
//! opened with its passphrase. A seed whose fingerprint names a key of a
//! wallet here loads with that wallet; a seed no wallet names is a
//! potential wallet: the person adds a BIP-39 passphrase or does not, and
//! chooses the kind of single-key wallet, before it becomes one.
//!
//! Seed XOR parts are BIP-39 words on their own, so each reads as a seed
//! here; they are put together in Add a key.

use crate::strings::EN;
use opensigner_core::strings::fill;
use std::collections::BTreeSet;

use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::bitcoin::hashes::{Hash, sha256};
use osk_bip::codex32::Codex32;
use osk_bip::keys::{Fingerprint, MasterKey};
use osk_bip::slip39;
use zeroize::Zeroizing;

use crate::create::NewKind;
use crate::wallet::{FileKind, Refusal, Session, Wallet, fp_text};
use crate::{Faraday, Item, forms, stem};

/// Where a seed found in the Inbox comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeedSource {
    /// A text file of BIP-39 words.
    Words(usize),
    /// A text file of BIP-39 words, with its BIP-39 passphrase in a file
    /// of its own, `passphrase-{fingerprint}.txt`, the fingerprint the
    /// words make with it (`docs/BACKUP-FLOW.md` §19.12 items 5 and 6).
    WordsWith(usize, usize),
    /// SLIP-39 shares that add up, by Inbox place.
    Slip39(Vec<usize>),
    /// A codex32 secret, or shares of it that add up.
    Codex32(Vec<usize>),
    /// Already loaded in this session.
    Loaded,
}

impl SeedSource {
    /// What the source is called on screen.
    pub fn name(&self, app: &Faraday) -> String {
        match self {
            SeedSource::Words(k) | SeedSource::WordsWith(k, _) => {
                app.inbox.get(*k).map_or(String::new(), |i| i.name.clone())
            }
            SeedSource::Slip39(ks) => fill(EN.inbox_slip_39_shares_2, &[&ks.len().to_string()]),
            SeedSource::Codex32(ks) if ks.len() == 1 => "codex32".to_string(),
            SeedSource::Codex32(ks) => fill(EN.inbox_codex32_shares, &[&ks.len().to_string()]),
            SeedSource::Loaded => EN.inbox_loaded.to_string(),
        }
    }

    /// Whether a passphrase can go with it: BIP-39 words take a BIP-39
    /// passphrase and SLIP-39 shares their own; a codex32 seed takes none.
    pub fn takes_passphrase(&self) -> bool {
        !matches!(self, SeedSource::Codex32(_))
    }
}

/// A seed the Inbox gives, or one already loaded.
#[derive(Debug, Clone)]
pub struct FoundSeed {
    /// Its master fingerprint, with no passphrase.
    pub fingerprint: Fingerprint,
    /// A label: the file's name without its extension, or the loaded
    /// key's label.
    pub label: String,
    /// Where it comes from: the first is the one used.
    pub sources: Vec<SeedSource>,
    /// The session has it already.
    pub loaded: bool,
}

/// A wallet the Inbox describes.
#[derive(Debug, Clone)]
pub struct FoundWallet {
    /// What it will be called.
    pub name: String,
    /// The descriptor, as text: one wallet however many files say it.
    pub descriptor: String,
    /// The files that describe it, by Inbox place.
    pub files: Vec<usize>,
    /// Rebuilt from split sheets rather than read from one file.
    pub from_shares: bool,
    /// Its keys' fingerprints.
    pub keys: Vec<Fingerprint>,
    /// Its shape: single key, or m of n and the script.
    pub shape: String,
    /// Signatures it needs.
    pub needed: usize,
    /// Of its keys, how many a seed here or loaded gives.
    pub seeds_here: usize,
    /// The session has it already.
    pub loaded: bool,
}

/// A backup still short of parts, or sealed.
#[derive(Debug, Clone)]
pub struct Waiting {
    /// What it is and what it lacks.
    pub line: String,
    /// The OpenSigner backup to open, by Inbox place.
    pub backup: Option<usize>,
}

/// Everything the Inbox gives, put together.
#[derive(Debug, Clone, Default)]
pub struct Found {
    /// Wallets, once each.
    pub wallets: Vec<FoundWallet>,
    /// Seeds, once each.
    pub seeds: Vec<FoundSeed>,
    /// Backups short of parts, and sealed backups.
    pub waiting: Vec<Waiting>,
    /// Account xpubs no wallet here uses and no seed here gives: each a
    /// potential watch-only wallet.
    pub xpubs: Vec<FoundXpub>,
}

/// An account xpub in the Inbox that no wallet here uses.
#[derive(Debug, Clone)]
pub struct FoundXpub {
    /// The key expression, `[fingerprint/path]xpub`.
    pub key: String,
    /// Its master fingerprint, when it has an origin.
    pub fingerprint: Option<Fingerprint>,
    /// Its path from the master, as text, when it has an origin.
    pub path: Option<String>,
    /// The single-key kind its path says (BIP-44, 49, 84 or 86), or
    /// `None` when it has no origin and the person chooses.
    pub kind: Option<NewKind>,
    /// The files it is in, by Inbox place.
    pub files: Vec<usize>,
}

/// The single-key kind an account path's purpose says; `Err` for a
/// multisig account (BIP-45, 48, 87), which is a cosigner's key and not a
/// wallet on its own.
fn kind_of_path(path: &osk_bip::bitcoin::bip32::DerivationPath) -> Result<Option<NewKind>, ()> {
    let purpose = path.into_iter().next().map(|c| match c {
        osk_bip::bitcoin::bip32::ChildNumber::Hardened { index }
        | osk_bip::bitcoin::bip32::ChildNumber::Normal { index } => *index,
    });
    match purpose {
        Some(44) => Ok(Some(NewKind::Legacy)),
        Some(49) => Ok(Some(NewKind::NestedSegwit)),
        Some(84) => Ok(Some(NewKind::NativeSegwit)),
        Some(86) => Ok(Some(NewKind::Taproot)),
        Some(45 | 48 | 87) => Err(()),
        _ => Ok(None),
    }
}

impl Found {
    /// Seeds no wallet here names: potential wallets.
    pub fn potential(&self) -> Vec<&FoundSeed> {
        self.seeds
            .iter()
            .filter(|s| !self.wallets.iter().any(|w| w.keys.contains(&s.fingerprint)))
            .collect()
    }

    /// Wallets not loaded yet.
    pub fn to_load(&self) -> Vec<&FoundWallet> {
        self.wallets.iter().filter(|w| !w.loaded).collect()
    }
}

/// A seed as bytes to derive from, with what the person may add.
pub(crate) enum Secret {
    /// BIP-39 words.
    Words(Mnemonic),
    /// SLIP-39 shares.
    Slip39(Vec<slip39::Share>),
    /// A master seed.
    Seed(Zeroizing<Vec<u8>>),
}

impl Secret {
    /// The master key, with `passphrase` where the form takes one.
    pub(crate) fn master(&self, passphrase: &str, session: &Session) -> Option<MasterKey> {
        crate::work::count(crate::work::Work::MasterKey);
        let network = session.network();
        match self {
            Secret::Words(m) => {
                let seed = m.to_seed(passphrase.as_bytes()).ok()?;
                Some(MasterKey::from_seed(&seed, network))
            }
            Secret::Slip39(shares) => {
                let secret = slip39::recover(shares, passphrase.as_bytes()).ok()?;
                let bytes = osk_crypto::SeedBytes::new(secret.expose().as_bytes())?;
                Some(MasterKey::from_seed_bytes(
                    &osk_crypto::Secret::new(bytes),
                    network,
                ))
            }
            Secret::Seed(s) => {
                let bytes = osk_crypto::SeedBytes::new(s)?;
                Some(MasterKey::from_seed_bytes(
                    &osk_crypto::Secret::new(bytes),
                    network,
                ))
            }
        }
    }

    /// Adds the key to the session, with `passphrase` where the form takes
    /// one.
    pub(crate) fn add(
        &self,
        passphrase: &str,
        label: &str,
        session: &mut Session,
    ) -> Result<Fingerprint, Refusal> {
        match self {
            Secret::Words(m) => session.add_words_with(&phrase(m), passphrase, label, None),
            Secret::Slip39(shares) => {
                let secret = slip39::recover(shares, passphrase.as_bytes())
                    .map_err(|e| Refusal::Words(format!("{e:?}")))?;
                session.add_seed(secret.expose().as_bytes(), label)
            }
            Secret::Seed(s) => session.add_seed(s, label),
        }
    }
}

/// A mnemonic's words, one space between each.
fn phrase(m: &Mnemonic) -> Zeroizing<String> {
    let lang = m.language();
    Zeroizing::new(
        m.indices()
            .iter()
            .map(|&i| lang.word(i))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// The words a text file spells, in whichever BIP-39 list they are in.
fn words_of(text: &str) -> Option<Mnemonic> {
    let typed = forms::file_words(text);
    Language::ALL
        .into_iter()
        .find_map(|lang| crate::wallet::parse_words(lang, &typed).ok())
}

pub(crate) use faraday_intake::{Part, part_of};

impl Faraday {
    /// The fingerprint of a seed with no passphrase, remembered by a hash
    /// of the secret so a frame does not run PBKDF2 again.
    fn fingerprint_memo(
        &self,
        tag: &[u8],
        secret: &[u8],
        f: impl FnOnce() -> Option<Fingerprint>,
    ) -> Option<Fingerprint> {
        let mut e = sha256::Hash::engine();
        osk_bip::bitcoin::hashes::HashEngine::input(&mut e, tag);
        osk_bip::bitcoin::hashes::HashEngine::input(&mut e, secret);
        let id = sha256::Hash::from_engine(e).to_byte_array();
        if let Some((_, fp)) = self.fp_memo.borrow().iter().find(|(k, _)| *k == id) {
            return Some(*fp);
        }
        let fp = f()?;
        self.fp_memo.borrow_mut().push((id, fp));
        Some(fp)
    }

    /// The secret a seed source gives.
    pub(crate) fn inbox_secret(&self, source: &SeedSource) -> Option<Secret> {
        self.secret_in(&self.inbox, source)
    }

    /// The secret a seed source among `items` gives.
    pub(crate) fn secret_in(&self, items: &[Item], source: &SeedSource) -> Option<Secret> {
        let text = |k: &usize| {
            items
                .get(*k)
                .map(|i| Zeroizing::new(String::from_utf8_lossy(&i.bytes).into_owned()))
        };
        match source {
            SeedSource::Words(k) | SeedSource::WordsWith(k, _) => {
                words_of(&text(k)?).map(Secret::Words)
            }
            SeedSource::Slip39(ks) => {
                let shares: Vec<slip39::Share> = ks
                    .iter()
                    .filter_map(|k| match part_of(&text(k)?) {
                        Some(Part::Slip39(s)) => Some(s),
                        _ => None,
                    })
                    .collect();
                Some(Secret::Slip39(shares))
            }
            SeedSource::Codex32(ks) => {
                let parts: Vec<Codex32> = ks
                    .iter()
                    .filter_map(|k| match part_of(&text(k)?) {
                        Some(Part::Codex32(c)) => Some(c),
                        _ => None,
                    })
                    .collect();
                let seed = match parts.iter().find(|c| c.is_secret()) {
                    Some(s) => s.to_seed(),
                    None => Codex32::recover(&parts).and_then(|s| s.to_seed()),
                }
                .ok()?;
                Some(Secret::Seed(Zeroizing::new(seed.expose().bytes().to_vec())))
            }
            SeedSource::Loaded => None,
        }
    }

    /// The fingerprint a source's seed among `items` has with no
    /// passphrase.
    fn source_fingerprint(&self, items: &[Item], source: &SeedSource) -> Option<Fingerprint> {
        let tag: &[u8] = match source {
            SeedSource::Words(_) => b"words",
            SeedSource::WordsWith(..) => b"words+passphrase",
            SeedSource::Slip39(_) => b"slip39",
            SeedSource::Codex32(_) => b"codex32",
            SeedSource::Loaded => return None,
        };
        let material: Vec<u8> = match source {
            SeedSource::Words(k) => items.get(*k)?.bytes.clone(),
            SeedSource::WordsWith(k, p) => {
                let mut v = items.get(*k)?.bytes.clone();
                v.push(0);
                v.extend_from_slice(&items.get(*p)?.bytes);
                v
            }
            SeedSource::Slip39(ks) | SeedSource::Codex32(ks) => ks
                .iter()
                .filter_map(|k| items.get(*k))
                .flat_map(|i| i.bytes.clone())
                .collect(),
            SeedSource::Loaded => return None,
        };
        let material = Zeroizing::new(material);
        self.fingerprint_memo(tag, &material, || {
            self.secret_in(items, source)?
                .master(&self.source_passphrase(items, source), &self.session)
                .map(|m| m.fingerprint())
        })
    }

    /// The passphrase a seed source among `items` brings: a passphrase
    /// file's one line, else none.
    pub(crate) fn source_passphrase(
        &self,
        items: &[Item],
        source: &SeedSource,
    ) -> Zeroizing<String> {
        let SeedSource::WordsWith(_, p) = source else {
            return Zeroizing::new(String::new());
        };
        let Some(item) = items.get(*p) else {
            return Zeroizing::new(String::new());
        };
        let text = Zeroizing::new(String::from_utf8_lossy(&item.bytes).into_owned());
        let line = text
            .strip_suffix('\n')
            .map(|t| t.strip_suffix('\r').unwrap_or(t))
            .unwrap_or(&text);
        Zeroizing::new(line.to_string())
    }

    /// Everything the Inbox gives, put together. Kept in the drawing memo
    /// while a frame is drawn (`docs/SPEED.md` §3.3). Costs a read of
    /// every Inbox file that may describe a wallet or a seed.
    pub fn inbox_found(&self) -> Found {
        if self.drawing.is_on() {
            return self
                .drawing
                .inbox_found
                .get_or_init(|| self.found_in(&self.inbox))
                .clone();
        }
        self.found_in(&self.inbox)
    }

    /// Everything `items` give, put together, with the session's own keys
    /// and wallets: the Inbox, or the files a boot import holds. Places in
    /// what it returns are places in `items`.
    pub(crate) fn found_in(&self, items: &[Item]) -> Found {
        let mut found = Found::default();

        // Seeds: words files, then split seeds that add up, then the
        // session's own keys; once each, by fingerprint.
        let add_seed =
            |found: &mut Found, fp: Fingerprint, label: String, src: SeedSource, loaded: bool| {
                match found.seeds.iter_mut().find(|s| s.fingerprint == fp) {
                    Some(s) => {
                        s.loaded |= loaded;
                        if !s.sources.contains(&src) {
                            s.sources.push(src);
                        }
                    }
                    None => found.seeds.push(FoundSeed {
                        fingerprint: fp,
                        label,
                        sources: vec![src],
                        loaded,
                    }),
                }
            };
        let loaded_fp = |fp: Fingerprint| self.session.keys.iter().any(|k| k.fingerprint() == fp);
        // A passphrase file a stick keeps beside a seed's words, named by
        // the fingerprint the words make with it (`docs/BACKUP-FLOW.md`
        // §19.12 item 6): the words with it are that key, and the words
        // alone no seed of their own.
        let passphrase_files: Vec<(usize, String)> = items
            .iter()
            .enumerate()
            .filter_map(|(k, it)| {
                let fp = it.name.strip_prefix("passphrase-")?.strip_suffix(".txt")?;
                (fp.len() == 8 && fp.bytes().all(|b| b.is_ascii_hexdigit()))
                    .then(|| (k, fp.to_ascii_lowercase()))
            })
            .collect();
        for (k, item) in items.iter().enumerate() {
            if item.kind != FileKind::Words {
                continue;
            }
            let mut with = false;
            for (p, name) in &passphrase_files {
                let src = SeedSource::WordsWith(k, *p);
                if let Some(fp) = self.source_fingerprint(items, &src)
                    && fp_text(fp).eq_ignore_ascii_case(name)
                {
                    add_seed(&mut found, fp, stem(&item.name), src, loaded_fp(fp));
                    with = true;
                }
            }
            if with {
                continue;
            }
            let src = SeedSource::Words(k);
            if let Some(fp) = self.source_fingerprint(items, &src) {
                add_seed(&mut found, fp, stem(&item.name), src, loaded_fp(fp));
            }
        }
        // Split seeds: SLIP-39 shares by their identifier, codex32 strings
        // by theirs.
        let mut slip: Vec<(u16, Vec<usize>, u8)> = Vec::new();
        let mut codex: Vec<([u8; 4], Vec<usize>, usize, bool)> = Vec::new();
        for (k, item) in items.iter().enumerate() {
            if item.kind != FileKind::SeedPart {
                continue;
            }
            match part_of(&String::from_utf8_lossy(&item.bytes)) {
                Some(Part::Slip39(s)) => match slip.iter_mut().find(|g| g.0 == s.identifier()) {
                    Some(g) => g.1.push(k),
                    None => slip.push((s.identifier(), vec![k], s.member_threshold())),
                },
                Some(Part::Codex32(c)) => {
                    let mut id = [0u8; 4];
                    for (d, s) in id.iter_mut().zip(c.identifier()) {
                        *d = s;
                    }
                    match codex.iter_mut().find(|g| g.0 == id) {
                        Some(g) => {
                            g.1.push(k);
                            g.3 |= c.is_secret();
                        }
                        None => {
                            codex.push((id, vec![k], usize::from(c.threshold()), c.is_secret()))
                        }
                    }
                }
                None => {}
            }
        }
        for (_, ks, need) in slip {
            let src = SeedSource::Slip39(ks.clone());
            match (ks.len() >= usize::from(need))
                .then(|| self.source_fingerprint(items, &src))
                .flatten()
            {
                Some(fp) => add_seed(
                    &mut found,
                    fp,
                    EN.inbox_slip_39_seed.to_string(),
                    src,
                    loaded_fp(fp),
                ),
                None => found.waiting.push(Waiting {
                    line: fill(
                        EN.inbox_slip_39_shares,
                        &[&ks.len().to_string(), &need.to_string()],
                    ),
                    backup: None,
                }),
            }
        }
        for (_, ks, need, secret) in codex {
            let src = SeedSource::Codex32(ks.clone());
            match (secret || ks.len() >= need)
                .then(|| self.source_fingerprint(items, &src))
                .flatten()
            {
                Some(fp) => add_seed(
                    &mut found,
                    fp,
                    EN.inbox_codex32_seed.to_string(),
                    src,
                    loaded_fp(fp),
                ),
                None => found.waiting.push(Waiting {
                    line: fill(
                        EN.inbox_codex32_shares_inline,
                        &[&ks.len().to_string(), &need.to_string()],
                    ),
                    backup: None,
                }),
            }
        }
        for key in &self.session.keys {
            add_seed(
                &mut found,
                key.fingerprint(),
                key.label.clone(),
                SeedSource::Loaded,
                true,
            );
        }
        for (k, item) in items.iter().enumerate() {
            if item.kind == FileKind::Backup {
                found.waiting.push(Waiting {
                    line: fill(EN.inbox_opensigner_backup, &[&item.name.to_string()]),
                    backup: Some(k),
                });
            }
        }

        // Wallets: every file that reads as one, once each by descriptor,
        // then split sheets that add up; and the session's own, so a seed
        // they name is not a potential wallet.
        let push_wallet = |found: &mut Found,
                           name: String,
                           text: &str,
                           file: Option<usize>,
                           shares: bool,
                           loaded: bool| {
            let Ok(policy) = crate::wallet::read_wallet(text) else {
                return;
            };
            let descriptor = crate::wallet::same_wallet(&policy);
            if let Some(w) = found
                .wallets
                .iter_mut()
                .find(|w| w.descriptor == descriptor)
            {
                if let Some(k) = file {
                    w.files.push(k);
                }
                w.loaded |= loaded;
                return;
            }
            let wallet = Wallet::new(
                name.clone(),
                policy,
                String::new(),
                crate::wallet::Origin::Named,
            );
            let keys: Vec<Fingerprint> = self
                .session
                .slots(&wallet)
                .iter()
                .filter_map(|s| s.fingerprint)
                .collect();
            found.wallets.push(FoundWallet {
                name,
                descriptor,
                files: file.into_iter().collect(),
                from_shares: shares,
                keys,
                shape: Session::shape(&wallet),
                needed: crate::wallet::needed(&wallet),
                seeds_here: 0,
                loaded,
            });
        };
        let loaded_desc: BTreeSet<String> = self
            .session
            .wallets
            .iter()
            .map(|w| crate::wallet::same_wallet(w.policy()))
            .collect();
        for (k, item) in items.iter().enumerate() {
            if item.kind != FileKind::Wallet {
                continue;
            }
            let text = String::from_utf8_lossy(&item.bytes).into_owned();
            let loaded = crate::wallet::read_wallet(&text)
                .is_ok_and(|p| loaded_desc.contains(&crate::wallet::same_wallet(&p)));
            push_wallet(
                &mut found,
                self.wallet_name_in(items, k),
                &text,
                Some(k),
                false,
                loaded,
            );
        }
        let shares: Vec<String> = items
            .iter()
            .filter(|i| i.kind == FileKind::Share)
            .map(|i| String::from_utf8_lossy(&i.bytes).into_owned())
            .collect();
        if !shares.is_empty() {
            match crate::restore::merge(&shares) {
                Ok(m) => match m.whole {
                    Some(whole) => {
                        let name = osk_bip::multisig_config::parse_named(&whole)
                            .ok()
                            .and_then(|(_, n)| n)
                            .unwrap_or_else(|| EN.inbox_restored_wallet.to_string());
                        let loaded = crate::wallet::read_wallet(&whole)
                            .is_ok_and(|p| loaded_desc.contains(&crate::wallet::same_wallet(&p)));
                        push_wallet(&mut found, name, &whole, None, true, loaded);
                    }
                    None => found.waiting.push(Waiting {
                        line: fill(
                            EN.inbox_split_sheets_of,
                            &[&m.have.len().to_string(), &m.n.to_string()],
                        ),
                        backup: None,
                    }),
                },
                Err(e) => found.waiting.push(Waiting {
                    line: fill(EN.inbox_split_sheets, &[&e.to_string()]),
                    backup: None,
                }),
            }
        }
        for w in &self.session.wallets {
            push_wallet(
                &mut found,
                w.name.clone(),
                &w.policy().to_descriptor(),
                None,
                false,
                true,
            );
        }
        let seeds: Vec<Fingerprint> = found.seeds.iter().map(|s| s.fingerprint).collect();
        for w in found.wallets.iter_mut() {
            w.seeds_here = w.keys.iter().filter(|k| seeds.contains(k)).count();
        }
        // Account xpubs: once each, and only those no wallet here uses and
        // no seed here gives (the seed is the stronger of the two).
        for (k, item) in items.iter().enumerate() {
            if item.kind != FileKind::Key {
                continue;
            }
            let text = String::from_utf8_lossy(&item.bytes);
            let Some(key) = crate::create::read_key(&text) else {
                continue;
            };
            let Ok(parsed) = osk_bip::policy::PolicyKey::parse(&key) else {
                continue;
            };
            let kind = match parsed.path().map(kind_of_path) {
                Some(Err(())) => continue,
                Some(Ok(k)) => k,
                None => None,
            };
            let xpub = parsed.xpub().to_string();
            let fp = parsed.fingerprint();
            if found.wallets.iter().any(|w| w.descriptor.contains(&xpub))
                || fp.is_some_and(|f| seeds.contains(&f))
            {
                continue;
            }
            match found.xpubs.iter_mut().find(|x| x.key == key) {
                Some(x) => x.files.push(k),
                None => found.xpubs.push(FoundXpub {
                    key,
                    fingerprint: fp,
                    path: parsed.path().map(|p| format!("m/{p}").replace('\'', "h")),
                    kind,
                    files: vec![k],
                }),
            }
        }
        found
    }

    /// The name a wallet file gives its wallet: a wallet .json's label,
    /// or the file's name in words.
    pub(crate) fn inbox_wallet_name(&self, index: usize) -> String {
        self.wallet_name_in(&self.inbox, index)
    }

    /// [`Faraday::inbox_wallet_name`] for a file among `items`.
    pub(crate) fn wallet_name_in(&self, items: &[Item], index: usize) -> String {
        let Some(item) = items.get(index) else {
            return String::new();
        };
        let text = String::from_utf8_lossy(&item.bytes);
        let name = stem(&item.name);
        let name = name
            .strip_suffix("-wallet")
            .unwrap_or(&name)
            .split(['-', '_'])
            .map(|w| {
                let mut c = w.chars();
                c.next()
                    .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" ");
        crate::wallet::json_string(&text, "label")
            .filter(|l| text.trim_start().starts_with('{') && !l.trim().is_empty())
            .unwrap_or(name)
    }

    /// Loads the chosen wallets the Inbox describes, each with the seeds
    /// here that are its keys. Seeds load only with no stick attached;
    /// the wallets load either way. Returns (wallets, seeds) loaded.
    pub(crate) fn inbox_load(&mut self) -> (usize, usize) {
        let inbox = std::mem::take(&mut self.inbox);
        let skip = self.inbox_skip.clone();
        let loaded = self.load_found_in(&inbox, &skip, &|_| false);
        self.inbox = inbox;
        self.refresh_spend();
        loaded
    }

    /// Loads the wallets `items` describe but those in `skip` (by
    /// descriptor), each with the seeds among `items` that are its keys,
    /// and the seeds `also` names whether a wallet chosen uses them or
    /// not. Seeds load only with no stick attached. Returns (wallets,
    /// seeds) loaded.
    pub(crate) fn load_found_in(
        &mut self,
        items: &[Item],
        skip: &BTreeSet<String>,
        also: &dyn Fn(Fingerprint) -> bool,
    ) -> (usize, usize) {
        let found = self.found_in(items);
        let chosen: Vec<FoundWallet> = found
            .to_load()
            .into_iter()
            .filter(|w| !skip.contains(&w.descriptor))
            .cloned()
            .collect();
        let mut keys = 0;
        if self.may_load_keys() {
            for s in &found.seeds {
                if s.loaded
                    || !(chosen.iter().any(|w| w.keys.contains(&s.fingerprint))
                        || also(s.fingerprint))
                {
                    continue;
                }
                let pass = s
                    .sources
                    .first()
                    .map(|src| self.source_passphrase(items, src))
                    .unwrap_or_default();
                if let Some(secret) = s.sources.first().and_then(|src| self.secret_in(items, src))
                    && secret.add(&pass, &s.label, &mut self.session).is_ok()
                {
                    keys += 1;
                }
            }
        }
        let mut wallets = 0;
        for w in &chosen {
            let (name, text) = match w.files.first() {
                Some(k) => (
                    self.wallet_name_in(items, *k),
                    String::from_utf8_lossy(&items[*k].bytes).into_owned(),
                ),
                None => (w.name.clone(), w.descriptor.clone()),
            };
            let source = w
                .files
                .first()
                .map_or(EN.inbox_its_shares.to_string(), |k| items[*k].name.clone());
            if self
                .session
                .add_wallet(&name, &text, &source, crate::wallet::Origin::Named)
                .is_ok()
            {
                wallets += 1;
            }
        }
        (wallets, keys)
    }

    /// After seeds load on their own (Import and load, Add a key): the
    /// wallets in the Inbox that name them load too.
    pub(crate) fn inbox_load_named(&mut self) -> usize {
        let found = self.inbox_found();
        let loaded: Vec<Fingerprint> = self.session.keys.iter().map(|k| k.fingerprint()).collect();
        let mut n = 0;
        for w in found.to_load() {
            if !w.keys.iter().any(|k| loaded.contains(k)) {
                continue;
            }
            let (name, text, source) = match w.files.first() {
                Some(k) => (
                    self.inbox_wallet_name(*k),
                    String::from_utf8_lossy(&self.inbox[*k].bytes).into_owned(),
                    self.inbox[*k].name.clone(),
                ),
                None => (
                    w.name.clone(),
                    w.descriptor.clone(),
                    EN.inbox_its_shares.to_string(),
                ),
            };
            if self
                .session
                .add_wallet(&name, &text, &source, crate::wallet::Origin::Named)
                .is_ok()
            {
                n += 1;
            }
        }
        if n > 0 {
            self.refresh_spend();
        }
        n
    }
}

/// A potential wallet being made: the seed, the passphrase the person
/// adds or not, and the kind of single-key wallet.
#[derive(Default)]
pub struct Potential {
    /// The seed's fingerprint with no passphrase, which names it.
    pub fingerprint: Option<Fingerprint>,
    /// The passphrase typed, empty for none.
    pub passphrase: crate::secret_text::SecretText,
    /// Typing goes to the passphrase.
    pub typing: bool,
    /// The passphrase is shown in the clear.
    pub shown: bool,
    /// The kind of single-key wallet it becomes.
    pub kind: NewKind,
    /// The fingerprint with the passphrase, worked out when it changes.
    pub with: Option<Fingerprint>,
    /// The last refusal.
    pub error: Option<String>,
    /// An OpenSigner backup being opened instead, by Inbox place.
    pub backup: Option<usize>,
    /// An account xpub becoming a watch-only wallet instead, with whether
    /// its path fixes the kind.
    pub xpub: Option<(String, bool)>,
}

/// The single-key kinds a potential wallet may become, the most common
/// first.
pub const KINDS: [NewKind; 4] = [
    NewKind::NativeSegwit,
    NewKind::Taproot,
    NewKind::NestedSegwit,
    NewKind::Legacy,
];

impl Faraday {
    /// Opens the sheet for making seed `fp` a wallet.
    pub(crate) fn potential_open(&mut self, fp: Fingerprint) {
        self.potential = Some(Potential {
            fingerprint: Some(fp),
            with: Some(fp),
            ..Potential::default()
        });
        self.sheet = Some(crate::Sheet::Potential);
    }

    /// Opens the sheet for making account xpub `i` of the Inbox a
    /// watch-only wallet.
    pub(crate) fn xpub_open(&mut self, i: usize) {
        let Some(x) = self.inbox_found().xpubs.into_iter().nth(i) else {
            return;
        };
        self.potential = Some(Potential {
            fingerprint: x.fingerprint,
            kind: x.kind.unwrap_or_default(),
            xpub: Some((x.key, x.kind.is_some())),
            ..Potential::default()
        });
        self.sheet = Some(crate::Sheet::Potential);
    }

    /// Makes the watch-only wallet an account xpub describes.
    fn xpub_make(&mut self) {
        let Some((key, kind, fp)) = self.potential.as_ref().and_then(|p| {
            p.xpub
                .as_ref()
                .map(|(k, _)| (k.clone(), p.kind, p.fingerprint))
        }) else {
            return;
        };
        let descriptor = kind.descriptor(1, &[key]);
        let name = fill(
            EN.inbox_watch_only,
            &[
                &fp.map(fp_text)
                    .unwrap_or_else(|| EN.inbox_xpub.to_string())
                    .to_string(),
                crate::family::kind_name(kind),
            ],
        );
        match self.session.add_wallet(
            &name,
            &descriptor,
            EN.inbox_an_xpub_in_files,
            crate::wallet::Origin::Named,
        ) {
            Ok(i) => {
                self.wallet = i;
                self.potential = None;
                self.sheet = None;
                self.refresh_spend();
                self.toast(&fill(EN.inbox_is_a_wallet, &[&name.to_string()]));
            }
            Err(e) => {
                if let Some(p) = self.potential.as_mut() {
                    p.error = Some(e.text());
                }
            }
        }
    }

    /// Opens the sheet for an OpenSigner backup's passphrase.
    pub(crate) fn backup_open(&mut self, k: usize) {
        self.potential = Some(Potential {
            backup: Some(k),
            typing: true,
            ..Potential::default()
        });
        self.sheet = Some(crate::Sheet::Potential);
    }

    /// The seed being made a wallet, as the Inbox or the session holds it.
    fn potential_seed(&self) -> Option<(FoundSeed, Option<Secret>)> {
        let fp = self.potential.as_ref()?.fingerprint?;
        let seed = self
            .inbox_found()
            .seeds
            .into_iter()
            .find(|s| s.fingerprint == fp)?;
        let secret = seed
            .sources
            .iter()
            .find(|s| **s != SeedSource::Loaded)
            .and_then(|s| self.inbox_secret(s));
        Some((seed, secret))
    }

    /// Works out the fingerprint the typed passphrase gives.
    pub(crate) fn potential_refresh(&mut self) {
        let Some(p) = self.potential.as_ref() else {
            return;
        };
        if p.backup.is_some() || p.xpub.is_some() {
            return;
        }
        let with = if p.passphrase.is_empty() {
            p.fingerprint
        } else {
            let pass = p.passphrase.clone();
            match self.potential_seed() {
                Some((_, Some(secret))) => {
                    secret.master(&pass, &self.session).map(|m| m.fingerprint())
                }
                Some((seed, None)) => self
                    .session
                    .keys
                    .iter()
                    .find(|k| k.fingerprint() == seed.fingerprint)
                    .and_then(|k| k.words.as_ref().map(|w| (w.clone(), k.language)))
                    .and_then(|(w, lang)| crate::wallet::parse_words(lang, &w).ok())
                    .and_then(|m| Secret::Words(m).master(&pass, &self.session))
                    .map(|m| m.fingerprint()),
                None => None,
            }
        };
        if let Some(p) = self.potential.as_mut() {
            p.with = with;
        }
    }

    /// The wallet here, in the Inbox or loaded, whose key the potential
    /// wallet's seed with its passphrase is.
    pub fn potential_match(&self) -> Option<String> {
        let with = self.potential.as_ref()?.with?;
        self.inbox_found()
            .wallets
            .into_iter()
            .find(|w| w.keys.contains(&with))
            .map(|w| w.name)
    }

    /// Makes the potential wallet a wallet: its seed loads with the
    /// passphrase typed, if any; a wallet in the Inbox that names the key
    /// so made loads with it; else a single-key wallet of the kind chosen.
    pub(crate) fn potential_make(&mut self) {
        // A watch-only wallet holds nothing secret: a stick may stay in.
        if self.potential.as_ref().is_some_and(|p| p.xpub.is_some()) {
            self.xpub_make();
            return;
        }
        if !self.may_load_keys() {
            let error = fill(EN.inbox_remove_the_first, &[self.medium.noun()]);
            if let Some(p) = self.potential.as_mut() {
                p.error = Some(error);
            }
            return;
        }
        if self.potential.as_ref().is_some_and(|p| p.backup.is_some()) {
            self.backup_unseal();
            return;
        }
        let Some(p) = self.potential.as_ref() else {
            return;
        };
        let pass = p.passphrase.clone();
        let kind = p.kind;
        let Some((seed, secret)) = self.potential_seed() else {
            return;
        };
        let label = if pass.is_empty() {
            seed.label.clone()
        } else {
            fill(EN.inbox_with_passphrase, &[&seed.label.to_string()])
        };
        // A passphrase file beside the words, where none is typed.
        let kept = seed
            .sources
            .first()
            .map(|src| self.source_passphrase(&self.inbox, src))
            .unwrap_or_default();
        // The key itself: already loaded with no passphrase, or added now.
        let added = match (&secret, pass.is_empty()) {
            (_, true) if seed.loaded => Ok(seed.fingerprint),
            (Some(s), true) => s.add(&kept, &label, &mut self.session),
            (Some(s), _) => s.add(&pass, &label, &mut self.session),
            (None, false) => {
                let words = self
                    .session
                    .keys
                    .iter()
                    .find(|k| k.fingerprint() == seed.fingerprint)
                    .and_then(|k| k.words.clone());
                match words {
                    Some(w) => self.session.add_words_with(&w, &pass, &label, None),
                    None => Err(Refusal::Words(EN.inbox_this_seed_was.to_string())),
                }
            }
            (None, true) => Ok(seed.fingerprint),
        };
        let fp = match added {
            Ok(fp) => fp,
            // Loaded already with this passphrase: the wallet is what is
            // left to make.
            Err(Refusal::Duplicate(_)) => self
                .potential
                .as_ref()
                .and_then(|p| p.with)
                .unwrap_or(seed.fingerprint),
            Err(e) => {
                if let Some(p) = self.potential.as_mut() {
                    p.error = Some(e.text());
                }
                return;
            }
        };
        // A wallet here that names the key: it is the wallet.
        if self.potential_match().is_some()
            || self
                .inbox_found()
                .wallets
                .iter()
                .any(|w| w.keys.contains(&fp))
        {
            self.inbox_load_named();
            self.potential = None;
            self.sheet = None;
            self.toast(&fill(EN.inbox_loaded_with_its, &[&fp_text(fp).to_string()]));
            return;
        }
        // Else a single-key wallet of the kind chosen.
        let Some(key) = self.session.keys.iter().find(|k| k.fingerprint() == fp) else {
            return;
        };
        let text = match kind.key_text(key.master()) {
            Ok(t) => t,
            Err(e) => {
                if let Some(p) = self.potential.as_mut() {
                    p.error = Some(e);
                }
                return;
            }
        };
        let descriptor = kind.descriptor(1, &[text]);
        let name = format!("{} · {}", fp_text(fp), crate::family::kind_name(kind));
        match self.session.add_wallet(
            &name,
            &descriptor,
            EN.inbox_a_seed_in_files,
            crate::wallet::Origin::Named,
        ) {
            Ok(i) => {
                self.wallet = i;
                self.potential = None;
                self.sheet = None;
                self.refresh_spend();
                self.toast(&fill(EN.inbox_is_a_wallet, &[&name.to_string()]));
            }
            Err(e) => {
                if let Some(p) = self.potential.as_mut() {
                    p.error = Some(e.text());
                }
            }
        }
    }

    /// Opens an OpenSigner backup with the passphrase typed: its words or
    /// seed load as a key, which is then a potential wallet or loads with
    /// its wallet; its recovery sheet loads as a wallet.
    fn backup_unseal(&mut self) {
        let Some(p) = self.potential.as_ref() else {
            return;
        };
        let Some(k) = p.backup else {
            return;
        };
        let Some(item) = self.inbox.get(k) else {
            return;
        };
        let label = stem(&item.name);
        use osk_backup::oskb::{Error, Opened};
        let opened = osk_backup::oskb::open_payload(&item.bytes, p.passphrase.as_bytes());
        let result: Result<String, String> = match opened {
            Ok(Opened::Words(m)) => self
                .session
                .add_words(&phrase(&m), &label, None)
                .map(|fp| fill(EN.inbox_loaded_n, &[&fp_text(fp).to_string()]))
                .map_err(|e| e.text()),
            Ok(Opened::Seed(s)) => self
                .session
                .add_seed(&s, &label)
                .map(|fp| fill(EN.inbox_loaded_n, &[&fp_text(fp).to_string()]))
                .map_err(|e| e.text()),
            Ok(Opened::Sheet(sheet)) => {
                let text = String::from_utf8_lossy(&sheet.descriptor).into_owned();
                let name = String::from_utf8_lossy(&sheet.name).into_owned();
                let name = if name.trim().is_empty() {
                    label.clone()
                } else {
                    name
                };
                self.session
                    .add_wallet(
                        &name,
                        &text,
                        &item_name(self, k),
                        crate::wallet::Origin::Named,
                    )
                    .map(|_| fill(EN.inbox_loaded_n, &[&name.to_string()]))
                    .map_err(|e| e.text())
            }
            Ok(Opened::Note(_)) => Err(EN.inbox_this_backup_holds.to_string()),
            Err(Error::Passphrase) => Err(EN.inbox_not_this_backup.to_string()),
            Err(e) => Err(format!("{e:?}")),
        };
        match result {
            Ok(said) => {
                self.inbox_load_named();
                self.refresh_spend();
                self.potential = None;
                self.sheet = None;
                self.toast(&said);
            }
            Err(e) => {
                if let Some(p) = self.potential.as_mut() {
                    p.error = Some(e);
                    p.passphrase.clear();
                }
            }
        }
    }
}

fn item_name(app: &Faraday, k: usize) -> String {
    app.inbox.get(k).map_or(String::new(), |i| i.name.clone())
}
