//! Renders the Argon QR Vault's screens to PNG, with no window, for
//! review (`docs/DEVELOPING.md`).
//!
//! ```text
//! cargo run -p aqrv-app --example snap -- WxH[@DPI] OUT_DIR
//! ```
//!
//! The Make tab (a sample, the settings, the working, the code, its
//! working and About, a typed item) and the Open tab (no file, a file
//! that is not a code, a code asking for its passphrase, the item shown,
//! a wrong passphrase) each step to one numbered picture in `OUT_DIR`.
//! The codes are made here at the smallest memory step, from a fixed
//! entropy: test data only.

use std::path::Path;
use std::process::ExitCode;

use aqrv::{Size, Step};
use aqrv_app::state::Field;
use aqrv_app::{Action, ArgonQrVault, OwnKind, Tab};
use faraday_shell_api::{
    App, BootState, Command, DisplayInfo, EntropyBytes, Event, FileKind, Key, SecureHardware,
};

const PASS: &str = "correct horse";

struct Shots<'a> {
    app: ArgonQrVault,
    dir: &'a Path,
    n: u32,
}

impl Shots<'_> {
    fn typed(&mut self, field: Field, text: &str) {
        self.app.tap(Action::Field(field));
        for c in text.chars() {
            self.app.event(Event::Key(Key::Char(c)));
        }
    }

    /// Answers what the app asks of a shell, with a fixed entropy; the
    /// files it would write are returned.
    fn serve(&mut self) -> Vec<(FileKind, Vec<u8>)> {
        let mut written = Vec::new();
        while let Some(command) = self.app.poll_command() {
            match command {
                Command::RequestEntropy => {
                    let bytes = std::array::from_fn(|i| 0x5a_u8.wrapping_add(i as u8 * 7));
                    self.app.event(Event::Entropy(EntropyBytes::new(bytes)));
                }
                Command::WriteFile { kind, bytes, .. } => {
                    written.push((kind, bytes));
                    self.app.event(Event::FileWritten { kind });
                }
                _ => {}
            }
        }
        written
    }

    fn shot(&mut self, name: &str) -> Result<(), String> {
        self.n += 1;
        let frame = self.app.frame();
        let path = self.dir.join(format!("{:02}-{name}.png", self.n));
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

    fn open_file(&mut self, bytes: Vec<u8>) {
        self.app.event(Event::File {
            kind: FileKind::Any,
            bytes,
        });
        self.app.finish_work();
    }
}

fn run(size: (u16, u16), dpi: u16, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut s = Shots {
        app: ArgonQrVault::new(|| {}),
        dir,
        n: 0,
    };
    s.app.event(Event::Display(DisplayInfo {
        width: size.0,
        height: size.1,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: true,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    s.app.finish_work();
    s.shot("make-item")?;
    s.app.tap(Action::Sample(1));
    s.shot("make-item-sample")?;
    s.app.tap(Action::Own);
    s.app.tap(Action::Kind(OwnKind::Note));
    s.typed(Field::Own, "a note for the vault");
    s.shot("make-item-typed")?;
    s.app.tap(Action::Sample(0));
    s.app.tap(Action::Next);
    s.shot("make-settings")?;
    s.app.tap(Action::Size(Size::Large));
    s.app.tap(Action::Step(Step::Mib64));
    s.typed(Field::Pass, PASS);
    s.typed(Field::Again, PASS);
    s.app.tap(Action::Eye);
    s.shot("make-settings-filled")?;
    s.app.tap(Action::Eye);
    s.app.tap(Action::Size(Size::Small));
    s.app.tap(Action::Make);
    s.app.event(Event::Tick { now_ms: 400 });
    s.shot("make-working")?;
    s.serve();
    s.app.finish_work();
    s.shot("make-code")?;
    s.app.tap(Action::ShowWorking);
    s.app.finish_work();
    s.shot("make-working-dialog")?;
    s.app.tap(Action::CloseDialog);
    s.app.tap(Action::SavePicture);
    s.app.tap(Action::SaveFile);
    let written = s.serve();
    s.shot("make-saved")?;
    s.app.tap(Action::About);
    s.shot("about")?;
    s.app.tap(Action::CloseDialog);

    let file = written
        .iter()
        .find(|(k, _)| *k == FileKind::Any)
        .map(|(_, b)| b.clone())
        .ok_or("no .aqrv file was written")?;
    s.app.tap(Action::Tab(Tab::Open));
    s.shot("open-empty")?;
    s.typed(Field::Path, "~/Downloads/argon-qr-vault.aqrv");
    s.shot("open-path")?;
    s.open_file(b"just some text".to_vec());
    s.shot("open-not-a-code")?;
    s.open_file(file.clone());
    s.shot("open-asking")?;
    s.typed(Field::OpenPass, "wrong horse");
    s.app.tap(Action::Unlock);
    s.app.finish_work();
    s.shot("open-does-not-open")?;
    s.open_file(file);
    s.typed(Field::OpenPass, PASS);
    s.app.tap(Action::Unlock);
    s.app.finish_work();
    s.shot("open-item")?;
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [spec, out] = args.as_slice() else {
        eprintln!("usage: snap WxH[@DPI] OUT_DIR");
        return ExitCode::FAILURE;
    };
    let (dims, dpi) = spec.split_once('@').unwrap_or((spec, "160"));
    let parsed = dims
        .split_once('x')
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?, dpi.parse().ok()?)));
    let Some((w, h, dpi)) = parsed else {
        eprintln!("snap: {spec} is not WxH[@DPI]");
        return ExitCode::FAILURE;
    };
    match run((w, h), dpi, Path::new(out)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("snap: {e}");
            ExitCode::FAILURE
        }
    }
}
