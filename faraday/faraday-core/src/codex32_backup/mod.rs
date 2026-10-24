//! Codex32 strings in a backup (`docs/CODEX32.md` §9): the checklist's
//! item for a seed kept as codex32 strings, and the set's actions.
//!
//! - **A set the plan makes** (§9.2): **Make them** splits the seed with
//!   this device's randomness (`faraday-keys`, `Keys::codex32_shares`),
//!   and each string placed on paper is shown, one at a time, and typed
//!   back on the codex32 keyboard ("Box 17 differs") or skipped.
//! - **A set that exists** (§9.4): nothing is made; **Check my copy**
//!   takes each string placed, typed back from the paper, and checks its
//!   checksum and its header against the set's facts; once enough are
//!   typed, that they rebuild this seed.
//! - **The set's actions** (§9.7): Add a share and Replace a lost share
//!   from enough strings typed in; Rotate the set, the old strings marked
//!   and the held ones destroyed first, then a new set of the same seed.
//!
//! What is here is the item's state and its rules, which read only what
//! they are given; `screen.rs` draws it, and the glue that reaches the
//! keys and the plan is on `Faraday` (`backups/mod.rs`). The strings are
//! secret: they live in this state as `faraday-keys` lends them, a paper
//! form (`Reveal::PaperForm`), and go with the backup.

pub(crate) mod screen;

use faraday_bip::codex32::Codex32;
use faraday_ui::keyboard::OskPress;
use zeroize::Zeroizing;

use crate::plan::codex32::{C32, index_char};
use crate::strings::{EN, fill};

/// A press on the codex32 item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    /// **Make them**: the set the plan makes, split now.
    Make,
    /// The string at this place among the item's opened, to show or to
    /// check.
    Open(u8),
    /// A key of the codex32 keyboard.
    Key(OskPress),
    /// **String** · **Codex32QR**, at 48 characters.
    Qr,
    /// **Continue**: the string typed back, the next one.
    Next,
    /// **Skip**: a first press asks, a second goes on.
    Skip,
    /// Back to the item's list of strings.
    List,
    /// A set action (§9.7).
    Set(SetPress),
    /// The copies by hand's **codex32** (§9.1): its choice of `k` and `n`
    /// opened, at 2 of 3.
    PaperOpen,
    /// That choice's threshold: 0 (the secret alone) or 2 to 9.
    PaperK(u8),
    /// That choice's strings, one more or one fewer.
    PaperN(i8),
}

/// The copies by hand's codex32 choice (§9.1) after `p`, from `now`
/// (`k`, `n`): a threshold of 0 or 2 to 9, at 0 one string, else `k`
/// to 31.
pub fn paper_choice(now: Option<(u8, u8)>, p: Press) -> Option<(u8, u8)> {
    use faraday_bip::codex32::{MAX_SHARES, MAX_THRESHOLD};
    let (k, n) = now.unwrap_or((2, 3));
    Some(match p {
        Press::PaperOpen => (2, 3),
        Press::PaperK(0) => (0, 1),
        Press::PaperK(k) => {
            let k = k.clamp(2, MAX_THRESHOLD);
            (k, n.max(k).clamp(3, MAX_SHARES))
        }
        Press::PaperN(d) if k >= 2 => (k, n.saturating_add_signed(d).clamp(k, MAX_SHARES)),
        _ => return now,
    })
}

/// A press in the set's actions (§9.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetPress {
    /// **Add a share**: strings typed in, then a new index.
    Add,
    /// **Replace a lost share** at this index (a bech32 value).
    Replace(u8),
    /// **Rotate the set**.
    Rotate,
    /// A string of the old set marked: held (0), lost (1) or exposed (2).
    Mark(u8, u8),
    /// The marks are done: what to destroy is said.
    Counted,
    /// A held string confirmed destroyed.
    Destroyed(u8),
    /// The new set's threshold, one more or one fewer.
    K(i8),
    /// The new set's strings, one more or one fewer.
    N(i8),
    /// **Make the new set**.
    MakeNew,
    /// The action left, nothing changed.
    Leave,
}

/// What a string typed back was found to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// The same as the string made here, or a string of the set at the
    /// index wanted, its checksum holding.
    Holds(String),
    /// Something to look at again, said.
    Differs(String),
}

/// The set action under way (§9.7).
#[derive(Debug, Default, PartialEq, Eq)]
pub enum SetStep {
    /// None.
    #[default]
    None,
    /// Strings of the set typed in, towards a new one at `index` (`None`:
    /// the next unused one).
    Typing {
        /// The index to make, as a bech32 value; `None` for the next
        /// unused one.
        index: Option<u8>,
    },
    /// The old strings marked, each held, lost or exposed, by index.
    Marking(Vec<(usize, u8)>),
    /// The held strings to destroy, each confirmed or not, and the
    /// count of old strings that may be out of reach.
    Destroying {
        /// The held strings, by index, and whether each is destroyed.
        held: Vec<(usize, bool)>,
        /// Strings marked lost or exposed.
        out: usize,
    },
    /// The new set: its threshold and strings.
    Scheme(u8, u8),
}

/// The checklist's codex32 item for one seed, while the backup is open.
#[derive(Default)]
pub struct Strings {
    /// The seed, by its place among the wallet's.
    pub seed: usize,
    /// The strings made here (a set the plan made, a share added, a new
    /// set), as `faraday-keys` lent them: each with its index (a bech32
    /// value). Kept while the backup is open.
    made: Vec<(usize, Zeroizing<String>)>,
    /// The string open: its index (a bech32 value).
    pub at: Option<usize>,
    /// Typed back, after `ms1`.
    pub typed: Zeroizing<String>,
    /// What the string typed back was found to be.
    pub found: Option<Found>,
    /// Skip pressed once: the caution is shown.
    pub skip_ask: bool,
    /// The open string's Codex32QR, while shown (48 characters).
    pub qr: Option<faraday_codec::qr::QrMatrix>,
    /// The strings typed back that held, for the check that enough of
    /// them rebuild the seed.
    pub held: Vec<Zeroizing<String>>,
    /// Whether the strings typed back rebuild this seed, once enough are.
    pub rebuilds: Option<bool>,
    /// A set action under way.
    pub set: SetStep,
    /// A line said under the item.
    pub note: Option<String>,
}

impl Strings {
    /// The item for seed `seed`, nothing open.
    pub fn new(seed: usize) -> Strings {
        Strings {
            seed,
            ..Strings::default()
        }
    }

    /// The string made here at index `j`, while the backup is open.
    pub fn made(&self, j: usize) -> Option<&str> {
        self.made
            .iter()
            .find(|(x, _)| *x == j)
            .map(|(_, s)| s.as_str())
    }

    /// Whether any string was made here.
    pub fn any_made(&self) -> bool {
        !self.made.is_empty()
    }

    /// Keeps the strings made, each with its index; a string already kept
    /// at an index is replaced.
    pub fn keep(&mut self, strings: Vec<(usize, Zeroizing<String>)>) {
        for (j, s) in strings {
            self.made.retain(|(x, _)| *x != j);
            self.made.push((j, s));
        }
    }

    /// Opens string `j`: shown (where made here) and typed back afresh.
    pub fn open(&mut self, j: usize) {
        self.at = Some(j);
        self.typed.clear();
        self.found = None;
        self.skip_ask = false;
        self.qr = None;
        self.note = None;
    }

    /// Back to the list of strings.
    pub fn list(&mut self) {
        self.at = None;
        self.typed.clear();
        self.found = None;
        self.skip_ask = false;
        self.qr = None;
    }

    /// A key of the codex32 keyboard while a string of the set is typed
    /// in for a set action, or the new set's identifier is typed: the
    /// character goes in, or the last comes out. A string of the set
    /// typed whole, its checksum and header holding, is returned to be
    /// counted; one already in is not counted twice.
    pub fn type_in(&mut self, form: &C32, press: OskPress) -> Option<String> {
        let room = match self.set {
            SetStep::Typing { .. } => usize::from(form.length).saturating_sub(3),
            SetStep::Scheme(..) => 4,
            _ => return None,
        };
        match press {
            OskPress::Char(ch) => {
                let ch = ch.to_ascii_lowercase();
                if self.typed.len() >= room
                    || !ch.is_ascii()
                    || !crate::plan::codex32::CHARSET.contains(&(ch as u8))
                {
                    return None;
                }
                self.typed.push(ch);
            }
            OskPress::Back => {
                self.typed.pop();
                self.found = None;
                return None;
            }
            _ => return None,
        }
        if self.typed.len() < room || matches!(self.set, SetStep::Scheme(..)) {
            return None;
        }
        let mut whole = Zeroizing::new(String::from("ms1"));
        whole.push_str(&self.typed);
        let index = Codex32::parse(&whole)
            .ok()
            .and_then(|s| crate::plan::codex32::index_of(s.share_index()));
        let found = match index {
            Some(j) => check(form, j, &whole, None),
            None => Found::Differs(EN.c32_checksum_fails.to_string()),
        };
        let holds = matches!(found, Found::Holds(_));
        self.found = Some(found);
        if !holds {
            return None;
        }
        self.typed.clear();
        let again = self
            .held
            .iter()
            .any(|h| h.as_bytes().get(8) == whole.as_bytes().get(8));
        (!again).then(|| whole.to_string())
    }

    /// A key of the codex32 keyboard while string `j` of `form` is open:
    /// the character goes in, or the last comes out; at the string's last
    /// box it is checked ([`check`]). Returns the string typed whole once
    /// it holds, to be counted towards the rebuild.
    pub fn key(&mut self, form: &C32, press: OskPress) -> Option<String> {
        let room = usize::from(form.length).saturating_sub(3);
        match press {
            OskPress::Char(ch) => {
                let ch = ch.to_ascii_lowercase();
                if self.typed.len() >= room
                    || !crate::plan::codex32::CHARSET.contains(&(ch as u8))
                    || !ch.is_ascii()
                {
                    return None;
                }
                self.typed.push(ch);
                self.skip_ask = false;
            }
            OskPress::Back => {
                self.typed.pop();
                self.found = None;
                return None;
            }
            _ => return None,
        }
        if self.typed.len() < room {
            return None;
        }
        let j = self.at?;
        let mut whole = Zeroizing::new(String::from("ms1"));
        whole.push_str(&self.typed);
        let found = check(form, j, &whole, self.made(j));
        let holds = matches!(found, Found::Holds(_));
        self.found = Some(found);
        holds.then(|| whole.to_string())
    }
}

impl Strings {
    /// A held string of the old set confirmed destroyed (Rotate the set,
    /// `docs/CODEX32.md` §9.7 step 3): once every one is, the new set's
    /// scheme is asked, the old one's ticked. Two live sets never exist
    /// at once by the flow's doing.
    pub fn destroyed(&mut self, form: &C32, j: usize) {
        let SetStep::Destroying { held, .. } = &mut self.set else {
            return;
        };
        if let Some(x) = held.iter_mut().find(|(x, _)| *x == j) {
            x.1 = true;
        }
        if held.iter().all(|(_, d)| *d) {
            self.note = None;
            self.typed.clear();
            self.set = SetStep::Scheme(form.k, u8::try_from(form.n()).unwrap_or(form.k));
        }
    }

    /// The new set's threshold (`k`) or strings, one more or one fewer:
    /// a threshold of 2 to 9, strings from it to 31.
    pub fn step_scheme(&mut self, k_not_n: bool, d: i8) {
        use faraday_bip::codex32::{MAX_SHARES, MAX_THRESHOLD};
        if let SetStep::Scheme(k, n) = &mut self.set {
            if k_not_n {
                *k = k.saturating_add_signed(d).clamp(2, MAX_THRESHOLD);
                *n = (*n).max(*k);
            } else {
                *n = n.saturating_add_signed(d).clamp(*k, MAX_SHARES);
            }
        }
    }
}

/// What string `typed`, typed back for index `j` of `form`, is: the box
/// that differs from the string made here where there is one; else its
/// checksum, and its header against the set's facts (`docs/CODEX32.md`
/// §9.4): threshold, identifier, and the index wanted.
pub fn check(form: &C32, j: usize, typed: &str, made: Option<&str>) -> Found {
    let id = form.id_text().unwrap_or_default();
    let index = index_char(j).to_ascii_uppercase().to_string();
    if let Some(want) = made {
        return match want.bytes().zip(typed.bytes()).position(|(a, b)| a != b) {
            Some(p) => Found::Differs(fill(EN.codex32_box_differs, &[&(p + 1).to_string()])),
            None => Found::Holds(fill(EN.c32_holds, &[&index, &id])),
        };
    }
    let Ok(s) = Codex32::parse(typed) else {
        return Found::Differs(EN.c32_checksum_fails.to_string());
    };
    // Box 4 is the threshold, 5 to 8 the identifier, 9 the index.
    let header: [(bool, usize); 3] = [
        (s.threshold() == form.k, 4),
        (Some(s.identifier()) == form.identifier, 5),
        (
            usize::from(s.share_index()) == usize::from(index_char(j) as u8),
            9,
        ),
    ];
    match header.iter().find(|(ok, _)| !ok) {
        Some(&(_, b)) => Found::Differs(fill(EN.c32_not_of_set, &[&b.to_string(), &id])),
        None => Found::Holds(fill(EN.c32_holds, &[&index, &id])),
    }
}

/// What rotating a set says once its old strings are marked
/// (`docs/CODEX32.md` §9.7 step 2): `marks` are each old string's index
/// and mark (0 held, 1 lost, 2 exposed). `Ok`: the held strings to
/// destroy, and the line; `Err`: the line that says rotating is not
/// enough.
pub fn rotation(form: &C32, marks: &[(usize, u8)]) -> Result<(Vec<usize>, String), String> {
    let id = form.id_text().unwrap_or_default();
    let out: Vec<usize> = marks
        .iter()
        .filter(|(_, m)| *m != 0)
        .map(|&(j, _)| j)
        .collect();
    let held: Vec<usize> = marks
        .iter()
        .filter(|(_, m)| *m == 0)
        .map(|&(j, _)| j)
        .collect();
    // Strings whose index is not known may be anywhere.
    let maybe = out.len() + usize::from(form.unknown);
    if maybe >= form.need() {
        return Err(fill(EN.c32_rotate_not_enough, &[&maybe.to_string(), &id]));
    }
    let names = |list: &[usize]| -> String {
        let v: Vec<String> = list
            .iter()
            .map(|&j| index_char(j).to_ascii_uppercase().to_string())
            .collect();
        v.join(", ")
    };
    let line = if out.is_empty() {
        fill(EN.c32_rotate_destroy_all, &[&names(&held)])
    } else {
        fill(
            EN.c32_rotate_destroy,
            &[
                &names(&held),
                &names(&out),
                &form.need().to_string(),
                &form.n().to_string(),
            ],
        )
    };
    Ok((held, line))
}

/// The next share index a set has not used: BIP 93's order, past every
/// index the set has.
pub fn next_index(form: &C32) -> Option<usize> {
    faraday_bip::codex32::INDEX_ORDER
        .iter()
        .filter_map(|&c| crate::plan::codex32::index_of(c))
        .find(|j| !form.indices.contains(j))
}

/// A share index typed as its character, read: lower case bech32, not the
/// secret's.
pub fn index_typed(c: char) -> Option<usize> {
    crate::plan::codex32::index_of(u8::try_from(c).ok()?)
        .filter(|&j| j != crate::plan::codex32::SECRET)
}
