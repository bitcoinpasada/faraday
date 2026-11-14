//! My seed makes my wallet's **Compare with the wallet app's**
//! (`docs/FLOWS.md`): a key, an address or a wallet the person gives, typed,
//! scanned or taken from Files, held against what the seed makes at the
//! account picked. Public values only: the key is read through
//! `faraday-keys`' public reads (an extended public key at a path).

use super::{Made, Source};
use crate::address_find::{DEPTHS, script};
use crate::strings::EN;
use crate::{Faraday, ScanPurpose, ScanState, Sheet};
use faraday_bip::bitcoin::address::NetworkUnchecked;
use faraday_bip::bitcoin::bip32::{DerivationPath, Xpub};
use faraday_bip::bitcoin::secp256k1::Secp256k1;
use faraday_bip::bitcoin::{Address, Network as BtcNetwork, NetworkKind};
use faraday_bip::keys::{Fingerprint, Network};
use faraday_bip::policy::{PolicyKey, WalletPolicy};
use faraday_bip::slip132;
use faraday_crypto::work::Compute;
use faraday_keys::Keys;
use faraday_shell_api::Command;
use faraday_text::Text;

/// The most characters the comparison's field takes: a wallet file's.
pub(super) const FIELD_CHARS: usize = 8192;

/// What the comparison says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    /// The value is this seed's, with the line saying more where there is one.
    Matches(Option<Text>),
    /// It is not, with the line saying more where there is one, and whether
    /// the line is a warning (an origin names this seed and its key is
    /// not).
    Differs(Option<Text>, bool),
    /// The value is none the comparison takes.
    Refused(Text),
}

/// The comparison, open on the Result.
#[derive(Default)]
pub(super) struct Compare {
    /// The value typed, scanned or taken from a file.
    pub(super) text: Text,
    /// Typing goes to the field.
    pub(super) focus: bool,
    /// What it was compared to, once it was.
    pub(super) answer: Option<Answer>,
}

/// What one key of a value is to the seed.
enum Verdict {
    /// The seed's own key.
    Ours,
    /// The key the same words make with no passphrase.
    Without,
    /// The origin names the seed, and the key is not its key.
    Fingerprint,
    /// Another key.
    Other,
}

/// The seed the comparison reads keys of.
struct Seed<'a> {
    c: &'a Compute,
    keys: &'a Keys,
    source: Source,
    made: &'a Made,
}

impl Seed<'_> {
    /// Costs a key derivation and an elliptic-curve multiplication. The
    /// seed's extended public key at `path`.
    fn xpub_at(&self, path: &DerivationPath) -> Option<Xpub> {
        match self.source {
            Source::Held => self.keys.check_xpub_at(self.c, path),
            Source::Loaded(i) => self.keys.xpub_at(self.c, i, path),
        }
    }

    /// Costs a PBKDF2, a key derivation and an elliptic-curve
    /// multiplication. The extended public key at `path` of the same words
    /// with no passphrase, where a passphrase was given.
    fn xpub_without_at(&self, path: &DerivationPath) -> Option<Xpub> {
        self.made.without?;
        match self.source {
            Source::Held => self.keys.check_xpub_without_passphrase_at(self.c, path),
            Source::Loaded(i) => self.keys.xpub_without_passphrase_at(self.c, i, path),
        }
    }

    /// Costs two key derivations and, where the seed has a passphrase, a
    /// PBKDF2 and two more. What the key `xpub`, at `path` and of the
    /// master `fingerprint` where its origin names one, is to the seed.
    fn judge(
        &self,
        fingerprint: Option<Fingerprint>,
        path: &DerivationPath,
        xpub: &Xpub,
    ) -> Verdict {
        let same = |a: Option<Xpub>| {
            a.is_some_and(|a| a.public_key == xpub.public_key && a.chain_code == xpub.chain_code)
        };
        match fingerprint {
            Some(f) if f == self.made.fingerprint => {
                if same(self.xpub_at(path)) {
                    Verdict::Ours
                } else {
                    Verdict::Fingerprint
                }
            }
            Some(f) if Some(f) == self.made.without => {
                if same(self.xpub_without_at(path)) {
                    Verdict::Without
                } else {
                    Verdict::Fingerprint
                }
            }
            Some(_) => Verdict::Other,
            None => {
                if same(self.xpub_at(path)) {
                    Verdict::Ours
                } else if same(self.xpub_without_at(path)) {
                    Verdict::Without
                } else {
                    Verdict::Other
                }
            }
        }
    }
}

/// The path as the Result writes it, `m/84h/0h/0h`.
fn path_text(path: &DerivationPath) -> Text {
    let shown = Text::replaced(&Text::formatted(format_args!("{path}")), "'", "h");
    Text::formatted(format_args!("m/{}", shown.as_str()))
}

fn network_is(kind: NetworkKind, network: Network) -> bool {
    (kind == NetworkKind::Main) == (network == Network::Mainnet)
}

/// Costs what [`faraday_bip::slip132::plain_descriptor`] and
/// `PolicyKey::parse` cost, and two key derivations (a PBKDF2 more where
/// the seed has a passphrase). `text` as a key with its origin, in any
/// spelling; `None` where it is none. A key with no origin is refused, and
/// one whose origin is at another path than the account picked is not that
/// account's key: the path is named.
fn as_key(seed: &Seed<'_>, network: Network, text: &str) -> Option<Answer> {
    // A bare key in a SLIP-132 spelling (`zpub`) names its script type in
    // its version and nothing else: refused, as every bare key is.
    if slip132::decode_xpub(text.trim()).is_ok() {
        return Some(Answer::Refused(Text::of(EN.create_scan_no_origin)));
    }
    let plain = slip132::plain_descriptor(text).ok()?;
    let parsed = PolicyKey::parse(plain.trim()).ok()?;
    let (Some(fingerprint), Some(path)) = (parsed.fingerprint(), parsed.path()) else {
        return Some(Answer::Refused(Text::of(EN.create_scan_no_origin)));
    };
    let xpub = parsed.xpub();
    if !network_is(xpub.network, network) {
        return Some(Answer::Refused(Text::of(EN.seed_check_compare_network)));
    }
    if path != &seed.made.derivation {
        return Some(Answer::Differs(
            Some(Text::fill(EN.seed_check_other_path, &[&path_text(path)])),
            false,
        ));
    }
    Some(match seed.judge(Some(fingerprint), path, xpub) {
        Verdict::Ours => Answer::Matches(None),
        Verdict::Without => Answer::Differs(Some(Text::of(EN.seed_check_without_pass)), false),
        Verdict::Fingerprint => finger_only(path),
        Verdict::Other => Answer::Differs(None, false),
    })
}

fn finger_only(path: &DerivationPath) -> Answer {
    Answer::Differs(
        Some(Text::fill(EN.seed_check_finger_only, &[&path_text(path)])),
        true,
    )
}

/// Costs an address derivation for each of the first
/// [`DEPTHS`]`[0]` indices of both chains. `text` as an address of the
/// account picked; `None` where it is none.
fn as_address(c: &Compute, made: &Made, network: Network, text: &str) -> Option<Answer> {
    let address: Address<NetworkUnchecked> = text.trim().parse().ok()?;
    let Ok(address) = address.require_network(BtcNetwork::from(network)) else {
        return Some(Answer::Refused(Text::of(EN.seed_check_compare_network)));
    };
    if made.cosigner {
        return Some(Answer::Refused(Text::of(EN.seed_check_compare_cosigner)));
    }
    let Ok(policy) = WalletPolicy::from_xpub(made.xpub, made.kind) else {
        return Some(Answer::Differs(None, false));
    };
    let target = address.script_pubkey();
    let secp = Secp256k1::verification_only();
    for index in 0..DEPTHS[0] {
        for change in [false, true] {
            if script(c, &secp, &policy, change, index).as_ref() == Some(&target) {
                let at = Text::formatted(format_args!(
                    "{}/{}/{index}",
                    made.path.as_str(),
                    u8::from(change)
                ));
                let line = if change {
                    EN.seed_check_found_change
                } else {
                    EN.seed_check_found_receive
                };
                let n = Text::num(u64::from(index));
                return Some(Answer::Matches(Some(Text::fill(line, &[&n, &at]))));
            }
        }
    }
    Some(Answer::Differs(
        Some(Text::fill(
            EN.seed_check_not_found,
            &[&Text::num(u64::from(DEPTHS[0]))],
        )),
        false,
    ))
}

/// Costs what Files' reader costs (`read_wallet`) and, for each key of the
/// wallet with this seed's fingerprint, what [`Seed::judge`] costs. `text`
/// as a descriptor, a BIP-388 policy or a wallet file.
fn as_wallet(seed: &Seed<'_>, network: Network, text: &str) -> Answer {
    let policy = match faraday_intake::wallet::read_wallet(seed.c, text) {
        Ok(p) => p,
        Err(e) => return Answer::Refused(e.text()),
    };
    if policy
        .keys()
        .iter()
        .any(|k| !network_is(k.network_kind(), network))
    {
        return Answer::Refused(Text::of(EN.seed_check_compare_network));
    }
    let mut without = false;
    let mut finger: Option<&DerivationPath> = None;
    for (k, key) in policy.keys().iter().enumerate() {
        let Some(path) = key.path() else { continue };
        match seed.judge(key.fingerprint(), path, key.xpub()) {
            Verdict::Ours => {
                let n = Text::num(k as u64 + 1);
                let line = match (policy.quorum(), policy.keys().len()) {
                    (Some((m, total)), _) => Some(Text::fill(
                        EN.seed_check_key_of,
                        &[&n, &Text::num(m as u64), &Text::num(total as u64)],
                    )),
                    (None, 1) => None,
                    (None, _) => Some(Text::fill(EN.seed_check_key_of_wallet, &[&n])),
                };
                // The account picked is one path; a key of the wallet at
                // another is named, so no answer passes it for this one.
                let line = if path == &seed.made.derivation {
                    line
                } else {
                    let at = path_text(path);
                    Some(match line {
                        Some(l) => {
                            Text::formatted(format_args!("{} · {}", l.as_str(), at.as_str()))
                        }
                        None => Text::fill(EN.seed_check_matches_at, &[&at]),
                    })
                };
                return Answer::Matches(line);
            }
            Verdict::Without => without = true,
            Verdict::Fingerprint => finger = finger.or(Some(path)),
            Verdict::Other => {}
        }
    }
    if without {
        Answer::Differs(Some(Text::of(EN.seed_check_without_pass)), false)
    } else if let Some(path) = finger {
        finger_only(path)
    } else {
        Answer::Differs(None, false)
    }
}

/// Costs what [`as_key`], [`as_address`] and [`as_wallet`] cost, and what
/// [`crate::scan_key::secret_code`] costs. What `text` is to the key at
/// `source`, whose Result is `made`: a private key or a seed is refused
/// first, then a key, an address and a wallet are tried in turn.
pub(super) fn compare(
    c: &Compute,
    keys: &Keys,
    source: Source,
    made: &Made,
    network: Network,
    text: &str,
) -> Answer {
    let text = text.trim();
    if crate::scan_key::secret_code(c, text.as_bytes()).is_some()
        || slip132::key_facts(text, network) == slip132::KeyReading::Private
    {
        return Answer::Refused(Text::of(EN.seed_check_compare_private));
    }
    let seed = Seed {
        c,
        keys,
        source,
        made,
    };
    if let Some(a) = as_key(&seed, network, text) {
        return a;
    }
    if let Some(a) = as_address(c, made, network, text) {
        return a;
    }
    if text.is_empty() {
        return Answer::Refused(Text::of(EN.seed_check_compare_unread));
    }
    as_wallet(&seed, network, text)
}

/// The comparison for the worker, owned: what the seed's key makes is read
/// from a view that holds that one key ([`Keys::compare_view`]).
pub(crate) struct CompareWork {
    keys: Keys,
    source: Source,
    made: Made,
    network: Network,
    text: Text,
}

/// What the comparison answered, with what it was asked, so that an answer
/// for a value or an account that changed meanwhile is dropped.
pub(crate) struct CompareDone {
    answer: Answer,
    text: Text,
    xpub: Xpub,
    path: DerivationPath,
}

impl CompareWork {
    /// Costs what [`compare`] costs.
    pub(crate) fn run(self, c: &Compute) -> CompareDone {
        let answer = compare(
            c,
            &self.keys,
            self.source,
            &self.made,
            self.network,
            &self.text,
        );
        CompareDone {
            answer,
            text: self.text,
            xpub: self.made.xpub,
            path: self.made.derivation,
        }
    }
}

/// Whether the field holds a secret: a private key, a seed, a secret file
/// or a part of one, as Transfer's Send field refuses them. Costs what
/// `transfer::check` costs.
fn holds_secret(c: &Compute, network: Network, text: &str) -> bool {
    use crate::transfer::TextCheck;
    let found = match crate::transfer::check(c, text) {
        TextCheck::Secret(_) | TextCheck::Inside(_) | TextCheck::Partial | TextCheck::SeedHex => {
            true
        }
        TextCheck::Fine | TextCheck::TooLong => false,
    };
    found || slip132::key_facts(text.trim(), network) == slip132::KeyReading::Private
}

/// Costs what [`holds_secret`] costs. A secret in the field is refused when
/// the whole value is taken (Compare, Enter, a code, a file): the field is
/// emptied, wiped, and the refusal is the answer. Returns whether it was
/// refused.
pub(super) fn refuse_secret(app: &mut Faraday) -> bool {
    let c = app.compute();
    let network = app.session.keys.network();
    let Some(cmp) = app
        .tools
        .seed_check
        .state_mut()
        .and_then(|s| s.compare_mut())
    else {
        return false;
    };
    if !holds_secret(&c, network, cmp.text.as_str()) {
        return false;
    }
    cmp.text.clear();
    cmp.answer = Some(Answer::Refused(Text::of(EN.seed_check_compare_private)));
    true
}

/// Costs what [`refuse_secret`] costs. The value in the field is compared
/// with the account picked, on the worker, and the answer shown under it
/// when it comes back ([`done`]).
pub(super) fn run(app: &mut Faraday) {
    if refuse_secret(app) {
        return;
    }
    let network = app.session.keys.network();
    let Some(s) = app.tools.seed_check.state() else {
        return;
    };
    let (Some(source), Some(made), Some(cmp)) = (s.source, s.made(), s.compare()) else {
        return;
    };
    let loaded = match source {
        Source::Held => None,
        Source::Loaded(i) => Some(i),
    };
    let Some(keys) = app.session.keys.compare_view(loaded) else {
        return;
    };
    let work = CompareWork {
        keys,
        source: if loaded.is_some() {
            Source::Loaded(0)
        } else {
            Source::Held
        },
        made: made.clone(),
        network,
        text: cmp.text.clone(),
    };
    app.start(
        crate::worker::Job::SeedCompare(Box::new(work)),
        EN.working_seed_compare,
    );
}

/// The comparison ended: its answer is shown unless the value or the account
/// changed while it ran.
pub(crate) fn done(app: &mut Faraday, done: CompareDone) {
    let Some(s) = app.tools.seed_check.state_mut() else {
        return;
    };
    let same = s
        .made()
        .is_some_and(|m| m.xpub == done.xpub && m.derivation == done.path);
    if !same {
        return;
    }
    if let Some(cmp) = s.compare_mut()
        && cmp.text.as_str() == done.text.as_str()
    {
        cmp.answer = Some(done.answer);
    }
}

/// A value from a scan or a file is put in the field and compared; a secret
/// is refused and gone.
pub(super) fn put(app: &mut Faraday, text: Text) {
    if let Some(cmp) = app
        .tools
        .seed_check
        .state_mut()
        .and_then(|s| s.compare_mut())
    {
        cmp.text = text;
        cmp.answer = None;
        cmp.focus = false;
    }
    run(app);
}

/// The scanner opens for the comparison.
pub(super) fn scan(app: &mut Faraday) {
    let mut scan = ScanState::default();
    scan.purpose = ScanPurpose::SeedCompare;
    app.camera.open(scan);
    app.sheet = Some(Sheet::Scan);
    app.out.commands.push_back(Command::CameraOn);
}

/// Costs what a code read costs, and what [`run`] costs. A code the
/// comparison's scanner read: put into its field and compared, the scanner
/// closed; a code that is no text is named on the scanner. Returns whether
/// the scanner on screen is that one.
pub(crate) fn compare_scanned(app: &mut Faraday, bytes: &zeroize::Zeroizing<Vec<u8>>) -> bool {
    if app.camera.scan().map(|s| s.purpose) != Some(ScanPurpose::SeedCompare) {
        return false;
    }
    let Some((_, _, data)) = app.read_code(bytes.clone()) else {
        return true;
    };
    let own = app.scanned_own(&data);
    let Ok(text) = std::str::from_utf8(&own).map(|t| Text::of(t.trim())) else {
        app.scan_say(EN.seed_check_compare_unread);
        return true;
    };
    app.scan_close();
    put(app, text);
    true
}
