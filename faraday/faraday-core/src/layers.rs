//! The plan's Access layers card (`docs/BACKUP-FLOW.md` §20.3, F38, F46):
//! its presses, what Work it out does with the board's plan, the
//! conflicts of §20.4 and what would resolve each, and the card as one
//! frame draws it ([`LayersView`], worked out once per frame through the
//! drawing memo, `docs/SPEED.md` §3.3). A press computes from the state as
//! it is, never from the memo. The drawing is `layers_screen.rs`; the
//! search is `work_out.rs`.

use std::rc::Rc;

use opensigner_core::strings::fill;

use crate::Faraday;
use crate::access::{Described, KindVault, Kinds, Layer, Route, SeedForm, layer_name};
use crate::plan::{self, Answers, Shape, Spot};
use crate::strings::EN;
use crate::work_out::{self, BUDGET, Fit, Outcome, Why};

/// A press on the Access layers card. Layers, routes, places, vaults and
/// seeds by number from 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayersPress {
    /// **+ layer**: a layer as a fresh plan has it.
    AddLayer,
    /// The × on a layer.
    RemoveLayer(u8),
    /// **+ route** on a layer: any `m` of every place.
    AddRoute(u8),
    /// The × on a route.
    RemoveRoute(u8, u8),
    /// The route's stepper: one place fewer is enough.
    Fewer(u8, u8),
    /// The route's stepper: one place more is needed.
    More(u8, u8),
    /// A place of the route ticked or unticked.
    Place(u8, u8, u8),
    /// The route a paper route, or not.
    Paper(u8, u8),
    /// A vault's password on the route: not known, then remembered, then
    /// written down, then not known again.
    Password(u8, u8, u8),
    /// A seed's passphrase remembered on the route, or not.
    Passphrase(u8, u8, u8),
    /// **Work it out** (§20.3).
    WorkItOut,
    /// A resolution: a place added to the route, a new one where every
    /// place is in it already, and at least two of its places needed.
    AddPlaceTo(u8, u8),
    /// A resolution: a new vault whose password the route knows.
    AddVaultFor(u8, u8),
    /// A resolution: a paper route for the layer.
    AddPaperRoute(u8),
    /// The route's kinds set by hand (§20.13), or left to Work it out.
    KindsSet(u8, u8),
    /// The kinds' stepper: one seed fewer at each place.
    KindsFewerSeeds(u8, u8),
    /// The kinds' stepper: one seed more at each place.
    KindsMoreSeeds(u8, u8),
    /// A resolution: so many seeds at each place.
    KindsSeeds(u8, u8, u8),
    /// The seeds' form at each place.
    KindsForm(u8, u8, SeedForm),
    /// How much of the description each place holds.
    KindsDescription(u8, u8, Described),
    /// A vault at each place, and how its password is had.
    KindsVault(u8, u8, KindVault),
    /// A resolution: any so many of the route's places, a new place
    /// added to it where it has too few.
    KindsAny(u8, u8, u8),
}

/// What Work it out found, kept on the board's state while the plan is as
/// it left it (§20.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worked {
    /// The map it put on the board, as stored.
    pub map: Vec<Spot>,
    /// The layers it worked to.
    pub layers: Vec<Layer>,
    /// The stranger margin it found.
    pub margin: usize,
    /// It ran out of steps: the best found.
    pub stopped: bool,
    /// The shape these routes need, where every placement in this one
    /// lets one place spend (§20.12 item 6), or that no shape tried does
    /// better.
    pub shape_line: Option<String>,
}

/// A conflict (§20.4): its line, and each press that would resolve it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    /// One line naming the layer and why.
    pub line: String,
    /// What would resolve it: a label and the press.
    pub fixes: Vec<(String, LayersPress)>,
}

/// "Place 1, Place 3": places by name, joined.
fn places_text(places: &[usize], name: &dyn Fn(usize) -> String) -> String {
    places
        .iter()
        .map(|&p| name(p))
        .collect::<Vec<_>>()
        .join(", ")
}

fn u(n: usize) -> u8 {
    u8::try_from(n).unwrap_or(u8::MAX)
}

/// The conflicts of `layers` in plan `a` for `shape` that can be read
/// from the layers alone (§20.4, §20.12 item 3): a layer with no paper
/// route; a route that can sign with fewer seeds than the wallet needs; a
/// route that one of its places alone meets, with nothing remembered,
/// which a stranger there meets too. Each with the presses that would
/// resolve it, the first the likeliest. A lookup per route.
pub fn conflicts(
    shape: &Shape,
    a: &Answers,
    layers: &[Layer],
    place: &dyn Fn(usize) -> String,
) -> Vec<Conflict> {
    let vaults = a.vault_count(shape);
    let mut out = Vec::new();
    for (l, layer) in layers.iter().enumerate() {
        let name = layer_name(&[], l);
        if !layer.routes.iter().any(|r| r.paper) {
            out.push(Conflict {
                line: fill(EN.layers_no_paper, &[&name]),
                fixes: vec![(
                    fill(EN.layers_give_paper, &[&name]),
                    LayersPress::AddPaperRoute(u(l)),
                )],
            });
        }
        for (r, route) in layer.routes.iter().enumerate() {
            let remember = |out: &mut Vec<(String, LayersPress)>| {
                for i in shape.here() {
                    if shape.seeds[i].passphrase && !route.passphrases.contains(&i) {
                        out.push((
                            fill(EN.layers_remember, &[&name, &shape.seeds[i].name]),
                            LayersPress::Passphrase(u(l), u(r), u(i)),
                        ));
                    }
                }
            };
            let can = work_out::usable(shape, route);
            if can < shape.m.max(1) {
                let mut fixes = Vec::new();
                remember(&mut fixes);
                out.push(Conflict {
                    line: fill(
                        EN.layers_short,
                        &[&name, &can.to_string(), &shape.m.max(1).to_string()],
                    ),
                    fixes,
                });
                continue;
            }
            if route.any > 1 || route.places.is_empty() || !work_out::open(shape, route) {
                continue;
            }
            let line = match route.places.as_slice() {
                [p] => {
                    let p = place(*p);
                    fill(EN.layers_alone, &[&name, &p, &p, &p])
                }
                ps => fill(EN.layers_any_one, &[&name, &places_text(ps, place)]),
            };
            let mut fixes = Vec::new();
            match (0..a.places).find(|p| !route.places.contains(p)) {
                Some(p) => fixes.push((
                    fill(EN.layers_add_place, &[&place(p), &name]),
                    LayersPress::AddPlaceTo(u(l), u(r)),
                )),
                None => fixes.push((
                    fill(EN.layers_new_place, &[&name]),
                    LayersPress::AddPlaceTo(u(l), u(r)),
                )),
            }
            if !route.paper {
                // A vault whose password it has written down cycles off
                // on a press: the next one it has not.
                match (0..vaults).find(|v| !route.written.contains(v)) {
                    Some(v) => fixes.push((
                        fill(EN.layers_knows, &[&name, &(v + 1).to_string()]),
                        LayersPress::Password(u(l), u(r), u(v)),
                    )),
                    None => fixes.push((
                        fill(EN.layers_add_vault, &[&name]),
                        LayersPress::AddVaultFor(u(l), u(r)),
                    )),
                }
            }
            remember(&mut fixes);
            out.push(Conflict { line, fixes });
        }
    }
    out.extend(kinds_conflicts(shape, layers));
    out
}

/// The conflicts of the routes' kinds with the wallet's shape (§20.13
/// item 6, [`work_out::problems`]), each naming its route, with what
/// would resolve it: more seeds at each place, more places, the
/// description whole, a passphrase remembered, and for a stranger who
/// could spend from one place, fewer seeds there or a part of the
/// description. A lookup per route.
fn kinds_conflicts(shape: &Shape, layers: &[Layer]) -> Vec<Conflict> {
    let mut out = Vec::new();
    let has = shape.here().len();
    let m = shape.m.max(1);
    for p in work_out::problems(shape, layers) {
        let Some((l, r)) = p.at else {
            continue;
        };
        let Some(route) = layers.get(l).and_then(|x| x.routes.get(r)) else {
            continue;
        };
        let Some(k) = route.kinds else {
            continue;
        };
        let name = fill(
            EN.layers_route_of,
            &[&layer_name(&[], l), &(r + 1).to_string()],
        );
        let (l8, r8) = (u(l), u(r));
        let seeds_to = |n: usize| {
            let label = if n == 1 {
                fill(EN.layers_seed_at_each, &[&name])
            } else {
                fill(EN.layers_seeds_at_each, &[&n.to_string(), &name])
            };
            (label, LayersPress::KindsSeeds(l8, r8, u(n)))
        };
        let whole = (
            fill(EN.layers_whole_each, &[&name]),
            LayersPress::KindsDescription(l8, r8, Described::Whole),
        );
        let mut fixes = Vec::new();
        let line = match p.why {
            Why::Impossible | Why::Unmet => continue,
            Why::Short { any, per, needs } => {
                let more = needs.div_ceil(any.max(1));
                if more <= has {
                    fixes.push(seeds_to(more));
                }
                if per > 0 {
                    let k = needs.div_ceil(per);
                    fixes.push((
                        fill(EN.layers_any_of, &[&k.to_string(), &name]),
                        LayersPress::KindsAny(l8, r8, u(k)),
                    ));
                }
                fill(
                    EN.layers_kinds_short,
                    &[
                        &name,
                        &any.to_string(),
                        &per.to_string(),
                        &(any * per).to_string(),
                        &needs.to_string(),
                    ],
                )
            }
            Why::TooMany { per, has } => {
                if has > 0 {
                    fixes.push(seeds_to(has));
                }
                fill(
                    EN.layers_kinds_too_many,
                    &[&name, &per.to_string(), &has.to_string()],
                )
            }
            Why::NoDescription => {
                fixes.push(whole);
                if shape.splits && route.any > 1 {
                    fixes.push((
                        fill(EN.layers_part_each, &[&name]),
                        LayersPress::KindsDescription(l8, r8, Described::Part),
                    ));
                }
                fill(EN.layers_kinds_no_description, &[&name])
            }
            Why::PartAlone => {
                fixes.push(whole);
                fill(EN.layers_kinds_part_alone, &[&name, &route.any.to_string()])
            }
            Why::NoPassword => {
                fixes.push((
                    fill(EN.layers_add_vault, &[&name]),
                    LayersPress::AddVaultFor(l8, r8),
                ));
                fill(EN.layers_kinds_no_password, &[&name])
            }
            Why::StrangerAlone => {
                if !route.paper && k.vault == KindVault::Written && route.any <= 1 {
                    fixes.push((
                        fill(EN.layers_vault_remembered, &[&name]),
                        LayersPress::KindsVault(l8, r8, KindVault::Remembered),
                    ));
                } else {
                    // Fewer seeds than a quorum at each place, past those
                    // that need a passphrase.
                    let guarded = shape
                        .here()
                        .into_iter()
                        .filter(|&i| shape.seeds[i].passphrase)
                        .count();
                    let fewer = (m - 1 + guarded).min(k.seeds.saturating_sub(1));
                    if fewer >= 1 && fewer < k.seeds {
                        fixes.push(seeds_to(fewer));
                    }
                    if shape.multi()
                        && shape.splits
                        && route.any > 1
                        && k.description == Described::Whole
                    {
                        fixes.push((
                            fill(EN.layers_part_each, &[&name]),
                            LayersPress::KindsDescription(l8, r8, Described::Part),
                        ));
                    }
                    for i in shape.here() {
                        if shape.seeds[i].passphrase && !route.passphrases.contains(&i) {
                            fixes.push((
                                fill(
                                    EN.layers_remember,
                                    &[&layer_name(&[], l), &shape.seeds[i].name],
                                ),
                                LayersPress::Passphrase(l8, r8, u(i)),
                            ));
                        }
                    }
                }
                fill(EN.layers_kinds_stranger, &[&name])
            }
        };
        out.push(Conflict { line, fixes });
    }
    out
}

/// Whether `layers` have a kinds conflict that another shape of wallet
/// might not: too few seeds from a route's places, or more at each than
/// the wallet has (§20.13 item 5). A lookup per route.
fn shape_bound(shape: &Shape, layers: &[Layer]) -> bool {
    work_out::problems(shape, layers)
        .iter()
        .any(|p| matches!(p.why, Why::Short { .. } | Why::TooMany { .. }))
}

/// A shape's name: "single key", "2 of 3". A lookup.
fn shape_name(m: usize, n: usize) -> String {
    if n <= 1 {
        EN.layers_single_key.to_string()
    } else {
        fill(EN.layers_m_of_n, &[&m.to_string(), &n.to_string()])
    }
}

/// Where every placement in `shape` lets one place spend: the first of
/// the shapes §20.12 item 2 tries (single key, 2 of 3, 3 of 5) other than
/// this one whose best placement keeps every place from spending alone,
/// named, or the line that none does; for routes whose kinds the shape
/// cannot give (`kinds`), the first shape whose kinds conflicts are none
/// and that keeps every place from spending alone, or nothing. Runs
/// [`work_out::fit`] once per shape tried, each with an eighth of the
/// budget.
pub fn shape_needed(shape: &Shape, a: &Answers, layers: &[Layer], kinds: bool) -> Option<String> {
    let here = shape.seeds.len();
    for (m, n) in [(1, 1), (2, 3), (3, 5)] {
        if (shape.m, here) == (m, n) {
            continue;
        }
        let other = Shape {
            m,
            keys: n,
            seeds: (0..n)
                .map(|i| plan::Seed {
                    name: format!("{i}"),
                    here: true,
                    passphrase: false,
                })
                .collect(),
            splits: n > 1,
            vaults: shape.vaults,
        };
        let mut b = Answers::defaults(&other);
        b.omit = m.saturating_sub(1);
        b.set_map(plan::stored_map(shape, a));
        let fitted: Vec<Layer> = layers
            .iter()
            .map(|l| Layer {
                routes: l
                    .routes
                    .iter()
                    .map(|r| Route {
                        passphrases: Vec::new(),
                        ..r.clone()
                    })
                    .collect(),
            })
            .collect();
        if matches!(work_out::fit(&other, &b, &fitted, BUDGET / 8), Fit::Placed(o) if o.margin >= 1)
        {
            return Some(fill(
                EN.layers_shape,
                &[&shape_name(m, n), &shape_name(shape.m, here)],
            ));
        }
    }
    (!kinds).then(|| EN.layers_one_place.to_string())
}

/// One route of a layer, as the card draws it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RouteView {
    /// "Route 1", "Route 2 · paper".
    pub title: String,
    /// The stepper's label and value: "Any of its 3 places", 2.
    pub any: (String, usize),
    /// Each place: its name and whether the route holds it.
    pub places: Vec<(String, bool)>,
    /// A paper route.
    pub paper: bool,
    /// Each vault's password: its label and its state's.
    pub passwords: Vec<(String, String)>,
    /// Each passphrase of a seed loaded here: its row and whether it is
    /// remembered.
    pub passphrases: Vec<(String, bool, usize)>,
    /// What each of its places holds, when set by hand (§20.13).
    pub kinds: Option<KindsView>,
}

/// A route's kinds, as the card draws them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct KindsView {
    /// Seeds at each place.
    pub seeds: usize,
    /// Their form; none on a paper route, where it is words on paper.
    pub form: Option<SeedForm>,
    /// The description at each place; none for a wallet of one seed.
    pub description: Option<Described>,
    /// The vault at each place; none on a paper route.
    pub vault: Option<KindVault>,
}

/// One layer, as the card draws it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LayerView {
    /// "Access layer 1".
    pub name: String,
    /// Its routes.
    pub routes: Vec<RouteView>,
}

/// The Access layers card as one frame draws it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LayersView {
    /// The layers.
    pub layers: Vec<LayerView>,
    /// The conflicts, a line each with what would resolve it.
    pub conflicts: Vec<Conflict>,
    /// Work it out is offered: before the lock.
    pub work: bool,
}

impl Faraday {
    /// Works out the Access layers card for the plan the board edits,
    /// once a frame (the drawing memo, `docs/SPEED.md` §3.3). Drawing
    /// reads it; nothing decides from it.
    pub(crate) fn layers_view(&self) -> Option<Rc<LayersView>> {
        if self.drawing.is_on() {
            return self
                .drawing
                .layers
                .get_or_init(|| self.layers_view_uncached().map(Rc::new))
                .clone();
        }
        self.layers_view_uncached().map(Rc::new)
    }

    fn layers_view_uncached(&self) -> Option<LayersView> {
        // In Create the card edits the plan said before the wallet
        // (`docs/BACKUP-FLOW.md` §20.14): its places are "Place n", and a
        // route remembers a passphrase, whichever seed comes to have one.
        let create = self.create_planning();
        let (shape, a) = if create {
            self.create_layers_plan()?
        } else {
            self.board_plan()?
        };
        let layers = a.layers_shown(&shape);
        let vaults = if create {
            Faraday::create_layers_vaults(&a, &shape)
        } else {
            a.vault_count(&shape)
        };
        let place = |p: usize| {
            if create {
                crate::backup_flow::BackupFlow::place_name(p)
            } else {
                self.board_place_name(p)
            }
        };
        let views = layers
            .iter()
            .enumerate()
            .map(|(l, layer)| LayerView {
                name: layer_name(&[], l),
                routes: layer
                    .routes
                    .iter()
                    .enumerate()
                    .map(|(r, route)| RouteView {
                        title: fill(
                            if route.paper {
                                EN.layers_route_paper
                            } else {
                                EN.layers_route
                            },
                            &[&(r + 1).to_string()],
                        ),
                        any: (
                            fill(EN.layers_any, &[&route.places.len().to_string()]),
                            route.any,
                        ),
                        places: (0..a.places)
                            .map(|p| (place(p), route.places.contains(&p)))
                            .collect(),
                        paper: route.paper,
                        passwords: (0..vaults)
                            .map(|v| {
                                let state = if route.passwords.contains(&v) {
                                    EN.layers_remembered
                                } else if route.written.contains(&v) {
                                    EN.layers_written
                                } else {
                                    EN.layers_not_known
                                };
                                (
                                    fill(EN.layers_password, &[&(v + 1).to_string()]),
                                    state.to_string(),
                                )
                            })
                            .collect(),
                        passphrases: if create {
                            vec![(
                                EN.layers_remembers_passphrase.to_string(),
                                !route.passphrases.is_empty(),
                                0,
                            )]
                        } else {
                            shape
                                .here()
                                .into_iter()
                                .filter(|&i| shape.seeds[i].passphrase)
                                .map(|i| {
                                    (
                                        fill(EN.layers_remembers, &[&shape.seeds[i].name]),
                                        route.passphrases.contains(&i),
                                        i,
                                    )
                                })
                                .collect()
                        },
                        kinds: route.kinds.map(|k| KindsView {
                            seeds: k.seeds,
                            form: (!route.paper).then_some(k.form),
                            description: shape.multi().then_some(k.description),
                            vault: (!route.paper).then_some(k.vault),
                        }),
                    })
                    .collect(),
            })
            .collect();
        if create {
            return Some(LayersView {
                layers: views,
                conflicts: self.create_conflicts(&a, &place),
                work: false,
            });
        }
        let mut found = conflicts(&shape, &a, &layers, &place);
        // What Work it out found that the layers alone do not say, while
        // the plan is as it left it.
        if let Some(line) = self
            .board_state()
            .and_then(|st| st.worked.as_ref())
            .filter(|w| w.layers == layers && w.map == plan::stored_map(&shape, &a))
            .and_then(|w| w.shape_line.clone())
        {
            found.push(Conflict {
                line,
                fixes: Vec::new(),
            });
        }
        Some(LayersView {
            layers: views,
            conflicts: found,
            work: self.board_locked().is_none(),
        })
    }

    /// The conflicts Create's Access layers card shows (§20.14 item 3):
    /// none until Suggest a wallet found no shape that fits, then those of
    /// the shape with fewest, or that no shape tried meets every layer.
    /// A lookup per route.
    fn create_conflicts(&self, a: &Answers, place: &dyn Fn(usize) -> String) -> Vec<Conflict> {
        let Some(ws) = self
            .create
            .as_ref()
            .and_then(|c| c.plan.as_ref())
            .and_then(|p| p.none)
        else {
            return Vec::new();
        };
        let (shape, b, layers) = crate::create_plan::candidate(ws, a);
        let mut found =
            crate::create_plan::plain(ws, &layers, conflicts(&shape, &b, &layers, place));
        if found.is_empty() {
            found.push(Conflict {
                line: EN.create_none_fits.to_string(),
                fixes: Vec::new(),
            });
        }
        found
    }

    /// The summary line of the Access layers card while it is closed.
    /// Costs a lookup.
    pub(crate) fn layers_summary(shape: &Shape, a: &Answers) -> String {
        let n = a.layers_shown(shape).len();
        if n == 1 {
            EN.layers_one.to_string()
        } else {
            fill(EN.layers_many, &[&n.to_string()])
        }
    }

    /// A press on the Access layers card, made to the plan the board
    /// edits (Back up's, or a wallet flow's), from the state as it is.
    pub(crate) fn layers_act(&mut self, press: LayersPress) {
        use LayersPress as P;
        if self.create_planning() {
            return self.create_layers_act(press);
        }
        if press == P::WorkItOut {
            return self.layers_work_out();
        }
        self.board_change(&|a, shape| {
            let mut layers = a.layers_shown(shape);
            edit(&mut layers, a, shape, press);
            a.layers = layers;
        });
    }

    /// Work it out (§20.3): the search on the plan the board edits, its
    /// result put on the board and kept, with what it found, on the
    /// board's state. Runs [`work_out::work_out`], and where every
    /// placement lets one place spend, the other shapes too
    /// ([`shape_needed`]). Never while drawing.
    fn layers_work_out(&mut self) {
        if self.board_locked().is_some() {
            return;
        }
        let Some((shape, a)) = self.board_plan() else {
            return;
        };
        let layers = a.layers_shown(&shape);
        // Kinds the wallet's shape cannot give name the shape they need
        // (§20.13 item 5).
        let kinds_line = shape_bound(&shape, &layers)
            .then(|| shape_needed(&shape, &a, &layers, true))
            .flatten();
        let Some(out) = work_out::work_out(&shape, &a, &layers, BUDGET) else {
            // No route can be met at all: the conflicts say why; the
            // board stays as it is.
            self.board_change(&|a, shape| a.layers = a.layers_shown(shape));
            if let Some((shape, a)) = self.board_plan() {
                let worked = Worked {
                    map: plan::stored_map(&shape, &a),
                    layers: layers.clone(),
                    margin: 0,
                    stopped: false,
                    shape_line: kinds_line,
                };
                if let Some(st) = self.board_state_mut() {
                    st.worked = Some(worked);
                }
            }
            return;
        };
        // A margin of 0 that no conflict of the layers explains is the
        // wallet's shape's (§20.12 item 6).
        let place = |p: usize| self.board_place_name(p);
        let explained = !conflicts(&shape, &a, &layers, &place).is_empty();
        let shape_line = kinds_line.or_else(|| {
            (out.margin == 0 && !explained)
                .then(|| shape_needed(&shape, &a, &layers, false))
                .flatten()
        });
        let put = |a: &mut Answers, shape: &Shape, out: &Outcome| {
            a.layers = layers.clone();
            crate::board::map_edit(a, shape, |_| out.map.clone());
        };
        self.board_change(&|a, shape| put(a, shape, &out));
        let Some((shape, a)) = self.board_plan() else {
            return;
        };
        let worked = Worked {
            map: plan::stored_map(&shape, &a),
            layers,
            margin: out.margin,
            stopped: out.stopped,
            shape_line,
        };
        let clean = {
            let place = |p: usize| self.board_place_name(p);
            conflicts(&shape, &a, &worked.layers, &place).is_empty() && worked.shape_line.is_none()
        };
        if let Some(st) = self.board_state_mut() {
            st.worked = Some(worked);
            st.held = None;
            st.forms = false;
        }
        // The result fills the board (§20.3): with nothing in conflict
        // the board opens on it.
        if clean {
            self.layers_open_board();
        }
    }

    /// The board's card opened, on Back up or the wallet flow.
    fn layers_open_board(&mut self) {
        match self.board_of() {
            Some(crate::board::BoardOf::Flow(i)) => {
                if let Some(f) = self.backup_flows.get_mut(i) {
                    f.live.plan_q = Some(crate::qstep::BOARD);
                }
            }
            Some(crate::board::BoardOf::Backup) => {
                if let Some(b) = self.backup.as_mut() {
                    b.q = Some(crate::qstep::BOARD);
                    b.scroll.follow = true;
                }
            }
            None => {}
        }
    }
}

/// Press `press` made to `layers`, the layers of plan `a` for `shape`: a
/// layer or route added or taken off, a route's places, threshold, paper,
/// passwords and passphrases, and the resolutions of §20.4. A new place
/// or vault a resolution needs is added to `a`. A lookup per route.
pub(crate) fn edit(layers: &mut Vec<Layer>, a: &mut Answers, shape: &Shape, press: LayersPress) {
    use LayersPress as P;
    let n = |x: u8| usize::from(x);
    match press {
        P::AddLayer => layers.push(crate::access::fresh(shape, a.places)),
        P::RemoveLayer(l) => {
            if layers.len() > 1 && n(l) < layers.len() {
                layers.remove(n(l));
            }
        }
        P::AddRoute(l) => {
            if let Some(layer) = layers.get_mut(n(l)) {
                let places = a.places.max(1);
                layer.routes.push(Route {
                    places: (0..places).collect(),
                    any: shape.m.max(1).min(places),
                    ..Route::default()
                });
            }
        }
        P::RemoveRoute(l, r) => {
            if let Some(layer) = layers.get_mut(n(l))
                && layer.routes.len() > 1
                && n(r) < layer.routes.len()
            {
                layer.routes.remove(n(r));
            }
        }
        P::AddPaperRoute(l) => {
            if let Some(layer) = layers.get_mut(n(l)) {
                let mut places: Vec<usize> = layer
                    .routes
                    .iter()
                    .flat_map(|r| r.places.iter().copied())
                    .collect();
                places.sort_unstable();
                places.dedup();
                if places.is_empty() {
                    places = (0..a.places.max(1)).collect();
                }
                let fresh = crate::access::fresh(shape, a.places);
                layer.routes.push(Route {
                    any: shape.m.max(1).min(places.len()),
                    places,
                    ..fresh.routes[0].clone()
                });
            }
        }
        P::AddPlaceTo(l, r) => {
            let free = (0..a.places).find(|p| {
                layers
                    .get(n(l))
                    .and_then(|x| x.routes.get(n(r)))
                    .is_some_and(|x| !x.places.contains(p))
            });
            let p = match free {
                Some(p) => p,
                None => {
                    if a.places >= crate::qstep::MAX_PLACES {
                        return;
                    }
                    a.add_place(shape);
                    a.places - 1
                }
            };
            if let Some(route) = layers.get_mut(n(l)).and_then(|x| x.routes.get_mut(n(r))) {
                route.places.push(p);
                route.places.sort_unstable();
                route.any = route.any.max(2).min(route.places.len());
            }
        }
        P::AddVaultFor(l, r) => {
            if layers.get(n(l)).and_then(|x| x.routes.get(n(r))).is_none() {
                return;
            }
            let v = a.add_vault(shape);
            if let Some(route) = layers.get_mut(n(l)).and_then(|x| x.routes.get_mut(n(r))) {
                // A route whose places hold a vault with its password
                // written down has the new one written down too.
                if route.kinds.is_some_and(|k| k.vault == KindVault::Written) {
                    route.written.push(v);
                } else {
                    route.passwords.push(v);
                }
            }
        }
        P::KindsAny(l, r, k) => {
            let k = n(k);
            loop {
                let Some(route) = layers.get(n(l)).and_then(|x| x.routes.get(n(r))) else {
                    return;
                };
                if route.places.len() >= k {
                    break;
                }
                let free = (0..a.places).find(|p| !route.places.contains(p));
                let p = match free {
                    Some(p) => p,
                    None => {
                        if a.places >= crate::qstep::MAX_PLACES {
                            return;
                        }
                        a.add_place(shape);
                        a.places - 1
                    }
                };
                if let Some(route) = layers.get_mut(n(l)).and_then(|x| x.routes.get_mut(n(r))) {
                    route.places.push(p);
                    route.places.sort_unstable();
                }
            }
            if let Some(route) = layers.get_mut(n(l)).and_then(|x| x.routes.get_mut(n(r))) {
                route.any = k.max(1).min(route.places.len());
            }
        }
        P::KindsVault(l, r, want) => {
            let vaults = a.vault_count(shape);
            let Some(route) = layers.get(n(l)).and_then(|x| x.routes.get(n(r))) else {
                return;
            };
            if route.paper || route.kinds.is_none() {
                return;
            }
            // The route's vault, had the way said: one it has, moved
            // across; else the plan's first; else a new one.
            let has = |r: &Route| match want {
                KindVault::Remembered => !r.passwords.is_empty(),
                KindVault::Written => !r.written.is_empty(),
                KindVault::None => true,
            };
            let new = if has(route) {
                None
            } else {
                let other = match want {
                    KindVault::Remembered => route.written.first(),
                    _ => route.passwords.first(),
                };
                Some(match (other, vaults) {
                    (Some(&v), _) => v,
                    (None, 0) => a.add_vault(shape),
                    (None, _) => 0,
                })
            };
            let Some(route) = layers.get_mut(n(l)).and_then(|x| x.routes.get_mut(n(r))) else {
                return;
            };
            if let Some(v) = new {
                route.passwords.retain(|&x| x != v);
                route.written.retain(|&x| x != v);
                match want {
                    KindVault::Remembered => route.passwords.push(v),
                    _ => route.written.push(v),
                }
                route.passwords.sort_unstable();
                route.written.sort_unstable();
            }
            if let Some(k) = route.kinds.as_mut() {
                k.vault = want;
            }
        }
        P::Fewer(l, r)
        | P::More(l, r)
        | P::Place(l, r, _)
        | P::Paper(l, r)
        | P::Password(l, r, _)
        | P::Passphrase(l, r, _)
        | P::KindsSet(l, r)
        | P::KindsFewerSeeds(l, r)
        | P::KindsMoreSeeds(l, r)
        | P::KindsSeeds(l, r, _)
        | P::KindsForm(l, r, _)
        | P::KindsDescription(l, r, _) => {
            let Some(route) = layers.get_mut(n(l)).and_then(|x| x.routes.get_mut(n(r))) else {
                return;
            };
            match press {
                P::Fewer(..) => route.any = route.any.saturating_sub(1).max(1),
                P::More(..) => route.any = (route.any + 1).min(route.places.len()),
                P::Place(_, _, p) => {
                    let p = n(p);
                    if p >= a.places {
                        return;
                    }
                    if let Some(k) = route.places.iter().position(|&q| q == p) {
                        route.places.remove(k);
                    } else {
                        route.places.push(p);
                        route.places.sort_unstable();
                    }
                    route.any = route.any.min(route.places.len()).max(1);
                }
                P::Paper(..) => {
                    route.paper = !route.paper;
                    // A paper route's kinds are on paper, with no vault.
                    if route.paper {
                        route.kinds = route.kinds.map(Kinds::on_paper);
                    }
                }
                P::KindsSet(..) => {
                    route.kinds = match route.kinds {
                        Some(_) => None,
                        None => Some(Kinds::start(route)),
                    };
                }
                P::KindsFewerSeeds(..) | P::KindsMoreSeeds(..) | P::KindsSeeds(..) => {
                    let most = shape.here().len();
                    if let Some(k) = route.kinds.as_mut() {
                        k.seeds = match press {
                            P::KindsFewerSeeds(..) => k.seeds.saturating_sub(1),
                            P::KindsMoreSeeds(..) => (k.seeds + 1).min(most.max(k.seeds)),
                            P::KindsSeeds(_, _, s) => n(s),
                            _ => k.seeds,
                        };
                    }
                }
                P::KindsForm(_, _, f) => {
                    if let Some(k) = route.kinds.as_mut()
                        && !route.paper
                    {
                        k.form = f;
                    }
                }
                P::KindsDescription(_, _, d) => {
                    if let Some(k) = route.kinds.as_mut() {
                        k.description = d;
                    }
                }
                P::Password(_, _, v) => {
                    let v = n(v);
                    if route.passwords.contains(&v) {
                        route.passwords.retain(|&x| x != v);
                        route.written.push(v);
                        route.written.sort_unstable();
                    } else if route.written.contains(&v) {
                        route.written.retain(|&x| x != v);
                    } else {
                        route.passwords.push(v);
                        route.passwords.sort_unstable();
                    }
                }
                P::Passphrase(_, _, i) => {
                    let i = n(i);
                    if shape.seeds.get(i).is_none_or(|s| !s.passphrase) {
                        return;
                    }
                    if route.passphrases.contains(&i) {
                        route.passphrases.retain(|&x| x != i);
                    } else {
                        route.passphrases.push(i);
                        route.passphrases.sort_unstable();
                    }
                }
                _ => {}
            }
        }
        P::WorkItOut => {}
    }
}
