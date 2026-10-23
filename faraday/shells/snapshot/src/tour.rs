//! The tour driver: the state a tour carries, the pump that answers the
//! app's commands, and the helpers several tours share.

use faraday_core::board::{BoardPress as BP, Chip, Thing, Zone};
use faraday_core::plan::{self as fplan, At, Spot, What};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Medium, Screen, StickInfo, StorageCommand, StorageEvent};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, BootState, Command, DisplayInfo, Event, Key, SecureHardware};
use std::path::{Path, PathBuf};

pub(crate) struct Tour {
    pub(crate) app: Faraday,
    pub(crate) out: PathBuf,
    pub(crate) kit: PathBuf,
    pub(crate) n: u32,
    pub(crate) now: u64,
    pub(crate) sticks: Vec<StickInfo>,
    /// Entropy requests answered, so each answer differs.
    pub(crate) answers: u8,
    pub(crate) dpi: u16,
    /// The online app's folders, for Transfer.
    pub(crate) host: Option<faraday_storage::Host>,
    /// What the app last asked to keep across a lock: the Inbox, the
    /// Outbox and the kept state.
    pub(crate) boxes: Option<(Files, Files, Files)>,
    /// The person goes on from the reminders sheet (F24) the first time a
    /// secret opens it, as every new process owes them again. Off while
    /// the tour shows that sheet itself.
    pub(crate) go_on: bool,
}

pub(crate) type Files = Vec<(String, Vec<u8>)>;

impl Tour {
    /// Back up's plan made into the plan preset `p` once placed, by the
    /// presses a person makes now (`testkit::backup_as`,
    /// `docs/BACKUP-FLOW.md` §17.21).
    pub(crate) fn backup_as(&mut self, p: faraday_core::plan::Preset) {
        testkit::backup_as(&mut self.app, p);
        self.pump();
    }

    pub(crate) fn stick(&self) -> StickInfo {
        let mut files: Vec<(String, u64)> = std::fs::read_dir(&self.kit)
            .map(|d| {
                d.filter_map(Result::ok)
                    .filter_map(|e| {
                        let m = e.metadata().ok()?;
                        m.is_file()
                            .then(|| (e.file_name().to_string_lossy().into_owned(), m.len()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        files.push(("IMG_0412.jpg".into(), 2_300_000));
        files.sort();
        StickInfo {
            serial: None,
            id: self.kit.display().to_string(),
            label: "TESTSTICK".into(),
            boot: false,
            files,
        }
    }

    /// Runs every command the app has queued, answering storage requests
    /// from the test kit's directory.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub(crate) fn pump(&mut self) {
        while let Some(c) = self.app.poll_command() {
            // Fixed bytes stand for the system's entropy, different for
            // each request as a generator's are: the tour is repeatable,
            // and nothing it makes holds value.
            if c == Command::RequestEntropy {
                let bytes = [0x5a ^ self.answers; 32];
                self.answers = self.answers.wrapping_add(1);
                self.app.event(Event::EntropySource("getrandom(2)"));
                self.app
                    .event(Event::Entropy(faraday_shell_api::EntropyBytes::new(bytes)));
                continue;
            }
            if c == Command::Exit {
                self.restart();
            }
        }
        while let Some(c) = self.app.poll_storage() {
            match c {
                StorageCommand::Read { stick, name } => {
                    let ev = match std::fs::read(Path::new(&stick).join(&name)) {
                        Ok(bytes) => StorageEvent::Read { stick, name, bytes },
                        Err(e) => StorageEvent::ReadFailed {
                            stick,
                            name,
                            reason: e.to_string(),
                        },
                    };
                    self.app.storage(ev);
                }
                StorageCommand::Write { stick, name, .. } => {
                    let wrote_as = name.clone();
                    self.app.storage(StorageEvent::Written {
                        stick,
                        name,
                        wrote_as,
                    });
                }
                StorageCommand::Print { name, bytes } => {
                    let path = self.out.join(&name);
                    let ev = match std::fs::write(&path, bytes) {
                        Ok(()) => StorageEvent::Printed {
                            path: path.display().to_string(),
                        },
                        Err(e) => StorageEvent::PrintFailed {
                            reason: e.to_string(),
                        },
                    };
                    self.app.storage(ev);
                }
                StorageCommand::ReadQr { stick, name } => {
                    let ev = match faraday_storage::read_qr_png(&self.app.compute(), &stick, &name)
                    {
                        Ok((bytes, payloads)) => StorageEvent::QrRead {
                            name,
                            bytes,
                            payloads,
                        },
                        Err(reason) => StorageEvent::ReadFailed {
                            stick,
                            name,
                            reason,
                        },
                    };
                    self.app.storage(ev);
                }
                StorageCommand::ReadHost { path } => {
                    let ev = match faraday_storage::read_host(Path::new(&path)) {
                        Ok(bytes) => StorageEvent::HostRead { path, bytes },
                        Err(reason) => StorageEvent::HostReadFailed { path, reason },
                    };
                    self.app.storage(ev);
                }
                StorageCommand::SaveDownload { name, bytes } => {
                    let Some(host) = self.host.as_ref() else {
                        self.app.storage(StorageEvent::SaveFailed {
                            reason: "no Downloads folder".into(),
                        });
                        continue;
                    };
                    let ev = match faraday_storage::save_download(&host.downloads, &name, &bytes) {
                        Ok(path) => StorageEvent::Saved {
                            path: path.display().to_string(),
                        },
                        Err(reason) => StorageEvent::SaveFailed { reason },
                    };
                    let files = host.files();
                    self.app.storage(ev);
                    self.app.storage(files);
                }
                // A stick's space as a 8 GB stick's: the tour's sticks are
                // not partitions.
                StorageCommand::StickSpace { stick } => {
                    self.app.storage(StorageEvent::StickSpace {
                        stick,
                        free: 6_650_000_000,
                        size: 7_950_000_000,
                    });
                }
                StorageCommand::SaveBoxes {
                    inbox,
                    outbox,
                    kept,
                } => self.boxes = Some((inbox, outbox, kept)),
                // The tour gives the upgrade's answers itself.
                StorageCommand::OpenDownloads
                | StorageCommand::OpenInstitute
                | StorageCommand::Wipe { .. }
                | StorageCommand::Rename { .. }
                | StorageCommand::Hash { .. }
                | StorageCommand::ZeroFree { .. }
                | StorageCommand::MakeEmpty { .. }
                | StorageCommand::BootRead
                | StorageCommand::BootWrite { .. }
                | StorageCommand::BootForget => {}
            }
        }
    }

    /// A fresh process on the same display, as after a lock.
    pub(crate) fn restart(&mut self) {
        // What the lock asked to keep, before this process is gone.
        while let Some(c) = self.app.poll_storage() {
            if let StorageCommand::SaveBoxes {
                inbox,
                outbox,
                kept,
            } = c
            {
                self.boxes = Some((inbox, outbox, kept));
            }
        }
        let size = self.size();
        let medium = self.app.medium;
        self.app = Faraday::new();
        self.app.medium = medium;
        // A lock keeps the mode, and the tours' wallets are on the test
        // network.
        let _ = faraday_core::testkit::move_to(&mut self.app.session, faraday_core::testkit::net());
        if medium == Medium::Stick {
            self.app.devices = laptop();
        }
        self.app.event(display(size, self.dpi));
    }

    pub(crate) fn size(&mut self) -> (u16, u16) {
        let f = self.app.frame();
        (f.width, f.height)
    }

    /// A tick after the frame on screen: the vault work it says is
    /// under way runs.
    pub(crate) fn tick(&mut self) {
        let _ = self.app.frame();
        self.now += 1000;
        self.app.event(Event::Tick { now_ms: self.now });
        self.pump();
    }

    pub(crate) fn press(&mut self, a: Action) {
        use faraday_core::board::BoardPress as B;
        // The tours that edit a board press Customize backup first
        // (`docs/BACKUP-FLOW.md` §17.6), and keep a drop the sheet asks
        // about (§19.5); the std tour shows both itself.
        if let Action::Board(p) = a
            && !matches!(p, B::Customize | B::Software | B::SeesKeep | B::SeesUndo)
            && self.app.board_plan().is_some_and(|(_, plan)| !plan.custom)
        {
            self.app.press(Action::Board(B::Customize));
        }
        self.app.press(a);
        if matches!(a, Action::Board(_)) && self.app.sheet == Some(faraday_core::Sheet::BoardSees) {
            self.app.press(Action::Board(B::SeesKeep));
        }
        self.pump();
        if self.go_on
            && self.app.sheet == Some(faraday_core::Sheet::Devices)
            && !self.app.device_reminders().is_empty()
        {
            self.app.press(Action::DevicesUnderstood);
            self.pump();
        }
    }

    /// Copies every test wallet's descriptor file in from the stick, as a
    /// stick visit does. Nothing is copied while a key is held.
    pub(crate) fn copy_wallets(&mut self) {
        if self.app.holds_secret() {
            return;
        }
        let attached = !self.sticks.is_empty();
        if !attached {
            self.sticks(true);
        }
        let names: Vec<String> = self.stick().files.iter().map(|(n, _)| n.clone()).collect();
        // Everything comes in ticked: Unselect all, then these alone.
        self.press(Action::VisitInAll);
        for (i, n) in names.iter().enumerate() {
            if n.ends_with("-wallet.txt") && !self.app.inbox().iter().any(|it| &it.name == n) {
                self.press(Action::VisitIn(i));
            }
        }
        self.press(Action::VisitCopy);
        if !attached {
            self.sticks(false);
        }
    }

    /// Types test key `n`'s words into Add a key, which is open, and adds
    /// the key.
    pub(crate) fn type_key(&mut self, n: usize) {
        type_text(self, &testkit::test_words(testkit::TEST_SEEDS[n].0));
        self.press(Action::EntryAdd);
    }

    /// Every test wallet and test key 1, the way a person loads them: the
    /// descriptor files from the stick, opened one by one in the kit's
    /// order, and the words typed.
    pub(crate) fn load_kit(&mut self) -> Result<(), String> {
        self.copy_wallets();
        let mut ids: Vec<String> = testkit::kits().iter().map(|k| k.id.to_string()).collect();
        ids.push("musig".into());
        ids.push("threshold".into());
        for id in ids {
            let name = format!("{id}-wallet.txt");
            let i = self
                .app
                .inbox()
                .iter()
                .position(|it| it.name == name)
                .ok_or(format!("{name} is not in Files"))?;
            self.press(Action::LoadWallet(i));
        }
        let keys = &self.app.session.keys;
        if !(0..keys.len()).any(|i| testkit::has_test_words(keys, i, 0)) {
            self.press(Action::Entry(None));
            self.type_key(0);
        }
        faraday_core::testkit::select_wallet(&mut self.app, 0);
        self.press(Action::Nav(Screen::WalletsStart));
        Ok(())
    }

    pub(crate) fn sticks(&mut self, on: bool) {
        self.sticks = if on { vec![self.stick()] } else { Vec::new() };
        self.app.storage(StorageEvent::Sticks(self.sticks.clone()));
        self.pump();
    }

    /// Costs a QR search (through `decode_luma`). The codes a camera would
    /// read off the frame now.
    pub(crate) fn decode(&mut self, c: &Compute) -> Vec<String> {
        let f = self.app.frame();
        let luma: Vec<u8> = f
            .rgba
            .chunks_exact(4)
            .map(|p| ((u32::from(p[0]) * 3 + u32::from(p[1]) * 6 + u32::from(p[2])) / 10) as u8)
            .collect();
        faraday_codec::decode::decode_luma(c, usize::from(f.width), usize::from(f.height), &luma)
            .into_iter()
            .map(|d| String::from_utf8_lossy(&d.bytes).into_owned())
            .collect()
    }

    pub(crate) fn shot(&mut self, name: &str) -> Result<(), String> {
        self.shot_after(name, 4000)
    }

    /// [`Tour::shot`] with `ms` passing first: none, for a toast that
    /// has just come up.
    pub(crate) fn shot_after(&mut self, name: &str, ms: u64) -> Result<(), String> {
        self.pump();
        self.now += ms;
        self.app.event(Event::Tick { now_ms: self.now });
        // A frame that asked for another (a scroll to the open step) is
        // drawn twice, as a shell would.
        let _ = self.app.frame();
        self.app.event(Event::Tick { now_ms: self.now });
        // What is still moving (a glide, a card opening, the scrollbar)
        // comes to rest, so the screen is taken as it ends up.
        self.app.settle();
        self.n += 1;
        let path = self.out.join(format!("{:02}-{name}.png", self.n));
        let frame = self.app.frame();
        let file = std::fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut enc = png::Encoder::new(
            std::io::BufWriter::new(file),
            u32::from(frame.width),
            u32::from(frame.height),
        );
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().map_err(|e| e.to_string())?;
        w.write_image_data(frame.rgba).map_err(|e| e.to_string())?;
        println!("{}", path.display());
        Ok(())
    }
}

/// The stick on a laptop, as its shell reads the machine at start: no
/// virtual machine, a panel of its own.
pub(crate) fn laptop() -> faraday_core::devices::DeviceCheck {
    faraday_core::devices::DeviceCheck::machine("flags\t: fpu vme", "LENOVO", "20XW", "10")
}

pub(crate) fn display((width, height): (u16, u16), dpi: u16) -> Event {
    Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    })
}

/// Test key 1's fingerprint, among the keys loaded.
pub(crate) fn test_key_1(t: &Tour) -> Result<[u8; 4], String> {
    let keys = &t.app.session.keys;
    (0..keys.len())
        .find(|&i| testkit::has_test_words(keys, i, 0))
        .map(|i| keys[i].fingerprint.0)
        .ok_or_else(|| "test key 1 is not loaded".to_string())
}

/// Costs what `parse` costs. The SeedQR digits of the seed the backup
/// shows, as a camera reads them off the copy; with `wrong`, that
/// word's number one off.
pub(crate) fn copy_digits(c: &Compute, t: &Tour, wrong: Option<usize>) -> Result<Vec<u8>, String> {
    let b = t.app.backup().ok_or("no backup")?;
    let key = t.app.session.keys.info(b.key).ok_or("no key")?;
    let words = t
        .app
        .session
        .keys
        .reveal(b.key, faraday_keys::Reveal::SeedQr);
    if words.is_empty() {
        return Err("no words".to_string());
    }
    let m =
        faraday_bip::bip39::Mnemonic::parse(c, key.language, &words).map_err(|e| format!("{e}"))?;
    let mut digits = faraday_codec::seedqr::to_digits(&m)
        .expose()
        .as_bytes()
        .to_vec();
    if let Some(k) = wrong {
        let d = &mut digits[k * 4 + 3];
        *d = if *d == b'9' { b'8' } else { *d + 1 };
    }
    Ok(digits)
}

/// The open copy item's seed typed back as its words, a space after
/// each, `count` words of it (all when `None`).
pub(crate) fn type_words(t: &mut Tour, count: Option<usize>) -> Result<(), String> {
    let b = t.app.backup().ok_or("no backup")?;
    let key = t.app.session.keys.info(b.key).ok_or("no key")?;
    if !key.has_words() {
        return Err("no words".to_string());
    }
    let words = t
        .app
        .session
        .keys
        .reveal(b.key, faraday_keys::Reveal::CopyByHand)
        .to_string();
    for w in words.split_whitespace().take(count.unwrap_or(usize::MAX)) {
        for c in w.chars().chain([' ']) {
            t.app.event(Event::Key(Key::Char(c)));
        }
    }
    Ok(())
}

/// Costs a map of the plan (through `map`). The plan the board on
/// screen edits, its map.
pub(crate) fn bmap(c: &Compute, t: &Tour) -> Vec<Spot> {
    t.app
        .board_plan()
        .map(|(shape, a)| fplan::map(c, &shape, &a))
        .unwrap_or_default()
}

/// The wallet's shape, as the board on screen has it.
pub(crate) fn bshape(t: &Tour) -> Option<fplan::Shape> {
    t.app.board_plan().map(|(shape, _)| shape)
}

/// Tap to place on the board: `th` pressed in the list, then zone `z`.
pub(crate) fn bput(t: &mut Tour, th: Thing, z: Zone) {
    t.press(Action::Board(BP::Chip(Chip::List(th))));
    t.press(Action::Board(BP::Zone(z)));
}

/// The × on the placed chip of `th` at `z`.
pub(crate) fn boff(t: &mut Tour, z: Zone, th: Thing) {
    t.press(Action::Board(BP::Off(z, th)));
}

/// Costs a map of the plan (through `bmap`). The stick at place `p`
/// that carries vault `v`.
pub(crate) fn vault_stick(c: &Compute, t: &Tour, p: usize, v: usize) -> Option<usize> {
    let m = bmap(c, t);
    (0..fplan::sticks_at(&m, p)).find(|&s| fplan::stick_vault(&m, p, s) == Some(v))
}

/// Costs a map of the plan (through `vault_stick`). The first place
/// that keeps vault `v`'s stick.
pub(crate) fn vault_place(c: &Compute, t: &Tour, v: usize) -> Option<usize> {
    let places = t.app.board_plan().map_or(0, |(_, a)| a.places);
    (0..places).find(|&p| vault_stick(c, t, p, v).is_some())
}

/// Costs a map of the plan (through `vault_stick` and `bmap`). Vault
/// `v` kept at place `p`, holding exactly `seeds`, on the board.
pub(crate) fn keep_vault(c: &Compute, t: &mut Tour, p: usize, v: usize, seeds: &[usize]) {
    if vault_stick(c, t, p, v).is_none() {
        bput(t, Thing::Vault(v), Zone::Card(p));
    }
    let Some(s) = vault_stick(c, t, p, v) else {
        return;
    };
    let n = bshape(t).map_or(0, |sh| sh.seeds.len());
    for i in 0..n {
        let held = fplan::holds_at(&bmap(c, t), At::Vault(v));
        let has = held.contains(&What::Seed(i)) || held.contains(&What::SeedPassphrase(i));
        if has != seeds.contains(&i) {
            if has {
                boff(t, Zone::Vault(p, s, v), Thing::Seed(i));
            } else {
                bput(t, Thing::Seed(i), Zone::Vault(p, s, v));
            }
        }
    }
}

/// Costs a map of the plan (through `bmap`). Where `th` dropped on
/// place `p`'s stick of files goes: its first stick with no vault, or
/// **+ stick**.
pub(crate) fn files_zone(c: &Compute, t: &Tour, p: usize) -> Zone {
    let m = bmap(c, t);
    (0..fplan::sticks_at(&m, p))
        .find(|&s| fplan::stick_vault(&m, p, s).is_none())
        .map_or(Zone::NewStick(p), |s| Zone::Files(p, s))
}

/// Costs a map of the plan (through `files_zone`). The whole
/// description as a file on place `p`'s stick of files.
pub(crate) fn description_file_at(c: &Compute, t: &mut Tour, p: usize) {
    let z = files_zone(c, t, p);
    bput(t, Thing::Whole, z);
}

/// Costs a map of the plan (through `files_zone`). Every seed here as a
/// words file at place `p`.
pub(crate) fn seed_files_at(c: &Compute, t: &mut Tour, p: usize) {
    for i in bshape(t).map(|s| s.here()).unwrap_or_default() {
        let z = files_zone(c, t, p);
        bput(t, Thing::Seed(i), z);
    }
}

/// Costs a map of the plan (through `bmap`). The description in shares
/// on paper, each place with its own share in place of the whole sheet.
pub(crate) fn shares_on_paper(c: &Compute, t: &mut Tour) {
    let Some((shape, a)) = t.app.board_plan() else {
        return;
    };
    for p in 0..a.places {
        if fplan::holds_at(&bmap(c, t), At::Place(p)).contains(&What::Sheet) {
            boff(t, Zone::Paper(p), Thing::Whole);
        }
        bput(t, Thing::Share(p % shape.keys.max(1)), Zone::Paper(p));
    }
}

/// The backup's plan made into the plan `preset` once placed, with
/// `ticks` ticked on its Software card (`BAnswer`'s list and row), then
/// its checklist.
pub(crate) fn to_checklist(t: &mut Tour, preset: faraday_core::plan::Preset, ticks: &[(u8, u8)]) {
    t.backup_as(preset);
    for &(list, row) in ticks {
        t.press(Action::BAnswer(list, row));
    }
    t.press(Action::BChecklist);
}

/// Scrolls the page until `action` is on screen, then a little further.
pub(crate) fn scroll_to(t: &mut Tour, action: Action) {
    for _ in 0..30 {
        t.app.settle();
        let _ = t.app.frame();
        let (_, h) = t.size();
        if t.app
            .where_offered(action)
            .is_some_and(|(_, y)| y < h * 2 / 3)
        {
            return;
        }
        scroll(t, 120);
    }
}

/// A new key for a Create slot from the device's generator, as fast as
/// the flow allows.
pub(crate) fn device_key(t: &mut Tour, slot: u8) {
    let device = faraday_core::keygen::Way::Device.index();
    t.press(Action::KeyGen(Some(slot)));
    t.press(Action::KWords(24));
    t.press(Action::KWay(device));
    t.press(Action::KNext);
    t.press(Action::KNext);
    t.press(Action::KLock);
    t.press(Action::KNext);
}

/// The page scrolled down by `dy`, as a wheel would.
pub(crate) fn scroll(t: &mut Tour, dy: i16) {
    let _ = t.app.frame();
    let (w, h) = t.size();
    t.app.event(Event::Scroll {
        x: w / 2,
        y: h / 2,
        dy,
    });
}

pub(crate) fn type_text(t: &mut Tour, text: &str) {
    for c in text.chars() {
        t.app.event(Event::Key(Key::Char(c)));
    }
    t.pump();
}

/// Costs what `with_plan` costs. A flow standing on Make over the vault
/// open now, as a person has it after the sticks and the vaults: the
/// tours that are not about the flow start from here.
pub(crate) fn flow_at_make(c: &Compute, t: &mut Tour, kind: faraday_core::backup_flow::Kind) {
    use faraday_core::backup_flow::steps::FlowAction as F;
    use faraday_core::backup_flow::{BackupFlow, Plan, Step};
    let (salt, name) = {
        let o = &t.app.vaults().open[t.app.vaults().current];
        (o.header().salt, o.name.clone())
    };
    let mut plan = Plan::key_default();
    plan.vaults[0].salt = Some(salt);
    plan.vaults[0].file = Some(name);
    plan.vaults[0].existing = true;
    let mut f = BackupFlow::with_plan(c, kind, plan);
    f.step = Step::Make;
    let i = faraday_core::testkit::flow_push(&mut t.app, f);
    t.press(Action::Flow(F::Show(i)));
}

/// The flow's Make pressed, and the key it made released as a key a
/// vault unlocked from a stick holds is; the vault's contents shown on
/// its kind.
pub(crate) fn flow_made(t: &mut Tour, kind: faraday_core::backup_flow::Kind) -> Result<(), String> {
    use faraday_core::backup_flow::steps::FlowAction as F;
    t.press(Action::Flow(F::Make));
    t.tick();
    t.tick();
    if t.app.screen == Screen::SetTime {
        t.press(Action::SetTimeKeep);
        t.press(Action::Flow(F::Make));
        t.tick();
        t.tick();
    }
    if !t.app.backup_flows().last().is_some_and(|f| f.made) {
        return Err("the flow made no key".into());
    }
    for f in faraday_core::testkit::flows_mut(&mut t.app) {
        for s in &mut f.slots {
            s.written = Some(faraday_core::backup_flow::Written {
                stick: String::new(),
                serial: None,
                label: "STICK".into(),
                time: None,
                files: Vec::new(),
            });
        }
    }
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::Category(kind.category())));
    Ok(())
}

/// Costs a key derivation. `kind`'s key text of session key `i` of `keys`
/// (`[fingerprint/path]xpub`): the public text a wallet over this key is
/// written with.
pub(crate) fn key_text_of(
    c: &Compute,
    keys: &faraday_keys::Keys,
    i: usize,
    kind: faraday_core::create::NewKind,
) -> Result<String, String> {
    use faraday_core::wallet::KeyTexts;
    keys.account_text(c, i, kind, 0)
}

/// The Load sheet an unlock offers (F62), put off with Not now: the tours
/// that look at the vault's contents, not at what it loads.
pub(crate) fn not_now_load(t: &mut Tour) {
    if t.app.sheet == Some(faraday_core::Sheet::UnlockLoad) {
        t.press(Action::Cancel);
    }
}
