# Compute: costly work takes a token

**Kind:** Spec
**Open:** The token's two hooks are built (`faraday-crypto`) and the root thread's token is minted in `Runtime::new`; the pool's lane threads that mint the other one (spec 05 §5.1), the costs lint's list of the two mint sites and the named costs of §5 are not built.

Every costly function in `core/` and in the application crates takes a `&Compute`, a token. The
runtime mints it in two places, and nothing else mints one in a shipped crate (R10). Drawing gets a
`View` with no token, so a screen that calls costly work does not compile (A19).

## 1. The guarantee

- A function that draws reaches `View` and `Show` only. Neither holds a `Compute` (V2, V14): the
  memo's `fill` is a function of its `input` alone.
- `update`, `start`, `notice` and a job's work get a `Ctx`, which is `View` plus `&Compute`; a
  costly root asks for `c: &Compute` and counts its work through it on its first line.
- What the types cannot hold is held by clippy's `disallowed-methods` on `Compute::new` and by
  `costs-lint`, which lists the two mint sites by path (§3).

## 2. The token

```rust
// core/faraday-crypto/src/work.rs
pub struct Hooks { pub count: fn(Work), pub stop: fn() -> bool, pub progress: fn(u32, u32) }
impl Compute {
    pub fn new(hooks: Hooks) -> Compute;
    pub fn count(&self, w: Work);
    pub fn stop_asked(&self) -> bool;
    pub fn progress(&self, done: u32, of: u32);
}
```

`Compute` is not `Clone`, `Copy`, `Default`, `Send` or `Sync`, so it cannot be put in a `static`
or sent to a job. The hooks are function pointers, so `faraday-crypto` stays `no_std` and the token
stays `!Send`. A test or a tool that has no drawing guarantee to keep mints with
`Compute::unchecked()`.

## 3. Who mints

- **The root thread**, once, in `Runtime::new` (`runtime/faraday-runtime/src/runtime.rs`). Its hooks
  count nothing it reports, never ask a job to stop and ignore progress. Every `Ctx` of event
  handling lends this token, and the self-test at `Start` runs with it.
- **Each pool lane thread**, once, as it starts (`runtime/faraday-runtime/src/pool.rs`): its hooks
  read the current job's signal from a thread-local (stop flag, two progress counters).

Each of the two sites is under `#[expect(clippy::disallowed_methods, reason = "R10 ...")]`. A third
mint in a shipped crate is refused by the lint; a test may mint with `Compute::unchecked()`.

## 4. The drawing side

Drawing works nothing out. What a frame needs that costs is kept in the memo (spec 04 §6), filled
by a function of its public input, or worked out in `update` and kept in the feature's state as a
`Derived`. A debug build fills a memo hit again and asserts equality (V13).

## 5. Named costs

A named costly operation is held by an instruction count, raised only in a commit that says why,
and a stated cost says how it was measured (decision F238). The counts and their tests come with
the features that own the operations.
