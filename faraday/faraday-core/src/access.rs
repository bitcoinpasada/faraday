//! Access layers (`docs/BACKUP-FLOW.md` §20, F38): who may spend, by
//! which routes, and what a stranger who finds places gets.
//!
//! An access layer has one or more routes; a route is a list of places,
//! any `k` of which are enough, with the vaults' passwords and the
//! passphrases the layer knows on it (§20.1, §20.9). A route is met when
//! every `k` of its places, with what it knows, rebuild the wallet and
//! spend. The stranger knows nothing and finds places (§20.2).
//!
//! Everything here reads the plan's map once ([`Reach`]) and then works
//! with bit masks over its places, seeds, shares and vaults: no
//! elliptic-curve work, no file, no text. Up to 64 of each are counted;
//! a plan has a handful.
//!
//! A layer's name says who and where, so it is kept only in a vault's
//! plan record (field [`field::PLAN_LAYER`]), beside the places' names:
//! never in the plan's text, which goes on a stick and into the flow's
//! kept state.

use faraday_vault::records::{Record, field};

use crate::backup::split_plan;
use crate::plan::{Answers, At, Shape, Spot, What, kept_vaults, map, on_stick};
use crate::strings::EN;
use opensigner_core::strings::fill;

/// One way a layer reaches the wallet (§20.9).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Route {
    /// The places it may draw on, by number from zero.
    pub places: Vec<usize>,
    /// How many of them are enough: "any `any` of" [`Route::places`].
    pub any: usize,
    /// The vaults whose passwords the layer knows on this route.
    pub passwords: Vec<usize>,
    /// The seeds whose BIP-39 passphrases the layer knows on this route.
    pub passphrases: Vec<usize>,
    /// A paper route (§20.12 item 3): it counts only what its places keep
    /// on paper (a vault's password written down among it), with what the
    /// layer remembers; no stick, so no vault opens on it.
    pub paper: bool,
    /// The vaults whose passwords the layer has written down rather than
    /// remembered (§20.9): Work it out keeps "Vault n's password" on paper
    /// at one of the route's places. Not in [`Route::passwords`].
    pub written: Vec<usize>,
    /// What each of its places holds (§20.13 item 1); `None`, Work it out
    /// chooses, as for a route saved before the kinds.
    pub kinds: Option<Kinds>,
}

/// What every place of a route holds (§20.13 item 1), each at least:
/// Work it out keeps that much there and may keep more where another
/// route needs it. The seeds are counted, never named: which key's seed
/// goes where is Work it out's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kinds {
    /// How many of the wallet's seeds each place holds, each a different
    /// seed.
    pub seeds: usize,
    /// The form they are kept in.
    pub form: SeedForm,
    /// How much of the wallet's description each place holds.
    pub description: Described,
    /// Whether each place holds a vault's stick, and how the route opens it.
    pub vault: KindVault,
}

/// The form a route's seeds take at each of its places.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedForm {
    /// Words on paper.
    Paper,
    /// A file on a stick.
    Stick,
}

/// How much of the description a route's place holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Described {
    /// None of it.
    None,
    /// A part: one of the board's description shares; any `k` of the
    /// route's places together hold every key.
    Part,
    /// The whole description.
    Whole,
}

/// A vault at each of a route's places (never on a paper route).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindVault {
    /// No vault.
    None,
    /// A stick of a vault whose password the route remembers
    /// ([`Route::passwords`]).
    Remembered,
    /// A stick of a vault whose password the route has written down
    /// ([`Route::written`]), the password on paper at one of its places.
    Written,
}

impl Kinds {
    /// The kinds a route starts with when they are first set: one seed in
    /// words on paper and a part of the description at each place, no
    /// vault. A lookup.
    pub fn start(route: &Route) -> Kinds {
        let vault = if route.paper {
            KindVault::None
        } else if !route.passwords.is_empty() {
            KindVault::Remembered
        } else if !route.written.is_empty() {
            KindVault::Written
        } else {
            KindVault::None
        };
        Kinds {
            seeds: 1,
            form: SeedForm::Paper,
            description: Described::Part,
            vault,
        }
    }

    /// The kinds as a paper route may have them: words on paper and no
    /// vault (§20.13 item 1). A lookup.
    pub fn on_paper(self) -> Kinds {
        Kinds {
            form: SeedForm::Paper,
            vault: KindVault::None,
            ..self
        }
    }

    /// The kinds in the plan's text: `kinds <seeds> <form> <description>
    /// <vault>`.
    fn code(&self) -> String {
        let form = match self.form {
            SeedForm::Paper => "words",
            SeedForm::Stick => "file",
        };
        let description = match self.description {
            Described::None => "none",
            Described::Part => "part",
            Described::Whole => "whole",
        };
        let vault = match self.vault {
            KindVault::None => "none",
            KindVault::Remembered => "remembered",
            KindVault::Written => "written",
        };
        format!("kinds {} {form} {description} {vault}", self.seeds)
    }

    /// [`Kinds::code`]'s four values read back.
    fn read(seeds: &str, form: &str, description: &str, vault: &str) -> Option<Kinds> {
        let seeds: usize = seeds.parse().ok()?;
        if seeds > 64 {
            return None;
        }
        Some(Kinds {
            seeds,
            form: match form {
                "words" => SeedForm::Paper,
                "file" => SeedForm::Stick,
                _ => return None,
            },
            description: match description {
                "none" => Described::None,
                "part" => Described::Part,
                "whole" => Described::Whole,
                _ => return None,
            },
            vault: match vault {
                "none" => KindVault::None,
                "remembered" => KindVault::Remembered,
                "written" => KindVault::Written,
                _ => return None,
            },
        })
    }
}

/// An access layer (§20.1): its routes, each checked on its own. Its name
/// is not here: it is kept in a vault's plan record alone.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Layer {
    /// Its routes, in order.
    pub routes: Vec<Route>,
}

impl Answers {
    /// The plan's access layers: those it keeps, or for a plan saved
    /// before them one layer able to spend with everything
    /// ([`everything`]). Costs a walk of the plan's map when it keeps none.
    pub fn access_layers(&self, shape: &Shape) -> Vec<Layer> {
        if self.layers.is_empty() {
            vec![everything(shape, self)]
        } else {
            self.layers.clone()
        }
    }
}

/// The one layer a plan saved before the layers reads as (§20.6): a
/// route of every place, all of them, knowing every vault's password and
/// every passphrase. Costs a walk of the plan's map, for its vaults.
pub fn everything(shape: &Shape, a: &Answers) -> Layer {
    Layer {
        routes: vec![Route {
            places: (0..a.places).collect(),
            any: a.places,
            passwords: kept_vaults(&map(shape, a)),
            passphrases: (0..shape.seeds.len())
                .filter(|&i| shape.seeds[i].passphrase)
                .collect(),
            paper: false,
            written: Vec::new(),
            kinds: None,
        }],
    }
}

/// The one layer a fresh plan starts with (§20.8 L2): a paper route of
/// every place, any `m` of them (at most every place), remembering the
/// passphrases of the seeds loaded here with one. A lookup per seed.
pub fn fresh(shape: &Shape, places: usize) -> Layer {
    let places = places.max(1);
    Layer {
        routes: vec![Route {
            places: (0..places).collect(),
            any: shape.m.max(1).min(places),
            passwords: Vec::new(),
            passphrases: (0..shape.seeds.len())
                .filter(|&i| shape.seeds[i].here && shape.seeds[i].passphrase)
                .collect(),
            paper: true,
            written: Vec::new(),
            kinds: None,
        }],
    }
}

impl Answers {
    /// The layers the Access layers card shows and Work it out works to
    /// (§20.3): those the plan keeps, or for a plan that keeps none the
    /// one a fresh plan starts with ([`fresh`]). A lookup.
    pub fn layers_shown(&self, shape: &Shape) -> Vec<Layer> {
        if self.layers.is_empty() {
            vec![fresh(shape, self.places)]
        } else {
            self.layers.clone()
        }
    }
}

/// Place `p` taken out of `layers`: it leaves each route, the places
/// after it move down one, and a route asks for at most the places it
/// has left. A lookup per route.
pub fn without_place(layers: &mut [Layer], p: usize) {
    for r in layers.iter_mut().flat_map(|l| l.routes.iter_mut()) {
        r.places.retain(|&q| q != p);
        for q in r.places.iter_mut() {
            if *q > p {
                *q -= 1;
            }
        }
        r.any = r.any.min(r.places.len());
    }
}

/// What layer `l` is called: its name from the vault's plan record when
/// one is known, else "Access layer n". A lookup.
pub fn layer_name(names: &[String], l: usize) -> String {
    match names.get(l).map(|n| n.trim()) {
        Some(n) if !n.is_empty() => n.to_string(),
        _ => fill(EN.plan_access_layer, &[&(l + 1).to_string()]),
    }
}

/// The layers' names a vault's plan record keeps, one per layer in
/// order (field [`field::PLAN_LAYER`]). A walk of the record's fields.
pub fn layer_names(r: &Record) -> Vec<String> {
    r.fields
        .iter()
        .filter(|f| f.number == field::PLAN_LAYER)
        .map(|f| String::from_utf8_lossy(&f.bytes).into_owned())
        .collect()
}

/// Puts the first `layers` of `names` into a plan record, one field per
/// layer in order, an unnamed layer as an empty one. A lookup per name.
pub fn push_layer_names(r: &mut Record, names: &[String], layers: usize) {
    if names.iter().all(|n| n.trim().is_empty()) {
        return;
    }
    for l in 0..layers {
        let name = names.get(l).map_or("", |n| n.trim());
        r.push(field::PLAN_LAYER, name.as_bytes());
    }
}

/// Brings `layers` to a plan of `places` places: a place that is gone
/// leaves each route, and a route asks for at most the places it has
/// left. A lookup per route.
pub fn fit(layers: &mut [Layer], places: usize) {
    for r in layers.iter_mut().flat_map(|l| l.routes.iter_mut()) {
        r.places.retain(|&p| p < places);
        r.any = r.any.min(r.places.len());
    }
}

/// Writes the layers into a plan's text (§20.6): a `layers n` line, then
/// a `route` line per route, `route <layer> <any> <places> <passwords>
/// <passphrases>`, each list numbers by commas, `-` when empty, ` paper`
/// after a paper route, ` written <vaults>` after a route with
/// passwords written down and ` kinds <seeds> <form> <description>
/// <vault>` after a route whose kinds are set (§20.13 item 7). Nothing for a plan with no layers of its own.
/// Never a name.
pub fn write(layers: &[Layer], out: &mut String) {
    if layers.is_empty() {
        return;
    }
    let list = |v: &[usize]| {
        if v.is_empty() {
            "-".to_string()
        } else {
            v.iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(",")
        }
    };
    out.push_str(&format!("layers {}\n", layers.len()));
    for (l, layer) in layers.iter().enumerate() {
        for r in &layer.routes {
            out.push_str(&format!(
                "route {l} {} {} {} {}",
                r.any,
                list(&r.places),
                list(&r.passwords),
                list(&r.passphrases),
            ));
            if r.paper {
                out.push(' ');
                out.push_str("paper");
            }
            if !r.written.is_empty() {
                out.push(' ');
                out.push_str("written");
                out.push(' ');
                out.push_str(&list(&r.written));
            }
            if let Some(k) = &r.kinds {
                out.push(' ');
                out.push_str(&k.code());
            }
            out.push('\n');
        }
    }
}

/// [`write`] read back for a plan of `places` places and `shape`'s
/// seeds: the `layers` count and each `route` line's value. None when a
/// line is not one [`write`] makes, or names a layer, place or seed the
/// plan does not have; no layers when the text has none.
pub fn read(
    shape: &Shape,
    places: usize,
    layers: Option<usize>,
    routes: &[&str],
) -> Option<Vec<Layer>> {
    let Some(n) = layers else {
        return routes.is_empty().then(Vec::new);
    };
    if n > 64 {
        return None;
    }
    let mut out = vec![Layer::default(); n];
    for value in routes {
        let parts: Vec<&str> = value.split(' ').collect();
        let list = |s: &str| -> Option<Vec<usize>> {
            if s == "-" {
                return Some(Vec::new());
            }
            s.split(',').map(|n| n.parse().ok()).collect()
        };
        let (l, any, ps, pws, pps, rest) = match parts.as_slice() {
            [l, any, ps, pws, pps, rest @ ..] => (l, any, ps, pws, pps, rest),
            _ => return None,
        };
        let (kinds, rest) = match rest {
            [rest @ .., "kinds", n, f, d, v] => (Some(Kinds::read(n, f, d, v)?), rest),
            _ => (None, rest),
        };
        let (paper, written) = match rest {
            [] => (false, Vec::new()),
            ["paper"] => (true, Vec::new()),
            ["written", w] => (false, list(w)?),
            ["paper", "written", w] => (true, list(w)?),
            _ => return None,
        };
        // A paper route's kinds are on paper, with no vault.
        if paper && kinds.is_some_and(|k| k != k.on_paper()) {
            return None;
        }
        let route = Route {
            places: list(ps)?,
            any: any.parse().ok()?,
            passwords: list(pws)?,
            passphrases: list(pps)?,
            paper,
            written,
            kinds,
        };
        let fits = route.places.iter().all(|&p| p < places)
            && route.any <= route.places.len()
            && route.passwords.iter().all(|&v| v < 1024)
            && route.written.iter().all(|&v| v < 1024)
            && route.passphrases.iter().all(|&i| i < shape.seeds.len());
        if !fits {
            return None;
        }
        out.get_mut(l.parse::<usize>().ok()?)?.routes.push(route);
    }
    Some(out)
}

/// Bit `i` of a mask; nothing past the 64th.
fn bit(i: usize) -> u64 {
    1u64.checked_shl(i as u32).unwrap_or(0)
}

/// What a place, or a vault opened, gives: as masks of seeds, passphrases,
/// shares, the vaults whose stick it keeps and the vaults whose password
/// it keeps written down; and whether it holds the whole description.
#[derive(Debug, Clone, Copy, Default)]
struct Gives {
    seeds: u64,
    pass: u64,
    wallet: bool,
    shares: u64,
    sticks: u64,
    written: u64,
}

impl Gives {
    fn add(&mut self, w: What) {
        match w {
            What::Words(i)
            | What::SeedQr(i)
            | What::Seed(i)
            | What::SeedFile(i)
            | What::SeedQrFile(i) => self.seeds |= bit(i),
            What::SeedPassphrase(i) => {
                self.seeds |= bit(i);
                self.pass |= bit(i);
            }
            What::Passphrase(i) | What::PassphraseFile(i) | What::PassphraseSealed(i) => {
                self.pass |= bit(i);
            }
            What::Sheet | What::Wallet | What::WalletFile(_) | What::SoftwareFile => {
                self.wallet = true;
            }
            What::Share(j) | What::ShareFile(j, _) => self.shares |= bit(j),
            What::VaultStick(v) => self.sticks |= bit(v),
            What::VaultPassword(v) => self.written |= bit(v),
        }
    }

    fn or(&mut self, o: &Gives) {
        self.seeds |= o.seeds;
        self.pass |= o.pass;
        self.wallet |= o.wallet;
        self.shares |= o.shares;
        self.sticks |= o.sticks;
        self.written |= o.written;
    }
}

/// The plan's map read once for the layers' checks: what each place
/// gives (its paper and its sticks), what each vault holds, and the
/// wallet's quorum and shares.
#[derive(Debug, Clone)]
pub struct Reach {
    m: usize,
    multi: bool,
    /// The seeds that need their passphrase to spend.
    need_pass: u64,
    /// Every key of the description, when it splits into shares.
    keys: u64,
    splits: bool,
    /// The keys share `j` holds.
    share_keys: Vec<u64>,
    places: Vec<Gives>,
    /// What each place keeps on paper alone.
    paper: Vec<Gives>,
    vaults: Vec<Gives>,
}

impl Reach {
    /// Reads plan `a` for `shape`. Costs one walk of its map
    /// ([`map`]).
    pub fn of(shape: &Shape, a: &Answers) -> Reach {
        Reach::new(shape, a.omit, &map(shape, a))
    }

    /// Reads the map `boxes` (as [`map`] or [`crate::plan::place_map`]
    /// gives it: a place's sticks on their own or folded into it) for
    /// `shape`, its shares leaving `omit` keys off. Costs one walk of
    /// `boxes`.
    pub fn new(shape: &Shape, omit: usize, boxes: &[Spot]) -> Reach {
        let mut places: Vec<Gives> = Vec::new();
        let mut paper: Vec<Gives> = Vec::new();
        let mut vaults: Vec<Gives> = Vec::new();
        let grow = |list: &mut Vec<Gives>, n: usize| {
            if list.len() <= n {
                list.resize(n + 1, Gives::default());
            }
        };
        for s in boxes {
            let (n, vault) = match s.at {
                At::Place(p) | At::Stick(p, _) => (p, false),
                At::Vault(v) => (v, true),
                _ => continue,
            };
            if n >= 64 {
                continue;
            }
            if vault {
                grow(&mut vaults, n);
                s.holds.iter().for_each(|(w, _)| vaults[n].add(*w));
                continue;
            }
            grow(&mut places, n);
            grow(&mut paper, n);
            for &(w, _) in &s.holds {
                places[n].add(w);
                if matches!(s.at, At::Place(_)) && !on_stick(w) {
                    paper[n].add(w);
                }
            }
        }
        let share_keys = if shape.splits {
            split_plan(shape.keys, shape.m, omit)
                .iter()
                .map(|row| row.iter().fold(0, |acc, &k| acc | bit(k)))
                .collect()
        } else {
            Vec::new()
        };
        Reach {
            m: shape.m.max(1),
            multi: shape.multi(),
            need_pass: (0..shape.seeds.len())
                .filter(|&i| shape.seeds[i].passphrase)
                .fold(0, |acc, i| acc | bit(i)),
            keys: (0..shape.keys).fold(0, |acc, k| acc | bit(k)),
            splits: shape.splits,
            share_keys,
            places,
            paper,
            vaults,
        }
    }

    /// How many places the plan has. A lookup.
    pub fn places(&self) -> usize {
        self.places.len()
    }

    /// What the places in `found` give, with the vaults' passwords
    /// `passwords` and the passphrases `passphrases` known (masks): each
    /// vault whose stick is found opened when its password is known or
    /// written at a found place. A pass over the places and vaults.
    fn open(&self, found: u64, passwords: u64, passphrases: u64) -> Gives {
        self.open_in(&self.places, found, passwords, passphrases)
    }

    /// [`Reach::open`] over what each place gives in `places`: all it
    /// keeps, or its paper alone.
    fn open_in(&self, places: &[Gives], found: u64, passwords: u64, passphrases: u64) -> Gives {
        let mut g = Gives {
            pass: passphrases,
            ..Gives::default()
        };
        for (p, gives) in places.iter().enumerate() {
            if found & bit(p) != 0 {
                g.or(gives);
            }
        }
        let known = passwords | g.written;
        let opened = g.sticks & known;
        for (v, gives) in self.vaults.iter().enumerate() {
            if opened & bit(v) != 0 {
                g.or(gives);
            }
        }
        g
    }

    /// The description, whole or from shares that together hold every
    /// key (the board's `check`). A pass over the shares.
    fn described(&self, g: &Gives) -> bool {
        if g.wallet {
            return true;
        }
        if !self.splits || g.shares == 0 {
            return false;
        }
        let covered = self
            .share_keys
            .iter()
            .enumerate()
            .filter(|(j, _)| g.shares & bit(*j) != 0)
            .fold(0, |acc, (_, k)| acc | k);
        covered & self.keys == self.keys
    }

    /// The seeds that sign: each kept, with its passphrase where it has
    /// one. A count of bits.
    fn signing(&self, g: &Gives) -> usize {
        (g.seeds & (!self.need_pass | g.pass)).count_ones() as usize
    }

    /// `g` rebuilds the wallet and spends: a quorum of seeds, and for more
    /// than one seed the description.
    fn spends(&self, g: &Gives) -> bool {
        self.signing(g) >= self.m && (!self.multi || self.described(g))
    }

    /// `g` shows the balance: the description, or a one-key wallet's seed.
    fn sees(&self, g: &Gives) -> bool {
        self.described(g) || (!self.multi && self.signing(g) >= 1)
    }

    /// Whether `route` is met (§20.1): every `any` of its places, with the
    /// passwords and passphrases it knows, rebuild the wallet and spend;
    /// for a paper route, from what they keep on paper alone. Tries each
    /// such set of places: C(n, any) for n places.
    pub fn route_met(&self, route: &Route) -> bool {
        let mut places: Vec<usize> = route.places.clone();
        places.sort_unstable();
        places.dedup();
        if route.any > places.len() || places.len() > 63 {
            return false;
        }
        let passwords = route.passwords.iter().fold(0, |acc, &v| acc | bit(v));
        let passphrases = route.passphrases.iter().fold(0, |acc, &i| acc | bit(i));
        let gives = if route.paper {
            &self.paper
        } else {
            &self.places
        };
        every_subset(places.len(), route.any, |pick| {
            let found = (0..places.len())
                .filter(|&k| pick & bit(k) != 0)
                .fold(0, |acc, k| acc | bit(places[k]));
            self.spends(&self.open_in(gives, found, passwords, passphrases))
        })
    }

    /// Whether `layer` can spend from paper (§20.12 item 3): it has a
    /// paper route, and that route is met. [`Reach::route_met`] per paper
    /// route.
    pub fn paper_met(&self, layer: &Layer) -> bool {
        layer.routes.iter().any(|r| r.paper && self.route_met(r))
    }

    /// Whether every one of `layers` can spend from paper, the paper
    /// shared: one copy at a place serves every route holding that place
    /// (§20.12 item 4). [`Reach::paper_met`] per layer.
    pub fn every_layer_on_paper(&self, layers: &[Layer]) -> bool {
        !layers.is_empty() && layers.iter().all(|l| self.paper_met(l))
    }

    /// Whether `layer` is met: it has a route, and every route of it is
    /// met. [`Reach::route_met`] per route.
    pub fn layer_met(&self, layer: &Layer) -> bool {
        !layer.routes.is_empty() && layer.routes.iter().all(|r| self.route_met(r))
    }

    /// The stranger margin (§20.2): the largest `k` such that a stranger
    /// who finds any `k` places, knowing nothing, cannot spend; every
    /// place when even all of them do not spend. Tries the sets of places
    /// by size up to the first that spends: at most 2^n for n places.
    pub fn stranger_margin(&self) -> usize {
        self.margin(|g| self.spends(g))
    }

    /// The balance margin (§20.2): the largest `k` such that no `k`
    /// places show the description (for a one-key wallet, its seed
    /// shows the balance too). Tries the sets of places by size up to the
    /// first that sees it: at most 2^n for n places.
    pub fn balance_margin(&self) -> usize {
        self.margin(|g| self.sees(g))
    }

    /// The knowledge `route` has, as masks of vault passwords and
    /// passphrases, and what its places give: all they keep, or on a
    /// paper route their paper alone. A pass over its lists.
    fn knows(&self, route: &Route) -> (u64, u64, &[Gives]) {
        let passwords = route.passwords.iter().fold(0, |acc, &v| acc | bit(v));
        let passphrases = route.passphrases.iter().fold(0, |acc, &i| acc | bit(i));
        let gives = if route.paper {
            &self.paper
        } else {
            &self.places
        };
        (passwords, passphrases, gives)
    }

    /// The places of `route`, each once, in order; none when it names
    /// more than 63.
    fn route_places(route: &Route) -> Vec<usize> {
        let mut places: Vec<usize> = route.places.clone();
        places.sort_unstable();
        places.dedup();
        if places.len() > 63 {
            places.clear();
        }
        places
    }

    /// Whether the places of the mask `found` spend by `route`, with what
    /// it knows: a pass over the places and vaults.
    fn route_spends(&self, route: &Route, found: u64) -> bool {
        let (passwords, passphrases, gives) = self.knows(route);
        self.spends(&self.open_in(gives, found, passwords, passphrases))
    }

    /// The fewest of `route`'s places that together spend by it (§20.7,
    /// "Places it needs"), or none when all of them do not. Tries the sets
    /// of its places by size up to the first that spends: at most 2^n for
    /// n places.
    pub fn route_needs(&self, route: &Route) -> Option<usize> {
        let places = Reach::route_places(route);
        for k in 1..=places.len() {
            let some = !every_subset(places.len(), k, |pick| {
                !self.route_spends(route, mask_of(&places, pick))
            });
            if some {
                return Some(k);
            }
        }
        None
    }

    /// Whether every `any` of `route`'s places, with what it knows, show
    /// the balance (§20.7, "Sees the balance"): as [`Reach::route_met`]
    /// asks of spending. Tries each such set: C(n, any) for n places.
    pub fn route_sees(&self, route: &Route) -> bool {
        let places = Reach::route_places(route);
        if route.any > places.len() || places.is_empty() {
            return false;
        }
        let (passwords, passphrases, gives) = self.knows(route);
        every_subset(places.len(), route.any, |pick| {
            let found = mask_of(&places, pick);
            self.sees(&self.open_in(gives, found, passwords, passphrases))
        })
    }

    /// The most places `routes` (a layer's, or one of them) may lose and
    /// still spend by some route (§20.9, "Places it can lose"): the
    /// largest `k` such that, whichever `k` of their places are lost, some
    /// route spends from what is left of its own; none when no route
    /// spends even from all of its places. Tries the sets of their places
    /// by size: at most 2^n sets for n places, each tried by every route.
    pub fn can_lose(&self, routes: &[Route]) -> Option<usize> {
        let mut all: Vec<usize> = routes.iter().flat_map(|r| r.places.clone()).collect();
        all.sort_unstable();
        all.dedup();
        if all.len() > 63 {
            return None;
        }
        let masks: Vec<u64> = routes
            .iter()
            .map(|r| r.places.iter().fold(0, |acc, &p| acc | bit(p)))
            .collect();
        let survives = |lost: u64| {
            routes
                .iter()
                .zip(&masks)
                .any(|(r, &m)| m & !lost != 0 && self.route_spends(r, m & !lost))
        };
        if !survives(0) {
            return None;
        }
        for k in 1..=all.len() {
            if !every_subset(all.len(), k, |pick| survives(mask_of(&all, pick))) {
                return Some(k - 1);
            }
        }
        Some(all.len())
    }

    /// The vaults `layer` opens (§20.7): those whose password one of its
    /// routes remembers, and those whose password is written at one of its
    /// places; as a mask. A pass over its routes and places.
    pub fn layer_opens(&self, layer: &Layer) -> u64 {
        let mut known = 0;
        for r in &layer.routes {
            known |= r.passwords.iter().fold(0, |acc, &v| acc | bit(v));
            for &p in &r.places {
                if let Some(g) = self.places.get(p) {
                    known |= g.written;
                }
            }
        }
        known
    }

    /// The vaults a stranger who finds the places of the mask `found`
    /// opens: those whose stick and written password are both among them.
    /// A pass over the places.
    pub fn stranger_opens(&self, found: u64) -> u64 {
        let g = self.open(found, 0, 0);
        g.sticks & g.written
    }

    /// Of every set of `k` places, the one that gives a stranger who finds
    /// it, knowing nothing, the most (§20.7): spending first, then the
    /// balance, then the seeds that sign, the keys of the description it
    /// holds and the things found; the first such set in order. As a mask
    /// of places. Tries each set: C(n, k) for n places.
    pub fn worst(&self, k: usize) -> u64 {
        let n = self.places.len().min(63);
        let k = k.min(n);
        let mut best: Option<(Score, u64)> = None;
        every_subset(n, k, |found| {
            let g = self.open(found, 0, 0);
            let keys = if g.wallet {
                self.keys.count_ones() + 1
            } else {
                self.share_keys
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| g.shares & bit(*j) != 0)
                    .fold(0, |acc, (_, k)| acc | k)
                    .count_ones()
            };
            let things = (g.seeds | g.pass).count_ones()
                + g.shares.count_ones()
                + g.sticks.count_ones()
                + g.written.count_ones()
                + u32::from(g.wallet);
            let score = (
                self.spends(&g),
                self.sees(&g),
                self.signing(&g),
                keys,
                things,
            );
            if best.is_none_or(|(b, _)| score > b) {
                best = Some((score, found));
            }
            true
        });
        best.map_or(0, |(_, found)| found)
    }

    fn margin(&self, gets: impl Fn(&Gives) -> bool) -> usize {
        let n = self.places.len().min(63);
        for k in 1..=n {
            if !every_subset(n, k, |found| !gets(&self.open(found, 0, 0))) {
                return k - 1;
            }
        }
        n
    }
}

/// What a set of places gives a stranger, compared in order: it spends, it
/// sees the balance, the seeds that sign, the description's keys, the
/// things found.
type Score = (bool, bool, usize, u32, u32);

/// The mask of the places `places[k]` for each bit `k` set in `pick`. A
/// pass over `places`.
fn mask_of(places: &[usize], pick: u64) -> u64 {
    (0..places.len())
        .filter(|&k| pick & bit(k) != 0)
        .fold(0, |acc, k| acc | bit(places[k]))
}

/// Whether `ok` holds for every set of `k` of `n` things, each set a mask
/// of its members, stopping at the first that fails: C(n, k) calls at
/// most. `n` is at most 63.
fn every_subset(n: usize, k: usize, mut ok: impl FnMut(u64) -> bool) -> bool {
    if k > n {
        return true;
    }
    if k == 0 {
        return ok(0);
    }
    let end = bit(n);
    let mut s: u64 = bit(k) - 1;
    while s < end {
        if !ok(s) {
            return false;
        }
        // The next mask with as many bits set (Gosper's hack).
        let c = s & s.wrapping_neg();
        let r = s + c;
        s = (((r ^ s) >> 2) / c) | r;
    }
    true
}

/// What plan `a` keeps (§20.3 item 2, §20.12 item 4): each thing at a
/// place or on a stick there counted once, however many layers' routes
/// hold that place; the sticks; the vaults; the vaults' passwords
/// written down (counted among the copies too).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Kept {
    /// Things kept at a place, on paper or as a file on a stick there; a
    /// vault's stick is counted with the sticks, not here.
    pub copies: usize,
    /// Sticks at the places.
    pub sticks: usize,
    /// Vaults.
    pub vaults: usize,
    /// Vaults' passwords written on paper.
    pub written: usize,
}

/// Counts what plan `a` keeps ([`Kept`]). Costs one walk of its map
/// ([`map`]); the layers do not enter it, so layers that share a place
/// share its copies.
pub fn kept(shape: &Shape, a: &Answers) -> Kept {
    kept_in(&map(shape, a))
}

/// [`kept`] on the map `boxes`, its sticks on their own (as [`map`]
/// gives it). One walk of `boxes`.
pub fn kept_in(boxes: &[Spot]) -> Kept {
    let mut k = Kept::default();
    for s in boxes {
        match s.at {
            At::Place(_) | At::Stick(..) => {
                if matches!(s.at, At::Stick(..)) {
                    k.sticks += 1;
                }
                for (w, _) in &s.holds {
                    match w {
                        What::VaultStick(_) => {}
                        What::VaultPassword(_) => {
                            k.written += 1;
                            k.copies += 1;
                        }
                        _ => k.copies += 1,
                    }
                }
            }
            At::Vault(_) => k.vaults += 1,
            _ => {}
        }
    }
    k
}

/// Whether every access layer of plan `a` can spend from paper
/// (§20.12 item 3). Costs a walk of the plan's map, then
/// [`Reach::every_layer_on_paper`].
pub fn every_layer_on_paper(shape: &Shape, a: &Answers) -> bool {
    Reach::of(shape, a).every_layer_on_paper(&a.access_layers(shape))
}

/// Whether `route` of plan `a` is met (§20.1). Costs a walk of the
/// plan's map, then [`Reach::route_met`]; a caller asking more than once
/// keeps a [`Reach`].
pub fn route_met(shape: &Shape, a: &Answers, route: &Route) -> bool {
    Reach::of(shape, a).route_met(route)
}

/// Whether `layer` of plan `a` is met: every route. Costs a walk of the
/// plan's map, then [`Reach::layer_met`].
pub fn layer_met(shape: &Shape, a: &Answers, layer: &Layer) -> bool {
    Reach::of(shape, a).layer_met(layer)
}

/// Plan `a`'s stranger margin (§20.2). Costs a walk of the plan's map,
/// then [`Reach::stranger_margin`].
pub fn stranger_margin(shape: &Shape, a: &Answers) -> usize {
    Reach::of(shape, a).stranger_margin()
}

/// Plan `a`'s balance margin (§20.2). Costs a walk of the plan's map,
/// then [`Reach::balance_margin`].
pub fn balance_margin(shape: &Shape, a: &Answers) -> usize {
    Reach::of(shape, a).balance_margin()
}
