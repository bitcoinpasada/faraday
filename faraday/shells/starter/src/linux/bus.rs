//! udisks2 over the system bus (`rustbus`, `docs/deps/rustbus.md`): the
//! transport under [`super::Udisks`]. Everything it reads goes into
//! [`Objects`] at once; what the objects mean is [`super::udisks`]'s.
//! Beside it, one call on the session bus at start: the desktop portal's
//! light or dark preference ([`portal_color_scheme`], F126).
//!
//! The Starter never runs as root (§2, D13). udisks2 asks polkit, and
//! polkit decides whether a password is asked for: opening a disk
//! (`org.freedesktop.udisks2.open-device`) usually asks for one, once;
//! mounting and unmounting a removable disk's partitions usually does
//! not.

use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::path::PathBuf;
use std::time::Duration;

use rustbus::connection::Timeout;
use rustbus::wire::unmarshal::traits::Variant;
use rustbus::wire::{ObjectPath, UnixFd};
use rustbus::{MessageBuilder, MessageType, RpcConn};

use super::udisks::{Objects, Props, Value, short};
use super::{Open, Udisks};

const SERVICE: &str = "org.freedesktop.UDisks2";
const ROOT: &str = "/org/freedesktop/UDisks2";
/// A call that does not wait on a person.
const QUICK: Duration = Duration::from_secs(25);
/// A call polkit may hold while a person types a password.
const ASKING: Duration = Duration::from_secs(300);

/// The `a{sv}` options udisks2's methods take.
mod opt {
    // The macro's enum and variants carry no documentation of their own.
    #![allow(missing_docs)]
    use rustbus::dbus_variant_sig;
    dbus_variant_sig!(Opt, Str => String; Int => i32; Flag => bool);
}
use opt::Opt;

/// `open(2)`'s flags udisks2 accepts in `OpenDevice`'s `flags` option,
/// as Linux numbers them (`O_DIRECT` differs on Arm).
const O_EXCL: i32 = 0o200;
const O_SYNC: i32 = 0o4_010_000;
#[cfg(not(any(target_arch = "arm", target_arch = "aarch64")))]
const O_DIRECT: i32 = 0o40_000;
#[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
const O_DIRECT: i32 = 0o200_000;

/// A connection to the system bus, speaking to udisks2.
pub struct Bus {
    conn: RpcConn,
}

fn bus_error(e: impl std::fmt::Display) -> String {
    format!("the system bus: {e}")
}

impl Bus {
    /// Connects to the system bus, and asks to hear udisks2's signals:
    /// a disk plugged in or pulled out (`InterfacesAdded`,
    /// `InterfacesRemoved`) and a disk's state changing
    /// (`PropertiesChanged`, a mount).
    pub fn connect() -> Result<Bus, String> {
        let conn = RpcConn::system_conn(Timeout::Duration(QUICK)).map_err(bus_error)?;
        let mut bus = Bus { conn };
        let mut rule =
            rustbus::standard_messages::add_match(&format!("type='signal',sender='{SERVICE}'"));
        bus.call(&mut rule, QUICK)?;
        Ok(bus)
    }

    /// Sends `msg` and waits for its answer, which is an error when
    /// udisks2 (or polkit through it) refused.
    fn call(
        &mut self,
        msg: &mut rustbus::message_builder::MarshalledMessage,
        wait: Duration,
    ) -> Result<rustbus::message_builder::MarshalledMessage, String> {
        let serial = self
            .conn
            .send_message(msg)
            .map_err(bus_error)?
            .write_all()
            .map_err(|(_, e)| bus_error(e))?;
        let reply = self
            .conn
            .wait_response(serial, Timeout::Duration(wait))
            .map_err(bus_error)?;
        // rustbus reads an array's length from the body as it stands
        // and does not check it against the body's own length before
        // reading the bytes (`docs/deps/rustbus.md`): the whole body is
        // checked against its signature first.
        reply
            .body
            .validate()
            .map_err(|e| bus_error(format!("a malformed answer: {e:?}")))?;
        if reply.typ == MessageType::Error {
            let name = reply.dynheader.error_name.clone().unwrap_or_default();
            let text = reply.body.parser().get::<String>().unwrap_or_default();
            return Err(if text.is_empty() { name } else { text });
        }
        Ok(reply)
    }

    fn method(
        object: &str,
        iface: &str,
        member: &str,
    ) -> rustbus::message_builder::MarshalledMessage {
        MessageBuilder::new()
            .call(member)
            .on(object)
            .with_interface(iface)
            .at(SERVICE)
            .build()
    }

    fn options(pairs: Vec<(&str, Opt)>) -> HashMap<String, Opt> {
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }
}

/// A variant's value, as far as the Starter reads it.
fn value(v: &Variant<'_, '_>) -> Value {
    if let Ok(s) = v.get::<String>() {
        Value::Str(s)
    } else if let Ok(b) = v.get::<bool>() {
        Value::Bool(b)
    } else if let Ok(n) = v.get::<u64>() {
        Value::Num(n)
    } else if let Ok(n) = v.get::<u32>() {
        Value::Num(u64::from(n))
    } else if let Ok(p) = v.get::<ObjectPath<String>>() {
        Value::Path(p.as_ref().to_string())
    } else if let Ok(b) = v.get::<Vec<u8>>() {
        Value::Bytes(b)
    } else if let Ok(l) = v.get::<Vec<Vec<u8>>>() {
        Value::ByteLists(l)
    } else {
        Value::Other
    }
}

type Managed<'a, 'b> =
    HashMap<ObjectPath<String>, HashMap<String, HashMap<String, Variant<'a, 'b>>>>;

impl Udisks for Bus {
    fn objects(&mut self) -> Result<Objects, String> {
        let mut msg = Bus::method(
            ROOT,
            "org.freedesktop.DBus.ObjectManager",
            "GetManagedObjects",
        );
        let reply = self.call(&mut msg, QUICK)?;
        let managed: Managed<'_, '_> = reply
            .body
            .parser()
            .get()
            .map_err(|e| bus_error(format!("{e:?}")))?;
        let mut objects = Objects::default();
        for (path, ifaces) in &managed {
            let mut entry: BTreeMap<String, Props> = BTreeMap::new();
            for (iface, props) in ifaces {
                let props: Props = props.iter().map(|(k, v)| (k.clone(), value(v))).collect();
                entry.insert(short(iface).to_string(), props);
            }
            objects.0.insert(path.as_ref().to_string(), entry);
        }
        Ok(objects)
    }

    fn wait(&mut self, timeout: Duration) -> Result<bool, String> {
        match self.conn.wait_signal(Timeout::Duration(timeout)) {
            Ok(_) => {
                // Signals come in bursts (a stick brings a drive, a disk
                // and its partitions): take what is already here too.
                while self.conn.try_get_signal().is_some() {}
                Ok(true)
            }
            Err(rustbus::connection::Error::TimedOut) => Ok(false),
            Err(e) => Err(bus_error(e)),
        }
    }

    fn mount_ro(&mut self, object: &str) -> Result<PathBuf, String> {
        let mut msg = Bus::method(object, "org.freedesktop.UDisks2.Filesystem", "Mount");
        msg.body
            .push_param(Bus::options(vec![
                ("options", Opt::Str(String::from("ro"))),
                // Never a password for this: a version not read is shown
                // as not read.
                ("auth.no_user_interaction", Opt::Flag(true)),
            ]))
            .map_err(|e| bus_error(format!("{e:?}")))?;
        let reply = self.call(&mut msg, QUICK)?;
        let at: String = reply
            .body
            .parser()
            .get()
            .map_err(|e| bus_error(format!("{e:?}")))?;
        Ok(PathBuf::from(at))
    }

    fn unmount(&mut self, object: &str) -> Result<(), String> {
        let mut msg = Bus::method(object, "org.freedesktop.UDisks2.Filesystem", "Unmount");
        msg.body
            .push_param(Bus::options(Vec::new()))
            .map_err(|e| bus_error(format!("{e:?}")))?;
        self.call(&mut msg, ASKING).map(drop)
    }

    fn open(&mut self, object: &str, how: Open) -> Result<File, String> {
        let (mode, flags) = match how {
            Open::Write => ("rw", O_EXCL | O_SYNC),
            Open::ReadDirect => ("r", O_DIRECT),
            Open::Read => ("r", 0),
        };
        let mut msg = Bus::method(object, "org.freedesktop.UDisks2.Block", "OpenDevice");
        msg.body
            .push_param2(mode, Bus::options(vec![("flags", Opt::Int(flags))]))
            .map_err(|e| bus_error(format!("{e:?}")))?;
        let reply = self.call(&mut msg, ASKING)?;
        let fd: UnixFd = reply
            .body
            .parser()
            .get()
            .map_err(|e| bus_error(format!("{e:?}")))?;
        let raw = fd
            .take_raw_fd()
            .ok_or_else(|| bus_error("udisks2 answered without a file descriptor"))?;
        Ok(owned(raw))
    }

    fn rescan(&mut self, object: &str) -> Result<(), String> {
        let mut msg = Bus::method(object, "org.freedesktop.UDisks2.Block", "Rescan");
        msg.body
            .push_param(Bus::options(Vec::new()))
            .map_err(|e| bus_error(format!("{e:?}")))?;
        self.call(&mut msg, QUICK).map(drop)
    }
}

/// How long the desktop portal gets to answer at start: past this the
/// Starter opens without its answer (F126).
const PORTAL_WAIT: Duration = Duration::from_millis(700);

/// The XDG desktop portal's `org.freedesktop.appearance` `color-scheme`
/// over the session bus, the one call the Starter makes there: `ReadOne`,
/// or `Read` for a portal older than version 2 (`crate::scheme::portal`
/// reads the number). `None` when there is no session bus or portal, or
/// it does not answer within [`PORTAL_WAIT`]. Costs a connection and one
/// or two calls; made once, at start.
pub fn portal_color_scheme() -> Option<u32> {
    let conn = RpcConn::session_conn(Timeout::Duration(PORTAL_WAIT)).ok()?;
    let mut bus = Bus { conn };
    for member in ["ReadOne", "Read"] {
        let mut msg = MessageBuilder::new()
            .call(member)
            .on("/org/freedesktop/portal/desktop")
            .with_interface("org.freedesktop.portal.Settings")
            .at("org.freedesktop.portal.Desktop")
            .build();
        msg.body
            .push_param2("org.freedesktop.appearance", "color-scheme")
            .ok()?;
        let Ok(reply) = bus.call(&mut msg, PORTAL_WAIT) else {
            continue;
        };
        let Ok(v) = reply.body.parser().get::<Variant<'_, '_>>() else {
            continue;
        };
        // `Read` wraps the value in one more variant.
        let n = v
            .get::<u32>()
            .ok()
            .or_else(|| v.get::<Variant<'_, '_>>().ok()?.get::<u32>().ok());
        if n.is_some() {
            return n;
        }
    }
    None
}

/// The descriptor udisks2 passed, as a `File` that closes it.
#[allow(unsafe_code)]
fn owned(raw: std::os::fd::RawFd) -> File {
    use std::os::fd::FromRawFd;
    // SAFETY: `raw` came from `UnixFd::take_raw_fd`, which hands over
    // ownership of a descriptor the kernel installed in this process
    // when the reply's SCM_RIGHTS message was received: it is open, and
    // nothing else (rustbus included, once taken) closes it, so the
    // `File` is its only owner.
    unsafe { File::from_raw_fd(raw) }
}
