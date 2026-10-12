//! What the elevated writer tells the hub app over its pipe
//! (`docs/HUB.md` §2): one line of UTF-8 per message, fields separated
//! by a tab, text fields with `\`, tab and newline escaped. Plain text,
//! so the tests check every message's round trip on any system.
//!
//! ```text
//! progress writing 1048576 84955136
//! progress reading-back 1048576 84955136
//! ok
//! failed TooSmall 15931539456 84955136
//! ```

use std::io::BufRead;

use faraday_hub::api::{InUse, Reason, StickFault, WriteStage};

/// The longest line read; a longer one ends the conversation.
pub const MAX_LINE: usize = 8192;

/// One message from the elevated writer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// How far the write is.
    Progress {
        /// Writing or reading back.
        stage: WriteStage,
        /// Bytes done.
        done: u64,
        /// Bytes in all.
        total: u64,
    },
    /// The write ended: read back and matched, or why not.
    Done(Result<(), Reason>),
}

pub(crate) fn escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

pub(crate) fn unescape(s: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('\\') => out.push('\\'),
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            _ => return Err(String::from("a bad escape")),
        }
    }
    Ok(out)
}

/// A reason as a name and its fields.
pub(crate) fn reason_fields(r: &Reason) -> (&'static str, Vec<String>) {
    let s = |v: &str| v.to_string();
    let n = |v: u64| v.to_string();
    match r {
        Reason::NoWriter => ("NoWriter", vec![]),
        Reason::NotCarried => ("NotCarried", vec![]),
        Reason::Changed {
            device,
            serial,
            size,
        } => ("Changed", vec![s(device), s(serial), n(*size)]),
        Reason::InUse(InUse::Mounted(at)) => ("InUseMounted", vec![s(at)]),
        Reason::InUse(InUse::Swap) => ("InUseSwap", vec![]),
        Reason::InUse(InUse::System) => ("InUseSystem", vec![]),
        Reason::BootOnly => ("BootOnly", vec![]),
        Reason::TooSmall { disk, image } => ("TooSmall", vec![n(*disk), n(*image)]),
        Reason::PayloadSum => ("PayloadSum", vec![]),
        Reason::ImageChanged { path, sha256 } => ("ImageChanged", vec![s(path), s(sha256)]),
        Reason::Gone => ("Gone", vec![]),
        Reason::Bus(d) => ("Bus", vec![s(d)]),
        Reason::Unmounting(d) => ("Unmounting", vec![s(d)]),
        Reason::OpenWrite(d) => ("OpenWrite", vec![s(d)]),
        Reason::OpenRead(d) => ("OpenRead", vec![s(d)]),
        Reason::Device { device, detail } => ("Device", vec![s(device), s(detail)]),
        Reason::Flush { device, detail } => ("Flush", vec![s(device), s(detail)]),
        Reason::NotStick { device, fault } => {
            let mut f = vec![s(device)];
            f.extend(fault_fields(fault));
            ("NotStick", f)
        }
        Reason::NotStickImage(fault) => ("NotStickImage", fault_fields(fault)),
        Reason::BootLarger { image, stick } => ("BootLarger", vec![n(*image), n(*stick)]),
        Reason::BootLabel => ("BootLabel", vec![]),
        Reason::BootShort => ("BootShort", vec![]),
        Reason::TableChanged(d) => ("TableChanged", vec![s(d)]),
        Reason::ReadingTable(d) => ("ReadingTable", vec![s(d)]),
        Reason::BackEnds { device, at } => ("BackEnds", vec![s(device), n(*at)]),
        Reason::BackDiffers { device, at } => ("BackDiffers", vec![s(device), n(*at)]),
        Reason::BackFailed { device, detail } => ("BackFailed", vec![s(device), s(detail)]),
        Reason::SourceRead(d) => ("SourceRead", vec![s(d)]),
        Reason::WriteFailed(d) => ("WriteFailed", vec![s(d)]),
        Reason::PayloadChanged => ("PayloadChanged", vec![]),
        Reason::Damaged(d) => ("Damaged", vec![s(d)]),
        Reason::Longer(v) => ("Longer", vec![n(*v)]),
        Reason::Shorter { expected, actual } => ("Shorter", vec![n(*expected), n(*actual)]),
        Reason::WrongSum(d) => ("WrongSum", vec![s(d)]),
        Reason::NoHome => ("NoHome", vec![]),
        Reason::NoDesktopApp => ("NoDesktopApp", vec![]),
        Reason::CacheSum(d) => ("CacheSum", vec![s(d)]),
        Reason::File { path, detail } => ("File", vec![s(path), s(detail)]),
        Reason::InTheWay { path } => ("InTheWay", vec![s(path)]),
        Reason::Start { path, detail } => ("Start", vec![s(path), s(detail)]),
        Reason::Restart(d) => ("Restart", vec![s(d)]),
        Reason::Missing(d) => ("Missing", vec![s(d)]),
        Reason::BootEntry(d) => ("BootEntry", vec![s(d)]),
        Reason::NoStick => ("NoStick", vec![]),
        Reason::Declined => ("Declined", vec![]),
        Reason::Unsupported => ("Unsupported", vec![]),
        Reason::NoAdmin => ("NoAdmin", vec![]),
        Reason::Managed(c) => ("Managed", vec![n(u64::from(*c))]),
        Reason::BitLocker { call, code } => ("BitLocker", vec![s(call), n(u64::from(*code))]),
        Reason::NotEncrypted => ("NotEncrypted", vec![]),
        Reason::NotRead => ("NotRead", vec![]),
    }
}

fn fault_fields(f: &StickFault) -> Vec<String> {
    let name = |v: &str| vec![v.to_string()];
    match f {
        StickFault::Reading(d) => vec!["Reading".into(), d.clone()],
        StickFault::NoHeader => name("NoHeader"),
        StickFault::NotGpt => name("NotGpt"),
        StickFault::HeaderDamaged => name("HeaderDamaged"),
        StickFault::HeaderChecksum => name("HeaderChecksum"),
        StickFault::EntriesDamaged => name("EntriesDamaged"),
        StickFault::EntriesPastEnd => name("EntriesPastEnd"),
        StickFault::EntriesChecksum => name("EntriesChecksum"),
        StickFault::DiskUuid => name("DiskUuid"),
        StickFault::Partitions(n) => vec!["Partitions".into(), n.to_string()],
        StickFault::BootName => name("BootName"),
        StickFault::BootType => name("BootType"),
        StickFault::DataType => name("DataType"),
        StickFault::BootDamaged => name("BootDamaged"),
        StickFault::BootSmall { boot, image } => {
            vec!["BootSmall".into(), boot.to_string(), image.to_string()]
        }
        StickFault::Overlap => name("Overlap"),
        StickFault::BootPastEnd => name("BootPastEnd"),
    }
}

/// Fields read in order.
pub(crate) struct Fields<'a>(pub(crate) std::slice::Iter<'a, String>);

impl Fields<'_> {
    fn s(&mut self) -> Result<String, String> {
        self.0
            .next()
            .cloned()
            .ok_or_else(|| String::from("a field missing"))
    }

    fn n(&mut self) -> Result<u64, String> {
        self.s()?.parse().map_err(|_| String::from("not a number"))
    }

    pub(crate) fn end(&mut self) -> Result<(), String> {
        match self.0.next() {
            None => Ok(()),
            Some(_) => Err(String::from("a field too many")),
        }
    }
}

fn fault(f: &mut Fields<'_>) -> Result<StickFault, String> {
    Ok(match f.s()?.as_str() {
        "Reading" => StickFault::Reading(f.s()?),
        "NoHeader" => StickFault::NoHeader,
        "NotGpt" => StickFault::NotGpt,
        "HeaderDamaged" => StickFault::HeaderDamaged,
        "HeaderChecksum" => StickFault::HeaderChecksum,
        "EntriesDamaged" => StickFault::EntriesDamaged,
        "EntriesPastEnd" => StickFault::EntriesPastEnd,
        "EntriesChecksum" => StickFault::EntriesChecksum,
        "DiskUuid" => StickFault::DiskUuid,
        "Partitions" => StickFault::Partitions(usize::try_from(f.n()?).map_err(|e| e.to_string())?),
        "BootName" => StickFault::BootName,
        "BootType" => StickFault::BootType,
        "DataType" => StickFault::DataType,
        "BootDamaged" => StickFault::BootDamaged,
        "BootSmall" => StickFault::BootSmall {
            boot: f.n()?,
            image: f.n()?,
        },
        "Overlap" => StickFault::Overlap,
        "BootPastEnd" => StickFault::BootPastEnd,
        other => return Err(format!("{other}: not a fault")),
    })
}

fn code(n: u64) -> Result<u32, String> {
    u32::try_from(n).map_err(|_| String::from("not a code"))
}

pub(crate) fn reason(name: &str, f: &mut Fields<'_>) -> Result<Reason, String> {
    Ok(match name {
        "NoWriter" => Reason::NoWriter,
        "NotCarried" => Reason::NotCarried,
        "Changed" => Reason::Changed {
            device: f.s()?,
            serial: f.s()?,
            size: f.n()?,
        },
        "InUseMounted" => Reason::InUse(InUse::Mounted(f.s()?)),
        "InUseSwap" => Reason::InUse(InUse::Swap),
        "InUseSystem" => Reason::InUse(InUse::System),
        "BootOnly" => Reason::BootOnly,
        "TooSmall" => Reason::TooSmall {
            disk: f.n()?,
            image: f.n()?,
        },
        "PayloadSum" => Reason::PayloadSum,
        "ImageChanged" => Reason::ImageChanged {
            path: f.s()?,
            sha256: f.s()?,
        },
        "Gone" => Reason::Gone,
        "Bus" => Reason::Bus(f.s()?),
        "Unmounting" => Reason::Unmounting(f.s()?),
        "OpenWrite" => Reason::OpenWrite(f.s()?),
        "OpenRead" => Reason::OpenRead(f.s()?),
        "Device" => Reason::Device {
            device: f.s()?,
            detail: f.s()?,
        },
        "Flush" => Reason::Flush {
            device: f.s()?,
            detail: f.s()?,
        },
        "NotStick" => Reason::NotStick {
            device: f.s()?,
            fault: fault(f)?,
        },
        "NotStickImage" => Reason::NotStickImage(fault(f)?),
        "BootLarger" => Reason::BootLarger {
            image: f.n()?,
            stick: f.n()?,
        },
        "BootLabel" => Reason::BootLabel,
        "BootShort" => Reason::BootShort,
        "TableChanged" => Reason::TableChanged(f.s()?),
        "ReadingTable" => Reason::ReadingTable(f.s()?),
        "BackEnds" => Reason::BackEnds {
            device: f.s()?,
            at: f.n()?,
        },
        "BackDiffers" => Reason::BackDiffers {
            device: f.s()?,
            at: f.n()?,
        },
        "BackFailed" => Reason::BackFailed {
            device: f.s()?,
            detail: f.s()?,
        },
        "SourceRead" => Reason::SourceRead(f.s()?),
        "WriteFailed" => Reason::WriteFailed(f.s()?),
        "PayloadChanged" => Reason::PayloadChanged,
        "Damaged" => Reason::Damaged(f.s()?),
        "Longer" => Reason::Longer(f.n()?),
        "Shorter" => Reason::Shorter {
            expected: f.n()?,
            actual: f.n()?,
        },
        "WrongSum" => Reason::WrongSum(f.s()?),
        "NoHome" => Reason::NoHome,
        "NoDesktopApp" => Reason::NoDesktopApp,
        "CacheSum" => Reason::CacheSum(f.s()?),
        "File" => Reason::File {
            path: f.s()?,
            detail: f.s()?,
        },
        "InTheWay" => Reason::InTheWay { path: f.s()? },
        "Start" => Reason::Start {
            path: f.s()?,
            detail: f.s()?,
        },
        "Restart" => Reason::Restart(f.s()?),
        "Missing" => Reason::Missing(f.s()?),
        "BootEntry" => Reason::BootEntry(f.s()?),
        "NoStick" => Reason::NoStick,
        "Declined" => Reason::Declined,
        "Unsupported" => Reason::Unsupported,
        "NoAdmin" => Reason::NoAdmin,
        "Managed" => Reason::Managed(code(f.n()?)?),
        "BitLocker" => Reason::BitLocker {
            call: f.s()?,
            code: code(f.n()?)?,
        },
        "NotEncrypted" => Reason::NotEncrypted,
        "NotRead" => Reason::NotRead,
        other => return Err(format!("{other}: not a reason")),
    })
}

impl Message {
    /// The message as one line, with its newline.
    pub fn encode(&self) -> String {
        let fields: Vec<String> = match self {
            Message::Progress { stage, done, total } => vec![
                "progress".into(),
                match stage {
                    WriteStage::Writing => "writing",
                    WriteStage::ReadingBack => "reading-back",
                }
                .into(),
                done.to_string(),
                total.to_string(),
            ],
            Message::Done(Ok(())) => vec!["ok".into()],
            Message::Done(Err(r)) => {
                let (name, rest) = reason_fields(r);
                let mut f = vec!["failed".to_string(), name.to_string()];
                f.extend(rest);
                f
            }
        };
        let mut line = fields
            .iter()
            .map(|f| escape(f))
            .collect::<Vec<_>>()
            .join("\t");
        line.push('\n');
        line
    }

    /// Reads one line (without its newline), or says why it is not a
    /// message.
    pub fn decode(line: &str) -> Result<Message, String> {
        let fields = line
            .split('\t')
            .map(unescape)
            .collect::<Result<Vec<_>, _>>()?;
        let mut f = Fields(fields.iter());
        let m = match f.s()?.as_str() {
            "progress" => {
                let stage = match f.s()?.as_str() {
                    "writing" => WriteStage::Writing,
                    "reading-back" => WriteStage::ReadingBack,
                    other => return Err(format!("{other}: not a stage")),
                };
                Message::Progress {
                    stage,
                    done: f.n()?,
                    total: f.n()?,
                }
            }
            "ok" => Message::Done(Ok(())),
            "failed" => {
                let name = f.s()?;
                Message::Done(Err(reason(&name, &mut f)?))
            }
            other => return Err(format!("{other}: not a message")),
        };
        f.end()?;
        Ok(m)
    }
}

/// Reads the next message from `r`: `None` at the end, an error for a
/// line that is too long, not UTF-8 or not a message.
pub fn next<R: BufRead>(r: &mut R) -> Option<Result<Message, String>> {
    let mut line = Vec::new();
    loop {
        let buf = match r.fill_buf() {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Some(Err(e.to_string())),
        };
        if buf.is_empty() {
            return (!line.is_empty()).then(|| Err(String::from("a line without its end")));
        }
        let (take, end) = match buf.iter().position(|&b| b == b'\n') {
            Some(i) => (i + 1, true),
            None => (buf.len(), false),
        };
        line.extend_from_slice(&buf[..take]);
        r.consume(take);
        if line.len() > MAX_LINE {
            return Some(Err(String::from("a line too long")));
        }
        if end {
            line.pop();
            return Some(
                String::from_utf8(line)
                    .map_err(|_| String::from("not UTF-8"))
                    .and_then(|l| Message::decode(&l)),
            );
        }
    }
}
