//! The images the finder found (`docs/IMAGES.md` §5): the release's
//! images in the Downloads folder and among the files dropped on the
//! window, as the shell answers. The shell hashes each file whose name is
//! on the release's image list, off the window's thread; the Starter
//! keeps what it answers, and Make a Faraday stick and Make a Pi card offer
//! it (`docs/STARTER.md` §3.2, §3.3).

use crate::api::{FoundImage, ImageFinding, PayloadKind, Source, StarterCommand, StarterEvent};
use crate::decide::Target;
use crate::macs::StickFor;
use crate::{Image, Starter};

/// What the finder has found, and the files dropped to be looked at.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Found {
    /// The files dropped on the window, as full paths.
    dropped: Vec<String>,
    /// The shell's last answer; nothing until it answers.
    findings: Vec<ImageFinding>,
    /// The release's image file names, as the shell gave them: what
    /// hardware there is an image for.
    wanted: Vec<String>,
    /// The found image a person chose, by its path; none while the
    /// first found stands.
    chosen: Option<String>,
}

/// One image a person can choose on Make a stick or Make a Pi card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageChoice {
    /// The file's name.
    pub head: String,
    /// The build it carries.
    pub build: String,
    /// Where it was found: the folder.
    pub folder: String,
    /// Whether this is what would be written.
    pub selected: bool,
}

/// The name a release's image of this target starts with.
fn prefix(target: Target) -> &'static str {
    match target {
        Target::Stick => "faraday-stick-",
        Target::Pi => "faraday-pi-",
    }
}

/// The folder of a full path, as a person reads it.
fn folder_of(path: &str) -> String {
    path.rsplit_once(['/', '\\'])
        .map_or(String::new(), |(f, _)| f.to_owned())
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

    /// The found images to offer for `target`: those named for it, one
    /// of each image (the same `.img` dropped from two places is one);
    /// for a stick, once For which computer? is answered, only the kind
    /// answered. A lookup.
    fn offered_found(&self, target: Target) -> Vec<&FoundImage> {
        let kind = match (target, self.making.as_ref().and_then(|m| m.stick_for)) {
            (Target::Stick, Some(StickFor::Pc)) => Some(PayloadKind::Stick),
            (Target::Stick, Some(StickFor::AppleSilicon)) => Some(PayloadKind::AppleStick),
            _ => None,
        };
        let mut out: Vec<&FoundImage> = Vec::new();
        for f in self.found_images() {
            if f.name.starts_with(prefix(target))
                && kind.is_none_or(|k| PayloadKind::of(&f.name) == Some(k))
                && !out.iter().any(|o| o.img_sha256 == f.img_sha256)
            {
                out.push(f);
            }
        }
        out
    }

    /// Whether an image of `kind` was found, whoever's it is. A lookup.
    pub(crate) fn found_kind_offered(&self, kind: PayloadKind) -> bool {
        self.found_images()
            .any(|f| PayloadKind::of(&f.name) == Some(kind))
    }

    /// What a stick or card is written from when a found image answers
    /// for `target`: the one chosen, else the first offered. A lookup.
    pub(crate) fn found_image(&self, target: Target) -> Option<Image> {
        let offered = self.offered_found(target);
        let f = offered
            .iter()
            .find(|f| self.found.chosen.as_deref() == Some(f.path.as_str()))
            .or(offered.first())?;
        Some(Image {
            source: Source::Found {
                path: f.path.clone(),
                xz_sha256: f.xz_sha256.clone(),
                img_sha256: f.img_sha256.clone(),
            },
            build: Some(f.build_id.clone()),
            sha256: f.img_sha256.clone(),
            size: f.inner_size,
            dev: false,
            boot_only: false,
        })
    }

    /// The images to choose from for `target`, as Make's list shows
    /// them. A lookup.
    pub fn image_choices(&self, target: Target) -> Vec<ImageChoice> {
        let now = self.image(target).map(|i| i.source);
        self.offered_found(target)
            .into_iter()
            .map(|f| ImageChoice {
                head: f.name.clone(),
                build: f.build_id.clone(),
                folder: folder_of(&f.path),
                selected: matches!(&now, Some(Source::Found { path, .. }) if *path == f.path),
            })
            .collect()
    }

    /// The release files for `target` that the finder has not found: the
    /// names to download. A lookup.
    pub fn images_to_download(&self, target: Target) -> Vec<&str> {
        self.found
            .wanted
            .iter()
            .map(String::as_str)
            .filter(|n| n.starts_with(prefix(target)))
            .filter(|n| !self.found_images().any(|f| f.name == *n))
            .collect()
    }

    /// Findings for `target` that were named and not offered: the file's
    /// name and, for another release's, the version this Starter needs.
    /// A lookup.
    pub fn images_refused(&self, target: Target) -> Vec<(&str, Option<&str>)> {
        self.found
            .findings
            .iter()
            .filter_map(|f| match f {
                ImageFinding::Mismatch { name, .. } => Some((name.as_str(), None)),
                ImageFinding::OtherRelease { name, needs, .. } => {
                    Some((name.as_str(), Some(needs.as_str())))
                }
                ImageFinding::Found(_) => None,
            })
            .filter(|(n, _)| n.starts_with(prefix(target)))
            .collect()
    }

    /// Choice `n` of [`Starter::image_choices`] tapped.
    pub(crate) fn image_chosen(&mut self, n: u16) {
        let Some(target) = self.making.as_ref().map(|m| m.target) else {
            return;
        };
        let choices = self.image_choices(target);
        let Some(c) = choices.get(usize::from(n)) else {
            return;
        };
        self.found.chosen = self
            .offered_found(target)
            .into_iter()
            .find(|f| f.name == c.head && c.build == f.build_id)
            .map(|f| f.path.clone());
        self.dirty = true;
    }

    /// The release's image file names the shell gave.
    pub(crate) fn images_wanted(&mut self, names: Vec<String>) {
        self.found.wanted = names;
        self.dirty = true;
    }

    /// What the shell answered about images.
    pub(crate) fn images_event(&mut self, event: StarterEvent) {
        if let StarterEvent::ImagesFound(findings) = event {
            self.found.findings = findings;
            self.dirty = true;
        }
    }
}
