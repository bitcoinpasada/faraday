//! The command line (S6: no event is made here).

use faraday_stick_adapter::StickOptions;
use faraday_text::Text;

/// Where the disk process's two pipes are, and where the carry waits between processes: fixed
/// paths the startup script makes (06 §5.1).
const FIFO_DIR: &str = "/run/faraday-disk";
const RAM_DIR: &str = "/run/faraday";
const FILES: &str = "/mnt/microsd";

/// What the command line gives: the stick adapter's options and the disk process's two paths.
pub struct Options {
    pub stick: StickOptions,
    pub fifo: String,
    pub ram_dir: String,
}

/// The command line of this process; one that cannot be read is said and the process ends.
#[must_use]
pub fn parse() -> Options {
    match read(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(why) => {
            eprintln!("faraday: {why}");
            std::process::exit(2);
        }
    }
}

/// The options the arguments give, or why they give none.
pub fn read(args: impl IntoIterator<Item = String>) -> Result<Options, String> {
    let mut stick = StickOptions {
        fb: Text::of("/dev/fb0"),
        size: None,
        depth: 0,
        dpi: 0,
        native: false,
        mount: Text::of(""),
        pointer_speed: 0,
        touch_grid: true,
        files: Text::of(FILES),
        verbose: false,
        timings: false,
        boot_report: false,
    };
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{arg} needs a value"));
        match arg.as_str() {
            "--fb" => stick.fb = Text::of(&value()?),
            "--size" => {
                let v = value()?;
                let (w, h) = v.split_once('x').ok_or("--size wants WxH")?;
                stick.size = Some((
                    w.parse().map_err(|_| "bad width")?,
                    h.parse().map_err(|_| "bad height")?,
                ));
            }
            "--depth" => {
                stick.depth = match value()?.parse() {
                    Ok(bits @ (16 | 24 | 32)) => bits,
                    _ => return Err(String::from("--depth wants 16, 24 or 32")),
                };
            }
            "--dpi" => stick.dpi = value()?.parse().map_err(|_| "bad dpi")?,
            "--native" => {
                stick.native = match value()?.as_str() {
                    "portrait" => true,
                    "landscape" => false,
                    _ => return Err(String::from("--native wants portrait or landscape")),
                };
            }
            "--mount" => {
                let v = value()?;
                if v != "90" && v != "270" {
                    return Err(String::from("--mount wants 90 or 270"));
                }
                stick.mount = Text::of(&v);
            }
            "--pointer-speed" => {
                let speed: f32 = value()?.parse().map_err(|_| "bad pointer speed")?;
                if !(speed.is_finite() && speed > 0.0) {
                    return Err(String::from("--pointer-speed wants a positive number"));
                }
                // Tenths of a device pixel per unit of motion (09 §6).
                stick.pointer_speed = (speed * 10.0).round().clamp(1.0, 255.0) as u8;
            }
            "--touch-grid" => {
                let v = value()?;
                let (w, h) = v.split_once('x').ok_or("--touch-grid wants WxH")?;
                let (w, h): (u16, u16) = (
                    w.parse().map_err(|_| "bad touch grid width")?,
                    h.parse().map_err(|_| "bad touch grid height")?,
                );
                if w == 0 || h == 0 {
                    return Err(String::from("--touch-grid has no pixels"));
                }
                // The adapter's options say only the default grid (09 §6): a size is read and
                // not carried.
            }
            "--files" => stick.files = Text::of(&value()?),
            "--verbose" => stick.verbose = true,
            "--timings" => stick.timings = true,
            "--boot-report" => stick.boot_report = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(Options {
        stick,
        fifo: String::from(FIFO_DIR),
        ram_dir: String::from(RAM_DIR),
    })
}
