//! Headless snapshot shell (`docs/PLANNING.md` §13.1, §16.5).
//!
//! Drives an [`App`] with a scripted event sequence at a given size and DPI
//! and writes PNGs. A review tool for humans, not a test harness (§16.7).
//!
//! ```text
//! cargo run -p opensigner-snapshot -- --size 480x640 --dpi 286 --out out/ [--app opensigner|gallery] [--tier C] [--script events.txt] [--strip NAME] [--camera-mounted] [--inset-bottom PX] [--inset-top PX] [--audit]
//! ```
//!
//! Script lines (one command per line, `#` starts a comment):
//!
//! ```text
//! tap X Y            down + up at (X, Y)
//! down X Y / move X Y / up X Y
//! key K              K = a single character, or backspace, enter, escape, up, down, left, right, tab
//! text "abc"         one `key` per character
//! word "abandon"     opensigner only: `text` the letters, then tap candidate cell 0 of the
//!                    word entry — a word typed out in full is that cell, and a tap on a
//!                    candidate is the only thing that takes a word (PLANNING §16.118)
//! scroll X Y DY
//! tick MS            advance the clock by MS and send Tick
//! snap NAME          drain commands and write NAME.png
//! tapid ID           opensigner only: tap the centre of widget ID (see opensigner_core::ids)
//! holdid ID          opensigner only: press widget ID, tick past the hold duration, release
//! pressid ID         opensigner only: press widget ID and keep it pressed (a hold-to-reveal
//!                    button shows its secret until `releaseid`)
//! releaseid ID       opensigner only: release widget ID pressed by `pressid`
//! scrollid ID DY     opensigner only: wheel-scroll DY pixels inside scroll region ID
//! padkey ID K        opensigner only: tap key K of on-screen keyboard ID (a character, or
//!                    backspace / enter); this taps the drawn key, whatever order a
//!                    shuffled pad drew it in
//! pin ID DIGITS      opensigner only: `padkey` each digit of DIGITS on keyboard ID, then ✓
//! entropy off        leave the app's entropy requests unanswered from here on (a weak
//!                    session, as with a shell that has no RNG)
//! file PATH          queue PATH's bytes as the answer to the app's next file request
//! files DIR          answer the app's next file request with a listing of DIR, as a shell
//!                    that can list its files does; the app shows it and reads what is tapped
//! settings PATH      send PATH's bytes as the settings a shell kept from an earlier
//!                    session, as one Event::Settings
//! frame PATH         send PATH (a PNG, any colour type) as one camera frame, converted to luma,
//!                    the clock first moved on by the scanner's decode interval;
//!                    only while the app has the camera on
//! camera off         answer the app's next CameraOn with CameraUnavailable (no camera)
//! restart [tier=A|B|C|D] [secure=none|tee|strongbox] [boot=unknown|verified|unverified]
//!                    opensigner only: close the app and open a new one, as a person does,
//!                    on a device with the tier, secure hardware and boot given (each one
//!                    not given stays as it was); the settings are the first-run-done ones
//!                    and a blob the element keeps is still kept
//! page N             gallery only: jump to page N
//! on CLASSES CMD     run CMD only on these size classes, comma-separated
//!                    (small, mobile, wide): one script drives four sizes even
//!                    where a screen has fewer pages on a larger one
//! ```
//!
//! Ids let one script drive every size. With no script, the gallery snaps
//! every page as `<page>.png` by tapping its "Next ▸" button; OpenSigner
//! snaps `home.png`.
//!
//! File channel: a `file` line answers a request with bytes, as a shell
//! with a picker of its own does, and a `files` line answers it with a
//! listing, as the Pi and the desktop do; the app then asks for one of
//! the names, which is read from that directory. A listing's dates are
//! this shell's own, one hour apart from a fixed instant
//! ([`LISTING_EPOCH`]) in the order the names sort, so a review render
//! is the same on every checkout however old its files are. A file
//! request with nothing queued is answered `FileUnavailable`, which
//! means "this device has no file channel"; there is no picker here to
//! cancel, so no script produces an `Event::FileCancelled`. A
//! `WriteFile` lands in `--out` under the app's name
//! hint and is answered `FileWritten`. Settings channel: this shell keeps nothing, so a
//! `StoreSettings` is dropped; it hands the app one setting at start —
//! that the first run is over, so a script opens on Home — and a
//! `settings` line replaces that with a file's own bytes. Camera channel: `CameraOn` turns the camera "on" (frames come
//! only from `frame` lines) unless `camera off` was given, in which case
//! it is answered `CameraUnavailable` once. Entropy channel: every
//! `RequestEntropy` is answered from a fixed seed ([`ENTROPY_SEED`],
//! stepped per answer), so that a script renders the same pixels on
//! every run; this shell is a review tool, not a signer, and its
//! "entropy" is public by design. Kept-secret channel: a shell with no
//! secure hardware, the default, answers every request "unavailable", so
//! the core offers no stored key; after `restart secure=tee` the shell
//! is a secure element held in memory: a fixed HMAC key
//! ([`ELEMENT_KEY`]) and one blob, which it keeps across later
//! `restart` lines, as a phone keeps it when the app is opened again.
//! Boot: the platform's attestation is `Unknown` unless a `restart`
//! line says `boot=verified` or `boot=unverified`.

use std::collections::VecDeque;
use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use opensigner_core::{AssuranceTier, BuildInfo, OpenSigner};
use osk_shell_api::{
    App, BootState, Command, DisplayInfo, EntropyBytes, Event, FileEntry, FileKind, Frame, Key,
    SecureHardware, TouchPhase,
};
use osk_ui::gallery::Gallery;
use osk_ui::geom::SizeClass;
use osk_ui::widgets::keyboard::KeyInput;
use osk_ui::{Id, Point};

/// The settings this shell hands the app at start: nothing but the one
/// fact that the first run is over, so that a script's first tap lands
/// on Home and not on the Start here document.
const FIRST_RUN_DONE: &[u8] = b"opensigner-settings 1\nfirst_run_done=on\n";

/// The fixed "entropy" this shell answers with. Public, deterministic,
/// and worthless as a secret: review renders must be reproducible.
const ENTROPY_SEED: [u8; 32] = *b"opensigner-snapshot-fixed-seed!!";

/// The date the newest file of a `files` listing carries, in seconds
/// since the epoch (2026-02-02 02:40 UTC), each older file an hour
/// before it. A checkout's own timestamps would put a different date on
/// every render; a review compares pixels.
const LISTING_EPOCH: u64 = 1_770_000_000;

/// The gap between one listed file's date and the next.
const LISTING_STEP: u64 = 3_600;

/// The bottom inset a 1080 x 2340 phone at 420 dpi reports for its
/// gesture bar: 63 px, which is 24 dp. The gallery strips report it for
/// their phone column so that the phone in a strip looks like the phone
/// in a run; `just snapshots` passes the same number to the phone size
/// with `--inset-bottom`.
const PHONE_INSET: u16 = 63;

/// The key a `restart secure=...` element authenticates under. Public
/// and fixed: the element is a review tool's, and a render must be the
/// same on every run.
const ELEMENT_KEY: [u8; 32] = *b"opensigner-snapshot-element-key!";

struct Args {
    width: u16,
    height: u16,
    dpi: u16,
    /// The strip at the bottom edge a person cannot use, in pixels: what
    /// a phone with a gesture bar reports. Zero unless a review asks for
    /// one.
    inset_bottom: u16,
    /// The same at the top edge: a cutout, a status bar.
    inset_top: u16,
    out: PathBuf,
    app: String,
    tier: AssuranceTier,
    script: Option<PathBuf>,
    strip: Option<String>,
    /// `--audit`: after every script line, ask the app what the frame
    /// on screen does wrong (`osk_ui::audit`) and print what is new.
    audit: bool,
    /// Whether the camera is one someone mounted, so the review sees the
    /// camera-rotation row. A snapshot run has no camera of its own.
    camera_mounted: bool,
}

fn usage() -> ExitCode {
    eprintln!(
        "usage: opensigner-snapshot --size WxH --dpi N --out DIR [--app opensigner|gallery] [--tier A|B|C|D] [--script FILE] [--strip NAME] [--camera-mounted] [--inset-bottom PX] [--inset-top PX] [--audit]"
    );
    ExitCode::from(2)
}

/// The app under review.
enum AnyApp {
    OpenSigner(Box<OpenSigner>),
    Gallery(Box<Gallery>),
}

impl App for AnyApp {
    fn event(&mut self, event: Event) {
        match self {
            AnyApp::OpenSigner(a) => a.event(event),
            AnyApp::Gallery(a) => a.event(event),
        }
    }

    fn poll_command(&mut self) -> Option<Command> {
        match self {
            AnyApp::OpenSigner(a) => a.poll_command(),
            AnyApp::Gallery(a) => a.poll_command(),
        }
    }

    fn frame(&mut self) -> Frame<'_> {
        match self {
            AnyApp::OpenSigner(a) => a.frame(),
            AnyApp::Gallery(a) => a.frame(),
        }
    }
}

fn new_app(name: &str, tier: AssuranceTier) -> Option<AnyApp> {
    match name {
        "opensigner" => Some(AnyApp::OpenSigner(Box::new(OpenSigner::new(
            tier,
            BuildInfo {
                version: env!("CARGO_PKG_VERSION"),
                core_hash: None,
            },
        )))),
        "gallery" => Some(AnyApp::Gallery(Box::new(Gallery::new()))),
        _ => None,
    }
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        width: 480,
        height: 640,
        dpi: 286,
        inset_bottom: 0,
        inset_top: 0,
        out: PathBuf::from("out"),
        app: String::from("opensigner"),
        tier: AssuranceTier::C,
        script: None,
        strip: None,
        camera_mounted: false,
        audit: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        let mut value = || it.next().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--size" => {
                let v = value()?;
                let (w, h) = v.split_once('x').ok_or("--size wants WxH")?;
                args.width = w.parse().map_err(|_| "bad width")?;
                args.height = h.parse().map_err(|_| "bad height")?;
            }
            "--dpi" => args.dpi = value()?.parse().map_err(|_| "bad dpi")?,
            "--inset-bottom" => {
                args.inset_bottom = value()?.parse().map_err(|_| "bad inset")?;
            }
            "--inset-top" => args.inset_top = value()?.parse().map_err(|_| "bad inset")?,
            "--out" => args.out = PathBuf::from(value()?),
            "--app" => args.app = value()?,
            "--tier" => {
                args.tier = AssuranceTier::parse(&value()?).ok_or("--tier wants A, B, C or D")?;
            }
            "--script" => args.script = Some(PathBuf::from(value()?)),
            "--strip" => args.strip = Some(value()?),
            "--camera-mounted" => args.camera_mounted = true,
            "--audit" => args.audit = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    // 240 across the short side, 320 along the long one, in either
    // orientation (`docs/PLANNING.md` §4.3).
    if args.width.min(args.height) < 240 || args.width.max(args.height) < 320 {
        return Err(String::from(
            "minimum supported size is 240x320 in either orientation",
        ));
    }
    Ok(args)
}

/// The shell side: owns the app, the clock, and the output directory.
struct Shell {
    app: AnyApp,
    now_ms: u64,
    out: PathBuf,
    written: Vec<PathBuf>,
    /// Bytes queued by `file` lines, one per future file request.
    files: VecDeque<Vec<u8>>,
    /// Directories queued by `files` lines, one listing per future file
    /// request.
    listings: VecDeque<PathBuf>,
    /// The directory the last listing came from, which the app reads a
    /// name from.
    listed: Option<PathBuf>,
    /// Whether the app has the camera on.
    camera_on: bool,
    /// `camera off` was given: the next `CameraOn` is refused.
    camera_absent: bool,
    /// What the next `RequestClipboard` is answered with: `Some(text)`
    /// from a `clipboard "..."` line, `None` after `clipboard off` and
    /// before any line at all.
    clipboard: Option<String>,
    /// The last text a `WriteClipboard` carried, so a script can read
    /// back what Copy put there.
    copied: Option<String>,
    /// `entropy off` was given: entropy requests go unanswered.
    entropy_absent: bool,
    /// After `restart secure=...`, the element: the one blob it keeps,
    /// if any. `None` is a shell with no secure hardware.
    element: Option<Option<Vec<u8>>>,
    /// What the shell reports in `Event::Display`, which a `restart`
    /// line changes the secure hardware and the boot of.
    display: DisplayInfo,
    /// The tier the app is built for.
    tier: AssuranceTier,
    /// Entropy answers given so far; each one differs from the last.
    entropy_answers: u8,
    /// This display's size class, for `on CLASSES CMD` lines.
    class: SizeClass,
    /// With `--audit`, the faults already printed, so each is printed
    /// once, the last snapshot's name, which says where it was, and the
    /// script's name, which the pictures of faulty frames are named by.
    audit: Option<(Vec<String>, String, String)>,
    /// With `--audit`, the kinds of screen audited, printed at the end so
    /// `just audit` can say which kinds no script reaches.
    audited: std::collections::BTreeSet<String>,
}

impl Shell {
    /// Delivers one event and acts on its commands; a file request is
    /// answered at once, and the answer's commands are handled too.
    fn send(&mut self, event: Event) {
        let mut pending = VecDeque::from([event]);
        while let Some(e) = pending.pop_front() {
            self.app.event(e);
            while let Some(c) = self.app.poll_command() {
                match c {
                    // A review tool keeps nothing between runs, so the
                    // settings the core hands over are dropped.
                    Command::Draw
                    | Command::Vibrate { .. }
                    | Command::Exit
                    | Command::StoreSettings { .. } => {}
                    Command::RequestFile { kind } => {
                        let event = if let Some(bytes) = self.files.pop_front() {
                            Event::File { kind, bytes }
                        } else if let Some(dir) = self.listings.pop_front() {
                            let entries = listing(&dir, kind);
                            self.listed = Some(dir);
                            // A review tool's listing is not a medium
                            // anyone would go and find.
                            Event::FileList {
                                kind,
                                entries,
                                place: None,
                            }
                        } else {
                            Event::FileUnavailable { kind }
                        };
                        pending.push_back(event);
                    }
                    Command::ReadFile { kind, name } => {
                        let path = self.listed.as_ref().map(|d| d.join(&name));
                        let bytes = path.as_ref().and_then(|p| fs::read(p).ok());
                        pending.push_back(match bytes {
                            Some(bytes) => Event::File { kind, bytes },
                            None => Event::FileUnavailable { kind },
                        });
                    }
                    Command::WriteFile {
                        kind,
                        name_hint,
                        bytes,
                    } => {
                        let path = self.out.join(name_hint);
                        if let Some(dir) = path.parent() {
                            let _ = fs::create_dir_all(dir);
                        }
                        let event = match fs::write(&path, &bytes) {
                            Ok(()) => {
                                println!("{}", path.display());
                                Event::FileWritten { kind }
                            }
                            Err(e) => {
                                eprintln!("error: cannot write {}: {e}", path.display());
                                Event::FileNotWritten { kind }
                            }
                        };
                        pending.push_back(event);
                    }
                    Command::RequestClipboard { kind } => {
                        pending.push_back(match self.clipboard.clone() {
                            Some(text) => Event::Clipboard { kind, text },
                            None => Event::ClipboardUnavailable { kind },
                        });
                    }
                    Command::WriteClipboard { kind, text } => {
                        println!("copied {text}");
                        self.copied = Some(text.clone());
                        self.clipboard = Some(text);
                        pending.push_back(Event::ClipboardWritten { kind });
                    }
                    Command::CameraOn => {
                        if self.camera_absent {
                            self.camera_absent = false;
                            pending.push_back(Event::CameraUnavailable);
                        } else {
                            self.camera_on = true;
                        }
                    }
                    Command::CameraOff => self.camera_on = false,
                    Command::RequestEntropy => {
                        if !self.entropy_absent {
                            let mut bytes = ENTROPY_SEED;
                            bytes[0] ^= self.entropy_answers;
                            self.entropy_answers = self.entropy_answers.wrapping_add(1);
                            pending.push_back(Event::Entropy(EntropyBytes::new(bytes)));
                        }
                    }
                    // With no element, the answer is "unavailable" to
                    // everything on the kept-secret channel, so the
                    // core offers no stored key.
                    Command::SecureMac { salt } => pending.push_back(match self.element {
                        Some(_) => Event::SecureMac {
                            mac: element_mac(&salt),
                        },
                        None => Event::SecureUnavailable,
                    }),
                    Command::StoreSecret { blob } => pending.push_back(match &mut self.element {
                        Some(kept) => {
                            *kept = Some(blob);
                            Event::SecretStored
                        }
                        None => Event::SecretNotStored,
                    }),
                    Command::LoadSecret => pending.push_back(match &self.element {
                        Some(Some(blob)) => Event::Secret { blob: blob.clone() },
                        _ => Event::SecretUnavailable,
                    }),
                    Command::ForgetSecret => {
                        if let Some(kept) = &mut self.element {
                            *kept = None;
                        }
                        pending.push_back(Event::SecretForgotten);
                    }
                }
            }
        }
    }

    /// What a shell sends a new app: the settings, the display and,
    /// where there is an element, whether it keeps a blob.
    fn start(&mut self) {
        // A review render is of a device that has been used before, so
        // the first run is behind it: the scripts that drive one start
        // on Home. A script that wants the first run sends settings of
        // its own, which replace these (`settings PATH`).
        self.send(Event::Settings {
            bytes: FIRST_RUN_DONE.to_vec(),
        });
        self.send(Event::Display(self.display));
        if let Some(kept) = &self.element {
            let kept = kept.is_some();
            self.send(Event::SecretKept { kept });
        }
        self.drain();
    }

    /// Drains any commands left over (none, normally: `send` handles them).
    fn drain(&mut self) {
        while self.app.poll_command().is_some() {}
    }

    fn tap(&mut self, x: u16, y: u16) {
        self.send(Event::Touch {
            x,
            y,
            phase: TouchPhase::Down,
        });
        self.send(Event::Touch {
            x,
            y,
            phase: TouchPhase::Up,
        });
    }

    fn tick(&mut self, ms: u64) {
        self.now_ms += ms;
        self.send(Event::Tick {
            now_ms: self.now_ms,
        });
    }

    /// Centre of widget `id`, scrolled into view (OpenSigner only).
    fn center_of(&mut self, id: Id) -> Result<Point, String> {
        match &mut self.app {
            AnyApp::OpenSigner(a) => a
                .reveal(id)
                .map(|r| r.center())
                .ok_or_else(|| format!("no widget with id {} on screen", id.0)),
            AnyApp::Gallery(_) => Err(String::from("ids are for --app opensigner")),
        }
    }

    /// Taps key `input` of on-screen keyboard `id` (OpenSigner only).
    fn padkey(&mut self, id: Id, input: KeyInput) -> Result<(), String> {
        let c = match &self.app {
            AnyApp::OpenSigner(a) => a
                .key_rect(id, input)
                .map(|r| r.center())
                .ok_or_else(|| format!("no enabled key {input:?} on keyboard {}", id.0))?,
            AnyApp::Gallery(_) => return Err(String::from("padkey is for --app opensigner")),
        };
        let (x, y) = (
            u16::try_from(c.x).map_err(|_| "key off screen")?,
            u16::try_from(c.y).map_err(|_| "key off screen")?,
        );
        self.tap(x, y);
        Ok(())
    }

    fn snap(&mut self, name: &str) -> Result<(), String> {
        self.drain();
        let frame = self.app.frame();
        let path = self.out.join(format!("{name}.png"));
        write_png(&path, frame.width, frame.height, frame.rgba)?;
        println!("{}", path.display());
        self.written.push(path);
        Ok(())
    }

    fn run_script(&mut self, script: &str) -> Result<(), String> {
        for (n, raw) in script.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            self.run_line(line)
                .map_err(|e| format!("line {}: {e}", n + 1))?;
            if let Some(name) = line.strip_prefix("snap ")
                && let Some((_, last, _)) = &mut self.audit
            {
                *last = String::from(name.trim());
            }
            self.audit_frame(n + 1);
        }
        for kind in &self.audited {
            println!("audited: {kind}");
        }
        Ok(())
    }

    /// With `--audit`, prints each fault of the frame on screen that has
    /// not been printed before: `audit: line 42 (after detail-02-key-menu):
    /// overlap: "Key" and icon Wallet`.
    fn audit_frame(&mut self, line: usize) {
        let AnyApp::OpenSigner(app) = &mut self.app else {
            return;
        };
        let Some((seen, last, script)) = &mut self.audit else {
            return;
        };
        self.audited.insert(format!("{:?}", app.screen()));
        let mut new = false;
        for f in app.audit() {
            if !seen.contains(&f) {
                println!("audit: line {line} (after {last}): {f}");
                seen.push(f);
                new = true;
            }
        }
        // The frame a new fault was found on, so it can be looked at:
        // `audit-wallet-line-42.png` beside the script's own snapshots.
        if new {
            let name = format!("audit-{script}-line-{line}");
            let _ = self.snap(&name);
        }
    }

    fn run_line(&mut self, line: &str) -> Result<(), String> {
        let mut parts = line.splitn(2, ' ');
        let cmd = parts.next().unwrap_or("");
        let rest = parts.next().unwrap_or("").trim();
        let nums = |n: usize| -> Result<Vec<i32>, String> {
            let v: Result<Vec<i32>, _> = rest.split_whitespace().map(str::parse::<i32>).collect();
            let v = v.map_err(|_| format!("{cmd} wants {n} numbers"))?;
            if v.len() != n {
                return Err(format!("{cmd} wants {n} numbers"));
            }
            Ok(v)
        };
        let coord =
            |v: i32| u16::try_from(v).map_err(|_| String::from("coordinates must be non-negative"));
        match cmd {
            "tap" => {
                let v = nums(2)?;
                self.tap(coord(v[0])?, coord(v[1])?);
            }
            "down" | "move" | "up" => {
                let v = nums(2)?;
                let phase = match cmd {
                    "down" => TouchPhase::Down,
                    "move" => TouchPhase::Move,
                    _ => TouchPhase::Up,
                };
                self.send(Event::Touch {
                    x: coord(v[0])?,
                    y: coord(v[1])?,
                    phase,
                });
            }
            "key" => self.send(Event::Key(parse_key(rest)?)),
            "text" => {
                let s = rest.trim_matches('"');
                for c in s.chars() {
                    self.send(Event::Key(Key::Char(c)));
                }
            }
            "word" => {
                // §16.118: the letters never take the word. A word
                // typed out in full is the one candidate left, which
                // the strip's first cell carries; on `small` a lone
                // candidate is already selected, so one tap takes it
                // there as well. The strip is a fixed grid, so the first
                // cell is where the first candidate always is (§16.120).
                let word = rest.trim_matches('"');
                for c in word.chars() {
                    self.send(Event::Key(Key::Char(c)));
                }
                let id = opensigner_core::ids::LOAD_CANDIDATES;
                let c = match &self.app {
                    AnyApp::OpenSigner(a) => a
                        .candidate_rect(id, 0)
                        .map(|r| r.center())
                        .ok_or_else(|| format!("{word} is not on the candidate strip"))?,
                    AnyApp::Gallery(_) => return Err(String::from("word is for --app opensigner")),
                };
                self.tap(coord(c.x)?, coord(c.y)?);
            }
            "scroll" => {
                let v = nums(3)?;
                self.send(Event::Scroll {
                    x: coord(v[0])?,
                    y: coord(v[1])?,
                    dy: i16::try_from(v[2]).map_err(|_| "dy out of range")?,
                });
            }
            "tick" => {
                let v = nums(1)?;
                self.tick(u64::try_from(v[0]).map_err(|_| "tick wants a positive delta")?);
            }
            "tapid" => {
                let v = nums(1)?;
                let c = self.center_of(Id(u32::try_from(v[0]).map_err(|_| "bad id")?))?;
                self.tap(coord(c.x)?, coord(c.y)?);
            }
            "holdid" => {
                let v = nums(1)?;
                let c = self.center_of(Id(u32::try_from(v[0]).map_err(|_| "bad id")?))?;
                let (x, y) = (coord(c.x)?, coord(c.y)?);
                self.send(Event::Touch {
                    x,
                    y,
                    phase: TouchPhase::Down,
                });
                self.tick(100);
                self.tick(osk_ui::widgets::tokens::HOLD_MS);
                self.send(Event::Touch {
                    x,
                    y,
                    phase: TouchPhase::Up,
                });
            }
            "pressid" | "releaseid" => {
                let v = nums(1)?;
                let c = self.center_of(Id(u32::try_from(v[0]).map_err(|_| "bad id")?))?;
                let (x, y) = (coord(c.x)?, coord(c.y)?);
                let phase = if cmd == "pressid" {
                    TouchPhase::Down
                } else {
                    TouchPhase::Up
                };
                self.send(Event::Touch { x, y, phase });
                if cmd == "pressid" {
                    self.tick(100);
                }
            }
            "scrollid" => {
                // The id is parsed on its own: a screen's reserved
                // rectangles sit at the top of the `u32` range.
                let (id, dy) = rest
                    .split_once(' ')
                    .ok_or("scrollid wants an id and a delta")?;
                let id: u32 = id.trim().parse().map_err(|_| "bad id")?;
                let dy: i16 = dy.trim().parse().map_err(|_| "dy out of range")?;
                let c = self.center_of(Id(id))?;
                self.send(Event::Scroll {
                    x: coord(c.x)?,
                    y: coord(c.y)?,
                    dy,
                });
            }
            "cand" => {
                // A candidate cell of a strip: the words the entry
                // screen offers, and the last cell where it pages.
                let (id, n) = rest
                    .split_once(' ')
                    .ok_or("cand wants a strip id and a cell")?;
                let id = Id(id.trim().parse().map_err(|_| "bad id")?);
                let n: usize = n.trim().parse().map_err(|_| "bad cell")?;
                let c = match &self.app {
                    AnyApp::OpenSigner(a) => a
                        .candidate_rect(id, n)
                        .map(|r| r.center())
                        .ok_or_else(|| format!("no candidate cell {n} on strip {}", id.0))?,
                    AnyApp::Gallery(_) => return Err(String::from("cand is for --app opensigner")),
                };
                self.tap(coord(c.x)?, coord(c.y)?);
            }
            "candmore" => {
                // The cell that turns the candidate strip's page,
                // wherever the class puts it.
                let c = match &self.app {
                    AnyApp::OpenSigner(a) => a
                        .candidate_more_rect()
                        .map(|r| r.center())
                        .ok_or("the strip has no page to turn")?,
                    AnyApp::Gallery(_) => {
                        return Err(String::from("candmore is for --app opensigner"));
                    }
                };
                self.tap(coord(c.x)?, coord(c.y)?);
            }
            "padkey" => {
                let (id, key) = rest.split_once(' ').ok_or("padkey wants an id and a key")?;
                let id = Id(id.trim().parse().map_err(|_| "bad id")?);
                let input = match key.trim() {
                    "backspace" => KeyInput::Backspace,
                    "enter" => KeyInput::Done,
                    // The jamo keyboard's shift, which the doubled
                    // consonants and the two shifted vowels sit under.
                    "shift" => KeyInput::Shift,
                    k => {
                        let mut chars = k.chars();
                        match (chars.next(), chars.next()) {
                            (Some(c), None) => KeyInput::Char(c),
                            _ => return Err(format!("unknown pad key {k}")),
                        }
                    }
                };
                self.padkey(id, input)?;
            }
            "pin" => {
                let (id, digits) = rest.split_once(' ').ok_or("pin wants an id and digits")?;
                let id = Id(id.trim().parse().map_err(|_| "bad id")?);
                for c in digits.trim().chars() {
                    self.padkey(id, KeyInput::Char(c))?;
                }
                self.padkey(id, KeyInput::Done)?;
            }
            "entropy" => {
                if rest != "off" {
                    return Err(String::from("entropy wants `off`"));
                }
                self.entropy_absent = true;
            }
            "snap" => {
                if rest.is_empty() {
                    return Err(String::from("snap wants a name"));
                }
                self.snap(rest)?;
            }
            "settings" => {
                if rest.is_empty() {
                    return Err(String::from("settings wants a path"));
                }
                let bytes = fs::read(rest).map_err(|e| format!("read {rest}: {e}"))?;
                self.send(Event::Settings { bytes });
            }
            "file" => {
                if rest.is_empty() {
                    return Err(String::from("file wants a path"));
                }
                let bytes = fs::read(rest).map_err(|e| format!("read {rest}: {e}"))?;
                self.files.push_back(bytes);
            }
            "files" => {
                if rest.is_empty() {
                    return Err(String::from("files wants a directory"));
                }
                let dir = PathBuf::from(rest);
                if !dir.is_dir() {
                    return Err(format!("{rest} is not a directory"));
                }
                self.listings.push_back(dir);
            }
            "frame" => {
                if rest.is_empty() {
                    return Err(String::from("frame wants a PNG path"));
                }
                if !self.camera_on {
                    return Err(String::from("the app does not have the camera on"));
                }
                let (width, height, luma) = read_png_luma(rest)?;
                // The fixture is a still, not a camera: there is no
                // chroma plane to send and the preview draws it grey.
                self.send(Event::CameraFrame {
                    width,
                    height,
                    luma: luma.clone(),
                    chroma: None,
                });
                // Reading the frame is the shell's, as it is on a real
                // shell; this one has a script to get through and no
                // reason for a worker, so it reads the frame here and
                // hands over what it found before the next line.
                if let Some(bytes) = opensigner_scanner::decode_frame(width, height, &luma) {
                    self.send(Event::Scanned { bytes });
                }
            }
            "clipboard" => {
                if rest.is_empty() {
                    return Err(String::from("clipboard wants text or `off`"));
                }
                self.clipboard = if rest == "off" {
                    None
                } else {
                    Some(String::from(unquote(rest)))
                };
            }
            "camera" => {
                if rest != "off" {
                    return Err(String::from("camera wants `off`"));
                }
                self.camera_absent = true;
            }
            "on" => {
                let mut p = rest.splitn(2, ' ');
                let classes = p.next().unwrap_or("");
                let inner = p.next().unwrap_or("").trim();
                if inner.is_empty() {
                    return Err(String::from("on wants CLASSES and a command"));
                }
                let here = match self.class {
                    SizeClass::Small => "small",
                    SizeClass::Mobile => "mobile",
                    SizeClass::Wide => "wide",
                };
                let mut known = true;
                let wanted = classes.split(',').any(|c| {
                    known &= matches!(c, "small" | "mobile" | "wide");
                    c == here
                });
                if !known {
                    return Err(format!("on wants small, mobile or wide, not {classes}"));
                }
                if wanted {
                    self.run_line(inner)?;
                }
            }
            "restart" => {
                if !matches!(self.app, AnyApp::OpenSigner(_)) {
                    return Err(String::from("restart is for --app opensigner"));
                }
                for option in rest.split_whitespace() {
                    let (name, value) = option
                        .split_once('=')
                        .ok_or("restart wants tier=, secure= or boot=")?;
                    match (name, value) {
                        ("tier", v) => {
                            self.tier = AssuranceTier::parse(v).ok_or("tier wants A, B, C or D")?;
                        }
                        ("secure", "none") => self.display.secure = SecureHardware::None,
                        ("secure", "tee") => self.display.secure = SecureHardware::Tee,
                        ("secure", "strongbox") => self.display.secure = SecureHardware::StrongBox,
                        ("boot", "unknown") => self.display.boot = BootState::Unknown,
                        ("boot", "verified") => self.display.boot = BootState::Verified,
                        ("boot", "unverified") => self.display.boot = BootState::Unverified,
                        _ => return Err(format!("restart does not know {option}")),
                    }
                }
                // The element is the device's, so its blob outlives the
                // app; a device with no secure hardware has none.
                if self.display.secure == SecureHardware::None {
                    self.element = None;
                } else if self.element.is_none() {
                    self.element = Some(None);
                }
                self.app = new_app("opensigner", self.tier).ok_or("no opensigner app")?;
                self.camera_on = false;
                self.start();
            }
            "page" => {
                let v = nums(1)?;
                match &mut self.app {
                    AnyApp::Gallery(g) => {
                        g.set_page(usize::try_from(v[0]).map_err(|_| "bad page")?);
                    }
                    AnyApp::OpenSigner(_) => return Err(String::from("page is for --app gallery")),
                }
            }
            other => return Err(format!("unknown command {other}")),
        }
        Ok(())
    }

    /// Snaps every gallery page in order, or OpenSigner's Home.
    fn run_default(&mut self) -> Result<(), String> {
        let pages = match &self.app {
            AnyApp::Gallery(_) => Gallery::page_count(),
            AnyApp::OpenSigner(_) => return self.snap("home"),
        };
        for _ in 0..pages {
            let (name, next) = match &self.app {
                AnyApp::Gallery(g) => (g.page_name(), g.next_button_center()),
                AnyApp::OpenSigner(_) => unreachable!(),
            };
            self.snap(name)?;
            let c = next.ok_or("gallery has no Next button")?;
            self.tap(c.x as u16, c.y as u16);
        }
        Ok(())
    }
}

fn parse_key(s: &str) -> Result<Key, String> {
    Ok(match s {
        "backspace" => Key::Backspace,
        "enter" => Key::Enter,
        "escape" => Key::Escape,
        "up" => Key::Up,
        "down" => Key::Down,
        "left" => Key::Left,
        "right" => Key::Right,
        "tab" => Key::Tab,
        "space" => Key::Char(' '),
        other => {
            let mut chars = other.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => Key::Char(c),
                _ => return Err(format!("unknown key {other}")),
            }
        }
    })
}

/// Reads a PNG of any colour type and depth as 8-bit luma.
fn read_png_luma(path: &str) -> Result<(u16, u16, Vec<u8>), String> {
    let file = fs::File::open(path).map_err(|e| format!("read {path}: {e}"))?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| format!("{path}: {e}"))?;
    let mut buf = vec![0u8; reader.output_buffer_size().unwrap_or(0)];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("{path}: {e}"))?;
    let (w, h) = (info.width as usize, info.height as usize);
    let px = &buf[..info.buffer_size()];
    let luma: Vec<u8> = match info.color_type {
        png::ColorType::Grayscale => px.to_vec(),
        png::ColorType::GrayscaleAlpha => px.chunks_exact(2).map(|p| p[0]).collect(),
        png::ColorType::Rgb => px
            .chunks_exact(3)
            .map(|p| luma_of(p[0], p[1], p[2]))
            .collect(),
        png::ColorType::Rgba => px
            .chunks_exact(4)
            .map(|p| luma_of(p[0], p[1], p[2]))
            .collect(),
        png::ColorType::Indexed => return Err(format!("{path}: palette PNG not expanded")),
    };
    if luma.len() != w * h {
        return Err(format!("{path}: unexpected pixel count"));
    }
    Ok((
        u16::try_from(w).map_err(|_| "frame too wide")?,
        u16::try_from(h).map_err(|_| "frame too tall")?,
        luma,
    ))
}

/// HMAC-SHA256 of `salt` under [`ELEMENT_KEY`]: what the element
/// answers a `SecureMac` with.
fn element_mac(salt: &[u8; 32]) -> [u8; 32] {
    use hmac::{Hmac, KeyInit, Mac};
    let mut mac =
        Hmac::<sha2::Sha256>::new_from_slice(&ELEMENT_KEY).expect("HMAC accepts any key length");
    mac.update(salt);
    mac.finalize().into_bytes().into()
}

/// Rec. 601 luma, integer.
fn luma_of(r: u8, g: u8, b: u8) -> u8 {
    ((u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114) / 1000) as u8
}

/// Writes premultiplied RGBA as a straight-alpha PNG.
fn write_png(path: &Path, width: u16, height: u16, premultiplied: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    }
    let mut straight = Vec::with_capacity(premultiplied.len());
    for px in premultiplied.chunks_exact(4) {
        let a = u32::from(px[3]);
        if a == 0 {
            straight.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            let un = |c: u8| ((u32::from(c) * 255 + a / 2) / a).min(255) as u8;
            straight.extend_from_slice(&[un(px[0]), un(px[1]), un(px[2]), px[3]]);
        }
    }
    let file = fs::File::create(path).map_err(|e| format!("create {}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), u32::from(width), u32::from(height));
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer
        .write_image_data(&straight)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Renders one page at the three reference sizes side by side.
fn write_strip(page: &str, out: &Path) -> Result<(), String> {
    // Width, height, dpi and the bottom inset the shell of that device
    // would report: only the phone has a gesture bar, and PHONE_INSET is
    // what it takes from the frame.
    const SIZES: [(u16, u16, u16, u16); 4] = [
        (240, 320, 143, 0),
        (480, 640, 286, 0),
        (1080, 2340, 420, PHONE_INSET),
        (960, 640, 160, 0),
    ];
    const GAP: usize = 16;
    let page_index = osk_ui::gallery::PAGES
        .iter()
        .position(|p| *p == page)
        .ok_or_else(|| format!("unknown page {page}"))?;
    let mut frames: Vec<(u16, u16, Vec<u8>)> = Vec::new();
    for (w, h, dpi, inset) in SIZES {
        let mut app = Gallery::new();
        app.event(Event::Display(DisplayInfo {
            width: w,
            height: h,
            dpi,
            inset_bottom: inset,
            inset_top: 0,
            buttons: 0,
            camera_fixed: true,
            secure: SecureHardware::None,
            boot: BootState::Unknown,
            // The snapshots are one picture per screen, so the device
            // this runs on must not change what is drawn.
            memory_mib: None,
        }));
        app.set_page(page_index);
        while app.poll_command().is_some() {}
        let f = app.frame();
        frames.push((f.width, f.height, f.rgba.to_vec()));
    }
    let total_w: usize =
        frames.iter().map(|f| usize::from(f.0)).sum::<usize>() + GAP * (frames.len() - 1);
    let total_h: usize = frames.iter().map(|f| usize::from(f.1)).max().unwrap_or(0);
    // Opaque near-black between and below the frames.
    let mut strip: Vec<u8> = [0x00, 0x00, 0x00, 0xff]
        .iter()
        .copied()
        .cycle()
        .take(total_w * total_h * 4)
        .collect();
    let mut x0 = 0usize;
    for (w, h, data) in &frames {
        let (w, h) = (usize::from(*w), usize::from(*h));
        for y in 0..h {
            let src = &data[y * w * 4..(y + 1) * w * 4];
            let dst = (y * total_w + x0) * 4;
            strip[dst..dst + w * 4].copy_from_slice(src);
        }
        x0 += w + GAP;
    }
    let path = out.join(format!("{page}-strip.png"));
    write_png(
        &path,
        u16::try_from(total_w).map_err(|_| "strip too wide")?,
        u16::try_from(total_h).map_err(|_| "strip too tall")?,
        &strip,
    )?;
    println!("{}", path.display());
    Ok(())
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            return usage();
        }
    };
    let Some(app) = new_app(&args.app, args.tier) else {
        eprintln!("error: unknown app {} (opensigner or gallery)", args.app);
        return usage();
    };
    if let Some(page) = &args.strip {
        if !matches!(app, AnyApp::Gallery(_)) {
            eprintln!("error: --strip is for --app gallery");
            return usage();
        }
        return match write_strip(page, &args.out) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        };
    }

    let display = DisplayInfo {
        width: args.width,
        height: args.height,
        dpi: args.dpi,
        inset_bottom: args.inset_bottom,
        inset_top: args.inset_top,
        buttons: 0,
        camera_fixed: !args.camera_mounted,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    };
    let mut shell = Shell {
        app,
        now_ms: 0,
        out: args.out.clone(),
        written: Vec::new(),
        files: VecDeque::new(),
        listings: VecDeque::new(),
        listed: None,
        clipboard: None,
        copied: None,
        camera_on: false,
        camera_absent: false,
        entropy_absent: false,
        element: None,
        display,
        tier: args.tier,
        entropy_answers: 0,
        class: SizeClass::of(&display),
        audit: args.audit.then(|| {
            let script = args
                .script
                .as_ref()
                .and_then(|p| p.file_stem())
                .map_or_else(
                    || String::from("default"),
                    |s| s.to_string_lossy().into_owned(),
                );
            (Vec::new(), String::from("start"), script)
        }),
        audited: std::collections::BTreeSet::new(),
    };
    shell.start();

    let result = match &args.script {
        Some(path) => match fs::read_to_string(path) {
            Ok(script) => shell.run_script(&script),
            Err(e) => Err(format!("read {}: {e}", path.display())),
        },
        None => shell.run_default(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// A directory as a shell that can list its files reports it: the files
/// that could be what was asked for, named in sort order and dated an
/// hour apart from [`LISTING_EPOCH`], newest first. The dates are this
/// shell's own so that a render is the same on every checkout.
fn listing(dir: &Path, kind: FileKind) -> Vec<FileEntry> {
    let mut names: Vec<(String, u64)> = match fs::read_dir(dir) {
        Ok(entries) => entries
            .flatten()
            .filter(|e| e.metadata().is_ok_and(|m| m.is_file()))
            .filter_map(|e| {
                let name = e.file_name().to_str()?.to_string();
                let size = e.metadata().ok()?.len();
                wanted(&name, kind).then_some((name, size))
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    names.sort();
    names
        .into_iter()
        .enumerate()
        .map(|(i, (name, size))| FileEntry {
            name,
            size,
            modified: Some(LISTING_EPOCH - i as u64 * LISTING_STEP),
        })
        .collect()
}

/// A script argument with its surrounding double quotes taken off, so a
/// `clipboard "..."` line can carry spaces.
fn unquote(text: &str) -> &str {
    text.strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .unwrap_or(text)
}

/// Whether a file could be what was asked for: the rule the Pi and the
/// desktop read their directories by.
fn wanted(name: &str, kind: FileKind) -> bool {
    match kind {
        FileKind::Psbt => {
            let lower = name.to_ascii_lowercase();
            lower.ends_with(".psbt") && lower.len() > ".psbt".len()
        }
        // Nothing asks for a text *file*: the kind names a clipboard
        // payload, and a listing of it is a listing of everything.
        FileKind::Text | FileKind::Any => !name.starts_with('.'),
        FileKind::Png => {
            let lower = name.to_ascii_lowercase();
            lower.ends_with(".png") && lower.len() > ".png".len()
        }
    }
}
