//! Renders Faraday's screens to PNG along one scripted tour.
//!
//! ```text
//! faraday-snapshot [--sd-card] WxH[@DPI] TESTKIT_DIR OUT_DIR [spend|themes|compact|seeds|keys|visit|copy|scan|public|seedfile|vaultway|kept|again|plan|inbox|transfer|upgrade|glance|fromvault]
//! ```
//!
//! `--sd-card` runs the tour as the Pi's stick shell starts the app:
//! with the removable medium an SD card (`Medium::SdCard`), so every
//! screen names it as the Pi does.
//!
//! `TESTKIT_DIR` is what `faraday-testkit` wrote; its files stand in
//! for a stick called TESTSTICK. Each step of the tour is one numbered
//! PNG in `OUT_DIR`. With `spend`, only the Spend tab's tour runs; with
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
//! and the wallet chip's list of the two; with `plan`, the backup's plan:
//! the presets and the questions with the map for a one-key wallet and
//! for Savings (2-of-3) with the whole sheet and with shares, the page
//! What is a share? opens, the map's own page on a small panel, the
//! checklist part done, the blank template written beside the screens
//! as `outbox-*`, and a 3-of-5's shares on Places and the checklist; with `inbox`, Stick visit offering the Inbox's
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
//! wallet with its plan, its chart from the backup up (§7.2).
//!
//! `@DPI` defaults to 160 (a desktop monitor); a real panel must give
//! its own, since the core picks `small`/`medium`/`wide` from physical
//! width, not pixels. The Pi's Waveshare panel is `480x640@286`; the
//! smallest panel the design supports is `240x320@143`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use faraday_core::testkit;
use faraday_core::vaults::VaultAction as V;
use faraday_core::{
    Action, BootPart, Faraday, Medium, Screen, StickInfo, StorageCommand, StorageEvent, bstep,
    qrow, qstep,
};
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
}

impl Tour {
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
                // The tour gives the upgrade's answers itself.
                StorageCommand::OpenDownloads
                | StorageCommand::SaveBoxes { .. }
                | StorageCommand::BootRead
                | StorageCommand::BootWrite { .. }
                | StorageCommand::BootForget => {}
            }
        }
    }

    /// A fresh process on the same display, as after a lock.
    fn restart(&mut self) {
        let size = self.size();
        let medium = self.app.medium;
        self.app = Faraday::new();
        self.app.medium = medium;
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
        self.app.press(a);
        self.pump();
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

fn run(
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
    };
    match only {
        Some("spend") => return spend_tour(&mut t),
        Some("themes") => return themes_tour(&mut t),
        Some("compact") => return compact_tour(&mut t),
        Some("seeds") => return seeds_tour(&mut t),
        Some("keys") => return keys_tour(&mut t),
        Some("visit") => return visit_tour(&mut t),
        Some("copy") => return copy_tour(&mut t),
        Some("scan") => return scan_tour(&mut t),
        Some("public") => return public_tour(&mut t),
        Some("seedfile") => return seedfile_tour(&mut t),
        Some("vaultway") => return vaultway_tour(&mut t),
        Some("kept") => return kept_tour(&mut t),
        Some("again") => return again_tour(&mut t),
        Some("plan") => return plan_tour(&mut t),
        Some("inbox") => return inbox_tour(&mut t),
        Some("transfer") => return transfer_tour(&mut t),
        Some("upgrade") => return upgrade_tour(&mut t),
        Some("glance") => return glance_tour(&mut t),
        Some("fromvault") => return from_vault_tour(&mut t),
        Some("first") => return first_tour(&mut t),
        _ => {}
    }
    t.shot("home-empty")?;
    // A stick arrives while nothing secret is held: the visit.
    t.sticks(true);
    // Every file Faraday reads comes in ticked but a seed's.
    t.shot("visit")?;
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
    t.shot("restore-shares")?;
    t.press(Action::RRebuild);
    let fp = t
        .app
        .session
        .wallets
        .first()
        .and_then(|w| w.policy.keys()[0].fingerprint())
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
        osk_bip::bip39::Language::English,
        &testkit::test_words("zebra"),
    )
    .map_err(|e| format!("{e:?}"))?;
    let digits = osk_codec::seedqr::to_digits(&zebra);
    t.app.event(Event::Scanned {
        bytes: digits.expose().as_bytes().to_vec(),
    });
    t.pump();
    let zebra_fp = [0x5d, 0x38, 0x83, 0x76];
    if t.app.session.keys.len() != keys_before + 1
        || !t
            .app
            .session
            .keys
            .iter()
            .any(|k| k.master.fingerprint().0 == zebra_fp)
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
    t.press(Action::BPreset(0));
    t.press(Action::BAnswer(qrow::SPLIT, 1));
    t.press(Action::BChecklist);
    t.press(Action::BStep(bstep::BLANK));
    t.shot("backup-blank")?;
    t.press(Action::BNext(bstep::BLANK));
    t.press(Action::BReveal);
    t.shot("backup-seed")?;
    // The copy scanned with its third word drawn wrong.
    t.press(Action::BScan);
    let wrong = copy_digits(&t, Some(2))?;
    t.app.event(Event::Scanned { bytes: wrong });
    t.shot("backup-scan-wrong")?;
    t.press(Action::Cancel);
    // Another paper form: test key 1 split into two Seed XOR parts.
    t.press(Action::BXor(2));
    t.shot("backup-xor")?;
    t.press(Action::BPaperHide);
    // The copy typed back.
    t.press(Action::BCheck);
    for c in copy_digits(&t, None)? {
        t.app.event(Event::Key(Key::Char(char::from(c))));
    }
    t.press(Action::BCheck);
    t.press(Action::BStep(bstep::SHEETS));
    t.shot("backup-split")?;
    t.press(Action::BStep(bstep::ENVELOPE));
    t.shot("backup-envelope")?;
    // Every item done: what waits for the stick, and the way to one.
    t.press(Action::BNext(bstep::ENVELOPE));
    t.shot("backup-done")?;
    t.press(Action::WriteAsk);
    t.shot("backup-write-out")?;
    t.press(Action::Cancel);
    // The sheets went For the stick as PDFs with the checklist; written
    // beside the shots for a look.
    t.press(Action::Backup(0));
    t.press(Action::BOut(3));
    for name in ["blank-template-24-words.pdf", "savings-backup.pdf"] {
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
    }
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
    let bacon = t.app.session.keys[0].master.fingerprint().0;
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
    if t.app.create_keys().is_ok() {
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
    let want = t.app.session.wallets[0].policy.to_descriptor_checksummed();
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
    if w.policy.record().is_none_or(|r| r.n() != 3 || r.t() != 2)
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
    // Unlocked: its keys and wallets wait on Files until chosen.
    if t.app.screen != Screen::Files || !t.app.session.keys.is_empty() {
        return Err("an unlock loaded keys before they were chosen".into());
    }
    t.shot("files-vault-open")?;
    t.press(Action::Vault(V::LoadChosen(0)));
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
    t.press(Action::BPreset(1));
    t.press(Action::BChecklist);
    if t.app.backup.as_ref().and_then(|b| b.open) != Some(bstep::COPY) {
        t.press(Action::BStep(bstep::COPY));
    }
    t.press(Action::BReveal);
    t.press(Action::BCheck);
    for k in 0..2 {
        if k > 0 {
            t.press(Action::BCheckClear);
        }
        for c in copy_digits(&t, None)? {
            t.app.event(Event::Key(Key::Char(char::from(c))));
        }
    }
    t.press(Action::BCheck);
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
    t.press(Action::Vault(V::Add));
    type_text(&mut t, "Test Person");
    t.app.event(Event::Key(Key::Tab));
    type_text(&mut t, "test@example.com");
    t.shot("vault-gpg-make")?;
    t.press(Action::Vault(V::FormSave));
    let gpg_files: Vec<String> = t
        .app
        .outbox
        .iter()
        .filter(|i| i.name.ends_with(".asc"))
        .map(|i| i.name.clone())
        .collect();
    if gpg_files.len() != 2 || !gpg_files.iter().any(|n| n.ends_with("-revocation.asc")) {
        return Err(format!("making a GPG key left {gpg_files:?} in the Outbox"));
    }
    t.shot("vault-gpg-key")?;
    t.press(Action::Vault(V::GpgSignPick));
    t.shot("vault-gpg-sign")?;
    let signed = t
        .app
        .inbox
        .iter()
        .position(|i| i.name == "vault-entries.txt")
        .ok_or("no file to sign")?;
    t.press(Action::Vault(V::GpgSign(signed)));
    if !t
        .app
        .outbox
        .iter()
        .any(|i| i.name == "vault-entries.txt.asc")
    {
        return Err("signing left no signature in the Outbox".into());
    }
    println!("a GPG key made in the vault signed a file");
    // Secure Boot keys made in the vault, the enrolment files, and the
    // test image signed after its hash is shown.
    t.press(Action::Vault(V::Category(5)));
    t.press(Action::Vault(V::Add));
    type_text(&mut t, "Test");
    t.press(Action::Vault(V::FormSave));
    t.tick();
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
    let (_, main) = faraday_vault::open(&sealed, testkit::VAULT_PASSPHRASES[0].as_bytes())
        .map_err(|e| e.reason())?;
    let (_, decoy) = faraday_vault::open(&sealed, testkit::VAULT_PASSPHRASES[1].as_bytes())
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
        .backups()
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
        faraday_vault::open(&made, p.as_bytes()).map_err(|e| format!("{p}: {}", e.reason()))?;
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
    boot_tour(&mut t)?;
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
    // New key's hint for cards, with its en dash.
    t.press(Action::KeyGen(None));
    t.press(Action::KWords(12));
    t.press(Action::KWay(faraday_core::keygen::Way::Cards.index()));
    t.press(Action::KNext);
    t.shot("keygen-cards")?;
    // Last, since nothing after it does anything: a self-test that failed.
    t.app.run_selftest_with(&[osk_selftest::Check {
        name: "BIP-39 vector (entropy → words → seed)",
        run: || false,
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
        .map(|k| k.master.fingerprint().0)
        .ok_or_else(|| "test key 1 is not loaded".to_string())
}

/// A fresh session at boot: the boot stick's files are copied into
/// memory and the sheet says what was read; pulling the stick puts the
/// vault's passphrase on the sheet. A wrong passphrase stays; Not now
/// leaves Home leading with the vault's Unlock; Choose what to import
/// lists the wallets, keys and files; the right passphrase imports
/// everything. Then a wallet made here is saved into the vault.
fn boot_tour(t: &mut Tour) -> Result<(), String> {
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
        || !t.app.inbox.iter().any(|i| i.name == "vault.ofv")
    {
        return Err("the boot stick's files did not come up for the import".into());
    }
    t.shot("boot-import-stick")?;
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    t.pump();
    if t.app.sheet != Some(Sheet::Import) || t.app.import_field().is_none() {
        return Err(
            "pulling the boot stick did not put the vault's passphrase on the sheet".into(),
        );
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
    // Not now leaves Home leading with the vault's Unlock, which opens
    // the sheet again.
    t.press(Action::Import(I::Later));
    t.shot("home-import-waiting")?;
    t.press(faraday_core::boot_import::OPEN);
    if t.app.sheet != Some(Sheet::Import) {
        return Err("the import did not open again from Home".into());
    }
    // Choose what to import: the wallets, the keys and the files.
    t.press(Action::Import(I::Choose));
    let (w, h) = t.size();
    let scroll = |t: &mut Tour, dy: i16| {
        t.app.event(Event::Scroll {
            x: w / 2,
            y: h / 2,
            dy,
        });
    };
    scroll(t, 1400);
    t.shot("boot-import-wallets")?;
    scroll(t, i16::MAX);
    t.shot("boot-import-end")?;
    scroll(t, i16::MIN);
    t.press(Action::Vault(V::FocusPassphrase));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.shot("boot-unlock")?;
    t.press(Action::Import(I::Submit));
    t.tick();
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
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
    t.press(Action::CBackup(1));
    for i in t.app.plan_shape(built).here().into_iter().skip(1) {
        t.press(Action::BAnswer(faraday_core::qrow::VAULTS, i as u8));
    }
    for (v, i) in t
        .app
        .plan_shape(built)
        .here()
        .into_iter()
        .enumerate()
        .skip(1)
    {
        t.press(Action::BAnswer(
            faraday_core::qrow::VAULTS + v as u8,
            i as u8,
        ));
    }
    // The description as files too, for the public files item below.
    t.press(Action::BAnswer(
        faraday_core::qrow::WALLET,
        faraday_core::plan::wallet::FILES as u8,
    ));
    t.press(Action::BChecklist);
    t.press(Action::BStep(faraday_core::bstep::VAULT));
    // Making the checklist saved the plan into the vault already.
    let before = t.app.vaults.open[0].changes;
    for k in t.app.backup_keys(built) {
        t.press(Action::BKey(k));
        t.press(Action::BVaultSave(0));
    }
    t.shot("create-backup-vault")?;
    t.press(Action::Vault(V::SaveWallet(built)));
    t.shot("create-backup-vault-saved")?;
    // The wallet and the two new keys; test key 1 was there already.
    let saved = t.app.vaults.open[0].changes - before;
    if saved != 3 {
        return Err(format!("the checklist saved {saved} records, not 3"));
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
    // Restore begins with the transaction to sign.
    t.press(Action::Lock);
    t.press(Action::Nav(Screen::Start));
    t.shot("start-empty")?;
    t.press(Action::RestoreWallet);
    t.shot("restore-psbt")?;
    t.press(Action::RNext(0));
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
        id: "sdb2@sdb#2".into(),
        label: "OSKDATA".into(),
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

fn transfer_tour(t: &mut Tour) -> Result<(), String> {
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
    t.app.online = true;
    t.app.session = faraday_core::wallet::Session::on(testkit::NET);
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
    t.shot("transfer-saved")
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

/// The small panel's Home, and Create a wallet with a new key from dice,
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
    type_text(t, "Ride the 7 bus");
    t.shot("keygen-key-passphrase")?;
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
    t.press(Action::CBackup(1));
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

/// A wallet from seed words alone: Restore's Type the seeds, two seeds
/// typed with the next steps, the shape (M of N on its sliders, the
/// third cosigner's xpub, the kind and the path) and the Check card;
/// then the Spend tab's words route with Add another seed and the
/// sliders.
fn seeds_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::family::{FamilyAction as F, Route, page};
    use faraday_core::seeds::{Focus, SLIDE_N, SeedsAction as S};
    t.press(Action::Lock);
    t.sticks(false);
    t.tick();
    t.press(Action::Network(testkit::NET));
    t.press(Action::RestoreWallet);
    t.press(Action::RNext(0));
    t.shot("restore-type-seeds")?;
    scroll(t, 300);
    t.shot("restore-type-seeds-foot")?;
    t.press(Action::RSeeds);
    for n in [0, 1] {
        t.press(Action::Entry(None));
        t.type_key(n);
    }
    if t.app.screen != Screen::Restore {
        return Err("Add a key did not return to Restore".into());
    }
    t.shot("restore-seeds-first")?;
    t.press(Action::Seeds(S::Shape));
    t.press(Action::Slide(SLIDE_N, 3));
    t.press(Action::Seeds(S::Focus(Focus::Cosigner(0))));
    type_text(t, &testkit::key(2, "m/48'/1'/0'/2'"));
    t.app.event(Event::Key(Key::Enter));
    t.shot("restore-seeds-shape")?;
    // Down the shape to Make the wallet: further on a small panel.
    let steps = if t.size().0 < 700 { 5 } else { 2 };
    for k in 1..=steps {
        scroll(t, 300);
        t.shot(&format!("restore-seeds-shape-{k}"))?;
    }
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
    t.press(Action::Family(F::Holding(Route::Words)));
    t.press(Action::Entry(None));
    t.type_key(0);
    t.shot("spend-words-one-seed")?;
    t.press(Action::Entry(None));
    t.type_key(1);
    t.shot("spend-words-two-seeds")?;
    t.press(Action::Slide(SLIDE_N, 3));
    t.press(Action::Seeds(S::Focus(Focus::Cosigner(0))));
    type_text(t, &testkit::key(2, "m/48'/1'/0'/2'"));
    t.app.event(Event::Key(Key::Enter));
    for k in 1..=steps {
        scroll(t, 300);
        t.shot(&format!("spend-words-shape-{k}"))?;
    }
    t.press(Action::Seeds(S::Make));
    t.shot("spend-words-made")?;
    if t.app.family_wallet().is_none() {
        return Err("the Spend tab's seeds made no wallet".into());
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
        .map(|k| k.master.fingerprint().0)
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
fn visit_tour(t: &mut Tour) -> Result<(), String> {
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
            kind: classify(name, &bytes),
            bytes,
            secret: false,
            picture: None,
        });
    }
    t.app.outbox.push(faraday_core::Item {
        name: "seed-words.txt".into(),
        kind: faraday_core::wallet::FileKind::Text,
        bytes: b"words".to_vec(),
        secret: true,
        picture: None,
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
fn copy_digits(t: &Tour, wrong: Option<usize>) -> Result<Vec<u8>, String> {
    let b = t.app.backup.as_ref().ok_or("no backup")?;
    let key = t.app.session.keys.get(b.key).ok_or("no key")?;
    let words = key.words.as_deref().ok_or("no words")?;
    let m = osk_bip::bip39::Mnemonic::parse(key.language, words.as_str())
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

/// The backup's plan from preset `preset`, with `ticks` ticked
/// (`BAnswer`'s list and row), then its checklist.
fn to_checklist(t: &mut Tour, preset: u8, ticks: &[(u8, u8)]) {
    t.press(Action::BPreset(preset));
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
fn public_tour(t: &mut Tour) -> Result<(), String> {
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
    t.press(Action::CBackup(2));
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
    // The description goes as files too: the public files item.
    let files = faraday_core::plan::wallet::FILES;
    if !t
        .app
        .backup
        .as_ref()
        .is_some_and(|b| b.answers.wallet[files])
    {
        t.press(Action::BAnswer(faraday_core::qrow::WALLET, files as u8));
    }
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
    to_checklist(t, 2, &[(qrow::FORM, 1)]);
    t.press(Action::BStep(bstep::PUBLIC));
    t.press(Action::PublicOut(built, 8));
    scroll_to(t, Action::PublicOut(built, 8));
    t.shot("public-backup")?;
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
    t.press(Action::Vault(V::Add));
    type_text(t, "Test Person");
    t.app.event(Event::Key(Key::Tab));
    type_text(t, "test@example.com");
    t.press(Action::Vault(V::FormSave));
    // On a small panel the key's page is opened from the list.
    t.press(Action::Vault(V::Item(0)));
    scroll_to(t, Action::CodePng(Code::GpgRevocation));
    t.shot("public-gpg")?;
    t.press(Action::CodePng(Code::GpgKey));
    t.press(Action::CodePng(Code::GpgRevocation));
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

fn copy_tour(t: &mut Tour) -> Result<(), String> {
    t.load_kit()?;
    t.press(Action::Backup(0));
    to_checklist(t, 0, &[]);
    t.press(Action::BNext(bstep::BLANK));
    t.press(Action::BReveal);
    t.app.storage(StorageEvent::Cameras(Vec::new()));
    scroll_to(t, Action::BCheck);
    t.shot("copy-no-camera")?;
    t.press(Action::BCheck);
    for &c in &copy_digits(t, None)?[..12] {
        t.app.event(Event::Key(Key::Char(char::from(c))));
    }
    t.shot("copy-typing")?;
    t.press(Action::BCheck);
    t.press(Action::BCheckClear);
    t.app.storage(StorageEvent::Cameras(vec![(
        "/dev/video0".into(),
        "Integrated Camera".into(),
    )]));
    scroll_to(t, Action::BScan);
    t.shot("copy-camera")?;
    t.press(Action::BScan);
    let wrong = copy_digits(t, Some(4))?;
    t.app.event(Event::Scanned { bytes: wrong });
    t.shot("copy-scan-differs")?;
    let right = copy_digits(t, None)?;
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
fn seedfile_tour(t: &mut Tour) -> Result<(), String> {
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
    to_checklist(t, 1, &[(qrow::SEEDS, 3)]);
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
fn kept_tour(t: &mut Tour) -> Result<(), String> {
    t.load_kit()?;
    t.press(Action::Backup(0));
    to_checklist(t, 0, &[(qrow::SEEDS, 3)]);
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
    to_checklist(t, 1, &[]);
    t.press(Action::BNext(bstep::BLANK));
    t.press(Action::BReveal);
    t.press(Action::BCheck);
    let words = testkit::test_words(testkit::TEST_SEEDS[0].0);
    let mn = osk_bip::bip39::Mnemonic::parse(osk_bip::bip39::Language::English, &words)
        .map_err(|e| format!("{e:?}"))?;
    let digits = osk_codec::seedqr::to_digits(&mn);
    let typed = String::from_utf8_lossy(digits.expose().as_bytes()).into_owned();
    type_text(t, &typed);
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
    to_checklist(t, 2, &every);
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
        .master
        .fingerprint();
    t.press(Action::KeyWallet(fp.0, 1));
    t.press(Action::Seeds(S::Make));
    let w = t.app.session.wallets.len() - 1;
    t.press(Action::Backup(w));
    to_checklist(t, 1, &[]);
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
fn plan_tour(t: &mut Tour) -> Result<(), String> {
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
    t.press(Action::Backup(spending));
    t.shot("plan-presets-one-key")?;
    t.press(Action::BPreset(1));
    t.shot("plan-seeds-one-key")?;
    t.press(Action::BQ(qstep::PLACES));
    t.shot("plan-places-one-key")?;
    t.press(Action::Backup(savings));
    t.press(Action::BPreset(0));
    t.shot("plan-seeds-two-of-three")?;
    t.press(Action::BQ(qstep::PLACES));
    t.shot("plan-places-two-of-three")?;
    // Its own share: the slider, the shares and the audit; then the page
    // What is a share? opens.
    t.press(Action::BAnswer(qrow::SPLIT, 1));
    scroll_to(t, Action::Slide(faraday_core::seeds::SLIDE_OMIT, 0));
    t.shot("plan-places-shares-two-of-three")?;
    t.press(Action::LearnShares);
    t.shot("plan-learn-shares")?;
    t.press(Action::Cancel);
    if t.app.backup_questions().contains(&qstep::MAP) {
        t.press(Action::BQ(qstep::MAP));
        t.shot("plan-map-two-of-three")?;
    }
    t.press(Action::BChecklist);
    t.press(Action::BStep(bstep::SHEETS));
    t.shot("plan-checklist-part-done")?;
    t.press(Action::BStep(bstep::ENVELOPE));
    t.shot("plan-envelopes")?;
    // A 3-of-5's shares: up to two keys left off each, on Places and on
    // the checklist's Shares card.
    let five = t
        .app
        .session
        .add_wallet("Five", &testkit::three_of_five(), "test")
        .map_err(|e| e.text())?;
    t.press(Action::Backup(five));
    t.press(Action::BPreset(0));
    // With no seed here the preset opens Places itself.
    if t.app.backup.as_ref().and_then(|b| b.q) != Some(qstep::PLACES) {
        t.press(Action::BQ(qstep::PLACES));
    }
    t.press(Action::BAnswer(qrow::SPLIT, 1));
    scroll_to(t, Action::Slide(faraday_core::seeds::SLIDE_OMIT, 0));
    t.shot("plan-places-shares-three-of-five")?;
    t.press(Action::BChecklist);
    t.press(Action::BStep(bstep::SHEETS));
    t.shot("plan-shares-three-of-five")?;
    // A vault per seed: Spending's one vault and its checklist item; then
    // Savings with test key 2 typed in too, two seeds here and a vault
    // each, on Seeds, Places, the map and the second vault's item.
    t.press(Action::Backup(spending));
    t.press(Action::BPreset(1));
    t.press(Action::BChecklist);
    t.press(Action::BStep(bstep::VAULT));
    t.shot("plan-checklist-vault-one-key")?;
    t.press(Action::Entry(None));
    t.type_key(1);
    t.press(Action::Backup(savings));
    t.press(Action::BPreset(1));
    if t.app.backup.as_ref().and_then(|b| b.q) != Some(qstep::SEEDS) {
        t.press(Action::BQ(qstep::SEEDS));
    }
    t.shot("plan-seeds-vaults-two-of-three")?;
    t.press(Action::BQ(qstep::PLACES));
    t.shot("plan-places-vaults-two-of-three")?;
    if t.app.backup_questions().contains(&qstep::MAP) {
        t.press(Action::BQ(qstep::MAP));
        t.shot("plan-map-vaults-two-of-three")?;
    }
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

/// The Wallets card at a glance: a one-key wallet with no plan, then with
/// Paper and vault; a 2-of-3 with test keys 1 and 2 here, key 2 with a
/// passphrase, and test key 3 a cosigner, with Paper and vault and key
/// 2's passphrase in its vault too. On a small panel, each card's At a
/// glance row and the chart's page. The sheets a press on the chart opens
/// (`docs/NEW-WALLET.md` §9): the wallet, a key here, a cosigner's key, a
/// place, a vault, a line inside a place; Where it is; and in the open
/// vault's view, a place's.
fn glance_tour(t: &mut Tour) -> Result<(), String> {
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
            .key_text(&t.app.session.keys[0].master)
            .map_err(|e| e.to_string())?;
        t.app
            .session
            .add_wallet("Spending", &format!("wpkh({text}/<0;1>/*)"), "test")
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
    t.press(Action::BPreset(1));
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
            &testkit::test_words(testkit::TEST_SEEDS[1].0),
            "glance",
            "Test key 2",
            None,
        )
        .map_err(|e| e.text())?;
    let mut elsewhere = faraday_core::wallet::Session::default();
    elsewhere
        .add_words(
            &testkit::test_words(testkit::TEST_SEEDS[2].0),
            "Test key 3",
            None,
        )
        .map_err(|e| e.text())?;
    let masters = [
        &t.app.session.keys[0].master,
        &t.app.session.keys[1].master,
        &elsewhere.keys[0].master,
    ];
    let keys: Vec<String> = masters
        .iter()
        .map(|m| NewKind::Multi.key_text(m))
        .collect::<Result<_, _>>()?;
    let two = t
        .app
        .session
        .add_wallet("Savings", &NewKind::Multi.descriptor(2, &keys), "test")
        .map_err(|e| e.text())?;
    shot_card(t, two, "glance-two-of-three-no-plan")?;
    t.press(Action::Backup(two));
    t.press(Action::BPreset(1));
    // Key 2's passphrase into its vault too.
    t.press(Action::BAnswer(qrow::PASS + 1, 3));
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
    t.press(Action::BPreset(1));
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
    t.press(Action::BPreset(1));
    t.press(Action::BAnswer(qrow::PASS + 1, 3));
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
fn first_tour(t: &mut Tour) -> Result<(), String> {
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
    let paper_vault = Preset::ALL
        .iter()
        .position(|p| *p == Preset::PaperVault)
        .ok_or("no Paper and vault")? as u8;
    t.press(Action::BPreset(paper_vault));
    t.press(Action::BQ(qstep::PLACES));
    let at = {
        let a = &t.app.backup.as_ref().ok_or("no backup")?.answers;
        (0..a.places)
            .find(|&p| a.stick_at(0, p))
            .ok_or("Vault 1's stick has no place")?
    };
    t.press(Action::BAnswer(qrow::STICKS, at as u8));
    t.shot("first-places-stickless")?;
    t.press(Action::BAnswer(qrow::STICKS, at as u8));
    t.press(Action::BChecklist);
    // The copy checked, typed back.
    let digits = {
        let key = t.app.session.keys.last().ok_or("no key")?;
        let words = key.words.as_ref().ok_or("no words")?.to_string();
        let m = osk_bip::bip39::Mnemonic::parse(key.language, &words).map_err(|e| e.to_string())?;
        osk_codec::seedqr::to_digits(&m)
            .expose()
            .as_bytes()
            .to_vec()
    };
    t.press(Action::BCheck);
    for c in digits {
        t.app.event(Event::Key(Key::Char(char::from(c))));
    }
    t.press(Action::BCheck);
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
        .backup_items()
        .into_iter()
        .filter_map(bstep::item)
        .all(|it| t.app.backup_item_done(it));
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
/// with one key here, its PSBT in Files, reaches Signers with Add a key
/// here and Collect a signature.
fn from_vault_tour(t: &mut Tour) -> Result<(), String> {
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
        here < faraday_core::wallet::needed(wl) && t.app.psbt_for(w).is_some()
    });
    let Some(short) = short else {
        return Err("no wallet short of keys has its PSBT in Files".into());
    };
    t.press(Action::OpenWallet(short));
    t.shot("vault-card-sign")?;
    t.press(Action::Family(F::SpendFrom(short)));
    t.press(Action::Step(faraday_core::wallet::step::SIGNERS));
    t.shot("vault-spend-signers")
}

fn spend_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::family::{FamilyAction as F, Route, page};
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
    t.press(fam(F::Holding(Route::Vault)));
    t.shot("spend-vault-none")?;
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
    t.press(fam(F::Choose(savings)));
    t.shot("spend-wallet-chosen")?;
    t.press(fam(F::Next(page::OPEN)));
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
    // Check is already closed: "It matches" on the Spend tab's own check
    // page marked this wallet checked this power-on.
    t.press(Action::StepNext(faraday_core::wallet::step::TRANSACTION));
    t.shot("spend-signers")?;
    let fp = {
        let w = &t.app.session.wallets[savings];
        t.app
            .session
            .slots(w)
            .iter()
            .find(|s| s.held_by.is_none())
            .and_then(|s| s.fingerprint)
            .ok_or("every key is here already")?
    };
    t.press(Action::Entry(Some(fp.0)));
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
    let first = t.app.session.keys[0].master.fingerprint().0;
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
    if let Some(fp) = t.app.session.keys.first().map(|k| k.master.fingerprint().0) {
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
            "usage: faraday-snapshot [--sd-card] WxH[@DPI] TESTKIT_DIR OUT_DIR [spend|themes|compact|seeds|keys|visit|copy|scan|public|seedfile|vaultway|kept|again|plan|inbox|transfer|upgrade|glance|fromvault|first]"
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
    let only = args.get(4).map(String::as_str);
    if only.is_some_and(|m| {
        ![
            "spend",
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
            "inbox",
            "transfer",
            "upgrade",
            "glance",
            "fromvault",
            "first",
        ]
        .contains(&m)
    }) {
        eprintln!(
            "the tour is spend, themes, compact, seeds, keys, visit, copy, scan, public, seedfile, vaultway, kept, again, plan, inbox, transfer, upgrade, glance, fromvault or first"
        );
        return ExitCode::from(2);
    }
    match run(
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
