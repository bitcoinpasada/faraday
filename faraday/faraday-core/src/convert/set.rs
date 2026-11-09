//! Convert a seed's source **A SLIP-39 set** (`docs/SEED-CONVERSION.md`
//! §8, F204): shares typed one at a time and held by `faraday-keys`, and
//! the three actions on them. The area holds no secret of its own beyond
//! the share being typed, wiped as the aezeed entry's is.

use super::{ConvertAction, ConvertArea, Value, cstep, slip_need};
use crate::Faraday;
use crate::strings::{EN, fill};
use faraday_bip::slip39::GroupSpec;
use faraday_keys::slip39_set::{RotateGroup, SetFacts, SetRefused, rotate_count};
use faraday_keys::{Held, Reveal};
use zeroize::Zeroizing;

/// What can be done with the set held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetAct {
    /// One more share of a group.
    Add,
    /// The share that was lost, again.
    Replace,
    /// A new set of the same secret.
    Rotate,
}

/// The flow's state for the set source. Nothing here is a secret.
#[derive(Debug, Clone, Default)]
pub struct SetState {
    /// The action pressed.
    pub act: Option<SetAct>,
    /// The group a share is made for, 0-based.
    pub group: u8,
    /// The member number, 0-based (shown 1 to 16).
    pub member: u8,
    /// Per group, shares lost or exposed.
    pub lost: Vec<u8>,
    /// The typed shares pressed destroyed, as (group, member).
    pub destroyed: Vec<(u8, u8)>,
    /// Every other share held is destroyed.
    pub others: bool,
    /// Why the last share typed was not held.
    pub error: Option<String>,
    /// Why nothing was made, for Result.
    pub refused: Option<String>,
}

/// Whether `act` is offered for `f`; else how many more are needed (shares
/// of one group, or groups).
pub fn offered(f: &SetFacts, act: SetAct) -> Result<(), u8> {
    match act {
        SetAct::Add | SetAct::Replace => {
            if f.groups.iter().any(|g| g.quorum) {
                return Ok(());
            }
            let short = f
                .groups
                .iter()
                .map(|g| g.threshold.saturating_sub(g.members.len() as u8))
                .min();
            Err(short.unwrap_or(1))
        }
        SetAct::Rotate => {
            if f.recoverable {
                return Ok(());
            }
            let met = f.groups.iter().filter(|g| g.quorum).count() as u8;
            Err(f.group_threshold.saturating_sub(met))
        }
    }
}

/// The groups with a met quorum, 0-based.
pub(crate) fn quorum_groups(f: &SetFacts) -> Vec<u8> {
    f.groups
        .iter()
        .filter(|g| g.quorum)
        .map(|g| g.index)
        .collect()
}

/// The groups as the count of lost or exposed shares sees them.
pub(crate) fn rotate_groups(f: &SetFacts, lost: &[u8]) -> Vec<RotateGroup> {
    (0..f.group_count)
        .map(|g| RotateGroup {
            threshold: f.groups.iter().find(|x| x.index == g).map(|x| x.threshold),
            lost_or_exposed: lost.get(usize::from(g)).copied().unwrap_or(0),
        })
        .collect()
}

/// Whether every typed share and every other share is pressed destroyed.
pub(crate) fn all_destroyed(f: &SetFacts, s: &SetState) -> bool {
    s.others
        && f.groups.iter().all(|g| {
            g.members
                .iter()
                .all(|&m| s.destroyed.contains(&(g.index, m)))
        })
}

/// The first member number not typed in `group`, from `from` on and round.
fn free_member(f: &SetFacts, group: u8, from: u8, step: i8) -> u8 {
    let typed: &[u8] = f
        .groups
        .iter()
        .find(|g| g.index == group)
        .map_or(&[], |g| &g.members);
    let mut m = from;
    for _ in 0..16 {
        if !typed.contains(&m) {
            return m;
        }
        m = (m + 16).wrapping_add_signed(step) % 16;
    }
    from
}

/// What a refusal says.
pub(crate) fn refused_text(r: &SetRefused, medium: crate::Medium) -> String {
    match r {
        SetRefused::Stick(_) => crate::stick::keys_load(medium),
        SetRefused::Practice => EN.convert_set_practice.to_string(),
        SetRefused::Share(e) => faraday_strings::bip::slip39_error(e),
        SetRefused::NotGiven => EN.convert_not_given.to_string(),
        SetRefused::NoRandomness => EN.convert_no_randomness.to_string(),
        SetRefused::SameIdentifier | SetRefused::NotSameSet => EN.convert_not_same_set.to_string(),
    }
}

impl ConvertArea {
    /// The share being typed.
    pub(crate) fn set_typed(&self) -> &str {
        &self.set_typed
    }

    /// Whether the set's entry is the card open.
    pub(crate) fn entering_share(&self) -> bool {
        self.state
            .as_ref()
            .is_some_and(|s| s.set.is_some() && s.open == Some(cstep::FROM))
    }

    /// The entry opens again on a set already held: what is held and what
    /// the cards show stay as they are.
    fn reopen_set(&mut self, ready: bool) {
        self.drop_result(false);
        if let Some(s) = self.state.as_mut() {
            s.done[usize::from(cstep::FROM)] = ready;
            s.open = Some(cstep::FROM);
            s.scroll.follow = true;
        }
    }

    /// The set becomes the source; nothing is chosen or made. A share
    /// half typed stays typed.
    fn choose_set(&mut self) {
        self.drop_result(false);
        if let Some(s) = self.state.as_mut() {
            s.key = None;
            s.aezeed = false;
            s.set = Some(SetState::default());
            s.slip_t = 1;
            s.slip = vec![(2, 3)];
            s.done[usize::from(cstep::FROM)] = false;
            s.open = Some(cstep::FROM);
            s.scroll.follow = true;
        }
    }

    fn held_set(&mut self) -> Option<&mut SetState> {
        self.state.as_mut().and_then(|s| s.set.as_mut())
    }

    /// The Settings card of `act`, with the first group and free member.
    fn open_set_act(&mut self, act: SetAct, f: &SetFacts) {
        self.drop_result(false);
        if let Some(s) = self.state.as_mut() {
            let group = quorum_groups(f).first().copied().unwrap_or(0);
            if let Some(x) = s.set.as_mut() {
                *x = SetState {
                    act: Some(act),
                    group,
                    member: free_member(f, group, 0, 1),
                    lost: vec![0; usize::from(f.group_count)],
                    ..SetState::default()
                };
            }
            s.settings = true;
            s.done[usize::from(cstep::TO)] = true;
            s.open = Some(cstep::SETTINGS);
            s.scroll.follow = true;
        }
    }

    /// What was made: Result opens.
    fn set_made(&mut self, given: Result<Value, String>) {
        self.drop_result(true);
        let (held, refused) = match given {
            Ok(v) => (Some(v), None),
            Err(line) => (None, Some(line)),
        };
        self.held = held;
        if let Some(s) = self.state.as_mut() {
            s.checkable = refused.is_none();
            if let Some(x) = s.set.as_mut() {
                x.refused = refused;
            }
            s.done[usize::from(cstep::SETTINGS)] = true;
            s.done[usize::from(cstep::RESULT)] = true;
            s.open = Some(cstep::RESULT);
            s.scroll.follow = true;
        }
    }
}

/// A key typed on the set's entry. Returns whether it was taken. Enter
/// holds the share typed.
pub(crate) fn key(app: &mut Faraday, key: faraday_shell_api::Key) -> bool {
    use faraday_shell_api::Key as K;
    if !app.tools.convert().entering_share() {
        return false;
    }
    let area = app.tools.convert_mut();
    match key {
        K::Char(c) if c.is_ascii_alphabetic() => area.set_typed.push(c.to_ascii_lowercase()),
        K::Char(' ') => {
            if !area.set_typed.is_empty() && !area.set_typed.ends_with(' ') {
                area.set_typed.push(' ');
            }
        }
        K::Backspace => {
            area.set_typed.pop();
        }
        K::Tab => {
            if let Some(word) = crate::forms::slip39_completion(&area.set_typed) {
                let cut = area.set_typed.rfind(' ').map_or(0, |i| i + 1);
                area.set_typed.truncate(cut);
                area.set_typed.push_str(&word);
                area.set_typed.push(' ');
            }
        }
        K::Enter => {
            hold(app);
            return true;
        }
        _ => return false,
    }
    if let Some(x) = area.held_set() {
        x.error = None;
    }
    true
}

/// Holds the share typed, on the press: a refusal shows its reason and
/// adds nothing.
fn hold(app: &mut Faraday) {
    let text = Zeroizing::new(app.tools.convert().set_typed().trim().to_lowercase());
    if text.is_empty() {
        return;
    }
    let medium = app.medium;
    let held = app
        .session
        .keys
        .slip39_set_hold(&text)
        .map(|f| f.recoverable);
    let ready = app
        .session
        .keys
        .slip39_set_facts()
        .is_some_and(|f| offered(f, SetAct::Add).is_ok());
    let area = app.tools.convert_mut();
    let error = match held {
        Ok(_) => {
            area.set_typed.clear();
            None
        }
        Err(r) => Some(refused_text(&r, medium)),
    };
    area.drop_result(false);
    if let Some(s) = area.state.as_mut() {
        s.done[usize::from(cstep::FROM)] = ready;
        s.scroll.follow = true;
    }
    if let Some(x) = area.held_set() {
        x.error = error;
    }
}

/// The press on the From row: the entry opens. Refused while a stick is
/// attached, as the aezeed's is; dimmed in Practice.
fn source(app: &mut Faraday) {
    if !app.stick.is_empty() {
        return app.toast(&crate::stick::keys_load(app.medium));
    }
    if !app.makes_secrets() || !app.may_load_keys() {
        return;
    }
    let held = app.session.keys.slip39_set_facts().is_some()
        && app.tools.convert().state().is_some_and(|s| s.set.is_some());
    if held {
        let ready = app
            .session
            .keys
            .slip39_set_facts()
            .is_some_and(|f| offered(f, SetAct::Add).is_ok());
        return app.tools.convert_mut().reopen_set(ready);
    }
    super::forget_held(app);
    app.tools.convert_mut().choose_set();
}

/// A press on the set's flow.
pub(crate) fn act(app: &mut Faraday, action: ConvertAction) {
    use ConvertAction as C;
    if action == C::SetSource {
        return source(app);
    }
    if app.tools.convert().state().is_none_or(|s| s.set.is_none()) {
        return;
    }
    let facts = app.session.keys.slip39_set_facts().cloned();
    match action {
        C::SetHold => hold(app),
        C::SetAgain => {
            app.session.keys.slip39_set_forget();
            app.tools.convert_mut().set_typed.clear();
            app.tools.convert_mut().choose_set();
        }
        C::SetAct(a) => {
            if let Some(f) = facts.filter(|f| offered(f, a).is_ok()) {
                app.tools.convert_mut().open_set_act(a, &f);
            }
        }
        C::SetGroup(by) => {
            let Some(f) = facts else { return };
            let groups = quorum_groups(&f);
            let area = app.tools.convert_mut();
            if let Some(x) = area.held_set()
                && let Some(at) = groups.iter().position(|&g| g == x.group)
            {
                let n = groups.len();
                x.group = groups[(at + n).wrapping_add_signed(isize::from(by)) % n];
                x.member = free_member(&f, x.group, 0, 1);
            }
        }
        C::SetMember(by) => {
            let Some(f) = facts else { return };
            let area = app.tools.convert_mut();
            if let Some(x) = area.held_set() {
                let next = (x.member + 16).wrapping_add_signed(by) % 16;
                x.member = free_member(&f, x.group, next, by);
            }
        }
        C::Lost(g, by) => {
            let area = app.tools.convert_mut();
            if let Some(x) = area.held_set()
                && let Some(n) = x.lost.get_mut(usize::from(g))
            {
                *n = n.saturating_add_signed(by).min(16);
                x.destroyed.clear();
                x.others = false;
            }
        }
        C::Destroy(g, m) => {
            if let Some(x) = app.tools.convert_mut().held_set() {
                match x.destroyed.iter().position(|&d| d == (g, m)) {
                    Some(at) => {
                        x.destroyed.remove(at);
                    }
                    None => x.destroyed.push((g, m)),
                }
            }
        }
        C::DestroyOthers => {
            if let Some(x) = app.tools.convert_mut().held_set() {
                x.others = !x.others;
            }
        }
        C::Make => make(app),
        _ => {}
    }
}

/// Makes the share, or the new set, from the settings. Rotating draws
/// again where the old identifier came up, unseen.
pub(crate) fn make(app: &mut Faraday) {
    let Some(f) = app.session.keys.slip39_set_facts().cloned() else {
        return;
    };
    let Some((x, slip_t, slip)) = app.tools.convert().state().and_then(|s| {
        s.set
            .clone()
            .filter(|_| s.settings)
            .map(|x| (x, s.slip_t, s.slip.clone()))
    }) else {
        return;
    };
    let Some(act) = x.act else { return };
    let why = Reveal::PaperForm;
    let medium = app.medium;
    let given: Result<Value, SetRefused> = match act {
        SetAct::Add | SetAct::Replace => app
            .session
            .keys
            .slip39_set_share(x.group, x.member, why)
            .map(|h: Held<_>| Value::Text(h)),
        SetAct::Rotate => {
            let count = rotate_count(f.group_threshold, &rotate_groups(&f, &x.lost));
            if count.may_be_in_other_hands || !all_destroyed(&f, &x) {
                return;
            }
            let groups: Vec<GroupSpec> = slip
                .iter()
                .map(|&(threshold, count)| GroupSpec { threshold, count })
                .collect();
            let need = slip_need(f.secret_len, slip_t, &slip);
            let mut out = Err(SetRefused::SameIdentifier);
            for _ in 0..8 {
                let Some(random) = app.fresh_bytes(b"convert split", need) else {
                    return app.toast(EN.convert_no_randomness);
                };
                out = app
                    .session
                    .keys
                    .slip39_set_rotated(slip_t, &groups, &random, why)
                    .map(Value::Set);
                if !matches!(out, Err(SetRefused::SameIdentifier)) {
                    break;
                }
            }
            out
        }
    };
    if let Err(SetRefused::NoRandomness) = given {
        return app.toast(EN.convert_no_randomness);
    }
    app.tools
        .convert_mut()
        .set_made(given.map_err(|r| refused_text(&r, medium)));
}

/// The Standard fact of a share or set made.
pub fn standard(f: &SetFacts, s: &super::ConvertState, x: &SetState) -> String {
    match x.act {
        Some(SetAct::Rotate) => super::slip_set_standard(s),
        _ => fill(
            EN.convert_set_share_standard,
            &[
                &(x.group + 1).to_string(),
                &(x.member + 1).to_string(),
                &format!("{:04x}", f.identifier),
            ],
        ),
    }
}

/// What the file saved is named: the identifier of the set it belongs to
/// (`id`: the new set's, for a rotated one) and what it holds; a share
/// by its group and its member, as a person counts them.
pub fn file(id: u16, x: &SetState) -> String {
    if x.act == Some(SetAct::Rotate) {
        format!("{id:04x}-slip39-set.txt")
    } else {
        format!(
            "{id:04x}-slip39-group-{}-member-{}.txt",
            x.group + 1,
            x.member + 1
        )
    }
}

/// Offers what was made on the secret sheet.
pub(crate) fn save(app: &mut Faraday) {
    let area = app.tools.convert();
    let (Some(held), Some(s), Some(f)) = (
        area.held.as_ref(),
        area.state(),
        app.session.keys.slip39_set_facts(),
    ) else {
        return;
    };
    let Some(x) = s.set.as_ref() else { return };
    let mut bytes = Zeroizing::new(Vec::new());
    match held {
        Value::Set(parts) => {
            for p in parts.iter() {
                bytes.extend_from_slice(p.as_bytes());
                bytes.push(b'\n');
            }
        }
        Value::Text(t) => bytes.extend_from_slice(t.as_str().as_bytes()),
        Value::Code(..) => return,
    }
    let id = match held {
        Value::Set(parts) => parts
            .first()
            .and_then(|p| faraday_bip::slip39::Share::parse(p.as_str()).ok())
            .map_or(f.identifier, |sh| sh.identifier()),
        _ => f.identifier,
    };
    let name = file(id, x);
    let mut out = crate::secrets::SecretOut::new(
        name,
        bytes,
        EN.convert_secret_slip39,
        EN.convert_secret_gives,
    );
    out.keep = crate::secrets::Keep::Note;
    app.offer_secret(out);
}
