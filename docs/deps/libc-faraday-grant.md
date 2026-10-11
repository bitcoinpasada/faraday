# `libc` 0.2 — Faraday's grant helper

**Where it is used:** `faraday/faraday-grant` (the `faraday-grant`
binary), on top of upstream's use in `opensigner-v4l2`
(`docs/deps/libc.md`). The version is the workspace's; no new crate
enters the lock file.

**Why it is needed.** `PLAN.md` §4.3: the helper starts as root, keeps
`CAP_CHOWN` alone, drops to its own user, empties its bounding set and
sets no-new-privileges, then changes the owner of partition nodes. `std`
offers none of these: `chown` and `chmod` on a path through `libc::chown`
and `libc::chmod`; `prctl` for the bounding set, the ambient set,
keep-capabilities and no-new-privileges; `setgroups`, `setgid` and
`setuid`; `syscall(SYS_capset)` for the capability sets. From `libc` it
takes those declarations and `SYS_capset`.

**What it is not used for.** The capability header and data structs are
declared in `faraday-grant`'s own `main.rs`. There is no seccomp filter
(`PLAN.md` §12, decision 8).

**Why `unsafe` is allowed in that crate alone.** Like `opensigner-v4l2`,
`faraday-grant` does not inherit the workspace's `unsafe_code = "forbid"`
and says so in its `Cargo.toml`. Each `unsafe` block is one `libc` call
with integers or a buffer that outlives it. The helper's decisions are in
its library, which has no `unsafe` and is tested without privilege
against a stand-in `/sys` (`faraday-grant/tests/grant.rs`).
