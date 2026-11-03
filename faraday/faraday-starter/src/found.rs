//! The images the finder found (`docs/IMAGES.md` §5): the release's
//! images in the Downloads folder and among the files dropped on the
//! window, as the shell answers. The shell hashes each file whose name is
//! on the release's image list, off the window's thread; the Starter
//! keeps what it answers, and no screen shows it yet.

use crate::Starter;
use crate::api::{FoundImage, ImageFinding, StarterCommand, StarterEvent};

/// What the finder has found, and the files dropped to be looked at.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Found {
    /// The files dropped on the window, as full paths.
    dropped: Vec<String>,
    /// The shell's last answer; nothing until it answers.
    findings: Vec<ImageFinding>,
}

impl Starter {
    /// Asks the shell to look again in Downloads and at the files
    /// dropped so far; answered by [`StarterEvent::ImagesFound`].
    pub fn find_images(&mut self) {
        self.out.push_back(StarterCommand::FindImages {
            dropped: self.found.dropped.clone(),
        });
    }

    /// A file dropped on the window, to be looked at with the rest.
    pub(crate) fn image_dropped(&mut self, path: &str) {
        if !self.found.dropped.iter().any(|p| p == path) {
            self.found.dropped.push(path.to_owned());
        }
        self.find_images();
    }

    /// Everything the finder answered, in the order it gave. A lookup.
    pub fn image_findings(&self) -> &[ImageFinding] {
        &self.found.findings
    }

    /// The images found whose SHA-256 is their line's: the ones to offer.
    /// A lookup.
    pub fn found_images(&self) -> impl Iterator<Item = &FoundImage> {
        self.found.findings.iter().filter_map(|f| match f {
            ImageFinding::Found(image) => Some(image),
            _ => None,
        })
    }

    /// What the shell answered about images.
    pub(crate) fn images_event(&mut self, event: StarterEvent) {
        if let StarterEvent::ImagesFound(findings) = event {
            self.found.findings = findings;
            self.dirty = true;
        }
    }
}
