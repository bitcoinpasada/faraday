//! The build and the program that printed a `--version` line (`main`'s `version.rs`).

use faraday_strings::{EN, fill};

/// The version About shows when the build scripts set no build ID.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The build ID the build scripts set: the kernel release string's part after `-faraday-`.
/// `None` for a plain `cargo build`, which sets no such environment variable.
pub const BUILD: Option<&str> = option_env!("FARADAY_BUILD");

/// Which Faraday program printed a `--version` line.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Program {
    /// The Starter.
    StarterApp,
    /// The desktop app.
    DesktopApp,
}

impl Program {
    /// The fixed words a program's `--version` line starts with.
    pub const fn name(self) -> &'static str {
        match self {
            Program::StarterApp => EN.name_faraday_starter_app,
            Program::DesktopApp => EN.name_faraday_desktop,
        }
    }

    /// The words the line started with under earlier names: a file that answers with them is
    /// still ours, so an install made with the old names is found and replaced.
    const fn old_names(self) -> [&'static str; 2] {
        match self {
            Program::StarterApp => [
                EN.name_faraday_starter_app_old,
                EN.name_faraday_starter_app_old2,
            ],
            Program::DesktopApp => [EN.name_faraday_desktop_old, EN.name_faraday_desktop_old2],
        }
    }
}

/// The line `--version` prints: the program's name, a space, the build string.
pub fn version_line(program: Program, build: &str) -> String {
    format!("{} {build}", program.name())
}

/// Which program, and which build string, a `--version` output is: its first line must be
/// exactly [`version_line`] for one of them. Anything else is `None`; this is how the Starter
/// tells its own and the desktop app's files from another program's of the same name.
pub fn parse_version_output(output: &str) -> Option<(Program, &str)> {
    let line = output.lines().next()?.trim();
    [Program::StarterApp, Program::DesktopApp]
        .into_iter()
        .find_map(|p| {
            let rest = std::iter::once(p.name())
                .chain(p.old_names())
                .find_map(|n| line.strip_prefix(n).filter(|r| r.starts_with(' ')))?;
            let build = rest.strip_prefix(' ')?.trim();
            (!build.is_empty()).then_some((p, build))
        })
}

/// Whether this is a release build: a build ID with nothing after the version. Test releases,
/// dev builds and a plain `cargo build` are not.
pub fn is_release() -> bool {
    BUILD.is_some_and(|b| !b.contains('+'))
}

/// What About shows for the running build: the build ID described when it is set, else the
/// crate's version, marked as a local build.
pub fn version_label() -> String {
    label(BUILD)
}

fn label(build: Option<&str>) -> String {
    match build {
        Some(build) => faraday_paper::build::describe(build),
        None => fill(EN.version_label_local, &[VERSION]),
    }
}

#[cfg(test)]
mod tests;
