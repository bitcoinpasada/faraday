//! `Facts` kept from port facts, `View` and `Show` built (05 §4).

use faraday_kernel::{
    AtLock, Clock, Downloads, Facts, FactsChanged, Memo, Mode, NoteUp, Running, SelfTest, Show,
    Started, Sticks, SystemFacts, View, session, shared, vaults,
};
use faraday_port as port;
use faraday_port::event::{
    Abilities, CameraFact, Event, HostFact, Machine, SecureBootAtStart, SticksFact, SystemFact,
};
use faraday_port::values::{Display, Medium, Orientation, Pace};

use crate::queue::{Item, Pending};
use crate::root::{Root, Stream, Tops};
use crate::runtime::Runtime;

/// What the process keeps across an in-process rebuild (§4.2): the facts, which belong to the
/// process and not to the scope, and the clean marker's latch.
pub(crate) struct Kept {
    /// Every field of the process's `Start` but `carried`, updated by later facts, and the rest.
    pub(crate) facts: Facts,
    /// The clean marker's latch (R14): stays set across a rebuild.
    pub(crate) tainted: bool,
    /// The last value of the marker worked out (and sent as `Order::Clean`).
    pub(crate) clean: bool,
    /// Whether the marker was sent once yet: at `Start`, after the model is built (R14).
    pub(crate) clean_sent: bool,
    /// The wall second the last `Notice::Facts(Time)` was sent for.
    pub(crate) last_second: Option<u64>,
}

impl Kept {
    /// Before `Start`: nothing is known and nothing reads it.
    pub(crate) fn new() -> Self {
        Kept {
            facts: Facts {
                display: Display {
                    width: 0,
                    height: 0,
                    dpi: 96,
                    orientation: Orientation::Landscape,
                    rotation_offered: None,
                    external: false,
                    preview: None,
                },
                medium: Medium::Stick,
                abilities: Abilities {
                    clipboard: false,
                    print: false,
                    host_files: false,
                    opens: false,
                    camera: false,
                    names_sticks: false,
                    holds_back_input: false,
                    restart_computer: false,
                    moves_window: false,
                    system: false,
                    lock_ends_process: false,
                },
                machine: Machine {
                    maker: None,
                    product: None,
                    family: None,
                    chassis: None,
                    hypervisor: false,
                    screen: None,
                    secure_boot: SecureBootAtStart::Unknown,
                    memory_total_mib: None,
                    cores: 0,
                },
                root_read_only: None,
                sticks: Sticks {
                    attached: false,
                    listed: Vec::new(),
                    boot_medium: None,
                    problems: Vec::new(),
                },
                cameras: Vec::new(),
                downloads: None,
                entropy_source: None,
                selftest: SelfTest::NotRun,
                clean: true,
                system: SystemFacts {
                    computer: None,
                    disks: Vec::new(),
                    no_writer: false,
                    installed: None,
                    dev_image: None,
                    images_wanted: Vec::new(),
                    concerns: Vec::new(),
                    other_encryption: None,
                    boot_entry: None,
                },
            },
            tainted: false,
            clean: true,
            clean_sent: false,
            last_second: None,
        }
    }
}

/// The three places a `View` reads that are not the facts: the settings, and the session and
/// vaults readers (the empty ones where the part has none to read, R13, R26).
pub(crate) struct Lent<'a> {
    pub(crate) settings: &'a shared::Settings,
    pub(crate) session: &'a dyn session::Read,
    pub(crate) vaults: &'a dyn vaults::Read,
}

/// `View` from shared borrows of separate places (§4.1), so a top state can be borrowed `&mut`
/// beside it. The clock's offset and whether it was set are the settings'.
pub(crate) fn build_view<'a>(
    lent: Lent<'a>,
    facts: &'a Facts,
    mode: Mode,
    clock: &Clock,
    running: &'a [Running],
    started: Started,
) -> View<'a> {
    View {
        session: lent.session,
        vaults: lent.vaults,
        settings: lent.settings,
        facts,
        mode,
        clock: Clock {
            now_ms: clock.now_ms,
            unix_secs: clock.unix_secs,
            offset_secs: lent.settings.offset_secs,
            set: lent.settings.checked,
            last_input_ms: clock.last_input_ms,
        },
        running,
        started,
    }
}

/// The settings as they were before a message to the settings owner (R13): every field is
/// `Copy`, so the copy is made field by field.
pub(crate) fn as_was(settings: &shared::Settings) -> shared::Settings {
    shared::Settings {
        idle_lock_min: settings.idle_lock_min,
        idle_off_min: settings.idle_off_min,
        qr_frame_ms: settings.qr_frame_ms,
        zone_min: settings.zone_min,
        offset_secs: settings.offset_secs,
        checked: settings.checked,
        appearance: settings.appearance,
        scale_pct: settings.scale_pct,
        rotation: settings.rotation,
        seal_amounts: settings.seal_amounts,
        nonce_history: settings.nonce_history,
    }
}

/// `Show` for a frame, with the memo and the note up (§4.1): the wording is the runtime's and
/// goes to `present` alone.
pub(crate) fn show_of<'a>(memo: &'a Memo, note: Option<NoteUp<'a>>) -> Show<'a> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the runtime builds `Show` for each frame (04 §3)"
    )]
    Show::new(&faraday_strings::EN, memo, note)
}

impl<R: Root> Runtime<R> {
    /// §2.2: the field of `Facts` replaced, then `Notice::Facts` to every top state; the
    /// facts that go to one feature are a stream.
    pub(crate) fn keep_fact(&mut self, event: Event) {
        let facts = &mut self.facts.facts;
        match event {
            Event::Display(display) => {
                facts.display = display;
                self.notify(Pending::Facts(FactsChanged::Display));
            }
            Event::Sticks(fact) => {
                match fact {
                    SticksFact::Attached(attached) => facts.sticks.attached = attached,
                    SticksFact::Listed(listed) => facts.sticks.listed = listed,
                    SticksFact::Problem { stick, read_only } => {
                        match facts
                            .sticks
                            .problems
                            .iter_mut()
                            .find(|(id, _)| *id == stick)
                        {
                            Some(known) => known.1 = read_only,
                            None => facts.sticks.problems.push((stick, read_only)),
                        }
                    }
                    SticksFact::BootMedium(medium) => facts.sticks.boot_medium = Some(medium),
                }
                self.notify(Pending::Facts(FactsChanged::Sticks));
            }
            Event::Camera(CameraFact::Listed(cameras)) => {
                facts.cameras = cameras;
                self.notify(Pending::Facts(FactsChanged::Cameras));
            }
            Event::Host(HostFact::Downloads { dir, files }) => {
                facts.downloads = Some(Downloads { dir, files });
                self.notify(Pending::Facts(FactsChanged::Downloads));
            }
            Event::System(fact @ SystemFact::WriteProgress { .. }) => {
                self.stream(Stream::System(fact));
            }
            Event::System(fact) => {
                match fact {
                    SystemFact::Computer(computer) => facts.system.computer = Some(computer),
                    SystemFact::Disks(disks) => facts.system.disks = disks,
                    SystemFact::NoWriter => facts.system.no_writer = true,
                    SystemFact::Installed(installed) => facts.system.installed = installed,
                    SystemFact::DevImage(image) => facts.system.dev_image = Some(image),
                    SystemFact::ImagesWanted(images) => facts.system.images_wanted = images,
                    SystemFact::Concerns(concerns) => facts.system.concerns = concerns,
                    SystemFact::OtherEncryption(other) => facts.system.other_encryption = other,
                    SystemFact::BootEntry(entry) => facts.system.boot_entry = Some(entry),
                    SystemFact::WriteProgress { .. } => {}
                }
                self.notify(Pending::Facts(FactsChanged::System));
            }
            Event::Devices(fact) => self.stream(Stream::Devices(fact)),
            Event::Camera(fact) => {
                if self.stream_check(&fact) {
                    self.stream(Stream::Camera(fact));
                }
            }
            Event::Host(fact) => self.stream(Stream::Host(fact)),
            Event::Start(_)
            | Event::Time(_)
            | Event::Input(_)
            | Event::CloseAsked
            | Event::Answer(..) => {}
        }
    }

    /// `Event::Time`: the clock set; `Notice::Facts` with `time` once each wall second; a frame
    /// is wanted when a note is due or the last frame said `Pace::Moving` (§2.2, §2.5).
    pub(crate) fn timed(&mut self, time: port::event::Time) {
        self.clock.now_ms = time.now_ms;
        self.clock.unix_secs = time.unix_secs;
        if self.facts.last_second != Some(time.unix_secs) {
            self.facts.last_second = Some(time.unix_secs);
            self.notify(Pending::Facts(FactsChanged::Time));
        }
        self.advance_motion(time.now_ms);
        if self.notes.tick(time.now_ms) {
            self.frame.want();
        } else if self.frame.pace() == Pace::Moving && !self.frame.wanted {
            // Motion the store advanced asked for the frame already, and as its own.
            self.frame.want();
        }
    }

    /// A stream fact: `Root::fact` says which top state it is for.
    fn stream(&mut self, stream: Stream) {
        let Some(tops) = Tops::lend(&self.tops) else {
            self.fault("a fact before the model was built");
            return;
        };
        if let Some(addr) = self.root.fact(&tops, stream) {
            self.queue.push(Item::Msg(addr));
        }
    }

    /// R14: `holds()` of every top state read; `Order::Clean` once at `Start`, after the model is
    /// built, then when the value changed.
    pub(crate) fn clean_marker(&mut self) -> Option<port::Effect> {
        if self.phase == crate::runtime::Phase::Starting {
            return None;
        }
        let tops = &self.tops;
        let holds = [
            tops.settings.as_ref().map(AtLock::holds),
            tops.session.as_ref().map(AtLock::holds),
            tops.vaults.as_ref().map(AtLock::holds),
            tops.app.as_ref().map(AtLock::holds),
            tops.outside.as_ref().map(AtLock::holds),
        ];
        let busy = holds.iter().flatten().any(|h| h.kept || h.typed);
        if busy {
            self.facts.tainted = true;
        }
        let clean = !self.facts.tainted && self.scope.stream.is_none() && !busy;
        if self.facts.clean_sent && clean == self.facts.clean {
            return None;
        }
        self.facts.clean_sent = true;
        self.facts.clean = clean;
        self.facts.facts.clean = clean;
        Some(port::Effect::Order(port::Order::Clean(clean)))
    }

    /// The self-test, run once per process on the root's thread (§4.2); its outcome is
    /// `Facts::selftest`.
    pub(crate) fn run_selftest(&mut self) {
        let outcome = faraday_selftest::run(&self.compute, self.facts.facts.root_read_only);
        self.facts.facts.selftest = match outcome {
            Ok(_) => SelfTest::Passed,
            Err(name) => SelfTest::Failed(name),
        };
    }
}
