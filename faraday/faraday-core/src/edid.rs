//! What a screen's EDID says it is (`docs/STARTER.md` §3.1a): a TV, or
//! the capture side of a KVM. One set of rules, with no I/O, for every
//! place an EDID is read: the Starter reads each screen's whole EDID
//! from its system; the stick reads the base block the EFI stub puts in
//! the kernel's boot parameters ([`from_boot_params`],
//! `docs/DECISIONS.md` F23), which holds a name and a maker but none of
//! the CEA-861 extension. The stick also reads the base block's size
//! ([`size`]): how many pixels the panel has and how large it is, which
//! say how dense it is and whether it is a small panel
//! (`docs/ORIENTATION.md` §2).

/// What a screen is, when its EDID says it is of concern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    /// A TV: it may record what it shows.
    Tv,
    /// The capture side of a KVM switch or an IP-KVM.
    Kvm,
}

/// Where the EFI stub puts the base block of the screen's EDID in the
/// boot parameters (`edid_info` in `struct boot_params`, the x86 boot
/// protocol), and its length.
const BOOT_PARAMS_EDID: usize = 0x140;
const BASE_BLOCK: usize = 128;

/// The screen's EDID base block from the contents of
/// `/sys/kernel/boot_params/data`: what the firmware's graphics output
/// reported for the screen it draws on. `None` when the firmware passed
/// none (the bytes are zero), the kernel's stub does not read one, or
/// the file is too short.
pub fn from_boot_params(data: &[u8]) -> Option<&[u8]> {
    let block = data.get(BOOT_PARAMS_EDID..BOOT_PARAMS_EDID + BASE_BLOCK)?;
    (block[..8] == HEADER).then_some(block)
}

/// The EDID's fixed header.
const HEADER: [u8; 8] = [0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00];

/// PNP IDs of makers whose screens include TVs, as the UEFI Forum's PNP
/// ID registry lists them: Samsung (`SAM`), LG (`GSM`, Goldstar), Sony
/// (`SNY`), Sharp (`SHP`), Panasonic (`MEI`, Matsushita), Toshiba
/// (`TOS`), Vizio (`VIZ`), Philips (`PHL`), Hisense (`HEC`), TCL
/// (`TCL`). Most of them make monitors too, so a maker alone is not a
/// TV: [`display`] also asks for the HDMI block and a broadcast timing.
const TV_MAKERS: [&str; 10] = [
    "SAM", "GSM", "SNY", "SHP", "MEI", "TOS", "VIZ", "PHL", "HEC", "TCL",
];

/// CEA-861 video identification codes of interlaced broadcast formats:
/// 1080i at 60 and 50 Hz (5, 20), 480i (6, 7) and 576i (21, 22). A
/// computer monitor has no use for them; a TV lists them for broadcast.
const BROADCAST_VICS: [u8; 6] = [5, 6, 7, 20, 21, 22];

/// The HDMI Licensing IEEE OUI, `00-0C-03`, as an HDMI vendor-specific
/// data block carries it (least significant byte first).
const HDMI_OUI: [u8; 3] = [0x03, 0x0c, 0x00];

/// The monitor name an EDID carries (its `0xFC` descriptor), trimmed;
/// `None` when it has none or the bytes are not an EDID.
pub fn edid_name(edid: &[u8]) -> Option<String> {
    if edid.get(..8)? != HEADER {
        return None;
    }
    (0..4).find_map(|i| {
        let d = edid.get(54 + 18 * i..72 + 18 * i)?;
        if d[..3] != [0, 0, 0] || d[3] != 0xfc {
            return None;
        }
        let text: String = d[5..]
            .iter()
            .take_while(|&&b| b != 0x0a && b != 0)
            .map(|&b| if b.is_ascii_graphic() { b as char } else { ' ' })
            .collect();
        let text = text.trim().to_string();
        (!text.is_empty()).then_some(text)
    })
}

/// The maker's three-letter PNP ID an EDID carries (bytes 8 and 9, five
/// bits a letter).
pub fn edid_maker(edid: &[u8]) -> Option<String> {
    if edid.get(..8)? != HEADER {
        return None;
    }
    let v = u16::from_be_bytes([*edid.get(8)?, *edid.get(9)?]);
    let letter = |n: u16| {
        let c = (n & 0x1f) as u8;
        (1..=26).contains(&c).then(|| (b'A' + c - 1) as char)
    };
    Some(
        [letter(v >> 10)?, letter(v >> 5)?, letter(v)?]
            .iter()
            .collect(),
    )
}

/// What the CEA-861 extensions of an EDID hold: an HDMI vendor-specific
/// data block, and the video codes listed.
fn cea(edid: &[u8]) -> (bool, Vec<u8>) {
    let mut hdmi = false;
    let mut vics = Vec::new();
    let count = edid.get(126).copied().unwrap_or(0) as usize;
    for n in 1..=count {
        let Some(block) = edid.get(128 * n..128 * (n + 1)) else {
            break;
        };
        if block[0] != 0x02 {
            continue;
        }
        let end = (block[2] as usize).clamp(4, 127);
        let mut at = 4;
        while at < end {
            let tag = block[at] >> 5;
            let len = (block[at] & 0x1f) as usize;
            let Some(body) = block.get(at + 1..(at + 1 + len).min(end)) else {
                break;
            };
            match tag {
                // Video data block: one code a byte; codes 1 to 64 may
                // carry the native flag in their top bit.
                2 => vics.extend(body.iter().map(|&b| {
                    if (129..=192).contains(&b) {
                        b & 0x7f
                    } else {
                        b
                    }
                })),
                3 if body.get(..3) == Some(&HDMI_OUI[..]) => hdmi = true,
                _ => {}
            }
            at += 1 + len;
        }
    }
    (hdmi, vics)
}

/// The words of a name, upper-cased, split at anything not a letter or
/// a digit. The Starter's USB rules read names the same way.
pub fn words(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_uppercase)
        .collect()
}

/// A name that names a TV: the word `TV`, or a short word ending in it
/// (`LGTV`, `SMARTTV`, `UHDTV`).
fn names_tv(name: &str) -> bool {
    words(name)
        .iter()
        .any(|w| w == "TV" || (w.ends_with("TV") && w.len() <= 7))
}

/// A name that names a KVM switch or an IP-KVM: `KVM` in it (PiKVM,
/// JetKVM, BliKVM), or TinyPilot. The Starter's USB rules use it too.
pub fn names_kvm(name: &str) -> bool {
    let up = name.to_ascii_uppercase();
    up.contains("KVM") || up.contains("TINYPILOT")
}

/// What a screen is, from its EDID: a TV when its monitor name names a
/// TV, or when a TV maker's PNP ID comes with the HDMI vendor-specific
/// block and an interlaced broadcast timing; a KVM when its name names
/// one (the EDID a KVM's capture side reports). `None` for anything
/// else, an internal panel or an ordinary monitor, and for bytes that
/// are not an EDID.
pub fn display(edid: &[u8]) -> Option<Display> {
    if edid.get(..8)? != HEADER {
        return None;
    }
    let name = edid_name(edid).unwrap_or_default();
    if names_kvm(&name) {
        return Some(Display::Kvm);
    }
    if names_tv(&name) {
        return Some(Display::Tv);
    }
    let maker = edid_maker(edid).unwrap_or_default();
    let (hdmi, vics) = cea(edid);
    let broadcast = vics.iter().any(|v| BROADCAST_VICS.contains(v));
    (TV_MAKERS.contains(&maker.as_str()) && hdmi && broadcast).then_some(Display::Tv)
}

/// Whether the maker an EDID names makes TVs as well as monitors: a
/// screen whose base block alone names no TV may still be one.
pub fn tv_maker(edid: &[u8]) -> bool {
    edid_maker(edid).is_some_and(|m| TV_MAKERS.contains(&m.as_str()))
}

/// What an EDID base block says of a screen's size: the first detailed
/// timing's active pixels, and how large the picture is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    /// The first detailed timing's active pixels, across and down: the
    /// native mode. `None` when the block has no detailed timing first.
    pub pixels: Option<(u16, u16)>,
    /// The picture's size in millimetres, across and down: the detailed
    /// timing's image size where it is the shape of its pixels, otherwise
    /// the screen's size in centimetres where that is. `None` when
    /// neither says.
    pub mm: Option<(u16, u16)>,
}

impl Size {
    /// The diagonal in tenths of a millimetre, from [`Size::mm`].
    pub fn diagonal_tenth_mm(&self) -> Option<u32> {
        let (w, h) = self.mm?;
        let (w, h) = (u64::from(w) * 10, u64::from(h) * 10);
        Some((w * w + h * h).isqrt() as u32)
    }

    /// Pixels per inch along the diagonal, rounded: the pixels over the
    /// inches. `None` without both a size and pixels.
    pub fn dpi(&self) -> Option<u16> {
        let (pw, ph) = self.pixels?;
        let diag_px = (u64::from(pw).pow(2) + u64::from(ph).pow(2)).isqrt();
        let tenth_mm = u64::from(self.diagonal_tenth_mm()?).max(1);
        // 254 tenths of a millimetre to the inch.
        u16::try_from((diag_px * 254 + tenth_mm / 2) / tenth_mm).ok()
    }
}

/// Whether a picture `mm` across and down is the shape of `pixels`
/// (square pixels, within a tenth): a size that is not is a placeholder
/// some panels report (60 × 60 mm for an 800 × 1280 panel) and is not
/// read.
fn same_shape(mm: (u16, u16), pixels: (u16, u16)) -> bool {
    let (mw, mh) = (u64::from(mm.0), u64::from(mm.1));
    let (pw, ph) = (u64::from(pixels.0), u64::from(pixels.1));
    if mw == 0 || mh == 0 || pw == 0 || ph == 0 {
        return false;
    }
    // mw / mh against pw / ph, cross-multiplied: within 10 %.
    let (a, b) = (mw * ph, mh * pw);
    a * 10 <= b * 11 && b * 10 <= a * 11
}

/// The screen's size from its EDID base block (`docs/ORIENTATION.md`
/// §2): the first detailed timing descriptor's active pixels (bytes 56
/// to 61) and image size in mm (bytes 66 to 68), or the screen's size in
/// cm (bytes 21 and 22) where the descriptor's is missing or not the
/// shape of its pixels. `None` for bytes that are not an EDID.
pub fn size(edid: &[u8]) -> Option<Size> {
    if edid.get(..8)? != HEADER || edid.len() < BASE_BLOCK {
        return None;
    }
    let d = &edid[54..72];
    // A pixel clock of zero is a display descriptor, not a timing.
    let timing = d[0] != 0 || d[1] != 0;
    let pixels = timing
        .then(|| {
            let w = u16::from(d[2]) | (u16::from(d[4] >> 4) << 8);
            let h = u16::from(d[5]) | (u16::from(d[7] >> 4) << 8);
            (w, h)
        })
        .filter(|(w, h)| *w > 0 && *h > 0);
    let image = timing.then(|| {
        let w = u16::from(d[12]) | (u16::from(d[14] >> 4) << 8);
        let h = u16::from(d[13]) | (u16::from(d[14] & 0x0f) << 8);
        (w, h)
    });
    let cm = (edid[21] > 0 && edid[22] > 0)
        .then(|| (u16::from(edid[21]) * 10, u16::from(edid[22]) * 10));
    let fits = |mm: (u16, u16)| pixels.is_none_or(|p| same_shape(mm, p));
    let mm = image.filter(|&m| fits(m)).or(cm.filter(|&m| fits(m)));
    Some(Size { pixels, mm })
}
