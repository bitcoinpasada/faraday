//! A layer's instructions, by hand (`docs/BACKUP-FLOW.md` §20.5, L4): for
//! each route of an access layer, the places to go to in order and what
//! to take from each, the secrets it needs by name, the vaults to open and
//! what to restore with. The map of which layer reaches what is not
//! printed and not written to any file: the template is shown on a sheet
//! to be copied out by hand, worked out once a frame through the drawing
//! memo (`docs/SPEED.md` §3.3) from the plan as it is, and gone when the
//! sheet closes. No secret's value is read here: seeds by fingerprint,
//! passphrases by whose they are.

use std::rc::Rc;

use opensigner_core::strings::fill;

use crate::access::{Layer, Route};
use crate::plan::{self, At, Shape, Spot, What, on_stick};
use crate::strings::EN;
use crate::{Faraday, Sheet};

/// One route's instructions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteSteps {
    /// "Route 2 · paper", where the layer has several routes.
    pub head: Option<String>,
    /// How many of its places are enough: "Any 2 of 3".
    pub any: String,
    /// The places to go to, in order: each by name, with what to take
    /// from it.
    pub places: Vec<(String, Vec<String>)>,
    /// The secrets it needs, by name only: "Vault 1's passphrase".
    pub know: Vec<String>,
    /// The vaults to open: "Vault 1".
    pub open: Vec<String>,
    /// What to restore with.
    pub restore: String,
    /// The watch-only software the plan names, if it names one.
    pub watch: Option<String>,
}

/// A layer's instructions: its name, then each route's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    /// The layer's name: typed with a vault open, else "Access layer n".
    pub title: String,
    /// Its routes, in order.
    pub routes: Vec<RouteSteps>,
}

/// How a thing kept at a place is taken: by kind and name, never its
/// value. Nothing for what is kept only in a vault. A lookup.
fn take(shape: &Shape, w: What, medium: crate::Medium) -> Option<String> {
    let seed = |i: usize| shape.seeds.get(i).map_or("?", |s| s.name.as_str());
    let n = |j: usize| (j + 1).to_string();
    Some(match w {
        What::Words(i) => fill(EN.instr_words, &[seed(i)]),
        What::SeedQr(i) => fill(EN.instr_seedqr, &[seed(i)]),
        What::Passphrase(i) => fill(EN.instr_passphrase, &[seed(i)]),
        What::Sheet => EN.instr_sheet.to_string(),
        What::Share(j) => fill(EN.instr_share, &[&n(j)]),
        What::VaultStick(v) => fill(EN.instr_vault_stick, &[&n(v), medium.noun()]),
        What::VaultPassword(v) => fill(EN.instr_vault_passphrase, &[&n(v)]),
        What::SeedFile(i) => fill(EN.instr_seed_file, &[seed(i)]),
        What::SeedQrFile(i) => fill(EN.instr_seedqr_file, &[seed(i)]),
        What::PassphraseFile(i) => fill(EN.instr_passphrase_file, &[seed(i)]),
        What::WalletFile(_) => EN.instr_wallet_file.to_string(),
        What::ShareFile(j, _) => fill(EN.instr_share_file, &[&n(j)]),
        What::SoftwareFile => EN.instr_software_file.to_string(),
        What::Seed(_) | What::SeedPassphrase(_) | What::PassphraseSealed(_) | What::Wallet => {
            return None;
        }
    })
}

/// What the plan's map `m` keeps at place `p` that `route` can use, each
/// once, in the map's order: on a paper route its paper alone; off paper
/// its paper, then each stick's files ("…, a file · Stick 2") and the
/// sticks of the vaults in `opens`. A walk of `m`.
fn takes_at(
    shape: &Shape,
    m: &[Spot],
    route: &Route,
    p: usize,
    opens: &[usize],
    medium: crate::Medium,
) -> Vec<String> {
    let stick_word = match medium {
        crate::Medium::Stick => EN.board_stick,
        crate::Medium::SdCard => EN.board_sd_card,
    };
    let mut out: Vec<String> = Vec::new();
    let mut push = |line: String| {
        if !out.contains(&line) {
            out.push(line);
        }
    };
    for s in m {
        let stick = match s.at {
            At::Place(q) if q == p => None,
            At::Stick(q, k) if q == p && !route.paper => Some(k),
            _ => continue,
        };
        for &(w, _) in &s.holds {
            if route.paper && on_stick(w) {
                continue;
            }
            if let What::VaultStick(v) = w
                && !opens.contains(&v)
            {
                continue;
            }
            let Some(line) = take(shape, w, medium) else {
                continue;
            };
            match stick {
                Some(k) if !matches!(w, What::VaultStick(_)) => push(fill(
                    "{} · {}",
                    &[&line, &fill(stick_word, &[&(k + 1).to_string()])],
                )),
                _ => push(line),
            }
        }
    }
    out
}

/// The vaults `route` opens on the map `m`: off paper, those whose stick
/// one of its places keeps and whose passphrase it remembers or finds
/// written at one of its places; in order. A walk of `m`.
fn route_opens(m: &[Spot], route: &Route) -> Vec<usize> {
    if route.paper {
        return Vec::new();
    }
    let at_route = |at: At| match at {
        At::Place(p) | At::Stick(p, _) => route.places.contains(&p),
        _ => false,
    };
    let held = |pick: &dyn Fn(What) -> Option<usize>| -> Vec<usize> {
        m.iter()
            .filter(|s| at_route(s.at))
            .flat_map(|s| s.holds.iter().filter_map(|(w, _)| pick(*w)))
            .collect()
    };
    let sticks = held(&|w| match w {
        What::VaultStick(v) => Some(v),
        _ => None,
    });
    let written = held(&|w| match w {
        What::VaultPassword(v) => Some(v),
        _ => None,
    });
    let mut out: Vec<usize> = sticks
        .into_iter()
        .filter(|v| route.passwords.contains(v) || written.contains(v))
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// The instructions for `layer` of a wallet of `shape` on the plan's map
/// `m`, its places named by `place`, the layer called `title`, the
/// watch-only software the plan names `watch`. A walk of `m` per place of
/// each route.
pub fn template(
    shape: &Shape,
    m: &[Spot],
    layer: &Layer,
    title: String,
    place: &dyn Fn(usize) -> String,
    watch: Option<String>,
    medium: crate::Medium,
) -> Template {
    let several = layer.routes.len() > 1;
    let routes = layer
        .routes
        .iter()
        .enumerate()
        .map(|(r, route)| {
            let mut places = route.places.clone();
            places.sort_unstable();
            places.dedup();
            let opens = route_opens(m, route);
            let mut know: Vec<String> = opens
                .iter()
                .filter(|v| route.passwords.contains(v))
                .map(|v| fill(EN.instr_vault_passphrase, &[&(v + 1).to_string()]))
                .collect();
            know.extend(
                route
                    .passphrases
                    .iter()
                    .filter_map(|&i| shape.seeds.get(i).filter(|s| s.passphrase))
                    .map(|s| fill(EN.instr_passphrase, &[&s.name])),
            );
            RouteSteps {
                head: several.then(|| {
                    let head = if route.paper {
                        EN.layers_route_paper
                    } else {
                        EN.layers_route
                    };
                    fill(head, &[&(r + 1).to_string()])
                }),
                any: fill(
                    EN.instr_any,
                    &[&route.any.to_string(), &places.len().to_string()],
                ),
                places: places
                    .iter()
                    .map(|&p| (place(p), takes_at(shape, m, route, p, &opens, medium)))
                    .collect(),
                know,
                open: opens
                    .iter()
                    .map(|v| fill(EN.instr_vault, &[&(v + 1).to_string()]))
                    .collect(),
                restore: EN.instr_faraday_restore.to_string(),
                watch: watch.clone(),
            }
        })
        .collect();
    Template { title, routes }
}

impl Faraday {
    /// Whether a layer's instructions are offered: on the board's plan,
    /// Back up's or a wallet flow's, not on Create's Access layers card,
    /// which has no wallet yet. A lookup.
    pub(crate) fn instructions_offered(&self) -> bool {
        !self.create_planning() && self.board_of().is_some()
    }

    /// The instructions on the sheet, worked out once a frame through the
    /// drawing memo (`docs/SPEED.md` §3.3) from the plan the board edits:
    /// nothing kept between frames. Drawing reads it.
    pub(crate) fn instructions_view(&self) -> Option<Rc<Template>> {
        if self.drawing.is_on() {
            return self
                .drawing
                .instructions
                .get_or_init(|| self.instructions_uncached().map(Rc::new))
                .clone();
        }
        self.instructions_uncached().map(Rc::new)
    }

    /// [`Faraday::instructions_view`] worked out: a walk of the plan's map
    /// per place of each of the layer's routes.
    fn instructions_uncached(&self) -> Option<Template> {
        let Some(Sheet::Instructions(l)) = self.sheet else {
            return None;
        };
        if !self.instructions_offered() {
            return None;
        }
        let l = usize::from(l);
        let (shape, a) = self.board_plan()?;
        let layers = a.layers_shown(&shape);
        let layer = layers.get(l)?;
        let m = plan::map(&shape, &a);
        let watch = m
            .iter()
            .any(|s| s.at == At::Software && !s.holds.is_empty())
            .then(|| a.software.iter().position(|&on| on))
            .flatten()
            .filter(|&k| k != plan::software::NOT_SURE)
            .map(|_| crate::board::software_name(&a));
        let place = |p: usize| self.board_place_name(p);
        Some(template(
            &shape,
            &m,
            layer,
            self.board_layer_name(l),
            &place,
            watch,
            self.medium,
        ))
    }

    /// A layer's instructions opened (§20.5): the sheet, layer `l`'s;
    /// nothing worked out or kept but which layer. A lookup.
    pub(crate) fn instructions_open(&mut self, l: u8) {
        let layers = self
            .board_plan()
            .map_or(0, |(shape, a)| a.layers_shown(&shape).len());
        if !self.instructions_offered() || usize::from(l) >= layers {
            return;
        }
        self.board_let_go();
        if let Some(b) = self.backup.as_mut() {
            b.naming = None;
            b.naming_layer = None;
        }
        self.sheet = Some(Sheet::Instructions(l));
        self.sheet_scroll = (0.0, None);
    }
}
