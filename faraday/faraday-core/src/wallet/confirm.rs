//! The wallet confirmation sheet's rows (F254): what a wallet of several
//! keys, or one a key not of its own spends alone, shows before it is
//! added, worked out once when the wallet is held back so that drawing
//! only reads. Check a multisig setup's Result shows the same rows, and
//! the register sheet too.

use super::keycheck::{KeyBlockOf, KeyCheck, draw_keys};
use super::{Origin, Session, Wallet, fp_text};
use crate::strings::EN;
use crate::ui::Ui;
use crate::ui::pal::Role;
use faraday_bip::bitcoin::bip32::{ChildNumber, DerivationPath, Xpub};
use faraday_bip::keys::Fingerprint;
use faraday_bip::policy::{PolicyKey, WalletPolicy};
use faraday_crypto::work::Compute;
use faraday_draw::tokens::{BODY_M, GAP};
use faraday_text::Text;

/// What a cosigner's row says of the keys loaded here and of the person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Mark {
    /// A loaded key's extended key at the path; `true` at a single-key
    /// account's path.
    Yours(bool),
    /// The person ticked it as compared with its own device.
    Checked,
    /// Nothing vouches for it.
    Unchecked,
    /// The fingerprint is a loaded key's, the key at the path is not.
    Wrong(Text),
}

impl Mark {
    /// The state this mark shows.
    pub(crate) fn state(&self) -> KeyCheck {
        match self {
            Mark::Yours(_) => KeyCheck::Yours,
            Mark::Checked => KeyCheck::Checked,
            Mark::Unchecked => KeyCheck::Unchecked,
            Mark::Wrong(_) => KeyCheck::Wrong,
        }
    }
}

/// One cosigner's row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KeyRow {
    pub(crate) number: Text,
    pub(crate) fingerprint: Text,
    pub(crate) path: Text,
    /// The extended public key whole, in groups of four.
    pub(crate) xpub: Text,
    pub(crate) mark: Mark,
    /// A `Yours` key made here whose backup is not done: it signs once it is.
    pub(crate) waiting: bool,
}

/// The rows of the confirmation sheet, worked out once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rows {
    pub(crate) shape: Text,
    pub(crate) keys: Vec<KeyRow>,
    /// Keys the descriptor uses twice, as lines.
    pub(crate) repeats: Vec<Text>,
    pub(crate) checksum: Text,
    /// The key path line of a script tree, and whether it is a danger.
    pub(crate) key_path: Option<(Text, bool)>,
}

impl Rows {
    /// Costs what [`Rows::of`] costs and parsing the descriptor: the rows
    /// of the wallet `descriptor` writes, for the register sheet. `None`
    /// where it does not read as a wallet.
    pub(crate) fn of_descriptor(session: &Session, c: &Compute, descriptor: &str) -> Option<Rows> {
        let policy = WalletPolicy::from_descriptor(c, descriptor).ok()?;
        Some(Rows::of(session, c, &policy, false))
    }

    /// Costs a key derivation for each key of `policy` under a loaded
    /// fingerprint, and the descriptor's checksum: the rows of `policy`
    /// as the sheet shows them. `given`: the checksum came with the text
    /// and holds, not worked out.
    pub(crate) fn of(session: &Session, c: &Compute, policy: &WalletPolicy, given: bool) -> Rows {
        let wallet = Wallet::new("", policy.clone(), "", Origin::Named);
        let keys = key_rows(session, c, policy, &Gives::new());
        let repeats = faraday_bip::descriptor::checksum_facts(c, &policy.to_descriptor())
            .map(|f| f.repeated)
            .unwrap_or_default()
            .iter()
            .map(|(a, b)| {
                Text::fill(
                    EN.tools_key_same_as,
                    &[&Text::num(*b as u64 + 1), &Text::num(*a as u64 + 1)],
                )
            })
            .collect();
        let checksum = if given {
            Text::fill(EN.mscheck_checksum_holds, &[policy.checksum()])
        } else {
            Text::fill(EN.mscheck_checksum_worked_out, &[policy.checksum()])
        };
        Rows {
            shape: Session::shape(&wallet),
            keys,
            repeats,
            checksum,
            key_path: session.key_path_line(policy).map(|l| (l.text, l.danger)),
        }
    }

    /// Toggles the person's tick of key `i` where it is theirs to tick.
    pub(crate) fn tick(&mut self, i: usize) {
        if let Some(k) = self.keys.get_mut(i) {
            k.mark = match k.mark {
                Mark::Unchecked => Mark::Checked,
                Mark::Checked => Mark::Unchecked,
                _ => return,
            };
        }
    }

    /// The person's ticks, one per key.
    pub(crate) fn ticks(&self) -> Vec<bool> {
        self.keys.iter().map(|k| k.mark == Mark::Checked).collect()
    }

    /// Whether a key not of the wallet spends it alone: the sheet's head,
    /// key path line and hold are in the error tone.
    pub(crate) fn danger(&self) -> bool {
        self.key_path.as_ref().is_some_and(|(_, d)| *d)
    }
}

/// What loaded keys gave at origin paths, worked out already: by the
/// fingerprint of the key and the path, so that no derivation is made twice.
pub(crate) type Gives = Vec<(Fingerprint, DerivationPath, Option<Xpub>)>;

/// Costs a key derivation where a loaded key is `k`'s and `memo` does not
/// have what it gives: the row of key `i` of a policy, its mark what the
/// loaded keys give, or unchecked.
pub(crate) fn key_row(
    session: &Session,
    c: &Compute,
    memo: &Gives,
    i: usize,
    k: &PolicyKey,
) -> KeyRow {
    let fp = k.fingerprint();
    let path = k.path();
    let mark = match (fp, path) {
        (Some(fp), Some(path)) => mark_of(session, c, memo, k.xpub(), fp, path),
        _ => Mark::Unchecked,
    };
    let waiting =
        matches!(mark, Mark::Yours(_)) && fp.is_some_and(|f| session.waiting.contains(&f));
    KeyRow {
        number: Text::fill(EN.mscheck_key, &[&Text::num(i as u64 + 1)]),
        fingerprint: fp.map(fp_text).unwrap_or_default(),
        path: path
            .map(|p| Text::of(&crate::register::path_text(p)))
            .unwrap_or_default(),
        xpub: groups_of_four(&k.xpub().to_string()),
        mark,
        waiting,
    }
}

/// Costs what [`key_row`] costs for each key of `policy`.
pub(crate) fn key_rows(
    session: &Session,
    c: &Compute,
    policy: &WalletPolicy,
    memo: &Gives,
) -> Vec<KeyRow> {
    policy
        .keys()
        .iter()
        .enumerate()
        .map(|(i, k)| key_row(session, c, memo, i, k))
        .collect()
}

/// A lookup: whether every key of `policy` has a fingerprint and an
/// origin path and the fingerprint is a loaded key's. Run after
/// [`Session::add_policy`] found no key under a loaded fingerprint that
/// is not what that key gives at its path (public key and chain code),
/// every key is then a loaded key's own at its path: nothing in the
/// wallet came in from outside.
pub(crate) fn all_keys_loaded(session: &Session, policy: &WalletPolicy) -> bool {
    policy.keys().iter().all(|k| {
        k.path().is_some()
            && k.fingerprint()
                .is_some_and(|fp| session.keys.position(fp).is_some())
    })
}

fn path_is_single_account(path: &DerivationPath) -> bool {
    let steps: Vec<&ChildNumber> = path.into_iter().collect();
    steps.len() == 3
        && matches!(
            steps[0],
            ChildNumber::Hardened {
                index: 44 | 49 | 84 | 86
            }
        )
}

/// Costs a key derivation: whether a key loaded here is this cosigner.
fn mark_of(
    session: &Session,
    c: &Compute,
    memo: &Gives,
    cosigner: &Xpub,
    fp: Fingerprint,
    path: &DerivationPath,
) -> Mark {
    let Some(i) = session.keys.position(fp) else {
        return Mark::Unchecked;
    };
    let gives = match memo.iter().find(|(f, p, _)| *f == fp && p == path) {
        Some((_, _, x)) => *x,
        None => session.keys.xpub_at(c, i, path),
    };
    match gives {
        Some(x) if x.public_key == cosigner.public_key && x.chain_code == cosigner.chain_code => {
            Mark::Yours(path_is_single_account(path))
        }
        _ => Mark::Wrong(Text::fill(
            EN.mscheck_wrong_key,
            &[&fp_text(fp), &crate::register::path_text(path)],
        )),
    }
}

/// `text` in groups of four characters.
pub(crate) fn groups_of_four(text: &str) -> Text {
    let chars: Vec<char> = text.chars().collect();
    let groups: Vec<String> = chars.chunks(4).map(|g| g.iter().collect()).collect();
    Text::of(&groups.join(" "))
}

/// Draws `r`'s rows at `(x, y)`, `w` wide: the shape, repeated keys, each
/// cosigner (number, fingerprint, path, the whole key, its state), the
/// checksum and the key path line; a cosigner whose place is in `seeded`
/// says it is the seed given. A press on a key the person ticks toggles
/// the tick in `of`. Returns the height. The one drawing of these rows:
/// the confirmation sheet's, the register sheet's and Check a multisig
/// setup's Result.
pub(crate) fn draw_rows(
    r: &Rows,
    seeded: &[usize],
    of: KeyBlockOf,
    ui: &mut Ui,
    at: (f32, f32, f32),
) -> f32 {
    draw_rows_with(r, &r.keys, seeded, true, of, ui, at)
}

/// `draw_rows` with `keys` as the cosigners' rows instead of `r`'s own:
/// Check a multisig setup's Result, for a loaded wallet's keys, borrowed.
pub(crate) fn draw_rows_over(
    r: &Rows,
    keys: &[KeyRow],
    seeded: &[usize],
    of: KeyBlockOf,
    ui: &mut Ui,
    at: (f32, f32, f32),
) -> f32 {
    draw_rows_with(r, keys, seeded, true, of, ui, at)
}

/// `draw_rows` without the shape line, for the sheet that states the
/// shape itself, with the order of the keys.
pub(crate) fn draw_rows_without_shape(
    r: &Rows,
    of: KeyBlockOf,
    ui: &mut Ui,
    at: (f32, f32, f32),
) -> f32 {
    draw_rows_with(r, &r.keys, &[], false, of, ui, at)
}

fn draw_rows_with(
    r: &Rows,
    keys: &[KeyRow],
    seeded: &[usize],
    shape: bool,
    of: KeyBlockOf,
    ui: &mut Ui,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    use faraday_ui::sheet::kv;
    let mut cy = if shape {
        ui.wrap_as((x, y, w), BODY_M, Role::OnSurface, &r.shape) + GAP
    } else {
        0.0
    };
    for line in &r.repeats {
        cy += ui.wrap_as((x, y + cy, w), BODY_M, Role::Error, line) + GAP;
    }
    cy += draw_keys(keys, seeded, of, keys.len() > 1, ui, (x, y + cy, w));
    // A typing check, not proof: the quieter tone.
    cy += kv(
        ui,
        x,
        y + cy,
        w,
        EN.mscheck_checksum,
        &r.checksum,
        Role::OnSurfaceVariant,
    );
    if let Some((line, danger)) = &r.key_path {
        let tone = if *danger {
            Role::Error
        } else {
            Role::OnSurface
        };
        cy += ui.wrap_as((x, y + cy, w), BODY_M, tone, line) + GAP;
    }
    cy
}
