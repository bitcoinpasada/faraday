//! The driver and its command line (06 §8): a product's model over the fixture adapters, a
//! tour's steps played, a picture written at each, and the checks of 07 §5 run on each frame.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use faraday_atoms::Pressed as _;
use faraday_device_product::Device;
use faraday_draw::canvas::InkKind;
use faraday_fixture_adapters::{Fixtures, Scripted};
use faraday_port::{Adapters, Waker};
use faraday_runtime::{Mark, Root, Runtime, Workers};

use crate::targets::{self, SIZES, Short};
use crate::{png, screens, tours};

/// Which product's model a run builds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Product {
    Device,
    Demo,
    Starter,
}

/// What the command line asks of one run.
pub struct Command {
    pub product: Product,
    pub width: u16,
    pub height: u16,
    pub dpi: u16,
    /// The tour's name; the default tour when `None`.
    pub tour: Option<String>,
    pub out: String,
    /// The test kit's folder: each folder in it is a stick.
    pub kit: String,
    /// Run every tour at the four sizes of the ratchet, no pictures, and hold the counts to the
    /// file (07 §5.2) instead of writing pictures.
    pub ratchet: Option<String>,
    /// Print the tours' names and stop.
    pub list: bool,
}

/// A box in pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Box4 {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// A press target as drawn: the control, the target grown to the floor, the message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HitLook {
    pub rect: Box4,
    pub target: Box4,
    /// The message's kind: the variant's name, as `Message::kind()` gives it.
    pub message: String,
}

/// A kept image, as far as the check reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeptLook {
    pub secret: bool,
}

/// What the runtime's output says of one frame (TS15): pixels, drawn text, hit targets, the page
/// and sheets, the event's marks and the kept images. Nothing else of the model is read.
#[derive(Clone, Debug, Default)]
pub struct Look {
    pub width: u16,
    pub height: u16,
    pub rgba: Vec<u8>,
    /// Each drawn text and whether it was drawn from a secret.
    pub texts: Vec<(String, bool)>,
    /// The targets of the top layer drawn: the only one that takes presses.
    pub hits: Vec<HitLook>,
    pub page: (String, u32),
    pub sheets: Vec<(String, u32)>,
    pub marks: Vec<Mark>,
    pub kept: Vec<KeptLook>,
}

/// What a tour plays over: a product's model and its fixtures, one frame at a time.
pub trait Play {
    fn input(&mut self, script: &[Scripted]);
    fn advance(&mut self, ms: u64);
    /// Runs the queue and the adapters to rest and reads the frame.
    fn settle(&mut self) -> Result<Look, String>;
}

/// One tour's run: the model over the fixtures, and the pictures and counts taken.
pub struct Tour {
    play: Box<dyn Play>,
    pub size: (u16, u16),
    pub dpi: u16,
    out: Option<PathBuf>,
    count: bool,
    n: usize,
    last: Option<Look>,
    /// The step names taken, in order.
    pub steps: Vec<String>,
    /// Every target found under the floor, when counting.
    pub shorts: Vec<Short>,
}

impl Tour {
    #[must_use]
    pub fn new(
        play: Box<dyn Play>,
        size: (u16, u16),
        dpi: u16,
        out: Option<PathBuf>,
        count: bool,
    ) -> Self {
        Tour {
            play,
            size,
            dpi,
            out,
            count,
            n: 0,
            last: None,
            steps: Vec::new(),
            shorts: Vec::new(),
        }
    }

    /// Queues a script of input.
    pub fn input(&mut self, script: &[Scripted]) {
        self.play.input(script);
    }

    /// Queues a touch on the control whose press has this kind, in the frame last taken: the
    /// first one drawn (07 §5.1 names a press by its message's kind, never by a position).
    pub fn press(&mut self, kind: &str) -> Result<(), String> {
        let hit = self
            .last
            .as_ref()
            .and_then(|look| look.hits.iter().find(|h| h.message == kind))
            .ok_or_else(|| format!("no control with the press {kind} in the frame"))?;
        let at = faraday_port::values::Point {
            x: hit.rect.x + hit.rect.w / 2,
            y: hit.rect.y + hit.rect.h / 2,
        };
        self.play.input(&[Scripted::Touch(at)]);
        Ok(())
    }

    /// Moves the scripted clock on.
    pub fn advance(&mut self, ms: u64) {
        self.play.advance(ms);
    }

    /// The frame last taken.
    #[must_use]
    pub fn look(&self) -> Option<&Look> {
        self.last.as_ref()
    }

    /// A named step: runs to rest, checks the frame (07 §5.3), counts its targets when asked
    /// (07 §5.2) and writes its picture when there is a folder, `NN-name.png`.
    pub fn step(&mut self, name: &str) -> Result<(), String> {
        let look = self.play.settle()?;
        screens::check(self.last.as_ref(), &look, name)?;
        if let Some(out) = &self.out {
            let path = out.join(format!("{:02}-{name}.png", self.n));
            png::write(
                &path,
                u32::from(look.width),
                u32::from(look.height),
                &look.rgba,
            )
            .map_err(|e| format!("{}: {e}", path.display()))?;
        }
        self.last = Some(look);
        if self.count {
            targets::walk(self)?;
        }
        self.n += 1;
        self.steps.push(String::from(name));
        Ok(())
    }
}

struct NoWake;

impl Waker for NoWake {
    fn wake(&self) {}
}

/// A product's runtime over the fixtures.
struct Played<R: Root> {
    runtime: Runtime<R>,
    fixtures: Fixtures,
}

impl<R: Root> Played<R> {
    fn new(root: R, mut fixtures: Fixtures, size: (u16, u16), dpi: u16) -> Self {
        fixtures.display_of(size.0, size.1, dpi);
        let mut runtime = Runtime::new(root, Arc::new(NoWake), Workers::Inline);
        runtime.record_ink(true);
        Played { runtime, fixtures }
    }

    fn pump(&mut self) -> Result<(), String> {
        for _ in 0..10_000 {
            let mut moved = false;
            while let Some(event) = self.fixtures.poll() {
                moved = true;
                for effect in self.runtime.event(event) {
                    self.fixtures.carry(effect);
                }
            }
            for effect in self.runtime.woken() {
                moved = true;
                self.fixtures.carry(effect);
            }
            if !moved {
                return Ok(());
            }
        }
        Err(String::from(
            "the model never came to rest: events kept arriving",
        ))
    }
}

impl<R: Root> Play for Played<R> {
    fn input(&mut self, script: &[Scripted]) {
        self.fixtures.input(script);
    }

    fn advance(&mut self, ms: u64) {
        self.fixtures.advance(ms);
    }

    fn settle(&mut self) -> Result<Look, String> {
        self.pump()?;
        let (width, height, rgba) = match self.runtime.frame() {
            Some(frame) => (frame.width, frame.height, frame.rgba.to_vec()),
            None => return Err(String::from("the runtime drew no frame at this step")),
        };
        let top = self.runtime.hits().iter().map(|h| h.layer).max();
        let hits = self
            .runtime
            .hits()
            .iter()
            .filter(|h| Some(h.layer) == top)
            .map(|h| HitLook {
                rect: Box4 {
                    x: h.rect.x,
                    y: h.rect.y,
                    w: h.rect.w,
                    h: h.rect.h,
                },
                target: Box4 {
                    x: h.target.x,
                    y: h.target.y,
                    w: h.target.w,
                    h: h.target.h,
                },
                message: String::from(h.press.kind()),
            })
            .collect();
        let seen = self.runtime.seen();
        let texts = seen.ink.map_or_else(Vec::new, |ink| {
            ink.0
                .iter()
                .filter_map(|i| match &i.kind {
                    InkKind::Text { text, .. } => Some((String::from(text.as_str()), i.secret)),
                    _ => None,
                })
                .collect()
        });
        Ok(Look {
            width,
            height,
            rgba,
            texts,
            hits,
            page: (String::from(seen.page.scope), seen.page.n),
            sheets: seen
                .sheets
                .iter()
                .map(|a| (String::from(a.scope), a.n))
                .collect(),
            marks: seen.marks.to_vec(),
            kept: seen
                .kept
                .iter()
                .map(|k| KeptLook { secret: k.secret })
                .collect(),
        })
    }
}

/// The model of `product` over the fixtures from `kit`. Where a product cannot be built yet, the
/// error says what is missing.
fn build(
    product: Product,
    kit: &Path,
    size: (u16, u16),
    dpi: u16,
) -> Result<Box<dyn Play>, String> {
    match product {
        Product::Device => Ok(Box::new(Played::new(
            Device,
            Fixtures::device(kit),
            size,
            dpi,
        ))),
        Product::Demo => Err(String::from(
            "Faraday Demo's model cannot be built over the fixtures yet: its product value has no public constructor",
        )),
        Product::Starter => Err(String::from(
            "the Starter's model cannot be built over the fixtures yet: its product value has no public constructor",
        )),
    }
}

/// Parses the command line.
pub fn parse_from(args: &[String]) -> Result<Command, String> {
    let mut command = Command {
        product: Product::Device,
        width: 1024,
        height: 600,
        dpi: 170,
        tour: None,
        out: String::from("out/snapshots"),
        kit: String::from("out/testkit"),
        ratchet: None,
        list: false,
    };
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |what: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{arg} takes {what}"))
        };
        match arg.as_str() {
            "--product" => {
                command.product = match value("device, demo or starter")?.as_str() {
                    "device" => Product::Device,
                    "demo" => Product::Demo,
                    "starter" => Product::Starter,
                    other => return Err(format!("no product {other}")),
                };
            }
            "--size" => {
                let v = value("WIDTHxHEIGHT")?;
                let (w, h) = v.split_once('x').ok_or("--size is WIDTHxHEIGHT")?;
                command.width = w.parse().map_err(|_| "--size is WIDTHxHEIGHT")?;
                command.height = h.parse().map_err(|_| "--size is WIDTHxHEIGHT")?;
            }
            "--dpi" => {
                command.dpi = value("a number")?
                    .parse()
                    .map_err(|_| "--dpi is a number")?
            }
            "--tour" => command.tour = Some(value("a tour's name")?),
            "--out" => command.out = value("a folder")?,
            "--kit" => command.kit = value("a folder")?,
            "--list" => command.list = true,
            "--ratchet" => command.ratchet = Some(value("the counts file")?),
            other => return Err(format!("no option {other}")),
        }
    }
    Ok(command)
}

/// The command line of this process.
pub fn parse() -> Result<Command, String> {
    parse_from(&std::env::args().skip(1).collect::<Vec<_>>())
}

/// Plays one tour; a tour that stops early, or a model not built yet, is an error.
fn play(
    command: &Command,
    name: Option<&str>,
    size: (u16, u16),
    dpi: u16,
    out: Option<PathBuf>,
    count: bool,
) -> Result<Tour, String> {
    let label = name.unwrap_or("default");
    let played = catch_unwind(AssertUnwindSafe(|| {
        let mut tour = Tour::new(
            build(command.product, Path::new(&command.kit), size, dpi)?,
            size,
            dpi,
            out,
            count,
        );
        match tours::run_named(&mut tour, label) {
            Some(result) => result?,
            None => return Err(String::from("no such tour")),
        }
        Ok(tour)
    }));
    match played {
        Ok(result) => result.map_err(|e: String| format!("tour {label}: {e}")),
        Err(panic) => {
            let what = panic
                .downcast_ref::<&str>()
                .map(|s| String::from(*s))
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_default();
            Err(format!(
                "tour {label}: the model is not built yet at this level of the kit ({what})"
            ))
        }
    }
}

fn ratchet(command: &Command, file: &str) -> Result<(), String> {
    let held = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
    let mut found = std::collections::BTreeMap::new();
    let mut names: Vec<Option<&str>> = vec![None];
    names.extend(tours::TOURS.iter().map(|(n, _)| Some(*n)));
    for (label, size, dpi) in SIZES {
        let mut shorts = Vec::new();
        for name in &names {
            shorts.extend(play(command, *name, size, dpi, None, true)?.shorts);
        }
        found.extend(targets::counted(label, &shorts));
    }
    match targets::ratchet(&found, &held) {
        Ok(lower) => {
            for line in lower {
                println!("fallen, lower the file: {line}");
            }
            Ok(())
        }
        Err(failed) => Err(failed.join("\n")),
    }
}

pub fn run(command: &Command) -> ExitCode {
    if command.list {
        println!("{}", tours::names());
        return ExitCode::SUCCESS;
    }
    let result = if let Some(file) = &command.ratchet {
        ratchet(command, file)
    } else {
        let out = PathBuf::from(&command.out).join(command.tour.as_deref().unwrap_or("default"));
        std::fs::create_dir_all(&out)
            .map_err(|e| format!("{}: {e}", out.display()))
            .and_then(|()| {
                play(
                    command,
                    command.tour.as_deref(),
                    (command.width, command.height),
                    command.dpi,
                    Some(out),
                    false,
                )
                .map(|tour| println!("{} pictures", tour.steps.len()))
            })
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("faraday-snapshot: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A model that shows a fixed page and nothing else.
    struct Still {
        page: u32,
    }

    impl Play for Still {
        fn input(&mut self, _script: &[Scripted]) {}
        fn advance(&mut self, _ms: u64) {}
        fn settle(&mut self) -> Result<Look, String> {
            self.page += 1;
            Ok(Look {
                width: 2,
                height: 1,
                rgba: vec![1, 2, 3, 255, 4, 5, 6, 255],
                page: (String::from("home"), 1),
                ..Look::default()
            })
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("faraday-snapshot-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    /// Invariant: A tour writes one picture for each named step, numbered in order, and none when it has no folder.
    /// Regression: A step takes its frame and drops it, or two steps write the same file, so a reviewer misses a screen.
    /// Owner: root
    #[test]
    fn a_tour_writes_one_picture_a_step() {
        let dir = scratch("steps");
        let mut tour = Tour::new(
            Box::new(Still { page: 0 }),
            (2, 1),
            160,
            Some(dir.clone()),
            false,
        );
        tour.input(&[]);
        tour.advance(0);
        assert_eq!(tour.step("open"), Ok(()));
        assert_eq!(tour.step("again"), Ok(()));
        assert!(dir.join("00-open.png").is_file());
        assert!(dir.join("01-again.png").is_file());
        let mut bare = Tour::new(Box::new(Still { page: 0 }), (2, 1), 160, None, true);
        assert_eq!(bare.step("open"), Ok(()));
        assert_eq!(bare.steps, ["open"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Invariant: A command line names a product, a size, a tour and a folder, and an option it does not know is refused.
    /// Regression: A misspelt option is ignored and the run goes on with a default the person did not ask for.
    /// Owner: root
    #[test]
    fn the_command_line_is_read_and_a_wrong_option_refused() {
        let args = |v: &[&str]| v.iter().map(|s| String::from(*s)).collect::<Vec<_>>();
        let command = parse_from(&args(&[
            "--size", "480x640", "--dpi", "286", "--tour", "power",
        ]));
        assert!(
            matches!(&command, Ok(c) if c.width == 480 && c.height == 640 && c.dpi == 286
            && c.tour.as_deref() == Some("power"))
        );
        assert!(parse_from(&args(&["--sizee", "1x1"])).is_err());
    }

    /// Invariant: Every tour written so far runs to its end at a small size with no pictures, the screens check run on each of its frames.
    /// Regression: A tour stops early on a press its frame no longer offers, or a frame fails the screens check, and nothing says so until a person renders it.
    /// Owner: root
    #[test]
    fn the_tours_written_run_to_their_end() {
        let command = parse_from(&[]).expect("the default command line");
        // Home and the Power sheet (06 §8). Each tour joins this list when it is written; the
        // rest of the table is still its `todo!()` bodies.
        for name in ["default", "power"] {
            let played = play(&command, Some(name), (480, 640), 286, None, false);
            assert!(played.is_ok(), "{name}: {:?}", played.err());
        }
    }
}
