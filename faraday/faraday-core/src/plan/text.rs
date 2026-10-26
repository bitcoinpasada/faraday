//! The plan as the vault keeps it (`docs/VAULT.md` §7, type 11): the
//! answers a line each, `name value`, and the map a `spot` line per spot,
//! written and read back.

use super::*;

impl Answers {
    /// The answers as the vault keeps them: a line each, `name value`.
    pub fn to_text(&self) -> String {
        let bits = |v: &[bool]| {
            v.iter()
                .map(|&b| if b { '1' } else { '0' })
                .collect::<String>()
        };
        let mut out = format!(
            "seeds {}\nplaces {}\nsplit {}\nwallet {}\nsoftware {}\nform {}\nomit {}\n",
            bits(&self.seeds),
            self.places,
            u8::from(self.split),
            bits(&self.wallet),
            bits(&self.software),
            bits(&self.form),
            self.omit
        );
        for (v, row) in self.vaults.iter().enumerate() {
            out.push_str(&format!("vault {v} {}\n", bits(row)));
        }
        for (v, row) in self.sticks.iter().enumerate() {
            out.push_str(&format!("sticks {v} {}\n", bits(row)));
        }
        for (i, row) in self.pass.iter().enumerate() {
            if !row.is_empty() {
                out.push_str(&format!("pass {i} {}\n", bits(row)));
            }
        }
        if !self.bare.is_empty() {
            let list: Vec<String> = self.bare.iter().map(|p| p.to_string()).collect();
            out.push_str(&format!("bare {}\n", list.join(",")));
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
            out.push_str(&format!("part {p} {i}\n"));
        }
        // Each seed's codex32 form (`docs/CODEX32.md` §9.4): never a
        // string.
        for f in &self.codex32 {
            out.push_str(&f.to_line());
        }
        out
    }
}

/// A spot as a plan's `spot` line writes it: `place0`, `stick0.1` (stick
/// 1 at place 0), `vault1`, `files`, `software`, `away`.
pub fn at_code(at: At) -> String {
    match at {
        At::Place(p) => format!("place{p}"),
        At::Stick(p, s) => format!("stick{p}.{s}"),
        At::Vault(v) => format!("vault{v}"),
        At::Files => "files".to_string(),
        At::Software => "software".to_string(),
        At::Away => "away".to_string(),
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
/// words file, `qrfile0` its SeedQR picture, `passfile0` its passphrase
/// file, `wallettxt` and `walletqr` the description's text and picture,
/// `sharetxt1` and `shareqr1` share 1's, `softwarefile` the software's.
pub fn what_code(w: What) -> String {
    let form = |f: usize| if f == form::TEXT { "txt" } else { "qr" };
    match w {
        What::Words(i) => format!("w{i}"),
        What::SeedQr(i) => format!("q{i}"),
        What::Passphrase(i) => format!("p{i}"),
        What::Sheet => "sheet".to_string(),
        What::Share(j) => format!("share{j}"),
        What::VaultStick(v) => format!("stick{v}"),
        What::Seed(i) => format!("seed{i}"),
        What::SeedPassphrase(i) => format!("seedp{i}"),
        What::PassphraseSealed(i) => format!("vpass{i}"),
        What::Wallet => "wallet".to_string(),
        What::SeedFile(i) => format!("file{i}"),
        What::SeedQrFile(i) => format!("qrfile{i}"),
        What::PassphraseFile(i) => format!("passfile{i}"),
        What::WalletFile(f) => format!("wallet{}", form(f)),
        What::ShareFile(j, f) => format!("share{}{j}", form(f)),
        What::SoftwareFile => "softwarefile".to_string(),
        What::Codex32Share(i, j) => format!("x{i}.{j}"),
        What::Codex32QrPaper(i, j) => format!("xq{i}.{j}"),
        What::Codex32ShareFile(i, j) => format!("xt{i}.{j}"),
        What::Codex32QrFile(i, j) => format!("xp{i}.{j}"),
    }
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

/// The fingerprints a plan's text lists as made here with their backup
/// pending, one `held` line each.
pub fn held_in(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| l.strip_prefix("held "))
        .map(|f| f.trim().to_string())
        .collect()
}

impl Answers {
    /// Reads [`Answers::to_text`] back for `shape`: None when it is not
    /// for a wallet of this shape. A plan saved before vaults were one per
    /// seed, with one `sticks` line and no `vault` line, reads as one
    /// vault holding every seed here, its stick at the places ticked.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub fn from_text(shape: &Shape, text: &str) -> Option<Answers> {
        fn bits<const N: usize>(s: &str) -> Option<[bool; N]> {
            let v = flags(s)?;
            v.try_into().ok()
        }
        fn flags(s: &str) -> Option<Vec<bool>> {
            s.chars()
                .map(|c| match c {
                    '1' => Some(true),
                    '0' => Some(false),
                    _ => None,
                })
                .collect()
        }
        let mut a = Answers::defaults(shape);
        a.bare = Vec::new();
        let mut seen = 0;
        let mut one_stick: Option<Vec<bool>> = None;
        let mut sticks: Vec<(usize, Vec<bool>)> = Vec::new();
        let mut vaults: Vec<(usize, Vec<bool>)> = Vec::new();
        // Each `spot` line's place and things, read once every answer is:
        // a thing written before the forms takes the `form` answers'.
        let mut lines: Vec<(At, Vec<&str>)> = Vec::new();
        let mut unmade: Vec<usize> = Vec::new();
        let numbered = |value: &str| -> Option<(usize, Vec<bool>)> {
            let (v, row) = value.split_once(' ')?;
            Some((v.parse().ok()?, flags(row)?))
        };
        for line in text.lines() {
            let (name, value) = line.split_once(' ')?;
            match name {
                "seeds" => a.seeds = bits(value)?,
                "places" => a.places = value.parse().ok()?,
                "split" => a.split = value == "1",
                "sticks" if value.contains(' ') => sticks.push(numbered(value)?),
                "sticks" => one_stick = Some(flags(value)?),
                "vault" => vaults.push(numbered(value)?),
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
                // A key made here whose backup is pending: read by the
                // caller (`docs/history/NEW-WALLET.md` §14.3), not an answer.
                "held" => continue,
                // The access layers' count and routes, written only by test
                // builds before the layers were withdrawn (F38, F46): read
                // past, never refused.
                "layers" | "route" => continue,
                // The preset a plan saved before F53 started from: read,
                // never refused, and no longer written. A plan from Paper
                // only, Paper and vault, or Paper, vault and watch-only
                // software reopens as it was, customisable
                // (`docs/BACKUP-FLOW.md` §17.21 item 7).
                "preset" => {
                    if value != "0" {
                        a.custom = true;
                    }
                    continue;
                }
                "custom" => {
                    a.custom = value == "1";
                    continue;
                }
                // A seed's codex32 form (`docs/CODEX32.md` §9.2).
                "codex32" => {
                    // A split the plan was to make (identifier `-`), saved
                    // when it made one: dropped, below.
                    if value.split(' ').nth(4) == Some("-") {
                        let seed: usize = value.split(' ').next()?.parse().ok()?;
                        if seed >= shape.seeds.len() {
                            return None;
                        }
                        unmade.push(seed);
                        continue;
                    }
                    let f = codex32::C32::from_value(value, shape.seeds.len())?;
                    if a.c32(f.seed).is_some() {
                        return None;
                    }
                    a.codex32.push(f);
                    continue;
                }
                // A SeedQR drawn in part (§14.7 item 3).
                "part" => {
                    let (p, i) = value.split_once(' ')?;
                    let pi = (p.parse().ok()?, i.parse().ok()?);
                    if !a.part.contains(&pi) {
                        a.part.push(pi);
                    }
                    continue;
                }
                // The map edited on the chart (§9.7).
                "spot" => {
                    let mut parts = value.split(' ');
                    let at = at_read(parts.next()?)?;
                    lines.push((at, parts.filter(|p| !p.is_empty()).collect()));
                    continue;
                }
                _ => return None,
            }
            seen += 1;
        }
        // A plan kept for more vaults than this shape has rows for (the
        // vaults set up before the keys) reads with all of them.
        let seen_rows = |l: &[(usize, Vec<bool>)]| l.iter().map(|(v, _)| v + 1).max().unwrap_or(0);
        if seen_rows(&vaults).max(seen_rows(&sticks)) > 1024 {
            return None;
        }
        if seen_rows(&vaults) > a.vaults.len() {
            a.vaults
                .resize_with(seen_rows(&vaults), || vec![false; shape.seeds.len()]);
        }
        let rows = shape.vault_rows().max(seen_rows(&sticks));
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
                What::VaultStick(v) => v < 1024,
                What::Codex32Share(i, _)
                | What::Codex32QrPaper(i, _)
                | What::Codex32ShareFile(i, _)
                | What::Codex32QrFile(i, _) => i < shape.seeds.len(),
                _ => true,
            };
            let sticks_ok = edited.iter().all(|s| match s.at {
                At::Stick(p, k) => {
                    p < places.len()
                        && k < 1024
                        && s.holds.iter().all(|(w, _)| on_stick(*w))
                        && s.holds
                            .iter()
                            .filter(|(w, _)| matches!(w, What::VaultStick(_)))
                            .count()
                            <= 1
                }
                _ => true,
            });
            let ok = places.iter().enumerate().all(|(n, &p)| n == p)
                && sticks_ok
                && edited
                    .iter()
                    .all(|s| !matches!(s.at, At::Vault(v) if v >= 1024))
                && edited
                    .iter()
                    .all(|s| s.holds.iter().all(|(w, _)| fits_shape(*w)));
            if !ok {
                return None;
            }
            a.set_map(edited);
        }
        let fits = seen >= 8
            && a.places >= 1
            && a.sticks.iter().all(|r| r.len() == a.places)
            && a.vaults.iter().all(|r| {
                r.len() == shape.seeds.len()
                    && r.iter().zip(&shape.seeds).all(|(&t, s)| !t || s.here)
            })
            && a.pass
                .iter()
                .zip(&shape.seeds)
                .all(|(r, s)| r.is_empty() || (s.here && s.passphrase && r.len() == a.places + 1));
        if !fits {
            return None;
        }
        // A plan saved when the plan could split a seed into codex32
        // strings it was to make: that form and its strings are dropped,
        // the rest of the plan read as written (`docs/CODEX32.md` §9.2).
        // A set that exists stays.
        for i in unmade {
            a.set_c32(shape, i, None);
        }
        Some(a)
    }
}
