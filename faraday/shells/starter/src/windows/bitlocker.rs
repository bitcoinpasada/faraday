//! BitLocker's steps on Windows (`docs/STARTER.md` §6, F49), with no
//! Win32 call in it, so it builds and is tested on every system:
//!
//! - [`Order`]: the elevated copy's `--bitlocker <verb>` arguments (§2).
//! - [`Wmi`]: what the verbs ask of WMI's `Win32_EncryptableVolume`
//!   (`root\CIMV2\Security\MicrosoftVolumeEncryption`), which answers
//!   in numbers in every Windows language. `sys` answers it on Windows;
//!   [`Recorded`] answers it from a recording (`tests/windows/*.wmi`),
//!   and [`record`] writes one, without any recovery password in it.
//! - [`read_key`], [`suspend`], [`resume`] and [`state`]: the verbs. Every
//!   method's `ReturnValue` is read; a non-zero one is a refusal.
//!   Suspend sets a count of [`SUSPEND_STARTS`] times Windows starts,
//!   after which Windows turns protection back on by itself; nothing is
//!   kept on the computer to undo it.
//! - [`Answer`]: what the elevated copy tells the window over its pipe,
//!   the recovery key among it, read and written through buffers that
//!   are wiped once the line is through.
//!
//! Built in 591ae189 and a18020f8, removed by F33 (53a715be) and brought
//! back by F49 without its marker, logon reminder, key file or the
//! adding of a recovery password: the Starter suspends for a count,
//! resumes, and reads the key that is there for the sheet.

use std::fmt;
use std::io::{self, Read, Write};

use faraday_starter::api::{Protection, Reason, RecoveryKey, SUSPEND_STARTS, drive_ok};
use zeroize::{Zeroize, Zeroizing};

use super::elevated::PIPE_PREFIX;
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
/// The longest line read over the pipe.
pub const MAX_LINE: usize = 4096;

// ---------------------------------------------------------------------
// The elevated copy's arguments
// ---------------------------------------------------------------------

/// What the elevated copy is asked to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    /// Read the system drive's recovery key, for the sheet.
    Key,
    /// Suspend the protected operating-system volumes for the next
    /// [`SUSPEND_STARTS`] times Windows starts.
    Suspend,
    /// Turn protection back on, now, on the suspended operating-system
    /// volumes.
    Resume,
}

/// A BitLocker step, as the elevated copy is given it.
///
/// ```text
/// --bitlocker key|suspend|resume --pipe \\.\pipe\faraday-starter-…
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    /// The step.
    pub verb: Verb,
    /// The pipe the answers go back over.
    pub pipe: String,
}

/// Whether `p` is a pipe the Starter makes.
fn pipe_ok(p: &str) -> bool {
    p.strip_prefix(PIPE_PREFIX).is_some_and(|id| {
        !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
    })
}

impl Order {
    /// The arguments, `--bitlocker` first.
    pub fn args(&self) -> Vec<String> {
        let verb = match self.verb {
            Verb::Key => "key",
            Verb::Suspend => "suspend",
            Verb::Resume => "resume",
        };
        vec![
            BITLOCKER.into(),
            verb.into(),
            "--pipe".into(),
            self.pipe.clone(),
        ]
    }

    /// Reads the arguments after the program's name, `--bitlocker` first,
    /// or says what is wrong with them.
    pub fn parse(args: &[String]) -> Result<Order, String> {
        match args {
            [first, verb, key, pipe] => {
                if first != BITLOCKER {
                    return Err(format!("{first}: not {BITLOCKER}"));
                }
                if key != "--pipe" {
                    return Err(format!("{key}: not --pipe"));
                }
                if !pipe_ok(pipe) {
                    return Err(format!("--pipe {pipe}: not the starter app's pipe"));
                }
                let verb = match verb.as_str() {
                    "key" => Verb::Key,
                    "suspend" => Verb::Suspend,
                    "resume" => Verb::Resume,
                    other => return Err(format!("{other}: not a verb")),
                };
                Ok(Order {
                    verb,
                    pipe: pipe.clone(),
                })
            }
            _ => Err(String::from(
                "--bitlocker key|suspend|resume --pipe PIPE, nothing else",
            )),
        }
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

/// The shell's `System.Volume.BitLockerProtection`, read without
/// elevation each time the Start page is shown: 1 on, 3 encrypting and
/// 6 on and locked are protected; 5 is suspended (the Windows test
/// virtual machine read 1 with BitLocker on and 5 after the Starter's
/// suspension, `tests/windows/vm-on.wmi`, `vm-suspended.wmi`). Anything
/// else, or nothing read, is not known: 2 is not trusted as "not
/// encrypted", since its meaning comes from community sources, not from
/// Microsoft. The Start page offers nothing for a drive not known.
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

/// How many more times Windows starts before a suspended volume's
/// protection comes back by itself (0: not until resumed), or `None`
/// when Windows does not say.
fn starts_left(wmi: &mut dyn Wmi, v: &Volume) -> Option<u32> {
    let o = wmi
        .call(&v.device, "GetSuspendCount", &[], &["SuspendCount"])
        .ok()?;
    (o.ret == 0).then(|| o.u32("SuspendCount")).flatten()
}

/// The system drive's protection as WMI reads it, and while suspended
/// how many times Windows starts before it comes back: not encrypted
/// only from `ProtectionStatus` 0 with `ConversionStatus` 0; not known
/// when WMI cannot be read or does not list the drive.
pub fn state(wmi: &mut dyn Wmi, drive: &str) -> (Protection, Option<u32>) {
    let Ok(vols) = wmi.volumes() else {
        return (Protection::Unknown, None);
    };
    let Some(v) = vols.iter().find(|v| same_drive(&v.letter, drive)) else {
        return (Protection::Unknown, None);
    };
    match protection_of(wmi, v).unwrap_or(Protection::Unknown) {
        Protection::Suspended => (Protection::Suspended, starts_left(wmi, v)),
        p => (p, None),
    }
}

/// Read the recovery key for the sheet (F49): the system drive's first
/// numerical password. Nothing changes on the computer: a drive with
/// none is refused, not given one.
pub fn read_key(wmi: &mut dyn Wmi, drive: &str) -> Result<RecoveryKey, Reason> {
    let vols = wmi.volumes()?;
    let v = vols
        .iter()
        .find(|v| same_drive(&v.letter, drive))
        .ok_or(Reason::NotRead)?;
    match protection_of(wmi, v)? {
        Protection::None => return Err(Reason::NotEncrypted),
        Protection::Unknown => return Err(Reason::NotRead),
        Protection::On | Protection::Suspended => {}
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
    let Some(id) = o.texts("VolumeKeyProtectorID").first().cloned() else {
        return Err(Reason::NoRecoveryKey);
    };
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
    o.text("NumericalPassword")
        .and_then(|k| RecoveryKey::parse(&v.letter, k))
        .ok_or_else(|| refused(call, 0))
}

/// Whether `v` is an operating-system volume: `VolumeType` 0, or the
/// system drive where Windows does not report the type.
fn os_volume(v: &Volume, drive: &str) -> bool {
    match v.kind {
        Some(k) => k == OS_VOLUME,
        None => same_drive(&v.letter, drive),
    }
}

/// Temporarily suspend BitLocker (§6, F49) on every operating-system
/// volume that is protected: `DisableKeyProtectors` with a
/// `DisableCount` of [`SUSPEND_STARTS`], as `manage-bde -protectors
/// -disable C: -RebootCount 3` does. Windows lowers the count each time
/// it starts and turns protection back on by itself at 0, sealing the
/// key again to the computer's state at that moment. A volume already
/// suspended is left as it is. A refusal is a managed computer's, and
/// the volumes this call suspended are resumed again, so nothing is half
/// done. Every volume is read again after: one still protected is a
/// refusal too.
pub fn suspend(wmi: &mut dyn Wmi, drive: &str) -> Result<(), Reason> {
    let vols = wmi.volumes()?;
    let mut targets: Vec<&Volume> = Vec::new();
    let mut unread = false;
    let mut suspended = false;
    for v in vols.iter().filter(|v| os_volume(v, drive)) {
        match protection_of(wmi, v)? {
            Protection::On => targets.push(v),
            Protection::Suspended => suspended = true,
            Protection::None => {}
            Protection::Unknown => unread = true,
        }
    }
    if unread {
        return Err(Reason::NotRead);
    }
    if targets.is_empty() {
        return if suspended {
            Ok(())
        } else if vols.iter().any(|v| os_volume(v, drive)) {
            Err(Reason::NotEncrypted)
        } else {
            Err(Reason::NotRead)
        };
    }
    let mut done: Vec<&Volume> = Vec::new();
    for v in &targets {
        let o = wmi.call(
            &v.device,
            "DisableKeyProtectors",
            &[("DisableCount", Value::U32(SUSPEND_STARTS))],
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
        done.push(v);
    }
    let after = wmi.volumes()?;
    let still = targets.iter().any(|v| {
        after
            .iter()
            .find(|a| a.device == v.device)
            .is_none_or(|a| a.protection != 0)
    });
    if still {
        undo(wmi, &done);
        return Err(Reason::Managed(0));
    }
    Ok(())
}

/// Resumes the volumes this suspension had suspended, after a refusal.
fn undo(wmi: &mut dyn Wmi, done: &[&Volume]) {
    for v in done {
        let _ = wmi.call(&v.device, "EnableKeyProtectors", &[], &[]);
    }
}

/// Resume BitLocker (§6, F49): `EnableKeyProtectors` on every
/// operating-system volume that is suspended, then each read again; one
/// not protected after is a failure. Whoever suspended it, the Starter
/// or Control Panel, it is the same step.
pub fn resume(wmi: &mut dyn Wmi, drive: &str) -> Result<(), Reason> {
    let vols = wmi.volumes()?;
    let mut resumed = Vec::new();
    for v in vols.iter().filter(|v| os_volume(v, drive)) {
        if protection_of(wmi, v)? != Protection::Suspended {
            continue;
        }
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
/// call \\?\Volume{…}\ DisableKeyProtectors DisableCount=u32:3 -> 0
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
        "# Win32_EncryptableVolume as faraday_starter_shell::windows::bitlocker::Recorded reads it.\n",
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
/// key       C:  111111-222222-…
/// state     suspended  3
/// state     on
/// ok
/// failed    Managed 2150694970
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// The system drive's protection, read at the end, and while
    /// suspended how many times Windows starts before it comes back.
    State(Protection, Option<u32>),
    /// The recovery key.
    Key(RecoveryKey),
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
            Answer::State(p, n) => {
                out.extend_from_slice(b"state\t");
                out.extend_from_slice(protection_word(*p).as_bytes());
                if let Some(n) = n {
                    out.push(b'\t');
                    out.extend_from_slice(n.to_string().as_bytes());
                }
            }
            Answer::Key(k) => {
                // The drive is checked when read and cannot hold a tab or
                // a line end.
                out.extend_from_slice(b"key\t");
                out.extend_from_slice(k.drive().as_bytes());
                out.push(b'\t');
                out.extend_from_slice(k.as_str().as_bytes());
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
        let protection = |w: &str| {
            Ok(match w {
                "none" => Protection::None,
                "on" => Protection::On,
                "suspended" => Protection::Suspended,
                "unknown" => Protection::Unknown,
                other => return Err(format!("{other}: not a state")),
            })
        };
        Ok(match (t[0], t.len()) {
            ("state", 2) => Answer::State(protection(t[1])?, None),
            ("state", 3) => Answer::State(
                protection(t[1])?,
                Some(t[2].parse().map_err(|_| String::from("not a count"))?),
            ),
            ("key", 3) => Answer::Key(RecoveryKey::parse(t[1], t[2]).ok_or("not a key line")?),
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
/// happened: the key's line is in no buffer of the Starter's once it is
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
    /// The system drive's protection, read at the end, and its count
    /// while suspended.
    pub state: Option<(Protection, Option<u32>)>,
    /// The recovery key, for the sheet.
    pub key: Option<RecoveryKey>,
    /// How the step ended; `None` when the copy ended without saying.
    pub done: Option<Result<(), Reason>>,
}

/// Reads every answer from `r` until it ends.
pub fn reply<R: Read>(r: &mut R) -> Result<Reply, String> {
    let mut buf = Zeroizing::new(vec![0u8; MAX_LINE]);
    let mut out = Reply::default();
    while let Some(a) = next(r, &mut buf) {
        match a? {
            Answer::State(p, n) => out.state = Some((p, n)),
            Answer::Key(k) => out.key = Some(k),
            Answer::Done(r) => out.done = Some(r),
        }
    }
    Ok(out)
}

/// The elevated copy's whole step against `wmi`, its answers written to
/// `w`: the key for [`Verb::Key`], then the system drive's state, then
/// how it ended. Whether it succeeded.
pub fn run<W: Write>(wmi: &mut dyn Wmi, verb: Verb, drive: &str, w: &mut W) -> bool {
    let mut scratch = Zeroizing::new(Vec::with_capacity(MAX_LINE));
    let result = match verb {
        Verb::Key => read_key(wmi, drive).and_then(|k| {
            send(w, &Answer::Key(k), &mut scratch)
                .map_err(|e| refused("the pipe", e.raw_os_error().unwrap_or(0) as u32))
        }),
        Verb::Suspend => suspend(wmi, drive),
        Verb::Resume => resume(wmi, drive),
    };
    let ok = result.is_ok();
    let (p, n) = state(wmi, drive);
    let _ = send(w, &Answer::State(p, n), &mut scratch);
    let _ = send(w, &Answer::Done(result), &mut scratch);
    ok
}
