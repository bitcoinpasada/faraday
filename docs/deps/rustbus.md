# rustbus

**Purpose.** The Linux Starter writes sticks and cards through udisks2
and never runs as root (`docs/STARTER.md` §2, D13). It talks to udisks2 on
the system bus: `GetManagedObjects` for the disks, its signals for a
disk plugged in or pulled out, `Filesystem.Mount` (read-only) and
`Filesystem.Unmount`, `Block.OpenDevice`, which passes back an open
file descriptor for the disk, and `Block.Rescan`.
`faraday/shells/starter/src/linux/bus.rs` is the only code that uses it, and
only on Linux; what udisks2's answers mean is read in `udisks.rs` with
no D-Bus in it, so the tests feed recorded answers.

**Why this crate.** It is pure Rust, blocking, and small, and it passes
file descriptors:

- Blocking calls with a timeout (`RpcConn`), so the Starter runs it on
  plain threads (the disk watcher, the writer) with no async runtime.
- `UnixFd` and the `SCM_RIGHTS` handling in its connection: a reply's
  descriptors are attached to the message, and `take_raw_fd` hands one
  over. `bus.rs` turns it into a `File` in one audited `unsafe` line
  (`owned`), which std requires for any raw descriptor.
- The dependency graph is six crates new to the lock file: `rustbus`,
  `rustbus_derive` (a derive macro, with `syn` 1.0.109, an older copy of
  the `syn` already in the graph), `nix` 0.26.4 (sockets and `recvmsg`;
  the graph had none), `memoffset` 0.7.1 and `pin-utils` 0.1.1 (both
  `nix`'s). `thiserror` 1, `bitflags` 1, `libc`, `cfg-if`, `quote` and
  `proc-macro2` were there already. MIT, which `deny.toml` allows.

Read in its source for this choice: the connection and authentication
(`connection/ll_conn.rs`, `auth.rs`: SASL `EXTERNAL` by uid, then
`NEGOTIATE_UNIX_FD`), the descriptor wrapper (`wire/wrapper_types/
unixfd.rs`), and the unmarshalling of what `bus.rs` reads (`wire/
unmarshal/`). One thing found and worked around: a received message's
body is not checked against its signature before it is read, and
reading an array of fixed-size elements (`ay`, which udisks2 uses for
paths) takes the array's length from the body and builds the slice
with `from_raw_parts` without comparing it with the bytes the body
holds. A body that lies about an array's length would read past it.
The system bus's daemon validates what it routes, so this needs a
malicious bus, but `bus.rs` calls `MarshalledMessageBody::validate` on
every reply before reading it, which walks the body against its
signature with bounds checks. The crate's other `unsafe` is the same
fast path for fixed-size slices when marshalling and the socket's
`from_raw_fd`.

**Cost.** Version 0.19.3, pinned exactly (`=0.19.3`), default features
(it has none to turn off). About 11,500 lines with tests and benches.
The last release is from August 2023; the D-Bus wire format and the
udisks2 calls the Starter makes have not changed since.
`supply-chain/config.toml` carries the six crates as exemptions; the
parts listed above were read, the rest was not.

**Not chosen.**

- `zbus` 5: the maintained, complete implementation, and the one most
  Rust desktop software uses, but async first. Its blocking API runs
  an async executor underneath and needs `async-io` or `tokio`: 87
  crates in its graph against `rustbus`'s 18.
- `dbus` (and `dbus-tokio`, `dbus-crossroads`): bindings to libdbus, a
  C library; the Starter loads no C library it does not need, as with
  liblzma (§1.3).
- `tokio-dbus`: pure Rust, on tokio.
- `mini-sansio-dbus`: a sans-I/O protocol crate, first published in
  2026, a few hundred downloads.
- Writing the protocol here: authentication, marshalling and descriptor
  passing are a few hundred lines, and a library that has had other
  users is the better place for them.

**Reopen when** a `rustbus` release appears (read its changelog and
the diff of the parts above), when `zbus` offers a blocking use with
no executor, or if the Starter stops needing D-Bus on Linux.

**Review.** Not reviewed: an exemption in `supply-chain/config.toml` (F71).