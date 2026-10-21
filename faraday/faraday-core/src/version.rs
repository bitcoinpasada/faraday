//! The build and the program that printed a `--version` line.

use crate::strings::{EN, fill};

/// The version shown on Settings.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The build ID the build scripts set (`docs/DECISIONS.md` F5): the
/// kernel release string's part after `-faraday-`. `None` for a plain
/// `cargo build`, which sets no such environment variable.
pub const BUILD: Option<&str> = option_env!("FARADAY_BUILD");

/// Which Faraday program printed a `--version` line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

impl Program {
    /// The words the program's `--version` line started with under
    /// earlier names (F16). A file that answers with them is still ours,
    /// so an install made with the old names is found and replaced.
    pub const fn old_names(self) -> [&'static str; 2] {
        match self {
            Program::StarterApp => [
                EN.name_faraday_starter_app_old,
                EN.name_faraday_starter_app_old2,
            ],
            Program::DesktopApp => [EN.name_faraday_desktop_old, EN.name_faraday_desktop_old],
        }
    }
}

/// The line `--version` prints: the program's name, a space, the build
/// string.
pub fn version_line(program: Program, build: &str) -> String {
    format!("{} {build}", program.name())
}

/// Which program, and which build string, a `--version` output is: its
/// first line must be exactly [`version_line`] for one of them. Anything
/// else is `None`; this is how the Starter tells its own and the
/// desktop app's files from another program's of the same name.
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

/// Whether this is a release build: a build ID with nothing after the
/// version (`upgrade::describe`). Test releases, dev builds and a plain
/// `cargo build` are not.
pub fn is_release() -> bool {
    BUILD.is_some_and(|b| !b.contains('+'))
}

/// What About and Settings show for the running build: `BUILD` described
/// (`docs/DECISIONS.md` F5) when set, else the crate's version, marked as
/// a local build.
pub fn version_label() -> String {
    match BUILD {
        Some(build) => crate::upgrade::describe(build),
        None => fill(EN.version_label_local, &[VERSION]),
    }
}
