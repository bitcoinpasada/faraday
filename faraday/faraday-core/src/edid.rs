//! What a screen's EDID says it is (`docs/STARTER.md` §3.1a): a TV, or
//! the capture side of a KVM. One set of rules, with no I/O, for every
//! place an EDID is read: the Starter reads each screen's whole EDID
//! from its system; the stick reads the base block the EFI stub puts in
//! the kernel's boot parameters ([`from_boot_params`],
//! `docs/DECISIONS.md` F23), which holds a name and a maker but none of
//! the CEA-861 extension.

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
