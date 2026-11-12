//! A SLIP-39 set typed into Convert a seed, and what is done with it: a
//! share made at a member index (add and replace are one operation) and a
//! new set of the same EMS (rotate), `docs/SEED-CONVERSION.md` §8.
//!
//! The typed shares are held here as [`HeldSet`], as an aezeed is: never a
//! key of the session, not in the list, the vault, Explore or Lock in. They
//! leave only as the shares these operations make, for `Reveal::PaperForm`.
//! Nothing is decrypted and no passphrase is taken: what is read back is
//! the EMS and its parameters, not a wallet, so the set's own passphrase
//! still opens the same wallet. No PBKDF2 runs here.
#![cfg_attr(not(test), warn(clippy::disallowed_methods))]

use faraday_bip::slip39::{self, Ems, GroupSpec, Share};
use subtle::ConstantTimeEq;

use crate::operations::{self, SecretLine};
use crate::{Held, Keys, Reveal};

/// Why a typed share was not held, or a share or a set was not made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetRefused {
    /// A stick is attached, named here, and no secret is typed in while one
    /// is.
    Stick(&'static str),
    /// Practice is on: a set made here would look like a real backup.
    Practice,
    /// SLIP-39 refused it, for this reason: a share that does not parse, a
    /// share of another set, a duplicate, a group of threshold 1, an index
    /// among the shares typed, a short quorum, a quorum whose digest fails,
    /// or a shape the split refuses.
    Share(slip39::Error),
    /// No set is held, or the reason is not `Reveal::PaperForm`.
    NotGiven,
    /// The randomness is too short for the shape
    /// ([`operations::slip39_random_len`] is enough).
    NoRandomness,
    /// The randomness drew the old set's identifier: new randomness draws
    /// another.
    SameIdentifier,
    /// What was made did not read back as the same EMS and parameters:
    /// nothing is returned.
    NotSameSet,
}

/// One group of the held set, as far as it has been typed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(
    clippy::disallowed_methods,
    reason = "a derived Clone: no secret type derives it"
)]
pub struct GroupFacts {
    /// The group, 0-based.
    pub index: u8,
    /// Shares of this group needed.
    pub threshold: u8,
    /// The member indices typed, 0-based, in the order typed.
    pub members: Vec<u8>,
    /// Whether the threshold's worth have been typed.
    pub quorum: bool,
}

/// What is held, for a screen: no secret in it. Worked out as each share
/// is typed; a screen reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(
    clippy::disallowed_methods,
    reason = "a derived Clone: no secret type derives it"
)]
pub struct SetFacts {
    /// The set's fifteen-bit identifier.
    pub identifier: u16,
    /// The extendable flag.
    pub extendable: bool,
    /// The iteration exponent.
    pub iteration_exponent: u8,
    /// The master secret's length in bytes, which a new set's randomness
    /// is sized by.
    pub secret_len: usize,
    /// Groups needed.
    pub group_threshold: u8,
    /// Groups in the set.
    pub group_count: u8,
    /// The groups typed, by index. A group none of whose shares has been
    /// typed is not here: only its own shares say its threshold.
    pub groups: Vec<GroupFacts>,
    /// Whether enough groups have met their quorum for the whole set to be
    /// recovered. By count: the digests are checked when a share or a set
    /// is made.
    pub recoverable: bool,
}

impl SetFacts {
    /// The facts of `shares`, which are not empty and passed
    /// [`slip39::check_gathered`].
    fn of(shares: &[Share]) -> Option<SetFacts> {
        let first = shares.first()?;
        let mut groups: Vec<GroupFacts> = Vec::new();
        for s in shares {
            let at = match groups.iter().position(|g| g.index == s.group_index()) {
                Some(at) => at,
                None => {
                    groups.push(GroupFacts {
                        index: s.group_index(),
                        threshold: s.member_threshold(),
                        members: Vec::new(),
                        quorum: false,
                    });
                    groups.len() - 1
                }
            };
            let group = &mut groups[at];
            group.members.push(s.member_index());
            group.quorum = group.members.len() == usize::from(group.threshold);
        }
        groups.sort_by_key(|g| g.index);
        let met = groups.iter().filter(|g| g.quorum).count();
        Some(SetFacts {
            identifier: first.identifier(),
            extendable: first.is_extendable(),
            iteration_exponent: first.iteration_exponent(),
            secret_len: first.secret_len(),
            group_threshold: first.group_threshold(),
            group_count: first.group_count(),
            recoverable: met >= usize::from(first.group_threshold()),
            groups,
        })
    }
}

/// The shares typed so far, each wiped as it drops, and their facts.
pub struct HeldSet {
    shares: Vec<Share>,
    facts: SetFacts,
}

impl HeldSet {
    /// A qualifying subset of what is typed: the groups that have met their
    /// quorum, `first` among them if it is one, then by index, as many as
    /// the set needs. `None` when too few have.
    fn whole_quorum(&self, first: Option<u8>) -> Option<Vec<Share>> {
        let mut met: Vec<u8> = self
            .facts
            .groups
            .iter()
            .filter(|g| g.quorum)
            .map(|g| g.index)
            .collect();
        met.sort_by_key(|&g| Some(g) != first);
        met.truncate(usize::from(self.facts.group_threshold));
        (met.len() == usize::from(self.facts.group_threshold)).then(|| {
            self.shares
                .iter()
                .filter(|s| met.contains(&s.group_index()))
                .map(Share::duplicate)
                .collect()
        })
    }
}

/// Whether two EMSs are one: the same identifier, flag, exponent and
/// ciphertext.
fn same_ems(a: &Ems, b: &Ems) -> bool {
    a.identifier() == b.identifier()
        && a.is_extendable() == b.is_extendable()
        && a.iteration_exponent() == b.iteration_exponent()
        && bool::from(
            a.ciphertext()
                .expose()
                .as_bytes()
                .ct_eq(b.ciphertext().expose().as_bytes()),
        )
}

impl Keys {
    /// Holds one more share of a SLIP-39 set, typed as `text`, and says
    /// what is now held. Refused while a stick is attached, as every way a
    /// key is added is, and in Practice; refused as SLIP-39 refuses it: a
    /// share that does not parse, of another set (identifier, flag,
    /// exponent, group threshold, group count, length), a member typed
    /// already or one more than its group's threshold. What was held before
    /// stays. The first share held drops an aezeed held as a source. It is
    /// not a key: [`Keys::len`] and the lists do not change.
    pub fn slip39_set_hold(&mut self, text: &str) -> Result<&SetFacts, SetRefused> {
        if let Some(noun) = self.stick_attached() {
            return Err(SetRefused::Stick(noun));
        }
        if self.demo() {
            return Err(SetRefused::Practice);
        }
        let share = Share::parse(text).map_err(SetRefused::Share)?;
        // Room for every member of every group, so the shares never move.
        let mut held = self.slip39_set.take().map_or_else(
            || Vec::with_capacity(slip39::MAX_COUNT * slip39::MAX_COUNT),
            |set| set.shares,
        );
        held.push(share);
        let refused = slip39::check_gathered(&held).err();
        if refused.is_some() {
            held.pop();
        }
        self.slip39_set = SetFacts::of(&held).map(|facts| HeldSet {
            shares: held,
            facts,
        });
        if let Some(e) = refused {
            return Err(SetRefused::Share(e));
        }
        self.aezeed = None;
        self.slip39_set
            .as_ref()
            .map(|set| &set.facts)
            .ok_or(SetRefused::NotGiven)
    }

    /// What is held, with no secret in it (a lookup).
    pub fn slip39_set_facts(&self) -> Option<&SetFacts> {
        self.slip39_set.as_ref().map(|set| &set.facts)
    }

    /// The held set is wiped and dropped.
    pub fn slip39_set_forget(&mut self) {
        self.slip39_set = None;
    }

    /// A share of `group` at `member_index` (0 to 15), from that group's
    /// quorum, given only for `Reveal::PaperForm`. Adding a share and
    /// replacing a lost one are this: a share does not say how many members
    /// its group has, so the person chooses the index, and the same index
    /// gives the same share again, word for word. `derive_member`'s
    /// refusals stand: a short quorum, a group of threshold 1, an index
    /// among the shares typed, a quorum whose digest fails.
    ///
    /// Read back: the new share parses, and swapped in for one of the
    /// quorum it gives the share it displaced, so it is on the group's own
    /// polynomial. Where the whole set is typed, the swapped set recovers
    /// the same EMS and parameters as well. One group's quorum alone cannot
    /// reach the EMS of a set of several groups.
    pub fn slip39_set_share(
        &self,
        group: u8,
        member_index: u8,
        why: Reveal,
    ) -> Result<Held<SecretLine>, SetRefused> {
        if why != Reveal::PaperForm {
            return Err(SetRefused::NotGiven);
        }
        if let Some(noun) = self.stick_attached() {
            return Err(SetRefused::Stick(noun));
        }
        let held = self.slip39_set.as_ref().ok_or(SetRefused::NotGiven)?;
        let quorum: Vec<Share> = held
            .shares
            .iter()
            .filter(|s| s.group_index() == group)
            .map(Share::duplicate)
            .collect();
        let made = slip39::derive_member(&quorum, member_index).map_err(SetRefused::Share)?;
        let line = operations::share_line(&made);
        let read = Share::parse(&line).map_err(|_| SetRefused::NotSameSet)?;

        let displaced = quorum.first().ok_or(SetRefused::NotSameSet)?;
        let mut swapped = quorum.iter().map(Share::duplicate).collect::<Vec<Share>>();
        swapped[0] = read.duplicate();
        let again = slip39::derive_member(&swapped, displaced.member_index())
            .map_err(|_| SetRefused::NotSameSet)?;
        if !bool::from(again.indices().ct_eq(displaced.indices())) {
            return Err(SetRefused::NotSameSet);
        }

        if let Some(mut whole) = held.whole_quorum(Some(group)) {
            let before = slip39::recover_ems(&whole).map_err(SetRefused::Share)?;
            let at = whole
                .iter()
                .position(|s| {
                    s.group_index() == group && s.member_index() == displaced.member_index()
                })
                .ok_or(SetRefused::NotSameSet)?;
            whole[at] = read;
            let after = slip39::recover_ems(&whole).map_err(|_| SetRefused::NotSameSet)?;
            if !same_ems(&before, &after) {
                return Err(SetRefused::NotSameSet);
            }
        }
        Ok(Held::of(line))
    }

    /// A new set of the held set's EMS, `group_threshold` of the `groups`,
    /// each a threshold of its members, given only for `Reveal::PaperForm`
    /// and from the whole set's quorum (a set short of it is
    /// [`slip39::Error::WrongGroupCount`]). `random` is as
    /// [`Keys::slip39_converted`] takes it: the first two bytes are the
    /// identifier, the rest the split's; fewer than
    /// [`operations::slip39_random_len`] gives is
    /// [`SetRefused::NoRandomness`]. With the extendable flag set the new
    /// set takes that identifier, and the old one drawn is
    /// [`SetRefused::SameIdentifier`], so old and new shares cannot be
    /// mixed by mistake; with it clear the two bytes go unused and the
    /// identifier stays, since it is in the encryption's salt. The flag and
    /// the exponent stay.
    ///
    /// Read back: every share parses, and every member of every group takes
    /// part in a qualifying subset that recovers the same EMS and
    /// parameters. Refused while a stick is attached, as holding is. The
    /// shares are in group, then member order.
    pub fn slip39_set_rotated(
        &self,
        group_threshold: u8,
        groups: &[GroupSpec],
        random: &[u8],
        why: Reveal,
    ) -> Result<Held<Vec<SecretLine>>, SetRefused> {
        if why != Reveal::PaperForm {
            return Err(SetRefused::NotGiven);
        }
        if let Some(noun) = self.stick_attached() {
            return Err(SetRefused::Stick(noun));
        }
        let held = self.slip39_set.as_ref().ok_or(SetRefused::NotGiven)?;
        let quorum = held.whole_quorum(None).ok_or_else(|| {
            SetRefused::Share(slip39::Error::WrongGroupCount {
                needed: held.facts.group_threshold,
                given: held.facts.groups.iter().filter(|g| g.quorum).count() as u8,
            })
        })?;
        let old = slip39::recover_ems(&quorum).map_err(SetRefused::Share)?;

        let need = operations::slip39_random_len(held.facts.secret_len, group_threshold, groups);
        let random = random.get(..need).ok_or(SetRefused::NoRandomness)?;
        let (id, mut rest) = random.split_at(2);
        let new = if old.is_extendable() {
            let identifier = u16::from_le_bytes([id[0], id[1]]) & 0x7fff;
            if identifier == old.identifier() {
                return Err(SetRefused::SameIdentifier);
            }
            old.with_identifier(identifier).map_err(SetRefused::Share)?
        } else {
            old
        };

        let mut fill = |out: &mut [u8]| {
            let (now, after) = rest.split_at(out.len());
            out.copy_from_slice(now);
            rest = after;
        };
        let total: usize = groups.iter().map(|g| usize::from(g.count)).sum();
        let mut shares = Vec::new();
        shares.resize_with(total, Share::default);
        let made = slip39::split_ems(&new, group_threshold, groups, &mut fill, &mut shares)
            .map_err(SetRefused::Share)?;
        if made != total {
            return Err(SetRefused::NotSameSet);
        }

        let lines: Vec<SecretLine> = shares.iter().map(operations::share_line).collect();
        let parsed: Option<Vec<Share>> = lines.iter().map(|l| Share::parse(l).ok()).collect();
        let parsed = parsed.ok_or(SetRefused::NotSameSet)?;
        // Every member of every group takes part in a recovery of the same
        // EMS and parameters: no PBKDF2 runs here, so all of them are read.
        let mut starts = Vec::with_capacity(groups.len());
        let mut at = 0;
        for spec in groups {
            starts.push(at);
            at += usize::from(spec.count);
        }
        let members_of = |gi: usize, with: Option<usize>| -> Vec<usize> {
            let (start, spec) = (starts[gi], groups[gi]);
            let others = (0..usize::from(spec.count)).filter(|&m| Some(m) != with);
            with.into_iter()
                .chain(others)
                .take(usize::from(spec.threshold))
                .map(|m| start + m)
                .collect()
        };
        for gi in 0..groups.len() {
            let rest: Vec<usize> = (0..groups.len())
                .filter(|&other| other != gi)
                .take(usize::from(group_threshold) - 1)
                .collect();
            for m in 0..usize::from(groups[gi].count) {
                let chosen: Vec<usize> = members_of(gi, Some(m))
                    .into_iter()
                    .chain(rest.iter().flat_map(|&other| members_of(other, None)))
                    .collect();
                let own: Vec<Share> = chosen.iter().map(|&n| parsed[n].duplicate()).collect();
                let read = slip39::recover_ems(&own).map_err(|_| SetRefused::NotSameSet)?;
                if !same_ems(&read, &new) {
                    return Err(SetRefused::NotSameSet);
                }
            }
        }
        Ok(Held::of(lines))
    }
}

/// One group, as the person counts it before a rotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RotateGroup {
    /// Shares of this group needed. `None` for a group none of whose shares
    /// has been typed, whose threshold nothing here can say: one share lost
    /// or exposed then counts as reaching it.
    pub threshold: Option<u8>,
    /// Its shares lost or exposed, which may exist out of the person's
    /// reach.
    pub lost_or_exposed: u8,
}

/// What [`rotate_count`] says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RotateCount {
    /// Groups whose lost and exposed shares reach their threshold: their
    /// secrets may be known.
    pub groups_out_of_reach: u8,
    /// Whether as many groups as the set needs are: the set may already be
    /// in other hands, rotating is not enough, and the coins move to a new
    /// key.
    pub may_be_in_other_hands: bool,
}

/// The count of Rotate's second step for a set needing `group_threshold`
/// groups (a lookup).
pub fn rotate_count(group_threshold: u8, groups: &[RotateGroup]) -> RotateCount {
    let out = groups
        .iter()
        .filter(|g| g.lost_or_exposed > 0 && g.lost_or_exposed >= g.threshold.unwrap_or(1))
        .count();
    RotateCount {
        groups_out_of_reach: out as u8,
        may_be_in_other_hands: out >= usize::from(group_threshold),
    }
}
