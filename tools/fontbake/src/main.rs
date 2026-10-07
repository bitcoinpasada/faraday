//! Turns the vendored fonts in `tools/fonts/` into glyph outlines for
//! `osk-ui`.
//!
//! Run from the repository root (`just fonts`). See `README.md` alongside
//! this file for the outline format. The device fills these outlines at
//! whatever pixel size a display asks for; nothing here is rasterised.
//!
//! The two text faces and the two icon faces come from files committed in
//! `tools/fonts/`. The mono face comes from Iosevka, whose release archive
//! is downloaded into `tools/fonts/iosevka/` and checked against
//! [`IOSEVKA_SHA256`]. The CJK fallback face comes from the four Noto Sans
//! CJK region subsets, which are 23 MB together and are not committed:
//! this tool downloads each into `tools/fonts/cjk/` when it is missing and
//! checks it against the digest in [`CJK_SOURCES`]. Only the baked `.outl`
//! files are committed, so building the crates never needs the fonts, the
//! network or this tool.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use osk_bip::bip39::Language;
use osk_crypto::sha256;
use osk_ui::widgets::keyboard::{self, KeyboardKind};
use ttf_parser::{Face, GlyphId, OutlineBuilder, Rect};

/// What a face's file holds.
#[derive(PartialEq, Eq, Clone, Copy)]
enum Kind {
    /// The text glyph set: ASCII, Latin-1 and the UI symbols.
    Text,
    /// The icon glyph set: SeedSigner's `U+E9xx` icons and the Font
    /// Awesome glyphs the UI names.
    Icon,
}

/// A face to bake: its identifier in `generated.rs` and its font file.
struct FaceSpec {
    name: &'static str,
    variant: &'static str,
    file: &'static str,
    kind: Kind,
}

const FACES: &[FaceSpec] = &[
    FaceSpec {
        name: "regular",
        variant: "Regular",
        file: "NotoSans-Regular.ttf",
        kind: Kind::Text,
    },
    FaceSpec {
        name: "semibold",
        variant: "SemiBold",
        file: "NotoSans-SemiBold.ttf",
        kind: Kind::Text,
    },
    FaceSpec {
        name: "regular-narrow",
        variant: "RegularNarrow",
        file: "NotoSans-SemiCondensed.ttf",
        kind: Kind::Text,
    },
    FaceSpec {
        name: "semibold-narrow",
        variant: "SemiBoldNarrow",
        file: "NotoSans-SemiCondensedSemiBold.ttf",
        kind: Kind::Text,
    },
    FaceSpec {
        name: "mono",
        variant: "Mono",
        file: IOSEVKA_FILE,
        kind: Kind::Text,
    },
    FaceSpec {
        name: "icon",
        variant: "Icon",
        file: "seedsigner-icons.otf",
        kind: Kind::Icon,
    },
];

/// UI symbols outside the two contiguous text ranges.
const SYMBOLS: &[char] = &[
    '•', '▾', '▸', '←', '→', '↑', '↓', '✓', '✗', '×', '…', '°', '₿',
];

/// The icon code points, and where each comes from. `U+E9xx` are
/// SeedSigner's own icon face; the rest are Font Awesome 6 Free Solid.
/// This list must match `Icon` in `core/osk-ui/src/widgets/icon.rs`; the
/// crate's tests assert that every `Icon` resolves in the outline file.
const ICONS: &[u32] = &[
    // seedsigner-icons.otf
    0xE900, // scan
    0xE901, // key
    0xE902, // settings
    0xE903, // tools
    0xE904, // back
    0xE905, // check
    0xE909, // chevron left
    0xE90A, // chevron right
    0xE90B, // chevron up
    0xE910, // power
    0xE912, // info
    0xE913, // success
    0xE914, // warning
    0xE915, // error
    0xE918, // derivation
    0xE91A, // fingerprint
    0xE91B, // passphrase
    0xE91D, // bitcoin
    0xE920, // QR code
    0xE921, // sign
    0xE922, // backspace
    0xE923, // space
    // Font_Awesome_6_Free-Solid-900.otf
    0xE2C5, // vault
    0xF002, // magnifying-glass
    0xF00A, // table-cells
    0xF00C, // check
    0xF015, // house
    0xF019, // download
    0xF023, // lock
    0xF024, // flag
    0xF02D, // book
    0xF03A, // list
    0xF066, // compress
    0xF06E, // eye
    0xF070, // eye-slash
    0xF0A0, // hard-drive
    0xF0AE, // list-check
    0xF0C4, // scissors
    0xF0C5, // copy
    0xF0CB, // list-ol
    0xF0E0, // envelope
    0xF0EA, // paste
    0xF11C, // keyboard
    0xF121, // code
    0xF12B, // superscript
    0xF141, // ellipsis
    0xF14E, // compass
    0xF15B, // file
    0xF1F8, // trash
    0xF21B, // user-secret
    0xF292, // hashtag
    0xF362, // right-left (swap)
    0xF3ED, // shield-halved
    0xF522, // dice
    0xF542, // diagram-project
    0xF543, // receipt
    0xF555, // wallet
    0xF560, // check-double
    0xF56E, // file-export
    0xF5FD, // layer-group
    0xF658, // envelope-open-text
];

/// The mono face: Iosevka's default Regular cut, SIL OFL 1.1
/// (`tools/fonts/OFL.txt`). It advances 0.5 em and tells 0 from O and 1
/// from l from I, which is what everything read character by character
/// needs (`docs/PLANNING.md` §16.126).
const IOSEVKA_VERSION: &str = "34.8.1";

/// The Regular TTF, unpacked from the archive into `tools/fonts/iosevka/`
/// and named relative to `tools/fonts/`.
const IOSEVKA_FILE: &str = "iosevka/Iosevka-Regular.ttf";

/// The release archive's digest. A mismatch aborts the bake.
const IOSEVKA_SHA256: &str = "0ea6f8a7d37444a974b45d24995b3a58f80924b71e8c51f0d91a879fa87c36be";

/// Where the CJK fallback face's glyphs come from: the Noto Sans CJK
/// region subsets, Regular weight, SIL OFL 1.1 (`tools/fonts/OFL.txt`).
/// Each is downloaded into `tools/fonts/cjk/` on first run and checked
/// against its digest; a mismatch aborts the bake.
struct CjkSource {
    /// The region subset's two-letter code, which is also its directory.
    region: &'static str,
    sha256: &'static str,
}

const CJK_SOURCES: &[CjkSource] = &[
    CjkSource {
        region: "JP",
        sha256: "dff723ba59d57d136764a04b9b2d03205544f7cd785a711442d6d2d085ac5073",
    },
    CjkSource {
        region: "KR",
        sha256: "69975a0ac8472717870aefeab0a4d52739308d90856b9955313b2ad5e0148d68",
    },
    CjkSource {
        region: "TC",
        sha256: "5bab0cb3c1cf89dde07c4a95a4054b195afbcfe784d69d75c340780712237537",
    },
    CjkSource {
        region: "SC",
        sha256: "faa6c9df652116dde789d351359f3d7e5d2285a2b2a1f04a2d7244df706d5ea9",
    },
];

const CJK_URL_PREFIX: &str =
    "https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/SubsetOTF/";

/// Indices into [`CJK_SOURCES`], by name.
const JP: usize = 0;
const KR: usize = 1;
const TC: usize = 2;
const SC: usize = 3;

/// The ideographic space, which is the separator between Japanese words.
const IDEOGRAPHIC_SPACE: char = '\u{3000}';

fn text_glyphs() -> Vec<char> {
    // U+00AD (soft hyphen) is invisible and the mono face has no glyph for it.
    let mut set: Vec<char> = (0x20u32..=0x7E)
        .chain(0xA0..=0xFF)
        .filter(|&c| c != 0xAD)
        .filter_map(char::from_u32)
        .collect();
    set.extend_from_slice(SYMBOLS);
    set.sort_unstable();
    set.dedup();
    set
}

fn icon_glyphs() -> Vec<char> {
    let mut set: Vec<char> = ICONS.iter().filter_map(|c| char::from_u32(*c)).collect();
    set.sort_unstable();
    set.dedup();
    assert_eq!(set.len(), ICONS.len(), "duplicate icon code point");
    set
}

/// Every code point the fallback face has to carry: every character of
/// every word of the ten lists, in both the published and the display
/// form, every keycap of the non-Latin keyboards, and the
/// ideographic space. Computed from the lists themselves, so it cannot
/// drift from them. What the text faces already carry is left out.
fn cjk_glyphs() -> Vec<char> {
    let mut set: BTreeSet<char> = BTreeSet::new();
    for lang in Language::ALL {
        for i in 0..2048u16 {
            set.extend(lang.word(i).chars());
            set.extend(lang.word_display(i).chars());
        }
    }
    // Every keycap comes from the keyboard itself, so a keycap can never
    // be drawn that was not baked. Pinyin types on Latin letters, which
    // the text faces already carry.
    set.extend(keyboard::keycaps(KeyboardKind::Kana));
    set.extend(keyboard::keycaps(KeyboardKind::Jamo));
    set.extend(keyboard::keycaps(KeyboardKind::Zhuyin));
    // The four card suits, which the Latin faces do not carry.
    set.extend(keyboard::keycaps(KeyboardKind::Cards));
    set.insert(IDEOGRAPHIC_SPACE);
    for c in text_glyphs() {
        set.remove(&c);
    }
    set.into_iter().collect()
}

/// Which region subset a code point is taken from, and in what order the
/// rest are tried. Each script comes from the region whose typography it
/// belongs to; a Han character comes from TC when the Traditional list
/// uses it and from SC otherwise, so that a word is drawn in the shapes
/// its own list is written in. The two Latin faces are tried first, for
/// the combining marks the accented lists decompose into, which no CJK
/// subset carries.
fn cjk_order(c: char, traditional: &BTreeSet<char>) -> [usize; 4] {
    let first = match c as u32 {
        0x3000..=0x303F | 0x3041..=0x30FF | 0x31F0..=0x31FF => JP,
        0x1100..=0x11FF | 0x3130..=0x318F | 0xA960..=0xA97F | 0xAC00..=0xD7FB => KR,
        0x02C7 | 0x02C9..=0x02CB | 0x02D9 | 0x3100..=0x312F => TC,
        _ if traditional.contains(&c) => TC,
        _ => SC,
    };
    // The preferred region, then the rest in a steady order, so that a
    // code point only one subset happens to carry is still found.
    let mut out = [first; 4];
    let mut n = 1;
    for region in [JP, KR, TC, SC] {
        if region != first {
            out[n] = region;
            n += 1;
        }
    }
    out
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is tools/fontbake; the root is two levels up.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

/// Outline command tags, one byte each, followed by the coordinates the
/// command takes as `i16` little-endian pairs.
const TAG_MOVE: u8 = b'M';
const TAG_LINE: u8 = b'L';
const TAG_QUAD: u8 = b'Q';
const TAG_CUBIC: u8 = b'C';
const TAG_CLOSE: u8 = b'Z';

/// One glyph ready to write: font units, y up, origin at the pen on the
/// baseline.
struct Outline {
    advance: u16,
    /// `x_min`, `y_min`, `x_max`, `y_max`.
    bbox: [i16; 4],
    commands: Vec<u8>,
}

/// Collects a `ttf-parser` outline into the command stream, rescaling
/// font units when a glyph comes from a face whose em is a different
/// number of units than the file being written.
struct Collector {
    out: Vec<u8>,
    scale: f64,
    open: bool,
}

impl Collector {
    fn new(scale: f64) -> Self {
        Collector {
            out: Vec::new(),
            scale,
            open: false,
        }
    }

    fn push(&mut self, tag: u8, coords: &[f32]) {
        self.out.push(tag);
        for &v in coords {
            let u = (f64::from(v) * self.scale).round();
            let u = i16::try_from(u as i64).expect("coordinate fits font units");
            self.out.extend_from_slice(&u.to_le_bytes());
        }
    }

    fn end_contour(&mut self) {
        if self.open {
            self.out.push(TAG_CLOSE);
            self.open = false;
        }
    }
}

impl OutlineBuilder for Collector {
    fn move_to(&mut self, x: f32, y: f32) {
        self.end_contour();
        self.push(TAG_MOVE, &[x, y]);
        self.open = true;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.push(TAG_LINE, &[x, y]);
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.push(TAG_QUAD, &[cx, cy, x, y]);
    }

    fn curve_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) {
        self.push(TAG_CUBIC, &[c1x, c1y, c2x, c2y, x, y]);
    }

    fn close(&mut self) {
        self.end_contour();
    }
}

fn rescale(v: f32, scale: f64) -> i16 {
    i16::try_from((f64::from(v) * scale).round() as i64).expect("coordinate fits font units")
}

/// The outline of `c` from `face`, in the units-per-em of the file being
/// written. `None` when the face has no glyph for the code point.
fn outline_from(face: &Face, c: char, upem: u16) -> Option<Outline> {
    let gid = face.glyph_index(c).filter(|g| *g != GlyphId(0))?;
    let scale = f64::from(upem) / f64::from(face.units_per_em());
    let advance = face.glyph_hor_advance(gid).unwrap_or(0);
    let advance = (f64::from(advance) * scale).round() as u16;
    let mut collector = Collector::new(scale);
    let rect = face.outline_glyph(gid, &mut collector);
    collector.end_contour();
    let bbox = match rect {
        Some(Rect {
            x_min,
            y_min,
            x_max,
            y_max,
        }) => [
            rescale(f32::from(x_min), scale),
            rescale(f32::from(y_min), scale),
            rescale(f32::from(x_max), scale),
            rescale(f32::from(y_max), scale),
        ],
        // A blank glyph (space): an advance and nothing to fill.
        None => [0; 4],
    };
    Some(Outline {
        advance,
        bbox,
        commands: collector.out,
    })
}

/// A thick line segment as a quadrilateral, in em units with y up.
fn segment(a: (f32, f32), b: (f32, f32), width: f32) -> Polygon {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = (dx * dx + dy * dy).sqrt();
    let (nx, ny) = (-dy / len * width / 2.0, dx / len * width / 2.0);
    // Extend each end by half the width so that joints have no notch.
    let (ex, ey) = (dx / len * width / 2.0, dy / len * width / 2.0);
    let a = (a.0 - ex, a.1 - ey);
    let b = (b.0 + ex, b.1 + ey);
    vec![
        (a.0 + nx, a.1 + ny),
        (b.0 + nx, b.1 + ny),
        (b.0 - nx, b.1 - ny),
        (a.0 - nx, a.1 - ny),
    ]
}

/// A closed polygon in em units, y up, origin at the pen on the baseline.
type Polygon = Vec<(f32, f32)>;

/// Polygons for the symbols no vendored face has. Returns
/// `(advance_em, polygons)`.
fn synthesized(c: char) -> Option<(f32, Vec<Polygon>)> {
    match c {
        '✓' => Some((
            0.8,
            vec![
                segment((0.13, 0.32), (0.32, 0.10), 0.11),
                segment((0.32, 0.10), (0.70, 0.60), 0.11),
            ],
        )),
        '✗' => Some((
            0.8,
            vec![
                segment((0.15, 0.08), (0.65, 0.58), 0.10),
                segment((0.15, 0.58), (0.65, 0.08), 0.10),
            ],
        )),
        _ => None,
    }
}

/// The synthesized polygons as line contours in font units.
fn outline_synthesized(advance_em: f32, polys: &[Polygon], upem: u16) -> Outline {
    let s = f64::from(upem);
    let mut collector = Collector::new(s);
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for poly in polys {
        for (i, &(x, y)) in poly.iter().enumerate() {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
            if i == 0 {
                collector.move_to(x, y);
            } else {
                collector.line_to(x, y);
            }
        }
        collector.close();
    }
    Outline {
        advance: (f64::from(advance_em) * s).round() as u16,
        bbox: [
            rescale(x0, s),
            rescale(y0, s),
            rescale(x1, s),
            rescale(y1, s),
        ],
        commands: collector.out,
    }
}

const HEADER_LEN: usize = 20;
const RECORD_LEN: usize = 20;

/// Writes one face file: its header, its glyph records and its outlines.
///
/// `glyph` supplies each code point's outline already in `upem` units;
/// the bake aborts if it cannot, so a face is never written with a hole
/// in the set it is supposed to cover.
fn bake(
    chars: &[char],
    upem: u16,
    (ascent, descent, line_gap): (i16, i16, i16),
    face_name: &str,
    glyph: impl Fn(char) -> Option<Outline>,
) -> Vec<u8> {
    let mut records: Vec<u8> = Vec::with_capacity(chars.len() * RECORD_LEN);
    let mut outlines: Vec<u8> = Vec::new();
    for &c in chars {
        let Some(o) = glyph(c) else {
            panic!("{face_name}: no glyph for U+{:04X} {c:?}", c as u32);
        };
        let offset = u32::try_from(outlines.len()).expect("outlines < 4 GiB");
        let len = u16::try_from(o.commands.len()).expect("one glyph < 64 KiB");
        outlines.extend_from_slice(&o.commands);
        records.extend_from_slice(&(c as u32).to_le_bytes());
        records.extend_from_slice(&o.advance.to_le_bytes());
        for v in o.bbox {
            records.extend_from_slice(&v.to_le_bytes());
        }
        records.extend_from_slice(&offset.to_le_bytes());
        records.extend_from_slice(&len.to_le_bytes());
    }

    let mut out = Vec::with_capacity(HEADER_LEN + records.len() + outlines.len());
    out.extend_from_slice(b"OSKO");
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // flags: none defined
    out.extend_from_slice(&upem.to_le_bytes());
    out.extend_from_slice(&ascent.to_le_bytes());
    out.extend_from_slice(&descent.to_le_bytes());
    out.extend_from_slice(&line_gap.to_le_bytes());
    out.extend_from_slice(&(u32::try_from(chars.len()).unwrap()).to_le_bytes());
    out.extend_from_slice(&records);
    out.extend_from_slice(&outlines);
    out
}

/// The bytes of one CJK region subset, downloaded into `tools/fonts/cjk/`
/// if the file is not there yet and checked against its pinned digest.
fn cjk_font(dir: &Path, src: &CjkSource) -> Vec<u8> {
    let name = format!("NotoSans{}-Regular.otf", src.region);
    let path = dir.join(&name);
    let url = format!("{CJK_URL_PREFIX}{}/{name}", src.region);
    let data = fetch(&path, &url);
    check_digest(&path, &data, src.sha256);
    data
}

/// The bytes at `path`, downloaded from `url` first if the file is not
/// there yet.
fn fetch(path: &Path, url: &str) -> Vec<u8> {
    if !path.exists() {
        println!("fetching {url}");
        let dir = path.parent().expect("a file has a directory");
        fs::create_dir_all(dir).unwrap_or_else(|e| panic!("create {}: {e}", dir.display()));
        let status = Command::new("curl")
            .args([
                "--fail",
                "--location",
                "--silent",
                "--show-error",
                "--output",
            ])
            .arg(path)
            .arg(url)
            .status()
            .expect("run curl");
        assert!(status.success(), "download {url}: {status}");
    }
    fs::read(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Aborts the bake unless `data` is the file that was pinned.
fn check_digest(path: &Path, data: &[u8], sha256_hex: &str) {
    let digest: String = sha256(data).iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        digest,
        sha256_hex,
        "{} is not the pinned file: delete it and let the baker fetch it again",
        path.display()
    );
}

/// Puts Iosevka's Regular TTF in `tools/fonts/iosevka/`, downloading the
/// release archive and checking its digest first if the TTF is not
/// already unpacked. The archive is the only form the release publishes,
/// so one download carries every cut and the baker keeps the one it uses.
fn ensure_iosevka(fonts_dir: &Path) {
    let ttf = fonts_dir.join(IOSEVKA_FILE);
    if ttf.exists() {
        return;
    }
    let dir = fonts_dir.join("iosevka");
    let archive = format!("PkgTTF-Iosevka-{IOSEVKA_VERSION}.zip");
    let path = dir.join(&archive);
    let url = format!(
        "https://github.com/be5invis/Iosevka/releases/download/v{IOSEVKA_VERSION}/{archive}"
    );
    let data = fetch(&path, &url);
    check_digest(&path, &data, IOSEVKA_SHA256);
    let name = ttf.file_name().expect("a file has a name");
    let status = Command::new("unzip")
        .arg("-o")
        .arg("-j")
        .arg(&path)
        .arg(name)
        .arg("-d")
        .arg(&dir)
        .status()
        .expect("run unzip");
    assert!(status.success(), "unpack {}: {status}", path.display());
}

fn main() {
    let root = repo_root();
    let fonts_dir = root.join("tools/fonts");
    let assets_dir = root.join("core/osk-ui/assets");
    let generated = root.join("core/osk-ui/src/fonts/generated.rs");
    fs::create_dir_all(&assets_dir).expect("create assets dir");
    // Stale face files from an earlier face list must not linger in the crate.
    for entry in fs::read_dir(&assets_dir)
        .expect("read assets dir")
        .flatten()
    {
        if entry.path().extension().is_some_and(|e| e == "outl") {
            fs::remove_file(entry.path()).expect("remove stale outline file");
        }
    }

    ensure_iosevka(&fonts_dir);
    let read =
        |file: &str| fs::read(fonts_dir.join(file)).unwrap_or_else(|e| panic!("read {file}: {e}"));
    fn parse(data: &[u8]) -> Face<'_> {
        Face::parse(data, 0).expect("parse font")
    }
    // Noto Sans lacks the arrows and triangles; Noto Sans Mono, which is
    // no longer a face of its own, still donates them to every face that
    // lacks them.
    let text_fallback_data = read("NotoSansMono-Regular.ttf");
    // The icon face carries U+E9xx; everything else comes from Font Awesome.
    let icon_fallback_data = read("Font_Awesome_6_Free-Solid-900.otf");
    let text_fallback = parse(&text_fallback_data);
    let icon_fallback = parse(&icon_fallback_data);
    let text_chars = text_glyphs();
    let icon_chars = icon_glyphs();
    let mut table = String::new();
    let mut total = 0usize;
    for spec in FACES {
        let data = read(spec.file);
        let face = parse(&data);
        let (chars, fallback) = match spec.kind {
            Kind::Text => (&text_chars, &text_fallback),
            Kind::Icon => (&icon_chars, &icon_fallback),
        };
        let upem = face.units_per_em();
        // Icons are drawn centred in a square box (`Widget::Icon`), so the
        // face's own baseline is not used: a full em above the baseline and
        // nothing below makes the line box the square the screen asked for.
        let metrics = if spec.kind == Kind::Icon {
            (i16::try_from(upem).expect("em fits i16"), 0i16, 0i16)
        } else {
            (face.ascender(), face.descender(), face.line_gap())
        };
        let bytes = bake(chars, upem, metrics, spec.name, |c| {
            outline_from(&face, c, upem)
                .or_else(|| outline_from(fallback, c, upem))
                .or_else(|| {
                    synthesized(c)
                        .map(|(advance, polys)| outline_synthesized(advance, &polys, upem))
                })
        });
        let file = format!("{}.outl", spec.name);
        fs::write(assets_dir.join(&file), &bytes).expect("write outline file");
        total += bytes.len();
        table.push_str(&entry(spec.variant, &file));
        println!("{file}: {} bytes, {upem} em units", bytes.len());
    }

    // The fifth face: every code point the wordlists and the three
    // non-Latin keyboards need and the text faces lack, taken from the
    // four Noto Sans CJK region subsets and rescaled to one em so the
    // file is one coordinate system.
    let cjk_dir = fonts_dir.join("cjk");
    let cjk_data: Vec<Vec<u8>> = CJK_SOURCES.iter().map(|s| cjk_font(&cjk_dir, s)).collect();
    let cjk_faces: Vec<Face<'_>> = cjk_data.iter().map(|d| parse(d)).collect();
    let traditional: BTreeSet<char> = Language::ChineseTraditional
        .words()
        .iter()
        .flat_map(|w| w.chars())
        .collect();
    let cjk_chars = cjk_glyphs();
    let upem = cjk_faces[JP].units_per_em();
    let metrics = (
        cjk_faces[JP].ascender(),
        cjk_faces[JP].descender(),
        cjk_faces[JP].line_gap(),
    );
    let latin_data = read(FACES[0].file);
    let latin = parse(&latin_data);
    let sources = |c: char| {
        outline_from(&latin, c, upem)
            .or_else(|| outline_from(&text_fallback, c, upem))
            .or_else(|| {
                cjk_order(c, &traditional)
                    .into_iter()
                    .find_map(|r| outline_from(&cjk_faces[r], c, upem))
            })
    };
    let bytes = bake(&cjk_chars, upem, metrics, "cjk", sources);
    fs::write(assets_dir.join("cjk.outl"), &bytes).expect("write outline file");
    total += bytes.len();
    table.push_str(&entry("Cjk", "cjk.outl"));
    println!("cjk.outl: {} bytes, {upem} em units", bytes.len());

    let source = format!(
        "//! Generated by `tools/fontbake` (`just fonts`). Do not edit.\n\
         //!\n\
         //! One entry per face; see `tools/fontbake/README.md`.\n\
         \n\
         use super::{{Face, Family}};\n\
         \n\
         /// Every face, one outline file each.\n\
         pub static FACES: &[Face] = &[\n{table}];\n"
    );
    fs::write(&generated, source).expect("write generated.rs");
    println!(
        "{} text glyphs, {} icon glyphs, {} CJK glyphs, {total} bytes total",
        text_chars.len(),
        icon_chars.len(),
        cjk_chars.len(),
    );
}

/// One `FACES` entry in `generated.rs`.
fn entry(variant: &str, file: &str) -> String {
    format!(
        "    Face {{\n        family: Family::{variant},\n        data: include_bytes!(\"../../assets/{file}\"),\n    }},\n"
    )
}
