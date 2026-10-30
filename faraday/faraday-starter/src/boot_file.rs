//! What to write's Boot file (`docs/STARTER.md` §4.5, F133): a stick for a
//! PC is written with the release's own `BOOTX64.EFI` unless the person
//! chooses one they signed with their own Secure Boot db key. "One I
//! signed" lists the `.efi` files in Downloads (a file dropped on the
//! window counts as one tapped); the one tapped is checked by the shell
//! against the release, and Continue waits until one is taken. Confirm
//! shows who signed it, and the write carries it; the writer checks it
//! again before it writes.

use crate::api::{BootFile, BootSigner, Reason, StarterCommand, StarterEvent};
use crate::decide::Target;
use crate::macs::StickFor;
use crate::{Page, Starter};

/// The boot file a stick is written with, as What to write chose it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BootPick {
    /// One the person signed, not the release's own.
    pub signed: bool,
    /// The folder the shell looked in and the `.efi` files it found
    /// there, as full paths; nothing until it answers.
    pub found: Option<(String, Vec<String>)>,
    /// The file being checked against the release.
    pub checking: Option<String>,
    /// The file taken, as the order names it, and who signed it.
    pub chosen: Option<(BootFile, BootSigner)>,
    /// The last file refused, and why.
    pub refused: Option<(String, Reason)>,
}

impl Starter {
    /// Whether the stick being made can be written with a boot file the
    /// person signed: a stick for a PC or an Intel Mac, whose boot file
    /// is `BOOTX64.EFI`. A lookup.
    pub fn boot_file_offered(&self) -> bool {
        self.making.as_ref().is_some_and(|m| {
            m.target == Target::Stick && m.stick_for != Some(StickFor::AppleSilicon)
        }) && self.image(Target::Stick).is_some()
    }

    /// What to write's boot file: the release's own, or one the person
    /// signed and the shell took. A lookup.
    pub fn boot_pick(&self) -> Option<&BootPick> {
        self.making.as_ref().map(|m| &m.boot)
    }

    /// Whether What to write's boot file lets Continue go on: the
    /// release's own, or a signed one taken.
    pub(crate) fn boot_ready(&self) -> bool {
        self.boot_pick()
            .is_none_or(|b| !b.signed || !self.boot_file_offered() || b.chosen.is_some())
    }

    /// Whether a confirmed write may go: a writer is there, the mode
    /// allows it, the stick's version is read where keep data needs it,
    /// and the boot file is ready.
    pub(crate) fn may_write(&self, mode: crate::api::Mode) -> bool {
        !self.no_writer
            && self.mode_refusal(mode).is_none()
            && !self.keep_unread()
            && self.boot_ready()
            && self.name_ready()
    }

    /// The boot file the write carries: the signed one taken, when it was
    /// chosen; `None` for the release's own.
    pub(crate) fn boot_order(&self) -> Option<BootFile> {
        let b = self
            .boot_pick()
            .filter(|b| b.signed && self.boot_file_offered())?;
        b.chosen.as_ref().map(|(f, _)| f.clone())
    }

    /// The release's own (`false`) or one the person signed (`true`),
    /// ticked on What to write; the signed one has Downloads listed.
    pub(crate) fn boot_choice(&mut self, signed: bool) {
        if self.page != Page::Mode || !self.boot_file_offered() {
            return;
        }
        if let Some(m) = self.making.as_mut() {
            m.boot.signed = signed;
        }
        if signed {
            self.out.push_back(StarterCommand::ListBootFiles);
        }
    }

    /// A file of the listing tapped, by its place in it.
    pub(crate) fn boot_file_tapped(&mut self, n: u16) {
        let path = self
            .boot_pick()
            .and_then(|b| b.found.as_ref())
            .and_then(|(_, files)| files.get(usize::from(n)))
            .cloned();
        if let Some(path) = path {
            self.check_boot_file(path);
        }
    }

    /// A file dropped on the window: on What to write with "One I
    /// signed" ticked, it is checked as one tapped.
    pub fn file_dropped(&mut self, path: String) {
        self.check_boot_file(path);
        self.dirty = true;
    }

    /// Asks the shell to check `path` against the release, once at a time.
    fn check_boot_file(&mut self, path: String) {
        let ready = self.page == Page::Mode
            && self.boot_file_offered()
            && self
                .boot_pick()
                .is_some_and(|b| b.signed && b.checking.is_none());
        let Some(source) = self.image(Target::Stick).map(|i| i.source) else {
            return;
        };
        if !ready {
            return;
        }
        if let Some(m) = self.making.as_mut() {
            m.boot.checking = Some(path.clone());
            m.boot.chosen = None;
            m.boot.refused = None;
        }
        self.out
            .push_back(StarterCommand::CheckBootFile { path, source });
    }

    /// What the shell answered about boot files.
    pub(crate) fn boot_event(&mut self, event: StarterEvent) {
        let Some(m) = self.making.as_mut() else {
            return;
        };
        match event {
            StarterEvent::BootFiles { folder, files } => m.boot.found = Some((folder, files)),
            StarterEvent::BootFileChecked { path, result } => {
                if m.boot.checking.as_ref() != Some(&path) {
                    return;
                }
                m.boot.checking = None;
                match result {
                    Ok(taken) => m.boot.chosen = Some(taken),
                    Err(why) => m.boot.refused = Some((path, why)),
                }
            }
            _ => {}
        }
    }
}
