//! Upgrading a Faraday stick (`PLAN.md` §5.5): the boot partition of the
//! stick Faraday started from, written raw over another Faraday stick's,
//! whose data partition is not touched.
//!
//! Settings → **Upgrade a Faraday stick**. If the session is not clean,
//! the lock sheet comes first and the fresh process opens here. Then:
//!
//! 1. The stick Faraday started from is read. The boot copier
//!    (`faraday-boot`) takes as the source whichever boot partition holds
//!    the running kernel's release string, so it does not matter when the
//!    stick went in, or whether it was pulled and put back.
//! 2. The stick to upgrade: its version and the one to be written, a
//!    newer one warned about, a boot partition too small refused.
//!    **Upgrade** has the copier write, read back and compare.
//! 3. Done; the stick is removed.
//!
//! The app does no I/O and hands the copier no bytes: it asks the shell
//! for the copier's listing ([`StorageEvent::Boots`]), for the source to
//! be read ([`StorageCommand::BootRead`]) and for a partition to be
//! written ([`StorageCommand::BootWrite`]). While the flow is on screen
//! the shell publishes the upgrade marker beside the clean marker
//! ([`Faraday::upgrading`]), and only then does `faraday-grant` hand the
//! copier a boot partition.

use osk_shell_api::Command;

use crate::{BootPart, Faraday, Screen, Sheet, StorageCommand, StorageEvent};

/// What the upgrade holds while it is on screen.
#[derive(Debug, Default)]
pub struct UpgradeState {
    /// The boot partitions the copier has now.
    pub parts: Vec<BootPart>,
    /// The source, once read: the partition, its release string, its
    /// size in bytes.
    pub source: Option<(String, String, u64)>,
    /// The source is being read.
    pub reading: bool,
    /// Why no source was taken, for the partitions in `tried`.
    pub refused: Option<String>,
    /// The partitions that were in when the source was refused: read
    /// again only once another goes in.
    tried: Vec<String>,
    /// The target chosen, when more than one is in.
    pub pick: Option<String>,
    /// The target being written.
    pub writing: Option<String>,
    /// The target written, read back and matched, and its release.
    pub done: Option<(String, String)>,
    /// The last write that failed: why, and whether the stick was pulled
    /// during it.
    pub failed: Option<(String, bool)>,
}

/// Where the flow stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Reading the stick Faraday started from.
    Source,
    /// The stick to upgrade.
    Target,
    /// Written.
    Done,
}

/// What stops a target being written, or warns about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fit {
    /// It can be written.
    Fits,
    /// It carries a newer Faraday than the one running: it can be written.
    Newer,
    /// The source is a dev build and this stick does not hold one: it
    /// gains a serial console and login. It can be written.
    Dev,
    /// It already carries this Faraday.
    Same,
    /// Its boot partition is smaller than the source: MB it has, MB
    /// needed.
    TooSmall(u64, u64),
}

/// The Faraday version in a release string: `0.2.0 (4d0680b1a2b3)` from
/// `6.6.84-faraday-0.2.0+4d0680b1a2b3`; a stick made before Faraday put
/// its version in the kernel shows as earlier.
pub fn version(release: Option<&str>) -> String {
    match release.and_then(|r| r.split_once("-faraday-")) {
        Some((_, build_id)) => describe(build_id),
        None => "0.1.0 or earlier".to_string(),
    }
}

/// A build ID described (`docs/DECISIONS.md` F5): the part of the kernel
/// release string after `-faraday-`, or [`crate::BUILD`] directly.
///
/// | Input | Output |
/// |---|---|
/// | `0.2.0` | `0.2.0` |
/// | `0.1.0+ef24784b48d8.test` | `0.1.0 test release (ef24784b48d8)` |
/// | `0.1.0+ef24784b48d8.dirty-3fa9c1d2.test` | `0.1.0 test release (ef24784b48d8, changes 3fa9c1d2)` |
/// | `0.1.0+ef24784b48d8.dev` | `0.1.0 dev (ef24784b48d8)` |
/// | `0.1.0+ef24784b48d8.dirty-3fa9c1d2.dev` | `0.1.0 dev (ef24784b48d8, changes 3fa9c1d2)` |
/// | `0.1.0+4d0680b1a2b3` | `0.1.0 (4d0680b1a2b3)` |
/// | `0.1.0+4d0680b1a2b3.dirty` | `0.1.0 (4d0680b1a2b3.dirty)` |
///
/// The last two are the old format, kept. Anything after `+` that this
/// does not recognise follows the old rule, `number (rest)`.
pub fn describe(id: &str) -> String {
    let Some((number, rest)) = id.split_once('+') else {
        // A release: the version alone.
        return id.to_string();
    };
    let mut parts = rest.splitn(3, '.');
    let commit = parts.next().unwrap_or(rest);
    let after_commit: Vec<&str> = parts.collect();
    let (changes, kind) = match after_commit.as_slice() {
        [kind] if *kind == "test" || *kind == "dev" => (None, Some(*kind)),
        [dirty, kind] if dirty.starts_with("dirty-") && (*kind == "test" || *kind == "dev") => {
            (dirty.strip_prefix("dirty-"), Some(*kind))
        }
        _ => (None, None),
    };
    match (kind, changes) {
        (Some("test"), None) => format!("{number} test release ({commit})"),
        (Some("test"), Some(fp)) => format!("{number} test release ({commit}, changes {fp})"),
        (Some("dev"), None) => format!("{number} dev ({commit})"),
        (Some("dev"), Some(fp)) => format!("{number} dev ({commit}, changes {fp})"),
        _ => format!("{number} ({rest})"),
    }
}

/// Whether a release string names a dev build: the part after
/// `-faraday-` ends with `.dev`. A target with no release string is not
/// dev.
fn is_dev(release: Option<&str>) -> bool {
    release
        .and_then(|r| r.split_once("-faraday-"))
        .is_some_and(|(_, v)| v.ends_with(".dev"))
}

/// The version number in a release string, to compare: `None` for a
/// stick with none, which is earlier than any.
fn number(release: Option<&str>) -> Option<(u64, u64, u64)> {
    let v = release?.split_once("-faraday-")?.1;
    let v = v.split(['+', '-']).next()?;
    let mut n = v.split('.').map(|p| p.parse::<u64>().ok());
    Some((n.next()??, n.next()??, n.next()??))
}

fn mb(bytes: u64) -> u64 {
    bytes.div_ceil(1 << 20)
}

impl UpgradeState {
    /// Where the flow stands.
    pub fn step(&self) -> Step {
        if self.done.is_some() {
            Step::Done
        } else if self.source.is_none() {
            Step::Source
        } else {
            Step::Target
        }
    }

    /// The sticks that could be upgraded: every boot partition but the
    /// source's.
    pub fn targets(&self) -> Vec<&BootPart> {
        let source = self.source.as_ref().map(|s| s.0.as_str());
        self.parts
            .iter()
            .filter(|p| !p.source && Some(p.id.as_str()) != source)
            .collect()
    }

    /// The target on show: the one chosen, else the first.
    pub fn target(&self) -> Option<&BootPart> {
        let targets = self.targets();
        self.pick
            .as_ref()
            .and_then(|id| targets.iter().find(|p| &p.id == id).copied())
            .or_else(|| targets.first().copied())
    }

    /// Whether `part` can take the source, and what to say if not.
    pub fn fit(&self, part: &BootPart) -> Fit {
        let Some((_, release, size)) = self.source.as_ref() else {
            return Fit::Fits;
        };
        if part.size < *size {
            return Fit::TooSmall(mb(part.size), mb(*size));
        }
        if part.release.as_deref() == Some(release.as_str()) {
            return Fit::Same;
        }
        if is_dev(Some(release)) && !is_dev(part.release.as_deref()) {
            return Fit::Dev;
        }
        match (number(part.release.as_deref()), number(Some(release))) {
            (Some(t), Some(s)) if t > s => Fit::Newer,
            _ => Fit::Fits,
        }
    }
}

impl Faraday {
    /// Whether the upgrade is on screen: the shell publishes the upgrade
    /// marker beside the clean marker while it is, and polls the boot
    /// copier for [`StorageEvent::Boots`].
    pub fn upgrading(&self) -> bool {
        self.screen == Screen::Upgrade && self.upgrade.is_some()
    }

    /// Whether a request that takes a while (reading or writing a boot
    /// partition) is queued for the shell: it draws what is on screen
    /// before serving it.
    pub fn long_storage_queued(&self) -> bool {
        self.storage_out.iter().any(|c| {
            matches!(
                c,
                StorageCommand::BootRead | StorageCommand::BootWrite { .. }
            )
        })
    }

    /// Settings → Upgrade: straight in when clean, else the lock sheet
    /// first, and the fresh process opens here.
    pub(crate) fn upgrade_open(&mut self) {
        if self.online {
            return;
        }
        if !self.clean() {
            self.upgrade_after_lock = true;
            self.not_now = false;
            self.sheet = Some(Sheet::Lock);
            return;
        }
        self.upgrade_start();
    }

    /// The flow from its first step.
    pub(crate) fn upgrade_start(&mut self) {
        self.upgrade_after_lock = false;
        self.screen = Screen::Upgrade;
        self.list_offset = 0.0;
        self.upgrade = Some(UpgradeState::default());
    }

    /// The flow left: the copier drops the source, and the shell stops
    /// publishing the marker.
    pub(crate) fn upgrade_leave(&mut self) {
        if self.upgrade.take().is_some() {
            self.storage_out.push_back(StorageCommand::BootForget);
        }
    }

    /// Asks for the source when a boot partition is in that has not been
    /// tried.
    fn upgrade_read_if_new(&mut self) {
        let Some(u) = self.upgrade.as_mut() else {
            return;
        };
        if u.source.is_some() || u.reading || u.parts.is_empty() {
            return;
        }
        if u.parts.iter().all(|p| u.tried.contains(&p.id)) {
            return;
        }
        u.reading = true;
        u.refused = None;
        self.storage_out.push_back(StorageCommand::BootRead);
    }

    /// What the shell said about the copier; `None` when it was taken.
    pub(crate) fn upgrade_event(&mut self, event: StorageEvent) -> Option<StorageEvent> {
        let Some(u) = self.upgrade.as_mut() else {
            return match event {
                StorageEvent::Boots(_)
                | StorageEvent::BootSource { .. }
                | StorageEvent::BootSourceFailed { .. }
                | StorageEvent::BootWritten { .. }
                | StorageEvent::BootWriteFailed { .. } => None,
                other => Some(other),
            };
        };
        match event {
            StorageEvent::Boots(parts) => {
                if u.parts == parts {
                    return None;
                }
                u.parts = parts;
                let ids: Vec<&String> = u.parts.iter().map(|p| &p.id).collect();
                u.tried.retain(|t| ids.contains(&t));
                if u.tried.is_empty() {
                    u.refused = None;
                }
                self.upgrade_read_if_new();
            }
            StorageEvent::BootSource { id, release, size } => {
                u.reading = false;
                u.refused = None;
                u.source = Some((id, release, size));
            }
            StorageEvent::BootSourceFailed { reason } => {
                u.reading = false;
                u.refused = Some(reason);
                u.tried = u.parts.iter().map(|p| p.id.clone()).collect();
            }
            StorageEvent::BootWritten { id, release } => {
                u.writing = None;
                u.failed = None;
                u.done = Some((id, release));
            }
            StorageEvent::BootWriteFailed { reason, pulled, .. } => {
                u.writing = None;
                u.failed = Some((reason, pulled));
            }
            other => return Some(other),
        }
        self.dirty = true;
        self.commands.push_back(Command::Draw);
        None
    }

    /// Upgrade: the target on show is written, if it can be.
    pub(crate) fn upgrade_write(&mut self) {
        let Some(u) = self.upgrade.as_mut() else {
            return;
        };
        if u.writing.is_some() || u.step() != Step::Target {
            return;
        }
        let Some(target) = u.target() else {
            return;
        };
        if matches!(u.fit(target), Fit::TooSmall(..) | Fit::Same) {
            return;
        }
        let id = target.id.clone();
        u.failed = None;
        u.writing = Some(id.clone());
        self.storage_out
            .push_back(StorageCommand::BootWrite { target: id });
    }

    /// The target on show, when more than one is in.
    pub(crate) fn upgrade_pick(&mut self, i: u8) {
        if let Some(u) = self.upgrade.as_mut()
            && u.writing.is_none()
            && let Some(id) = u.targets().get(usize::from(i)).map(|p| p.id.clone())
        {
            u.pick = Some(id);
            u.failed = None;
        }
    }

    /// Another stick, from the same source.
    pub(crate) fn upgrade_again(&mut self) {
        if let Some(u) = self.upgrade.as_mut() {
            u.done = None;
            u.failed = None;
            u.pick = None;
        }
    }

    /// The data partition beside a boot partition: same disk, as the
    /// disk process names it (`sdb2@sdb#7` beside `sdb1@sdb#7`).
    pub fn upgrade_data_of(&self, boot: &BootPart) -> Option<&crate::StickInfo> {
        let disk = boot.id.split_once('@')?.1;
        self.sticks
            .iter()
            .find(|s| s.id.split_once('@').is_some_and(|(_, d)| d == disk))
    }
}
