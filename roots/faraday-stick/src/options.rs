//! The command line (S6: no event is made here).

use faraday_port::Text;
use faraday_port::values::Orientation;
use faraday_stick_adapter::{Calibration, StickOptions};

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
        native: None,
        mount: Text::of(""),
        pointer_speed: 0,
        touch_grid: None,
        touch_name: None,
        calibration: Calibration::default(),
        files: Text::of(FILES),
        verbose: false,
        timings: false,
        boot_report: false,
        input: Vec::new(),
        frames: None,
        camera: None,
        no_camera: false,
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
                    "portrait" => Some(Orientation::Portrait),
                    "landscape" => Some(Orientation::Landscape),
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
                stick.touch_grid = Some((w, h));
            }
            "--touch-name" => stick.touch_name = Some(Text::of(&value()?)),
            "--touch-swap" => stick.calibration.swap = true,
            "--touch-invert-x" => stick.calibration.invert_x = true,
            "--touch-invert-y" => stick.calibration.invert_y = true,
            "--input" => stick.input.push(Text::of(&value()?)),
            "--frames" => stick.frames = Some(value()?.parse().map_err(|_| "bad frame count")?),
            "--camera" => stick.camera = Some(Text::of(&value()?)),
            "--no-camera" => stick.no_camera = true,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn read_flags(flags: &str) -> Result<Options, String> {
        read(flags.split_whitespace().map(String::from))
    }

    /// Invariant: every stick flag of `main` reaches the adapter's options, and a bad grid is refused.
    /// Regression: a flag accepted by the parser and not carried (as `--touch-grid` once was).
    /// Owner: root
    #[test]
    fn each_stick_flag_reaches_the_options() {
        let o = read_flags(
            "--native landscape --touch-grid 640x480 --touch-name Goodix --touch-swap \
             --touch-invert-x --touch-invert-y --input keys:/a --input /b --frames 3 \
             --camera /dev/video2 --no-camera",
        )
        .expect("parses")
        .stick;
        assert_eq!(o.native, Some(Orientation::Landscape));
        assert_eq!(o.touch_grid, Some((640, 480)));
        assert_eq!(o.touch_name.as_ref().map(Text::as_str), Some("Goodix"));
        assert!(o.calibration.swap && o.calibration.invert_x && o.calibration.invert_y);
        assert_eq!(o.input.len(), 2);
        assert_eq!(o.frames, Some(3));
        assert_eq!(o.camera.as_ref().map(Text::as_str), Some("/dev/video2"));
        assert!(o.no_camera);
        let none = read_flags("").expect("parses").stick;
        assert!(none.native.is_none() && none.touch_grid.is_none() && none.input.is_empty());
        for bad in [
            "--touch-grid 640",
            "--touch-grid 0x480",
            "--touch-grid axb",
            "--frames x",
        ] {
            assert!(read_flags(bad).is_err(), "{bad}");
        }
    }
}
