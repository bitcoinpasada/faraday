//! **Work it out** (`docs/BACKUP-FLOW.md` §20.3, §20.8): where each thing
//! of the backup is kept, and in what form, so that every access layer is
//! met, each with its paper route, and a stranger who finds places gets
//! as little as can be.
//!
//! A branch and bound over the things to place: the description whole or
//! its shares, the seeds' words, each vault's sticks, each vault's
//! password written down, and (for a vault whose password is written
//! down) what the vault holds. Each thing is given the set of places that
//! keep it, in turn; a branch is dropped as soon as it cannot meet every
//! layer any more (every thing not yet placed assumed at every place it
//! may go) or cannot beat the best found on §20.3 item 2's order: the
//! largest stranger margin, then the largest balance margin, then the
//! fewest things kept (a copy counted once whichever layers share it),
//! then the fewest secrets kept unsealed. Both margins only fall as
//! things are added, and what is kept only grows, so a branch's margins
//! so far and what it keeps so far bound everything under it.
//!
//! Places whose routes are the same are interchangeable, and so are seeds
//! that every route treats the same: a placement and its renumberings are
//! tried once (the places' sets as columns and the seeds' as rows, both
//! kept in order, "double lex"). Paper is the only form placed at a place:
//! a file on a stick spends as paper does for every route but a paper
//! route and costs a stick, so it is never better. Passphrases are not
//! placed: a route counts the ones its layer remembers.
//!
//! A route whose kinds are set (§20.13) constrains the search: each of
//! its places keeps at least so many seeds in words or as files, a part
//! of the description (a share) or all of it, and a stick of one of its
//! vaults. These needs are worked out per place from the routes that hold
//! it, so places alike by their routes stay alike by their needs, and the
//! needs count seeds, never name them: the symmetry above stands. Seeds
//! kept as files are things of their own, and while there are any the
//! seeds' order is not kept. A route whose kinds cannot give full access
//! with the wallet's shape is a conflict ([`kinds_problem`]) and is left
//! out, as one that cannot be met at all.
//!
//! The search counts its steps and stops at [`BUDGET`], never at a clock,
//! so it gives the same answer on every machine and in every run
//! (§20.8). It runs on the press of Work it out only, never while
//! drawing.

use crate::access::{Described, KindVault, Kinds, Layer, Route, SeedForm};
use crate::backup::split_plan;
use crate::plan::{Answers, At, Shape, Spot, What, stored_map, tag_of};

/// The most steps one Work it out takes (§20.8): a step is one choice of
/// places for one thing. Set so that the worst preset case (a 3 of 5,
/// three layers each with a paper route, two vaults) takes at most 250 ms
/// on the build machine's release build: measured 2026-10-19, it ends
/// every placement tried in 320,119 steps, about 165 ms, and the whole
/// budget takes about 200 ms (`docs/BACKUP-FLOW.md` §20.8, L2 built).
pub const BUDGET: u64 = 400_000;

/// The most places the search works with: a route's places past it are
/// left out of the search.
const MOST_PLACES: usize = 24;

/// What Work it out found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The map to put on the board: the places, their sticks and the
    /// vaults as found, and the plan's other spots (the stick of files,
    /// the software, the seeds on their own devices) as they were.
    pub map: Vec<Spot>,
    /// The stranger margin of the map (§20.2).
    pub margin: usize,
    /// The balance margin of the map (§20.2).
    pub balance: usize,
    /// Things kept: copies, sticks and vaults, a copy counted once.
    pub kept: usize,
    /// Secrets kept unsealed: seeds' words and written passwords.
    pub unsealed: usize,
    /// The budget ran out before every placement was tried.
    pub stopped: bool,
    /// The steps taken.
    pub steps: u64,
}

/// Why a route's kinds (§20.13 item 6), or the routes together, cannot
/// be had with a wallet's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// The route cannot be met at all ([`possible`]): no place asked for,
    /// more than it has, or fewer seeds it can sign with than the quorum.
    Impossible,
    /// Any `any` of its places, `per` seeds each, give fewer seeds than
    /// the quorum `needs`.
    Short {
        /// The route's `k`.
        any: usize,
        /// Seeds at each place.
        per: usize,
        /// The quorum.
        needs: usize,
    },
    /// More seeds at each place than the wallet has here, `has`.
    TooMany {
        /// Seeds at each place.
        per: usize,
        /// The seeds loaded here.
        has: usize,
    },
    /// A wallet of more than one seed, no description at its places and
    /// no vault to hold it.
    NoDescription,
    /// A part of the description at each place, and the route's `any`
    /// places cannot be all of it: one place is enough, or the
    /// description does not split.
    PartAlone,
    /// A vault at each place, and the route has no vault's password of
    /// the kind said.
    NoPassword,
    /// What each place holds lets a stranger who finds one of them spend,
    /// while some route asks for more than one place.
    StrangerAlone,
    /// No placement meets every route (the routes together).
    Unmet,
}

/// A conflict of a set of routes with a wallet's shape: the route, by
/// layer and route number, where it is one route's, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Problem {
    /// The layer and the route, from 0; none for [`Why::Unmet`].
    pub at: Option<(usize, usize)>,
    /// Why.
    pub why: Why,
}

/// What `layers` give with a wallet of `shape` (§20.13 item 4): the best
/// placement, or why there is none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fit {
    /// The best placement found.
    Placed(Outcome),
    /// The routes' conflicts with the shape.
    Conflicts(Vec<Problem>),
}

/// Why `route`'s kinds cannot give full access in a wallet of `shape`
/// (§20.13 items 3 and 6), if they cannot: too few seeds from any `k` of
/// its places, more at each than there are, no description or a part
/// where one place must be enough, a vault with no password to open it.
/// Nothing for a route whose kinds are unset. A lookup.
pub fn kinds_problem(shape: &Shape, route: &Route) -> Option<Why> {
    let k = route.kinds?;
    let multi = shape.multi();
    let has = shape.here().len();
    if k.seeds > has {
        return Some(Why::TooMany { per: k.seeds, has });
    }
    let vault = !route.paper && k.vault != KindVault::None;
    if vault {
        let named = match k.vault {
            KindVault::Remembered => &route.passwords,
            _ => &route.written,
        };
        return named.is_empty().then_some(Why::NoPassword);
    }
    let needs = shape.m.max(1);
    if route.any * k.seeds < needs {
        return Some(Why::Short {
            any: route.any,
            per: k.seeds,
            needs,
        });
    }
    if multi {
        match k.description {
            Described::None => return Some(Why::NoDescription),
            Described::Part if route.any <= 1 || !shape.splits => return Some(Why::PartAlone),
            _ => {}
        }
    }
    None
}

/// Whether one place of `route`, by its kinds, lets a stranger spend
/// whatever else is kept there: its seeds, past any that need a
/// passphrase, are a quorum, and for more than one seed it holds the
/// whole description; or its vault's password is written at one of its
/// places and one place is enough. A lookup per seed.
fn stranger_alone(shape: &Shape, route: &Route) -> bool {
    let Some(k) = route.kinds else {
        return false;
    };
    if !route.paper && k.vault == KindVault::Written && route.any <= 1 {
        return true;
    }
    let guarded = shape
        .here()
        .into_iter()
        .filter(|&i| shape.seeds[i].passphrase)
        .count();
    let free = k.seeds.saturating_sub(guarded);
    free >= shape.m.max(1) && (!shape.multi() || k.description == Described::Whole)
}

/// The conflicts of `layers`' kinds with a wallet of `shape` (§20.13 item
/// 6), and the routes that cannot be met at all: per route, the first of
/// [`kinds_problem`], then [`Why::StrangerAlone`] where one of its places
/// would let a stranger spend while some route asks for more than one
/// (a route of any one place with nothing remembered is §20.4's conflict
/// already). A lookup per route.
pub fn problems(shape: &Shape, layers: &[Layer]) -> Vec<Problem> {
    let wide = layers
        .iter()
        .flat_map(|l| l.routes.iter())
        .any(|r| r.any > 1);
    let mut out = Vec::new();
    for (l, layer) in layers.iter().enumerate() {
        for (r, route) in layer.routes.iter().enumerate() {
            let at = Some((l, r));
            if let Some(why) = kinds_problem(shape, route) {
                out.push(Problem { at, why });
            } else if !possible(shape, route) {
                out.push(Problem {
                    at,
                    why: Why::Impossible,
                });
            } else if wide
                && stranger_alone(shape, route)
                && !(route.any <= 1 && open(shape, route))
            {
                out.push(Problem {
                    at,
                    why: Why::StrangerAlone,
                });
            }
        }
    }
    out
}

/// The shape check (§20.13 items 4 and 5): `layers` worked out for plan
/// `a` in a wallet of `shape`, in at most `budget` steps, or their
/// conflicts with it ([`problems`], or [`Why::Unmet`] when no placement
/// meets every route). Create (L6) calls it once per shape it tries;
/// Back up with the wallet's own. Costs [`problems`], then
/// [`work_out`] when there are none.
pub fn fit(shape: &Shape, a: &Answers, layers: &[Layer], budget: u64) -> Fit {
    let found = problems(shape, layers);
    if !found.is_empty() {
        return Fit::Conflicts(found);
    }
    match work_out(shape, a, layers, budget) {
        Some(out) => Fit::Placed(out),
        None => Fit::Conflicts(vec![Problem {
            at: None,
            why: Why::Unmet,
        }]),
    }
}

/// Whether `route` can be met at all in a wallet of `shape`: it asks for
/// at least one place, no more than it has, and the seeds it can use
/// (loaded here, each with its passphrase remembered where it has one)
/// are a quorum. A lookup per seed.
pub fn possible(shape: &Shape, route: &Route) -> bool {
    route.any >= 1 && route.any <= route.places.len() && usable(shape, route) >= shape.m.max(1)
}

/// How many seeds `route` can sign with: loaded here, and remembered
/// with their passphrase where they have one. A lookup per seed.
pub fn usable(shape: &Shape, route: &Route) -> usize {
    (0..shape.seeds.len())
        .filter(|&i| {
            let s = &shape.seeds[i];
            s.here && (!s.passphrase || route.passphrases.contains(&i))
        })
        .count()
}

/// Whether `route`, met, lets a stranger who finds `any` of its places
/// spend as well (§20.12 item 5): it knows no vault's password it can
/// use and remembers no passphrase of a seed loaded here.
pub fn open(shape: &Shape, route: &Route) -> bool {
    let pass = route
        .passphrases
        .iter()
        .any(|&i| shape.seeds.get(i).is_some_and(|s| s.here && s.passphrase));
    !pass && (route.paper || route.passwords.is_empty())
}

/// What one place, or one vault, gives: seeds, the whole description,
/// shares, the vaults whose stick it keeps, the vaults whose password it
/// keeps written down.
#[derive(Debug, Clone, Copy, Default)]
struct G {
    seeds: u64,
    wallet: bool,
    shares: u64,
    sticks: u32,
    written: u32,
}

impl G {
    fn or(&mut self, o: &G) {
        self.seeds |= o.seeds;
        self.wallet |= o.wallet;
        self.shares |= o.shares;
        self.sticks |= o.sticks;
        self.written |= o.written;
    }
}

/// What the places and vaults give for one check, kept on the stack.
struct Bufs {
    at: [G; MOST_PLACES],
    paper: [G; MOST_PLACES],
    vaults: [G; 32],
}

impl Bufs {
    fn new() -> Bufs {
        Bufs {
            at: [G::default(); MOST_PLACES],
            paper: [G::default(); MOST_PLACES],
            vaults: [G::default(); 32],
        }
    }
}

/// A route, as the search checks it.
#[derive(Debug, Clone)]
struct R {
    /// Every set of `any` of its places, as masks.
    subsets: Vec<u32>,
    /// The vaults whose passwords it knows (none on a paper route).
    passwords: u32,
    /// The passphrases it remembers, by seed.
    pass: u64,
    paper: bool,
}

/// One thing the search places.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Th {
    Sheet,
    Share(usize),
    Seed(usize),
    /// Seed `i` as a file on a stick at a place.
    File(usize),
    Stick(usize),
    Written(usize),
    /// Vault `v` holds seed `i`, or the description for `WALLET`.
    Item(usize, usize),
}

const WALLET: usize = usize::MAX;

#[derive(Debug, Clone)]
struct Thing {
    th: Th,
    /// The places it may be kept at; for an item, bit 0 is "held".
    allowed: u32,
    /// For a seed: its class, seeds of one class interchangeable; the
    /// seed before it in the list of the same class, whose set its own
    /// may not pass.
    after: Option<usize>,
    /// The places it must be kept at (a route's kinds).
    must: u32,
    /// It must be kept at some place.
    some: bool,
}

/// What a place must keep by the kinds of the routes that hold it
/// (§20.13), each at least.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Need {
    words: usize,
    files: usize,
    shares: usize,
    sheet: bool,
    /// For each: a stick of one of these vaults.
    sticks: Vec<u32>,
}

impl Need {
    fn any(&self) -> bool {
        *self != Need::default()
    }
}

/// The score §20.3 item 2 orders by: larger margins, then fewer things.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Score {
    margin: usize,
    balance: usize,
    kept: usize,
    unsealed: usize,
}

impl Score {
    fn better(&self, o: &Score) -> bool {
        (self.margin, self.balance, o.kept, o.unsealed)
            > (o.margin, o.balance, self.kept, self.unsealed)
    }
}

struct Search<'a> {
    n: usize,
    m: usize,
    multi: bool,
    need_pass: u64,
    keys: u64,
    share_keys: &'a [u64],
    routes: Vec<R>,
    /// Every set of the places any route holds, by its size.
    sizes: Vec<Vec<u32>>,
    /// The most the stranger margin can be (§20.12 item 5).
    cap: usize,
    /// Each place's class: places of one class interchangeable.
    class: Vec<u32>,
    vaults: usize,
    /// The vaults set up before the keys.
    existing: usize,
    /// Each route's need of a written password: the vault, and the
    /// route's places, one of which keeps it.
    written_need: Vec<(usize, u32)>,
    /// What each vault holds when its password is written nowhere.
    full: Vec<G>,
    /// What each place must keep by the routes' kinds; empty when no
    /// route's kinds are set.
    needs: Vec<Need>,
    things: Vec<Thing>,
    choice: Vec<u32>,
    steps: u64,
    budget: u64,
    stopped: bool,
    best: Option<(Score, Vec<u32>, Vec<Thing>)>,
}

/// Bit `i`; nothing past the 64th.
fn bit(i: usize) -> u64 {
    1u64.checked_shl(i as u32).unwrap_or(0)
}

/// Every set of `k` of the places in `mask`, as masks.
fn subsets(mask: u32, k: usize) -> Vec<u32> {
    let places: Vec<usize> = (0..32).filter(|&p| mask & (1 << p) != 0).collect();
    let n = places.len();
    let mut out = Vec::new();
    if k == 0 || k > n {
        return out;
    }
    let mut s: u64 = (1u64 << k) - 1;
    while s < (1u64 << n) {
        out.push(
            (0..n)
                .filter(|&j| s & (1 << j) != 0)
                .fold(0u32, |acc, j| acc | (1 << places[j])),
        );
        let c = s & s.wrapping_neg();
        let r = s + c;
        s = (((r ^ s) >> 2) / c) | r;
    }
    out
}

/// A set of places read with place 0 as its most significant: the order
/// in which a seed's set may not pass the set of the seed before it.
fn lex(n: usize, s: u32) -> u32 {
    (0..n)
        .filter(|&p| s & (1 << p) != 0)
        .fold(0, |acc, p| acc | (1 << (n - 1 - p)))
}

impl Search<'_> {
    fn described(&self, g: &G) -> bool {
        if g.wallet {
            return true;
        }
        if g.shares == 0 {
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

    fn signing(&self, g: &G, pass: u64) -> usize {
        (g.seeds & (!self.need_pass | pass)).count_ones() as usize
    }

    fn spends(&self, g: &G, pass: u64) -> bool {
        self.signing(g, pass) >= self.m && (!self.multi || self.described(g))
    }

    fn sees(&self, g: &G) -> bool {
        self.described(g) || (!self.multi && self.signing(g, 0) >= 1)
    }

    /// What the places in `found` give from `paper` (and, unless on
    /// paper, their sticks), each vault whose stick is found opened with
    /// `known` or a password written at a found place.
    fn gather(&self, at: &[G], vaults: &[G], found: u32, known: u32, paper: bool) -> G {
        let mut g = G::default();
        let mut f = found;
        while f != 0 {
            let p = f.trailing_zeros() as usize;
            f &= f - 1;
            g.or(&at[p]);
        }
        if paper {
            return g;
        }
        let mut opened = g.sticks & (known | g.written);
        while opened != 0 {
            let v = opened.trailing_zeros() as usize;
            opened &= opened - 1;
            if let Some(x) = vaults.get(v) {
                g.or(x);
            }
        }
        g
    }

    /// Whether every route is met by the places `at` (their paper as
    /// `paper`) and the vaults `vaults`.
    fn met(&self, at: &[G], paper: &[G], vaults: &[G]) -> bool {
        self.routes.iter().all(|r| {
            let from = if r.paper { paper } else { at };
            r.subsets.iter().all(|&s| {
                let g = self.gather(from, vaults, s, r.passwords, r.paper);
                self.spends(&g, r.pass)
            })
        })
    }

    /// The largest `k` such that no `k` of the places reached do `gets`
    /// for a stranger; every place when even all of them do not.
    fn margin(&self, at: &[G], vaults: &[G], gets: impl Fn(&G) -> bool) -> usize {
        for (k, sets) in self.sizes.iter().enumerate().skip(1) {
            for &s in sets {
                if gets(&self.gather(at, vaults, s, 0, false)) {
                    return k - 1;
                }
            }
        }
        self.n
    }

    /// The places and vaults with things `0..upto` placed as chosen,
    /// and, `hopeful`, every later thing at every place it may go.
    fn build(&self, upto: usize, hopeful: bool, b: &mut Bufs) {
        let Bufs { at, paper, vaults } = b;
        at.fill(G::default());
        paper.fill(G::default());
        vaults[..self.vaults].copy_from_slice(&self.full);
        // Vaults whose contents are things of their own start empty.
        for t in &self.things {
            if let Th::Item(v, _) = t.th {
                vaults[v] = G::default();
            }
        }
        for (k, t) in self.things.iter().enumerate() {
            let set = if k < upto {
                self.choice[k]
            } else if hopeful {
                t.allowed
            } else {
                continue;
            };
            if let Th::Item(v, i) = t.th {
                if set & 1 != 0 {
                    if i == WALLET {
                        vaults[v].wallet = true;
                    } else {
                        vaults[v].seeds |= bit(i);
                    }
                }
                continue;
            }
            let mut s = set;
            while s != 0 {
                let p = s.trailing_zeros() as usize;
                s &= s - 1;
                let (on_paper, g) = match t.th {
                    Th::Sheet => (
                        true,
                        G {
                            wallet: true,
                            ..G::default()
                        },
                    ),
                    Th::Share(j) => (
                        true,
                        G {
                            shares: bit(j),
                            ..G::default()
                        },
                    ),
                    Th::Seed(i) => (
                        true,
                        G {
                            seeds: bit(i),
                            ..G::default()
                        },
                    ),
                    Th::File(i) => (
                        false,
                        G {
                            seeds: bit(i),
                            ..G::default()
                        },
                    ),
                    Th::Written(v) => (
                        true,
                        G {
                            written: 1 << v,
                            ..G::default()
                        },
                    ),
                    Th::Stick(v) => (
                        false,
                        G {
                            sticks: 1 << v,
                            ..G::default()
                        },
                    ),
                    Th::Item(..) => continue,
                };
                at[p].or(&g);
                if on_paper {
                    paper[p].or(&g);
                }
            }
        }
    }

    /// Copies, sticks and vaults kept, and secrets unsealed, by things
    /// `0..upto`.
    fn kept(&self, upto: usize) -> (usize, usize) {
        let mut kept = 0;
        let mut unsealed = 0;
        let mut vaults = 0u32;
        let mut files = 0u32;
        for (k, t) in self.things.iter().enumerate().take(upto) {
            let c = self.choice[k].count_ones() as usize;
            match t.th {
                Th::Item(..) => {}
                Th::File(_) => {
                    kept += c;
                    unsealed += c;
                    files |= self.choice[k];
                }
                Th::Stick(v) => {
                    kept += c;
                    if c > 0 {
                        vaults |= 1 << v;
                    }
                }
                Th::Seed(_) | Th::Written(_) => {
                    kept += c;
                    unsealed += c;
                }
                Th::Sheet | Th::Share(_) => kept += c,
            }
        }
        // A place's files are on one stick of files there.
        (
            kept + vaults.count_ones() as usize + files.count_ones() as usize,
            unsealed,
        )
    }

    /// Whether things `0..upto`, as chosen, can still lead to a placement
    /// that meets every route and beats the best found.
    fn hopeful(&self, upto: usize) -> bool {
        if !self.needs_met(upto, true) {
            return false;
        }
        let mut b = Bufs::new();
        if let Some((best, _, _)) = &self.best {
            self.build(upto, false, &mut b);
            let margin = self
                .margin(&b.at, &b.vaults, |g| self.spends(g, 0))
                .min(self.cap);
            let beats = if margin != best.margin {
                margin > best.margin
            } else {
                let balance = self.margin(&b.at, &b.vaults, |g| self.sees(g)).min(margin);
                if balance != best.balance {
                    balance > best.balance
                } else {
                    let (kept, unsealed) = self.kept(upto);
                    let more = self.short(&b.paper) + self.forced(upto, &b.at, best.margin);
                    (kept + more, unsealed) < (best.kept, best.unsealed)
                }
            };
            if !beats {
                return false;
            }
        }
        // A thing given every place it may go leaves every route as met
        // as the branch above it, which was.
        if upto > 0 && self.choice[upto - 1] == self.things[upto - 1].allowed {
            return true;
        }
        self.build(upto, true, &mut b);
        self.met(&b.at, &b.paper, &b.vaults)
    }

    /// Whether every place can still keep what the routes' kinds need of
    /// it: things `0..upto` as chosen, and, `hopeful`, every later thing
    /// at every place it may go. A pass over the things per place that
    /// needs any.
    fn needs_met(&self, upto: usize, hopeful: bool) -> bool {
        self.needs.iter().enumerate().all(|(p, need)| {
            if !need.any() {
                return true;
            }
            let at = |k: usize| {
                let set = if k < upto {
                    self.choice[k]
                } else if hopeful {
                    self.things[k].allowed
                } else {
                    0
                };
                set & (1 << p) != 0
            };
            let (mut words, mut files, mut shares, mut sheet) = (0, 0, 0, false);
            let mut sticks = 0u32;
            for (k, t) in self.things.iter().enumerate() {
                if !at(k) {
                    continue;
                }
                match t.th {
                    Th::Seed(_) => words += 1,
                    Th::File(_) => files += 1,
                    Th::Share(_) => shares += 1,
                    Th::Sheet => sheet = true,
                    Th::Stick(v) => sticks |= 1 << v,
                    _ => {}
                }
            }
            words >= need.words
                && files >= need.files
                && shares >= need.shares
                && (sheet || !need.sheet)
                && need.sticks.iter().all(|&m| m & sticks != 0)
        })
    }

    /// The fewest sticks and vaults still to be kept for a margin of
    /// `margin`: a set of a route's places no larger than it may not spend
    /// from what a stranger there finds, so a route through a vault whose
    /// password it knows needs a stick of one at each such set. Counts
    /// such sets with no stick yet, no two sharing a place, and a vault
    /// when none has a stick yet. No copy on paper serves them, so this
    /// adds to [`Search::short`].
    fn forced(&self, upto: usize, at: &[G], margin: usize) -> usize {
        // The vaults with a stick kept already.
        let kept =
            self.things[..upto]
                .iter()
                .zip(&self.choice)
                .fold(0u32, |acc, (t, &c)| match t.th {
                    Th::Stick(v) if c != 0 => acc | (1 << v),
                    _ => acc,
                });
        let mut used = 0u32;
        let mut sets = 0;
        let mut vaults = 0u32;
        let mut more = 0;
        for r in self.routes.iter().filter(|r| !r.paper && r.passwords != 0) {
            let mut lacks = false;
            for &set in &r.subsets {
                if set.count_ones() as usize > margin {
                    continue;
                }
                let g = self.gather(at, &[], set, 0, true);
                if g.sticks & r.passwords != 0 {
                    continue;
                }
                lacks = true;
                if set & used == 0 {
                    used |= set;
                    sets += 1;
                }
            }
            // A vault more for each such route whose vaults are all new
            // and none another such route counted.
            if lacks && r.passwords & (kept | vaults) == 0 {
                vaults |= r.passwords;
                more += 1;
            }
        }
        sets + more
    }

    /// The fewest copies still to be kept: the most any one set of a
    /// paper route's places lacks of the seeds it signs with and of the
    /// description, each copy giving a set one thing at most. A pass over
    /// the paper routes' sets.
    fn short(&self, paper: &[G]) -> usize {
        let mut most = 0;
        for r in self.routes.iter().filter(|r| r.paper) {
            for &set in &r.subsets {
                let g = self.gather(paper, &[], set, 0, true);
                let seeds = self.m.saturating_sub(self.signing(&g, r.pass));
                let desc = usize::from(self.multi && !self.described(&g));
                most = most.max(seeds + desc);
            }
        }
        most
    }

    /// Each choice of places for thing `t`, fewest first: within each
    /// class of places still alike, the first so many of them.
    fn options(&self, t: usize) -> Vec<u32> {
        let thing = &self.things[t];
        if let Th::Item(..) = thing.th {
            return vec![1, 0];
        }
        // The classes of places still alike: same class, same things.
        let mut groups: Vec<(u32, u64, Vec<usize>)> = Vec::new();
        for p in 0..self.n {
            if thing.allowed & (1 << p) == 0 {
                continue;
            }
            let col = (0..t).fold(0u64, |acc, k| {
                acc | if self.choice[k] & (1 << p) != 0 {
                    bit(k)
                } else {
                    0
                }
            });
            match groups
                .iter_mut()
                .find(|(c, x, _)| *c == self.class[p] && *x == col)
            {
                Some(g) => g.2.push(p),
                None => groups.push((self.class[p], col, vec![p])),
            }
        }
        let mut out: Vec<u32> = vec![0];
        for (_, _, places) in &groups {
            let mut next = Vec::with_capacity(out.len() * (places.len() + 1));
            for &base in &out {
                let mut s = base;
                next.push(s);
                for &p in places {
                    s |= 1 << p;
                    next.push(s);
                }
            }
            out = next;
        }
        if let Th::Written(v) = thing.th {
            out.retain(|&s| {
                self.written_need
                    .iter()
                    .filter(|(w, _)| *w == v)
                    .all(|(_, need)| s & need != 0)
            });
        }
        if thing.must != 0 || thing.some {
            out.retain(|&s| s & thing.must == thing.must && (s != 0 || !thing.some));
        }
        if let Some(prev) = thing.after {
            let most = lex(self.n, self.choice[prev]);
            out.retain(|&s| lex(self.n, s) <= most);
        }
        // One place first, then none, then more: a thing at one place
        // each is what a large margin is made of, so the first placements
        // found are good ones and the bound drops the rest sooner.
        out.sort_by_key(|s| match s.count_ones() {
            1 => 0,
            0 => 1,
            c => c,
        });
        out
    }

    fn leaf(&mut self) {
        let k = self.things.len();
        if !self.needs_met(k, false) {
            return;
        }
        let mut b = Bufs::new();
        self.build(k, false, &mut b);
        if !self.met(&b.at, &b.paper, &b.vaults) {
            return;
        }
        let (kept, unsealed) = self.kept(k);
        let margin = self.margin(&b.at, &b.vaults, |g| self.spends(g, 0));
        let balance = self.margin(&b.at, &b.vaults, |g| self.sees(g));
        let score = Score {
            margin,
            balance,
            kept,
            unsealed,
        };
        if self.best.as_ref().is_none_or(|(b, _, _)| score.better(b)) {
            self.best = Some((score, self.choice.clone(), self.things.clone()));
        }
    }

    fn dfs(&mut self, t: usize) {
        if t == self.things.len() {
            self.leaf();
            return;
        }
        for s in self.options(t) {
            if self.steps >= self.budget {
                self.stopped = true;
                return;
            }
            self.steps += 1;
            self.choice[t] = s;
            if self.hopeful(t + 1) {
                self.dfs(t + 1);
            }
            if self.stopped {
                return;
            }
        }
        self.choice[t] = 0;
    }
}

/// Works out plan `a`'s placement for `layers` within `shape`, the
/// wallet's shape kept (§20.12 item 6), in at most `budget` steps. The
/// routes that cannot be met at all ([`possible`]) are left out; with no
/// route left, nothing is placed: `None`. The search of this module:
/// costs up to `budget` steps of a few hundred mask operations each.
pub fn work_out(shape: &Shape, a: &Answers, layers: &[Layer], budget: u64) -> Option<Outcome> {
    work_out_counted(shape, a, layers, budget).0
}

/// [`work_out`], with the steps it took whether or not it placed
/// anything: what Create's Suggest a wallet shares the budget by
/// (`docs/BACKUP-FLOW.md` §20.14 item 3). The search: costs up to
/// `budget` steps of a few hundred mask operations each.
pub fn work_out_counted(
    shape: &Shape,
    a: &Answers,
    layers: &[Layer],
    budget: u64,
) -> (Option<Outcome>, u64) {
    let n = a.places.clamp(1, MOST_PLACES);
    let routes: Vec<&Route> = layers
        .iter()
        .flat_map(|l| l.routes.iter())
        .filter(|r| possible(shape, r) && kinds_problem(shape, r).is_none())
        .collect();
    let routes: Vec<Route> = routes
        .into_iter()
        .map(|r| {
            let mut r = r.clone();
            r.places.retain(|&p| p < n);
            r.places.sort_unstable();
            r.places.dedup();
            r.any = r.any.min(r.places.len());
            r
        })
        .filter(|r| r.any >= 1)
        .collect();
    if routes.is_empty() {
        return (None, 0);
    }
    let mask = |v: &[usize]| v.iter().fold(0u32, |acc, &p| acc | (1 << p));
    let vmask = |v: &[usize]| {
        v.iter()
            .filter(|&&x| x < 32)
            .fold(0u32, |acc, &x| acc | (1 << x))
    };
    let pmask = |v: &[usize]| v.iter().fold(0u64, |acc, &i| acc | bit(i));
    let reach = routes.iter().fold(0u32, |acc, r| acc | mask(&r.places));

    // The vaults: those of the plan and those a route names.
    let stored = stored_map(shape, a);
    let named = stored
        .iter()
        .filter_map(|s| match s.at {
            At::Vault(v) => Some(v + 1),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    let wanted = routes
        .iter()
        .filter(|r| !r.paper)
        .flat_map(|r| r.passwords.iter().chain(r.written.iter()))
        .map(|&v| v + 1)
        .max()
        .unwrap_or(0);
    let vaults = named.max(shape.vaults).max(wanted).min(32);

    // The seeds placed: loaded here, and of use to some route.
    let seeds: Vec<usize> = (0..shape.seeds.len().min(64))
        .filter(|&i| {
            let s = &shape.seeds[i];
            s.here && (!s.passphrase || routes.iter().any(|r| r.passphrases.contains(&i)))
        })
        .collect();
    let need_pass = (0..shape.seeds.len().min(64))
        .filter(|&i| shape.seeds[i].passphrase)
        .fold(0, |acc, i| acc | bit(i));

    // Places alike when every route holds both or neither.
    let sig: Vec<u64> = (0..n)
        .map(|p| {
            routes.iter().enumerate().fold(0u64, |acc, (k, r)| {
                acc | if r.places.contains(&p) { bit(k) } else { 0 }
            })
        })
        .collect();
    let mut class = vec![0u32; n];
    let mut seen: Vec<u64> = Vec::new();
    for p in 0..n {
        let c = seen.iter().position(|&s| s == sig[p]).unwrap_or_else(|| {
            seen.push(sig[p]);
            seen.len() - 1
        });
        class[p] = c as u32;
    }

    let mut sticks = vec![0u32; vaults];
    let mut written = vec![0u32; vaults];
    let mut written_need = Vec::new();
    for r in routes.iter().filter(|r| !r.paper) {
        for &v in r.passwords.iter().chain(r.written.iter()) {
            if v < vaults {
                sticks[v] |= mask(&r.places);
            }
        }
        for &v in &r.written {
            if v < vaults {
                written[v] |= mask(&r.places);
                written_need.push((v, mask(&r.places)));
            }
        }
    }
    let multi = shape.multi();

    // What each place must keep by the routes' kinds (§20.13): the most
    // any route holding it asks for; none when no route's kinds are set.
    let mut needs = vec![Need::default(); n];
    let mut files_at = 0u32;
    for r in &routes {
        let Some(k) = r.kinds else {
            continue;
        };
        let k: Kinds = if r.paper { k.on_paper() } else { k };
        let vault = match k.vault {
            KindVault::Remembered => vmask(&r.passwords),
            KindVault::Written => vmask(&r.written),
            KindVault::None => 0,
        };
        for &p in &r.places {
            let need = &mut needs[p];
            match k.form {
                SeedForm::Paper => need.words = need.words.max(k.seeds),
                SeedForm::Stick => {
                    need.files = need.files.max(k.seeds);
                    if k.seeds > 0 {
                        files_at |= 1 << p;
                    }
                }
            }
            if multi {
                match k.description {
                    Described::Part => need.shares = need.shares.max(1),
                    Described::Whole => need.sheet = true,
                    Described::None => {}
                }
            }
            if vault != 0 && !need.sticks.contains(&vault) {
                need.sticks.push(vault);
            }
        }
    }
    if !needs.iter().any(Need::any) {
        needs.clear();
    }
    let sheet_at = needs
        .iter()
        .enumerate()
        .filter(|(_, x)| x.sheet)
        .fold(0u32, |acc, (p, _)| acc | (1 << p));
    let need_part = needs.iter().any(|x| x.shares > 0);
    let need_whole = sheet_at != 0;

    let mut full = vec![G::default(); vaults];
    for (v, f) in full.iter_mut().enumerate() {
        if sticks[v] != 0 {
            *f = G {
                seeds: pmask(&seeds),
                wallet: multi,
                ..G::default()
            };
        }
    }

    let share_keys: Vec<u64> = if shape.splits {
        split_plan(shape.keys, shape.m, a.omit)
            .iter()
            .map(|row| row.iter().fold(0, |acc, &k| acc | bit(k)))
            .collect()
    } else {
        Vec::new()
    };
    let keys = (0..shape.keys).fold(0, |acc, k| acc | bit(k));
    let cap = routes
        .iter()
        .filter(|r| open(shape, r))
        .map(|r| r.any - 1)
        .min()
        .unwrap_or(n);

    // The things, the description first, so that a stranger's reach is
    // known as soon as the seeds are placed. Each way of keeping the
    // description is tried in turn: its shares, or the sheet; where the
    // kinds need parts and the whole, both; where they need one of them,
    // that way first, then both (the sheet then kept somewhere, or both
    // would try the shares' way again).
    let shares: Vec<Th> = (0..shape.keys).map(Th::Share).collect();
    let both: Vec<Th> = std::iter::once(Th::Sheet).chain(shares.clone()).collect();
    let modes: Vec<(Vec<Th>, bool)> = if !multi {
        vec![(Vec::new(), false)]
    } else if !shape.splits {
        vec![(vec![Th::Sheet], false)]
    } else {
        match (need_part, need_whole) {
            (false, false) => vec![(shares, false), (vec![Th::Sheet], false)],
            (true, false) => vec![(shares, false), (both, true)],
            (false, true) => vec![(vec![Th::Sheet], false), (both, false)],
            (true, true) => vec![(both, false)],
        }
    };
    let items = written.iter().any(|&w| w != 0);
    // Seeds as files are things beside their words: their order is not
    // kept then.
    let files = files_at != 0;
    let seed_class = |i: usize| {
        (
            need_pass & bit(i) != 0,
            routes.iter().enumerate().fold(0u64, |acc, (k, r)| {
                acc | if r.passphrases.contains(&i) {
                    bit(k)
                } else {
                    0
                }
            }),
        )
    };
    let mut ordered = seeds.clone();
    ordered.sort_by_key(|&i| (seed_class(i), i));

    let mut search = Search {
        n,
        m: shape.m.max(1),
        multi,
        need_pass,
        keys,
        share_keys: &share_keys,
        routes: routes
            .iter()
            .map(|r| R {
                subsets: subsets(mask(&r.places), r.any),
                passwords: if r.paper { 0 } else { vmask(&r.passwords) },
                pass: pmask(&r.passphrases),
                paper: r.paper,
            })
            .collect(),
        sizes: (0..=reach.count_ones() as usize)
            .map(|k| subsets(reach, k))
            .collect(),
        cap,
        class,
        vaults,
        existing: shape.vaults,
        written_need,
        full,
        needs,
        things: Vec::new(),
        choice: Vec::new(),
        steps: 0,
        budget,
        stopped: false,
        best: None,
    };
    let thing = |th: Th, allowed: u32| Thing {
        th,
        allowed,
        after: None,
        must: 0,
        some: false,
    };
    for (mode, sheet_some) in modes {
        let mut things: Vec<Thing> = mode
            .into_iter()
            .map(|th| Thing {
                must: if th == Th::Sheet { sheet_at } else { 0 },
                some: th == Th::Sheet && sheet_some,
                ..thing(th, reach)
            })
            .collect();
        for (k, &i) in ordered.iter().enumerate() {
            let after = (k > 0 && !items && !files && seed_class(ordered[k - 1]) == seed_class(i))
                .then(|| things.len() - 1);
            things.push(Thing {
                after,
                ..thing(Th::Seed(i), reach)
            });
        }
        if files {
            for &i in &ordered {
                things.push(thing(Th::File(i), files_at));
            }
        }
        for (v, &s) in sticks.iter().enumerate() {
            if s != 0 {
                things.push(thing(Th::Stick(v), s));
            }
        }
        for (v, &w) in written.iter().enumerate() {
            if w != 0 {
                things.push(thing(Th::Written(v), w));
                for &i in seeds.iter().chain(multi.then_some(&WALLET)) {
                    things.push(thing(Th::Item(v, i), 1));
                }
            }
        }
        search.choice = vec![0; things.len()];
        search.things = things;
        if search.hopeful(0) {
            search.dfs(0);
        }
        if search.stopped {
            break;
        }
    }
    let Some((score, choice, things)) = search.best.clone() else {
        return (None, search.steps);
    };
    let out = Outcome {
        map: map_of(&search, &stored, &choice, &things),
        margin: score.margin,
        balance: score.balance,
        kept: score.kept,
        unsealed: score.unsealed,
        stopped: search.stopped,
        steps: search.steps,
    };
    (Some(out), search.steps)
}

/// The map of a placement: each place's paper and the sticks it keeps,
/// each vault and what it holds, then the stick of files, the software
/// and the seeds on their own devices as the plan `stored` has them.
fn map_of(s: &Search, stored: &[Spot], choice: &[u32], things: &[Thing]) -> Vec<Spot> {
    let mut places: Vec<Vec<What>> = vec![Vec::new(); s.n];
    let mut vaults: Vec<Option<Vec<What>>> = vec![None; s.vaults];
    let full = |v: usize| -> Vec<What> {
        let g = s.full.get(v).copied().unwrap_or_default();
        (0..64)
            .filter(|&i| g.seeds & bit(i) != 0)
            .map(What::Seed)
            .chain(g.wallet.then_some(What::Wallet))
            .collect()
    };
    // The order things go on a place: words, the description, written
    // passwords, then the vaults' sticks.
    let rank = |w: &What| match w {
        What::Words(_) => 0,
        What::Sheet | What::Share(_) => 1,
        What::VaultPassword(_) => 2,
        _ => 3,
    };
    for (k, t) in things.iter().enumerate() {
        let set = choice[k];
        match t.th {
            Th::Item(v, i) => {
                let held = vaults[v].get_or_insert_with(Vec::new);
                if set & 1 != 0 {
                    held.push(if i == WALLET {
                        What::Wallet
                    } else {
                        What::Seed(i)
                    });
                }
                continue;
            }
            Th::Stick(v) if set != 0 && vaults[v].is_none() => {
                vaults[v] = Some(full(v));
            }
            _ => {}
        }
        let what = match t.th {
            Th::Sheet => What::Sheet,
            Th::Share(j) => What::Share(j),
            Th::Seed(i) => What::Words(i),
            Th::File(i) => What::SeedFile(i),
            Th::Written(v) => What::VaultPassword(v),
            Th::Stick(v) => What::VaultStick(v),
            Th::Item(..) => continue,
        };
        for (p, held) in places.iter_mut().enumerate() {
            if set & (1 << p) != 0 {
                held.push(what);
            }
        }
    }
    let mut out: Vec<Spot> = Vec::new();
    for (p, mut held) in places.into_iter().enumerate() {
        held.sort_by_key(|w| rank(w));
        let at = At::Place(p);
        out.push(Spot {
            at,
            holds: held.into_iter().map(|w| (w, tag_of(at, w))).collect(),
        });
    }
    // A vault set up before the keys that no route needs stays, holding
    // nothing; one the plan made goes.
    for x in stored {
        if let At::Vault(v) = x.at
            && v < vaults.len().min(s.existing)
            && vaults[v].is_none()
        {
            vaults[v] = Some(Vec::new());
        }
    }
    for (v, held) in vaults.into_iter().enumerate() {
        if let Some(held) = held {
            let at = At::Vault(v);
            out.push(Spot {
                at,
                holds: held.into_iter().map(|w| (w, tag_of(at, w))).collect(),
            });
        }
    }
    out.extend(
        stored
            .iter()
            .filter(|x| matches!(x.at, At::Files | At::Software | At::Away))
            .cloned(),
    );
    out
}
