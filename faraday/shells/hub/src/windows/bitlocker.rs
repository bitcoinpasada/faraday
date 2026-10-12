//! The BitLocker guard's work on Windows (`docs/HUB.md` §6), with no
//! Win32 call in it, so it builds and is tested on every system:
//!
//! - [`Order`]: the elevated copy's `--bitlocker <verb>` arguments (§2).
//! - [`Wmi`]: what the verbs ask of WMI's `Win32_EncryptableVolume`
//!   (`root\CIMV2\Security\MicrosoftVolumeEncryption`), which answers
//!   in numbers in every Windows language. `sys` answers it on Windows;
//!   [`Recorded`] answers it from a recording (`tests/windows/*.wmi`),
//!   and [`record`] writes one, without any recovery password in it.
//! - [`show_key`], [`suspend`], [`resume`] and [`state`]: the verbs. Every
//!   method's `ReturnValue` is read; a non-zero one is a refusal.
//! - [`Answer`]: what the elevated copy tells the window over its pipe,
//!   the recovery key among it, read and written through buffers that
//!   are wiped once the line is through.
//! - [`MarkerFile`]: the marker (§6.3), and what the window does after a
//!   suspension and a resume: the marker and the logon reminder.
//! - [`save_key_to`]: Save (§6.2), the one place the key goes to a file,
//!   and only when the person chooses it.

use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use faraday_hub::api::{KeyShown, Marker, Protection, Reason};
use faraday_qr::bitlocker::RecoveryKey;
use zeroize::{Zeroize, Zeroizing};

use super::elevated::PIPE_PREFIX;
use super::install::{self, Ask, Dirs, Registry};
use super::pipe::{self, Fields};

/// The first argument of the elevated copy for a BitLocker step.
pub const BITLOCKER: &str = "--bitlocker";
/// WMI's BitLocker namespace.
pub const NAMESPACE: &str = r"root\CIMV2\Security\MicrosoftVolumeEncryption";
/// Its class.
pub const CLASS: &str = "Win32_EncryptableVolume";
/// `KeyProtectorType` 3: a numerical password, the recovery key.
pub const NUMERICAL_PASSWORD: u32 = 3;
/// `VolumeType` 0: an operating-system volume.
pub const OS_VOLUME: u32 = 0;
/// The marker's name, in `%LOCALAPPDATA%\faraday`.
pub const MARKER: &str = "bitlocker-suspended";
/// The marker's first line.
const MARKER_HEAD: &str = "faraday bitlocker-suspended 1";
/// The most volumes an order or a marker names.
const MAX_VOLUMES: usize = 26;
/// The longest line read over the pipe.
pub const MAX_LINE: usize = 4096;

// ---------------------------------------------------------------------
// The elevated copy's arguments
// ---------------------------------------------------------------------

/// What the elevated copy is asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verb {
    /// Read the system drive's recovery key, adding one when it has none.
    ShowKey,
    /// Suspend every protected operating-system volume until resumed.
    Suspend,
    /// Resume these volumes (`\\?\Volume{…}\`), the marker's.
    Resume(Vec<String>),
}

/// A BitLocker step, as the elevated copy is given it.
///
/// ```text
/// --bitlocker show-key|suspend --pipe \\.\pipe\faraday-hub-…
/// --bitlocker resume --pipe \\.\pipe\faraday-hub-… --volume \\?\Volume{…}\ …
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    /// The step.
    pub verb: Verb,
    /// The pipe the answers go back over.
    pub pipe: String,
}

/// Whether `p` is a pipe the hub app makes.
fn pipe_ok(p: &str) -> bool {
    p.strip_prefix(PIPE_PREFIX).is_some_and(|id| {
        !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
    })
}

/// Whether `s` is a GUID in braces, `{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}`.
pub fn guid_ok(s: &str) -> bool {
    let Some(inner) = s.strip_prefix('{').and_then(|s| s.strip_suffix('}')) else {
        return false;
    };
    let parts: Vec<&str> = inner.split('-').collect();
    parts.len() == 5
        && parts
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(p, n)| p.len() == n && p.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// Whether `s` is a volume's name, `\\?\Volume{…}\`.
pub fn volume_ok(s: &str) -> bool {
    s.strip_prefix(r"\\?\Volume")
        .and_then(|s| s.strip_suffix('\\'))
        .is_some_and(guid_ok)
}

/// Whether `s` is a drive, `C:`.
fn drive_ok(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 2 && b[0].is_ascii_alphabetic() && b[1] == b':'
}

impl Order {
    /// The arguments, `--bitlocker` first.
    pub fn args(&self) -> Vec<String> {
        let verb = match &self.verb {
            Verb::ShowKey => "show-key",
            Verb::Suspend => "suspend",
            Verb::Resume(_) => "resume",
        };
        let mut a: Vec<String> = vec![
            BITLOCKER.into(),
            verb.into(),
            "--pipe".into(),
            self.pipe.clone(),
        ];
        if let Verb::Resume(volumes) = &self.verb {
            for v in volumes {
                a.extend(["--volume".into(), v.clone()]);
            }
        }
        a
    }

    /// Reads the arguments after the program's name, `--bitlocker` first,
    /// or says what is wrong with them.
    pub fn parse(args: &[String]) -> Result<Order, String> {
        let (first, rest) = args.split_first().ok_or("no arguments")?;
        if first != BITLOCKER {
            return Err(format!("{first}: not {BITLOCKER}"));
        }
        let (verb, rest) = rest.split_first().ok_or("no verb")?;
        if !rest.len().is_multiple_of(2) {
            return Err(String::from("a key without its value"));
        }
        let mut pipe = None;
        let mut volumes = Vec::new();
        for pair in rest.chunks(2) {
            match pair[0].as_str() {
                "--pipe" if pipe.is_none() => pipe = Some(pair[1].clone()),
                "--pipe" => return Err(String::from("--pipe twice")),
                "--volume" => {
                    if !volume_ok(&pair[1]) {
                        return Err(format!("--volume {}: not a volume", pair[1]));
                    }
                    if volumes.len() == MAX_VOLUMES {
                        return Err(String::from("too many volumes"));
                    }
                    volumes.push(pair[1].clone());
                }
                other => return Err(format!("{other}: not a key")),
            }
        }
        let pipe = pipe.ok_or("--pipe missing")?;
        if !pipe_ok(&pipe) {
            return Err(format!("--pipe {pipe}: not the hub app's pipe"));
        }
        let verb = match verb.as_str() {
            "show-key" | "suspend" if !volumes.is_empty() => {
                return Err(format!("{verb} takes no --volume"));
            }
            "show-key" => Verb::ShowKey,
            "suspend" => Verb::Suspend,
            "resume" => Verb::Resume(volumes),
            other => return Err(format!("{other}: not a verb")),
        };
        Ok(Order { verb, pipe })
    }
}

/// The drive a Windows folder is on (`C:\Windows` is on `C:`); `C:`
/// when the folder was not read.
pub fn drive_of(windows_dir: &str) -> String {
    let d = windows_dir.get(..2).unwrap_or("");
    if drive_ok(d) {
        d.to_ascii_uppercase()
    } else {
        String::from("C:")
    }
}

/// The shell's `System.Volume.BitLockerProtection` as the guard reads
/// it (§6.2): 1 on, 3 encrypting and 6 on and locked are protected; 5
/// is suspended. Anything else, or nothing read, is not known, which the
/// guard treats as protected.
///
/// 2 is not trusted as "not encrypted". The values come from community
/// sources, not from Microsoft, and a suspended drive, or a Home
/// computer's device encryption waiting to be turned on, might read 2
/// too. Only an elevated WMI reading of `ProtectionStatus` 0 with
/// `ConversionStatus` 0 concludes that the drive is not encrypted. Batch
/// W's first virtual machine run records what each state reads
/// (`--record-bitlocker`).
pub fn shell_protection(v: Option<i32>) -> Protection {
    match v {
        Some(1 | 3 | 6) => Protection::On,
        Some(5) => Protection::Suspended,
        _ => Protection::Unknown,
    }
}

// ---------------------------------------------------------------------
// What the verbs ask of WMI
// ---------------------------------------------------------------------

/// A method's argument or output.
#[derive(Clone, PartialEq, Eq)]
pub enum Value {
    /// A `uint32`.
    U32(u32),
    /// A string, wiped when dropped: a recovery password is one.
    Str(Zeroizing<String>),
    /// An array of strings (protector IDs).
    Strs(Vec<String>),
}

impl Value {
    /// A string value.
    pub fn text(s: &str) -> Value {
        Value::Str(Zeroizing::new(s.to_string()))
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::U32(v) => write!(f, "U32({v})"),
            Value::Str(_) => f.write_str("Str(..)"),
            Value::Strs(v) => write!(f, "Strs({v:?})"),
        }
    }
}

/// What a method returned: its `ReturnValue` and the outputs asked for
/// that it set.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Outputs {
    /// `ReturnValue`: 0 is success.
    pub ret: u32,
    /// The outputs, by name.
    pub values: Vec<(String, Value)>,
}

impl Outputs {
    fn get(&self, name: &str) -> Option<&Value> {
        self.values
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v)
    }

    /// A `uint32` output.
    pub fn u32(&self, name: &str) -> Option<u32> {
        match self.get(name)? {
            Value::U32(v) => Some(*v),
            _ => None,
        }
    }

    /// A string output.
    pub fn text(&self, name: &str) -> Option<&str> {
        match self.get(name)? {
            Value::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// A string array output; empty when it is not there.
    pub fn texts(&self, name: &str) -> &[String] {
        match self.get(name) {
            Some(Value::Strs(v)) => v,
            _ => &[],
        }
    }
}

/// A `Win32_EncryptableVolume` instance's properties.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Volume {
    /// `DeviceID`: `\\?\Volume{…}\`.
    pub device: String,
    /// `DriveLetter`: `C:`, or empty.
    pub letter: String,
    /// `VolumeType`, where Windows reports it: 0 for an
    /// operating-system volume.
    pub kind: Option<u32>,
    /// `ProtectionStatus`: 0 off, 1 on, 2 unknown.
    pub protection: u32,
}

/// A WMI call that failed before its method returned: the call and the
/// HRESULT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// What was called.
    pub call: String,
    /// Its HRESULT.
    pub code: u32,
}

impl From<Failure> for Reason {
    fn from(f: Failure) -> Reason {
        Reason::BitLocker {
            call: f.call,
            code: f.code,
        }
    }
}

/// What the verbs ask of WMI.
pub trait Wmi {
    /// Every `Win32_EncryptableVolume`.
    fn volumes(&mut self) -> Result<Vec<Volume>, Failure>;
    /// Calls `method` on the volume `device` with `args`, and reads
    /// `ReturnValue` and the outputs named in `outs`.
    fn call(
        &mut self,
        device: &str,
        method: &str,
        args: &[(&str, Value)],
        outs: &[&str],
    ) -> Result<Outputs, Failure>;
}

fn same_drive(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

fn refused(call: &str, code: u32) -> Reason {
    Reason::BitLocker {
        call: call.to_string(),
        code,
    }
}

/// How a volume stands: protection on; off and encrypted (suspended, or
/// a key left in the clear); off and not encrypted; or not known.
fn protection_of(wmi: &mut dyn Wmi, v: &Volume) -> Result<Protection, Failure> {
    Ok(match v.protection {
        1 => Protection::On,
        0 => {
            let o = wmi.call(&v.device, "GetConversionStatus", &[], &["ConversionStatus"])?;
            match (o.ret, o.u32("ConversionStatus")) {
                (0, Some(0)) => Protection::None,
                (0, Some(_)) => Protection::Suspended,
                _ => Protection::Unknown,
            }
        }
        _ => Protection::Unknown,
    })
}

/// The system drive's protection as WMI reads it: not encrypted only
/// from `ProtectionStatus` 0 with `ConversionStatus` 0; not known when
/// WMI cannot be read or does not list the drive.
pub fn state(wmi: &mut dyn Wmi, drive: &str) -> Protection {
    let Ok(vols) = wmi.volumes() else {
        return Protection::Unknown;
    };
    match vols.iter().find(|v| same_drive(&v.letter, drive)) {
        None => Protection::Unknown,
        Some(v) => protection_of(wmi, v).unwrap_or(Protection::Unknown),
    }
}

/// The recovery key found: the drive, the protector's ID and the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// The drive (`C:`).
    pub drive: String,
    /// The numerical password protector's ID.
    pub id: String,
    /// The key.
    pub key: RecoveryKey,
}

/// Show the recovery key (§6.2): the system drive's first numerical
/// password; a drive with none gets one added first, and that one is
/// read. Nothing else changes.
pub fn show_key(wmi: &mut dyn Wmi, drive: &str) -> Result<Found, Reason> {
    let vols = wmi.volumes()?;
    let v = vols
        .iter()
        .find(|v| same_drive(&v.letter, drive))
        .ok_or(Reason::NotRead)?;
    if protection_of(wmi, v)? == Protection::None {
        return Err(Reason::NotEncrypted);
    }
    let call = "GetKeyProtectors";
    let o = wmi.call(
        &v.device,
        call,
        &[("KeyProtectorType", Value::U32(NUMERICAL_PASSWORD))],
        &["VolumeKeyProtectorID"],
    )?;
    if o.ret != 0 {
        return Err(refused(call, o.ret));
    }
    let id = match o.texts("VolumeKeyProtectorID").first() {
        Some(id) => id.clone(),
        None => {
            let call = "ProtectKeyWithNumericalPassword";
            let o = wmi.call(&v.device, call, &[], &["VolumeKeyProtectorID"])?;
            if o.ret != 0 {
                return Err(refused(call, o.ret));
            }
            o.text("VolumeKeyProtectorID")
                .ok_or_else(|| refused(call, 0))?
                .to_string()
        }
    };
    if !guid_ok(&id) {
        return Err(refused(call, 0));
    }
    let call = "GetKeyProtectorNumericalPassword";
    let o = wmi.call(
        &v.device,
        call,
        &[("VolumeKeyProtectorID", Value::text(&id))],
        &["NumericalPassword"],
    )?;
    if o.ret != 0 {
        return Err(refused(call, o.ret));
    }
    let key = o
        .text("NumericalPassword")
        .and_then(RecoveryKey::parse)
        .ok_or_else(|| refused(call, 0))?;
    Ok(Found {
        drive: v.letter.clone(),
        id,
        key,
    })
}

/// Whether `v` is an operating-system volume: `VolumeType` 0, or the
/// system drive where Windows does not report the type.
fn os_volume(v: &Volume, drive: &str) -> bool {
    match v.kind {
        Some(k) => k == OS_VOLUME,
        None => same_drive(&v.letter, drive),
    }
}

/// Suspend BitLocker (§6.2) on every operating-system volume that is
/// protected, until resumed: `DisableKeyProtectors` with a restart count
/// of 0. A volume already suspended with restarts left (Control Panel's
/// suspension, which ends at the next restart) is suspended again with
/// none. A refusal is a managed computer's (§6.4), and the volumes this
/// call suspended are resumed again, so nothing is half done. Every
/// volume is read again after: one still protected is a refusal too.
/// The volumes suspended, by name and drive.
pub fn suspend(wmi: &mut dyn Wmi, drive: &str) -> Result<Vec<(String, String)>, Reason> {
    let vols = wmi.volumes()?;
    let mut targets: Vec<(&Volume, bool)> = Vec::new();
    let mut unread = false;
    let mut listed = false;
    for v in vols.iter().filter(|v| os_volume(v, drive)) {
        listed = true;
        match protection_of(wmi, v)? {
            Protection::On => targets.push((v, true)),
            Protection::Suspended => targets.push((v, false)),
            Protection::None => {}
            Protection::Unknown => unread = true,
        }
    }
    if unread {
        return Err(Reason::NotRead);
    }
    if targets.is_empty() {
        // Not encrypted only when every system volume read as such.
        return Err(if listed {
            Reason::NotEncrypted
        } else {
            Reason::NotRead
        });
    }
    let mut done: Vec<&Volume> = Vec::new();
    for &(v, on) in &targets {
        if !on {
            let o = wmi.call(&v.device, "GetSuspendCount", &[], &["SuspendCount"])?;
            if o.ret == 0 && o.u32("SuspendCount") == Some(0) {
                continue;
            }
        }
        let o = wmi.call(
            &v.device,
            "DisableKeyProtectors",
            &[("RebootCount", Value::U32(0))],
            &[],
        );
        let refusal = match o {
            Ok(o) if o.ret == 0 => None,
            Ok(o) => Some(Reason::Managed(o.ret)),
            Err(f) => Some(f.into()),
        };
        if let Some(why) = refusal {
            undo(wmi, &done);
            return Err(why);
        }
        if on {
            done.push(v);
        }
    }
    let after = wmi.volumes()?;
    let still = targets.iter().any(|(v, _)| {
        after
            .iter()
            .find(|a| a.device == v.device)
            .is_none_or(|a| a.protection != 0)
    });
    if still {
        undo(wmi, &done);
        return Err(Reason::Managed(0));
    }
    Ok(targets
        .iter()
        .map(|(v, _)| (v.device.clone(), v.letter.clone()))
        .collect())
}

/// Resumes the volumes this suspension had suspended, after a refusal.
fn undo(wmi: &mut dyn Wmi, done: &[&Volume]) {
    for v in done {
        let _ = wmi.call(&v.device, "EnableKeyProtectors", &[], &[]);
    }
}

/// Resume BitLocker (§6.3) on the marker's volumes:
/// `EnableKeyProtectors` on each still here, then each read again; one
/// not protected after is a failure, and the marker stays.
pub fn resume(wmi: &mut dyn Wmi, devices: &[String]) -> Result<(), Reason> {
    let vols = wmi.volumes()?;
    let mut resumed = Vec::new();
    for d in devices {
        let Some(v) = vols.iter().find(|v| v.device.eq_ignore_ascii_case(d)) else {
            continue;
        };
        let call = "EnableKeyProtectors";
        let o = wmi.call(&v.device, call, &[], &[])?;
        if o.ret != 0 {
            return Err(refused(call, o.ret));
        }
        resumed.push(v.device.clone());
    }
    let after = wmi.volumes()?;
    let off = resumed.iter().any(|d| {
        after
            .iter()
            .find(|a| &a.device == d)
            .is_some_and(|a| a.protection != 1)
    });
    if off {
        return Err(refused("EnableKeyProtectors", 0));
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Recorded answers
// ---------------------------------------------------------------------

fn value_text(v: &Value) -> String {
    match v {
        Value::U32(n) => format!("u32:{n}"),
        Value::Str(s) => format!("str:{}", s.as_str()),
        Value::Strs(v) => format!("strs:{}", v.join(",")),
    }
}

fn value_of(t: &str) -> Result<Value, String> {
    if let Some(n) = t.strip_prefix("u32:") {
        return n
            .parse()
            .map(Value::U32)
            .map_err(|_| format!("{t}: not a u32"));
    }
    if let Some(s) = t.strip_prefix("str:") {
        return Ok(Value::text(s));
    }
    if let Some(s) = t.strip_prefix("strs:") {
        let v = if s.is_empty() {
            Vec::new()
        } else {
            s.split(',').map(str::to_string).collect()
        };
        return Ok(Value::Strs(v));
    }
    Err(format!("{t}: not a value"))
}

fn pair_of(t: &str) -> Result<(String, Value), String> {
    let (n, v) = t
        .split_once('=')
        .ok_or_else(|| format!("{t}: not name=value"))?;
    Ok((n.to_string(), value_of(v)?))
}

/// One recorded call.
#[derive(Debug, Clone)]
struct Call {
    device: String,
    method: String,
    args: Vec<String>,
    answer: Result<Outputs, u32>,
    used: bool,
}

/// WMI's answers as recorded: snapshots of the volumes, one for each
/// time they are listed (the last repeats), and each method's answer.
///
/// ```text
/// volume \\?\Volume{…}\ C: type 0 protection 1
/// then
/// volume \\?\Volume{…}\ C: type 0 protection 0
/// call \\?\Volume{…}\ GetConversionStatus -> 0 ConversionStatus=u32:1
/// call \\?\Volume{…}\ DisableKeyProtectors RebootCount=u32:0 -> 0
/// fail \\?\Volume{…}\ GetKeyProtectors 80041003
/// ```
///
/// A `call` line answers the first call of that method on that volume
/// with those arguments not yet answered; a `fail` line has it fail with
/// that HRESULT. A call nothing answers fails with `WBEM_E_NOT_FOUND`.
/// Every call made is kept in [`Recorded::log`], as `Method device
/// args`.
#[derive(Debug, Clone, Default)]
pub struct Recorded {
    snapshots: Vec<Vec<Volume>>,
    listed: usize,
    calls: Vec<Call>,
    /// The calls made, in order.
    pub log: Vec<String>,
}

/// `WBEM_E_NOT_FOUND`.
const NOT_FOUND: u32 = 0x8004_1002;

impl Recorded {
    /// Reads a recording.
    pub fn parse(text: &str) -> Result<Recorded, String> {
        let mut r = Recorded {
            snapshots: vec![Vec::new()],
            ..Recorded::default()
        };
        for (i, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let at = |e: String| format!("line {}: {e}", i + 1);
            let t: Vec<&str> = line.split_whitespace().collect();
            match t[0] {
                "then" => r.snapshots.push(Vec::new()),
                "volume" => {
                    if t.len() != 7 || t[3] != "type" || t[5] != "protection" {
                        return Err(at(String::from("volume DEVICE LETTER type N protection N")));
                    }
                    let kind = match t[4] {
                        "-" => None,
                        n => Some(n.parse().map_err(|_| at(format!("{n}: not a type")))?),
                    };
                    let letter = if t[2] == "-" { "" } else { t[2] };
                    let protection = t[6]
                        .parse()
                        .map_err(|_| at(format!("{}: not a status", t[6])))?;
                    if let Some(s) = r.snapshots.last_mut() {
                        s.push(Volume {
                            device: t[1].to_string(),
                            letter: letter.to_string(),
                            kind,
                            protection,
                        });
                    }
                }
                "call" => {
                    let arrow = t
                        .iter()
                        .position(|x| *x == "->")
                        .ok_or_else(|| at(String::from("no ->")))?;
                    if arrow < 3 || t.len() < arrow + 2 {
                        return Err(at(String::from("call DEVICE METHOD [ARGS] -> RET [OUTS]")));
                    }
                    for a in &t[3..arrow] {
                        pair_of(a).map_err(at)?;
                    }
                    let ret = t[arrow + 1]
                        .parse()
                        .map_err(|_| at(format!("{}: not a return value", t[arrow + 1])))?;
                    let values = t[arrow + 2..]
                        .iter()
                        .map(|o| pair_of(o))
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(at)?;
                    r.calls.push(Call {
                        device: t[1].to_string(),
                        method: t[2].to_string(),
                        args: t[3..arrow].iter().map(|s| s.to_string()).collect(),
                        answer: Ok(Outputs { ret, values }),
                        used: false,
                    });
                }
                "fail" => {
                    if t.len() != 4 {
                        return Err(at(String::from("fail DEVICE METHOD HRESULT")));
                    }
                    let code = u32::from_str_radix(t[3], 16)
                        .map_err(|_| at(format!("{}: not an HRESULT", t[3])))?;
                    r.calls.push(Call {
                        device: t[1].to_string(),
                        method: t[2].to_string(),
                        args: Vec::new(),
                        answer: Err(code),
                        used: false,
                    });
                }
                other => return Err(at(format!("{other}: not a line of a recording"))),
            }
        }
        Ok(r)
    }

    /// The calls of `method` made, in order.
    pub fn made(&self, method: &str) -> usize {
        self.log
            .iter()
            .filter(|l| l.split(' ').next() == Some(method))
            .count()
    }
}

impl Wmi for Recorded {
    fn volumes(&mut self) -> Result<Vec<Volume>, Failure> {
        let i = self.listed.min(self.snapshots.len().saturating_sub(1));
        self.listed += 1;
        self.log.push(String::from("volumes"));
        Ok(self.snapshots.get(i).cloned().unwrap_or_default())
    }

    fn call(
        &mut self,
        device: &str,
        method: &str,
        args: &[(&str, Value)],
        _outs: &[&str],
    ) -> Result<Outputs, Failure> {
        let args: Vec<String> = args
            .iter()
            .map(|(n, v)| format!("{n}={}", value_text(v)))
            .collect();
        self.log.push(
            format!("{method} {device} {}", args.join(" "))
                .trim_end()
                .to_string(),
        );
        let fail = |code| Failure {
            call: method.to_string(),
            code,
        };
        let Some(c) = self.calls.iter_mut().find(|c| {
            !c.used
                && c.device == device
                && c.method == method
                && (c.answer.is_err() || c.args == args)
        }) else {
            return Err(fail(NOT_FOUND));
        };
        c.used = true;
        c.answer.clone().map_err(fail)
    }
}

/// This computer's BitLocker as a recording (`--record-bitlocker`):
/// every volume, and each one's conversion status, suspend count and
/// numerical password protectors' IDs. Read-only, and no recovery
/// password is read.
pub fn record(wmi: &mut dyn Wmi) -> String {
    use std::fmt::Write as _;
    let mut out = String::from(
        "# Win32_EncryptableVolume as faraday_hub_shell::windows::bitlocker::Recorded reads it.\n",
    );
    let vols = match wmi.volumes() {
        Ok(v) => v,
        Err(f) => {
            let _ = writeln!(out, "# volumes: {} {:08x}", f.call, f.code);
            return out;
        }
    };
    for v in &vols {
        let letter = if v.letter.is_empty() { "-" } else { &v.letter };
        let kind = v.kind.map_or_else(|| String::from("-"), |k| k.to_string());
        let _ = writeln!(
            out,
            "volume {} {letter} type {kind} protection {}",
            v.device, v.protection
        );
    }
    for v in &vols {
        for (method, args, outs) in [
            ("GetConversionStatus", vec![], vec!["ConversionStatus"]),
            ("GetSuspendCount", vec![], vec!["SuspendCount"]),
            (
                "GetKeyProtectors",
                vec![("KeyProtectorType", Value::U32(NUMERICAL_PASSWORD))],
                vec!["VolumeKeyProtectorID"],
            ),
        ] {
            let shown: Vec<String> = args
                .iter()
                .map(|(n, a)| format!(" {n}={}", value_text(a)))
                .collect();
            match wmi.call(&v.device, method, &args, &outs) {
                Ok(o) => {
                    let values: Vec<String> = o
                        .values
                        .iter()
                        .map(|(n, a)| format!(" {n}={}", value_text(a)))
                        .collect();
                    let _ = writeln!(
                        out,
                        "call {} {method}{} -> {}{}",
                        v.device,
                        shown.concat(),
                        o.ret,
                        values.concat()
                    );
                }
                Err(f) => {
                    let _ = writeln!(out, "fail {} {method} {:08x}", v.device, f.code);
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------
// The answers over the pipe
// ---------------------------------------------------------------------

/// What the elevated copy tells the window, one line each.
///
/// ```text
/// key       C:  {6A0E…}  111111-222222-…
/// suspended \\?\Volume{…}\  C:
/// state     suspended
/// ok
/// failed    Managed 2150694970
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// The system drive's protection, read at the end.
    State(Protection),
    /// The recovery key.
    Key(Found),
    /// A volume suspended.
    Suspended {
        /// `\\?\Volume{…}\`.
        device: String,
        /// Its drive.
        letter: String,
    },
    /// The step ended.
    Done(Result<(), Reason>),
}

fn protection_word(p: Protection) -> &'static str {
    match p {
        Protection::None => "none",
        Protection::On => "on",
        Protection::Suspended => "suspended",
        Protection::Unknown => "unknown",
    }
}

impl Answer {
    /// The answer as one line, with its newline, into `out`.
    pub fn encode_into(&self, out: &mut Vec<u8>) {
        out.clear();
        match self {
            Answer::State(p) => {
                out.extend_from_slice(b"state\t");
                out.extend_from_slice(protection_word(*p).as_bytes());
            }
            Answer::Key(f) => {
                // The drive and the ID are checked when read; neither can
                // hold a tab or a line end.
                out.extend_from_slice(b"key\t");
                out.extend_from_slice(f.drive.as_bytes());
                out.push(b'\t');
                out.extend_from_slice(f.id.as_bytes());
                out.push(b'\t');
                out.extend_from_slice(f.key.as_str().as_bytes());
            }
            Answer::Suspended { device, letter } => {
                out.extend_from_slice(b"suspended\t");
                out.extend_from_slice(pipe::escape(device).as_bytes());
                out.push(b'\t');
                out.extend_from_slice(pipe::escape(letter).as_bytes());
            }
            Answer::Done(Ok(())) => out.extend_from_slice(b"ok"),
            Answer::Done(Err(r)) => {
                let (name, rest) = pipe::reason_fields(r);
                out.extend_from_slice(b"failed\t");
                out.extend_from_slice(name.as_bytes());
                for f in rest {
                    out.push(b'\t');
                    out.extend_from_slice(pipe::escape(&f).as_bytes());
                }
            }
        }
        out.push(b'\n');
    }

    /// Reads one line (without its newline).
    pub fn decode(line: &[u8]) -> Result<Answer, String> {
        let line = std::str::from_utf8(line).map_err(|_| String::from("not UTF-8"))?;
        let t: Vec<&str> = line.split('\t').collect();
        Ok(match (t[0], t.len()) {
            ("state", 2) => Answer::State(match t[1] {
                "none" => Protection::None,
                "on" => Protection::On,
                "suspended" => Protection::Suspended,
                "unknown" => Protection::Unknown,
                other => return Err(format!("{other}: not a state")),
            }),
            ("key", 4) => {
                if !drive_ok(t[1]) || !guid_ok(t[2]) {
                    return Err(String::from("not a key line"));
                }
                Answer::Key(Found {
                    drive: t[1].to_string(),
                    id: t[2].to_string(),
                    key: RecoveryKey::parse(t[3]).ok_or("not a recovery key")?,
                })
            }
            ("suspended", 3) => Answer::Suspended {
                device: pipe::unescape(t[1])?,
                letter: pipe::unescape(t[2])?,
            },
            ("ok", 1) => Answer::Done(Ok(())),
            ("failed", n) if n >= 2 => {
                let fields = t[2..]
                    .iter()
                    .map(|f| pipe::unescape(f))
                    .collect::<Result<Vec<_>, _>>()?;
                let mut f = Fields(fields.iter());
                let r = pipe::reason(t[1], &mut f)?;
                f.end()?;
                Answer::Done(Err(r))
            }
            (other, _) => return Err(format!("{other}: not an answer")),
        })
    }
}

/// Writes `a` to `w` through `scratch`, which is wiped after, whatever
/// happened: the key's line is in no buffer of the hub's once it is
/// through.
pub fn send<W: Write>(w: &mut W, a: &Answer, scratch: &mut Vec<u8>) -> io::Result<()> {
    a.encode_into(scratch);
    let r = w.write_all(scratch).and_then(|()| w.flush());
    scratch.iter_mut().for_each(|b| *b = 0);
    r
}

/// Reads the next answer from `r` a byte at a time into `buf`, and wipes
/// what it put there once the line is decoded: `None` at the end, an
/// error for a line longer than `buf`, unfinished, not UTF-8 or not an
/// answer. Nothing reads ahead of the line, so no other buffer holds it.
pub fn next<R: Read>(r: &mut R, buf: &mut [u8]) -> Option<Result<Answer, String>> {
    let mut n = 0usize;
    let mut byte = [0u8; 1];
    let end = loop {
        match r.read(&mut byte) {
            Ok(0) if n == 0 => return None,
            Ok(0) => break Err(String::from("a line without its end")),
            Ok(_) if byte[0] == b'\n' => break Ok(()),
            Ok(_) if n == buf.len() => break Err(String::from("a line too long")),
            Ok(_) => {
                buf[n] = byte[0];
                n += 1;
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => break Err(e.to_string()),
        }
    };
    let answer = end.and_then(|()| Answer::decode(&buf[..n]));
    buf[..n].zeroize();
    byte.zeroize();
    Some(answer)
}

/// Everything the elevated copy answered for one step.
#[derive(Debug, Default)]
pub struct Reply {
    /// The system drive's protection, read at the end.
    pub state: Option<Protection>,
    /// The recovery key, for Show the recovery key.
    pub key: Option<Found>,
    /// The volumes suspended, for Suspend.
    pub suspended: Vec<(String, String)>,
    /// How the step ended; `None` when the copy ended without saying.
    pub done: Option<Result<(), Reason>>,
}

/// Reads every answer from `r` until it ends.
pub fn reply<R: Read>(r: &mut R) -> Result<Reply, String> {
    let mut buf = Zeroizing::new(vec![0u8; MAX_LINE]);
    let mut out = Reply::default();
    while let Some(a) = next(r, &mut buf) {
        match a? {
            Answer::State(p) => out.state = Some(p),
            Answer::Key(f) => out.key = Some(f),
            Answer::Suspended { device, letter } => {
                if out.suspended.len() < MAX_VOLUMES {
                    out.suspended.push((device, letter));
                }
            }
            Answer::Done(r) => out.done = Some(r),
        }
    }
    Ok(out)
}

/// The elevated copy's whole step against `wmi`, its answers written to
/// `w`: the key or the volumes suspended, then the system drive's state,
/// then how it ended. Whether it succeeded.
pub fn run<W: Write>(wmi: &mut dyn Wmi, verb: &Verb, drive: &str, w: &mut W) -> bool {
    let mut scratch = Zeroizing::new(Vec::with_capacity(MAX_LINE));
    let result = match verb {
        Verb::ShowKey => show_key(wmi, drive).and_then(|f| {
            send(w, &Answer::Key(f), &mut scratch)
                .map_err(|e| refused("the pipe", e.raw_os_error().unwrap_or(0) as u32))
        }),
        Verb::Suspend => suspend(wmi, drive).map(|list| {
            for (device, letter) in list {
                let _ = send(w, &Answer::Suspended { device, letter }, &mut scratch);
            }
        }),
        Verb::Resume(devices) => resume(wmi, devices),
    };
    let ok = result.is_ok();
    let _ = send(w, &Answer::State(state(wmi, drive)), &mut scratch);
    let _ = send(w, &Answer::Done(result), &mut scratch);
    ok
}

// ---------------------------------------------------------------------
// The marker, and what the window does after a step
// ---------------------------------------------------------------------

/// The marker (§6.3): the volumes the hub app suspended, and whether
/// Secure Boot was on then.
///
/// ```text
/// faraday bitlocker-suspended 1
/// volume \\?\Volume{…}\ C:
/// secure-boot on
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerFile {
    /// Each volume suspended: its name and its drive.
    pub volumes: Vec<(String, String)>,
    /// Secure Boot was on when it suspended.
    pub secure_boot_was_on: bool,
    /// When Windows started, in seconds since 1970, as it was read when
    /// the marker was written (`sys::boot_time` on Windows). A
    /// marker without it is from an earlier boot.
    pub boot: Option<u64>,
}

/// How far apart two readings of Windows' start time may be and still be
/// the same boot: the reading is the clock minus the tick count, so it
/// moves by a second or two between reads.
const BOOT_SLACK: u64 = 120;

/// Whether the marker was written in the boot now running: both start
/// times are known and agree. A marker with none recorded is from an
/// earlier boot; a start time now that cannot be read counts as the same
/// boot, so Resume is not offered on a guess.
pub fn same_boot(recorded: Option<u64>, now: Option<u64>) -> bool {
    match (recorded, now) {
        (None, _) => false,
        (Some(_), None) => true,
        (Some(a), Some(b)) => a.abs_diff(b) <= BOOT_SLACK,
    }
}

impl MarkerFile {
    /// The file's text.
    pub fn text(&self) -> String {
        let mut s = format!("{MARKER_HEAD}\n");
        for (device, letter) in &self.volumes {
            let letter = if letter.is_empty() { "-" } else { letter };
            s.push_str(&format!("volume {device} {letter}\n"));
        }
        let sb = if self.secure_boot_was_on { "on" } else { "off" };
        s.push_str(&format!("secure-boot {sb}\n"));
        if let Some(boot) = self.boot {
            s.push_str(&format!("boot {boot}\n"));
        }
        s
    }

    /// Reads a marker's text, or `None` for anything else.
    pub fn parse(text: &str) -> Option<MarkerFile> {
        let mut lines = text.lines();
        if lines.next()? != MARKER_HEAD {
            return None;
        }
        let mut volumes = Vec::new();
        let mut sb = None;
        let mut boot = None;
        for line in lines {
            let t: Vec<&str> = line.split(' ').collect();
            match t.as_slice() {
                ["volume", device, letter] if volume_ok(device) => {
                    let letter = if *letter == "-" { "" } else { letter };
                    if !letter.is_empty() && !drive_ok(letter) {
                        return None;
                    }
                    volumes.push((device.to_string(), letter.to_string()));
                }
                ["secure-boot", "on"] if sb.is_none() => sb = Some(true),
                ["secure-boot", "off"] if sb.is_none() => sb = Some(false),
                ["boot", n] if boot.is_none() => boot = Some(n.parse().ok()?),
                [""] => {}
                _ => return None,
            }
        }
        if volumes.is_empty() || volumes.len() > MAX_VOLUMES {
            return None;
        }
        Some(MarkerFile {
            volumes,
            secure_boot_was_on: sb?,
            boot,
        })
    }

    /// As the hub sees it, with Windows' start time now.
    pub fn marker(&self, boot_now: Option<u64>) -> Marker {
        Marker {
            volumes: self
                .volumes
                .iter()
                .map(|(d, l)| if l.is_empty() { d.clone() } else { l.clone() })
                .collect(),
            secure_boot_was_on: self.secure_boot_was_on,
            this_boot: same_boot(self.boot, boot_now),
        }
    }

    /// The volumes' names, for Resume.
    pub fn devices(&self) -> Vec<String> {
        self.volumes.iter().map(|(d, _)| d.clone()).collect()
    }
}

/// `%LOCALAPPDATA%\faraday\bitlocker-suspended`.
pub fn marker_path(local: &Path) -> PathBuf {
    local.join("faraday").join(MARKER)
}

/// The marker, when it is there and reads as one. A file that does not
/// read as a marker counts as none: the drive then shows as suspended by
/// someone else, and the guard is offered again, which writes it anew.
pub fn read_marker(local: &Path) -> Option<MarkerFile> {
    let text = fs::read_to_string(marker_path(local)).ok()?;
    MarkerFile::parse(&text)
}

fn file_error(path: &Path, e: &io::Error) -> Reason {
    Reason::File {
        path: path.display().to_string(),
        detail: e.to_string(),
    }
}

/// Writes the marker, through a file beside it, so a marker is whole or
/// not there.
pub fn write_marker(local: &Path, m: &MarkerFile) -> Result<(), Reason> {
    let path = marker_path(local);
    let fail = |e: io::Error| file_error(&path, &e);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(fail)?;
    }
    let tmp = path.with_extension("new");
    fs::write(&tmp, m.text()).map_err(fail)?;
    fs::rename(&tmp, &path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        fail(e)
    })
}

/// Removes the marker, and `%LOCALAPPDATA%\faraday` if that left it
/// empty. A marker not there is not an error.
pub fn remove_marker(local: &Path) -> Result<(), Reason> {
    let path = marker_path(local);
    match fs::remove_file(&path) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(file_error(&path, &e)),
    }
    let _ = fs::remove_file(path.with_extension("new"));
    if let Some(dir) = path.parent() {
        let _ = fs::remove_dir(dir);
    }
    Ok(())
}

/// After the elevated copy suspended `volumes`: the marker, with whether
/// Secure Boot is on now (not known counts as on, so Resume is never
/// offered with Secure Boot off on a computer that may have had it on),
/// and when Windows started (`boot`, so Resume waits for the next boot),
/// then the logon reminder, for an installed copy only (§6.3).
pub fn after_suspend(
    dirs: &Dirs,
    volumes: &[(String, String)],
    secure_boot: Option<bool>,
    boot: Option<u64>,
    ask: Ask<'_>,
    reg: &mut dyn Registry,
) -> Result<(), Reason> {
    write_marker(
        &dirs.local,
        &MarkerFile {
            volumes: volumes.to_vec(),
            secure_boot_was_on: secure_boot != Some(false),
            boot,
        },
    )?;
    install::add_reminder(dirs, ask, reg).map(drop)
}

/// After the elevated copy resumed: the marker and the logon reminder go.
pub fn after_resume(dirs: &Dirs, reg: &mut dyn Registry) -> Result<(), Reason> {
    remove_marker(&dirs.local)?;
    install::remove_reminder(dirs, reg)
}

/// Save (§6.2): the key's text, as the QR code carries it, written to a
/// new file in `folder` named from `name` (with `{}` for the key ID, as
/// `faraday_core::strings` gives it). Never over a file already there. Where it
/// went.
pub fn save_key_to(folder: &Path, key: &KeyShown, name: &str) -> Result<PathBuf, Reason> {
    let text = key.text.write().ok_or(Reason::Unsupported)?;
    let id = key.key_id();
    let mut last = None;
    for n in 1..=9 {
        let tag = if n == 1 {
            id.clone()
        } else {
            format!("{id} {n}")
        };
        let path = folder.join(opensigner_core::strings::fill(name, &[&tag]));
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path);
        match file {
            Ok(mut f) => {
                let mut body = Zeroizing::new(Vec::with_capacity(text.len() + 2));
                body.extend_from_slice(text.as_bytes());
                body.extend_from_slice(b"\r\n");
                return f
                    .write_all(&body)
                    .and_then(|()| f.sync_all())
                    .map(|()| path.clone())
                    .map_err(|e| file_error(&path, &e));
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => last = Some(path),
            Err(e) => return Err(file_error(&path, &e)),
        }
    }
    let path = last.unwrap_or_else(|| folder.to_path_buf());
    Err(file_error(
        &path,
        &io::Error::from(io::ErrorKind::AlreadyExists),
    ))
}
