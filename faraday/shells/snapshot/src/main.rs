//! Renders Faraday's screens to PNG along one scripted tour.
//!
//! ```text
//! faraday-snapshot [--sd-card] WxH[@DPI] TESTKIT_DIR OUT_DIR [spend|themes|compact|seeds|keys|visit|copy|scan|public|seedfile|vaultway|kept|again|plan|board|inbox|transfer|upgrade|glance|chartedit|fromvault|first|restore|osk|flow|bootplace|passseeds|power|pills]
//! ```
//!
//! `--sd-card` runs the tour as the Pi's stick shell starts the app:
//! with the removable medium an SD card (`Medium::SdCard`), so every
//! screen names it as the Pi does.
//!
//! `TESTKIT_DIR` is what `faraday-testkit` wrote; its files stand in
//! for a stick called TESTSTICK. Each step of the tour is one numbered
//! PNG in `OUT_DIR`. With `spend`, only the Spend tab's tour runs; with
//! `devices`, the stick's device check and its reminders; with
//! `themes`, three screens in each theme; with `keys`, Wallets with keys
//! in no wallet; with `visit`, Stick visit with a backup's 20 files in the
//! Outbox, each list scrolled to its end; with `copy`, the backup's seeds
//! step checking the copy, without a camera and with one; with `scan`,
//! Home's Scan pressed with a stick in, then the camera once it is pulled;
//! with `public`, the public files offered as a code and a picture (a
//! signed message, Silent payments, Create's keys and Public files card,
//! the QR sheet, the backup's public files item, a GPG key in a vault), and the
//! pictures put in the Outbox written beside the screens as `outbox-*`;
//! with `seedfile`, the backup's checklist offering the seed into a
//! vault or as a file, with no vault open and with the test vault, the
//! secret sheet for the seed, and its SeedQR pictures, Standard and
//! Compact, written beside the screens as `outbox-*`; with `vaultway`,
//! Tools' GPG key with no vault, Create a vault made for it, the tile
//! with the test vault locked, Unlock for it and the GPG keys it comes
//! back to; with `kept`, the backup's copy item and its map
//! for Savings (2-of-3, one seed here, put out as a file, no vault) and
//! for Spending (one key, in the open test vault, its copy checked); with
//! `again`, a wallet loaded again: Savings with no seed here, opening its
//! backup on the presets and its public files, then a one-key wallet
//! restored from test key 2, its checklist saving it into the test vault,
//! and the wallet chip's list of the two; with `plan`, the backup's plan
//! by place (`docs/BACKUP-FLOW.md` §17): the presets, the places, each
//! place's card, a vault's sheet, where the description goes and The
//! plan for a one-key wallet; for Savings (2-of-3) the description whole
//! and in shares, the page What is a share? opens, the checklist part
//! done, the blank template written beside the screens as `outbox-*`; a
//! 3-of-5's shares on the description card and the checklist; and a
//! vault's sheet with two seeds; with `board`, the backup board
//! (`docs/BACKUP-FLOW.md` §19) for Savings with its three keys here: on
//! Paper and vault, a seed selected with the zones that take it edged,
//! a placed seed's forms open, a share placed with the keys left off,
//! and the software place; with `inbox`, Stick visit offering the Inbox's
//! files under "From the Inbox" (a wallet, a vault, a wallet's QR
//! picture, seed words and text), Select all, and the secret sheet for
//! the text and for the words; with `transfer`, the desktop app's
//! Transfer over a Downloads folder made in `OUT_DIR/downloads` (a PSBT,
//! a descriptor, a 100 KB file and one over 256 KiB): the list, a PSBT
//! sent, the larger file refused, and Receive saving a descriptor and a
//! seed's words sent from a device; with `upgrade`, Upgrade a Faraday
//! stick from Settings, the lock it asks for with a key loaded, and each
//! step with the boot copier's answers given by hand: the source read, an
//! older stick, a newer one beside it, one too small, one pulled during
//! the write, and one written; with `glance`, the Wallets card's chart
//! at a glance (`docs/NEW-WALLET.md` §6): a one-key wallet with no plan,
//! then with Paper and vault, and a 2-of-3 with two keys here, the second
//! with a passphrase, and a cosigner, with Paper and vault and the
//! passphrase in the vault too; on a small panel the card's At a glance
//! row and the chart's own page; then an open vault holding the one-key
//! wallet with its plan, its chart from the backup up (§7.2); with
//! `chartedit`, the chart's changes (§9.5–§9.7): a move and a copy
//! destroyed, each asked with the check before and after, the chart once
//! moved, a line marked lost and a place marked exposed with what
//! follows, a vault copied to another stick, and a cosigner's holder
//! named; with `restore`, Restore a wallet's cards (§12): Kind with the description
//! shortcut, Quorum, Seeds with a seed matched and one refused, Check and
//! Done; with `passseeds`, a vault keeping a 2-of-3's third seed without
//! the passphrase the wallet uses it with (F62): its Load sheet, Seeds
//! with no passphrase stored from the wallet's chart, a passphrase that
//! gives another key, and the right one; with `flow`, the backup flow (`docs/BACKUP-FLOW.md`) for a GPG key
//! kept in the test vault and a new vault at three places: Where, the
//! first stick with the vault choice, the place confirmed, two empty sticks
//! read alike and the hold, Vaults, Make, the Lock card, each stick
//! written and Done; with `vaultshow`, the test vault's key with its
//! SeedQR held (`docs/BACKUP-FLOW.md` §19.7), its Save to a stick, and a
//! wallet's Save to a stick.
//!
//! `@DPI` defaults to 160 (a desktop monitor); a real panel must give
//! its own, since the core picks `small`/`medium`/`wide` from physical
//! width, not pixels. The Pi's Waveshare panel is `480x640@286`; the
//! smallest panel the design supports is `240x320@143`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use faraday_core::board::{BoardPress as BP, Chip, Thing, Zone};
use faraday_core::file_menu::{FilePress, Row};
use faraday_core::plan::{self as fplan, At, Spot, What};
use faraday_core::testkit;
use faraday_core::vaults::VaultAction as V;
use faraday_core::{
    Action, BootPart, Faraday, Medium, Screen, StickInfo, StorageCommand, StorageEvent, bstep,
    qrow, qstep,
};
use osk_crypto::work::Compute;
use osk_shell_api::{App, BootState, Command, DisplayInfo, Event, Key, SecureHardware};

struct Tour {
    app: Faraday,
    out: PathBuf,
    kit: PathBuf,
    n: u32,
    now: u64,
    sticks: Vec<StickInfo>,
    /// Entropy requests answered, so each answer differs.
    answers: u8,
    dpi: u16,
    /// The online app's folders, for Transfer.
    host: Option<faraday_storage::Host>,
    /// What the app last asked to keep across a lock: the Inbox, the
    /// Outbox and the kept state.
    boxes: Option<(Files, Files, Files)>,
    /// The person goes on from the reminders sheet (F24) the first time a
    /// secret opens it, as every new process owes them again. Off while
    /// the tour shows that sheet itself.
    go_on: bool,
}

type Files = Vec<(String, Vec<u8>)>;

impl Tour {
    /// Back up's plan made into the plan preset `p` once placed, by the
    /// presses a person makes now (`testkit::backup_as`,
    /// `docs/BACKUP-FLOW.md` §19.21).
    fn backup_as(&mut self, p: faraday_core::plan::Preset) {
        testkit::backup_as(&mut self.app, p);
        self.pump();
    }

    fn stick(&self) -> StickInfo {
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
    fn pump(&mut self) {
        while let Some(c) = self.app.poll_command() {
            // Fixed bytes stand for the system's entropy, different for
            // each request as a generator's are: the tour is repeatable,
            // and nothing it makes holds value.
            if c == Command::RequestEntropy {
                let bytes = [0x5a ^ self.answers; 32];
                self.answers = self.answers.wrapping_add(1);
                self.app
                    .event(Event::Entropy(osk_shell_api::EntropyBytes::new(bytes)));
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
                    let ev = match faraday_storage::read_qr_png(&stick, &name) {
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
    fn restart(&mut self) {
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
        if medium == Medium::Stick {
            self.app.devices = laptop();
        }
        self.app.event(display(size, self.dpi));
    }

    fn size(&mut self) -> (u16, u16) {
        let f = self.app.frame();
        (f.width, f.height)
    }

    /// A tick after the frame on screen: the vault work it says is
    /// under way runs.
    fn tick(&mut self) {
        let _ = self.app.frame();
        self.now += 1000;
        self.app.event(Event::Tick { now_ms: self.now });
        self.pump();
    }

    fn press(&mut self, a: Action) {
        use faraday_core::board::BoardPress as B;
        // The tours that edit a board press Customize backup first
        // (`docs/BACKUP-FLOW.md` §19.6), and keep a drop the sheet asks
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
    fn copy_wallets(&mut self) {
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
            if n.ends_with("-wallet.txt") && !self.app.inbox.iter().any(|it| &it.name == n) {
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
    fn type_key(&mut self, n: usize) {
        type_text(self, &testkit::test_words(testkit::TEST_SEEDS[n].0));
        self.press(Action::EntryAdd);
    }

    /// Every test wallet and test key 1, the way a person loads them: the
    /// descriptor files from the stick, opened one by one in the kit's
    /// order, and the words typed.
    fn load_kit(&mut self) -> Result<(), String> {
        self.copy_wallets();
        let mut ids: Vec<String> = testkit::kits().iter().map(|k| k.id.to_string()).collect();
        ids.push("musig".into());
        ids.push("threshold".into());
        for id in ids {
            let name = format!("{id}-wallet.txt");
            let i = self
                .app
                .inbox
                .iter()
                .position(|it| it.name == name)
                .ok_or(format!("{name} is not in Files"))?;
            self.press(Action::LoadWallet(i));
        }
        let bacon = testkit::TEST_SEEDS[0].0;
        if !self.app.session.keys.iter().any(|k| {
            k.words
                .as_deref()
                .is_some_and(|w| w.as_str() == testkit::test_words(bacon))
        }) {
            self.press(Action::Entry(None));
            self.type_key(0);
        }
        self.app.wallet = 0;
        self.press(Action::Nav(Screen::Start));
        Ok(())
    }

    fn sticks(&mut self, on: bool) {
        self.sticks = if on { vec![self.stick()] } else { Vec::new() };
        self.app.storage(StorageEvent::Sticks(self.sticks.clone()));
        self.pump();
    }

    /// The codes a camera would read off the frame now.
    fn decode(&mut self) -> Vec<String> {
        let f = self.app.frame();
        let luma: Vec<u8> = f
            .rgba
            .chunks_exact(4)
            .map(|p| ((u32::from(p[0]) * 3 + u32::from(p[1]) * 6 + u32::from(p[2])) / 10) as u8)
            .collect();
        osk_codec::decode::decode_luma(usize::from(f.width), usize::from(f.height), &luma)
            .into_iter()
            .map(|d| String::from_utf8_lossy(&d.bytes).into_owned())
            .collect()
    }

    fn shot(&mut self, name: &str) -> Result<(), String> {
        self.shot_after(name, 4000)
    }

    /// [`Tour::shot`] with `ms` passing first: none, for a toast that
    /// has just come up.
    fn shot_after(&mut self, name: &str, ms: u64) -> Result<(), String> {
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
fn laptop() -> faraday_core::devices::DeviceCheck {
    faraday_core::devices::DeviceCheck::machine("flags\t: fpu vme", "LENOVO", "20XW", "10")
}

/// A wireless receiver: Logitech's Unifying receiver, or with `bolt` its
/// Bolt receiver.
fn receiver(bolt: bool) -> faraday_core::devices::UsbDevice {
    use faraday_core::devices::{Interface, UsbDevice};
    let hid = |protocol| Interface {
        class: 3,
        subclass: 1,
        protocol,
    };
    UsbDevice {
        port: "1-2".into(),
        vendor: 0x046d,
        product: if bolt { 0xc548 } else { 0xc52b },
        manufacturer: "Logitech".into(),
        name: "USB Receiver".into(),
        interfaces: vec![hid(1), hid(2)],
    }
}

/// The stick's device check (`docs/STARTER.md` §3.1a, F24): before the
/// first secret, the reminders with a wireless receiver attached, and
/// with a Bolt receiver; an IP-KVM named at start, and Continue anyway
/// asking; a virtual machine with an IP-KVM, which has no way on; a
/// desktop's reminder alone, and the same desktop whose firmware passed
/// a TV's EDID. Home empty ends it.
fn devices_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::devices::{DeviceCheck, Interface, UsbDevice};
    let hid = |protocol| Interface {
        class: 3,
        subclass: 1,
        protocol,
    };
    let kvm = UsbDevice {
        port: "1-1".into(),
        vendor: 0x1d6b,
        product: 0x0104,
        manufacturer: "PiKVM".into(),
        name: "Composite KVM Device".into(),
        interfaces: vec![
            hid(1),
            hid(2),
            Interface {
                class: 8,
                subclass: 6,
                protocol: 0x50,
            },
        ],
    };
    t.go_on = false;
    t.app.devices = laptop();
    t.app.usb_devices(&[receiver(false)]);
    t.press(Action::Entry(None));
    t.shot("devices-receiver")?;
    t.press(Action::Cancel);
    t.app.usb_devices(&[receiver(true)]);
    t.press(Action::Entry(None));
    t.shot("devices-receiver-bolt")?;
    t.press(Action::Cancel);
    t.app.usb_devices(std::slice::from_ref(&kvm));
    t.pump();
    t.shot("devices-kvm")?;
    t.press(Action::DevicesContinue);
    t.shot("devices-continue-ask")?;
    t.app.usb_devices(&[]);
    t.press(Action::DevicesCheck);
    t.app.devices = DeviceCheck::machine(
        "flags\t: fpu vme hypervisor",
        "QEMU",
        "Standard PC (Q35 + ICH9, 2009)",
        "1",
    );
    t.app.usb_devices(&[kvm]);
    t.pump();
    t.shot("devices-virtual-machine")?;
    t.press(Action::Cancel);
    t.app.devices = DeviceCheck::machine("flags\t: fpu vme", "Dell Inc.", "OptiPlex 7090", "3");
    t.app.usb_devices(&[]);
    t.press(Action::Entry(None));
    t.shot("devices-before")?;
    t.press(Action::Cancel);
    // The same desktop whose firmware passed a TV's EDID: named, not asked.
    let mut edid = [0u8; 128];
    edid[..8].copy_from_slice(&[0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0]);
    edid[8..10].copy_from_slice(&[0x4c, 0x2d]);
    edid[54..59].copy_from_slice(&[0, 0, 0, 0xfc, 0]);
    edid[59..72].copy_from_slice(b"EXAMPLE TV\n  ");
    t.app.devices = DeviceCheck::machine("flags\t: fpu vme", "Dell Inc.", "OptiPlex 7090", "3")
        .with_edid(Some(&edid));
    t.app.usb_devices(&[]);
    t.pump();
    t.shot("devices-tv-edid")?;
    t.press(Action::Cancel);
    t.app.devices = laptop();
    t.app.usb_devices(&[]);
    t.pump();
    t.go_on = true;
    t.shot("home-empty")
}

fn display((width, height): (u16, u16), dpi: u16) -> Event {
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

/// The wide layout's on-screen keyboard: a receiver's line on Home, the
/// keyboard brought up from its button on a passphrase entry, and on
/// Tools.
/// The power sheet on a PC stick that started with Secure Boot on, and
/// with it off (the reminder and this computer's BIOS key).
fn power_tour(t: &mut Tour) -> Result<(), String> {
    t.app.secure_boot = faraday_core::SecureBootAtStart::On;
    t.press(Action::PowerAsk);
    t.shot("power-sheet")?;
    t.app.event(Event::Key(Key::Escape));
    t.app.secure_boot = faraday_core::SecureBootAtStart::Off;
    t.app.maker = "LENOVO".into();
    t.app.maker_family = "ThinkPad X1 Carbon".into();
    t.app.maker_model = "Example 14".into();
    t.press(Action::PowerAsk);
    t.shot("power-sheet-secure-boot-off")
}

/// The desktop app's stick pills: the default two, then a wallet's backup
/// with five vaults.
fn pills_tour(t: &mut Tour) -> Result<(), String> {
    t.app.shell_sticks = true;
    t.app.online = true;
    let stick = |name: &str| StickInfo {
        id: name.into(),
        label: name.into(),
        boot: name == "BOOTSTICK",
        serial: None,
        files: Vec::new(),
    };
    t.app.storage(StorageEvent::Sticks(vec![
        stick("BOOTSTICK"),
        stick("STICK-2"),
    ]));
    t.press(Action::Nav(Screen::Home));
    t.shot("pills-default")?;
    let multi = faraday_core::create::NewKind::ALL
        .iter()
        .position(|k| *k == faraday_core::create::NewKind::Multi)
        .unwrap_or(0) as u8;
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    t.press(Action::CVaults(true));
    t.press(Action::CNext(faraday_core::cstep::WHERE));
    t.press(Action::CVaultN(5));
    t.press(Action::CNext(faraday_core::cstep::HOW_MANY));
    t.shot("pills-five-vaults")
}

/// A released Faraday Desktop's first screen: testnet, behind the online
/// warning (F61), then Home once it is acknowledged.
fn start_warning_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    t.app.online = true;
    t.app.shell_sticks = true;
    t.app.session = faraday_core::wallet::Session::default();
    t.app.session.set_demo(c);
    t.app
        .session
        .set_network(testkit::NET)
        .map_err(|e| e.text())?;
    t.app.with_start_warning();
    t.shot("desktop-start-warning")?;
    t.press(Action::AirgapUnderstood);
    t.shot("desktop-start-home")
}

/// The demo's dimmed actions (`docs/DEMO.md` §7): BIP-85's Value card
/// and the Lightning node key, on a public seed.
fn demo_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    t.app.online = true;
    t.app.session = faraday_core::wallet::Session::default();
    t.app.session.set_demo(c);
    t.press(Action::Entry(None));
    type_text(t, &faraday_core::demo::words("bacon"));
    t.press(Action::EntryAdd);
    let fp = t
        .app
        .session
        .keys
        .first()
        .ok_or("bacon was not loaded")?
        .master()
        .fingerprint()
        .0;
    t.press(Action::Bip85);
    t.press(Action::PKey(fp));
    t.press(Action::PNext);
    t.press(Action::PNext);
    t.shot("demo-bip85-value")?;
    t.press(Action::Lightning);
    t.press(Action::LKey(fp));
    t.shot("demo-lightning")
}

fn osk_tour(t: &mut Tour) -> Result<(), String> {
    t.app.devices = laptop();
    t.app.usb_devices(&[receiver(false)]);
    t.pump();
    t.shot("osk-home-receiver")?;
    t.press(Action::Entry(None));
    type_text(t, &testkit::test_words(testkit::TEST_SEEDS[2].0));
    t.press(Action::EntryPassphrase);
    type_text(t, "TREZOR");
    t.shot("osk-passphrase-button")?;
    t.press(Action::Osk(faraday_core::OskPress::Show));
    for c in ['a', 'b'] {
        t.press(Action::Osk(faraday_core::OskPress::Char(c)));
    }
    t.shot("osk-passphrase-keyboard")?;
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Osk(faraday_core::OskPress::Show));
    t.shot("osk-tools-keyboard")
}

fn run(
    c: &Compute,
    size: (u16, u16),
    dpi: u16,
    kit: &Path,
    out: &Path,
    only: Option<&str>,
    medium: Medium,
) -> Result<(), String> {
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let mut app = Faraday::new();
    app.medium = medium;
    app.event(display(size, dpi));
    // `FARADAY_SNAP_THEME=<id>` takes the tour in that theme ("light",
    // "dark" …) instead of the first start's.
    if let Ok(id) = std::env::var("FARADAY_SNAP_THEME")
        && let Some(theme) = faraday_core::ui::Theme::from_id(&id)
    {
        app.press(Action::Theme(theme));
    }
    let mut t = Tour {
        app,
        out: out.to_path_buf(),
        kit: kit.to_path_buf(),
        n: 0,
        now: 0,
        sticks: Vec::new(),
        answers: 0,
        dpi,
        host: None,
        boxes: None,
        go_on: true,
    };
    match only {
        Some("spend") => return spend_tour(&mut t),
        Some("devices") => return devices_tour(&mut t),
        Some("themes") => return themes_tour(&mut t),
        Some("compact") => return compact_tour(&mut t),
        Some("seeds") => return seeds_tour(&mut t),
        Some("keys") => return keys_tour(&mut t),
        Some("visit") => return visit_tour(c, &mut t),
        Some("copy") => return copy_tour(c, &mut t),
        Some("scan") => return scan_tour(&mut t),
        Some("public") => return public_tour(c, &mut t),
        Some("seedfile") => return seedfile_tour(c, &mut t),
        Some("vaultway") => return vaultway_tour(&mut t),
        Some("kept") => return kept_tour(c, &mut t),
        Some("again") => return again_tour(&mut t),
        Some("plan") => return plan_tour(c, &mut t),
        Some("board") => return board_tour(c, &mut t),
        Some("menu") => return menu_tour(&mut t),
        Some("inbox") => return inbox_tour(&mut t),
        Some("transfer") => return transfer_tour(c, &mut t),
        Some("upgrade") => return upgrade_tour(&mut t),
        Some("glance") => return glance_tour(c, &mut t),
        Some("chartedit") => return chart_edit_tour(c, &mut t),
        Some("fromvault") => return from_vault_tour(c, &mut t),
        Some("first") => return first_tour(c, &mut t),
        Some("restore") => return restore_tour(&mut t),
        Some("osk") => return osk_tour(&mut t),
        Some("flow") => return flow_tour(&mut t),
        Some("bootplace") => return boot_place_tour(&mut t),
        Some("passseeds") => return pass_seeds_tour(c, &mut t),
        Some("power") => return power_tour(&mut t),
        Some("pills") => return pills_tour(&mut t),
        Some("walletflow") => return wallet_flow_tour(c, &mut t),
        Some("demo") => return demo_tour(c, &mut t),
        Some("startwarn") => return start_warning_tour(c, &mut t),
        Some("vaultshow") => return vault_show_tour(&mut t),
        Some("addkey") => return add_key_tour(&mut t),
        Some("createfrom") => return create_from_tour(c, &mut t),
        Some("inprogress") => return in_progress_tour(&mut t),
        Some("cancel") => return cancel_tour(&mut t),
        Some("std") => return standard_tour(&mut t),
        Some("parts") => return partials_tour(&mut t),
        _ => {}
    }
    if medium == Medium::Stick {
        devices_tour(&mut t)?;
    } else {
        t.shot("home-empty")?;
    }
    // Settings' Time section, and Set the time with the clock not known
    // yet (`docs/TOTP.md` §9).
    t.press(Action::Nav(Screen::Settings));
    t.shot("settings-time")?;
    t.press(Action::OpenSetTime);
    t.shot("set-time")?;
    t.press(Action::SetTimeBack);
    t.press(Action::Nav(Screen::Home));
    // A stick arrives while nothing secret is held: the visit.
    t.sticks(true);
    // Every file Faraday reads comes in ticked but a seed's.
    t.shot("visit")?;
    // A file on the stick: its ⋮, Rename, Delete from the stick and
    // Details (`docs/FLOWS.md` decision 61 item 4).
    let k = t
        .stick()
        .files
        .iter()
        .position(|(n, _)| n.ends_with(".psbt"))
        .ok_or("the kit has no PSBT")?;
    t.press(Action::File(FilePress::Menu(Row::Stick(k))));
    t.shot("visit-file-menu")?;
    t.press(Action::Cancel);
    // Delete files: one sheet, chosen files, the free space or the whole
    // partition.
    t.press(Action::DeleteFiles);
    t.press(Action::DeletePick(1));
    t.shot("visit-delete-files")?;
    t.press(Action::DeleteWhat(1));
    t.shot("visit-delete-free")?;
    t.press(Action::DeleteWhat(2));
    t.press(Action::DeletePasses(2));
    t.shot("visit-delete-whole")?;
    t.press(Action::Cancel);
    t.press(Action::VisitInAll);
    for name in [
        "savings-unsigned.psbt",
        "taproot-multisig-unsigned.psbt",
        "taproot-multisig-cosigner-5d388376.psbt",
        "spending-unsigned.psbt",
        "savings-cosigner-5d388376.psbt",
        "inheritance-unsigned.psbt",
    ] {
        if let Some(i) = t.stick().files.iter().position(|(n, _)| n == name) {
            t.press(Action::VisitIn(i));
        }
    }
    t.shot("visit-chosen")?;
    t.press(Action::VisitCopy);
    t.shot("visit-copied")?;
    // A picture of a wallet's QR code is read off the stick into Files.
    let before = t.app.inbox.len();
    let png = t
        .stick()
        .files
        .iter()
        .position(|(n, _)| n == "nested-wallet-qr.png")
        .ok_or("the kit has no nested-wallet-qr.png")?;
    // Ticked like any file: its codes are read on Import.
    t.press(Action::VisitInAll);
    t.press(Action::VisitIn(png));
    t.press(Action::VisitCopy);
    // What its code holds, and the picture itself, kept to copy on.
    if t.app.inbox.len() != before + 2 {
        return Err("the QR code in nested-wallet-qr.png did not reach Files".into());
    }
    t.shot("visit-png-read")?;
    t.press(Action::Nav(Screen::Home));
    t.shot("home-stick-attached")?;
    // Add a key with the stick in: the sheet asks for it to be pulled,
    // and Add a key opens when it is.
    t.press(Action::Entry(None));
    t.shot("home-pull")?;
    t.sticks(false);
    t.shot("entry-after-pull")?;
    t.press(Action::Nav(Screen::Home));
    t.shot("home-files")?;
    t.press(Action::Nav(Screen::Files));
    t.shot("files")?;
    t.press(Action::Nav(Screen::Start));
    t.shot("start-before")?;
    t.load_kit()?;
    t.shot("start")?;
    t.press(Action::OpenWallet(0));
    t.shot("wallet-card")?;
    t.press(Action::Entry(Some([0x5d, 0x38, 0x83, 0x76])));
    for c in "zebra zebra zeb".chars() {
        t.app.event(Event::Key(if c == ' ' {
            Key::Char(' ')
        } else {
            Key::Char(c)
        }));
    }
    t.shot("entry-typing")?;
    // The other forms: one SLIP-39 share in, the next being typed.
    t.press(Action::EntryForm(1));
    type_text(
        &mut t,
        "shadow pistol academic always adequate wildlife fancy gross oasis cylinder mustang wrist rescue view short owner flip making coding armed",
    );
    t.app.event(Event::Key(Key::Enter));
    type_text(&mut t, "shadow pistol academic acid");
    t.shot("entry-slip39")?;
    t.press(Action::EntryForm(0));
    // Spanish words, typed without their accents.
    t.press(Action::EntryLanguage(1));
    type_text(
        &mut t,
        "abaco abdomen abeja abierto abogado abono aborto abrazo abrir abuelo abuso acabar",
    );
    t.shot("entry-spanish")?;
    t.press(Action::EntryClear);
    t.press(Action::EntryLanguage(0));
    t.press(Action::Nav(Screen::Wallets));
    let psbt = t
        .app
        .inbox
        .iter()
        .position(|i| i.name == "savings-unsigned.psbt")
        .ok_or("no psbt")?;
    t.press(Action::StartSpend(psbt));
    t.shot("spend-transaction")?;
    t.press(Action::ToggleTable);
    t.shot("spend-table")?;
    t.press(Action::StepNext(2));
    t.shot("spend-check")?;
    t.press(Action::StepNext(1));
    t.shot("spend-signers")?;
    t.press(Action::StepNext(4));
    t.shot("spend-sign")?;
    t.press(Action::SignHere);
    t.shot("spend-signed")?;
    let copy = t
        .app
        .inbox
        .iter()
        .position(|i| i.name == "savings-cosigner-5d388376.psbt")
        .ok_or("no copy")?;
    t.press(Action::Collect(copy));
    t.shot("spend-finished")?;
    // The end of signing: the nonce check, the raw hex, and the
    // transaction taken apart.
    t.press(Action::SpendHex);
    t.app.list_offset = 0.0;
    // Drawn before the scroll, as a shell draws after each press: how far
    // the page now goes is what the scroll stops at.
    let _ = t.app.frame();
    t.app.event(Event::Scroll {
        x: 700,
        y: 400,
        dy: 600,
    });
    t.shot("spend-nonce-check")?;
    t.press(Action::SpendHex);
    t.press(Action::DecodeFinished);
    t.shot("decode")?;
    let _ = t.app.frame();
    t.app.event(Event::Scroll {
        x: 700,
        y: 400,
        dy: 500,
    });
    t.shot("decode-outputs")?;
    // The Spend tab with wallets and seeds loaded: what is loaded and the
    // next step for each.
    t.press(Action::Nav(Screen::Family));
    t.shot("spend-loaded")?;
    t.press(Action::Nav(Screen::Spend));
    t.press(Action::QrSigned);
    t.shot("qr-signed-psbt")?;
    t.press(Action::Cancel);
    // Finish put both files For the stick; the primary opens Files.
    t.shot("spend-in-outbox")?;
    t.press(Action::Primary);
    t.shot("files-outbox")?;
    // A file For the stick: its ⋮ and the Rename popup (`docs/FLOWS.md`
    // decision 61).
    t.press(Action::File(FilePress::Menu(Row::Out(0))));
    t.shot("files-menu")?;
    t.press(Action::File(FilePress::Rename(Row::Out(0))));
    t.shot("files-rename")?;
    t.press(Action::Cancel);
    // A single-key spend: no signers or signatures steps.
    let single = t
        .app
        .inbox
        .iter()
        .position(|i| i.name == "spending-unsigned.psbt")
        .ok_or("no single")?;
    t.press(Action::StartSpend(single));
    t.shot("single-check")?;
    for n in [1u8, 2, 3] {
        t.press(Action::StepNext(n));
    }
    t.shot("single-sign")?;
    t.press(Action::SignHere);
    t.shot("single-finished")?;
    // A Taproot multisig spend, signed here and collected.
    let tap = t
        .app
        .inbox
        .iter()
        .position(|i| i.name == "taproot-multisig-unsigned.psbt")
        .ok_or("no tap")?;
    t.press(Action::StartSpend(tap));
    for n in [1u8, 2, 3, 4] {
        t.press(Action::StepNext(n));
    }
    t.press(Action::SignHere);
    t.shot("tapmulti-signed")?;
    let tcopy = t
        .app
        .inbox
        .iter()
        .position(|i| i.name == "taproot-multisig-cosigner-5d388376.psbt")
        .ok_or("no tap copy")?;
    t.press(Action::Collect(tcopy));
    t.shot("tapmulti-finished")?;
    // MuSig2, signing last: both nonces in, sign, both partials in.
    t.sticks(true);
    t.press(Action::Lock);
    t.app.storage(StorageEvent::Sticks(t.sticks.clone()));
    t.pump();
    let names: Vec<String> = t.stick().files.iter().map(|(n, _)| n.clone()).collect();
    t.press(Action::VisitInAll);
    for (i, n) in names.iter().enumerate() {
        if n.starts_with("musig-") && n.ends_with(".psbt") || n == "inheritance-unsigned.psbt" {
            t.press(Action::VisitIn(i));
        }
    }
    t.press(Action::VisitCopy);
    t.sticks(false);
    t.load_kit()?;
    let mu = t
        .app
        .inbox
        .iter()
        .position(|i| i.name == "musig-unsigned.psbt")
        .ok_or("no musig psbt")?;
    t.press(Action::StartSpend(mu));
    for n in [1u8, 2, 3, 4] {
        t.press(Action::StepNext(n));
    }
    for name in ["musig-nonce-5d388376.psbt", "musig-nonce-cf2e083d.psbt"] {
        let i = t
            .app
            .inbox
            .iter()
            .position(|it| it.name == name)
            .ok_or(format!("no {name}"))?;
        t.press(Action::Collect(i));
    }
    t.shot("musig-nonces")?;
    t.press(Action::StepNext(9));
    t.press(Action::SignHere);
    for name in ["musig-partial-5d388376.psbt", "musig-partial-cf2e083d.psbt"] {
        let i = t
            .app
            .inbox
            .iter()
            .position(|it| it.name == name)
            .ok_or(format!("no {name}"))?;
        t.press(Action::Collect(i));
    }
    t.shot("musig-finished")?;
    if t.app
        .spend
        .as_ref()
        .is_none_or(|s| s.spend.finished.is_none())
    {
        return Err(format!(
            "the MuSig2 spend did not finish: {:?}",
            t.app.spend.as_ref().and_then(|s| s.error.clone())
        ));
    }
    println!("MuSig2 spend finished in the app");
    // FROST 2 of 3, first location: test key 1 signs as share 1 and
    // chooses share 2, and the carry file goes to the Outbox.
    t.sticks(true);
    t.press(Action::Lock);
    t.app.storage(StorageEvent::Sticks(t.sticks.clone()));
    t.pump();
    if let Some(i) = t
        .stick()
        .files
        .iter()
        .position(|(n, _)| n == "threshold-unsigned.psbt")
    {
        t.press(Action::VisitInAll);
        t.press(Action::VisitIn(i));
        t.press(Action::VisitCopy);
    }
    t.sticks(false);
    t.load_kit()?;
    let fu = t
        .app
        .inbox
        .iter()
        .position(|i| i.name == "threshold-unsigned.psbt")
        .ok_or("no threshold psbt")?;
    t.press(Action::StartSpend(fu));
    for n in [1u8, 2, 3] {
        t.press(Action::StepNext(n));
    }
    t.shot("frost-signers")?;
    t.press(Action::TOther(1));
    t.shot("frost-chosen")?;
    t.press(Action::StepNext(4));
    t.press(Action::SignHere);
    t.shot("frost-carry")?;
    if t.app.spend.as_ref().is_none_or(|s| s.carry_out.is_none()) {
        return Err(format!(
            "share 1 left no carry file: {:?}",
            t.app.spend.as_ref().and_then(|s| s.error.clone())
        ));
    }
    // The carry holds a secret nonce: the sheet asks where it goes.
    t.press(Action::CarryToOutbox);
    t.shot("secret-out")?;
    if t.app
        .outbox
        .iter()
        .any(|i| i.name == "threshold-partly-signed.osk")
    {
        return Err("the carry reached the Outbox before it was allowed".into());
    }
    t.press(Action::SecretUnprotected);
    if t.app
        .outbox
        .iter()
        .any(|i| i.name == "threshold-partly-signed.osk")
    {
        return Err("the carry went out before the warning was acknowledged".into());
    }
    t.press(Action::SecretAck);
    t.press(Action::SecretUnprotected);
    t.press(Action::Nav(Screen::Files));
    t.shot("files-unprotected")?;
    t.press(Action::Nav(Screen::Spend));
    let carry = t
        .app
        .outbox
        .iter()
        .find(|i| i.name == "threshold-partly-signed.osk")
        .map(|i| (i.name.clone(), i.bytes.clone()))
        .ok_or("the carry file is not in the Outbox")?;
    // The second location: test key 2 alone, the record and the carry
    // file copied in.
    t.sticks(true);
    t.press(Action::Lock);
    t.pump();
    let record = std::fs::read(t.kit.join("threshold-wallet.txt")).map_err(|e| e.to_string())?;
    t.app.storage(StorageEvent::Restored {
        inbox: vec![("threshold-wallet.txt".to_string(), record), carry],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.sticks(false);
    t.press(Action::Entry(None));
    t.type_key(1);
    t.press(Action::LoadWallet(0));
    t.press(Action::Nav(Screen::Files));
    t.shot("frost-files")?;
    t.press(Action::StartSpend(1));
    for n in [1u8, 2, 3, 4] {
        t.press(Action::StepNext(n));
    }
    t.shot("frost-second")?;
    t.press(Action::SignHere);
    t.shot("frost-finished")?;
    if t.app.spend.as_ref().is_none_or(|s| !s.complete) {
        return Err(format!(
            "the FROST spend did not finish: {:?}",
            t.app.spend.as_ref().and_then(|s| s.error.clone())
        ));
    }
    println!("FROST spend signed at two places and finished in the app");
    // Restore Savings from two of its three shares, then type test key 1.
    t.sticks(true);
    t.press(Action::Lock);
    t.app.storage(StorageEvent::Sticks(t.sticks.clone()));
    t.pump();
    t.press(Action::VisitInAll);
    for name in ["savings-share-1-of-3.txt", "savings-share-2-of-3.txt"] {
        if let Some(i) = t.stick().files.iter().position(|(n, _)| n == name) {
            t.press(Action::VisitIn(i));
        }
    }
    t.press(Action::VisitCopy);
    // The descriptor files too, for the wallets loaded after the restore.
    t.copy_wallets();
    t.sticks(false);
    t.press(Action::RestoreWallet);
    for name in ["savings-share-1-of-3.txt", "savings-share-2-of-3.txt"] {
        let i = t
            .app
            .inbox
            .iter()
            .position(|it| it.name == name)
            .ok_or(format!("no {name}"))?;
        t.press(Action::RShare(i));
    }
    t.press(Action::RStep(faraday_core::rstep::DESCRIPTION));
    t.shot("restore-shares")?;
    t.press(Action::RRebuild);
    // The question of vaults, in its turn before the seeds.
    t.press(Action::RVaults(false));
    t.press(Action::RNext(faraday_core::rstep::WHERE));
    let fp = t
        .app
        .session
        .wallets
        .first()
        .and_then(|w| w.policy().keys()[0].fingerprint())
        .ok_or("no wallet")?;
    t.press(Action::Entry(Some(fp.0)));
    t.type_key(0);
    t.shot("restore-seeds")?;
    if t.app.screen != Screen::Restore {
        return Err("adding the key did not return to the restore".into());
    }
    // A SeedQR held up to the camera from Add a key: Test key 2's words.
    let keys_before = t.app.session.keys.len();
    t.press(Action::Entry(None));
    t.press(Action::ScanSeed);
    t.app.event(Event::CameraFrame {
        width: 640,
        height: 480,
        luma: (0..640u32 * 480).map(|i| ((i % 640) / 3) as u8).collect(),
        chroma: None,
    });
    t.shot("scan-seedqr")?;
    let zebra = osk_bip::bip39::Mnemonic::parse(
        c,
        osk_bip::bip39::Language::English,
        &testkit::test_words("zebra"),
    )
    .map_err(|e| format!("{e:?}"))?;
    let digits = osk_codec::seedqr::to_digits(&zebra);
    t.app.event(Event::Scanned {
        bytes: digits.expose().as_bytes().to_vec(),
    });
    t.pump();
    // The seed read asks how it loads: without a passphrase, or with one
    // typed twice, each with its key.
    if t.app.sheet != Some(faraday_core::Sheet::SeedRead) {
        return Err("the scanned SeedQR did not ask how it loads".into());
    }
    t.shot("scan-seedqr-read")?;
    t.press(Action::SeedRead(faraday_core::seed_read::Choice::AddPass));
    type_text(&mut t, "a passphrase");
    t.press(Action::SeedRead(faraday_core::seed_read::Choice::Field(1)));
    type_text(&mut t, "a passphrase");
    t.shot("scan-seedqr-passphrase")?;
    t.press(Action::SeedRead(faraday_core::seed_read::Choice::Plain));
    let zebra_fp = [0x5d, 0x38, 0x83, 0x76];
    if t.app.session.keys.len() != keys_before + 1
        || !t
            .app
            .session
            .keys
            .iter()
            .any(|k| k.master().fingerprint().0 == zebra_fp)
    {
        return Err("the scanned SeedQR did not add Test key 2".into());
    }
    t.shot("scan-seedqr-added")?;
    println!("SeedQR scanned into Add a key");
    t.load_kit()?;
    // The camera: a frame, then the Savings PSBT as animated UR parts.
    let savings = std::fs::read(t.kit.join("savings-unsigned.psbt")).map_err(|e| e.to_string())?;
    t.press(Action::Scan);
    let frame: Vec<u8> = (0..640u32 * 480)
        .map(|i| ((i % 640) / 3 + (i / 640) / 4) as u8)
        .collect();
    t.app.event(Event::CameraFrame {
        width: 640,
        height: 480,
        luma: frame,
        chroma: None,
    });
    let mut enc = osk_codec::ur::Encoder::psbt(&savings, 120).map_err(|e| format!("{e:?}"))?;
    let mut parts = 0;
    while t.app.scan.is_some() && parts < 40 {
        let part = enc.next_part();
        t.app.event(Event::Scanned {
            bytes: part.into_bytes(),
        });
        parts += 1;
        if parts == 2 {
            // Two cameras, and a code found where the scanner saw it.
            t.app.storage(StorageEvent::Cameras(vec![
                ("/dev/video0".into(), "Integrated Camera".into()),
                ("/dev/video2".into(), "USB Camera".into()),
            ]));
            t.app.storage(StorageEvent::QrSeen {
                width: 640,
                height: 480,
                corners: [(220, 120), (430, 130), (420, 340), (210, 330)],
                module_tenths: 60,
                read: true,
            });
            t.shot("scan-progress")?;
        }
    }
    t.pump();
    let got = t
        .app
        .inbox
        .iter()
        .find(|i| i.name.starts_with("scanned-"))
        .ok_or("nothing scanned")?;
    if got.bytes != savings {
        return Err("the scanned PSBT differs from the one shown".into());
    }
    println!("scanned the Savings PSBT in {parts} UR parts");
    // The Inheritance wallet: the Path step.
    let inh = t
        .app
        .inbox
        .iter()
        .position(|i| i.name == "inheritance-unsigned.psbt");
    if let Some(i) = inh {
        t.press(Action::StartSpend(i));
        t.press(Action::StepNext(1));
        t.shot("inheritance-path")?;
    }
    t.press(Action::Nav(Screen::Wallets));
    t.shot("wallets-list")?;
    // The backup of Savings: the presets, then paper only, each place
    // with its own share.
    t.press(Action::Backup(0));
    t.shot("backup-plan")?;
    t.backup_as(faraday_core::plan::Preset::Paper);
    shares_on_paper(c, &mut t);
    t.press(Action::BChecklist);
    t.press(Action::BStep(bstep::BLANK));
    t.shot("backup-blank")?;
    t.press(Action::BNext(bstep::BLANK));
    t.press(Action::BReveal);
    t.shot("backup-seed")?;
    // The copy scanned with its third word drawn wrong.
    t.press(Action::BScan);
    let wrong = copy_digits(c, &t, Some(2))?;
    t.app.event(Event::Scanned { bytes: wrong });
    t.shot("backup-scan-wrong")?;
    t.press(Action::Cancel);
    // Another paper form: test key 1 split into two Seed XOR parts.
    t.press(Action::BXor(2));
    t.shot("backup-xor")?;
    t.press(Action::BPaperHide);
    // The copy typed back.
    t.press(Action::BCheck);
    type_words(&mut t, None)?;
    t.press(Action::BStep(bstep::SHEETS));
    t.shot("backup-split")?;
    t.press(Action::BStep(bstep::ENVELOPE));
    t.shot("backup-envelope")?;
    // Every item done: what waits for the stick, and the way to one.
    t.press(Action::BNext(bstep::ENVELOPE));
    // The last item: one lock, then each stick by place.
    t.shot("backup-write-to-sticks")?;
    // The sheets went For the stick as PDFs with the checklist; written
    // beside the shots for a look. The blank template is printed from
    // Faraday Starter.
    t.press(Action::Backup(0));
    t.press(Action::BOut(3));
    let name = "savings-backup.pdf";
    let item = t
        .app
        .outbox
        .iter()
        .find(|it| it.name == name)
        .ok_or(format!("no {name}"))?;
    if !item.bytes.starts_with(b"%PDF-") {
        return Err(format!("{name} is not a PDF"));
    }
    std::fs::write(t.out.join(name), &item.bytes).map_err(|e| e.to_string())?;
    // A message, signed with the Spending wallet and checked.
    t.press(Action::SignMessage);
    let spending = t
        .app
        .session
        .wallets
        .iter()
        .position(|w| w.name == "Spending")
        .ok_or("no Spending")?;
    t.press(Action::MWallet(spending));
    t.press(Action::MNext(0));
    for c in "I control this address.".chars() {
        t.app.event(Event::Key(Key::Char(c)));
    }
    t.shot("message-text")?;
    t.press(Action::MNext(1));
    t.press(Action::MNext(2));
    t.press(Action::MSign);
    t.shot("message-signed")?;
    t.sticks(true);
    t.press(Action::Lock);
    t.app.storage(StorageEvent::Sticks(t.sticks.clone()));
    t.pump();
    if let Some(i) = t
        .stick()
        .files
        .iter()
        .position(|(n, _)| n == "spending-message.txt")
    {
        t.press(Action::VisitInAll);
        t.press(Action::VisitIn(i));
        t.press(Action::VisitCopy);
    }
    t.sticks(false);
    t.load_kit()?;
    let m = t
        .app
        .inbox
        .iter()
        .position(|i| i.name == "spending-message.txt")
        .ok_or("no message")?;
    t.press(Action::CheckMessage(m));
    t.shot("message-check")?;
    // Create a 2-of-3: test key 1 here, a new key, and a cosigner's key
    // from a file.
    t.sticks(true);
    t.press(Action::Lock);
    t.app.storage(StorageEvent::Sticks(t.sticks.clone()));
    t.pump();
    if let Some(i) = t
        .stick()
        .files
        .iter()
        .position(|(n, _)| n == "xpub-3-cf2e083d-multisig.txt")
    {
        t.press(Action::VisitInAll);
        t.press(Action::VisitIn(i));
        t.press(Action::VisitCopy);
    }
    t.sticks(false);
    t.load_kit()?;
    t.press(Action::CreateWallet);
    // Create opens on Kind (`docs/NEW-WALLET.md` §2.1): single key is
    // the default, ticked to Multisig instead.
    t.press(Action::CKind(4));
    t.shot("create-kind")?;
    t.press(Action::CNext(0));
    t.press(Action::CNext(1));
    let bacon = t.app.session.keys[0].master().fingerprint().0;
    t.press(Action::CSlotHere(0, bacon));
    dice_key(&mut t, 1)?;
    // The third cosigner's key is not here yet: the slot waits for it.
    t.press(Action::CSlotLater(2));
    t.press(Action::CKeyOut(0));
    let out = format!("xpub-{}.txt", fp_hex(bacon));
    if !t.app.outbox.iter().any(|i| i.name == out) {
        return Err(format!(
            "{out} did not reach the Outbox: {:?} {:?} keys {}",
            t.app.toast_text(),
            t.app.screen,
            t.app.session.keys.len()
        ));
    }
    t.shot("create-keys-later")?;
    t.press(Action::CNext(2));
    t.shot("create-waiting")?;
    if t.app.create_keys(c).is_ok() {
        return Err("a wallet with a key to come could be made".into());
    }
    t.press(Action::Nav(Screen::Start));
    t.shot("start-create-waiting")?;
    t.press(Action::CreateWallet);
    if t.app.create_waiting() != 1 {
        return Err("Create did not come back to the wallet being made".into());
    }
    // The key file arrives; it fills the waiting slot.
    let kf = t
        .app
        .inbox
        .iter()
        .position(|i| i.name == "xpub-3-cf2e083d-multisig.txt")
        .ok_or("no key file")?;
    t.press(Action::CSlotFile(2, kf));
    t.shot("create-keys")?;
    // Keys' Continue makes the wallet and opens Check.
    t.press(Action::CNext(2));
    t.shot("create-made")?;
    t.press(Action::OpenWallet(0));
    t.press(Action::QrWallet(0));
    t.shot("qr-wallet")?;
    // The code on screen is the descriptor, as a camera reads it.
    let read = t.decode();
    let want = t.app.session.wallets[0]
        .policy()
        .to_descriptor_checksummed();
    if read.first() != Some(&want) {
        return Err(format!("the wallet QR reads {read:?}, not the descriptor"));
    }
    println!("wallet QR decodes to the descriptor");
    t.press(Action::Cancel);
    // A FROST wallet dealt here: test key 1 and a new key are shares 1
    // and 2, and the deal computes share 3.
    t.press(Action::CreateWallet);
    t.press(Action::CKind(8)); // Threshold · FROST
    t.press(Action::CNext(0));
    t.press(Action::CNext(1));
    t.press(Action::CSlotHere(0, bacon));
    device_key(&mut t, 1);
    t.shot("create-frost-keys")?;
    let keys_before = t.app.session.keys.len();
    // Keys' Continue deals the shares.
    t.press(Action::CNext(2));
    t.shot("create-frost-dealt")?;
    let made = t.app.create.as_ref().and_then(|c| c.built).ok_or_else(|| {
        format!(
            "the deal made no wallet: {:?}",
            t.app.create.as_ref().and_then(|c| c.error.clone())
        )
    })?;
    let w = &t.app.session.wallets[made];
    if w.policy().record().is_none_or(|r| r.n() != 3 || r.t() != 2)
        || t.app.session.keys.len() != keys_before + 1
        || t.app.session.shares_here(w).len() != 3
    {
        return Err("the deal did not make a 2-of-3 record with its third share loaded".into());
    }
    println!("FROST 2 of 3 dealt here, share 3 computed");
    // A stick arrives while a key is held.
    t.sticks(true);
    t.shot("lock-sheet")?;
    t.press(Action::NotNow);
    t.shot("not-now")?;
    t.press(Action::Lock);
    t.app.storage(StorageEvent::Sticks(t.sticks.clone()));
    t.pump();
    t.shot("after-lock-visit")?;
    // Vaults: the test stick's vault copied in, unlocked, an entry added,
    // and sealed into the Outbox when the session locks.
    let vi = t
        .stick()
        .files
        .iter()
        .position(|(n, _)| n == "vault.ofv")
        .ok_or("the kit has no vault.ofv")?;
    t.press(Action::VisitInAll);
    t.press(Action::VisitIn(vi));
    for name in ["vault-entries.txt", "seed-2-5d388376.oskb", "BOOTX64.EFI"] {
        let k = t
            .stick()
            .files
            .iter()
            .position(|(n, _)| n == name)
            .ok_or(format!("the kit has no {name}"))?;
        t.press(Action::VisitIn(k));
    }
    t.press(Action::VisitCopy);
    let original = t
        .app
        .inbox
        .iter()
        .find(|i| i.name == "vault.ofv")
        .map(|i| i.bytes.clone())
        .ok_or("vault.ofv was not copied in")?;
    // With the stick still in, the passphrase waits for it to be pulled.
    t.press(Action::Vault(V::Open(0)));
    t.shot("unlock-stick")?;
    t.sticks(false);
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    t.press(Action::Nav(Screen::Vaults));
    t.shot("vaults")?;
    t.press(Action::Vault(V::Open(0)));
    type_text(&mut t, testkit::VAULT_PASSPHRASES[0]);
    t.shot("unlock")?;
    t.press(Action::Vault(V::Unlock));
    t.tick();
    // Unlocked from the Vaults list (`docs/NEW-WALLET.md` §11.1a): its
    // Load sheet (F62), every card ticked; Load ticked lands on Wallets.
    if t.app.sheet != Some(faraday_core::Sheet::UnlockLoad) || !t.app.session.keys.is_empty() {
        return Err("the unlock did not ask what to load".into());
    }
    t.shot("vault-load-sheet")?;
    t.press(Action::Vault(V::LoadTickAll));
    t.shot("vault-load-sheet-none")?;
    t.press(Action::Vault(V::LoadTickAll));
    t.press(Action::Vault(V::LoadTicked));
    if t.app.screen != Screen::Wallets || t.app.session.keys.is_empty() {
        return Err("the unlock did not load the vault's wallets".into());
    }
    t.shot("wallets-vault-open")?;
    t.press(Action::Nav(Screen::VaultContents));
    if t.app.vaults.open.len() != 1
        || t.app.session.keys.len() != 1
        || !t.app.session.wallets.iter().any(|w| w.name == "Spending")
    {
        return Err(format!(
            "the test vault did not unlock with its key and wallet: {:?}",
            t.app.vaults.unlock_error
        ));
    }
    t.shot("vault-keys")?;
    t.press(Action::Vault(V::Category(2)));
    t.shot("vault-entries")?;
    t.press(Action::Vault(V::Add));
    type_text(&mut t, "Bank");
    t.app.event(Event::Key(Key::Tab));
    type_text(&mut t, "me@example.com");
    t.app.event(Event::Key(Key::Tab));
    type_text(&mut t, "hunter2");
    t.shot("vault-entry-form")?;
    t.press(Action::Vault(V::FormSave));
    t.shot("vault-entry-added")?;
    t.press(Action::Vault(V::Category(3)));
    t.shot("vault-notes")?;
    // Entries from a text file and from a TOTP setup code.
    t.press(Action::Vault(V::Category(2)));
    t.press(Action::Vault(V::ImportAllEntries));
    t.press(Action::Vault(V::ScanEntry));
    t.app.event(Event::Scanned {
        bytes: b"otpauth://totp/Mail:me%40example.com?secret=ABCDEFGH&issuer=Mail".to_vec(),
    });
    t.pump();
    t.shot("vault-entry-scanned")?;
    t.press(Action::Vault(V::FormSave));
    let entries = |t: &Tour| {
        t.app.vaults.open[0]
            .contents
            .records
            .iter()
            .filter(|r| r.kind == 6)
            .count()
    };
    if entries(&t) != 5 {
        return Err(format!(
            "expected 5 entries after the imports, found {}",
            entries(&t)
        ));
    }
    t.shot("vault-entries-imported")?;
    // Test key 2 from an OpenSigner backup.
    t.press(Action::Vault(V::Category(0)));
    let backup = t
        .app
        .inbox
        .iter()
        .position(|i| i.name == "seed-2-5d388376.oskb")
        .ok_or("the backup was not copied in")?;
    t.press(Action::Vault(V::ImportBackup(backup)));
    type_text(&mut t, testkit::BACKUP_PASSPHRASE);
    t.shot("vault-backup-prompt")?;
    t.press(Action::Vault(V::PromptGo));
    t.tick();
    let keys = |t: &Tour| {
        t.app.vaults.open[0]
            .contents
            .records
            .iter()
            .filter(|r| r.kind == 2)
            .count()
    };
    if keys(&t) != 2 {
        return Err(format!(
            "the backup's key is not in the vault: {:?}",
            t.app.vaults.prompt.as_ref().and_then(|p| p.error.clone())
        ));
    }
    // Test key 3 with a BIP-39 passphrase, saved with it.
    t.press(Action::Entry(None));
    type_text(&mut t, &testkit::test_words(testkit::TEST_SEEDS[2].0));
    t.press(Action::EntryPassphrase);
    type_text(&mut t, "TREZOR");
    t.shot("entry-passphrase")?;
    t.press(Action::EntryAdd);
    let with = t
        .app
        .session
        .keys
        .iter()
        .position(|k| k.passphrase.is_some())
        .ok_or("the key with a passphrase was not added")?;
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::Category(0)));
    t.press(Action::Vault(V::Add));
    t.shot("vault-save-key")?;
    t.press(Action::Vault(V::SaveKeyWithPassphrase(with)));
    if !t.app.vaults.open[0]
        .contents
        .records
        .iter()
        .any(|r| r.kind == 2 && r.text(4) == Some("TREZOR"))
    {
        return Err("the key was not saved with its passphrase".into());
    }
    t.shot("vault-keys-three")?;
    println!(
        "entries imported and scanned, a backup opened, a key saved with its BIP-39 passphrase"
    );
    // The vault's Spending wallet backed up with Paper and vault, its
    // copy checked once for each place: Backups, and the card's line.
    let spending = t
        .app
        .session
        .wallets
        .iter()
        .position(|w| w.name == "Spending")
        .ok_or("Spending is not loaded")?;
    t.press(Action::Backup(spending));
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    t.press(Action::BChecklist);
    if t.app.backup.as_ref().and_then(|b| b.open) != Some(bstep::COPY) {
        t.press(Action::BStep(bstep::COPY));
    }
    t.press(Action::BReveal);
    for _ in 0..2 {
        t.press(Action::BCheck);
        type_words(&mut t, None)?;
    }
    t.press(Action::BackupsOf(spending));
    t.shot("backups")?;
    t.press(Action::OpenWallet(spending));
    t.shot("wallet-card-backup")?;
    t.press(Action::Nav(Screen::VaultContents));
    // A GPG key made in the vault, its certificate and revocation in the
    // Outbox, and a file signed with it.
    t.app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    t.press(Action::Vault(V::Category(4)));
    flow_at_make(c, &mut t, faraday_core::backup_flow::Kind::Gpg);
    type_text(&mut t, "Test Person");
    t.app.event(Event::Key(Key::Tab));
    type_text(&mut t, "test@example.com");
    t.shot("vault-gpg-make")?;
    // On the stick, with the clock not checked, Make asks for the time
    // first (`docs/TOTP.md` §2.2); Keep this time, then Make again.
    t.press(Action::Flow(
        faraday_core::backup_flow_steps::FlowAction::Make,
    ));
    if t.app.screen != Screen::SetTime {
        return Err("making a GPG key did not ask for the time".into());
    }
    flow_made(&mut t, faraday_core::backup_flow::Kind::Gpg)?;
    let gpg_files: Vec<String> = t
        .app
        .outbox
        .iter()
        .filter(|i| i.name.ends_with(".asc"))
        .map(|i| i.name.clone())
        .collect();
    if gpg_files.len() != 1 {
        return Err(format!("making a GPG key left {gpg_files:?} in the Outbox"));
    }
    t.shot("vault-gpg-key")?;
    t.press(Action::Vault(V::GpgSignPick));
    // Two files ticked: the checkbox list, before Sign 2 files.
    let signable: Vec<String> = t
        .app
        .inbox
        .iter()
        .filter(|i| !i.name.ends_with(".asc"))
        .map(|i| i.name.clone())
        .collect();
    for name in ["vault-entries.txt", "BOOTX64.EFI"] {
        let k = signable
            .iter()
            .position(|n| n == name)
            .ok_or(format!("no {name} to sign"))?;
        t.press(Action::Vault(V::GpgTick(k)));
    }
    t.shot("vault-gpg-sign")?;
    // Sign opens Sign with {key}: the files, the time, the hold (F63).
    t.press(Action::Vault(V::GpgSign));
    if t.app.sheet != Some(faraday_core::Sheet::GpgSign) {
        return Err("Sign did not open its sheet".into());
    }
    t.shot("vault-gpg-sign-sheet")?;
    t.press(Action::Vault(V::GpgHoldSign));
    for name in ["vault-entries.txt", "BOOTX64.EFI"] {
        if !t.app.outbox.iter().any(|i| i.name == format!("{name}.asc")) {
            return Err(format!(
                "signing left no signature for {name} in the Outbox"
            ));
        }
    }
    println!("a GPG key made in the vault signed two files at once");
    // TOTP codes for the entries the imported file left in the vault
    // (`docs/TOTP.md` §9): the clock is the GPG step's fixed one, so the
    // codes are the same in every render.
    t.press(Action::OpenCodes);
    if t.app.screen == Screen::SetTime {
        t.press(Action::SetTimeKeep);
    }
    if t.app.screen != Screen::Codes {
        return Err("TOTP codes did not open".into());
    }
    t.shot("totp-codes")?;
    let totp_item = t.app.vaults.open[0]
        .contents
        .records
        .iter()
        .filter(|r| r.kind == 6)
        .position(|r| r.text(1) == Some("Example Git"))
        .ok_or("the imported TOTP entry is not in the vault")?;
    t.press(Action::CodesOpen(0, totp_item));
    t.shot("vault-entry-code")?;
    // Secure Boot keys made in the vault, the enrolment files, and the
    // test image signed after its hash is shown.
    t.press(Action::Vault(V::Category(5)));
    flow_at_make(c, &mut t, faraday_core::backup_flow::Kind::SecureBoot);
    type_text(&mut t, "Test");
    flow_made(&mut t, faraday_core::backup_flow::Kind::SecureBoot)?;
    if !t.app.vaults.open[0]
        .contents
        .records
        .iter()
        .any(|r| r.kind == 8)
    {
        return Err("no Secure Boot keys were made".into());
    }
    t.shot("vault-secure-boot")?;
    t.press(Action::Vault(V::SbEnrol));
    for name in ["PK.auth", "KEK.auth", "db.auth", "db.esl", "db.cer"] {
        if !t.app.outbox.iter().any(|i| i.name == name) {
            return Err(format!("{name} is not in the Outbox"));
        }
    }
    t.press(Action::Vault(V::SbImages));
    let efi = t
        .app
        .inbox
        .iter()
        .position(|i| i.name == "BOOTX64.EFI")
        .ok_or("the test image was not copied in")?;
    t.press(Action::Vault(V::SbPick(efi)));
    t.shot("vault-secure-boot-sign")?;
    t.press(Action::Vault(V::SbHoldSign));
    let signed = t
        .app
        .outbox
        .iter()
        .find(|i| i.name == "BOOTX64.EFI")
        .map(|i| i.bytes.clone())
        .ok_or("the signed image is not in the Outbox")?;
    let keys = t.app.vaults.open[0]
        .contents
        .records
        .iter()
        .find(|r| r.kind == 8)
        .and_then(faraday_core::secureboot::keys_of)
        .ok_or("the keys do not read back")?;
    if faraday_sb::pe::check(&signed, &keys.db.cert) != Ok(faraday_sb::Check::Valid) {
        return Err("the signed image does not check against the db key".into());
    }
    println!("Secure Boot keys made, enrolment files written, an image signed");
    t.press(Action::LockAsk);
    t.shot("vault-lock-ask")?;
    // Lock seals the changed slot; the decoy slot is copied byte for byte.
    t.app.press(Action::Lock);
    let sealed = t
        .app
        .outbox
        .iter()
        .find(|i| i.name == "vault.ofv")
        .map(|i| i.bytes.clone())
        .ok_or("locking put no sealed vault in the Outbox")?;
    // What the lock saved for the next process: the boxes and the kept
    // state.
    let mut saved = None;
    while let Some(c) = t.app.poll_storage() {
        if let StorageCommand::SaveBoxes {
            inbox,
            outbox,
            kept,
        } = c
        {
            saved = Some((inbox, outbox, kept));
        }
    }
    t.pump();
    let (_, main) = faraday_vault::open(c, &sealed, testkit::VAULT_PASSPHRASES[0].as_bytes())
        .map_err(|e| e.reason())?;
    let (_, decoy) = faraday_vault::open(c, &sealed, testkit::VAULT_PASSPHRASES[1].as_bytes())
        .map_err(|e| e.reason())?;
    let slot = 65_536 + 40;
    let changed = (0..4)
        .filter(|i| {
            let at = faraday_vault::HEADER_LEN + i * slot;
            original[at..at + slot] != sealed[at..at + slot]
        })
        .count();
    if !main.records.iter().any(|r| r.text(1) == Some("Bank"))
        || decoy.records.iter().filter(|r| r.kind == 6).count() != 1
        || changed != 1
    {
        return Err("the sealed vault lost its entry, or changed more than the open slot".into());
    }
    println!("vault unlocked, an entry added, sealed on lock with the other slots unchanged");
    // The next process, given what the lock saved: the locked vault's row
    // and Home's lead say what it held when last seen open. Then a fresh
    // process again, as the lock left it, for the rest of the tour.
    let (inbox, outbox, kept) = saved.ok_or("the lock saved nothing")?;
    t.app.storage(StorageEvent::Restored {
        inbox,
        outbox,
        kept,
    });
    t.press(Action::Nav(Screen::Vaults));
    t.shot("vaults-remembered")?;
    t.press(Action::Nav(Screen::Home));
    t.shot("home-vault-remembered")?;
    // A stick for the vault sealed at lock: the visit goes to Unlock.
    t.sticks(true);
    t.shot("visit-then-unlock")?;
    t.press(Action::VisitWrite);
    t.sticks(false);
    if t.app.screen != Screen::Unlock {
        return Err("pulling the stick after writing the vault did not go to Unlock".into());
    }
    // After the visit: Spending is listed from what the vault was seen
    // to hold, its vault line on the stick written.
    t.press(Action::Nav(Screen::Backups));
    let at = t
        .app
        .backups_now(c)
        .iter()
        .position(|e| e.name == "Spending")
        .ok_or("Spending is not listed after the lock")?;
    t.press(Action::BackupsPage(at));
    t.shot("backups-after-visit")?;
    t.restart();
    // A new vault: two passphrases at the Light cost.
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Create));
    // It opens on Name and passphrases, the size closed on its default.
    t.shot("create-vault")?;
    t.press(Action::Vault(V::CStep(faraday_core::vaults::vstep::PRESET)));
    t.shot("create-vault-size")?;
    // Customise: the three cards in the size's place.
    t.press(Action::Vault(V::CCustomise));
    t.shot("create-vault-customise")?;
    t.press(Action::Vault(V::CNext(faraday_core::vaults::vstep::WHERE)));
    t.press(Action::Vault(V::CPreset(0)));
    t.shot("create-vault-cost")?;
    // A fresh Create, on the default size, makes the tour's vault.
    t.press(Action::Vault(V::Create));
    // The first passphrase from dice: six words, thirty rolls, run
    // together with no spaces.
    let rolls = "123456123456123456123456123456";
    t.press(Action::Vault(V::Dice(0)));
    type_text(&mut t, rolls);
    t.shot("create-vault-dice")?;
    // The first word in its list, and back to the rolls as they were.
    let first = faraday_core::vaults::dice_word_indices(rolls, osk_bip::diceware::List::Large)[0];
    t.press(Action::WordList(
        faraday_core::wordlist::WordListAction::Open(1, Some(first)),
    ));
    t.shot("create-vault-dice-word")?;
    t.press(Action::WordList(
        faraday_core::wordlist::WordListAction::Close,
    ));
    t.press(Action::Vault(V::DiceUse));
    let dice_phrase = t
        .app
        .vaults
        .create
        .as_ref()
        .map(|c| c.phrases[0].0.text.to_string())
        .unwrap_or_default();
    let expect = faraday_core::vaults::dice_words(rolls, osk_bip::diceware::List::Large).join("");
    if dice_phrase != expect {
        return Err(format!("dice gave {dice_phrase:?}, expected {expect:?}"));
    }
    t.press(Action::Vault(V::CAddPhrase));
    type_text(&mut t, "second passphrase");
    t.app.event(Event::Key(Key::Tab));
    type_text(&mut t, "second passphrase");
    t.shot("create-vault-phrases")?;
    t.press(Action::Vault(V::CGo));
    t.tick();
    let made = t
        .app
        .outbox
        .iter()
        .find(|i| i.name == "vault.ofv")
        .map(|i| i.bytes.clone())
        .ok_or_else(|| {
            format!(
                "no vault was made: {:?}",
                t.app.vaults.create.as_ref().and_then(|c| c.error.clone())
            )
        })?;
    for p in [dice_phrase.as_str(), "second passphrase"] {
        faraday_vault::open(c, &made, p.as_bytes()).map_err(|e| format!("{p}: {}", e.reason()))?;
    }
    // Made: Unlock, the new vault picked, its passphrase asked once more.
    t.shot("create-vault-unlock")?;
    println!("a vault made here opens with each of its two passphrases");
    let before = t.app.vaults.open.len();
    let row = t
        .app
        .vault_files()
        .iter()
        .position(|f| Some(&f.name) == t.app.vaults.just_made.as_ref())
        .ok_or("the vault just made is not listed")?;
    t.press(Action::Vault(V::Open(row)));
    // Opening it now is the same Unlock screen as any other vault: its
    // passphrase typed again, not carried over from making it.
    type_text(&mut t, &dice_phrase);
    t.press(Action::Vault(V::Unlock));
    t.tick();
    if t.app.vaults.open.len() != before + 1 {
        return Err(format!(
            "the vault just made did not open on its own passphrase: {:?}",
            t.app.vaults.unlock_error
        ));
    }
    t.shot("vault-kept-open")?;
    t.press(Action::Nav(Screen::Files));
    t.shot("files-outbox-vault-open")?;
    println!("the vault just made opens on its first passphrase typed again");
    seeds_tour(&mut t)?;
    boot_tour(c, &mut t)?;
    spend_tour(&mut t)?;
    // Tools: every flow, and Find a tool.
    t.press(Action::Nav(Screen::Catalog));
    t.shot("tools")?;
    type_text(&mut t, "bip-8");
    t.shot("tools-find")?;
    t.app.event(Event::Key(Key::Escape));
    // New key as SLIP-39 shares: the plan and what shares reveal.
    t.press(Action::KeyGenSlip39);
    t.shot("keygen-slip39")?;
    t.press(Action::Nav(Screen::Catalog));
    word_lists(&mut t)?;
    rolled_words(&mut t)?;
    keyed_entry(&mut t)?;
    silent_wallet(&mut t)?;
    t.press(Action::Nav(Screen::Catalog));
    // Create › MuSig2: every key signs.
    t.press(Action::Catalog(
        faraday_core::catalog::TILES
            .iter()
            .position(|x| x.name == "MuSig2")
            .unwrap_or(0) as u8,
    ));
    t.press(Action::CNext(faraday_core::cstep::KIND));
    t.press(Action::CNext(faraday_core::cstep::QUORUM));
    t.shot("create-musig")?;
    // Left alone for five minutes with a key loaded: the warning and its
    // countdown.
    t.now += 5 * 60_000 + 20_000;
    t.app.event(Event::Tick { now_ms: t.now });
    t.shot("idle-warning")?;
    t.app.event(Event::Key(Key::Escape));
    t.press(Action::Nav(Screen::Settings));
    t.shot("settings")?;
    // The bottom of Settings, where About says what the self-test found.
    for _ in 0..30 {
        t.app.event(Event::Key(Key::Down));
    }
    t.shot("settings-about")?;
    t.press(Action::PowerAsk);
    t.shot("power-sheet")?;
    t.app.event(Event::Key(Key::Escape));
    // A stick that started with Secure Boot off reminds at power-off.
    t.app.secure_boot = faraday_core::SecureBootAtStart::Off;
    t.app.maker = "LENOVO".into();
    t.app.maker_family = "ThinkPad X1 Carbon".into();
    t.app.maker_model = "Example 14".into();
    t.press(Action::PowerAsk);
    t.shot("power-sheet-secure-boot-off")?;
    t.app.event(Event::Key(Key::Escape));
    t.app.secure_boot = faraday_core::SecureBootAtStart::Unknown;
    // New key's hint for cards, with its en dash.
    t.press(Action::KeyGen(None));
    t.press(Action::KWords(12));
    t.press(Action::KWay(faraday_core::keygen::Way::Cards.index()));
    t.press(Action::KNext);
    t.shot("keygen-cards")?;
    // Last, since nothing after it does anything: a self-test that failed.
    t.app.run_selftest_with(&[osk_selftest::Check {
        name: "BIP-39 vector (entropy → words → seed)",
        run: |_| false,
    }]);
    t.shot("selftest-failed")?;
    Ok(())
}

/// Test key 1's fingerprint, among the keys loaded.
fn test_key_1(t: &Tour) -> Result<[u8; 4], String> {
    let words = testkit::test_words(testkit::TEST_SEEDS[0].0);
    t.app
        .session
        .keys
        .iter()
        .find(|k| k.words.as_deref().is_some_and(|w| w.as_str() == words))
        .map(|k| k.master().fingerprint().0)
        .ok_or_else(|| "test key 1 is not loaded".to_string())
}

/// A fresh session at boot: the boot stick's files are copied into
/// memory and the sheet lists them, each ticked (F60); pulling the stick
/// leaves the sheet as it was. Import brings them in and puts the vault's
/// passphrase on the sheet. A wrong passphrase stays; Later leaves Home
/// leading with the vault's Unlock; the right passphrase loads the
/// vault's wallets and keys. Then a wallet made here is saved into the
/// vault.
fn boot_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::Sheet;
    use faraday_core::boot_import::ImportAction as I;
    t.press(Action::Lock);
    let mut boot = t.stick();
    boot.boot = true;
    boot.label = "FARADAY".into();
    t.app.storage(StorageEvent::Sticks(vec![boot]));
    t.pump();
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    if t.app.screen != Screen::Home
        || t.app.sheet != Some(Sheet::Import)
        || t.app.inbox.iter().any(|i| i.name == "vault.ofv")
    {
        return Err("the boot stick's files did not come up for the import".into());
    }
    t.shot("boot-import-stick")?;
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    if t.app.sheet != Some(Sheet::Import) || t.app.import_field().is_some() {
        return Err("pulling the boot stick did not leave the sheet's list".into());
    }
    let (w, h) = t.size();
    t.app.event(Event::Scroll {
        x: w / 2,
        y: h / 2,
        dy: i16::MAX,
    });
    t.shot("boot-import-end")?;
    t.press(Action::Import(I::Go));
    if t.app.sheet != Some(Sheet::Import) || t.app.import_field().is_none() {
        return Err("Import did not put the vault's passphrase on the sheet".into());
    }
    t.shot("boot-import")?;
    // A wrong passphrase stays on the sheet and says so. A tap on the
    // field first, which on a small panel brings the keyboard up.
    t.press(Action::Vault(V::FocusPassphrase));
    type_text(t, "not the passphrase");
    t.press(Action::Import(I::Submit));
    t.tick();
    if t.app.sheet != Some(Sheet::Import) || t.app.vaults.unlock_error.is_none() {
        return Err("a wrong passphrase did not stay on the sheet".into());
    }
    t.shot("boot-import-wrong")?;
    // Later leaves Home leading with the vault's Unlock, which opens the
    // sheet again.
    t.press(Action::Import(I::Later));
    t.shot("home-import-waiting")?;
    t.press(faraday_core::boot_import::OPEN);
    if t.app.sheet != Some(Sheet::Import) {
        return Err("the import did not open again from Home".into());
    }
    t.press(Action::Vault(V::FocusPassphrase));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.shot("boot-unlock")?;
    t.press(Action::Import(I::Submit));
    t.tick();
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    // Its Load sheet (F62), then Load ticked; none when everything it
    // holds is loaded already.
    if t.app.sheet == Some(Sheet::UnlockLoad) {
        t.shot("boot-load-sheet")?;
        t.press(Action::Vault(V::LoadTicked));
    }
    if t.app.vaults.open.len() != 1
        || t.app.import.is_some()
        || t.app.sheet.is_some()
        || t.app.screen != Screen::Wallets
    {
        return Err("unlocking on the import sheet did not import and land on Wallets".into());
    }
    if t.app.session.wallets.len() < 12 || t.app.session.keys.is_empty() {
        return Err(format!(
            "the import gave {} wallets and {} keys",
            t.app.session.wallets.len(),
            t.app.session.keys.len()
        ));
    }
    t.shot("boot-imported")?;
    t.press(Action::Nav(Screen::Files));
    t.shot("boot-loaded")?;
    // A 2-of-3 made here over test key 1 and two new keys, saved into
    // the open vault with the new keys.
    t.press(Action::CreateWallet);
    t.press(Action::CKind(4));
    t.press(Action::CNext(0));
    t.press(Action::CNext(1));
    // Test key 1, which the vault holds.
    let bacon = test_key_1(t)?;
    t.press(Action::CSlotHere(0, bacon));
    device_key(t, 1);
    device_key(t, 2);
    t.press(Action::CNext(faraday_core::cstep::KEYS));
    t.press(Action::CNext(faraday_core::cstep::CHECK));
    // After the check, the backup plan: Paper and vault, every seed
    // ticked into Vault 1, puts the new keys and the wallet into the open
    // vault from the checklist.
    let built = t.app.create.as_ref().and_then(|c| c.built).unwrap_or(0);
    t.press(Action::CBackup);
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    let here = t.app.plan_shape_now(built).here();
    let vaults = t
        .app
        .backup
        .as_ref()
        .map_or(0, |b| b.answers.vault_count(&t.app.plan_shape_now(built)));
    let p = vault_place(c, t, 0).unwrap_or(0);
    keep_vault(c, t, p, 0, &here);
    for v in 1..vaults {
        vault_off(c, t, v);
    }
    // The description into the vault, and as files too, for the public
    // files item below.
    description_into_vault(c, t, 0);
    description_file_at(c, t, 0);
    t.press(Action::BChecklist);
    // The items before the vault's wait for it (`docs/NEW-WALLET.md`
    // §14.1): each copy by hand checked in turn, then the vault.
    items_before(c, t, faraday_core::bstep::VAULT)?;
    if t.app.backup.as_ref().and_then(|b| b.open) != Some(faraday_core::bstep::VAULT) {
        t.press(Action::BStep(faraday_core::bstep::VAULT));
    }
    // Making the checklist saved the plan into the vault already.
    use faraday_vault::records::kind;
    let count = |t: &Tour, k: u8| t.app.vaults.open[0].contents.of(k).count();
    let before = t.app.vaults.open[0].changes;
    let plans = count(t, kind::PLAN);
    let keys = count(t, kind::KEY);
    let wallets = count(t, kind::WALLET);
    t.shot("create-backup-vault")?;
    // One Save into the vault saves all the plan puts there
    // (`docs/NEW-WALLET.md` §14.1): the two new keys and the wallet;
    // test key 1 was there already. The plan record the checklist wrote
    // is kept as it is, one of it: nothing in it changed.
    t.press(Action::BVaultSave(0));
    t.shot("create-backup-vault-saved")?;
    let saved = t.app.vaults.open[0].changes - before;
    if saved != 3 {
        return Err(format!("the checklist saved {saved} records, not 3"));
    }
    if count(t, kind::KEY) != keys + 2 || count(t, kind::WALLET) != wallets + 1 {
        return Err("the vault's save did not add the two new keys and the wallet".into());
    }
    if plans == 0 || count(t, kind::PLAN) != plans {
        return Err(format!(
            "the vault holds {} plan records, {plans} before the save",
            count(t, kind::PLAN)
        ));
    }
    // Then the public files, to the Outbox.
    t.press(Action::BStep(faraday_core::bstep::PUBLIC));
    t.press(Action::PublicOut(built, 1));
    t.press(Action::PublicOut(built, 2));
    t.shot("create-backup-public")?;
    let outbox: Vec<_> = t.app.outbox.iter().map(|i| i.kind.exposure()).collect();
    if outbox.contains(&faraday_core::secrets::Exposure::Secret) {
        return Err("a secret reached the Outbox from Create".into());
    }
    println!("boot: imported from the boot stick, a new wallet saved into its vault");
    // BIP-85: a password from test key 1, into the open vault.
    t.press(Action::Nav(Screen::Start));
    t.press(Action::Bip85);
    if t.app.bip85.is_none() {
        return Err("BIP-85 did not open".into());
    }
    let first = bacon;
    t.press(Action::PKey(first));
    t.press(Action::PApp(4));
    t.shot("bip85-app")?;
    t.press(Action::PNext);
    t.press(Action::PNext);
    t.press(Action::PShow);
    t.shot("bip85-value")?;
    t.press(Action::PVault);
    // Silent payments: test key 1's address with label 1.
    t.press(Action::Nav(Screen::Start));
    t.press(Action::Silent);
    t.press(Action::SKey(first));
    t.press(Action::SLabel(1));
    t.shot("silent-address")?;
    // Explore: test key 1 at BIP-86's first address.
    t.press(Action::Nav(Screen::Start));
    t.shot("start-tools")?;
    t.press(Action::Explore);
    t.press(Action::XPreset(1));
    t.shot("explore")?;
    // Lightning: test key 1 as ldk-node reads it.
    t.press(Action::Nav(Screen::Start));
    t.press(Action::Lightning);
    t.press(Action::LKey(first));
    t.shot("lightning")?;
    // Tools: a key in every spelling.
    t.press(Action::Nav(Screen::Start));
    t.press(Action::Tools);
    t.press(Action::TTool(3));
    type_text(
        t,
        "xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V",
    );
    t.shot("tools-convert")?;
    // Restore begins with the kind of wallet (`docs/NEW-WALLET.md` §12).
    t.press(Action::Lock);
    t.press(Action::Nav(Screen::Start));
    t.shot("start-empty")?;
    t.press(Action::RestoreWallet);
    t.shot("restore-kind")?;
    t.press(Action::RNext(faraday_core::rstep::KIND));
    t.shot("restore-sources")?;
    Ok(())
}

/// The Spend tab from a fresh start: the map, the envelope, the vault and
/// the PSBT copied in from a stick, the unlock, the wallet chosen, the
/// check, the transaction, test key 2 typed from the signing card, the
/// signature and the result.
/// Each theme on three screens, with the test kit loaded: the Wallets
/// tab's first page, the wallets, and Settings.
/// The desktop app's Transfer over a Downloads folder in the output
/// directory: the list newest first, a PSBT sent as codes, a file over
/// 256 KiB refused, and Receive saving a descriptor and a seed's words
/// that a device sent, each said as saved, the words as a secret.
/// Upgrade a Faraday stick (`PLAN.md` §5.5), the boot copier's answers
/// given by hand.
fn upgrade_tour(t: &mut Tour) -> Result<(), String> {
    const RUNNING: &str = "6.6.84-faraday-0.2.0+4d0680b1a2b3";
    const MB: u64 = 1 << 20;
    let part = |id: &str, size: u64, release: Option<&str>, source: bool| BootPart {
        id: id.into(),
        size,
        release: release.map(str::to_string),
        source,
    };
    let source = part("sda1@sda#1", 48 * MB, Some(RUNNING), true);
    t.press(Action::Nav(Screen::Settings));
    t.shot("settings")?;
    // With a key loaded, the lock comes first.
    t.press(Action::Entry(None));
    t.type_key(0);
    t.press(Action::Nav(Screen::Settings));
    t.press(Action::UpgradeOpen);
    t.shot("upgrade-lock")?;
    t.press(Action::Lock);
    // The fresh process, given what the lock kept.
    t.app.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept: vec![("upgrade".into(), b"1".to_vec())],
    });
    t.pump();
    if t.app.screen != Screen::Upgrade {
        return Err("the fresh process did not open on the upgrade".into());
    }
    t.shot("upgrade-insert")?;
    t.app.storage(StorageEvent::Boots(vec![part(
        "sda1@sda#1",
        48 * MB,
        Some(RUNNING),
        false,
    )]));
    t.shot("upgrade-reading")?;
    t.app.storage(StorageEvent::BootSource {
        id: "sda1@sda#1".into(),
        release: RUNNING.into(),
        size: 48 * MB,
    });
    t.app.storage(StorageEvent::Boots(vec![source.clone()]));
    t.shot("upgrade-source-read")?;
    // The vault stick goes in: an older Faraday, its data partition with
    // a vault and the settings file.
    t.app.storage(StorageEvent::Sticks(vec![StickInfo {
        serial: None,
        id: "sdb2@sdb#2".into(),
        label: "FARADAYDATA".into(),
        boot: false,
        files: vec![
            ("vault.ofv".into(), 16_778_240),
            ("faraday-settings.txt".into(), 120),
        ],
    }]));
    let older = part("sdb1@sdb#2", 48 * MB, None, false);
    t.app
        .storage(StorageEvent::Boots(vec![source.clone(), older.clone()]));
    t.shot("upgrade-older")?;
    // A newer one beside it.
    let newer = part(
        "sdc1@sdc#3",
        48 * MB,
        Some("6.6.84-faraday-0.3.0+0123456789ab"),
        false,
    );
    t.app.storage(StorageEvent::Boots(vec![
        source.clone(),
        older.clone(),
        newer,
    ]));
    t.press(Action::UpgradePick(1));
    t.shot("upgrade-newer")?;
    // One too small.
    t.app.storage(StorageEvent::Boots(vec![
        source.clone(),
        part("sdd1@sdd#4", 32 * MB, None, false),
    ]));
    t.shot("upgrade-too-small")?;
    // Pulled during the write.
    t.app
        .storage(StorageEvent::Boots(vec![source.clone(), older.clone()]));
    t.press(Action::UpgradeWrite);
    t.shot("upgrade-writing")?;
    t.app.storage(StorageEvent::BootWriteFailed {
        id: older.id.clone(),
        reason: "the stick was removed".into(),
        pulled: true,
    });
    t.shot("upgrade-pulled")?;
    // Put back, and written.
    t.press(Action::UpgradeWrite);
    t.app.storage(StorageEvent::BootWritten {
        id: older.id.clone(),
        release: RUNNING.into(),
    });
    t.app.storage(StorageEvent::Boots(vec![
        source,
        part("sdb1@sdb#2", 48 * MB, Some(RUNNING), false),
    ]));
    t.shot("upgrade-done")?;
    Ok(())
}

fn transfer_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    let downloads = t.out.join("downloads");
    std::fs::create_dir_all(&downloads).map_err(|e| e.to_string())?;
    let put = |name: &str, bytes: &[u8]| -> Result<(), String> {
        let path = downloads.join(name);
        if !path.exists() {
            std::fs::write(&path, bytes).map_err(|e| format!("{name}: {e}"))?;
        }
        // Written apart in the order given, so the last is the newest.
        std::thread::sleep(std::time::Duration::from_millis(20));
        Ok(())
    };
    let spending = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .ok_or("no spending kit")?;
    put("photo-2026-10-01.jpg", &vec![0x5a; 300 * 1024])?;
    put("notes-backup.tar", &vec![0x33; 100 * 1024])?;
    put("spending-wallet.txt", spending.descriptor.as_bytes())?;
    let psbt = std::fs::read(t.kit.join("savings-unsigned.psbt")).map_err(|e| e.to_string())?;
    put("savings-unsigned.psbt", &psbt)?;
    // The online app starts on mainnet as the demo; a test network waits
    // for its warning (F20).
    t.app.online = true;
    t.app.session = faraday_core::wallet::Session::default();
    t.app.session.set_demo(c);
    t.press(Action::Network(testkit::NET));
    t.shot("test-network")?;
    t.press(Action::AirgapUnderstood);
    let host = faraday_storage::Host {
        print: t.out.clone(),
        downloads: downloads.clone(),
        opens: true,
    };
    t.app.storage(host.files());
    t.host = Some(host);
    t.press(Action::Nav(Screen::Transfer));
    if t.app.screen != Screen::Transfer {
        return Err("Transfer did not open in the online app".into());
    }
    t.shot("transfer")?;
    let at = |t: &Tour, name: &str| {
        t.app
            .transfer
            .files
            .iter()
            .position(|(n, _)| n == name)
            .ok_or(format!("{name} is not listed"))
    };
    let i = at(t, "savings-unsigned.psbt")?;
    t.press(Action::TransferSend(i));
    if t.app.qr.is_none() {
        return Err("the PSBT was not shown as codes".into());
    }
    t.shot("transfer-send-psbt")?;
    t.press(Action::Cancel);
    let i = at(t, "photo-2026-10-01.jpg")?;
    t.press(Action::TransferSend(i));
    t.shot("transfer-refused")?;
    // Receive: a frame, then what a device's Files sends as codes.
    t.press(Action::TransferReceive);
    let frame: Vec<u8> = (0..640u32 * 480)
        .map(|i| ((i % 640) / 3 + (i / 640) / 4) as u8)
        .collect();
    t.app.event(Event::CameraFrame {
        width: 640,
        height: 480,
        luma: frame,
        chroma: None,
    });
    let envelope = |name: &str, bytes: &[u8]| -> Result<Vec<String>, String> {
        let env = faraday_qr::envelope::pack(name, bytes, "file")?;
        faraday_qr::bbqr::encode('J', &env, 160).ok_or_else(|| "too large".to_string())
    };
    for part in envelope("spending-wallet.txt", spending.descriptor.as_bytes())? {
        t.app.event(Event::Scanned {
            bytes: part.into_bytes(),
        });
    }
    t.pump();
    t.shot("transfer-received")?;
    let words = testkit::test_words(testkit::TEST_SEEDS[0].0);
    for part in envelope("seed-words.txt", words.as_bytes())? {
        t.app.event(Event::Scanned {
            bytes: part.into_bytes(),
        });
    }
    t.pump();
    t.shot("transfer-received-secret")?;
    t.press(Action::Cancel);
    t.shot("transfer-saved")?;
    t.press(Action::Nav(Screen::Home));
    t.shot("transfer-home")?;
    t.press(Action::Nav(Screen::Start));
    t.shot("transfer-wallets")
}

/// Home's Scan with a stick in: the sheet asks for it to be pulled, and
/// the camera opens when it is.
fn scan_tour(t: &mut Tour) -> Result<(), String> {
    t.sticks(true);
    t.press(Action::Nav(Screen::Home));
    t.press(Action::Scan);
    t.shot("home-pull-scan")?;
    t.sticks(false);
    if t.app.sheet != Some(faraday_core::Sheet::Scan) {
        return Err("pulling the stick did not open the camera".into());
    }
    t.shot("scan-after-pull")
}

/// Add a key, which is New key opening on Start (`docs/NEW-WALLET.md`
/// §15): the Start card, its Other forms, the Words card typed, the Key
/// card for words typed, locked in; then the codex32 tile's Words card.
fn add_key_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::keygen::Start;
    t.press(Action::Nav(Screen::Start));
    t.press(Action::KeyGen(None));
    t.shot("addkey-start")?;
    t.press(Action::KOthers);
    t.shot("addkey-start-others")?;
    t.press(Action::KOthers);
    t.press(Action::KStart(Start::Words.index()));
    t.press(Action::KNext);
    t.shot("addkey-words")?;
    let words = testkit::test_words(testkit::TEST_SEEDS[0].0);
    let cut = words.len() / 2;
    type_text(t, &words[..cut]);
    t.shot("addkey-words-typing")?;
    type_text(t, &words[cut..]);
    t.shot("addkey-words-typed")?;
    t.press(Action::KNext);
    t.shot("addkey-key")?;
    t.press(Action::KLock);
    t.shot("addkey-key-locked")?;
    t.press(Action::KNext);
    t.press(Action::Nav(Screen::Catalog));
    let codex32 = faraday_core::catalog::TILES
        .iter()
        .position(|tile| tile.name == "codex32")
        .ok_or("no codex32 tile")?;
    t.press(Action::Catalog(codex32 as u8));
    t.shot("addkey-codex32")
}

/// The small panel's Home, and Make a new wallet with a new key from dice,
/// a step to a page.
fn compact_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::cstep;
    t.shot("home")?;
    t.press(Action::Nav(Screen::Start));
    t.shot("wallets-empty")?;
    // Add a key with a stick in: the sheet, then Add a key once it is
    // pulled.
    t.sticks(true);
    t.press(Action::Nav(Screen::Home));
    t.shot("home-stick")?;
    t.press(Action::Entry(None));
    t.shot("home-pull")?;
    t.sticks(false);
    t.shot("entry-after-pull")?;
    // Its foot: Scan a SeedQR, Make a new key, and the way in from a stick.
    let _ = t.app.frame();
    t.app.event(Event::Scroll {
        x: 240,
        y: 200,
        dy: 1200,
    });
    t.shot("entry-foot")?;
    t.press(Action::Nav(Screen::Home));
    t.press(Action::CreateWallet);
    // Create opens on Kind, single key the default
    // (`docs/NEW-WALLET.md` §2.1).
    t.shot("create-kind")?;
    t.press(Action::CNext(cstep::KIND));
    t.shot("create-keys")?;
    t.press(Action::KeyGen(Some(0)));
    t.shot("keygen-length")?;
    t.press(Action::KWords(12));
    t.shot("keygen-source")?;
    t.press(Action::KNext);
    t.shot("keygen-rolls-none")?;
    let mut x: u32 = 0x2545_f491;
    for i in 0..128 {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        t.press(Action::KRoll((x % 6) as u8 + 1));
        if i == 20 {
            t.shot("keygen-rolls")?;
        }
    }
    // The randomness check is on the entry card once the last roll is
    // in; its Continue opens Key (`docs/NEW-WALLET.md` §3.2).
    t.shot("keygen-rolls-done")?;
    t.press(Action::KNext);
    t.shot("keygen-key")?;
    t.press(Action::KPassField(0));
    // Focused and empty: the No passphrase pill, the caret after it
    // (`docs/NEW-WALLET.md` §13.4).
    t.shot("keygen-key-passphrase-empty")?;
    type_text(t, "Ride the 7 bus");
    t.shot("keygen-key-passphrase")?;
    // Five of the dots selected with Shift and Left (§13.2).
    t.app.event(Event::Shift { held: true });
    for _ in 0..5 {
        t.app.event(Event::Key(Key::Left));
    }
    t.app.event(Event::Shift { held: false });
    t.shot("keygen-key-passphrase-selected")?;
    t.press(Action::KPassField(1));
    type_text(t, "Ride the 7 bus");
    // Lock in adds the key and, a single key being the whole wallet,
    // makes it: the Key card shows its descriptor and first address.
    t.press(Action::KLock);
    t.shot("keygen-key-locked")?;
    t.press(Action::KNext);
    t.shot("create-backup")?;
    t.press(Action::CStep(cstep::KEYS));
    t.shot("create-keys-filled")?;
    t.press(Action::CStep(cstep::CHECK));
    t.shot("create-check")?;
    t.press(Action::CStep(cstep::BACKUP));
    // Paper and vault: the backup on the new wallet, preset applied.
    t.press(Action::CBackup);
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    t.shot("create-backup-plan")?;
    t.press(Action::Nav(Screen::Home));
    t.shot("home-loaded")?;
    // Add a key: two words typed, the third begun; then a SLIP-39 share.
    t.press(Action::Entry(None));
    type_text(t, "zebra zebra zeb");
    t.shot("entry-typing")?;
    t.press(Action::EntryForm(1));
    type_text(t, "shadow pistol academic ac");
    t.shot("entry-slip39")?;
    t.press(Action::Nav(Screen::Home));
    seeds_tour(t)?;
    Ok(())
}

/// Restore a wallet made elsewhere (`docs/NEW-WALLET.md` §12): Kind with
/// the description shortcut (Scan it, From Files), Quorum at 2 of 3,
/// Savings' description from Files, then Seeds with test key 1 matched
/// to its key and a seed of no test wallet refused, the other two keys
/// Not here, Check and Done.
fn restore_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::rstep;
    use faraday_core::seeds::SeedsAction as S;
    t.press(Action::Network(testkit::NET));
    t.copy_wallets();
    t.sticks(false);
    t.press(Action::RestoreWallet);
    t.shot("restore-kind")?;
    t.press(Action::RKind(4));
    t.press(Action::RNext(rstep::KIND));
    t.shot("restore-quorum")?;
    let i = t
        .app
        .inbox
        .iter()
        .position(|it| it.name == "savings-wallet.txt")
        .ok_or("no savings-wallet.txt in Files")?;
    t.press(Action::RUse(i));
    // The question of vaults, in its turn before the seeds.
    t.press(Action::RVaults(false));
    t.press(Action::RNext(rstep::WHERE));
    t.press(Action::RSlotWords);
    t.type_key(0);
    t.shot("restore-seeds-matched")?;
    t.press(Action::RSlotWords);
    type_text(
        t,
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    );
    t.press(Action::EntryAdd);
    t.shot("restore-seeds-refused")?;
    let w = t
        .app
        .restore
        .as_ref()
        .and_then(|r| r.wallet)
        .ok_or("the description made no wallet")?;
    let empty: Vec<u8> = t
        .app
        .session
        .slots(&t.app.session.wallets[w])
        .iter()
        .enumerate()
        .filter(|(_, s)| s.held_by.is_none())
        .map(|(k, _)| k as u8)
        .collect();
    for k in empty {
        t.press(Action::RAbsent(k));
    }
    t.shot("restore-seeds-filled")?;
    t.press(Action::Seeds(S::Make));
    t.shot("restore-check")?;
    t.press(Action::RNext(rstep::CHECK));
    t.shot("restore-done")?;
    Ok(())
}

/// A wallet from seed words alone: Restore on a 2-of-3 with no
/// description, two seeds typed into their slots, the third cosigner's
/// xpub in the last, and the Check card;
/// then the Spend tab's What do you have? (`docs/SCAN-TO-SIGN.md` §4):
/// empty, after one seed, after two, the intake scanner asking for the
/// description, the shape made by hand, and Check the money guessed,
/// after No money shows (§5).
fn seeds_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::family::{FamilyAction as F, page};
    use faraday_core::seeds::{Focus, SLIDE_N, SeedsAction as S};
    t.press(Action::Lock);
    t.sticks(false);
    t.tick();
    t.press(Action::Network(testkit::NET));
    t.press(Action::RestoreWallet);
    t.press(Action::RKind(4));
    t.press(Action::RNext(faraday_core::rstep::KIND));
    t.press(Action::RNext(faraday_core::rstep::QUORUM));
    t.shot("restore-type-seeds")?;
    t.press(Action::RSeeds);
    t.press(Action::RVaults(false));
    t.press(Action::RNext(faraday_core::rstep::WHERE));
    for n in [0, 1] {
        t.press(Action::RSlotWords);
        t.type_key(n);
    }
    if t.app.screen != Screen::Restore {
        return Err("Add a key did not return to Restore".into());
    }
    t.shot("restore-seeds-first")?;
    t.press(Action::Seeds(S::Focus(Focus::Cosigner(0))));
    type_text(t, &testkit::key(2, "m/48'/1'/0'/2'"));
    t.app.event(Event::Key(Key::Enter));
    t.shot("restore-seeds-shape")?;
    t.press(Action::Seeds(S::Make));
    t.shot("restore-seeds-check")?;
    if t.app.restore.as_ref().and_then(|r| r.wallet).is_none() {
        return Err("the seeds made no wallet".into());
    }
    // The Spend tab: words, then another seed, then how many sign.
    t.press(Action::Lock);
    // The process after the lock has its clock from the next tick.
    t.tick();
    t.press(Action::Network(testkit::NET));
    t.press(Action::Nav(Screen::Family));
    t.press(Action::Family(F::StartOver));
    t.press(Action::Family(F::Next(page::MAP)));
    t.press(Action::Family(F::Next(page::SAFE)));
    t.shot("spend-intake")?;
    t.press(Action::Family(F::TypeWords));
    t.type_key(0);
    t.shot("spend-words-one-seed")?;
    t.press(Action::Family(F::TypeWords));
    t.type_key(1);
    t.shot("spend-words-two-seeds")?;
    t.press(Action::Family(F::Scan));
    t.shot("spend-intake-scanner")?;
    t.press(Action::Cancel);
    t.press(Action::Family(F::NoDescription));
    t.press(Action::Slide(SLIDE_N, 3));
    t.press(Action::Seeds(S::Focus(Focus::Cosigner(0))));
    type_text(t, &testkit::key(2, "m/48'/1'/0'/2'"));
    t.app.event(Event::Key(Key::Enter));
    // Down the shape to Make the wallet: further on a small panel.
    let steps = if t.size().0 < 700 { 5 } else { 2 };
    for k in 1..=steps {
        scroll(t, 300);
        t.shot(&format!("spend-words-shape-{k}"))?;
    }
    t.press(Action::Seeds(S::Make));
    t.shot("spend-words-made")?;
    if t.app.family_wallet().is_none() {
        return Err("the Spend tab's seeds made no wallet".into());
    }
    // Made without its description: No money shows tries the next kind.
    t.press(Action::Family(F::NoMoney));
    t.shot("spend-guess-multi")?;
    // One seed and That's everything: native SegWit, guessed.
    t.press(Action::Family(F::StartOver));
    t.press(Action::Family(F::TypeWords));
    t.type_key(0);
    t.press(Action::Family(F::Everything));
    t.shot("spend-guess-check")?;
    t.press(Action::Family(F::NoMoney));
    t.shot("spend-guess-passphrase")?;
    t.press(Action::Family(F::Passphrase(false)));
    t.shot("spend-guess-taproot")?;
    if t.app.family_guess().is_none_or(|g| g.at != 2) {
        return Err("No money shows did not go on to Taproot".into());
    }
    t.press(Action::Lock);
    Ok(())
}

/// Keys in no wallet on Wallets: a key beside the test kit's wallets,
/// with the column scrolled to its buttons; then one key added to an
/// empty session, the wallet made from it, and two keys.
fn keys_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::seeds::SeedsAction as S;
    let abandon = "abandon abandon abandon abandon abandon abandon \
                   abandon abandon abandon abandon abandon about";
    t.load_kit()?;
    t.press(Action::Entry(None));
    type_text(t, abandon);
    t.press(Action::EntryAdd);
    t.press(Action::Nav(Screen::Wallets));
    t.shot("wallets-kit-and-key")?;
    scroll(t, 1200);
    t.shot("wallets-kit-and-key-foot")?;
    t.press(Action::Lock);
    t.sticks(false);
    t.tick();
    t.press(Action::Entry(None));
    type_text(t, abandon);
    t.press(Action::EntryAdd);
    t.shot("wallets-one-key")?;
    let fp = t
        .app
        .session
        .keys
        .first()
        .map(|k| k.master().fingerprint().0)
        .ok_or("no key added")?;
    t.press(Action::KeyWallet(fp, 1));
    t.shot("from-key-shape")?;
    let steps = if t.size().0 < 700 { 5 } else { 2 };
    for k in 1..=steps {
        scroll(t, 300);
        t.shot(&format!("from-key-shape-{k}"))?;
    }
    t.press(Action::Nav(Screen::Wallets));
    t.press(Action::KeyWallet(fp, 2));
    t.shot("from-key-two")?;
    t.press(Action::Nav(Screen::Wallets));
    t.press(Action::Entry(None));
    t.type_key(1);
    t.press(Action::Nav(Screen::Wallets));
    t.shot("wallets-two-keys")?;
    t.press(Action::KeyWallet(fp, 1));
    t.press(Action::Seeds(S::Make));
    t.press(Action::Nav(Screen::Wallets));
    t.shot("wallets-one-wallet-one-key")?;
    // The small panel's wallet list, where the loose key has its row.
    t.press(Action::Nav(Screen::Start));
    t.shot("wallet-list-one-wallet-one-key")?;
    Ok(())
}

/// Stick visit with a wallet backup's 20 files in the Outbox and an
/// unprotected secret: the Outbox list at its top and scrolled to its end
/// by the wheel over it, then the stick's files at their end beside it.
/// On a small panel the page scrolls as one.
fn visit_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::wallet::classify;
    let stem = "family-savings";
    let mut names: Vec<String> = [
        "descriptor.txt",
        "wallet.json",
        "sparrow.txt",
        "coldcard.txt",
        "nunchuk.bsms",
        "core-import.txt",
        "backup.pdf",
    ]
    .iter()
    .map(|e| format!("{stem}-{e}"))
    .collect();
    for k in 1..=3 {
        names.push(format!("{stem}-share-{k}-of-3.pdf"));
        names.push(format!("{stem}-share-{k}-of-3.txt"));
        names.push(format!("{stem}-key-{k}-sheet.pdf"));
    }
    names.push(format!("{stem}-vault.ofv"));
    names.push(format!("{stem}-checklist.pdf"));
    names.push(format!("{stem}-recovery.txt"));
    for name in &names {
        let bytes = vec![b'#'; 1800];
        t.app.outbox.push(faraday_core::Item {
            name: name.clone(),
            kind: classify(c, name, &bytes),
            bytes,
            secret: false,
            picture: None,
            sha: None,
            wallet: None,
            psbt: None,
        });
    }
    t.app.outbox.push(faraday_core::Item {
        name: "seed-words.txt".into(),
        kind: faraday_core::wallet::FileKind::Text,
        bytes: b"words".to_vec(),
        secret: true,
        picture: None,
        sha: None,
        wallet: None,
        psbt: None,
    });
    t.sticks(true);
    t.press(Action::Nav(Screen::Visit));
    t.shot("visit-outbox")?;
    if t.app.is_compact() {
        // Down past the stick's files to the Outbox, then through it.
        for _ in 0..400 {
            let _ = t.app.frame();
            if t.app.offers(Action::VisitOutAll) {
                break;
            }
            scroll(t, 200);
            t.app.settle();
        }
        t.shot("visit-page-outbox")?;
        for k in 1..=4 {
            scroll(t, 500);
            t.shot(&format!("visit-page-outbox-{k}"))?;
        }
        return Ok(());
    }
    let _ = t.app.frame();
    let over = t
        .app
        .where_offered(Action::VisitOut(0))
        .ok_or("the Outbox's first file is not on screen")?;
    t.app.event(Event::Wheel {
        x: over.0,
        y: over.1,
        dy: 5000,
    });
    t.shot("visit-outbox-end")?;
    let over = t
        .app
        .where_offered(Action::VisitIn(0))
        .ok_or("the stick's first file is not on screen")?;
    t.app.event(Event::Wheel {
        x: over.0,
        y: over.1,
        dy: 5000,
    });
    t.shot("visit-both-ends")?;
    t.press(Action::VisitOutAll);
    t.press(Action::VisitOutAll);
    t.shot("visit-outbox-none")?;
    Ok(())
}

/// Stick visit with files copied in from another stick offered under
/// "From the Inbox".
fn inbox_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::wallet::FileKind as K;
    let item = |name: &str, kind: K, len: usize, picture: Option<K>| faraday_core::Item {
        name: name.into(),
        bytes: vec![b'#'; len],
        kind,
        secret: false,
        picture,
        sha: None,
        wallet: None,
        psbt: None,
    };
    t.app
        .outbox
        .push(item("savings-signed.psbt", K::Psbt, 1400, None));
    for i in [
        item("family-savings-descriptor.txt", K::Wallet, 620, None),
        item("household.ofv", K::Vault, 4_200_000, None),
        item("savings-photo-qr.png", K::Other, 2900, Some(K::Wallet)),
        item("seed-3-words.txt", K::Words, 160, None),
        item("recovery-codes.txt", K::Text, 210, None),
    ] {
        t.app.inbox.push(i);
    }
    t.sticks(true);
    t.press(Action::Nav(Screen::Visit));
    if t.app.is_compact() {
        // Down past the stick's files to the write list.
        for _ in 0..400 {
            let _ = t.app.frame();
            if t.app.offers(Action::VisitInbox(4)) {
                break;
            }
            scroll(t, 200);
            t.app.settle();
        }
    }
    t.shot("inbox-offered")?;
    t.press(Action::VisitOutAll);
    t.shot("inbox-select-all")?;
    t.press(Action::VisitInbox(4));
    t.shot("inbox-text-sheet")?;
    if t.app.is_compact() {
        // The small panel's sheet scrolls to its acknowledgement.
        scroll(t, 400);
        t.shot("inbox-text-sheet-foot")?;
    }
    t.press(Action::SecretAck);
    t.shot("inbox-text-ack")?;
    t.press(Action::SecretUnprotected);
    t.press(Action::VisitInbox(3));
    t.shot("inbox-words-sheet")?;
    t.press(Action::Cancel);
    t.shot("inbox-chosen")?;
    Ok(())
}

/// The SeedQR digits of the seed the backup shows, as a camera reads
/// them off the copy; with `wrong`, that word's number one off.
fn copy_digits(c: &Compute, t: &Tour, wrong: Option<usize>) -> Result<Vec<u8>, String> {
    let b = t.app.backup.as_ref().ok_or("no backup")?;
    let key = t.app.session.keys.get(b.key).ok_or("no key")?;
    let words = key.words.as_deref().ok_or("no words")?;
    let m = osk_bip::bip39::Mnemonic::parse(c, key.language, words.as_str())
        .map_err(|e| format!("{e}"))?;
    let mut digits = osk_codec::seedqr::to_digits(&m)
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
fn type_words(t: &mut Tour, count: Option<usize>) -> Result<(), String> {
    let b = t.app.backup.as_ref().ok_or("no backup")?;
    let key = t.app.session.keys.get(b.key).ok_or("no key")?;
    let words = key.words.as_deref().ok_or("no words")?.to_string();
    for w in words.split_whitespace().take(count.unwrap_or(usize::MAX)) {
        for c in w.chars().chain([' ']) {
            t.app.event(Event::Key(Key::Char(c)));
        }
    }
    Ok(())
}

/// Does each checklist item before `target` in the checklist's order,
/// as a person must (`docs/NEW-WALLET.md` §14.1): a copy by hand is
/// checked by its words typed back, the envelopes by their Continue.
fn items_before(c: &Compute, t: &mut Tour, target: u8) -> Result<(), String> {
    use faraday_core::bstep;
    while let Some(n) = t.app.backup_waiting(c) {
        if n == target {
            break;
        }
        if t.app.backup.as_ref().and_then(|b| b.open) != Some(n) {
            t.press(Action::BStep(n));
        }
        if bstep::is_copy(n) {
            t.press(Action::BCheck);
            type_words(t, None)?;
        }
        t.press(Action::BNext(n));
        if t.app.backup_waiting(c) == Some(n) {
            return Err(format!("checklist item {n} was not done in turn"));
        }
    }
    Ok(())
}

/// The plan the board on screen edits, its map.
fn bmap(c: &Compute, t: &Tour) -> Vec<Spot> {
    t.app
        .board_plan()
        .map(|(shape, a)| fplan::map(c, &shape, &a))
        .unwrap_or_default()
}

/// The wallet's shape, as the board on screen has it.
fn bshape(t: &Tour) -> Option<fplan::Shape> {
    t.app.board_plan().map(|(shape, _)| shape)
}

/// Tap to place on the board: `th` pressed in the list, then zone `z`.
fn bput(t: &mut Tour, th: Thing, z: Zone) {
    t.press(Action::Board(BP::Chip(Chip::List(th))));
    t.press(Action::Board(BP::Zone(z)));
}

/// The × on the placed chip of `th` at `z`.
fn boff(t: &mut Tour, z: Zone, th: Thing) {
    t.press(Action::Board(BP::Off(z, th)));
}

/// The stick at place `p` that carries vault `v`.
fn vault_stick(c: &Compute, t: &Tour, p: usize, v: usize) -> Option<usize> {
    let m = bmap(c, t);
    (0..fplan::sticks_at(&m, p)).find(|&s| fplan::stick_vault(&m, p, s) == Some(v))
}

/// The first place that keeps vault `v`'s stick.
fn vault_place(c: &Compute, t: &Tour, v: usize) -> Option<usize> {
    let places = t.app.board_plan().map_or(0, |(_, a)| a.places);
    (0..places).find(|&p| vault_stick(c, t, p, v).is_some())
}

/// Vault `v` kept at place `p`, holding exactly `seeds`, on the board.
fn keep_vault(c: &Compute, t: &mut Tour, p: usize, v: usize, seeds: &[usize]) {
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

/// Vault `v`'s stick taken off every place.
fn vault_off(c: &Compute, t: &mut Tour, v: usize) {
    let places = t.app.board_plan().map_or(0, |(_, a)| a.places);
    for p in 0..places {
        while let Some(s) = vault_stick(c, t, p, v) {
            boff(t, Zone::Files(p, s), Thing::Vault(v));
        }
    }
}

/// Every seed's words and SeedQR taken off every place's paper.
fn words_off(c: &Compute, t: &mut Tour) {
    let places = t.app.board_plan().map_or(0, |(_, a)| a.places);
    for p in 0..places {
        for w in fplan::holds_at(&bmap(c, t), At::Place(p)) {
            if let What::Words(i) | What::SeedQr(i) = w
                && fplan::holds_at(&bmap(c, t), At::Place(p)).contains(&w)
            {
                boff(t, Zone::Paper(p), Thing::Seed(i));
            }
        }
    }
}

/// Where `th` dropped on place `p`'s stick of files goes: its first stick
/// with no vault, or **+ stick**.
fn files_zone(c: &Compute, t: &Tour, p: usize) -> Zone {
    let m = bmap(c, t);
    (0..fplan::sticks_at(&m, p))
        .find(|&s| fplan::stick_vault(&m, p, s).is_none())
        .map_or(Zone::NewStick(p), |s| Zone::Files(p, s))
}

/// The whole description sealed in vault `v`, at the first place that
/// keeps its stick.
fn description_into_vault(c: &Compute, t: &mut Tour, v: usize) {
    if let Some(p) = vault_place(c, t, v)
        && let Some(s) = vault_stick(c, t, p, v)
    {
        bput(t, Thing::Whole, Zone::Vault(p, s, v));
    }
}

/// The whole description as a file on place `p`'s stick of files.
fn description_file_at(c: &Compute, t: &mut Tour, p: usize) {
    let z = files_zone(c, t, p);
    bput(t, Thing::Whole, z);
}

/// Every seed here as a words file at place `p`.
fn seed_files_at(c: &Compute, t: &mut Tour, p: usize) {
    for i in bshape(t).map(|s| s.here()).unwrap_or_default() {
        let z = files_zone(c, t, p);
        bput(t, Thing::Seed(i), z);
    }
}

/// Seed `i`'s passphrase sealed with it in each vault that holds it.
fn pass_into_vault(c: &Compute, t: &mut Tour, i: usize) {
    let vaults: Vec<usize> = bmap(c, t)
        .iter()
        .filter_map(|s| match s.at {
            At::Vault(v) if s.holds.iter().any(|(w, _)| *w == What::Seed(i)) => Some(v),
            _ => None,
        })
        .collect();
    for v in vaults {
        if let Some(p) = vault_place(c, t, v)
            && let Some(s) = vault_stick(c, t, p, v)
        {
            bput(t, Thing::Pass(i), Zone::Vault(p, s, v));
        }
    }
}

/// The description in shares on paper, each place with its own share in
/// place of the whole sheet.
fn shares_on_paper(c: &Compute, t: &mut Tour) {
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
fn to_checklist(t: &mut Tour, preset: faraday_core::plan::Preset, ticks: &[(u8, u8)]) {
    t.backup_as(preset);
    for &(list, row) in ticks {
        t.press(Action::BAnswer(list, row));
    }
    t.press(Action::BChecklist);
}

/// Scrolls the checklist until the open item's Continue is on screen.
fn scroll_to_open_next(t: &mut Tour) {
    if let Some(n) = t.app.backup.as_ref().and_then(|b| b.open) {
        scroll_to(t, Action::BNext(n));
    }
}

/// Scrolls the page until `action` is on screen, then a little further.
fn scroll_to(t: &mut Tour, action: Action) {
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

/// The backup's copy item checking the copy: with no camera, the typed
/// check; with one, Scan my copy, a copy with a word wrong on the sheet,
/// and the match back on the step.
/// Public files as codes and pictures: a signed message, Silent
/// payments, Create's keys and the backup's public files item for a
/// 2-of-3 made here, the QR sheet with its PNG, the backup's public files
/// item for Savings, and a GPG key made in the test vault.
fn public_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::Code;
    t.load_kit()?;
    // A signed message: For the stick as it is signed, as a code, as a
    // picture.
    t.press(Action::SignMessage);
    if let Some(m) = t.app.message.as_mut() {
        m.text = "Faraday signs this".to_string();
    }
    t.press(Action::MSign);
    t.press(Action::MStep(faraday_core::mstep::SIGN));
    scroll_to(t, Action::MQr);
    t.shot("public-message")?;
    t.press(Action::CodePng(Code::Message));
    // Silent payments: the record beside the address.
    t.press(Action::Silent);
    scroll_to(t, Action::CodePng(Code::SilentRecord));
    t.shot("public-silent")?;
    t.press(Action::SQr(false));
    t.shot("public-silent-sheet")?;
    t.press(Action::QrPng);
    t.press(Action::Cancel);
    t.press(Action::CreateWallet);
    t.press(Action::CKind(4));
    t.press(Action::CNext(0));
    t.press(Action::CNext(1));
    let bacon = test_key_1(t)?;
    t.press(Action::CSlotHere(0, bacon));
    device_key(t, 1);
    device_key(t, 2);
    scroll_to(t, Action::CodePng(Code::Key(0)));
    t.shot("public-keys")?;
    t.press(Action::CNext(faraday_core::cstep::KEYS));
    t.press(Action::CNext(faraday_core::cstep::CHECK));
    // The public files are the backup checklist's item; with every
    // software chosen and the description going as files, it offers
    // them all.
    let built = t.app.create.as_ref().and_then(|c| c.built).unwrap_or(0);
    t.press(Action::CBackup);
    t.backup_as(faraday_core::plan::Preset::PaperVaultSoftware);
    for row in 0..5u8 {
        let on = t
            .app
            .backup
            .as_ref()
            .is_some_and(|b| b.answers.software[usize::from(row)]);
        if !on {
            t.press(Action::BAnswer(faraday_core::qrow::SOFTWARE, row));
        }
    }
    // The description goes as files too, at Place 1: the public files
    // item.
    description_file_at(c, t, 0);
    t.press(Action::BChecklist);
    t.press(Action::BStep(faraday_core::bstep::PUBLIC));
    // Found while it is offered: once done, the row says so instead.
    scroll_to(t, Action::PublicOut(built, 8));
    t.press(Action::PublicOut(built, 8));
    t.shot("public-create")?;
    t.press(Action::CodePng(Code::MultisigConfig(built)));
    t.press(Action::CodePng(Code::Bsms(built)));
    scroll_to(t, Action::CodePng(Code::WalletKeyBsms(built, 0)));
    t.shot("public-create-keys")?;
    t.press(Action::CodePng(Code::WalletKey(built, 0)));
    t.press(Action::ShowCode(Code::MultisigConfig(built)));
    t.shot("public-sheet")?;
    t.press(Action::Cancel);
    t.press(Action::Backup(built));
    to_checklist(
        t,
        faraday_core::plan::Preset::PaperVaultSoftware,
        &[(qrow::FORM, 1)],
    );
    // Opened as the wallet's chart opens it: the items before it wait
    // for the copies and the vaults, which this tour does not make.
    t.press(Action::Chart(faraday_core::glance_sheet::ChartAction::Do(
        faraday_core::glance::Press::Loaded(built),
        faraday_core::glance_sheet::Step::Item(bstep::PUBLIC),
    )));
    t.press(Action::PublicOut(built, 8));
    scroll_to(t, Action::PublicOut(built, 8));
    t.shot("public-backup")?;
    // The files few people need, under More options (F64): the button,
    // then the rows it shows.
    scroll_to(t, Action::PublicMore(built));
    t.shot("public-more")?;
    t.press(Action::PublicMore(built));
    scroll_to(t, Action::PublicOut(built, 10));
    t.shot("public-more-open")?;
    // A GPG key made in the test vault: its public key and revocation,
    // and a signature, each as a code and a picture.
    let vault = testkit::files()?
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .ok_or("no test vault")?;
    let outbox: Vec<(String, Vec<u8>)> = t
        .app
        .outbox
        .iter()
        .map(|i| (i.name.clone(), i.bytes.clone()))
        .collect();
    t.app.storage(StorageEvent::Restored {
        inbox: vec![
            vault,
            (
                "SHA256SUMS".into(),
                b"3d6f  faraday-0.1.0-linux-x86_64\n".to_vec(),
            ),
        ],
        outbox,
        kept: Vec::new(),
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Open(0)));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..20 {
        t.tick();
    }
    if t.app.vaults.open.len() != 1 {
        return Err("the test vault did not open".into());
    }
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::Category(4)));
    flow_at_make(c, t, faraday_core::backup_flow::Kind::Gpg);
    type_text(t, "Test Person");
    t.app.event(Event::Key(Key::Tab));
    type_text(t, "test@example.com");
    flow_made(t, faraday_core::backup_flow::Kind::Gpg)?;
    // On a small panel the key's page is opened from the list.
    t.press(Action::Vault(V::Item(0)));
    scroll_to(t, Action::CodePng(Code::GpgKey));
    t.shot("public-gpg")?;
    t.press(Action::CodePng(Code::GpgKey));
    t.press(Action::Vault(V::GpgSignPick));
    // Back to the top of the list of files.
    for _ in 0..10 {
        scroll(t, -600);
    }
    t.shot("public-gpg-sign")?;
    t.press(Action::ShowCode(Code::GpgKey));
    t.shot("public-gpg-sheet")?;
    t.press(Action::Cancel);
    // The pictures made, beside the screens.
    for item in t.app.outbox.iter().filter(|i| i.name.ends_with(".png")) {
        let path = t.out.join(format!("outbox-{}", item.name));
        std::fs::write(&path, &item.bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("{}", path.display());
    }
    Ok(())
}

fn copy_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::board::{BoardPress, Thing, Zone};
    t.load_kit()?;
    t.press(Action::Backup(0));
    // The words and the SeedQR both copied by hand.
    t.backup_as(faraday_core::plan::Preset::Paper);
    t.press(Action::Board(BoardPress::Form(
        Zone::Paper(0),
        Thing::Seed(0),
        1,
    )));
    t.press(Action::BChecklist);
    t.press(Action::BNext(bstep::BLANK));
    t.press(Action::BReveal);
    t.app.storage(StorageEvent::Cameras(Vec::new()));
    scroll_to(t, Action::BCheck);
    t.shot("copy-no-camera")?;
    t.press(Action::BCheck);
    type_words(t, Some(3))?;
    t.shot("copy-typing")?;
    t.press(Action::BCheckClear);
    t.app.storage(StorageEvent::Cameras(vec![(
        "/dev/video0".into(),
        "Integrated Camera".into(),
    )]));
    type_words(t, None)?;
    scroll_to(t, Action::BScan);
    t.shot("copy-camera")?;
    t.press(Action::BScan);
    let wrong = copy_digits(c, t, Some(4))?;
    t.app.event(Event::Scanned { bytes: wrong });
    t.shot("copy-scan-differs")?;
    let right = copy_digits(c, t, None)?;
    t.app.event(Event::Scanned { bytes: right });
    scroll_to(t, Action::BScan);
    t.shot("copy-scan-matched")?;
    Ok(())
}

/// The backup's checklist offering a seed into a vault or as a file:
/// with no vault open, the step and the secret sheet; with the test
/// vault open, a seed it holds and one it does not, and the sheet; then
/// the seed's SeedQR, Standard and Compact, put in the Outbox past the
/// warning and written beside the screens.
fn seedfile_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    t.load_kit()?;
    t.press(Action::Entry(None));
    t.type_key(1);
    let second = t
        .app
        .session
        .keys
        .iter()
        .position(|k| {
            k.words
                .as_deref()
                .is_some_and(|w| w.as_str() == testkit::test_words(testkit::TEST_SEEDS[1].0))
        })
        .ok_or("test key 2 was not added")?;
    t.press(Action::Backup(0));
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    seed_files_at(c, t, 0);
    t.press(Action::BChecklist);
    t.press(Action::BStep(bstep::FILES));
    scroll_to(t, Action::BFile);
    t.shot("seedfile-no-vault")?;
    t.press(Action::BFile);
    t.shot("seedfile-sheet-no-vault")?;
    // On a small panel the sheet scrolls: its middle.
    scroll(t, 240);
    t.shot("seedfile-sheet-no-vault-down")?;
    t.press(Action::Cancel);
    // The test vault, unlocked: it holds test key 1, not test key 2.
    let vault = testkit::files()?
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .ok_or("no test vault")?;
    let inbox: Vec<(String, Vec<u8>)> = t
        .app
        .inbox
        .iter()
        .map(|i| (i.name.clone(), i.bytes.clone()))
        .chain(std::iter::once(vault))
        .collect();
    t.app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Open(0)));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..20 {
        t.tick();
    }
    if t.app.vaults.open.len() != 1 {
        return Err("the test vault did not open".into());
    }
    t.press(Action::Nav(Screen::Backup));
    t.press(Action::BStep(bstep::VAULT));
    t.shot("seedfile-in-vault")?;
    t.press(Action::BKey(second));
    t.shot("seedfile-vault")?;
    t.press(Action::BStep(bstep::FILES));
    t.press(Action::BFile);
    t.press(Action::SecretForm(1));
    t.press(Action::SecretAck);
    t.shot("seedfile-sheet-seedqr")?;
    scroll(t, 240);
    t.shot("seedfile-sheet-seedqr-down")?;
    t.press(Action::SecretUnprotected);
    t.press(Action::BCompact(true));
    t.press(Action::BFile);
    t.press(Action::SecretForm(1));
    t.press(Action::SecretAck);
    t.press(Action::SecretUnprotected);
    let pictures: Vec<_> = t
        .app
        .outbox
        .iter()
        .filter(|i| i.name.ends_with(".png"))
        .collect();
    if pictures.len() != 2 {
        return Err(format!("{} SeedQR pictures in the Outbox", pictures.len()));
    }
    for item in pictures {
        let path = t.out.join(format!("outbox-{}", item.name));
        std::fs::write(&path, &item.bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("{}", path.display());
    }
    Ok(())
}

/// Where each seed is kept, on the backup's panel (the seeds page on a
/// small panel): Savings with test key 1 here, its words put in the
/// Outbox unprotected and no vault; then, with the test vault open,
/// Spending, whose one key and wallet the vault holds, its copy typed
/// back and matched.
fn kept_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    t.load_kit()?;
    t.press(Action::Backup(0));
    t.backup_as(faraday_core::plan::Preset::Paper);
    seed_files_at(c, t, 0);
    t.press(Action::BChecklist);
    t.press(Action::BNext(bstep::BLANK));
    t.press(Action::BFile);
    t.press(Action::SecretAck);
    t.press(Action::SecretUnprotected);
    scroll_to_open_next(t);
    t.shot("kept-two-of-three")?;
    let vault = testkit::files()?
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .ok_or("no test vault")?;
    let inbox: Vec<(String, Vec<u8>)> = t
        .app
        .inbox
        .iter()
        .map(|i| (i.name.clone(), i.bytes.clone()))
        .chain(std::iter::once(vault))
        .collect();
    t.app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Open(0)));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..20 {
        t.tick();
    }
    if t.app.vaults.open.len() != 1 {
        return Err("the test vault did not open".into());
    }
    t.press(Action::Backup(1));
    to_checklist(t, faraday_core::plan::Preset::PaperVault, &[]);
    t.press(Action::BNext(bstep::BLANK));
    t.press(Action::BReveal);
    t.press(Action::BCheck);
    type_words(t, None)?;
    scroll_to_open_next(t);
    t.shot("kept-one-key")?;
    Ok(())
}

/// A loaded wallet backed up again: Savings loaded from its descriptor
/// file with no seed here opens on its public files, every one on offer;
/// a one-key wallet restored from test key 2 saves itself into the test
/// vault from its checklist; then the wallet chip's list of the two.
fn again_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::seeds::SeedsAction as S;
    t.copy_wallets();
    let savings = t
        .app
        .inbox
        .iter()
        .position(|it| it.name == "savings-wallet.txt")
        .ok_or("savings-wallet.txt is not in Files")?;
    t.press(Action::LoadWallet(savings));
    t.press(Action::Backup(0));
    t.shot("again-watch-only")?;
    let every = [
        (qrow::SOFTWARE, 0),
        (qrow::SOFTWARE, 1),
        (qrow::SOFTWARE, 2),
        (qrow::SOFTWARE, 3),
        (qrow::FORM, 1),
    ];
    to_checklist(t, faraday_core::plan::Preset::PaperVaultSoftware, &every);
    t.press(Action::BStep(bstep::PUBLIC));
    scroll_to(t, Action::PublicOut(0, 7));
    t.shot("again-watch-only-files")?;
    let vault = testkit::files()?
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .ok_or("no test vault")?;
    let inbox: Vec<(String, Vec<u8>)> = t
        .app
        .inbox
        .iter()
        .map(|i| (i.name.clone(), i.bytes.clone()))
        .chain(std::iter::once(vault))
        .collect();
    t.app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.press(Action::Entry(None));
    t.type_key(1);
    let fp = t
        .app
        .session
        .keys
        .last()
        .ok_or("test key 2 did not load")?
        .master()
        .fingerprint();
    t.press(Action::KeyWallet(fp.0, 1));
    t.press(Action::Seeds(S::Make));
    let w = t.app.session.wallets.len() - 1;
    t.press(Action::Backup(w));
    to_checklist(t, faraday_core::plan::Preset::PaperVault, &[]);
    t.press(Action::BStep(bstep::VAULT));
    scroll_to(t, Action::Vault(V::OpenFrom(0, Screen::Backup)));
    t.shot("again-restored-locked")?;
    t.app.vaults.ms_per_unit = Some(180);
    t.press(Action::Vault(V::OpenFrom(0, Screen::Backup)));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..20 {
        t.tick();
    }
    if t.app.vaults.open.len() != 1 || t.app.screen != Screen::Backup {
        return Err("the test vault did not open back to the backup".into());
    }
    scroll_to(t, Action::Vault(V::SaveWallet(w)));
    t.shot("again-restored-vault")?;
    t.press(Action::Vault(V::SaveWallet(w)));
    t.shot("again-restored-saved")?;
    // Back to the head, where the chip is.
    for _ in 0..10 {
        scroll(t, -120);
        t.app.settle();
    }
    t.press(Action::BWallets);
    t.shot("again-wallets")?;
    Ok(())
}

/// The backup's plan: for Spending (one key, test key 1 here), the
/// presets, the seeds and the places with the map beside them; for
/// Savings (2-of-3, one seed here), the seeds and the places, with the
/// whole sheet and then with its own share, the page What is a share?
/// opens, and on a small panel the map's own page; then the checklist
/// part done, and the blank template written beside the screens; then a
/// 3-of-5's shares on Places and on the checklist.
fn plan_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    t.load_kit()?;
    let find = |t: &Tour, name: &str| {
        t.app
            .session
            .wallets
            .iter()
            .position(|w| w.name == name)
            .ok_or(format!("no {name}"))
    };
    let spending = find(t, "Spending")?;
    let savings = find(t, "Savings")?;
    // The board (`docs/BACKUP-FLOW.md` §19): a one-key wallet on Paper
    // and vault, its board, Vault 1 held with the places that take it
    // edged, and The plan.
    t.press(Action::Backup(spending));
    t.shot("plan-board-read-only-one-key")?;
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    t.shot("plan-board-one-key")?;
    t.press(Action::Board(BP::Chip(Chip::List(Thing::Vault(0)))));
    t.shot("plan-board-vault-held-one-key")?;
    t.press(Action::Board(BP::LetGo));
    t.press(Action::BQ(qstep::PLAN));
    t.shot("plan-plan-one-key")?;
    scroll_to(t, Action::BChecklist);
    t.shot("plan-plan-one-key-foot")?;
    // Savings (2-of-3), no seed here: its board holds the vaults alone;
    // each place with a share on paper; What is a share? opens.
    t.press(Action::Backup(savings));
    t.backup_as(faraday_core::plan::Preset::Paper);
    t.shot("plan-board-two-of-three")?;
    shares_on_paper(c, t);
    t.shot("plan-board-shares-two-of-three")?;
    t.press(Action::LearnShares);
    t.shot("plan-learn-shares")?;
    t.press(Action::Cancel);
    t.press(Action::BQ(qstep::PLAN));
    t.shot("plan-map-two-of-three")?;
    t.press(Action::BChecklist);
    t.press(Action::BStep(bstep::SHEETS));
    t.shot("plan-checklist-part-done")?;
    t.press(Action::BStep(bstep::ENVELOPE));
    t.shot("plan-envelopes")?;
    // A 3-of-5's shares: up to two keys left off each, on the board and
    // on the checklist's Shares card.
    let five = t
        .app
        .session
        .add_wallet(
            c,
            "Five",
            &testkit::three_of_five(),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .map_err(|e| e.text())?;
    t.press(Action::Backup(five));
    t.backup_as(faraday_core::plan::Preset::Paper);
    shares_on_paper(c, t);
    t.press(Action::BOmit(2));
    t.shot("plan-board-shares-three-of-five")?;
    t.press(Action::BChecklist);
    t.press(Action::BStep(bstep::SHEETS));
    t.shot("plan-shares-three-of-five")?;
    // A vault per seed: Spending's one vault and its checklist item; then
    // Savings with test key 2 typed in too, two seeds here and a vault
    // each: the board, Vault 1's stick at Place 1 too, The plan and the
    // second vault's item.
    t.press(Action::Backup(spending));
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    t.press(Action::BChecklist);
    t.press(Action::BStep(bstep::VAULT));
    t.shot("plan-checklist-vault-one-key")?;
    t.press(Action::Entry(None));
    t.type_key(1);
    t.press(Action::Backup(savings));
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    t.shot("plan-board-vaults-two-of-three")?;
    if vault_stick(c, t, 0, 0).is_none() {
        bput(t, Thing::Vault(0), Zone::Card(0));
    }
    t.shot("plan-board-vault-linked-two-of-three")?;
    t.press(Action::BQ(qstep::PLAN));
    t.shot("plan-map-vaults-two-of-three")?;
    t.press(Action::BChecklist);
    t.press(Action::BStep(bstep::VAULT + 1));
    t.shot("plan-checklist-vaults-two-of-three")?;
    for item in t
        .app
        .outbox
        .iter()
        .filter(|i| i.name.starts_with("blank-template"))
    {
        let path = t.out.join(format!("outbox-{}", item.name));
        std::fs::write(&path, &item.bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("{}", path.display());
    }
    Ok(())
}

/// The backup board (`docs/BACKUP-FLOW.md` §19) for Savings, its three
/// keys typed in: the board on Paper and vault, Seed 1 selected with the
/// zones that take it edged, its words on Place 1's paper with their
/// forms open, a share on paper and in Vault 1, and the software place.
fn board_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::board::{BoardPress as B, Chip, Thing, Zone};
    let was = t.app.theme;
    t.load_kit()?;
    for n in 1..3 {
        t.press(Action::Entry(None));
        t.type_key(n);
    }
    let savings = t
        .app
        .session
        .wallets
        .iter()
        .position(|w| w.name == "Savings")
        .ok_or("no Savings")?;
    t.press(Action::Backup(savings));
    // The preset opens the board.
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    t.shot("board-paper-and-vault")?;
    t.press(Action::Board(B::Chip(Chip::List(Thing::Seed(0)))));
    t.shot("board-seed-selected")?;
    t.press(Action::Board(B::Zone(Zone::Paper(1))));
    t.press(Action::Board(B::Chip(Chip::Placed(
        Zone::Paper(1),
        Thing::Seed(0),
    ))));
    t.shot("board-seed-forms")?;
    t.press(Action::Board(B::Form(Zone::Paper(1), Thing::Seed(0), 1)));
    t.press(Action::Board(B::LetGo));
    t.press(Action::Board(B::Chip(Chip::List(Thing::Share(0)))));
    t.press(Action::Board(B::Zone(Zone::Paper(2))));
    t.press(Action::Board(B::Chip(Chip::List(Thing::Share(1)))));
    t.press(Action::Board(B::Zone(Zone::Vault(0, 0, 0))));
    t.press(Action::Board(B::Chip(Chip::List(Thing::Software))));
    t.shot("board-shares-and-software")?;
    // One stick with Vault 1 on it, a seed sealed inside, and beside the
    // vault on the same stick another seed's words file and SeedQR
    // picture; Paper under the stick box (§19.14): in the dark theme and
    // the light.
    let p = vault_place(c, t, 0).ok_or("Vault 1's stick has no place")?;
    let s = vault_stick(c, t, p, 0).ok_or("no stick for Vault 1")?;
    // The stick with Vault 1 and nothing beside it: its files box empty
    // (§19.17).
    scroll_to(t, Action::Board(B::Zone(Zone::Files(p, s))));
    t.shot("board-vault-no-files")?;
    bput(t, Thing::Seed(1), Zone::Files(p, s));
    t.press(Action::Board(B::Chip(Chip::Placed(
        Zone::Files(p, s),
        Thing::Seed(1),
    ))));
    t.press(Action::Board(B::Form(Zone::Files(p, s), Thing::Seed(1), 1)));
    t.press(Action::Board(B::LetGo));
    t.press(Action::Theme(faraday_core::ui::Theme::Dark));
    // In the narrow column the stick box is under Add and Place 1's head.
    if t.app.is_compact() {
        scroll(t, 260);
    }
    t.shot("board-vault-and-files-dark")?;
    t.press(Action::Theme(faraday_core::ui::Theme::Light));
    t.shot("board-vault-and-files-light")?;
    // Back to the theme the tour is in.
    t.press(Action::Theme(was));
    // + stick pressed with nothing held: an empty stick with its files
    // box; **+ vault**: a vault at no place, dim on the list (§19.16).
    t.press(Action::Board(B::Zone(Zone::NewStick(p))));
    scroll_to(t, Action::Board(B::RemoveEmpty(p)));
    t.shot("board-empty-stick")?;
    t.press(Action::Board(B::RemoveEmpty(p)));
    t.press(Action::Board(B::AddVault));
    t.shot("board-vault-at-no-place")?;
    // The narrow column: Add opens the list as a sheet; a chip pressed
    // there opens its menu, Add to… (§19.12 item 3, §19.20 item 4).
    if t.app.is_compact() {
        t.press(Action::Board(B::OpenList));
        t.shot("board-narrow-add-sheet")?;
        t.press(Action::Board(B::Chip(Chip::List(Thing::Seed(2)))));
        t.shot("board-narrow-seed-menu")?;
        t.press(Action::Board(B::MenuClose));
        t.press(Action::Cancel);
    }
    t.press(Action::BQ(qstep::PLAN));
    t.shot("board-the-plan")?;
    // A place's name, typed with a vault open: just pressed, the whole of
    // it selected; after one key, that key alone (§19.18).
    open_a_vault(t, "board phrase")?;
    t.press(Action::Backup(savings));
    if t.app.backup.as_ref().and_then(|b| b.q) != Some(qstep::BOARD) {
        t.press(Action::BQ(qstep::BOARD));
    }
    scroll_to(t, Action::BName(0));
    t.press(Action::BName(0));
    t.shot("board-name-pressed")?;
    type_text(t, "K");
    t.shot("board-name-one-key")?;
    t.app.event(Event::Key(Key::Enter));
    Ok(())
}

/// The ⋮ menu on the backup board (`docs/BACKUP-FLOW.md` §19.20) for
/// Savings, its three keys typed in, on Paper and vault, customised: on
/// `wide` a list chip's Add to…, a placed chip's menu and its Move to…,
/// and the menu opened by the keys; in the narrow column the Add sheet
/// with each chip's ⋮, a chip's Add to… over it, and a placed chip's menu.
fn menu_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::board::{BoardPress as B, Chip, Thing, Verb, Zone};
    t.load_kit()?;
    for n in 1..3 {
        t.press(Action::Entry(None));
        t.type_key(n);
    }
    let savings = t
        .app
        .session
        .wallets
        .iter()
        .position(|w| w.name == "Savings")
        .ok_or("no Savings")?;
    t.press(Action::Backup(savings));
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    t.press(Action::Board(B::Customize));
    let seed = Chip::List(Thing::Seed(0));
    // A chip placed on Place 1's paper, as the preset placed it or put
    // there now.
    let placed = Chip::Placed(Zone::Paper(0), Thing::Seed(0));
    let _ = t.app.frame();
    if !t.app.offers(Action::Board(B::Menu(placed))) {
        bput(t, Thing::Seed(0), Zone::Paper(0));
    }
    if t.app.is_compact() {
        t.press(Action::Board(B::OpenList));
        t.shot("menu-narrow-add-sheet")?;
        t.press(Action::Board(B::Menu(seed)));
        t.shot("menu-narrow-add-to")?;
        t.press(Action::Board(B::MenuClose));
        t.press(Action::Cancel);
        scroll_to(t, Action::Board(B::Menu(placed)));
        t.press(Action::Board(B::Menu(placed)));
        t.shot("menu-narrow-placed")?;
        t.press(Action::Board(B::MenuClose));
        return Ok(());
    }
    let _ = t.app.frame();
    t.press(Action::Board(B::Menu(seed)));
    t.shot("menu-add-to")?;
    t.press(Action::Board(B::MenuClose));
    scroll_to(t, Action::Board(B::Menu(placed)));
    t.press(Action::Board(B::Menu(placed)));
    t.shot("menu-placed")?;
    t.press(Action::Board(B::MenuVerb(Verb::Move)));
    t.shot("menu-move-to")?;
    t.press(Action::Board(B::MenuClose));
    // By the keys: Tab to the first ⋮, Enter, and Down twice.
    let _ = t.app.frame();
    t.app.event(Event::Key(Key::Tab));
    t.shot("menu-dot-focused")?;
    t.app.event(Event::Key(Key::Enter));
    t.app.event(Event::Key(Key::Down));
    t.app.event(Event::Key(Key::Down));
    t.shot("menu-keys")?;
    t.app.event(Event::Key(Key::Escape));
    Ok(())
}

/// A vault made with `phrase` and unlocked, as a person makes one from
/// Vaults.
fn open_a_vault(t: &mut Tour, phrase: &str) -> Result<(), String> {
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Create));
    for second in [false, true] {
        t.press(Action::Vault(V::CFocus(0, second)));
        type_text(t, phrase);
    }
    t.press(Action::Vault(V::CGo));
    for _ in 0..10 {
        if t.app.screen == Screen::Unlock {
            break;
        }
        t.tick();
    }
    type_text(t, phrase);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..10 {
        if !t.app.vaults.open.is_empty() {
            break;
        }
        t.tick();
    }
    if t.app.vaults.open.is_empty() {
        return Err("the vault made for the tour did not open".to_string());
    }
    Ok(())
}

/// The Wallets card at a glance: a one-key wallet with no plan, then with
/// Paper and vault; a 2-of-3 with test keys 1 and 2 here, key 2 with a
/// passphrase, and test key 3 a cosigner, with Paper and vault and key
/// 2's passphrase in its vault too. On a small panel, each card's At a
/// glance row and the chart's page. The sheets a press on the chart opens
/// (`docs/NEW-WALLET.md` §9): the wallet, a key here, a cosigner's key, a
/// place, a vault, a line inside a place; Where it is; and in the open
/// vault's view, a place's.
fn glance_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::create::NewKind;
    use faraday_core::glance::Press;
    use faraday_core::glance_sheet::{ChartAction as C, Target};
    let shot_sheet = |t: &mut Tour, press: Press, target: Target, name: &str| {
        t.press(Action::Chart(C::Open(press, target)));
        let shot = t.shot(name);
        t.press(Action::Cancel);
        shot
    };
    t.press(Action::Entry(None));
    t.type_key(0);
    let one = {
        let text = NewKind::NativeSegwit
            .key_text(c, t.app.session.keys[0].master())
            .map_err(|e| e.to_string())?;
        t.app
            .session
            .add_wallet(
                c,
                "Spending",
                &format!("wpkh({text}/<0;1>/*)"),
                "test",
                faraday_core::wallet::Origin::Named,
            )
            .map_err(|e| e.text())?
    };
    let shot_card = |t: &mut Tour, w: usize, name: &str| -> Result<(), String> {
        t.press(Action::OpenWallet(w));
        t.shot(name)?;
        if t.app.is_compact() {
            t.press(Action::Glance(true));
            t.shot(&format!("{name}-page"))?;
            for k in 1..=3 {
                scroll(t, 500);
                t.shot(&format!("{name}-page-{k}"))?;
            }
            t.press(Action::Glance(false));
        } else {
            scroll(t, 400);
            t.shot(&format!("{name}-foot"))?;
        }
        Ok(())
    };
    shot_card(t, one, "glance-no-plan")?;
    t.press(Action::Backup(one));
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    t.press(Action::BChecklist);
    shot_card(t, one, "glance-one-key")?;
    let press = Press::Loaded(one);
    t.press(Action::OpenWallet(one));
    shot_sheet(t, press, Target::Wallet, "glance-sheet-wallet")?;
    shot_sheet(t, press, Target::Key(0), "glance-sheet-key")?;
    shot_sheet(t, press, Target::Node(0), "glance-sheet-place")?;
    shot_sheet(t, press, Target::Node(2), "glance-sheet-vault")?;
    shot_sheet(t, press, Target::Line(0, 0), "glance-sheet-words")?;
    // Test key 2 with a passphrase, test key 3 a cosigner's.
    t.app
        .session
        .add_words_with(
            c,
            &testkit::test_words(testkit::TEST_SEEDS[1].0),
            "glance",
            "Test key 2",
            None,
        )
        .map_err(|e| e.text())?;
    let mut elsewhere = faraday_core::wallet::Session::default();
    elsewhere
        .add_words(
            c,
            &testkit::test_words(testkit::TEST_SEEDS[2].0),
            "Test key 3",
            None,
        )
        .map_err(|e| e.text())?;
    let masters = [
        &t.app.session.keys[0].master(),
        &t.app.session.keys[1].master(),
        &elsewhere.keys[0].master(),
    ];
    let keys: Vec<String> = masters
        .iter()
        .map(|m| NewKind::Multi.key_text(c, m))
        .collect::<Result<_, _>>()?;
    let two = t
        .app
        .session
        .add_wallet(
            c,
            "Savings",
            &NewKind::Multi.descriptor(2, &keys),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .map_err(|e| e.text())?;
    shot_card(t, two, "glance-two-of-three-no-plan")?;
    t.press(Action::Backup(two));
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    // Key 2's passphrase into its vault too.
    pass_into_vault(c, t, 1);
    t.press(Action::BChecklist);
    shot_card(t, two, "glance-two-of-three")?;
    let press = Press::Loaded(two);
    t.press(Action::OpenWallet(two));
    shot_sheet(t, press, Target::Key(2), "glance-sheet-cosigner")?;
    if !t.app.is_compact() {
        t.press(Action::Chart(C::Where(press, 1)));
        t.shot("glance-where")?;
    }
    // The open vault's view (§7.2): a vault made and open, the one-key
    // wallet's plan made with it, its seed and the wallet saved into it;
    // the wallet's chart from the backup up.
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Create));
    for second in [false, true] {
        t.press(Action::Vault(V::CFocus(0, second)));
        type_text(t, "glance phrase");
    }
    t.press(Action::Vault(V::CGo));
    for _ in 0..10 {
        if t.app.screen == Screen::Unlock {
            break;
        }
        t.tick();
    }
    type_text(t, "glance phrase");
    t.press(Action::Vault(V::Unlock));
    for _ in 0..10 {
        if !t.app.vaults.open.is_empty() {
            break;
        }
        t.tick();
    }
    if t.app.vaults.open.is_empty() {
        return Err("the vault made for the glance tour did not open".to_string());
    }
    t.press(Action::Backup(one));
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    t.press(Action::BChecklist);
    t.press(Action::BVaultSave(0));
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::AddKind(1)));
    t.press(Action::Vault(V::SaveWallet(one)));
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::Category(1)));
    t.press(Action::Vault(V::Item(0)));
    let shot_item = |t: &mut Tour, name: &str| -> Result<(), String> {
        t.shot(name)?;
        if t.app.is_compact() {
            for k in 1..=3 {
                scroll(t, 500);
                t.shot(&format!("{name}-{k}"))?;
            }
        } else {
            // The item's own region, right of the list.
            let _ = t.app.frame();
            let (w, h) = t.size();
            t.app.event(Event::Scroll {
                x: w - w / 5,
                y: h / 2,
                dy: 600,
            });
            t.shot(&format!("{name}-foot"))?;
        }
        Ok(())
    };
    shot_item(t, "glance-vault")?;
    // The 2-of-3's plan made with the vault open, and the wallet saved.
    t.press(Action::Backup(two));
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    // Key 2's passphrase into its vault too.
    pass_into_vault(c, t, 1);
    t.press(Action::BChecklist);
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::AddKind(1)));
    t.press(Action::Vault(V::SaveWallet(two)));
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::Category(1)));
    t.press(Action::Vault(V::Item(1)));
    shot_item(t, "glance-vault-two-of-three")?;
    if let Some(r) = t.app.vault_selected_index() {
        t.press(Action::Vault(V::Item(1)));
        shot_sheet(
            t,
            Press::Vault(t.app.vaults.current, r),
            Target::Node(0),
            "glance-vault-sheet-place",
        )?;
    }
    Ok(())
}

/// The chart's changes (`docs/NEW-WALLET.md` §9.5–§9.7), on a one-key
/// wallet with a vault open and the Paper and vault plan made with it:
/// the vault's stick moved to Place 1, asked with the check before and
/// after, and the chart once moved; the words at Place 2 destroyed,
/// asked, a line made worse; a line marked lost and a place marked
/// exposed, with what follows; the vault copied to a stick at a new
/// place; and on a 2-of-3, the cosigner's holder typed and the card with
/// it.
fn chart_edit_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::create::NewKind;
    use faraday_core::glance::{Backup, Press};
    use faraday_core::glance_sheet::{ChartAction as C, Target};
    use faraday_core::plan::{At, Edit, Mark, What};
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    t.press(Action::Entry(None));
    t.type_key(0);
    let one = {
        let text = NewKind::NativeSegwit
            .key_text(c, t.app.session.keys[0].master())
            .map_err(|e| e.to_string())?;
        t.app
            .session
            .add_wallet(
                c,
                "Spending",
                &format!("wpkh({text}/<0;1>/*)"),
                "test",
                faraday_core::wallet::Origin::Named,
            )
            .map_err(|e| e.text())?
    };
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Create));
    for second in [false, true] {
        t.press(Action::Vault(V::CFocus(0, second)));
        type_text(t, "chart phrase");
    }
    t.press(Action::Vault(V::CGo));
    for _ in 0..10 {
        if t.app.screen == Screen::Unlock {
            break;
        }
        t.tick();
    }
    type_text(t, "chart phrase");
    t.press(Action::Vault(V::Unlock));
    for _ in 0..10 {
        if !t.app.vaults.open.is_empty() {
            break;
        }
        t.tick();
    }
    if t.app.vaults.open.is_empty() {
        return Err("the vault made for the chart tour did not open".to_string());
    }
    t.press(Action::Backup(one));
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    t.press(Action::BChecklist);
    t.press(Action::BVaultSave(0));
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::AddKind(1)));
    t.press(Action::Vault(V::SaveWallet(one)));
    let press = Press::Loaded(one);
    t.press(Action::OpenWallet(one));
    // The card, or on a small panel the chart's own page.
    let shot_chart = |t: &mut Tour, w: usize, name: &str| -> Result<(), String> {
        t.press(Action::OpenWallet(w));
        if t.app.is_compact() {
            t.press(Action::Glance(true));
            t.shot(name)?;
            scroll(t, 500);
            t.shot(&format!("{name}-2"))?;
            t.press(Action::Glance(false));
        } else {
            t.shot(name)?;
            scroll(t, 400);
            t.shot(&format!("{name}-foot"))?;
        }
        Ok(())
    };
    // A move: the vault's stick to Place 1, asked, then made.
    let stick = Edit::Move(At::Place(1), What::VaultStick(0), At::Place(0));
    t.press(Action::Chart(C::Edit(press, stick)));
    t.shot("chart-move-ask")?;
    t.press(Action::Chart(C::Confirm(press)));
    shot_chart(t, one, "chart-moved")?;
    // The words at Place 1 destroyed: asked, a line worse.
    t.press(Action::Chart(C::Edit(
        press,
        Edit::Drop(At::Place(0), What::Words(0)),
    )));
    t.shot("chart-drop-ask")?;
    t.press(Action::Cancel);
    let node = |t: &Tour, at: At| -> Option<(u8, Vec<What>)> {
        let g = faraday_core::glance::of_now(c, &t.app, one)?;
        let Backup::Plan { nodes, .. } = g.backup else {
            return None;
        };
        nodes
            .iter()
            .position(|n| n.at == at)
            .map(|i| (i as u8, nodes[i].whats.clone()))
    };
    // The words at Place 2 marked lost: they are at Place 1 too.
    if let Some((n, whats)) = node(t, At::Place(1))
        && let Some(j) = whats.iter().position(|w| *w == What::Words(0))
    {
        t.press(Action::Chart(C::Mark(press, n, Some(j as u8), Mark::Lost)));
        t.shot("chart-lost")?;
        t.press(Action::Cancel);
    }
    // Place 1, with the words and the vault's stick, marked exposed.
    if let Some((n, _)) = node(t, At::Place(0)) {
        t.press(Action::Chart(C::Mark(press, n, None, Mark::Exposed)));
        t.shot("chart-exposed")?;
        t.press(Action::Cancel);
    }
    shot_chart(t, one, "chart-marked")?;
    // The vault copied to a stick at a new place.
    t.press(Action::Chart(C::Open(
        press,
        Target::CopyTo(What::VaultStick(0)),
    )));
    t.shot("chart-vault-copy-to")?;
    t.press(Action::Chart(C::Edit(
        press,
        Edit::Add(At::Place(2), What::VaultStick(0)),
    )));
    t.shot("chart-vault-copy-ask")?;
    t.press(Action::Chart(C::Confirm(press)));
    t.press(Action::Cancel);
    shot_chart(t, one, "chart-vault-copy")?;
    // A 2-of-3 with test key 3 a cosigner's: its holder named.
    t.press(Action::Entry(None));
    t.type_key(1);
    let mut elsewhere = faraday_core::wallet::Session::default();
    elsewhere
        .add_words(
            c,
            &testkit::test_words(testkit::TEST_SEEDS[2].0),
            "Test key 3",
            None,
        )
        .map_err(|e| e.text())?;
    let masters = [
        &t.app.session.keys[0].master(),
        &t.app.session.keys[1].master(),
        &elsewhere.keys[0].master(),
    ];
    let keys: Vec<String> = masters
        .iter()
        .map(|m| NewKind::Multi.key_text(c, m))
        .collect::<Result<_, _>>()?;
    let two = t
        .app
        .session
        .add_wallet(
            c,
            "Savings",
            &NewKind::Multi.descriptor(2, &keys),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .map_err(|e| e.text())?;
    t.press(Action::Backup(two));
    t.backup_as(faraday_core::plan::Preset::Paper);
    t.press(Action::BChecklist);
    let press = Press::Loaded(two);
    t.press(Action::OpenWallet(two));
    t.press(Action::Chart(C::Open(press, Target::Holder(2))));
    type_text(t, "Alice's Coldcard");
    t.shot("chart-holder")?;
    t.press(Action::Chart(C::Save(press)));
    t.press(Action::Chart(C::KeyChecked(press, 2)));
    t.press(Action::Chart(C::Open(press, Target::Key(2))));
    t.shot("chart-holder-key")?;
    t.press(Action::Cancel);
    shot_chart(t, two, "chart-holder-card")?;
    Ok(())
}

/// Tools' GPG key with no vault file, Create a vault made for it, and
/// back; then with the test vault locked, the tile again, Unlock for it,
/// and the GPG keys it comes back to.
fn vaultway_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::catalog::TILES;
    let gpg = TILES
        .iter()
        .position(|tile| tile.name == "GPG key")
        .ok_or("no GPG key tile")? as u8;
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    // On a small panel the tile is found by typing, as far down as it is.
    let find = |t: &mut Tour| {
        t.press(Action::Nav(Screen::Catalog));
        if t.app.is_compact() {
            t.app.event(Event::Key(Key::Escape));
            type_text(t, "gpg");
        } else {
            scroll_to(t, Action::Catalog(gpg));
        }
    };
    find(t);
    t.shot("tools-no-vault")?;
    t.press(Action::Catalog(gpg));
    t.shot("create-then-gpg")?;
    t.press(Action::Vault(V::Back));
    let vault = testkit::files()?
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .ok_or("no test vault")?;
    t.app.storage(StorageEvent::Restored {
        inbox: vec![vault],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    find(t);
    t.shot("tools-vault-locked")?;
    t.press(Action::Catalog(gpg));
    t.shot("unlock-for-gpg")?;
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..20 {
        t.tick();
    }
    if t.app.screen != Screen::VaultContents {
        return Err("unlocking did not come back to the GPG keys".into());
    }
    t.shot("unlocked-gpg")?;
    Ok(())
}

/// The backup before spending (`docs/NEW-WALLET.md` §14): a single-key
/// wallet made here, its card with Finish the backup first and the
/// receive warning; Places with Vault 1's stick unticked; the vault item
/// with no vault open, with one made and open, and saved into; the end of
/// the backup with the checks strip and the chart.
fn first_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::create::NewKind;
    use faraday_core::plan::Preset;
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    let kind = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::NativeSegwit)
        .ok_or("no native SegWit")? as u8;
    t.press(Action::CreateWallet);
    t.press(Action::CKind(kind));
    t.press(Action::CNext(faraday_core::cstep::KIND));
    device_key(t, 0);
    let w = t
        .app
        .create
        .as_ref()
        .and_then(|c| c.built)
        .ok_or("the wallet was not made")?;
    t.press(Action::OpenWallet(w));
    t.shot("first-card-held")?;
    // The plan: Paper and vault; Vault 1's stick unticked on Places.
    t.press(Action::BackupFirst(w));
    t.backup_as(Preset::PaperVault);
    // The board, Vault 1 on the place that keeps its stick.
    vault_place(c, t, 0).ok_or("Vault 1's stick has no place")?;
    t.shot("first-place-vault-board")?;
    t.press(Action::BChecklist);
    // The copy checked, typed back.
    t.press(Action::BCheck);
    type_words(t, None)?;
    t.press(Action::BStep(bstep::VAULT));
    scroll_to(t, Action::Vault(V::CreateFrom(Screen::Backup)));
    t.shot("first-vault-make")?;
    t.press(Action::Vault(V::CreateFrom(Screen::Backup)));
    for second in [false, true] {
        t.press(Action::Vault(V::CFocus(0, second)));
        type_text(t, "first phrase");
    }
    t.press(Action::Vault(V::CGo));
    for _ in 0..10 {
        if t.app.screen == Screen::Unlock {
            break;
        }
        t.tick();
    }
    type_text(t, "first phrase");
    t.press(Action::Vault(V::Unlock));
    for _ in 0..10 {
        if !t.app.vaults.open.is_empty() && t.app.screen == Screen::Backup {
            break;
        }
        t.tick();
    }
    if t.app.vaults.open.is_empty() {
        return Err("the vault made for the first tour did not open".to_string());
    }
    scroll_to(t, Action::BVaultSave(0));
    t.shot("first-vault-save")?;
    t.press(Action::BVaultSave(0));
    scroll_to(t, Action::BNext(bstep::VAULT));
    t.shot("first-vault-saved")?;
    t.press(Action::BNext(bstep::VAULT));
    if t.app.backup.as_ref().and_then(|b| b.open) != Some(bstep::ENVELOPE) {
        t.press(Action::BStep(bstep::ENVELOPE));
    }
    t.press(Action::BNext(bstep::ENVELOPE));
    let done = t
        .app
        .backup_items_now(c)
        .into_iter()
        .filter_map(bstep::item)
        .all(|it| t.app.backup_item_done_now(c, it));
    if !done {
        return Err("the first tour's checklist is not done".to_string());
    }
    t.shot("first-done")?;
    if t.app.is_compact() {
        // The done card's top: the checks strip, then the chart.
        scroll(t, 420);
        t.shot("first-done-strip")?;
    }
    scroll_to(t, Action::BPlan);
    t.shot("first-done-foot")?;
    Ok(())
}

fn themes_tour(t: &mut Tour) -> Result<(), String> {
    t.load_kit()?;
    for theme in faraday_core::ui::Theme::ALL {
        t.press(Action::Theme(theme));
        t.press(Action::Nav(Screen::Start));
        t.shot(&format!("theme-{}-wallets", theme.id()))?;
        t.press(Action::Nav(Screen::Wallets));
        t.shot(&format!("theme-{}-list", theme.id()))?;
        t.press(Action::Nav(Screen::Settings));
        t.shot(&format!("theme-{}-settings", theme.id()))?;
    }
    Ok(())
}

/// From the vault to a spend (`docs/NEW-WALLET.md` §11): the boot
/// stick's vault unlocked after the pull lands on Wallets with a toast;
/// a wallet's card leads with Spend from this wallet, which opens the
/// Spend tab on Load the wallet in Sparrow with the shortcut; a 2-of-3
/// with one key here, its PSBT in Files, reaches Signers with Scan a key
/// and Collect a signature; the card's Scan a key opens the key scanner,
/// and a missing key's SeedQR makes the wallet able to spend.
fn from_vault_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::boot_import::ImportAction as I;
    use faraday_core::family::{FamilyAction as F, page};
    t.press(Action::Lock);
    // The vault and the one PSBT Sparrow wrote, for Savings; no seed
    // beside them, so Savings has only the vault's key here.
    let mut boot = t.stick();
    boot.boot = true;
    boot.label = "FARADAY".into();
    boot.files
        .retain(|(n, _)| n == "vault.ofv" || n == "savings-unsigned.psbt");
    t.app.storage(StorageEvent::Sticks(vec![boot]));
    t.pump();
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.press(Action::Import(I::Go));
    t.press(Action::Vault(V::FocusPassphrase));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.press(Action::Import(I::Submit));
    t.tick();
    if t.app.screen != Screen::Wallets {
        return Err(format!("the unlock landed on {:?}", t.app.screen));
    }
    t.shot_after("vault-landing", 0)?;
    let at = |t: &Tour, name: &str| {
        t.app
            .session
            .wallets
            .iter()
            .position(|w| w.name == name)
            .ok_or_else(|| format!("{name} is not loaded"))
    };
    let spending = at(t, "Spending")?;
    t.press(Action::OpenWallet(spending));
    t.shot("vault-card-spend")?;
    t.press(Action::Family(F::SpendFrom(spending)));
    t.shot("vault-spend-first")?;
    t.press(Action::Family(F::Next(page::CHECK)));
    t.shot("vault-spend-write")?;
    // A wallet of several keys with fewer here than sign, and its PSBT.
    let short = (0..t.app.session.wallets.len()).find(|&w| {
        let wl = &t.app.session.wallets[w];
        let here = t
            .app
            .session
            .slots(wl)
            .iter()
            .filter(|s| s.held_by.is_some())
            .count();
        here < faraday_core::wallet::needed(wl) && t.app.psbt_for(c, w).is_some()
    });
    let Some(short) = short else {
        return Err("no wallet short of keys has its PSBT in Files".into());
    };
    t.press(Action::OpenWallet(short));
    t.shot("vault-card-sign")?;
    t.press(Action::Family(F::SpendFrom(short)));
    t.press(Action::Step(faraday_core::wallet::step::SIGNERS));
    t.shot("vault-spend-signers")?;
    // The card's Scan a key: the key scanner, then a missing key's
    // SeedQR, which loads at once, and the card can spend.
    t.press(Action::OpenWallet(short));
    t.press(Action::ScanKey(short));
    t.app.event(Event::CameraFrame {
        width: 640,
        height: 480,
        luma: (0..640u32 * 480).map(|i| ((i % 640) / 3) as u8).collect(),
        chroma: None,
    });
    t.shot("wallet-scan-key")?;
    let missing: Vec<[u8; 4]> = {
        let wl = &t.app.session.wallets[short];
        t.app
            .session
            .slots(wl)
            .iter()
            .filter(|s| s.held_by.is_none())
            .filter_map(|s| s.fingerprint.map(|f| f.0))
            .collect()
    };
    let words = testkit::TEST_SEEDS
        .iter()
        .map(|(w, _)| testkit::test_words(w))
        .find(|w| {
            let mut s = testkit::session();
            s.add_words(c, w, "", None)
                .is_ok_and(|fp| missing.contains(&fp.0))
        })
        .ok_or("no test seed is a missing key")?;
    let m = osk_bip::bip39::Mnemonic::parse(c, osk_bip::bip39::Language::English, &words)
        .map_err(|e| format!("{e:?}"))?;
    let digits = osk_codec::seedqr::to_digits(&m);
    t.app.event(Event::Scanned {
        bytes: digits.expose().as_bytes().to_vec(),
    });
    t.pump();
    if !t.app.can_spend(short) {
        return Err("the scanned key did not make the wallet able to spend".into());
    }
    t.shot("vault-card-can-spend")?;
    println!("Scan a key on a wallet's card loaded its missing key at once");
    Ok(())
}

fn spend_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::family::{FamilyAction as F, page};
    let fam = |a: F| Action::Family(a);
    t.press(Action::Lock);
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    t.press(Action::Nav(Screen::Family));
    t.shot("spend-map")?;
    t.press(fam(F::More(page::MAP)));
    t.shot("spend-map-more")?;
    t.press(fam(F::Next(page::MAP)));
    t.shot("spend-stick")?;
    t.press(fam(F::Next(page::SAFE)));
    t.press(fam(F::Unsure));
    t.shot("spend-holding-unsure")?;
    t.press(fam(F::Unsure));
    // The vault and the transaction from a stick: the visit, and back.
    t.press(Action::Nav(Screen::Family));
    t.sticks(true);
    t.press(Action::VisitInAll);
    for name in ["vault.ofv", "savings-unsigned.psbt"] {
        if let Some(i) = t.stick().files.iter().position(|(n, _)| n == name) {
            t.press(Action::VisitIn(i));
        }
    }
    t.press(Action::VisitCopy);
    t.shot("spend-visit")?;
    t.sticks(false);
    if t.app.screen != Screen::Family {
        return Err("pulling the stick did not return to the Spend tab".into());
    }
    t.press(fam(F::Pick(0)));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.shot("spend-vault-passphrase")?;
    t.press(fam(F::Unlock));
    t.tick();
    t.tick();
    t.shot("spend-vault-list")?;
    t.press(Action::Vault(V::LoadChosen(0)));
    t.shot("spend-vault-opened")?;
    let savings = t
        .app
        .session
        .wallets
        .iter()
        .position(|w| w.name == "Savings")
        .ok_or("the vault did not load Savings")?;
    // The PSBT in Files chose Savings: What do you have? is done.
    if t.app.family_wallet() != Some(savings) {
        return Err("the PSBT in Files did not choose Savings".into());
    }
    t.shot("spend-check")?;
    t.press(fam(F::Next(page::CHECK)));
    t.shot("spend-write")?;
    t.press(fam(F::Next(page::WRITE)));
    t.shot("spend-bring")?;
    let at = t
        .app
        .inbox
        .iter()
        .position(|i| i.name == "savings-unsigned.psbt")
        .ok_or("the PSBT is not in Files")?;
    t.press(fam(F::UsePsbt(at)));
    t.shot("spend-transaction")?;
    // Check is already closed: "Yes, my money shows" on the Spend tab's
    // own check page marked this wallet checked this power-on.
    t.press(Action::StepNext(faraday_core::wallet::step::TRANSACTION));
    t.shot("spend-signers")?;
    // Who signs' Scan a key, and its Type the words instead.
    t.press(Action::ScanKey(savings));
    t.press(Action::EntryWallet(savings));
    t.type_key(1);
    t.shot("spend-signers-two")?;
    t.press(Action::StepNext(faraday_core::wallet::step::SIGNERS));
    t.press(Action::SignHere);
    t.shot("spend-signed")?;
    t.press(Action::StepNext(faraday_core::wallet::step::SIGN));
    t.shot("spend-collect")?;
    t.press(Action::StepNext(faraday_core::wallet::step::COLLECT));
    t.shot("spend-send")?;
    t.press(Action::StepNext(faraday_core::wallet::step::FINISH));
    t.shot("spend-away")?;
    println!("spend: vault unlocked on the tab, Savings signed by two keys and finished");
    // A vanity address for test key 1, on its account number.
    use faraday_core::vanity::{VanityAction as Va, vstep};
    t.press(Action::Vanity(Va::Open));
    let first = t.app.session.keys[0].master().fingerprint().0;
    t.press(Action::Vanity(Va::Key(first)));
    t.shot("vanity-dial")?;
    t.press(Action::Vanity(Va::Next(vstep::DIAL)));
    t.press(Action::Vanity(Va::Next(vstep::SCRIPT)));
    type_text(t, "qq");
    t.shot("vanity-prefix")?;
    t.press(Action::Vanity(Va::Next(vstep::PREFIX)));
    t.press(Action::Vanity(Va::Start));
    // Ticks a tenth of a second apart, as a shell sends them: well inside
    // the idle lock.
    for _ in 0..4000 {
        t.now += 100;
        t.app.event(Event::Tick { now_ms: t.now });
        t.pump();
        if t.app
            .vanity
            .as_ref()
            .is_some_and(|v| v.grind.find.is_some())
        {
            break;
        }
    }
    if t.app.vanity.as_ref().is_none_or(|v| v.grind.find.is_none()) {
        let v = t.app.vanity.as_ref();
        return Err(format!(
            "the vanity search found nothing: running {:?} tried {:?} prefix {:?} error {:?} screen {:?}",
            v.map(|v| v.running),
            v.map(|v| v.grind.tested),
            v.map(|v| v.grind.prefix.as_str().to_string()),
            v.and_then(|v| v.error.clone()),
            t.app.screen
        ));
    }
    t.shot("vanity-found")?;
    println!("vanity: test key 1's account dial found an address beginning tb1qqq");
    Ok(())
}

/// The New key quiz, every word answered rightly.
fn pass_quiz(t: &mut Tour) {
    for _ in 0..30 {
        let Some(q) = t.app.keygen.as_ref().and_then(|k| k.quiz.as_ref()) else {
            return;
        };
        if q.state() == faraday_core::keygen::QuizState::Passed {
            return;
        }
        let slot = q.correct_slot() as u8;
        t.press(Action::KQuiz(slot));
    }
}

/// A new key for a Create slot from the device's generator, as fast as
/// the flow allows.
fn device_key(t: &mut Tour, slot: u8) {
    let device = faraday_core::keygen::Way::Device.index();
    t.press(Action::KeyGen(Some(slot)));
    t.press(Action::KWords(24));
    t.press(Action::KWay(device));
    t.press(Action::KNext);
    t.press(Action::KNext);
    t.press(Action::KLock);
    t.press(Action::KNext);
}

/// A new key for a Create slot from 99 dice rolls, with a picture of each
/// card on the way.
fn dice_key(t: &mut Tour, slot: u8) -> Result<(), String> {
    t.press(Action::KeyGen(Some(slot)));
    t.shot("keygen-length")?;
    t.press(Action::KWords(24));
    t.shot("keygen-source")?;
    t.press(Action::KGroup(1));
    t.press(Action::KWay(faraday_core::keygen::Way::DiceHashed.index()));
    t.shot("keygen-source-computed")?;
    t.press(Action::KNext);
    // Rolls a die would give: a fixed run from a small generator.
    let mut x: u32 = 0x2545_f491;
    for i in 0..99 {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        t.press(Action::KRoll((x % 6) as u8 + 1));
        if i == 40 {
            t.shot("keygen-rolls")?;
        }
    }
    // The randomness check, shown on the entry card once the last roll
    // is in (`docs/NEW-WALLET.md` §3.2), before Continue moves on.
    t.shot("keygen-rolls-done")?;
    t.press(Action::KNext);
    t.shot("keygen-slot-key")?;
    t.press(Action::Learn);
    t.shot("learn-keygen")?;
    // The next page, scrolled to the dice table's em dashes.
    t.press(Action::LearnPage(1));
    let _ = t.app.frame();
    t.app.event(Event::Scroll {
        x: 640,
        y: 400,
        dy: 260,
    });
    t.shot("learn-randomness-sources")?;
    t.press(Action::Cancel);
    t.press(Action::KLock);
    t.shot("keygen-slot-key-locked")?;
    t.press(Action::KNext);
    Ok(())
}

/// New key from coin flips read off a die: the words as the flips come
/// in, a word in its list, the rest typed as a string, the checksum's
/// last word once the words are made; and the Word lists tile.
fn word_lists(t: &mut Tour) -> Result<(), String> {
    use faraday_core::keygen::{Way, kstep};
    use faraday_core::wordlist::WordListAction as WL;
    t.press(Action::KeyGen(None));
    t.press(Action::KWords(12));
    t.press(Action::KWay(Way::Coins.index()));
    t.press(Action::KNext);
    t.press(Action::KByDie(true));
    t.press(Action::KTyping(false));
    for i in 0..60u32 {
        t.press(Action::KDie(((i * 5 + i / 3) % 6) as u8 + 1));
    }
    t.shot("keygen-coins-by-die")?;
    let first = t
        .app
        .keygen
        .as_ref()
        .and_then(|k| k.live_words().first().copied())
        .ok_or("sixty flips named no word")?;
    t.press(Action::WordList(WL::Open(0, Some(first))));
    t.shot("wordlist-bip39-marked")?;
    t.press(Action::WordList(WL::Close));
    t.press(Action::KTyping(true));
    let rest: String = (0..68u32)
        .map(|i| char::from(b'1' + ((i * 7 + i / 4) % 6) as u8))
        .collect();
    type_text(t, &rest);
    t.shot("keygen-coins-typed")?;
    t.press(Action::KNext);
    t.press(Action::KStep(kstep::ENTER));
    t.shot("keygen-coins-made")?;
    for _ in 0..5 {
        t.app.event(Event::Key(Key::Down));
    }
    t.shot("keygen-coins-checksum")?;
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Catalog(
        faraday_core::catalog::TILES
            .iter()
            .position(|x| x.name == "EFF large list")
            .unwrap_or(0) as u8,
    ));
    type_text(t, "zeb");
    t.shot("wordlist-eff-large-find")?;
    t.press(Action::WordList(WL::Close));
    Ok(())
}

/// New key from rolls that name words (BitBox): the words as the rolls
/// come in, each roll over its bits, then the last word's rolled bits and
/// the checksum once the words are made.
fn rolled_words(t: &mut Tour) -> Result<(), String> {
    use faraday_core::keygen::{Way, kstep};
    t.press(Action::KeyGen(None));
    t.press(Action::KWords(12));
    t.press(Action::KWay(Way::DiceWords.index()));
    t.press(Action::KNext);
    let rolls: String = (0..27u32)
        .map(|i| {
            let face = if i % 6 == 5 {
                i % 6 + 1
            } else {
                (i * 3 + i / 6) % 4 + 1
            };
            char::from(b'0' + face as u8)
        })
        .collect();
    type_text(t, &rolls);
    t.shot("keygen-bitbox-rolls")?;
    let rest: String = (27..72u32)
        .map(|i| {
            let face = if i % 6 == 5 {
                i % 5 + 2
            } else {
                (i * 7 + i / 5) % 4 + 1
            };
            char::from(b'0' + face as u8)
        })
        .collect();
    type_text(t, &rest);
    t.press(Action::KNext);
    t.press(Action::KStep(kstep::ENTER));
    // A frame first, so the keys have a scrolled region to move.
    let _ = t.app.frame();
    for _ in 0..14 {
        t.app.event(Event::Key(Key::Down));
    }
    t.shot("keygen-bitbox-made")?;
    // On its own, New key goes from Key to Words and the Quiz, the key
    // added at Lock in and the Quiz's Done leaving the flow
    // (`docs/NEW-WALLET.md` §3.4).
    t.press(Action::KStep(kstep::KEY));
    t.shot("keygen-key-own")?;
    t.press(Action::KLock);
    t.shot("keygen-key-own-locked")?;
    t.press(Action::KNext);
    t.shot("keygen-words")?;
    t.press(Action::KShow);
    t.shot("keygen-words-shown")?;
    t.press(Action::KShow);
    t.press(Action::KNext);
    t.shot("keygen-quiz")?;
    pass_quiz(t);
    t.shot("keygen-quiz-passed")?;
    t.press(Action::KAdd);
    t.press(Action::Nav(Screen::Catalog));
    Ok(())
}

/// Add a key from the lists typed on keyboards of their own: Japanese
/// with a word half typed, Korean, Simplified Chinese with a reading
/// awaiting its tone and with its characters offered, and Traditional.
fn keyed_entry(t: &mut Tour) -> Result<(), String> {
    use faraday_core::forms::LANGUAGES;
    use osk_bip::bip39::Language;
    let at = |l: Language| LANGUAGES.iter().position(|x| *x == l).unwrap_or(0) as u8;
    t.press(Action::Entry(None));
    t.press(Action::EntryLanguages);
    t.press(Action::EntryLanguage(at(Language::Japanese)));
    t.press(Action::EntryKey('あ'));
    t.shot("entry-japanese")?;
    for _ in 0..13 {
        t.press(Action::EntryKey('あ'));
        t.press(Action::EntryCandidate(0));
    }
    t.press(Action::EntryKey('か'));
    t.shot("entry-japanese-thirteen")?;
    t.press(Action::EntryLanguage(at(Language::Korean)));
    t.shot("entry-korean")?;
    t.press(Action::EntryLanguage(at(Language::ChineseSimplified)));
    type_text(t, "shi");
    t.shot("entry-chinese-pinyin")?;
    type_text(t, "4");
    t.shot("entry-chinese-pinyin-tone")?;
    t.press(Action::EntryLanguage(at(Language::ChineseTraditional)));
    t.shot("entry-chinese-zhuyin")?;
    t.press(Action::Nav(Screen::Wallets));
    Ok(())
}

/// A key's silent payments wallet among the wallets: its card, and its
/// page with Check a payment open.
fn silent_wallet(t: &mut Tour) -> Result<(), String> {
    use faraday_core::silent::sstep;
    t.press(Action::Silent);
    if let Some(fp) = t
        .app
        .session
        .keys
        .first()
        .map(|k| k.master().fingerprint().0)
    {
        t.press(Action::SKey(fp));
    }
    t.press(Action::SAddWallet);
    let i = t.app.session.wallets.len().saturating_sub(1);
    t.press(Action::PickWallet(i));
    t.press(Action::Nav(Screen::Wallets));
    t.shot("wallet-silent")?;
    t.press(Action::SWallet(i));
    t.press(Action::SStep(sstep::CHECK));
    t.shot("silent-check")?;
    t.press(Action::Nav(Screen::Wallets));
    Ok(())
}

/// The page scrolled down by `dy`, as a wheel would.
fn scroll(t: &mut Tour, dy: i16) {
    let _ = t.app.frame();
    let (w, h) = t.size();
    t.app.event(Event::Scroll {
        x: w / 2,
        y: h / 2,
        dy,
    });
}

fn type_text(t: &mut Tour, text: &str) {
    for c in text.chars() {
        t.app.event(Event::Key(Key::Char(c)));
    }
    t.pump();
}

/// The test vault's key and wallet shown and saved (`docs/BACKUP-FLOW.md`
/// §19.7): the key with its SeedQR held, its Save to a stick, and the
/// Spending wallet's Save to a stick.
fn vault_show_tour(t: &mut Tour) -> Result<(), String> {
    use osk_shell_api::TouchPhase;
    let vault = testkit::files()?
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .ok_or("no test vault")?;
    t.app.storage(StorageEvent::Restored {
        inbox: vec![vault],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Open(0)));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.press(Action::Vault(V::Unlock));
    t.tick();
    t.press(Action::Nav(Screen::VaultContents));
    t.press(Action::Vault(V::Category(0)));
    t.press(Action::Vault(V::Item(0)));
    let show = Action::Vault(V::ShowSeedQr);
    let _ = t.app.frame();
    let (x, y) = t
        .app
        .where_offered(show)
        .ok_or("Show SeedQR is not on screen")?;
    t.app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    t.shot("vault-key-seedqr")?;
    t.app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
    t.press(Action::Vault(V::SaveToStick));
    t.shot("vault-key-save")?;
    t.press(Action::Cancel);
    t.press(Action::Vault(V::Category(1)));
    let k = t.app.vaults.open[0]
        .contents
        .records
        .iter()
        .filter(|r| {
            [
                faraday_vault::records::kind::WALLET,
                faraday_vault::records::kind::WALLET_SHARE,
            ]
            .contains(&r.kind)
        })
        .position(|r| r.text(faraday_vault::records::field::WALLET_NAME) == Some("Spending"))
        .ok_or("no Spending in the test vault")?;
    t.press(Action::Vault(V::Item(k)));
    t.press(Action::Vault(V::SaveToStick));
    t.shot("vault-wallet-save")
}

/// Keys from a vault on Create's Keys card, and New wallet from this one
/// (`docs/NEW-WALLET.md` §16): a 2-of-3's Keys with the test vault locked,
/// then unlocked from there with test key 1 offered under its name; a
/// 2-of-3 over the vault's key, test key 2 here and test key 3's xpub, its
/// card with New wallet from this one, and Create filled from it: Kind
/// with the Account stepper, then Keys with key 3 waiting for its xpub at
/// account 1.
fn create_from_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::create::NewKind;
    use faraday_core::cstep;
    let vault = testkit::files()?
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .ok_or("no test vault")?;
    t.app.storage(StorageEvent::Restored {
        inbox: vec![vault],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    let (word, label) = testkit::TEST_SEEDS[1];
    t.app
        .session
        .add_words(c, &testkit::test_words(word), label, None)
        .map_err(|e| e.text())?;
    let multi = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .ok_or("no multisig")? as u8;
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    t.press(Action::CNext(cstep::KIND));
    t.press(Action::CVaults(false));
    t.press(Action::CNext(cstep::WHERE));
    t.press(Action::CNext(cstep::QUORUM));
    t.shot("create-keys-vault-locked")?;
    // A small panel's page, down to what the change put there.
    let down = |t: &mut Tour, action: Action, name: &str| -> Result<(), String> {
        if t.app.is_compact() {
            scroll_to(t, action);
            t.shot(name)?;
        }
        Ok(())
    };
    let unlock = Action::Vault(V::OpenFrom(0, Screen::Create));
    down(t, unlock, "create-keys-vault-locked-down")?;
    t.press(Action::Vault(V::OpenFrom(0, Screen::Create)));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..10 {
        if !t.app.vaults.open.is_empty() {
            break;
        }
        t.tick();
    }
    if t.app.screen != Screen::Create {
        return Err("unlocking did not come back to Create".into());
    }
    t.shot("create-keys-vault-open")?;
    let record = t.app.vaults.open[0]
        .contents
        .records
        .iter()
        .position(|r| r.kind == faraday_vault::records::kind::KEY)
        .ok_or("no key in the test vault")?;
    down(
        t,
        Action::CSlotVault(0, 0, record),
        "create-keys-vault-open-down",
    )?;
    // The vault's key and test key 3 worked out apart from the session.
    let mut apart = faraday_core::wallet::Session::default();
    for n in [0, 2] {
        let (word, label) = testkit::TEST_SEEDS[n];
        apart
            .add_words(c, &testkit::test_words(word), label, None)
            .map_err(|e| e.text())?;
    }
    let keys: Vec<String> = [
        apart.keys[0].master(),
        t.app.session.keys[0].master(),
        apart.keys[1].master(),
    ]
    .iter()
    .map(|m| NewKind::Multi.key_text(c, m))
    .collect::<Result<_, _>>()?;
    let i = t
        .app
        .session
        .add_wallet(
            c,
            "Shared",
            &NewKind::Multi.descriptor(2, &keys),
            "Test",
            faraday_core::wallet::Origin::Named,
        )
        .map_err(|e| e.text())?;
    t.press(Action::OpenWallet(i));
    t.shot("wallet-new-from")?;
    down(t, Action::NewWalletFrom(i), "wallet-new-from-down")?;
    t.press(Action::NewWalletFrom(i));
    t.press(Action::CStep(cstep::KIND));
    t.shot("create-from-kind")?;
    t.press(Action::CNext(cstep::KIND));
    t.press(Action::CVaults(false));
    t.press(Action::CNext(cstep::WHERE));
    t.shot("create-from-keys")?;
    down(t, Action::CSlotClear(2), "create-from-keys-down")?;
    // A fresh single-key wallet: Check's Path, and Change back to Kind's
    // Account (`docs/NEW-WALLET.md` §17).
    t.press(Action::CreateOver);
    t.press(Action::CNext(cstep::KIND));
    t.press(Action::CVaults(false));
    t.press(Action::CNext(cstep::WHERE));
    let fp = t.app.session.keys[0].fingerprint().0;
    t.press(Action::CSlotHere(0, fp));
    t.press(Action::CNext(cstep::KEYS));
    t.shot("create-check-path")?;
    down(t, Action::CPathChange, "create-check-path-down")?;
    t.press(Action::CPathChange);
    t.press(Action::CAccount(1));
    t.shot("create-path-kind-account")
}

/// A flow standing on Make over the vault open now, as a person has it after
/// the sticks and the vaults: the tours that are not about the flow start
/// from here.
fn flow_at_make(c: &Compute, t: &mut Tour, kind: faraday_core::backup_flow::Kind) {
    use faraday_core::backup_flow::{BackupFlow, Plan, Step};
    use faraday_core::backup_flow_steps::FlowAction as F;
    let (salt, name) = {
        let o = &t.app.vaults.open[t.app.vaults.current];
        (o.header().salt, o.name.clone())
    };
    let mut plan = Plan::key_default();
    plan.vaults[0].salt = Some(salt);
    plan.vaults[0].file = Some(name);
    plan.vaults[0].existing = true;
    let mut f = BackupFlow::with_plan(c, kind, plan);
    f.step = Step::Make;
    t.app.backup_flows.push(f);
    let i = t.app.backup_flows.len() - 1;
    t.press(Action::Flow(F::Show(i)));
}

/// The flow's Make pressed, and the key it made released as a key a
/// vault unlocked from a stick holds is; the vault's contents shown on
/// its kind.
fn flow_made(t: &mut Tour, kind: faraday_core::backup_flow::Kind) -> Result<(), String> {
    use faraday_core::backup_flow_steps::FlowAction as F;
    t.press(Action::Flow(F::Make));
    t.tick();
    t.tick();
    if t.app.screen == Screen::SetTime {
        t.press(Action::SetTimeKeep);
        t.press(Action::Flow(F::Make));
        t.tick();
        t.tick();
    }
    if !t.app.backup_flows.last().is_some_and(|f| f.made) {
        return Err("the flow made no key".into());
    }
    for f in &mut t.app.backup_flows {
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

/// The backup flow's Bring in the sticks with the boot stick taken for
/// Place 1 (`docs/BACKUP-FLOW.md` §18): it is Stick 1, the Boot stick,
/// and the next stick is Stick 2.
/// A vault of the three test keys, the third kept without the passphrase
/// its 2-of-3 uses it with, and a seed of no wallet (F62): the Load sheet,
/// Seeds with no passphrase stored, a passphrase giving another key, and
/// the right one.
fn pass_seeds_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::vaults::VaultAction as V;
    use faraday_vault::records::{self, field, kind};
    use faraday_vault::{Contents, Cost, Record};
    use osk_bip::bip39::{Language, Mnemonic};
    use osk_bip::bitcoin::bip32::DerivationPath;
    let words = |p: &str| Mnemonic::parse(c, Language::English, p).map_err(|e| format!("{e:?}"));
    let key = |m: &Mnemonic, label: &str| {
        Record::new(kind::KEY)
            .with(field::KEY, &records::words_payload(m))
            .with(field::KEY_LABEL, label.as_bytes())
    };
    let p = "m/48'/1'/0'/2'";
    let third = words(&testkit::test_words(testkit::TEST_SEEDS[2].0))?;
    let seed = third
        .to_seed(c, b"kept apart")
        .map_err(|e| format!("{e:?}"))?;
    let master = osk_bip::keys::MasterKey::from_seed(c, &seed, testkit::NET);
    let path: DerivationPath = p.parse().map_err(|e| format!("{e:?}"))?;
    let trio = format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,[{}/48h/1h/0h/2h]{}/<0;1>/*))",
        testkit::key(0, p),
        testkit::key(1, p),
        master.fingerprint(),
        master.derive(c, &path).to_xpub(c)
    );
    let mut records = vec![Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Trio")];
    for n in 0..3 {
        let m = words(&testkit::test_words(testkit::TEST_SEEDS[n].0))?;
        records.push(key(&m, testkit::TEST_SEEDS[n].1));
    }
    records.push(key(
        &words("abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about")?,
        "Spare",
    ));
    records.push(
        Record::new(kind::WALLET)
            .with(field::WALLET, trio.as_bytes())
            .with(field::WALLET_NAME, b"Trio"),
    );
    let cost = Cost {
        memory_kib: 64 * 1024,
        passes: 1,
        lanes: 1,
    };
    let file = faraday_vault::create(
        c,
        faraday_vault::SLOT_SIZES[0],
        cost,
        &[b"trio phrase".as_slice()],
        &[Contents { records }],
        &[0x5e; 32],
    )
    .map_err(|e| e.reason())?;
    t.app.storage(StorageEvent::Restored {
        inbox: vec![("trio.ofv".into(), file)],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Open(0)));
    type_text(t, "trio phrase");
    t.press(Action::Vault(V::Unlock));
    t.tick();
    if t.app.sheet != Some(faraday_core::Sheet::UnlockLoad) {
        return Err(format!(
            "the trio vault did not ask what to load: {:?}",
            t.app.vaults.unlock_error
        ));
    }
    t.shot("passseeds-load-sheet")?;
    let keys: Vec<usize> = t
        .app
        .vault_load_view_now(c)
        .ok_or("no Load sheet")?
        .keys
        .iter()
        .filter(|k| k.ticked)
        .map(|k| k.record)
        .collect();
    for r in keys {
        t.press(Action::Vault(V::LoadTick(r)));
    }
    t.press(Action::Vault(V::LoadTicked));
    t.press(Action::OpenWallet(0));
    t.press(Action::Vault(V::PassphraseKey(0)));
    t.shot("passseeds-list")?;
    let pick = t
        .app
        .vault_passphrase_seeds_now(c)
        .iter()
        .find(|s| s.line.ends_with(testkit::TEST_SEEDS[2].1))
        .map(|s| (s.vault, s.record))
        .ok_or("test key 3 is not listed")?;
    t.press(Action::Vault(V::PassphrasePick(pick.0, pick.1)));
    type_text(t, "wrong");
    t.press(Action::Vault(V::PromptGo));
    t.shot("passseeds-mismatch")?;
    type_text(t, "kept apart");
    t.press(Action::Vault(V::PromptGo));
    if t.app.sheet.is_some() || t.app.session.keys.len() != 3 {
        return Err("the right passphrase did not load test key 3".into());
    }
    t.shot("passseeds-loaded")?;
    println!("passseeds: a seed kept without its passphrase, found by its passphrase");
    Ok(())
}

fn boot_place_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::backup_flow::Kind;
    use faraday_core::backup_flow_steps::FlowAction as F;
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    t.press(Action::OpenSetTime);
    t.press(Action::SetTimeKeep);
    t.app.storage(StorageEvent::Sticks(vec![StickInfo {
        id: "boot".into(),
        label: "FARADAY".into(),
        boot: true,
        serial: Some(0x1234_5678),
        files: Vec::new(),
    }]));
    t.pump();
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|x| x.go == faraday_core::catalog::Go::VaultCategory(Kind::Gpg.category()))
        .ok_or("no GPG tile")? as u8;
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Catalog(tile));
    t.press(Action::Flow(F::Continue));
    t.shot("bootplace-1-insert")?;
    let k = t
        .app
        .sticks
        .iter()
        .position(|s| s.boot)
        .ok_or("no boot stick")?;
    t.press(Action::Flow(F::UseBoot(k)));
    t.pump();
    t.press(Action::Flow(F::Tap));
    t.pump();
    t.shot("bootplace-2-bound")?;
    println!("the boot stick taken for Place 1: Stick 1, the next Stick 2");
    Ok(())
}

/// The backup flow for a GPG key kept in two vaults at three places
/// (`docs/BACKUP-FLOW.md`): the screens a person sees from Where to Done.
/// Two flows in progress (`docs/FLOWS.md`, Flows in progress): a GPG
/// key's flow at its sticks, and Make a new wallet left on its backup's
/// Places. Each row is lit on its own screen; Settings shows both under
/// their tabs, and the small Home leads with a tile for each.
fn in_progress_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::backup_flow::Kind;
    use faraday_core::backup_flow_steps::FlowAction as F;
    use faraday_core::cstep;
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    t.press(Action::OpenSetTime);
    t.press(Action::SetTimeKeep);
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|x| x.go == faraday_core::catalog::Go::VaultCategory(Kind::Gpg.category()))
        .ok_or("no GPG tile")? as u8;
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Catalog(tile));
    t.press(Action::Flow(F::Continue));
    t.shot("inprogress-gpg")?;
    for n in 0..3 {
        t.press(Action::Entry(None));
        type_text(t, &testkit::test_words(testkit::TEST_SEEDS[n].0));
        t.press(Action::EntryAdd);
    }
    let multi = faraday_core::create::NewKind::ALL
        .iter()
        .position(|k| *k == faraday_core::create::NewKind::Multi)
        .ok_or("no multisig")? as u8;
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    for slot in 0..3u8 {
        let fp = t.app.session.keys[usize::from(slot)].fingerprint().0;
        t.press(Action::CSlotHere(slot, fp));
    }
    t.press(Action::CNext(cstep::KEYS));
    t.press(Action::CNext(cstep::CHECK));
    t.press(Action::CBackup);
    t.backup_as(faraday_core::plan::Preset::Paper);
    t.shot("inprogress-places")?;
    t.press(Action::Nav(Screen::Settings));
    t.shot("inprogress-settings")?;
    t.press(Action::Nav(Screen::Home));
    t.shot("inprogress-home")
}

/// Cancel at the top of a flow (`docs/FLOWS.md`, Flows in progress,
/// Cancel; F51): Make a new wallet past Kind with Cancel beside the
/// switch, the sheet it brings up, and the sheet over its backup's Places.
fn cancel_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::cstep;
    for n in 0..3 {
        t.press(Action::Entry(None));
        type_text(t, &testkit::test_words(testkit::TEST_SEEDS[n].0));
        t.press(Action::EntryAdd);
    }
    let multi = faraday_core::create::NewKind::ALL
        .iter()
        .position(|k| *k == faraday_core::create::NewKind::Multi)
        .ok_or("no multisig")? as u8;
    t.press(Action::Nav(Screen::Start));
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    t.press(Action::CNext(cstep::KIND));
    t.press(Action::CVaults(false));
    t.press(Action::CNext(cstep::WHERE));
    t.shot("cancel-1-create")?;
    t.press(Action::CancelFlow);
    t.shot("cancel-2-sheet")?;
    t.press(Action::Cancel);
    t.press(Action::CNext(cstep::QUORUM));
    for slot in 0..3u8 {
        let fp = t.app.session.keys[usize::from(slot)].fingerprint().0;
        t.press(Action::CSlotHere(slot, fp));
    }
    t.press(Action::CNext(cstep::KEYS));
    t.press(Action::CNext(cstep::CHECK));
    t.press(Action::CBackup);
    t.backup_as(faraday_core::plan::Preset::Paper);
    t.shot("cancel-3-places")?;
    t.press(Action::CancelFlow);
    t.shot("cancel-4-places-sheet")?;
    t.press(Action::CancelFlowNow);
    t.shot("cancel-5-wallets")
}

/// The standard way on the board (`docs/BACKUP-FLOW.md` §19.6, §19.3,
/// §19.5, §19.21): Make a new wallet opening on Kind; its Back up step
/// with Back up this wallet; a single key's Back up open on the board
/// read-only; a 2-of-3's the same, then with Add watch-only software
/// pressed; the 2-of-3 after
/// Customize backup with its form toggles; and the sheet over a drop that
/// lets a place see the balance.
fn standard_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::board::{BoardPress as B, Chip, Thing, Zone};
    use faraday_core::cstep;
    t.press(Action::Entry(None));
    type_text(t, &testkit::test_words(testkit::TEST_SEEDS[0].0));
    t.press(Action::EntryAdd);
    t.press(Action::CreateWallet);
    t.shot("std-1-create-kind")?;
    // A single key from the key loaded.
    t.press(Action::CNext(cstep::KIND));
    let fp = t.app.session.keys[0].fingerprint().0;
    t.press(Action::CSlotHere(0, fp));
    t.press(Action::CNext(cstep::KEYS));
    // The vault question with its line (§19.21 item 5), then Create's
    // Back up step: Back up this wallet (item 2).
    t.press(Action::CStep(cstep::WHERE));
    t.shot("std-2a-vault-question")?;
    t.press(Action::CStep(cstep::BACKUP));
    t.shot("std-2b-create-back-up")?;
    t.press(Action::CBackup);
    t.shot("std-2-single-key")?;
    for n in 1..3 {
        t.press(Action::Entry(None));
        type_text(t, &testkit::test_words(testkit::TEST_SEEDS[n].0));
        t.press(Action::EntryAdd);
    }
    let multi = faraday_core::create::NewKind::ALL
        .iter()
        .position(|k| *k == faraday_core::create::NewKind::Multi)
        .ok_or("no multisig")? as u8;
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    for slot in 0..3u8 {
        let fp = t.app.session.keys[usize::from(slot)].fingerprint().0;
        t.press(Action::CSlotHere(slot, fp));
    }
    t.press(Action::CNext(cstep::KEYS));
    let three = t
        .app
        .session
        .wallets
        .len()
        .checked_sub(1)
        .ok_or("no wallet")?;
    t.press(Action::Backup(three));
    t.shot("std-3-two-of-three")?;
    // Add watch-only software on the read-only board (§19.21 item 3).
    t.press(Action::Board(B::Software));
    t.shot("std-3a-watch-only")?;
    t.press(Action::Board(B::Software));
    t.press(Action::Board(B::Customize));
    t.shot("std-4-customize")?;
    // Whole off Place 3, Share 1 dropped there beside seed 3: the sheet.
    t.press(Action::Board(B::Off(Zone::Paper(2), Thing::Whole)));
    t.app
        .press(Action::Board(B::Chip(Chip::List(Thing::Share(0)))));
    t.app.press(Action::Board(B::Zone(Zone::Paper(2))));
    t.pump();
    t.shot("std-5-sees-sheet")
}

/// Partial descriptions by one button (`docs/BACKUP-FLOW.md` §19.19): a
/// 2-of-3's board customised; the sheet after adding them; the board with
/// a share at each place and the whole taken off every place; the board
/// after Remove partial descriptions, its warning under it.
fn partials_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::board::BoardPress as B;
    use faraday_core::cstep;
    for n in 0..3 {
        t.press(Action::Entry(None));
        type_text(t, &testkit::test_words(testkit::TEST_SEEDS[n].0));
        t.press(Action::EntryAdd);
    }
    let multi = faraday_core::create::NewKind::ALL
        .iter()
        .position(|k| *k == faraday_core::create::NewKind::Multi)
        .ok_or("no multisig")? as u8;
    t.press(Action::CreateWallet);
    t.press(Action::CKind(multi));
    for slot in 0..3u8 {
        let fp = t.app.session.keys[usize::from(slot)].fingerprint().0;
        t.press(Action::CSlotHere(slot, fp));
    }
    t.press(Action::CNext(cstep::KEYS));
    let w = t
        .app
        .session
        .wallets
        .len()
        .checked_sub(1)
        .ok_or("no wallet")?;
    t.press(Action::Backup(w));
    t.press(Action::Board(B::Customize));
    t.app.press(Action::Board(B::Partials));
    t.pump();
    t.shot("parts-1-sheet")?;
    t.press(Action::Board(B::PartialsWholeOff));
    t.shot("parts-2-shares")?;
    t.press(Action::Board(B::Partials));
    scroll(t, 400);
    t.shot("parts-3-removed")
}

fn flow_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::backup_flow::Kind;
    use faraday_core::backup_flow_steps::FlowAction as F;
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    t.press(Action::OpenSetTime);
    t.press(Action::SetTimeKeep);
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|x| x.go == faraday_core::catalog::Go::VaultCategory(Kind::Gpg.category()))
        .ok_or("no GPG tile")? as u8;
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Catalog(tile));
    // Vault 1 is the test vault, on the first stick; Vault 2 is a new one
    // at the second and third places, on two empty sticks.
    t.press(Action::Flow(F::AddPlace));
    t.press(Action::Flow(F::PlaceVault(1, 1)));
    t.press(Action::Flow(F::PlaceVault(2, 1)));
    t.press(Action::Flow(F::VaultExisting(0)));
    t.shot("flow-1-where")?;
    t.press(Action::Flow(F::Continue));
    t.shot("flow-2-insert")?;
    let first = t.stick();
    t.app.storage(StorageEvent::Sticks(vec![first.clone()]));
    t.pump();
    t.shot("flow-3-which-vault")?;
    let k = t
        .app
        .flow()
        .and_then(|f| f.live.reading.as_ref())
        .and_then(|r| r.files.iter().position(|(f, _)| f.name == "vault.ofv"))
        .ok_or("the stick shows no vault.ofv")?;
    t.press(Action::Flow(F::PickFile(0, k)));
    t.shot("flow-4-confirm")?;
    t.press(Action::Flow(F::Tap));
    // Bound: its pill is lit and the card says to mark it (§18).
    t.shot("flow-4b-mark")?;
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    // Two empty sticks that read exactly alike: only the person can tell
    // them apart.
    let empty = |id: &str| StickInfo {
        serial: Some(0x1357_9bdf),
        id: id.into(),
        label: "SPARE".into(),
        boot: false,
        files: Vec::new(),
    };
    t.app.storage(StorageEvent::Sticks(vec![empty("empty-a")]));
    t.pump();
    t.press(Action::Flow(F::Tap));
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.app.storage(StorageEvent::Sticks(vec![empty("empty-b")]));
    t.pump();
    t.shot("flow-5-hold")?;
    t.press(Action::Flow(F::Hold));
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.shot("flow-6-vaults")?;
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.press(Action::Vault(V::Unlock));
    t.tick();
    t.tick();
    t.shot("flow-7-create-vault")?;
    t.press(Action::Flow(F::CreateVault(1)));
    t.shot("flow-8-create-vault-form")?;
    type_text(t, "a second vault phrase");
    t.press(Action::Vault(V::CFocus(0, true)));
    type_text(t, "a second vault phrase");
    t.press(Action::Vault(V::CGo));
    t.tick();
    t.tick();
    t.tick();
    t.shot("flow-9-make")?;
    type_text(t, "Test Person");
    t.app.event(Event::Key(Key::Tab));
    type_text(t, "test@example.com");
    t.press(Action::Flow(F::Make));
    t.shot("flow-10-made")?;
    t.press(Action::Flow(F::MadeContinue));
    t.shot("flow-11-lock")?;
    t.press(Action::Flow(F::LockWrite));
    let (inbox, outbox, kept) = t.boxes.clone().ok_or("nothing was kept by the lock")?;
    t.app.storage(StorageEvent::Restored {
        inbox,
        outbox,
        kept,
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    t.pump();
    t.shot("flow-12-write")?;
    t.app.storage(StorageEvent::Sticks(vec![first]));
    t.pump();
    t.shot("flow-13-written")?;
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    // A stick that is not the one read for the place: Use another stick.
    let mut other = empty("other");
    other.label = "ELSEWHERE".into();
    other.serial = Some(0x2468_ace0);
    t.app.storage(StorageEvent::Sticks(vec![other]));
    t.pump();
    t.shot("flow-14-use-another")?;
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.app.storage(StorageEvent::Sticks(vec![empty("empty-a")]));
    t.pump();
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.app.storage(StorageEvent::Sticks(vec![empty("empty-b")]));
    t.pump();
    t.shot("flow-15-second-stick")?;
    t.press(Action::Flow(F::Hold));
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.shot("flow-16-done")?;
    println!("the backup flow of a GPG key in two vaults at three places, from Where to Done");
    Ok(())
}

/// A wallet's backup flow: Make a new wallet asks whether vaults are part
/// of the backup and how many, brings in each vault's stick, opens or makes
/// the vaults, makes the key; Back up's plan offers the vaults; the
/// checklist ends in Write to sticks; one lock, each stick by place, done.
fn wallet_flow_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    use faraday_core::backup_flow_steps::FlowAction as F;
    use faraday_core::cstep;
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    t.app.vaults.ms_per_unit = Some(180);
    t.press(Action::OpenSetTime);
    t.press(Action::SetTimeKeep);
    let empty = |id: &str, serial: u32| StickInfo {
        serial: Some(serial),
        id: id.into(),
        label: "KEYSTICK".into(),
        boot: false,
        files: Vec::new(),
    };
    t.press(Action::CreateWallet);
    t.press(Action::CNext(cstep::KIND));
    t.shot("wallet-flow-1-create-yes-no")?;
    t.press(Action::CVaults(true));
    t.press(Action::CNext(cstep::WHERE));
    t.press(Action::CVaultN(2));
    t.shot("wallet-flow-2-create-count")?;
    t.press(Action::CNext(cstep::HOW_MANY));
    t.shot("wallet-flow-3-insert")?;
    // Vault 1: a stick that holds a vault, and the person picks.
    let first = t.stick();
    t.app.storage(StorageEvent::Sticks(vec![first.clone()]));
    t.pump();
    t.shot("wallet-flow-4-which-vault")?;
    t.press(Action::Flow(F::PickBeside(0)));
    t.press(Action::Flow(F::Tap));
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    // Vault 2: an empty stick, a new vault.
    t.app.storage(StorageEvent::Sticks(vec![empty("b", 2)]));
    t.pump();
    t.press(Action::Flow(F::Tap));
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    t.shot("wallet-flow-5-vaults")?;
    for v in 0..2 {
        t.press(Action::Flow(F::CreateVault(v)));
        type_text(t, "a vault phrase for the tour");
        t.press(Action::Vault(V::CFocus(0, true)));
        type_text(t, "a vault phrase for the tour");
        t.press(Action::Vault(V::CGo));
        for _ in 0..3 {
            t.tick();
        }
    }
    // Back in Make a new wallet, on Keys, with the cards done.
    t.shot("wallet-flow-6-create-keys")?;
    t.press(Action::Entry(None));
    t.type_key(1);
    let fp = t
        .app
        .session
        .keys
        .last()
        .ok_or("no key")?
        .master()
        .fingerprint()
        .0;
    t.press(Action::Nav(Screen::Create));
    t.press(Action::CSlotHere(0, fp));
    t.press(Action::CNext(cstep::KEYS));
    t.press(Action::CNext(cstep::CHECK));
    t.press(Action::CBackup);
    t.backup_as(faraday_core::plan::Preset::PaperVault);
    // Back up's plan offers the two open vaults.
    t.shot("wallet-flow-7-back-up-vaults")?;
    // The key in both vaults, none on paper.
    words_off(c, t);
    let p = vault_place(c, t, 1).unwrap_or(0);
    keep_vault(c, t, p, 1, &[0]);
    t.press(Action::BChecklist);
    t.press(Action::BVaultSave(0));
    t.press(Action::BVaultSave(1));
    t.press(Action::BOut(3));
    t.press(Action::BNext(bstep::ENVELOPE));
    t.shot("wallet-flow-8-write-to-sticks")?;
    t.press(Action::Flow(F::WalletLock));
    let (inbox, outbox, kept) = t.boxes.clone().ok_or("nothing was kept by the lock")?;
    t.app.storage(StorageEvent::Restored {
        inbox,
        outbox,
        kept,
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.app.storage(StorageEvent::Clock {
        unix_secs: 1_791_000_000,
    });
    t.pump();
    t.shot("wallet-flow-9-write")?;
    // Change the plan in the flow: the board, drawn in the flow on the
    // flow's own plan (`docs/BACKUP-FLOW.md` §19.12 item 1): The plan,
    // then the board after the lock, Seed 1 selected, a vault's stick
    // also at another place.
    t.press(Action::Flow(F::ChangePlan));
    t.shot("wallet-flow-9a-plan-card")?;
    t.press(Action::Flow(F::Plan(faraday_core::PlanPress::Q(
        qstep::BOARD,
    ))));
    t.shot("wallet-flow-9b-plan-board")?;
    t.press(Action::Board(BP::Chip(Chip::List(Thing::Vault(0)))));
    t.shot("wallet-flow-9c-plan-board-vault-held")?;
    t.press(Action::Cancel);
    if let Some(n) = t.app.flow().map(|f| f.step.n()) {
        t.press(Action::Flow(F::Open(n)));
    }
    // Each stick by place: the sticks read for the vaults, then the stick
    // of public files.
    for n in 0..3 {
        let slot = t
            .app
            .flow()
            .and_then(|f| f.slots.iter().position(|s| s.written.is_none()));
        let Some(slot) = slot else { break };
        let stick = match t.app.flow().and_then(|f| f.slots[slot].bound.clone()) {
            Some(b) if b.label == "TESTSTICK" => t.stick(),
            Some(b) => empty("b", b.serial.unwrap_or(2)),
            None => empty("c", 3),
        };
        t.app.storage(StorageEvent::Sticks(vec![stick]));
        t.pump();
        if n == 0 {
            t.shot("wallet-flow-10-written")?;
        }
        for _ in 0..2 {
            let hold = t
                .app
                .flow()
                .and_then(|f| f.live.reading.as_ref())
                .is_some_and(|r| r.hold);
            t.press(Action::Flow(if hold { F::Hold } else { F::Tap }));
        }
        t.app.storage(StorageEvent::Sticks(Vec::new()));
        t.pump();
    }
    t.shot("wallet-flow-11-done")?;
    println!("the backup flow of a single key in two vaults, from the question of vaults to Done");
    Ok(())
}

fn fp_hex(fp: [u8; 4]) -> String {
    fp.iter().map(|b| format!("{b:02x}")).collect()
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().collect();
    let medium = match args.iter().position(|a| a == "--sd-card") {
        Some(i) => {
            args.remove(i);
            Medium::SdCard
        }
        None => Medium::Stick,
    };
    if !(4..=5).contains(&args.len()) {
        eprintln!(
            "usage: faraday-snapshot [--sd-card] WxH[@DPI] TESTKIT_DIR OUT_DIR [spend|themes|compact|seeds|keys|visit|copy|scan|public|seedfile|vaultway|kept|again|plan|inbox|transfer|upgrade|glance|chartedit|fromvault|first|restore|osk|flow|walletflow|demo|vaultshow|addkey|cancel]"
        );
        return ExitCode::from(2);
    }
    let (size_arg, dpi_arg) = match args[1].split_once('@') {
        Some((s, d)) => (s, Some(d)),
        None => (args[1].as_str(), None),
    };
    let Some((w, h)) = size_arg
        .split_once('x')
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
    else {
        eprintln!("size must be WxH[@DPI]");
        return ExitCode::from(2);
    };
    // 160 is a desktop monitor's rough density; a real panel (the Pi's
    // 480x640 at 286 dpi, or the 240x320 minimum at 143) must say its
    // own, because the core picks its size class from physical width,
    // not from pixels.
    let dpi: u16 = match dpi_arg.map(str::parse) {
        Some(Ok(d)) => d,
        Some(Err(_)) => {
            eprintln!("dpi must be a number");
            return ExitCode::from(2);
        }
        None => 160,
    };
    // The tool's token, for the costly work it does outside the app's
    // events once that work takes one (docs/COMPUTE.md §3). A tool makes
    // no drawing claim, so it counts nothing.
    #[expect(
        clippy::disallowed_methods,
        reason = "a mint: the snapshot tool (docs/COMPUTE.md §3)"
    )]
    let compute = osk_crypto::work::Compute::new(|_| {});
    let only = args.get(4).map(String::as_str);
    if only.is_some_and(|m| {
        ![
            "spend",
            "devices",
            "themes",
            "compact",
            "seeds",
            "keys",
            "visit",
            "copy",
            "scan",
            "public",
            "seedfile",
            "vaultway",
            "kept",
            "again",
            "plan",
            "board",
            "menu",
            "inbox",
            "transfer",
            "upgrade",
            "glance",
            "chartedit",
            "fromvault",
            "first",
            "restore",
            "osk",
            "flow",
            "bootplace",
            "passseeds",
            "power",
            "pills",
            "walletflow",
            "demo",
            "startwarn",
            "vaultshow",
            "addkey",
            "createfrom",
            "inprogress",
            "cancel",
            "std",
            "parts",
        ]
        .contains(&m)
    }) {
        eprintln!(
            "the tour is spend, devices, themes, compact, seeds, keys, visit, copy, scan, public, seedfile, vaultway, kept, again, plan, board, menu, inbox, transfer, upgrade, glance, chartedit, fromvault, first, restore, osk, flow, bootplace, passseeds, power, pills, walletflow, demo, startwarn, vaultshow, addkey, createfrom, inprogress, cancel, std or parts"
        );
        return ExitCode::from(2);
    }
    match run(
        &compute,
        (w, h),
        dpi,
        Path::new(&args[2]),
        Path::new(&args[3]),
        only,
        medium,
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("faraday-snapshot: {e}");
            ExitCode::FAILURE
        }
    }
}
