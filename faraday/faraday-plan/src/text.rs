//! The plan as the vault keeps it (`docs/VAULT.md` §7, type 11): the
//! answers a line each, `name value`, and the map a `spot` line per spot,
//! written and read back.

use super::*;
use faraday_text::{Text, text};

impl Answers {
    /// The answers as the vault keeps them: a line each, `name value`.
    pub fn to_text(&self) -> Text {
        let bits = |v: &[bool]| -> Text { v.iter().map(|&b| if b { '1' } else { '0' }).collect() };
        let mut out = text!(
            "seeds {}\nplaces {}\nsplit {}\nwallet {}\nsoftware {}\nform {}\nomit {}\n",
            &*bits(&self.seeds),
            self.places,
            u8::from(self.split),
            &*bits(&self.wallet),
            &*bits(&self.software),
            &*bits(&self.form),
            self.omit
        );
        for (v, row) in self.vaults.iter().enumerate() {
            out.push_str(&text!("vault {v} {}\n", &*bits(row)));
        }
        for (v, row) in self.sticks.iter().enumerate() {
            out.push_str(&text!("sticks {v} {}\n", &*bits(row)));
        }
        for (i, row) in self.pass.iter().enumerate() {
            if !row.is_empty() {
                out.push_str(&text!("pass {i} {}\n", &*bits(row)));
            }
        }
        if !self.bare.is_empty() {
            out.push_str("bare ");
            for (n, p) in self.bare.iter().enumerate() {
                if n > 0 {
                    out.push(',');
                }
                out.push_str(&Text::num(*p as u64));
            }
            out.push('\n');
        }
        // The map edited on the chart, a `spot` line per spot: where, and
        // what it holds. Never a name, a date or a mark.
        for spot in self.map.iter().flatten() {
            out.push_str("spot ");
            out.push_str(&at_code(spot.at));
            for (w, _) in &spot.holds {
                out.push(' ');
                out.push_str(&what_code(*w));
            }
            out.push('\n');
        }
        // Whether the board was opened for changes (§19.6).
        if self.custom {
            out.push_str("custom 1\n");
        }
        // A SeedQR drawn in part, at its place (§14.7 item 3).
        for (p, i) in &self.part {
            out.push_str(&text!("part {p} {i}\n"));
        }
        // Each seed's codex32 form (`docs/CODEX32.md` §9.4): never a
        // string.
        for f in &self.codex32 {
            out.push_str(&f.to_line());
        }
        // Each seed's Seed XOR and SLIP-39 sets (F276): public facts
        // only, never a part or a share.
        for x in &self.xor {
            out.push_str(&x.to_line());
        }
        for z in &self.slip39 {
            out.push_str(&z.to_line());
        }
        // The sets rotated away (`docs/CODEX32.md` §9.7): seed, threshold
        // and identifier, never a string.
        for (i, k, id) in &self.rotated {
            out.push_str(&text!(
                "rotated {i} {k} {}\n",
                &*id.iter().map(|&b| char::from(b)).collect::<Text>()
            ));
        }
        out
    }
}

/// A spot as a plan's `spot` line writes it: `place0`, `stick0.1` (stick
/// 1 at place 0), `vault1`, `files`, `software`, `away`.
pub fn at_code(at: At) -> Text {
    match at {
        At::Place(p) => text!("place{p}"),
        At::Stick(p, s) => text!("stick{p}.{s}"),
        At::Vault(v) => text!("vault{v}"),
        At::Files => Text::of("files"),
        At::Software => Text::of("software"),
        At::Away => Text::of("away"),
    }
}

/// [`at_code`] read back.
pub fn at_read(s: &str) -> Option<At> {
    Some(match s {
        "files" => At::Files,
        "software" => At::Software,
        "away" => At::Away,
        _ => {
            if let Some(rest) = s.strip_prefix("stick") {
                let (p, k) = rest.split_once('.')?;
                return Some(At::Stick(p.parse().ok()?, k.parse().ok()?));
            }
            match s.strip_prefix("place") {
                Some(p) => At::Place(p.parse().ok()?),
                None => At::Vault(s.strip_prefix("vault")?.parse().ok()?),
            }
        }
    })
}

/// A thing as a plan's `spot` line writes it: `w0` seed 0's words, `q0`
/// its SeedQR, `p0` its passphrase, `sheet`, `share1`, `stick0` vault 0's
/// stick, `seed0`, `seedp0` with its passphrase, `wallet`, `file0` its
/// words file, `qrfile0` its SeedQR picture, `kef0` and `kefp0` its Krux code
/// on a stick and on paper, `passfile0` its passphrase
/// file, `wallettxt` and `walletqr` the description's text and picture,
/// `sharetxt1` and `shareqr1` share 1's, `softwarefile` the software's.
pub fn what_code(w: What) -> Text {
    let form = |f: usize| if f == form::TEXT { "txt" } else { "qr" };
    match w {
        What::Words(i) => text!("w{i}"),
        What::SeedQr(i) => text!("q{i}"),
        What::Passphrase(i) => text!("p{i}"),
        What::Sheet => Text::of("sheet"),
        What::Share(j) => text!("share{j}"),
        What::VaultStick(v) => text!("stick{v}"),
        What::Seed(i) => text!("seed{i}"),
        What::SeedPassphrase(i) => text!("seedp{i}"),
        What::PassphraseSealed(i) => text!("vpass{i}"),
        What::Wallet => Text::of("wallet"),
        What::SeedFile(i) => text!("file{i}"),
        What::SeedQrFile(i) => text!("qrfile{i}"),
        What::PassphraseFile(i) => text!("passfile{i}"),
        What::WalletFile(f) => text!("wallet{}", form(f)),
        What::ShareFile(j, f) => text!("share{}{j}", form(f)),
        What::SoftwareFile => Text::of("softwarefile"),
        What::Codex32Share(i, j) => text!("x{i}.{j}"),
        What::Codex32QrPaper(i, j) => text!("xq{i}.{j}"),
        What::Codex32ShareFile(i, j) => text!("xt{i}.{j}"),
        What::Codex32QrFile(i, j) => text!("xp{i}.{j}"),
        What::KefCode(i) => text!("kef{i}"),
        What::KefCodePaper(i) => text!("kefp{i}"),
        What::XorPart(i, j) => text!("xorp{i}.{j}"),
        What::Slip39Share(i, g, m) => text!("slip{i}.{g}.{m}"),
    }
}

/// A Seed XOR part, `xorp0.1`, or a SLIP-39 share, `slip0.1.2`, as
/// [`what_code`] writes them (F276).
fn set_piece_read(s: &str) -> Option<What> {
    if let Some(rest) = s.strip_prefix("xorp") {
        let (i, j) = rest.split_once('.')?;
        return Some(What::XorPart(i.parse().ok()?, j.parse().ok()?));
    }
    let mut f = s.strip_prefix("slip")?.split('.');
    let w = What::Slip39Share(
        f.next()?.parse().ok()?,
        f.next()?.parse().ok()?,
        f.next()?.parse().ok()?,
    );
    f.next().is_none().then_some(w)
}

/// [`what_code`] read back. A plan saved before the forms
/// (`docs/BACKUP-FLOW.md` §17.3) wrote `walletfile` and `sharefile1`,
/// which [`Answers::from_text`] reads as every form its `form` answers
/// had on; read alone (a note on the chart), each is its QR picture, the
/// form those answers start with.
pub fn what_read(s: &str) -> Option<What> {
    let n = |p: &str| -> Option<usize> { s.strip_prefix(p)?.parse().ok() };
    // A codex32 string: seed and share index, `x0.10`.
    let two = |p: &str| -> Option<(usize, usize)> {
        let (i, j) = s.strip_prefix(p)?.split_once('.')?;
        Some((i.parse().ok()?, j.parse().ok().filter(|&j: &usize| j < 32)?))
    };
    Some(match s {
        "sheet" => What::Sheet,
        "wallet" => What::Wallet,
        "wallettxt" => What::WalletFile(form::TEXT),
        "walletqr" | "walletfile" => What::WalletFile(form::QR),
        "softwarefile" => What::SoftwareFile,
        _ if s.starts_with("sharefile") => What::ShareFile(n("sharefile")?, form::QR),
        _ if s.starts_with("sharetxt") => What::ShareFile(n("sharetxt")?, form::TEXT),
        _ if s.starts_with("shareqr") => What::ShareFile(n("shareqr")?, form::QR),
        _ if s.starts_with("kefp") => What::KefCodePaper(n("kefp")?),
        _ if s.starts_with("kef") => What::KefCode(n("kef")?),
        _ if s.starts_with("xorp") || s.starts_with("slip") => set_piece_read(s)?,
        _ if s.starts_with("xq") => two("xq").map(|(i, j)| What::Codex32QrPaper(i, j))?,
        _ if s.starts_with("xt") => two("xt").map(|(i, j)| What::Codex32ShareFile(i, j))?,
        _ if s.starts_with("xp") => two("xp").map(|(i, j)| What::Codex32QrFile(i, j))?,
        _ if s.starts_with('x') => two("x").map(|(i, j)| What::Codex32Share(i, j))?,
        _ if s.starts_with("seedp") => What::SeedPassphrase(n("seedp")?),
        _ if s.starts_with("vpass") => What::PassphraseSealed(n("vpass")?),
        _ if s.starts_with("seed") => What::Seed(n("seed")?),
        _ if s.starts_with("share") => What::Share(n("share")?),
        _ if s.starts_with("stick") => What::VaultStick(n("stick")?),
        _ if s.starts_with("qrfile") => What::SeedQrFile(n("qrfile")?),
        _ if s.starts_with("passfile") => What::PassphraseFile(n("passfile")?),
        _ if s.starts_with("file") => What::SeedFile(n("file")?),
        _ if s.starts_with('w') => What::Words(n("w")?),
        _ if s.starts_with('q') => What::SeedQr(n("q")?),
        _ if s.starts_with('p') => What::Passphrase(n("p")?),
        _ => return None,
    })
}

/// How a thing kept at a spot may be read: anything in a vault, and a
/// vault's stick, sealed; the description, a sheet, a share and the
/// software's file public; the rest secret.
pub fn tag_of(at: At, what: What) -> Tag {
    match (at, what) {
        (At::Vault(_), _) | (_, What::VaultStick(_)) => Tag::Sealed,
        (_, What::KefCode(_) | What::KefCodePaper(_)) => Tag::Sealed,
        (
            _,
            What::Sheet
            | What::Share(_)
            | What::Wallet
            | What::WalletFile(_)
            | What::ShareFile(..)
            | What::SoftwareFile,
        ) => Tag::Public,
        _ => Tag::Secret,
    }
}

/// One `seals` line of a plan (`docs/VAULT.md` §7): the seal count and
/// digest this session last saw of another vault,
/// `seals <id> <count> <digest>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SealLine {
    /// The vault's identifier (its root group's UUID, in hex).
    pub id: [u8; 32],
    /// The vault's seal count.
    pub count: u64,
    /// The SHA-256 of the whole file at that count.
    pub digest: [u8; 32],
}

impl SealLine {
    /// The line as the plan keeps it, with its line break.
    pub fn to_line(&self) -> Text {
        text!(
            "seals {} {} {}\n",
            &*Text::from_utf8_lossy(&self.id),
            self.count,
            &*crate::hex32(&self.digest)
        )
    }

    /// One line read back; None for a line that is not a `seals` line or
    /// does not read.
    pub(crate) fn from_line(line: &str) -> Option<SealLine> {
        let mut f = line.strip_prefix("seals ")?.split(' ');
        let id = f.next()?;
        let line = SealLine {
            id: <[u8; 32]>::try_from(id.as_bytes())
                .ok()
                .filter(|b| b.iter().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f')))?,
            count: f.next()?.parse().ok().filter(|c| *c >= 1)?,
            digest: crate::read_hex32(f.next()?)?,
        };
        f.next().is_none().then_some(line)
    }

    /// Whether this line and `other` are for one vault.
    pub fn same_vault(&self, other: &SealLine) -> bool {
        self.id == other.id
    }
}

/// The `seals` lines a plan's text holds, those that read.
pub fn seals_in(text: &str) -> Vec<SealLine> {
    text.lines().filter_map(SealLine::from_line).collect()
}

/// `text` with its `seals` lines replaced by `lines`, at the end.
pub fn with_seals(text: &str, lines: &[SealLine]) -> Text {
    let mut out = Text::new();
    for l in text.lines().filter(|l| !l.starts_with("seals ")) {
        out.push_str(l);
        out.push('\n');
    }
    for l in lines {
        out.push_str(&l.to_line());
    }
    out
}

/// The fingerprints a plan's text lists as made here with their backup
/// pending, one `held` line each.
pub fn held_in(text: &str) -> Vec<Text> {
    text.lines()
        .filter_map(|l| l.strip_prefix("held "))
        .map(|f| Text::of(f.trim()))
        .collect()
}

impl Answers {
    /// Which seeds a stored plan `text` gives a passphrase's places: those
    /// with a non-empty `pass` row, by seed number. A plan read as stored
    /// takes this and not the loaded key's passphrase.
    #[must_use]
    pub fn pass_seeds(text: &str) -> Vec<bool> {
        let mut out: Vec<bool> = Vec::new();
        for line in text.lines() {
            let Some(("pass", value)) = line.split_once(' ') else {
                continue;
            };
            let Some((i, row)) = value.split_once(' ') else {
                continue;
            };
            let Ok(i) = i.parse::<usize>() else {
                continue;
            };
            if i < 4096 {
                if out.len() <= i {
                    out.resize(i + 1, false);
                }
                out[i] = !row.is_empty();
            }
        }
        out
    }

    /// Which seeds these answers give a passphrase's places (a non-empty
    /// row), the same as [`Answers::pass_seeds`] reads of their text.
    #[must_use]
    pub fn pass_flags(&self) -> Vec<bool> {
        self.pass.iter().map(|r| !r.is_empty()).collect()
    }

    /// Reads [`Answers::to_text`] back for `shape`: None when it is not
    /// for a wallet of this shape. A plan saved before vaults were one per
    /// seed, with one `sticks` line and no `vault` line, reads as one
    /// vault holding every seed here, its stick at the places ticked.
    pub fn from_text(shape: &Shape, text: &str) -> Option<Answers> {
        if shape.vaults > crate::MAX_VAULTS {
            return None;
        }
        let mut a = Answers::defaults(shape);
        a.bare = Vec::new();
        let mut seen = 0;
        let mut got = Lines::default();
        for line in text.lines() {
            let (name, value) = line.split_once(' ')?;
            if answer_line(&mut a, name, value, &mut got)? {
                seen += 1;
            } else if !skipped_line(&mut a, name, value) {
                place_line(&mut a, shape, name, value, &mut got)?;
            }
        }
        place_rows(&mut a, shape, got.one_stick, got.sticks, got.vaults)?;
        read_map(&mut a, shape, got.lines)?;
        let fits = seen >= 8
            && a.places >= 1
            && a.sticks.iter().all(|r| r.len() == a.places)
            // A seed not loaded here keeps its ticks and its passphrase's
            // row as saved: it reads as in the plan and absent, and a save
            // writes them back.
            && a.vaults.iter().all(|r| r.len() == shape.seeds.len())
            && a.pass.iter().zip(&shape.seeds).all(|(r, s)| {
                r.is_empty() || ((!s.here || s.passphrase) && r.len() == a.places + 1)
            });
        if !fits {
            return None;
        }
        // A plan saved when the plan could split a seed into codex32
        // strings it was to make: that form and its strings are dropped,
        // the rest of the plan read as written (`docs/CODEX32.md` §9.2).
        // A set that exists stays.
        for i in got.unmade {
            a.set_c32(shape, i, None);
        }
        // A string a spot keeps is one of its seed's set: an index the
        // form does not list would count toward rebuilding the seed.
        if a.map.iter().flatten().any(|s| {
            s.holds.iter().any(|(w, _)| {
                codex32::string_of(*w)
                    .is_some_and(|(i, j)| a.c32(i).is_some_and(|f| !f.indices.contains(&j)))
            })
        }) {
            return None;
        }
        // A set rotated away is a real identifier and not the plan's own
        // (its strings would be taken out at a save); the lines may come
        // before the seed's `codex32` line, so this is checked last.
        if a.rotated.iter().any(|&(i, _, id)| {
            !id.iter().all(|c| codex32::CHARSET.contains(c))
                || a.c32(i)
                    .is_some_and(|f| f.identifier.eq_ignore_ascii_case(&id))
        }) {
            return None;
        }
        Some(a)
    }
}

/// A string of `1` and `0`, as the bools it ticks.
fn flags(s: &str) -> Option<Vec<bool>> {
    s.chars()
        .map(|c| match c {
            '1' => Some(true),
            '0' => Some(false),
            _ => None,
        })
        .collect()
}

/// [`flags`] of exactly `N` of them.
fn bits<const N: usize>(s: &str) -> Option<[bool; N]> {
    let v = flags(s)?;
    v.try_into().ok()
}

/// A row's number and its ticks, `1 0110`.
fn numbered(value: &str) -> Option<(usize, Vec<bool>)> {
    let (v, row) = value.split_once(' ')?;
    Some((v.parse().ok()?, flags(row)?))
}

/// What the lines of a plan give that is read once all of them are.
#[derive(Default)]
struct Lines<'a> {
    one_stick: Option<Vec<bool>>,
    sticks: Vec<(usize, Vec<bool>)>,
    vaults: Vec<(usize, Vec<bool>)>,
    /// Each `spot` line's place and things, read once every answer is:
    /// a thing written before the forms takes the `form` answers'.
    lines: Vec<(At, Vec<&'a str>)>,
    unmade: Vec<usize>,
}

/// An answer line, read into `a` or `got`: whether it was one, None when
/// it is malformed. Another line is not an answer.
fn answer_line(a: &mut Answers, name: &str, value: &str, got: &mut Lines) -> Option<bool> {
    match name {
        "seeds" => a.seeds = bits(value)?,
        "places" => {
            // More than the places a person could ever add (rows
            // above are held to the same count) is not a plan.
            a.places = value.parse().ok().filter(|&n| n <= crate::MAX_PLACES)?;
        }
        "split" => a.split = value == "1",
        "sticks" if value.contains(' ') => got.sticks.push(numbered(value)?),
        "sticks" => got.one_stick = Some(flags(value)?),
        "vault" => got.vaults.push(numbered(value)?),
        "wallet" => a.wallet = bits(value)?,
        "software" => a.software = bits(value)?,
        "form" => a.form = bits(value)?,
        "omit" => a.omit = value.parse().ok()?,
        "bare" => {
            a.bare = value
                .split(',')
                .map(|p| p.parse().ok())
                .collect::<Option<Vec<usize>>>()?;
        }
        "pass" => {
            let (i, row) = value.split_once(' ')?;
            let i: usize = i.parse().ok()?;
            *a.pass.get_mut(i)? = flags(row)?;
        }
        _ => return Some(false),
    }
    Some(true)
}

/// A line that is not an answer and is read past (or into the board's
/// switch), never refused: whether `name` is one.
fn skipped_line(a: &mut Answers, name: &str, value: &str) -> bool {
    match name {
        // A key made here whose backup is pending: read by the
        // caller (`docs/WALLETS.md` §13), not an answer.
        "held" => {}
        // The seal counts of the other vaults this plan places
        // (`docs/VAULT.md` §7): read by the plan
        // area ([`SealLine`]), not an answer.
        "seals" => {}
        // The access layers' count and routes, written only by test
        // builds before the layers were withdrawn (F38, F46): read
        // past, never refused.
        "layers" | "route" => {}
        // The preset a plan saved before F53 started from: read,
        // never refused, and no longer written. A plan from Paper
        // only, Paper and vault, or Paper, vault and watch-only
        // software reopens as it was, customisable
        // (`docs/BACKUP-FLOW.md` §17.21 item 7).
        "preset" => {
            if value != "0" {
                a.custom = true;
            }
        }
        "custom" => a.custom = value == "1",
        _ => return false,
    }
    true
}

/// A line of the places and what they keep, read into `a` or `got`: None
/// when it is malformed or not a line of a plan.
fn place_line<'a>(
    a: &mut Answers,
    shape: &Shape,
    name: &str,
    value: &'a str,
    got: &mut Lines<'a>,
) -> Option<()> {
    match name {
        // A seed's codex32 form (`docs/CODEX32.md` §9.2).
        "codex32" => {
            // A split the plan was to make (identifier `-`), saved
            // when it made one: dropped, below.
            if value.split(' ').nth(4) == Some("-") {
                let seed: usize = value.split(' ').next()?.parse().ok()?;
                if seed >= shape.seeds.len() {
                    return None;
                }
                got.unmade.push(seed);
                return Some(());
            }
            let f = codex32::C32::from_value(value, shape.seeds.len())?;
            if a.c32(f.seed).is_some() {
                return None;
            }
            a.codex32.push(f);
        }
        // A set of a seed rotated away (`docs/CODEX32.md` §9.7).
        "rotated" => {
            let mut parts = value.split(' ');
            let seed: usize = parts.next()?.parse().ok()?;
            let k: u8 = parts.next()?.parse().ok()?;
            let id = <[u8; 4]>::try_from(parts.next()?.as_bytes()).ok()?;
            if seed >= shape.seeds.len() || k == 1 || k > 9 || parts.next().is_some() {
                return None;
            }
            if !a.rotated.contains(&(seed, k, id)) {
                a.rotated.push((seed, k, id));
            }
        }
        // A seed's Seed XOR set (F276).
        "xor" => {
            let f = xor::Xor::from_value(value, shape.seeds.len())?;
            if a.xor(f.seed).is_some() {
                return None;
            }
            a.xor.push(f);
        }
        // A seed's SLIP-39 set (F276).
        "slip39" => {
            let f = slip39::Slip39::from_value(value, shape.seeds.len())?;
            if a.slip39(f.seed).is_some() {
                return None;
            }
            a.slip39.push(f);
        }
        // A SeedQR drawn in part (§14.7 item 3).
        "part" => {
            let (p, i) = value.split_once(' ')?;
            let pi: (usize, usize) = (p.parse().ok()?, i.parse().ok()?);
            if pi.1 >= shape.seeds.len() {
                return None;
            }
            if !a.part.contains(&pi) {
                a.part.push(pi);
            }
        }
        // The map edited on the chart (§9.7).
        "spot" => {
            let mut parts = value.split(' ');
            let at = at_read(parts.next()?)?;
            got.lines
                .push((at, parts.filter(|p| !p.is_empty()).collect()));
        }
        _ => return None,
    }
    Some(())
}

/// The vault and stick rows read, set into `a`.
fn place_rows(
    a: &mut Answers,
    shape: &Shape,
    one_stick: Option<Vec<bool>>,
    sticks: Vec<(usize, Vec<bool>)>,
    vaults: Vec<(usize, Vec<bool>)>,
) -> Option<()> {
    // A plan kept for more vaults than this shape has rows for (the
    // vaults set up before the keys) reads with all of them.
    // A row numbered past what a count can hold is not a plan.
    let seen_rows = |l: &[(usize, Vec<bool>)]| -> Option<usize> {
        l.iter()
            .try_fold(0, |most: usize, (v, _)| Some(most.max(v.checked_add(1)?)))
    };
    let (vault_rows, stick_rows) = (seen_rows(&vaults)?, seen_rows(&sticks)?);
    if vault_rows > crate::MAX_VAULTS || stick_rows > crate::MAX_VAULTS {
        return None;
    }
    if vault_rows > a.vaults.len() {
        a.vaults
            .resize_with(vault_rows, || vec![false; shape.seeds.len()]);
    }
    let rows = shape.vault_rows().max(stick_rows);
    a.sticks = vec![vec![false; a.places]; rows];
    match one_stick {
        // Saved with one vault: it holds every seed here.
        Some(row) if vaults.is_empty() && sticks.is_empty() => {
            for (v, r) in a.vaults.iter_mut().enumerate() {
                r.iter_mut()
                    .enumerate()
                    .for_each(|(i, s)| *s = v == 0 && shape.seeds[i].here);
            }
            a.sticks[0] = row;
        }
        Some(_) => return None,
        None => {
            for (v, row) in sticks {
                *a.sticks.get_mut(v)? = row;
            }
            for (v, row) in vaults {
                *a.vaults.get_mut(v)? = row;
            }
        }
    }
    Some(())
}

/// The map edited on the chart, its spots read from their `spot` lines and
/// set into `a`; None when it is not for a wallet of this shape.
fn read_map(a: &mut Answers, shape: &Shape, lines: Vec<(At, Vec<&str>)>) -> Option<()> {
    let mut edited: Vec<Spot> = Vec::new();
    for (at, parts) in lines {
        let mut holds: Vec<(What, Tag)> = Vec::new();
        for part in parts {
            // A vault's password on paper, placed only by those same
            // test builds: read past.
            if part.starts_with("vpw") {
                continue;
            }
            // The description's or a share's files saved before the
            // forms: every form the `form` answers had on
            // (`docs/BACKUP-FLOW.md` §17.3).
            let whats: Vec<What> = match part {
                "walletfile" => a.file_forms().into_iter().map(What::WalletFile).collect(),
                _ if part.starts_with("sharefile") => {
                    let j: usize = part.strip_prefix("sharefile")?.parse().ok()?;
                    a.file_forms()
                        .into_iter()
                        .map(|f| What::ShareFile(j, f))
                        .collect()
                }
                _ => vec![what_read(part)?],
            };
            for w in whats {
                if !holds.iter().any(|(x, _)| *x == w) {
                    holds.push((w, tag_of(at, w)));
                }
            }
        }
        edited.push(Spot { at, holds });
    }
    if !edited.is_empty() {
        // Its places numbered in order, each stick at one of them with
        // at most one vault on it and nothing but stick forms, and
        // nothing in it a seed, a share or a key the wallet does not
        // have.
        let places: Vec<usize> = edited
            .iter()
            .filter_map(|s| match s.at {
                At::Place(p) => Some(p),
                _ => None,
            })
            .collect();
        let fits_shape = |w: What| match w {
            What::Words(i)
            | What::SeedQr(i)
            | What::Passphrase(i)
            | What::Seed(i)
            | What::SeedPassphrase(i)
            | What::PassphraseSealed(i)
            | What::SeedFile(i)
            | What::SeedQrFile(i)
            | What::PassphraseFile(i) => i < shape.seeds.len(),
            What::Share(j) => j < shape.keys.max(1),
            What::ShareFile(j, f) => j < shape.keys.max(1) && f <= form::TEXT,
            What::WalletFile(f) => f <= form::TEXT,
            What::VaultStick(v) => v < crate::MAX_VAULTS,
            What::Codex32Share(i, _)
            | What::Codex32QrPaper(i, _)
            | What::Codex32ShareFile(i, _)
            | What::Codex32QrFile(i, _)
            | What::KefCode(i)
            | What::KefCodePaper(i)
            | What::XorPart(i, _)
            | What::Slip39Share(i, _, _) => i < shape.seeds.len(),
            _ => true,
        };
        let sticks_ok = edited.iter().all(|s| match s.at {
            At::Stick(p, k) => {
                p < places.len()
                    && k < crate::MAX_STICKS
                    && s.holds.iter().all(|(w, _)| on_stick(*w))
            }
            _ => true,
        });
        // A Seed XOR part or a SLIP-39 share is paper only, at a place,
        // and of a set the plan holds the facts of (F276).
        let sets_ok = edited.iter().all(|s| {
            s.holds.iter().all(|(w, _)| match *w {
                What::XorPart(i, j) => {
                    matches!(s.at, At::Place(_)) && a.xor(i).is_some_and(|x| j < x.n())
                }
                What::Slip39Share(i, g, m) => {
                    matches!(s.at, At::Place(_)) && a.slip39(i).is_some_and(|z| z.has_member(g, m))
                }
                _ => true,
            })
        });
        let ok = places.iter().enumerate().all(|(n, &p)| n == p)
            && sticks_ok
            && sets_ok
            && edited
                .iter()
                .all(|s| !matches!(s.at, At::Vault(v) if v >= crate::MAX_VAULTS))
            && edited
                .iter()
                .all(|s| s.holds.iter().all(|(w, _)| fits_shape(*w)));
        if !ok {
            return None;
        }
        a.set_map(edited);
    }
    Some(())
}

/// [`Note::to_text`] read back, whatever seed it names.
pub(super) fn note_read(s: &str) -> Option<Note> {
    let mut parts = s.splitn(4, ' ');
    let kind = parts.next()?;
    let a = parts.next()?;
    let b = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("");
    let after = |n: usize| Text::of(s.splitn(n + 1, ' ').nth(n).unwrap_or(""));
    Some(match kind {
        "checked" => Note::Checked(at_read(a)?, what_read(b)?, Text::of(rest)),
        "keychecked" => Note::KeyChecked(fp_read(a)?, after(2)),
        "lost" => Note::Marked(at_read(a)?, what_read(b)?, Mark::Lost),
        "exposed" => Note::Marked(at_read(a)?, what_read(b)?, Mark::Exposed),
        "holder" => Note::Holder(fp_read(a)?, after(2)),
        "vaultname" => Note::VaultName(a.parse().ok()?, after(2)),
        // A date alone: anything else is refused.
        "moved"
            if b.is_empty() && rest.is_empty() && faraday_crypto::date::parse_ymd(a).is_some() =>
        {
            Note::Moved(Text::of(a))
        }
        _ => return None,
    })
}

impl What {
    /// The seed it is of, when it is of one.
    pub fn seed_index(self) -> Option<usize> {
        match self {
            What::Words(i)
            | What::SeedQr(i)
            | What::Passphrase(i)
            | What::Seed(i)
            | What::SeedPassphrase(i)
            | What::PassphraseSealed(i)
            | What::SeedFile(i)
            | What::SeedQrFile(i)
            | What::PassphraseFile(i)
            | What::Codex32Share(i, _)
            | What::Codex32QrPaper(i, _)
            | What::Codex32ShareFile(i, _)
            | What::Codex32QrFile(i, _)
            | What::KefCode(i)
            | What::KefCodePaper(i)
            | What::XorPart(i, _)
            | What::Slip39Share(i, _, _) => Some(i),
            _ => None,
        }
    }
}

impl What {
    /// The same thing of seed `map[i]` where it was of seed `i`; None
    /// when `map` has no such seed. A thing not of a seed is itself. A
    /// share of a split's keys is None: which keys it holds depends on
    /// the keys' order, so it cannot be moved to another order.
    pub(crate) fn renumbered(self, map: &[usize]) -> Option<What> {
        if matches!(self, What::Share(_) | What::ShareFile(..)) {
            return None;
        }
        let Some(i) = self.seed_index() else {
            return Some(self);
        };
        let n = *map.get(i)?;
        Some(match self {
            What::Words(_) => What::Words(n),
            What::SeedQr(_) => What::SeedQr(n),
            What::Passphrase(_) => What::Passphrase(n),
            What::Seed(_) => What::Seed(n),
            What::SeedPassphrase(_) => What::SeedPassphrase(n),
            What::PassphraseSealed(_) => What::PassphraseSealed(n),
            What::SeedFile(_) => What::SeedFile(n),
            What::SeedQrFile(_) => What::SeedQrFile(n),
            What::PassphraseFile(_) => What::PassphraseFile(n),
            What::Codex32Share(_, j) => What::Codex32Share(n, j),
            What::Codex32QrPaper(_, j) => What::Codex32QrPaper(n, j),
            What::Codex32ShareFile(_, j) => What::Codex32ShareFile(n, j),
            What::Codex32QrFile(_, j) => What::Codex32QrFile(n, j),
            What::KefCode(_) => What::KefCode(n),
            What::KefCodePaper(_) => What::KefCodePaper(n),
            What::XorPart(_, j) => What::XorPart(n, j),
            What::Slip39Share(_, g, m) => What::Slip39Share(n, g, m),
            other => other,
        })
    }
}

impl Note {
    /// The note with the seed it is of numbered `map[i]` for seed `i`
    /// (a plan's seeds are numbered by the wallet's key order). None when
    /// `map` has no such seed.
    pub fn renumbered(&self, map: &[usize]) -> Option<Note> {
        Some(match self {
            Note::Checked(at, w, d) => Note::Checked(*at, w.renumbered(map)?, d.clone()),
            Note::Marked(at, w, m) => Note::Marked(*at, w.renumbered(map)?, *m),
            other => other.clone(),
        })
    }
}

/// A plan's text with every seed number `i` written `map[i]` instead:
/// the text of a plan saved for a wallet whose keys were in another
/// order, read for the wallet in this one (`map` says where each seed
/// went). None when a seed or a row has no place in `map`, and when the
/// plan holds shares of a split (the `split` answer, a share on a spot):
/// its shares depend on the keys' order, which this does not keep.
pub fn renumber_seeds(text: &str, map: &[usize]) -> Option<Text> {
    let seed = |s: &str| -> Option<usize> { map.get(s.parse::<usize>().ok()?).copied() };
    let mut out = Text::new();
    for line in text.lines() {
        let (name, value) = line.split_once(' ')?;
        let new: Text = match name {
            "split" if value != "0" => return None,
            // A flag per seed.
            "vault" => {
                let (v, row) = value.split_once(' ')?;
                if row.len() != map.len() {
                    return None;
                }
                let mut flags = vec!['0'; map.len()];
                for (c, &to) in row.chars().zip(map) {
                    *flags.get_mut(to)? = c;
                }
                text!("vault {v} {}", &*flags.into_iter().collect::<Text>())
            }
            // The seed, then what is not one.
            "pass" | "codex32" | "xor" | "slip39" | "rotated" => {
                let (i, rest) = value.split_once(' ')?;
                text!("{name} {} {rest}", seed(i)?)
            }
            "part" => {
                let (p, i) = value.split_once(' ')?;
                text!("part {p} {}", seed(i)?)
            }
            "spot" => {
                let mut out = Text::of("spot");
                for (n, t) in value.split(' ').enumerate() {
                    out.push(' ');
                    match what_read(t).filter(|_| n > 0) {
                        Some(w)
                            if w.seed_index().is_some()
                                || matches!(w, What::Share(_) | What::ShareFile(..)) =>
                        {
                            out.push_str(&what_code(w.renumbered(map)?));
                        }
                        _ => out.push_str(t),
                    }
                }
                out
            }
            _ => Text::of(line),
        };
        out.push_str(&new);
        out.push('\n');
    }
    Some(out)
}

/// A fingerprint as a plan's note writes it: eight lower-case hex digits.
pub(super) fn fp_code(fp: [u8; 4]) -> Text {
    faraday_crypto::hex::encode_string(&fp)
}

/// A fingerprint a plan's note wrote, read back.
pub(super) fn fp_read(s: &str) -> Option<[u8; 4]> {
    faraday_crypto::hex::decode_exact(s)
}

/// The line said in place of the checklist when a seed here is past the
/// checklist's numbers (`CHECKLIST_SEEDS`): it would have no item of
/// its own, so no checklist is made. A lookup.
pub(super) fn past_checklist(shape: &Shape) -> Option<Text> {
    let i = shape
        .here()
        .into_iter()
        .find(|&i| i >= crate::CHECKLIST_SEEDS)?;
    Some(Text::fill(
        faraday_strings::EN.backup_seed_past_checklist,
        &[
            &Text::num(i as u64 + 1),
            &Text::num(crate::CHECKLIST_SEEDS as u64),
        ],
    ))
}
