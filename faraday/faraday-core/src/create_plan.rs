//! Create plans the backup first (`docs/BACKUP-FLOW.md` §20.11, §20.12,
//! §20.14, F38): Create's Start, the Access layers card in Create, Suggest
//! a wallet and the Wallet card, and The standard way.
//!
//! Faraday never makes or chooses a key here. From the layers it suggests
//! a wallet's shape (how many keys, how many of them spend); the person
//! then fills every key on Keys the ways Create has always offered, and
//! only once the keys exist does Back up's Work it out place each seed and
//! the description's parts.
//!
//! The layers are said before there is a wallet, so the card edits a plan
//! for a nominal shape ([`planning`]) and Suggest a wallet works each shape
//! tried out on a copy of it ([`candidate`]). Everything a frame draws is
//! worked out on a press and kept here: drawing reads it.

use opensigner_core::strings::fill;

use crate::access::{Described, KindVault, Kinds, Layer, Route, SeedForm};
use crate::backup_flow::BackupFlow;
use crate::create::{NewKind, Source};
use crate::layers::{Conflict, LayersPress, conflicts};
use crate::plan::{self, Answers, At, Item, Shape, Spot};
use crate::strings::EN;
use crate::work_out::{self, BUDGET};
use crate::{Action, Faraday, cstep};

/// The way chosen on Create's Start (§20.14 item 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CreateWay {
    /// Plan the backup first: the Access layers card, then a wallet
    /// suggested.
    Plan,
    /// I know what I want: Kind, as Create has always opened.
    #[default]
    Know,
    /// The standard way: Kind as I know what I want, and the backup as a
    /// plain checklist (§20.11).
    Standard,
}

impl CreateWay {
    /// Every way, in the order Start offers them.
    pub const ALL: [CreateWay; 3] = [CreateWay::Plan, CreateWay::Know, CreateWay::Standard];

    /// Its name on Start.
    pub fn name(self) -> &'static str {
        match self {
            CreateWay::Plan => EN.create_plan_first,
            CreateWay::Know => EN.create_know,
            CreateWay::Standard => EN.create_standard,
        }
    }

    /// Its place in [`CreateWay::ALL`].
    pub fn index(self) -> u8 {
        CreateWay::ALL.iter().position(|w| *w == self).unwrap_or(0) as u8
    }
}

/// A wallet's shape Faraday tries (§20.12 item 2): how many keys, how many
/// of them spend, and for a single key whether it has a passphrase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalletShape {
    /// One key.
    Single,
    /// One key with a BIP-39 passphrase the layers remember.
    SinglePassphrase,
    /// Any 2 of 3 keys.
    TwoOfThree,
    /// Any 3 of 5 keys.
    ThreeOfFive,
}

impl WalletShape {
    /// Every shape tried, in the order tried.
    pub const ALL: [WalletShape; 4] = [
        WalletShape::Single,
        WalletShape::SinglePassphrase,
        WalletShape::TwoOfThree,
        WalletShape::ThreeOfFive,
    ];

    /// Signatures needed.
    pub fn m(self) -> usize {
        match self {
            WalletShape::Single | WalletShape::SinglePassphrase => 1,
            WalletShape::TwoOfThree => 2,
            WalletShape::ThreeOfFive => 3,
        }
    }

    /// Keys.
    pub fn n(self) -> usize {
        match self {
            WalletShape::Single | WalletShape::SinglePassphrase => 1,
            WalletShape::TwoOfThree => 3,
            WalletShape::ThreeOfFive => 5,
        }
    }

    /// Its key has a passphrase.
    pub fn passphrase(self) -> bool {
        self == WalletShape::SinglePassphrase
    }

    /// The kind Create makes it as: native SegWit, single key or multisig.
    pub fn kind(self) -> NewKind {
        if self.n() > 1 {
            NewKind::Multi
        } else {
            NewKind::NativeSegwit
        }
    }

    /// Its name: "Single key", "2 of 3".
    pub fn name(self) -> String {
        match self {
            WalletShape::Single => EN.create_shape_single.to_string(),
            WalletShape::SinglePassphrase => EN.create_shape_single_pass.to_string(),
            _ => fill(
                EN.create_shape_m_of_n,
                &[&self.m().to_string(), &self.n().to_string()],
            ),
        }
    }

    /// Its place in [`WalletShape::ALL`].
    pub fn index(self) -> u8 {
        WalletShape::ALL
            .iter()
            .position(|w| *w == self)
            .unwrap_or(0) as u8
    }

    /// The plan's shape for it: every key's seed loaded here, the first
    /// with a passphrase for [`WalletShape::SinglePassphrase`]. A lookup.
    pub fn shape(self) -> Shape {
        shape_of(self.m(), self.n(), self.passphrase())
    }
}

/// A plan's shape of `n` seeds loaded here, `m` of them to spend, the
/// first with a passphrase when `passphrase`. A lookup per seed.
fn shape_of(m: usize, n: usize, passphrase: bool) -> Shape {
    Shape {
        m,
        keys: n,
        seeds: (0..n)
            .map(|i| plan::Seed {
                name: format!("{}", i + 1),
                here: true,
                passphrase: passphrase && i == 0,
            })
            .collect(),
        splits: n > 1,
        vaults: 0,
    }
}

/// The nominal shape the Access layers card edits in Create, before any
/// wallet is chosen: up to five seeds a place may hold, any two of a
/// route's places to begin with, the description in parts. Nothing is
/// worked out for it; Suggest a wallet works the shapes tried out.
pub fn planning() -> Shape {
    shape_of(2, 5, false)
}

/// One shape as Suggest a wallet found it (§20.14 item 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeFit {
    /// The shape.
    pub shape: WalletShape,
    /// It meets every layer, each from its paper route, with nothing in
    /// conflict.
    pub fits: bool,
    /// Its one line: what it gives, or the conflict that stops it.
    pub line: String,
    /// The stranger margin of its best placement (§20.2).
    pub margin: usize,
    /// Things its best placement keeps.
    pub kept: usize,
    /// The search's steps for it.
    pub steps: u64,
    /// Its conflicts, for ranking the shapes that do not fit.
    pub conflicts: usize,
}

/// Plan the backup first's state in Create.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatePlan {
    /// The places and the layers, for the [`planning`] shape.
    pub answers: Answers,
    /// What Suggest a wallet found, best first, those that do not fit
    /// after; empty before it runs or after the layers change.
    pub fits: Vec<ShapeFit>,
    /// The steps Suggest a wallet took, every shape together.
    pub steps: u64,
    /// The shape chosen on the Wallet card.
    pub chosen: Option<WalletShape>,
    /// No shape fitted: the one whose conflicts the Access layers card
    /// shows, the fewest first.
    pub none: Option<WalletShape>,
    /// The shape on Kind and Quorum checked against the layers again: its
    /// conflict, a warning only (§20.14 item 4).
    pub warning: Option<String>,
}

impl CreatePlan {
    /// A fresh plan (§20.14 item 2): three places, one layer whose paper
    /// route is any 2 of them, each holding one seed in words and a part
    /// of the description. A lookup.
    pub fn fresh() -> CreatePlan {
        let shape = planning();
        let mut answers = Answers::empty(&shape);
        answers.add_place(&shape);
        answers.add_place(&shape);
        answers.layers = vec![Layer {
            routes: vec![Route {
                places: vec![0, 1, 2],
                any: 2,
                paper: true,
                kinds: Some(Kinds {
                    seeds: 1,
                    form: SeedForm::Paper,
                    description: Described::Part,
                    vault: KindVault::None,
                }),
                ..Route::default()
            }],
        }];
        CreatePlan {
            answers,
            fits: Vec::new(),
            steps: 0,
            chosen: None,
            none: None,
            warning: None,
        }
    }
}

/// `layers` as a wallet reads them: a route that remembers a passphrase
/// remembers the passphrase of a single key's seed when `passphrase`, and
/// nothing otherwise. A lookup per route.
fn candidate_layers(passphrase: bool, layers: &[Layer]) -> Vec<Layer> {
    layers
        .iter()
        .map(|l| Layer {
            routes: l
                .routes
                .iter()
                .map(|r| Route {
                    passphrases: if passphrase && !r.passphrases.is_empty() {
                        vec![0]
                    } else {
                        Vec::new()
                    },
                    ..r.clone()
                })
                .collect(),
        })
        .collect()
}

/// The plan worked out for a wallet of `shape`, its seed with a passphrase
/// when `passphrase`: its own answers, holding nothing, over the places of
/// `a`, and the layers of `a` as it reads them. A lookup per place and
/// route.
fn plan_for(shape: &Shape, passphrase: bool, a: &Answers) -> (Answers, Vec<Layer>) {
    let mut b = Answers::defaults(shape);
    b.omit = shape.m.saturating_sub(1);
    b.set_map(
        (0..a.places.max(1))
            .map(|p| Spot {
                at: At::Place(p),
                holds: Vec::new(),
            })
            .collect(),
    );
    (b, candidate_layers(passphrase, &a.layers))
}

/// The plan Suggest a wallet works out for shape `ws` ([`plan_for`]). A
/// lookup per place and route.
pub fn candidate(ws: WalletShape, a: &Answers) -> (Shape, Answers, Vec<Layer>) {
    let shape = ws.shape();
    let (b, layers) = plan_for(&shape, ws.passphrase(), a);
    (shape, b, layers)
}

/// "a stranger needs 2 places", or every place is not enough. A lookup.
fn stranger_line(margin: usize, places: usize) -> String {
    if margin >= places {
        EN.create_stranger_none.to_string()
    } else if margin == 0 {
        EN.create_stranger_place.to_string()
    } else {
        fill(EN.create_stranger_places, &[&(margin + 1).to_string()])
    }
}

/// Suggest a wallet (§20.12 item 2, §20.14 item 3): each shape tried
/// against the layers of `a`, its conflicts first ([`conflicts`], a lookup
/// per route) and, with none, the search, the shapes sharing one
/// [`BUDGET`] in turn: each is given an equal share of what the ones
/// before it left. The shapes
/// that fit come first, the largest stranger margin first, then the fewest
/// things kept, then the order tried; then those that do not, in the
/// order tried. Returns them and the steps taken. Costs up to [`BUDGET`]
/// steps of the search.
pub fn suggest(a: &Answers) -> (Vec<ShapeFit>, u64) {
    let place = |p: usize| BackupFlow::place_name(p);
    let mut used = 0u64;
    let mut found: Vec<ShapeFit> = Vec::new();
    for (k, ws) in WalletShape::ALL.into_iter().enumerate() {
        let (shape, b, layers) = candidate(ws, a);
        let named = |line: &str| fill(EN.create_shape_with, &[&ws.name(), line]);
        let stops = plain(ws, &layers, conflicts(&shape, &b, &layers, &place));
        if let Some(c) = stops.first() {
            let cause = if ws.passphrase() && none_remembers(&layers) {
                EN.create_no_route_remembers
            } else {
                &c.line
            };
            found.push(ShapeFit {
                shape: ws,
                fits: false,
                line: named(cause),
                margin: 0,
                kept: 0,
                steps: 0,
                conflicts: stops.len(),
            });
            continue;
        }
        // What is left, shared equally by this shape and those after it.
        let left = (WalletShape::ALL.len() - k) as u64;
        let budget = BUDGET.saturating_sub(used) / left;
        let (out, steps) = work_out::work_out_counted(&shape, &b, &layers, budget);
        used += steps;
        match out {
            Some(o) => {
                let keys = if ws.n() == 1 {
                    EN.create_key_to_make.to_string()
                } else {
                    fill(EN.create_keys_to_make, &[&ws.n().to_string()])
                };
                let mut line = fill(
                    EN.create_shape_line,
                    &[&ws.name(), &stranger_line(o.margin, a.places), &keys],
                );
                if o.stopped {
                    line = fill(EN.create_best_found, &[&line]);
                }
                found.push(ShapeFit {
                    shape: ws,
                    fits: true,
                    line,
                    margin: o.margin,
                    kept: o.kept,
                    steps,
                    conflicts: 0,
                });
            }
            None => found.push(ShapeFit {
                shape: ws,
                fits: false,
                line: named(if steps >= budget {
                    EN.create_none_found
                } else {
                    EN.create_unmet
                }),
                margin: 0,
                kept: 0,
                steps,
                conflicts: 1,
            }),
        }
    }
    let order = |f: &ShapeFit| f.shape.index();
    found.sort_by(|x, y| {
        (!x.fits, usize::MAX - x.margin, x.kept, order(x)).cmp(&(
            !y.fits,
            usize::MAX - y.margin,
            y.kept,
            order(y),
        ))
    });
    // Those that do not fit keep the order tried.
    let split = found.iter().position(|f| !f.fits).unwrap_or(found.len());
    found[split..].sort_by_key(order);
    (found, used)
}

/// The checklist of The standard way (§20.11): each seed loaded here
/// copied by hand and checked, the descriptor shown, and the wallet's
/// sheet made for the stick, to keep or print. A lookup per seed.
pub fn standard_items(shape: &Shape) -> Vec<Item> {
    let mut out: Vec<Item> = shape.here().into_iter().map(Item::Copy).collect();
    out.push(Item::ShowDescriptor);
    out.push(Item::Sheets);
    out
}

/// Whether `layers` keep anything in a vault: a route through one, or a
/// route whose places hold one. A lookup per route.
fn uses_vaults(layers: &[Layer]) -> bool {
    layers.iter().flat_map(|l| l.routes.iter()).any(|r| {
        !r.paper
            && (!r.passwords.is_empty()
                || !r.written.is_empty()
                || r.kinds.is_some_and(|k| k.vault != KindVault::None))
    })
}

/// The conflicts of a wallet of `ws` with `layers` said as their cause
/// in Create: for a single key with a passphrase, a route that can sign
/// with no seed is one that does not remember the passphrase, and its
/// resolution is that it does. Every other conflict as it is. A lookup per
/// conflict.
pub(crate) fn plain(ws: WalletShape, layers: &[Layer], found: Vec<Conflict>) -> Vec<Conflict> {
    if !ws.passphrase() {
        return found;
    }
    let mut out: Vec<Conflict> = Vec::new();
    for (l, layer) in layers.iter().enumerate() {
        for (r, route) in layer.routes.iter().enumerate() {
            if !route.passphrases.is_empty() {
                continue;
            }
            let name = fill(
                EN.layers_route_of,
                &[&crate::access::layer_name(&[], l), &(r + 1).to_string()],
            );
            out.push(Conflict {
                line: fill(EN.create_route_forgets, &[&name]),
                fixes: vec![(
                    fill(EN.create_route_remember, &[&name]),
                    LayersPress::Passphrase(l as u8, r as u8, 0),
                )],
            });
        }
    }
    // The "can sign with 0 seeds" lines are those above, said plainly.
    let said = !out.is_empty();
    let short = |c: &Conflict| {
        said && c.fixes.len() == 1 && matches!(c.fixes[0].1, LayersPress::Passphrase(..))
    };
    out.extend(found.into_iter().filter(|c| !short(c)));
    out
}

/// Whether no route of `layers` remembers a passphrase. A lookup per
/// route.
fn none_remembers(layers: &[Layer]) -> bool {
    layers
        .iter()
        .flat_map(|l| l.routes.iter())
        .all(|r| r.passphrases.is_empty())
}

impl Faraday {
    /// Whether Make a new wallet's card `k` waits for the Wallet card:
    /// Kind, Quorum and Keys on Plan the backup first before a shape is
    /// chosen (`docs/BACKUP-FLOW.md` §20.14 item 4). A lookup.
    pub fn create_waits_for_wallet(&self, k: u8) -> bool {
        self.create.as_ref().is_some_and(|c| {
            c.start
                && c.way == CreateWay::Plan
                && c.built.is_none()
                && c.plan.as_ref().is_some_and(|p| p.chosen.is_none())
                && matches!(k, cstep::KIND | cstep::QUORUM | cstep::KEYS)
        })
    }

    /// Whether the Access layers card on screen is Create's: Create is
    /// open, planning the backup first.
    pub(crate) fn create_planning(&self) -> bool {
        self.screen == crate::Screen::Create
            && self
                .create
                .as_ref()
                .is_some_and(|c| c.way == CreateWay::Plan && c.plan.is_some())
    }

    /// The plan Create's Access layers card edits, for the [`planning`]
    /// shape. A clone of it.
    pub(crate) fn create_layers_plan(&self) -> Option<(Shape, Answers)> {
        let p = self.create.as_ref()?.plan.as_ref()?;
        Some((planning(), p.answers.clone()))
    }

    /// The vaults Create's plan names: in its map or on a route. A lookup
    /// per route.
    fn create_named_vaults(a: &Answers, shape: &Shape) -> usize {
        let named = plan::stored_map(shape, a)
            .iter()
            .filter_map(|s| match s.at {
                At::Vault(v) => Some(v + 1),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        let routes = a
            .layers
            .iter()
            .flat_map(|l| l.routes.iter())
            .flat_map(|r| r.passwords.iter().chain(r.written.iter()))
            .map(|&v| v + 1)
            .max()
            .unwrap_or(0);
        named.max(routes)
    }

    /// The vaults Create's Access layers card has a password row for: those
    /// the plan names, at least one. A lookup per route.
    pub(crate) fn create_layers_vaults(a: &Answers, shape: &Shape) -> usize {
        Faraday::create_named_vaults(a, shape).max(1)
    }

    /// A press on Create's Access layers card, made to its plan: the same
    /// edits as Back up's, a route's passphrase remembered or not, a vault
    /// added as the next the plan names. What Suggest a wallet found is of
    /// the layers before; the shape chosen is checked again.
    pub(crate) fn create_layers_act(&mut self, press: LayersPress) {
        let shape = planning();
        let Some(p) = self.create.as_mut().and_then(|c| c.plan.as_mut()) else {
            return;
        };
        let a = &mut p.answers;
        match press {
            LayersPress::WorkItOut => return,
            LayersPress::Passphrase(l, r, _) => {
                if let Some(route) = a
                    .layers
                    .get_mut(usize::from(l))
                    .and_then(|x| x.routes.get_mut(usize::from(r)))
                {
                    route.passphrases = if route.passphrases.is_empty() {
                        vec![0]
                    } else {
                        Vec::new()
                    };
                }
            }
            LayersPress::AddVaultFor(l, r) => {
                let v = Faraday::create_named_vaults(a, &shape);
                if let Some(route) = a
                    .layers
                    .get_mut(usize::from(l))
                    .and_then(|x| x.routes.get_mut(usize::from(r)))
                {
                    if route.kinds.is_some_and(|k| k.vault == KindVault::Written) {
                        route.written.push(v);
                    } else {
                        route.passwords.push(v);
                    }
                }
            }
            _ => {
                let mut layers = a.layers_shown(&shape);
                crate::layers::edit(&mut layers, a, &shape, press);
                a.layers = layers;
            }
        }
        p.fits.clear();
        self.create_plan_recheck();
    }

    /// Suggest a wallet (§20.14 item 3): the shapes tried, kept for the
    /// Wallet card, which opens; with none that fits, the Access layers
    /// card stays, with the conflicts of the shape that has fewest. Runs
    /// [`suggest`]: up to [`BUDGET`] steps of the search, on the press.
    pub(crate) fn create_suggest(&mut self) {
        let Some(c) = self.create.as_mut() else {
            return;
        };
        if c.built.is_some() {
            return;
        }
        let Some(p) = c.plan.as_mut() else {
            return;
        };
        let (fits, steps) = suggest(&p.answers);
        p.none = (!fits.iter().any(|f| f.fits))
            .then(|| fits.iter().min_by_key(|f| (f.conflicts, f.shape.index())))
            .flatten()
            .map(|f| f.shape);
        p.steps = steps;
        let any = p.none.is_none();
        p.fits = fits;
        if any {
            c.done[cstep::LAYERS as usize] = true;
            c.open = Some(cstep::WALLET);
        } else {
            c.open = Some(cstep::LAYERS);
        }
        c.scroll.follow = true;
    }

    /// A shape chosen on the Wallet card (§20.14 item 4): Kind and Quorum
    /// filled from it and closed as done, the slots for its keys, all
    /// empty; where the layers keep nothing in a vault, the question of
    /// vaults before the keys answered no; then the next card, Keys. No key
    /// is made or chosen.
    pub(crate) fn create_choose(&mut self, k: u8) {
        let Some(&ws) = WalletShape::ALL.get(usize::from(k)) else {
            return;
        };
        let owner_changed = {
            let Some(c) = self.create.as_mut() else {
                return;
            };
            if c.built.is_some() {
                return;
            }
            let Some(p) = c.plan.as_mut() else {
                return;
            };
            if !p.fits.iter().any(|f| f.shape == ws && f.fits) {
                return;
            }
            // The same shape chosen again keeps the keys chosen for it.
            let same = p.chosen == Some(ws) && (c.kind, c.m, c.n) == (ws.kind(), ws.m(), ws.n());
            p.chosen = Some(ws);
            p.warning = None;
            let vaults = uses_vaults(&p.answers.layers);
            if !same {
                c.kind = ws.kind();
                c.m = ws.m();
                c.n = ws.n();
                c.slots = vec![Source::Empty; ws.n()];
                c.close_pass();
            }
            for s in [cstep::WALLET, cstep::KIND, cstep::QUORUM] {
                c.done[s as usize] = true;
            }
            let answer = (!vaults && c.vaults.answer.is_none()).then_some(c.owner);
            if answer.is_some() {
                c.vaults.answer = Some(false);
                c.done[cstep::WHERE as usize] = true;
            }
            answer
        };
        if let Some(owner) = owner_changed {
            self.wallet_vaults_changed(owner);
        }
        self.create_open_next();
    }

    /// The shape on Kind and Quorum, once a shape was chosen, checked
    /// against the layers again (§20.14 item 4): its first conflict, or
    /// that no placement meets every route, kept as a warning. Costs the
    /// conflicts' lookups and up to a quarter of [`BUDGET`] steps of the
    /// search.
    pub(crate) fn create_plan_recheck(&mut self) {
        let Some(c) = self.create.as_ref() else {
            return;
        };
        let Some(p) = c.plan.as_ref() else {
            return;
        };
        if p.chosen.is_none() || c.way != CreateWay::Plan {
            return;
        }
        let pass = p.chosen.is_some_and(WalletShape::passphrase) && !c.kind.multi();
        let (m, n) = if c.kind.multi() { (c.m, c.n) } else { (1, 1) };
        let shape = shape_of(m, n, pass);
        let (b, layers) = plan_for(&shape, pass, &p.answers);
        let place = |q: usize| BackupFlow::place_name(q);
        let name = if n > 1 {
            fill(EN.create_shape_m_of_n, &[&m.to_string(), &n.to_string()])
        } else if pass {
            EN.create_shape_single_pass.to_string()
        } else {
            EN.create_shape_single.to_string()
        };
        let ws = if pass {
            WalletShape::SinglePassphrase
        } else {
            WalletShape::Single
        };
        let warning = match plain(ws, &layers, conflicts(&shape, &b, &layers, &place)).first() {
            Some(c) => Some(fill(EN.create_shape_with, &[&name, &c.line])),
            None => work_out::work_out(&shape, &b, &layers, BUDGET / 4)
                .is_none()
                .then(|| fill(EN.create_shape_with, &[&name, EN.create_unmet])),
        };
        if let Some(p) = self.create.as_mut().and_then(|c| c.plan.as_mut()) {
            p.warning = warning;
        }
    }

    /// The way on Create's Start ticked (§20.14 item 1); Plan the backup
    /// first starts a fresh plan the first time.
    pub(crate) fn create_way(&mut self, k: u8) {
        let Some(&way) = CreateWay::ALL.get(usize::from(k)) else {
            return;
        };
        if let Some(c) = self.create.as_mut()
            && c.built.is_none()
        {
            c.way = way;
            if way == CreateWay::Plan && c.plan.is_none() {
                c.plan = Some(CreatePlan::fresh());
            }
        }
    }

    /// Start's Continue: Plan the backup first opens the Access layers
    /// card; the other ways open Kind, as Create always has.
    pub(crate) fn create_start_next(&mut self) {
        let Some(c) = self.create.as_mut() else {
            return;
        };
        c.done[cstep::START as usize] = true;
        if c.way == CreateWay::Plan {
            if c.plan.is_none() {
                c.plan = Some(CreatePlan::fresh());
            }
            c.open = Some(cstep::LAYERS);
            c.scroll.follow = true;
            return;
        }
        self.create_open_next();
    }

    /// Create's Back up, planned (§20.14 item 5): Back up a wallet on the
    /// wallet just made with the plan's places and layers, a passphrase
    /// remembered being its seeds' passphrases, and Work it out run on its
    /// keys: with nothing in conflict the board opens filled. Runs the
    /// search ([`crate::layers`]'s Work it out), on the press.
    pub(crate) fn create_backup_planned(&mut self) {
        let Some((i, p)) = self
            .create
            .as_ref()
            .and_then(|c| Some((c.built?, c.plan.clone()?)))
        else {
            return;
        };
        self.act(Action::Backup(i));
        if self.backup.as_ref().is_none_or(|b| b.wallet != i) {
            return;
        }
        let shape = self.plan_shape(i);
        let mut a = Answers::empty(&shape);
        for _ in 1..p.answers.places.max(1) {
            a.add_place(&shape);
        }
        let pass: Vec<usize> = (0..shape.seeds.len())
            .filter(|&k| shape.seeds[k].here && shape.seeds[k].passphrase)
            .collect();
        a.layers = p
            .answers
            .layers
            .iter()
            .map(|l| Layer {
                routes: l
                    .routes
                    .iter()
                    .map(|r| Route {
                        passphrases: if r.passphrases.is_empty() {
                            Vec::new()
                        } else {
                            pass.clone()
                        },
                        ..r.clone()
                    })
                    .collect(),
            })
            .collect();
        if let Some(b) = self.backup.as_mut() {
            b.from_create = true;
            b.answers = a;
            b.q = Some(crate::qstep::LAYERS);
            b.scroll.follow = true;
        }
        self.layers_act(LayersPress::WorkItOut);
    }

    /// Back up's The standard way (§20.11): the checklist of each seed's
    /// copy, the descriptor shown and the wallet's sheet, with no places,
    /// layers or board. Makes the sheet for the stick
    /// (`backup_files_make`).
    pub(crate) fn backup_standard(&mut self) {
        if self.backup_locked_flow().is_some() {
            return self.toast(EN.backup_flow_past_lock);
        }
        let Some(b) = self.backup.as_mut() else {
            return;
        };
        b.standard = true;
        b.stage = crate::BStage::Checklist;
        b.naming = None;
        b.scroll = crate::flow::Scroll::default();
        self.backup_files_make();
        let first = self
            .backup_items()
            .into_iter()
            .find(|&n| crate::bstep::item(n).is_some_and(|it| !self.backup_item_done(it)));
        self.backup_item_open(first);
    }

    /// Create's Back up the standard way: Back up a wallet on the wallet
    /// just made, its checklist the standard way's.
    pub(crate) fn create_backup_standard(&mut self) {
        let Some(i) = self.create.as_ref().and_then(|c| c.built) else {
            return;
        };
        self.act(Action::Backup(i));
        if let Some(b) = self.backup.as_mut() {
            b.from_create = true;
        }
        self.backup_standard();
    }

    /// The line under a Keys slot of a single key with a passphrase whose
    /// key has none (§20.14 item 4): the passphrase is asked on its Key
    /// card, or by Add a passphrase. A lookup.
    pub(crate) fn create_slot_passphrase_line(&self, slot: usize) -> Option<String> {
        let c = self.create.as_ref()?;
        let p = c.plan.as_ref()?;
        if c.way != CreateWay::Plan
            || !p.chosen.is_some_and(WalletShape::passphrase)
            || c.kind.multi()
        {
            return None;
        }
        let Source::Here(fp) = c.slots.get(slot)? else {
            return None;
        };
        let key = self
            .session
            .keys
            .iter()
            .find(|k| k.fingerprint().0 == *fp)?;
        key.passphrase
            .is_none()
            .then(|| fill(EN.create_no_passphrase, &[&(slot + 1).to_string()]))
    }
}
