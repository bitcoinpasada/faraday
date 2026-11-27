# The skeleton manifest: crates, modules, types, traits and signatures

**Kind:** Spec
**Open:** Not built.

This section lays out the workspace of `next` from sections 01 to 08: the tree, every crate with
its directory, its `[package.metadata.faraday]` values and its edges to other workspace crates,
the modules of each kernel, kit, application, runtime, adapter, root and tool crate, and the
public items each exports, as Rust sketches without bodies. For the domain crates copied from
`main` it gives the list and the edges cut, not their API. It places every file of the inventory
(`docs/migration/inventory/features/`) in exactly one crate. It is written for whoever builds the
skeleton (the plan's §4, step 3: this manifest compiled, every body `todo!()`) and for whoever
adds a crate later. A shape already given in an earlier section is cited, not repeated. It answers
the questions sections 01, 03, 04, 05, 06, 07 and 08 left for it.

## 1. The tree

### 1.1 The top level

This is `README.md`'s "Repository layout" and the `TOP` list of `tools/lint-tree.sh` on `next`.

| Path | What it holds |
|---|---|
| `core/` | the domain and the port: every crate of `layer = "domain"` and `layer = "port"` (§2) |
| `kit/` | the UI kit, tokens to templates (§3) |
| `app/` | the application: the kernels, parts, features, composites, owners and product crates (§4) |
| `runtime/` | the runtime (§5) |
| `adapters/` | the adapters and the two adapter processes (§6) |
| `roots/` | the root binaries and the tour root (§7) |
| `tools/` | the tool and test-support crates, the lints, the cross-check, references, vectors and the scripts `just` runs (§8) |
| `build/` | the image build and the build list: `main`'s `faraday/image/` (as `build/image/`), `faraday/build/` (as `build/device/`, `build/linux/`, `build/windows/`), `faraday/builds.toml`, `faraday/release-key.asc` and `faraday/faraday.just` (as `build/build.just`, imported by the justfile) |
| `fuzz/` | the fuzz targets: `main`'s `fuzz/` as it is, and `main`'s `faraday/fuzz/` as `fuzz/faraday/`, each its own workspace with its lockfile |
| `third_party/` | vendored crates, each with its patches (`rqrr`) |
| `supply-chain/` | cargo vet's audits and exemptions |
| `docs/` | what is true now or still to be built |
| `.github/`, `.config/`, `.claude/` | as on `main` |
| files | `.gitignore`, `CLAUDE.md`, `CONTRIBUTING.md`, `Cargo.lock`, `Cargo.toml`, `LICENSE`, `README.md`, `SECURITY.md`, `deny.toml`, `justfile`, `rust-toolchain.toml` |

`main`'s `faraday/` is gone: its crates are placed by layer, and its `faraday/tools/*.py` and
`stage-release.sh` move to `tools/`. **Decision: the domain keeps `main`'s directory name,
`core/`.** Reason: `main`'s rule "`core/` is Faraday's own code" carries (08 §3.7), and sections
05 and 08 already name `core/` paths. Turned down: `domain/`, which would rename every path the
carried documents cite.

### 1.2 Naming

- A crate's directory is its layer's directory, then for `app/` its role's folder, then the
  crate's name: `app/features/faraday-decode/`. Copied crates keep their names; `main`'s
  `faraday-shell-api` is renamed `faraday-port`, and `faraday-ui` and `faraday-draw` are
  rearranged (§3).
- The sketches of sections 03 to 06 name an application crate by its short name (`device::Page`,
  `create::MakeWallet`). The short name is the crate's name without `faraday-` and without
  `-area`, imported as `use faraday_wallets_area as wallets;`. No dependency is renamed in a
  `Cargo.toml`, so `lint-layers`' module rule and V7 read exact paths.
- A crate's enum of its screens is `Page` (06 §4.2); the presentation kernel's trait of the same
  name is always named by its path, `faraday_present::Page`.
- The composites of the device composite carry `-area` (`faraday-keys-area`) so that none takes a
  domain crate's name (`faraday-keys`, `faraday-files`).

### 1.3 The workspace manifest and the metadata

The root `Cargo.toml` lists every crate of §2 to §8 as a member (resolver 2), sets
`[workspace.lints.rust] unsafe_code = "forbid"` (U1), and holds the workspace's outside
dependencies. Each crate carries `[lints] workspace = true`, but for the U2 list (§1.5), and its
`[package.metadata.faraday]` as section 01 §1 gives it, extended with the keys the earlier sections
name: `no_std` (D2), `boundary` (§2.3 there), `wording` (§2.2 there), `owner` (A24), `online` and
`offline` (S8), `ships` (S5), `checks` (07 TS13), `process` (S2), `values` (P4) and `reexports`
(K7). The tables below give each crate's values; a key not given is absent.

It also holds `main`'s profiles as they are: `[profile.release]` with `panic = "abort"`, and
`overflow-checks = true` for `faraday-fat`, `faraday-bip`, `faraday-codec` and `faraday-psbt`,
whose arithmetic on lengths read from untrusted input must stop the app on a wrap rather than read
on at a wrong bound; the per-package speed settings come with the packages they name.

### 1.4 The clippy configuration of each directory

Clippy reads the nearest `clippy.toml` and nothing above it, so each file below holds every set it
needs. `tools/clippy.py` (as on `main`) checks that the copies of each set are identical and that
it names no path that resolves to nothing. The domain's directories carry no io set: D1 is read
over library code only (`src/` outside `#[cfg(test)]` modules) by `lint-layers`' io rule, since
the domain's tests and examples name these paths to read fixtures and to time, and clippy's
`disallowed_methods` cannot be allowed for them without allowing the mint and keys sets.

| Set | What it refuses |
|---|---|
| **io** | D1's names, in the kit and the application: `std::fs`, `std::net`, `std::process`, `std::env`, `std::thread::spawn`, `SystemTime::now`, `Instant::now` |
| **io-rt** | R3: `std::fs`, `std::net`, `std::process`, `std::env` |
| **mint** | R2, R10: `Compute::new` and `Compute::unchecked` (allowed only under `#[expect(..., reason)]`) |
| **keys** | the key boundary and costly-work paths of `main`'s `faraday/clippy.toml` (70 paths: seeds in hand, signing, vault open and seal, master keys) |
| **app** | A16: `AskFor::into_parts`, `ChangeFor::into_parts`, `Effects::into_list`, `Job::into_parts`; R1 and 05 §6: `Memo::new`, `Memo::end_frame`, `Memo::clear`; 04: `Show::new`; R4: `Runtime::record_ink` |
| **owner** | A35: `Job::heavy`, `Job::seal` |

| Directory | `clippy.toml` holds | Notes |
|---|---|---|
| `core/` | mint, keys | the port included (P6); io is `lint-layers`' (D1). A domain crate with a file of its own refuses `Compute::new` alone: its tests mint with `Compute::unchecked` throughout (or with `Compute::new` under `#[expect]`, as kef's do), which clippy cannot allow for tests alone, and the `unchecked` feature, enabled only by dev-dependencies, keeps it out of shipped code |
| `core/faraday-{bip,crypto,entropy,keys,vault,pgp,sb,kef}/` | `Compute::new`, and `main`'s own file of that crate | `main`'s per-crate copies |
| `core/faraday-{psbt,codec,selftest}/` | `Compute::new` | below `faraday-keys`, so the keys set names nothing they can call |
| `kit/` | io, mint, keys | |
| `app/` | io, mint, keys, app, owner | every application crate but the owners |
| `app/owners/` | io, mint, keys, app | A35's calls allowed |
| `runtime/` | io-rt, mint, keys, app (but `into_parts`) | `Compute::new` under `#[expect]` at the two sites of §5.2 |
| `adapters/` | mint, keys, app | |
| `roots/` | mint, keys, app | |
| `roots/faraday-snapshot/` | mint, keys | the tour root may call `record_ink` |
| `tools/` | mint, keys | `Compute::unchecked` under `#[expect]`; `record_ink` allowed |

### 1.5 unsafe code

The U2 list, by crate name: `faraday-crypto`, `faraday-storage`, `faraday-grant`, `faraday-v4l2`,
`faraday-mediafoundation`, `faraday-window`, `faraday-os-adapter`, `faraday-alloc-count`,
and `faraday-leak-scan`. The stick adapter is not on it: it holds no `unsafe`, as `main`'s stick
shell holds none, and inherits the workspace's `forbid`. The two ioctls it calls (the vblank wait
and the input grab) are `faraday-v4l2`'s `wait_vblank` and `grab_input`, as in `main`'s
`shells/v4l2`. No root keeps `unsafe`.

## 2. The domain and the port

### 2.1 The domain crates

Every crate here is `layer = "domain"`, in `core/`, copied from `main` at `d89dde5de` with its
own tests (D3). Edges are to workspace crates; `main`'s edges are kept but for those struck in the
last column.

| Crate | From `main` | Metadata | Normal edges on `next` | Dev edges | Cut or changed |
|---|---|---|---|---|---|
| `faraday-text` | `core/` | `no_std` | none | none | gains `secret::{SecretText, ROOM}` |
| `faraday-crypto` | `core/` | `no_std` | text | itself, with `unchecked` (its own tests mint with `Compute::unchecked`) | `work::{Hooks, Compute::new(Hooks)}` (05 §5.2) |
| `faraday-bip` | `core/` | `no_std` | crypto, text | crypto | |
| `faraday-psbt` | `core/` | `no_std` | bip, crypto, text | crypto | |
| `faraday-codec` | `core/` | `no_std` (without `decode`) | bip, crypto, text | crypto | gains `otpauth::is_otpauth` |
| `faraday-entropy` | `core/` | `no_std` | bip, crypto | none | |
| `faraday-selftest` | `core/` | `no_std` | bip, crypto, psbt, text | crypto | the mount-list read moves to the stick adapter |
| `faraday-handwriting` | `faraday/` | | bip, text | none | no application crate depends on it (F325: compiled out of the app) |
| `faraday-boot` | `faraday/` | | none | none | |
| `faraday-payload` | `faraday/` | | none | none | |
| `faraday-keys` | `faraday/` | `boundary` | bip, codec, crypto, entropy, kef, pgp, psbt, sb, strings, text, vault | none | ~~forms~~, ~~intake~~; gains `Refusal`, `Makes` |
| `faraday-vault` | `faraday/` | `no_std`, `boundary` | bip, codec, crypto, entropy, text | alloc-count | ~~intake~~ |
| `faraday-pgp` | `faraday/` | `no_std` | crypto, text | none | |
| `faraday-sb` | `faraday/` | `no_std` | crypto, pgp, text | none | |
| `faraday-kef` | `faraday/` | `no_std` | codec, crypto | none | |
| `faraday-ots` | `faraday/` | `no_std` | crypto | none | |
| `faraday-plan` | `faraday/` | | bip, crypto, entropy, forms, strings, text, vault | none | |
| `faraday-fat` | `faraday/` | `no_std` | text | none | |
| `faraday-sqlite` | `faraday/` | | text | none | |
| `faraday-core-wallet` | `faraday/` | | bip, crypto, keys, sqlite, strings, text | none | ~~intake~~ (`network_name` to strings) |
| `faraday-qr` | `faraday/` | | bip, codec, crypto, text | alloc-count, codec, files | |
| `faraday-files` | `faraday/` | | codec, crypto, fat, strings, text | none | its folder `Place` to the desktop adapter |
| `faraday-intake` | `faraday/` | | bip, codec, crypto, qr, strings, text | none | loses `SecretText`, `ROOM`, `is_otpauth`, `network_name` (to strings) and `Refusal`'s session variants (to `faraday-keys`); keeps its own answer, `NotAWallet` |
| `faraday-forms` | `faraday/` | | keys, strings | none | gains the edge to keys (`Makes`) |
| `faraday-paper` | `faraday/` | | codec, crypto, strings, tokens | crypto (`unchecked`) | ~~draw~~ (to `faraday-tokens`); gains `blank::{sheet_blank, codex32_printed}` |
| `faraday-strings` | `faraday/` | `wording` | bip, crypto, text | none | gains `network_name` |
| `faraday-guide` | `faraday/` | `wording` | strings | none | |
| `faraday-samples` | `faraday/` | `online` | bip, forms, text | bip, codec, crypto, intake, keys, ots, psbt, strings, testkit, text, vault | ~~core~~ (normal and dev), ~~host~~, ~~shell-api~~ (dev) |

- **The samples' types** (`SampleKey`, `SampleForm`, `SampleSize`, `SampleKind`, `SampleScript`,
  `SampleWallet`, `SampleInputs`, `SampleHsm`, `Samples`, from `faraday-core`'s `samples` module)
  move into `faraday-samples`' `types` module, over `faraday_forms::Form`. Its copied tests lose
  their `faraday-core` and `faraday-host` edges; a test that needed either is not copied, and is
  named in section 10's mapping. The test wallets they read come from `faraday-testkit` (section
  10 §7.2). This is the one copied crate whose tests change beyond D3's cuts.
- **The moved types' paths:** `faraday_text::secret::{SecretText, ROOM}`;
  `faraday_keys::{Refusal, Makes}`; `faraday_codec::otpauth::is_otpauth`;
  `faraday_strings::network_name`; `faraday_paper::blank`.
- **Who may name `faraday-samples`:** only `online` crates (S8). A press in the device composite
  that loads a sample (Tools' Use a sample, Spend from a backup's sample PSBT, the codex32 walk
  through) sends an `Out` that the online composite routes to the strip, which loads it (§4.7).

### 2.2 Two domain crates that keep a little I/O

`faraday-files` keeps its rules over `Place`; its FAT `Place` is over `faraday-fat` and does no
`std::fs`. `faraday-selftest` keeps its vectors. On `main` the one file of `faraday-vault` that
names an I/O path (`git grep` of the io set's names) is a `#[cfg(test)]` module reading
toml-test's vendored cases; it is test code, outside D1's library code, and is copied as it is
(section 10 §7.1).

### 2.3 The port

`faraday-port`, `core/faraday-port/`, `layer = "port"`, `no_std`, `values = "values"`. Normal
edges: `faraday-text`, `faraday-crypto` (P5). Dev edges: none. Its modules and items are 02 §1's
table, and their shapes 02 §2 to §6. Exports at the crate root: `Event`, `Effect`, `Ask`,
`Answer`, `AskId`, `Request`, `Order`, `Exit`, `Frame`, `Adapters`, `Waker`, `Failure`, and the
modules `event`, `ask`, `order`, `frame`, `values`, `bytes`, `storage`, `host`, `camera`,
`devices`, `system`.

## 3. The kit

### 3.1 Crates

All in `kit/`, `layer = "kit"`.

| Crate | Level | Metadata | Normal edges | Dev edges | From `main` |
|---|---|---|---|---|---|
| `faraday-tokens` | tokens | `no_std` | none | none | `core/faraday-draw/src/tokens.rs`; `faraday-ui`'s `pal`, `theme` |
| `faraday-draw` | atoms | `no_std` | tokens, port (`values`), text, crypto, codec | none | the rest of `core/faraday-draw` |
| `faraday-atoms-derive` | atoms | proc-macro | none | none | new: `#[derive(Pressed)]` and `#[hides]` (04 §2, §11.1) |
| `faraday-atoms` | atoms | | draw, atoms-derive, tokens, port (`values`), text, codec | none | `faraday-ui`'s `lib.rs`, `controls`, `press`, `region`, `hold`, `mark` |
| `faraday-molecules` | molecules | | atoms, draw, tokens, strings, codec, text | none | `faraday-ui`'s `lists`, `parts`, `roles`, `tabs`, `nav::chip`, `stick`, `appearance`, `flow`; `faraday-core`'s `screens/parts.rs` |
| `faraday-organisms` | organisms | `reexports` (§3.3) | molecules, atoms, tokens, strings, guide, codec, text | none | `faraday-ui`'s `sheet`, `menu`, `snackbar`, `tooltip`, `keyboard`, `nav`, `how`, `demo`; `faraday-core`'s `screens/session_pane/`, `ways_in`, `scan_frame.rs`, worksheet, chart and board drawing |
| `faraday-organisms-online` | organisms | `online`, `reexports` | molecules, atoms, tokens, strings | none | `faraday-ui`'s host strip drawing (`faraday-host`'s) |
| `faraday-templates` | templates | `reexports` (§3.3) | organisms, molecules, atoms, tokens, strings | none | `faraday-core`'s `screens/hub_layout/`, `flow.rs`, `flow/`, `screens/menu_page.rs`, the frame of `screens/mod.rs` |

**Decision: the atoms level is three crates: the drawing base, the derive and the controls.**
Reason: `faraday-draw` is `no_std` on `main` and stays so (D2), while `faraday-ui`'s frame and
controls use `std` (`Rc`, collections); one crate would force a rewrite of one or the other.
Turned down: one atoms crate made `no_std`, which rewrites `faraday-ui`'s copied code. **Decision:
the host strip's organism is a crate of its own, `online = true`**, so that no drawing of the strip
reaches the device's graph (S8). The "Demo only" card is drawn by device-composite pages in
Practice and stays in `faraday-organisms`. Both need same-level edges (asked of section 01, K1).

### 3.2 Modules and what each crate exports

- **`faraday-tokens`**: `dims` (every named dimension), `palette` (`Role`, the scheme table,
  fixed colours, stick tones), `type_roles`, `motion` (durations, easing, `STEP_MS` = 300 ms), `elevation`;
  `Theme`, `TextSize`, `Contrast`, `Mode` (light, dark). Constants and their helpers only (K2).
- **`faraday-draw`**: `canvas` (`Canvas`, the call list `Calls`, the ink record `Ink` of
  `Inked` entries, 04 §9), `surface`, `blend`, `geom` (`Rect`, `Size`; `Point` from the port),
  `text` (`Font`, shaping and measure), `fonts` (the baked fonts), `shapes`, `icons` (`Icon`),
  `ease`, `qr` and `picture`:

```rust
/// `qr`: `code` written into `at` on a white square, whole pixels per module, the quiet zone
/// included, centred; each row of modules written straight into the canvas and nothing kept,
/// so no secret's code outlives its frame (`main`'s `faraday-ui` `parts::qr_code`).
pub fn modules(canvas: &mut Canvas, code: &faraday_codec::qr::QrMatrix, at: Rect);

/// `picture`: costs what `faraday_codec::png` costs. `code` with `title` and then each of
/// `lines` under it, drawn in the baked fonts and written as a PNG file; the pixels drawn are
/// wiped before it returns (`main`'s `faraday-core` `picture::labelled_png`).
pub fn labelled_png(c: &Compute, code: &QrMatrix, title: &str, lines: &[&str]) -> Secret<Vec<u8>>;
```

  `labelled_png` takes the compute token because writing the PNG is costly work, and returns a
  wiping buffer because the code may be a secret's (a SeedQR, a codex32 share). Its words are
  its caller's (section 10 §6).
- **`faraday-atoms`**:

```rust
pub use faraday_atoms_derive::Pressed;
pub trait Pressed: Clone + PartialEq + Send + 'static { fn hides(&self) -> bool; }   // 04 §2
pub struct Ui<'f, A: Pressed> { /* 04 §7.1 */ }
pub struct FrameOut<R: Pressed> { /* 04 §7.1 */ }
pub struct Hit<R> { pub rect: Rect, pub target: Rect, pub press: R, pub layer: u8 }   // 04 §7.1
pub struct Scrolled { pub id: Anchor, pub view: Rect, pub extent: Size }             // 04 §7.1
pub struct Pointer<R> { pub at: Option<Point>, pub down: bool, pub focused: Option<R> }  // 04 §7.1
pub enum Laid { Placed(Placement), Room(bool), Fits(bool), Rows(u16) }
pub enum Placement { Stacked, Beside, OneColumn }
pub struct MotionStore { /* 04 §8.1 */ }
pub enum Motion { /* 04 §8.1 */ }
pub enum Turn { Page, Step }                                // 04 §8.1
pub struct ComponentId(/* page Anchor, component Anchor */);
pub struct Shown<'s>(/* private */);          // 04 §11
pub struct PlainRef<'s>(/* &'s Text | &'static str */);
pub enum Offered<R> { Yes(R), Refused(R, Drawn), No }    // 04 V11: a control's press, or why not
pub enum Drawn { Dimmed, AsUsual }                        // the kernel's Refused::drawn answer (03 §10)
pub enum Style { Filled, Tonal, Outlined, Text, Elevated, Error }
pub use faraday_tokens::{Theme, TextSize};
impl<'f, A: Pressed> Ui<'f, A> {
    pub fn new(canvas: &'f mut Canvas, theme: Theme, scale: f32, motion: &'f mut MotionStore,
               pointer: &'f Pointer<A>, ink: bool) -> Self;  // the runtime's only
    pub fn finish(self) -> FrameOut<A>;                     // the runtime's only
    pub fn within<B: Pressed>(&mut self, lift: fn(B) -> A, f: impl FnOnce(&mut Ui<'_, B>));
    pub fn anchor(&mut self, id: Anchor);
    pub fn laid(&mut self, id: Anchor, fact: Laid, to: fn(Laid) -> A);
}
```

  Its modules: `ui`, `frame_out`, `hits`, `focus`, `region`, `motion`, `shown`, `offered`, and one
  per control (`button`, `fab`, `chip`, `pill`, `tag`, `badge`, `check`, `switch`, `segmented`,
  `icon_button`, `stepper`, `progress`, `hold`, `field`, `line`, `rule`, `mark`, `shadow`,
  `frost`), each a `pub fn` over `&mut Ui<A>` and a model struct (§3.4). `focus` holds
  `step`, the Tab order's move (04 §7.1). `Drawn` is the kit's, and the
  kernel's `Refused::drawn` returns it through the presentation kernel (asked of section 03 below:
  the kernel may not name the kit, so `Drawn` is a kernel enum the presentation kernel maps).
- **`faraday-molecules`**: one module per molecule of 04 §12.1's row (`row`, `kind_rows`,
  `options`, `multi`, `chips`, `buttons`, `code_box`, `address`, `pass`, `words`, `pads`,
  `time_zone`, `appearance`, `tabs`, `destination`, `stick_pill`, `status`, `flow_text`,
  `guide_block`, `caution`, `qr`), each a model struct (§3.4) and a `pub fn draw<A: Pressed>(&Model,
  &mut Ui<A>)`.
- **`faraday-organisms`** and **`faraday-templates`**: §3.3.

### 3.3 Organisms and templates: which crate and module holds each

This answers section 04's question.

| Organism or template (04 §12.1, §12.3, §12.4) | Crate | Module | Model |
|---|---|---|---|
| the content organism | `faraday-organisms` | `content` | `Content<'s, A>`, `Block<'s, A>` (04 §12.3) |
| the session pane (rows, the Waiting band, pick rows, vault marks, details, the scan in the pane) | `faraday-organisms` | `session_pane` | `SessionPaneModel<'s, A>` |
| sheets, dialogs, the side sheet | `faraday-organisms` | `sheet` | `SheetModel<'s, A>`, `DialogModel<'s, A>` |
| menus | `faraday-organisms` | `menu` | `MenuModel<'s, A>` |
| the snackbar | `faraday-organisms` | `snackbar` | `SnackbarModel<'s>` |
| tooltips | `faraday-organisms` | `tooltip` | `TipModel<'s>` |
| ways-in | `faraday-organisms` | `ways_in` | `WaysInModel<'s, A>` |
| the navigation rail, the navigation bar, the top app bar | `faraday-organisms` | `nav` | `NavModel<'s, A>` (drawn as the rail by `draw`, as the bar by `draw_bar`), `TopBarModel<'s, A>` (`draw_top`) |
| the viewfinder | `faraday-organisms` | `viewfinder` | `ViewfinderModel<'s, A>` |
| the on-screen keyboard's panel | `faraday-organisms` | `keyboard` | `KeyboardModel<A>` |
| the guide's pages | `faraday-organisms` | `guide` | `GuidePageModel<'s, A>` |
| the codex32 worksheet | `faraday-organisms` | `worksheet` | `WorksheetModel<'s, A>` |
| the structure chart and the wallet cards | `faraday-organisms` | `chart` | `ChartModel<'s, A>`, `WalletCardModel<'s, A>` |
| the backup board | `faraday-organisms` | `board` | `BoardModel<'s, A>` |
| QR codes and the QR copy grid | `faraday-organisms` | `qr_grid` | `QrGridModel<'s, A>` |
| the "Demo only" card | `faraday-organisms` | `demo_only` | `DemoOnlyModel<'s, A>` |
| Faraday Demo's host strip | `faraday-organisms-online` | `strip` | `StripModel<'s, A>` |
| the screen template | `faraday-templates` | `screen` | `FrameModel<'s, R>`; `draw` (04 §4) |
| the pane container | `faraday-templates` | `panes` | `Panes<'s, A>` |
| the step-flow template | `faraday-templates` | `flow` | `FlowModel<'s, A>` (04 §5) |
| the menu-page template | `faraday-templates` | `menu_page` | `MenuPageModel<'s, A>` |

Each module exports its model and `pub fn draw<A: Pressed>(m: &Model<'_, A>, ui: &mut Ui<'_, A>)`.
The sketches' `kit::screen`, `kit::flow` and `kit::snackbar` are `faraday_templates::screen`,
`faraday_templates::flow` and `faraday_organisms::snackbar`.

```rust
/// `content`: the content organism's `Block::Part` (04 §12.3).
pub enum Organism<'s, A> {
    Worksheet(WorksheetModel<'s, A>),
    Chart(ChartModel<'s, A>),
    WalletCard(WalletCardModel<'s, A>),
    Board(BoardModel<'s, A>),
    WaysIn(WaysInModel<'s, A>),
    QrGrid(QrGridModel<'s, A>),       // Krux's codes and their copy grid (inventory `krux-codes.md`)
    DemoOnly(DemoOnlyModel<'s, A>),   // Spend from backup's sample PSBT (`spend-backup.md`)
}

/// `screen`: the screen template's model. The navigation is one model at every size; the
/// template draws it as the rail, the bar or the sidebar by the room (04 V10).
pub struct FrameModel<'s, R> {
    pub nav: NavModel<'s, R>,          // destinations, Scan, power, the app's name, the lock tag
    pub top: TopBarModel<'s, R>,       // the title, the way back, the pills
    pub busy: Option<BusyModel<'s>>,   // the busy line
}
```

- A step's card is the content organism alone (04 §5's `card`), so every organism a
  step flow's card draws is a variant of `Organism`. A flow whose card needs another organism
  adds its variant here. Held by `ShowFlow::card`'s return type.
- **Decision: one navigation model, drawn as the rail or the bar by the screen template.**
  Reason: V10 keeps the model the same at every window size, and which form fits is the
  template's (04 §12.4). Turned down: a rail model and a bar model, which would make `present`
  choose by the size.

`reexports` (K7): `faraday-organisms` names `faraday_atoms::{Ui, Pressed, Shown, PlainRef,
Offered, Drawn, Style, Laid, Placement}`, `faraday_draw::icons::Icon` and each molecule model a
`Block` variant holds; `faraday-organisms-online` names the same atoms; `faraday-templates` names
the organisms' models its own models hold and the same atoms.

### 3.4 The kit's models

Every control, molecule and organism has one model struct, in its module, with the name and
generics below (atoms and molecules; §3.3 gives the organisms and templates).

- A kit model's name, module and generics are this section's; its fields are those
  of the `main` component its module ports (04 §12.1's places), written when it is ported. A
  field obeys V9 (no coordinate, size or rectangle), K12 (every text a `Shown<'s>`) and V11 (a
  press only as an `Offered<A>`, or as the press itself for a key, a pad's face or a word). So a
  model with `'s` holds text, one with `A` holds presses, and one with neither is a unit struct,
  drawn from the theme alone. Held by the compiler (the organisms and templates name these
  types) and by `lint-layers` (V9).

| Crate | Text and presses, `<'s, A>` | Text only, `<'s>` | Presses only, `<A>`, or neither |
|---|---|---|---|
| `faraday-atoms` | `button::ButtonModel` (04 §12.3), `check::CheckModel`, `chip::ChipModel`, `fab::FabModel`, `field::FieldModel`, `hold::HoldModel`, `icon_button::IconButtonModel`, `line::LineModel`, `pill::PillModel`, `segmented::SegmentedModel`, `stepper::StepperModel`, `switch::SwitchModel` | `badge::BadgeModel`, `progress::ProgressModel`, `tag::TagModel` | `frost::FrostModel`, `mark::MarkModel`, `rule::RuleModel`, `shadow::ShadowModel` |
| `faraday-molecules` | `appearance::AppearanceModel`, `buttons::ButtonsModel`, `chips::ChipsModel`, `destination::DestinationModel`, `guide_block::GuideBlockModel`, `kind_rows::KindRowsModel`, `multi::MultiModel`, `options::ChoiceModel`, `pass::PassModel`, `row::RowModel`, `tabs::TabsModel`, `time_zone::TimeZoneModel`, `words::WordGridModel` | `address::AddressModel`, `caution::CautionModel`, `code_box::CodeBoxModel`, `flow_text::FlowText`, `qr::QrModel`, `status::StatusModel`, `status::BusyModel`, `stick_pill::StickPillModel` | `pads::PadModel<A>` |

## 4. The application

All in `app/`, `layer = "app"`. **The feature base** is the set every feature and composite crate
depends on: `faraday-kernel`, `faraday-present`, `faraday-port`, `faraday-strings`,
`faraday-organisms`, `faraday-templates`; and as a dev edge, `faraday-app-test` (§8). The tables
give only what a crate holds beyond it.

### 4.1 The kernel and its derive

| Crate | Directory | Metadata | Normal edges | Dev edges |
|---|---|---|---|---|
| `faraday-kernel` | `app/kernel/faraday-kernel/` | `role = "kernel"` | port, text, crypto, bip, psbt, keys, vault, strings, kernel-derive | none |
| `faraday-kernel-derive` | `app/kernel/faraday-kernel-derive/` | `role = "kernel"`, proc-macro | none | none |

Its modules and items are 03 §1's table, with shapes in 03 §2 to §9, 04 §3, §6, §8.2, §13 and 05
§1.1, §7.3. The derive writes `faraday_kernel::__private::Sealed` (03 §3.2), so the kernel's
crate name is `faraday-kernel`. The manifest adds these items, which sections 03 and 06 name
without a shape:

```rust
// handle
/// What a scan asks for; the scan answers along the path the composites record from the
/// `Out::Scan` that opened it, as the camera's holder (06 §6.2).
#[derive(Public, Copy, PartialEq, Eq)]
pub enum ScanFor { Psbt, Key, Wallet, Address, Message, Text, File, Any }
/// A feature's ask to show a secret; the device composite routes it to devices, and back (06 §7).
#[derive(Public, Copy, PartialEq, Eq)] pub struct Gate(pub u32);
/// The tools a menu page opens (06 §3.3): one variant per tool-opening `catalog::Go` value of
/// `main` (inventory `features/tools.md`, `check-page.md`), named the same, each added with the
/// feature it opens.
#[derive(Public, Copy, PartialEq, Eq)] pub enum Tool { /* main's catalog::Go */ }
/// A published sample a device feature asks for (`Out::Sample`, §4.4): the online composite routes
/// it to the strip, which loads it and answers the feature that asked; the device app drops it.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum SampleAsk { Codex32, Spend, ToolText(Tool) }   // BIP 93's second vector; the sample wallet's spend; the open tool's text

// nav
pub struct Nav<P: Copy + Eq> {
    shown: P,
    back: Vec<P>,       // Back's pages, the latest last
    forward: Vec<P>,    // what Back left, for Forward
    begun: Vec<(P, P)>, // each flow begun and the page it was begun from (06 §4.3)
}
impl<P: Copy + Eq> Nav<P> {
    pub fn new(home: P) -> Self;
    pub fn open(&mut self, p: P) -> Option<P>;        // the page left, for Notice::Left
    pub fn back(&mut self) -> Option<P>;
    pub fn forward(&mut self) -> Option<P>;
    pub fn shown(&self) -> P;
    pub fn begun_from(&mut self, flow: P, from: P);
    pub fn drop_where(&mut self, f: fn(&P) -> bool);  // the Starter's Page::Demo entries
}
/// The frame sheets by kind (06 §4.4); the stack's order is `Ord`.
#[derive(Public, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FrameSheet { Lock, LockAsk, IdleWarn, NewInput, Devices, Power, Guide,
                      Restart, Uninstall, LetThrough, How }
/// Which child holds the camera, recorded by each composite from its children's outs.
#[derive(Public, Copy, PartialEq, Eq)] pub struct Holder(pub u8);
pub struct Frame<P: Copy + Eq> { nav: Nav<P>, sheets: Vec<FrameSheet>, camera: Option<Holder> }

// lock
impl CarryName {
    /// A feature's crate declares each of its names as a `const` (its `CARRY`, §4.4).
    pub const fn new(name: &'static str, first: bool, any_mode: bool) -> Self;
}
/// Where a held file lives: 05 §7.3's `put_file` and `take_files` take it.
pub enum FileBox { Inbox, Outbox }
```

### 4.2 The presentation kernel

`faraday-present`, `app/kernel/faraday-present/`, `role = "present"`. Normal edges: kernel, port,
strings, organisms, templates, pass-field (asked of section 01 below: the read view of the
passphrase part is mapped to `PassModel` here, 04 §12.3). Dev edges: app-test. No edge to the
atoms (01 K6): the atoms' items it names (`Pressed`, `Ui`, `Shown`, `Offered`, `Laid`, 04 §2) are
the organisms' re-exports (K7).

| Module | Exports |
|---|---|
| crate root | `Pressed`, the organisms' re-export of the kit's press bound and its derive, which a feature's `msg` names in place of a kit path (01 §5) |
| `stages` | `Present`, `Page` (04 §2) |
| `screen` | `ScreenModel`, `Drawn`, `draw` (04 §4) |
| `flow` | `ShowFlow`, `FlowPress`, the blanket `Present` and `Page` for every `ShowFlow`, `fill` (04 §5) |
| `help` | `offer`, `line`, `busy`, `pass_model`, `refused` (04 §2; `Refused::drawn` to the kit's `Drawn`) |

```rust
pub fn offer<P: Pressed, F: Feature>(state: &F::State, msg: &F::Msg, press: P, view: &View<'_>) -> Offered<P>;
pub fn line<'s>(l: &'s Line) -> Shown<'s>;
pub fn busy<'s>(running: &[Running], kind: JobKind, show: &Show<'s>) -> Option<BusyModel<'s>>;
pub fn pass_model<'s, A: Pressed>(v: faraday_pass_field::Read<'s>, to: fn(PassPress) -> A) -> PassModel<'s, A>;

// flow
/// One line of Your choices, worded by `ShowFlow::choice` (04 §5).
pub struct ChoiceLine(pub Line);
/// The blanket `Present` of every step flow: fills the step-flow template's model from
/// `StepFlow` and `ShowFlow`, field by field as 04 §5's table says.
pub fn fill<'s, F: ShowFlow>(flow: &'s Flow<F>, view: &View<'s>, show: &Show<'s>) -> FlowModel<'s, PressOf<F>>;
```

### 4.3 Parts

In `app/parts/`, `role = "part"`. Each exports `State` (fields private), `Msg`, `Press`,
`fn update(&mut State, Msg, &Ctx<'_>) -> Effects<Msg, Never>` (`Never` is the kernel's, 03 §3.1),
a `Read<'s>` view for its owner's `present`, and `AtLock` for `State`. Normal edges: kernel, port,
and what the row adds.

| Crate | Holds | Adds | Inventory |
|---|---|---|---|
| `faraday-pass-field` | the passphrase field and its eye, the one holder of a typed passphrase (A3) | text, keys | `secrets.md` (the field) |
| `faraday-text-field` | a text field, its caret and selection, invisible characters | text, forms | `typing.md` |
| `faraday-word-grid` | the on-screen word grid and the wordlist pick | text, bip | `add-key.md` (the grid) |
| `faraday-ways-in` | the ways in to a key (Scan, From Files, Type) and the key-form pick | text, forms, intake, text-field, word-grid | `add-key.md` (the ways in) |
| `faraday-key-making` | making a key: its entries, the words made, the quiz (03 §9.3) | bip, entropy, keys, word-grid, pass-field | `keygen-flow.md` (the making) |
| `faraday-scan-part` | the scanner sheet's state: the decoder's parts, the read card, classification | codec, intake, qr, bip, pgp | `scanner-qr.md` (the state) |
| `faraday-secrets` | the Secret out sheet, the text checks (`TextCheck`), file classification | bip, codec, intake, keys, pgp, qr, text | `secrets.md` |

The text check of `faraday-secrets` (Transfer's Send text, My seed's Compare, a wallet's name;
inventory `features/secrets.md`) answers with `main`'s `text_check::TextCheck`:

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TextCheck { Fine, TooLong, Secret(SecretKind), Inside(SecretKind), Partial, SeedHex }
/// The secret a text is or holds: the kinds of `main`'s `scan_read::Kind` that `Kind::secret`
/// names, and `Wallet` for a wallet file holding a secret.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SecretKind { SeedWords, SeedQr, CompactSeedQr, Codex32Secret, Codex32Share, Slip39Share,
                      BadChecksum, Aezeed, Wif, Xprv, UrSeed, Nostr, SpendKey, Paperkey, Totp,
                      Entries, Wallet }
```

This answers section 03's question: **the making of a key is the part `faraday-key-making`**,
embedded by New key and by Make a new wallet. Parts embed parts (asked of section 01 below).

### 4.4 The shape of a feature crate

```rust
// app/features/faraday-decode/src/lib.rs
mod state; mod msg; mod update; mod model; mod present; mod page;  // 01 §5, 04 §2.2
pub use state::State;                      // fields private (A7)
pub use msg::{Msg, Out, Press};            // Msg: From<Press>
pub use model::Model;
pub use update::Refusal;
pub struct Decode;                         // the feature: Feature + Present + faraday_present::Page
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Page { Decode, NonceCheck, SignerCheck }   // its screens (06 §4.2); absent with one screen
pub const JOBS: &[JobKind] = &[/* "decode.read", ... */];
pub const CARRY: &[CarryName] = &[];       // 05 §7.4
pub const ANCHORS: &[Anchor] = &[/* each page and sheet */];
```

A step-flow feature exports its `StepFlow` type in place of the unit struct (`create::MakeWallet`)
and its `Step`. A feature is a step flow when its one page is a step flow on `main` (inventory
`flow-container.md`): keygen, create, restore, spend, backup, backup-flow, spend-backup, bip85,
convert, krux, ots-check, core-import, silent, vanity and make-stick. The variants of its `Step`
are the cards of that flow on `main`, in order (03 §9.3 gives create's), and its `Answer` is a
`Public` value whose fields are the flow's answers, which come with the port.

A feature crate with several pages holds a step flow where `main`'s page is one: decode, messages,
seed-checks, lightning and calculators each hold, for those of their pages that are step flows on
`main`, a `Step` and an `Answer` of the same form, run by the one flow runner of 03 §9.

A feature's `Model<'s>` is the value `present` works out for one frame and borrows from the
feature's state. The manifest fixes its name and its lifetime; its fields come with the port.
A page that has one variant when the crate is written holds the variant of its first screen; the
others come with the port (06 §4.2). A feature's `Out` holds the variants its parent routes; those the routing of
sections 03 and 06 names are fixed here and the rest are the porting's:

| `Out` variant | Meaning | Routed by |
|---|---|---|
| `Open(Page)` | show one of its pages (06 §4.3) | each composite lifts it; the frame's `Nav` |
| `Leave(Ending)` | a flow ended | the composite; `Nav` returns to where it began |
| `Scan(ScanFor)`, `Camera(bool)` | ask for a scan; the camera on or off | the composites record the holder (06 §6.2) |
| `Reveal(Gate)` | a press that would show a secret | the device composite, to devices (06 §7) |
| `Front(Option<FrameSheet>)` | a frame sheet (power, devices, guide only) | the frame (06 §4.4) |
| `Go(Tool)` | open a tool (checks, tools only) | the device composite (06 §3.3) |
| `Sample(SampleAsk)` | load a published sample (§2.1) | the online composite, to the strip; dropped in the device app |
| `Top(Top)` | a lock, an end, a broadcast (power, owners only) | lifted to the top state (05 §1.1) |

Composite crates export the same shape with `Msg`, `Out`, `Press`, `Page`, `Body<'s>` and
`Over<'s>` enums of their children's, as 03 §6 and 06 §4.2 sketch.

A composite's and a product's `impl Drawn<Press>` for `Body` and `Over` are in a module `page`,
which holds nothing else, and `present` works out the models and names no `Ui` (01 §5).

An owner's `SheetModel<'s>` follows the rule for a feature's `Model<'s>` above: the name and the
lifetime are fixed here, the fields come with the page that draws them (04 §2, §12).

### 4.5 The shared owners

In `app/owners/`, `owner = true`. Their crates may call `Job::heavy` and `Job::seal` (A35).

| Crate | Role | Holds | Adds (normal) | Inventory |
|---|---|---|---|---|
| `faraday-session` | feature | the key store, wallets, what waits, Inbox and Outbox, the session's randomness, keys held for backup (03 §7.1); `impl session::Read` | keys, bip, crypto, entropy, psbt, intake, paper, vault, text | `wallets.md` (the session), `files-inbox.md` (Inbox, Outbox), `randomness.md` (the session's), `lock.md` (held for backup) |
| `faraday-settings` | feature | the person's choices, Set the time, the settings file (03 §7.2) | paper, text | `settings-time.md` |
| `faraday-vaults` | composite | the open vaults and the vault features; `impl vaults::Read` | keys, vault, and its six children | the vault area |
| `faraday-vault-list` | feature | the list and Create a vault | keys, vault, bip, codec, pass-field | `vault-list-create.md` |
| `faraday-vault-unlock` | feature | Unlock (`Job::heavy`) | keys, vault, pass-field | `vault-unlock.md` |
| `faraday-vault-contents` | feature | inside a vault, KeePass export | keys, vault, bip, codec, crypto, psbt, pass-field, secrets | `vault-contents.md` |
| `faraday-vault-seal` | feature | the seal at a lock (`Job::seal`) | keys, vault, crypto, plan | `vault-seal.md` |
| `faraday-totp` | feature | one-time codes | keys, vault, text | `totp-codes.md` |
| `faraday-gpg` | feature | GPG keys, paperkey restore | keys, pgp, vault | `gpg-keys.md` |

**Decision: the vault composite's six children are owner crates (`owner = true`, in
`app/owners/`).** Reason: Unlock's stretch and the seal are their jobs, and A35 lets only owner
crates call `Job::heavy` and `Job::seal`. Turned down: building those jobs in the composite, which
would put each use case's work back in one module. `lint-layers` refuses an edge into an `owner`
crate from any crate but another owner, a product and a tool.

Each owner exports, beside the feature shape: `impl shared::Owner`, its `Change` and `Done`
variants (they are the kernel's `session::Change`, `vaults::Change`, `settings::Change`, 03 §7.3),
and its `Press`. The vault composite and the settings owner re-export their `Page`, which is the
kernel's (`shared::vaults::Page`, `shared::settings::Page`, 06 §4.2); the session and the vault
composite's children have no page of the app's and no `Page`.

```rust
// faraday-kernel, shared
pub enum vaults::Page { Vaults, CreateVault, Unlock, Contents, Codes }   // the vault area's screens
pub enum settings::Page { Settings, SetTime }
```

### 4.6 The device composite and its children

`faraday-device`, `app/composites/faraday-device/`, `role = "composite"`. Children's crates in
`app/composites/` (the areas) and `app/features/` (the rest), each `role = "feature"` unless
marked. Every row adds the feature base.

| Crate | Child of | Inventory (its file) | Pages | Adds (normal) |
|---|---|---|---|---|
| `faraday-home` | device | `menu-pages.md` | Home | guide |
| `faraday-power` | device | `lock.md` | the Lock, LockAsk, Locked, Power, IdleWarn sheets; `FailedModel` | none |
| `faraday-devices` | device | `devices-inputs.md` | the Devices, NewInput sheets | none |
| `faraday-scan` | device; the Starter | `scanner-qr.md` | the Scan, Qr sheets | scan-part, codec, qr |
| `faraday-guide-sheet` | device | `guide.md` | the Guide sheet | guide |
| `faraday-keys-area` (composite) | device | none | none of its own | its nine children |
| `faraday-keygen` | keys | `keygen-flow.md` | KeyGen | key-making, pass-field, bip, crypto, entropy, keys |
| `faraday-randomness` | keys | `randomness.md` | Randomness | bip, entropy |
| `faraday-codex-shares` | keys | `codex-shares.md` | the worksheet in KeyGen and Entry | bip, codec, crypto, entropy, keys |
| `faraday-add-key` | keys | `add-key.md` | Entry | ways-in, word-grid, pass-field, bip, entropy, intake, keys, vault |
| `faraday-krux` | keys | `krux-codes.md` | KefTool | pass-field, codec, crypto, intake, keys, kef |
| `faraday-bip85` | keys | `child-keys.md` | Bip85 | bip, crypto, keys, vault |
| `faraday-convert` | keys | `convert.md` | Convert | bip, codec, crypto, keys, vault |
| `faraday-vanity` | keys | `vanity.md` | Vanity | bip, keys |
| `faraday-secure-boot` | keys | `secure-boot.md` | its flow in KeyGen and Backup | crypto, keys, sb, vault, guide |
| `faraday-wallets-area` (composite) | device | none | none of its own | its twelve children |
| `faraday-wallets` | wallets | `wallets.md` | WalletsStart, Wallets | bip, crypto, intake, keys, paper, psbt, vault |
| `faraday-create` | wallets | `create-wallet.md` | Create | key-making, pass-field, text-field, bip, codec, crypto, intake, keys, qr, vault |
| `faraday-restore` | wallets | `restore-wallet.md` | Restore | ways-in, pass-field, bip, crypto, keys, vault |
| `faraday-seeds` | wallets | `seeds.md` | the SeedRead and Sweep sheets | pass-field, bip, codec, crypto, intake, keys, psbt, vault |
| `faraday-public-files` | wallets | `public-files.md` | none: rows in Backup and Files | bip, crypto, intake, paper, qr |
| `faraday-core-import` | wallets | `core-wallet.md` | CoreWallet | pass-field, bip, crypto, keys, sqlite, core-wallet |
| `faraday-addresses` | wallets | `address-check-explore.md` | Explore, CheckAddress | bip, codec, intake, keys, text |
| `faraday-silent` | wallets | `silent-payments.md` | Silent | bip, crypto, keys, psbt, vault |
| `faraday-backup` | wallets | `wallet-backup.md`, `codex-backup.md`, `backup-board.md` (one state: the checklist, its codex32 item and its board) | Backup | secrets, bip, codec, crypto, forms, intake, keys, paper, plan, text, vault |
| `faraday-backup-flow` | wallets | `backup-flow.md` | BackupFlow | secrets, pass-field, bip, boot, codec, crypto, forms, keys, pgp, plan, text, vault |
| `faraday-backups-list` | wallets | `backups-list.md` | Backups | crypto, plan, vault |
| `faraday-spend-backup` | wallets | `spend-backup.md` | SpendBackup | bip, crypto, keys |
| `faraday-signing-area` (composite) | device | none | none of its own | its three children |
| `faraday-spend` | signing | `spend-signing.md` | Spend | bip, keys, psbt |
| `faraday-decode` | signing | `decode.md` | Decode, NonceCheck, SignerCheck | bip, crypto, keys, psbt, text |
| `faraday-messages` | signing | `messages.md` | Message, CheckMessage | crypto, keys, psbt |
| `faraday-checks-area` (composite) | device | `check-page.md` (the Check page is the composite's own) | Check | seed-checks |
| `faraday-seed-checks` | checks | `seed-checks.md` | SeedCheck, MultisigCheck | ways-in, pass-field, bip, crypto, intake, keys |
| `faraday-tools-area` (composite) | device | `tools.md` (the catalog is the composite's own) | Catalog | its three children |
| `faraday-calculators` | tools | the calculators of `tools.md` | Tools | ways-in, bip, codec, crypto, text |
| `faraday-lightning` | tools | Lightning keys and invoices of `tools.md` | Lightning, Invoice | pass-field, bip, crypto, keys |
| `faraday-ots-check` | tools | the OTS check of `tools.md` | OtsCheck | ots, crypto |
| `faraday-files-area` (composite) | device | `files-inbox.md` (the Files page and the file menu) | Files | its three children, public-files, bip, boot, codec, crypto, fat, intake, text |
| `faraday-boot-import` | files | `boot-import.md` | the list tile, the Import and TextSeed sheets | pass-field, intake |
| `faraday-stick-visit` | files | `stick-visit.md` | the visit in Files; Taken, NameTaken, WriteOut, Pull sheets | codec, crypto, fat, files, intake, text |
| `faraday-zeroing` | files | `delete-and-zero.md` | the Delete, Zeroing sheets | fat, text |

The Files page has no press of its own: `faraday-files-area`'s `FilesPress` is an enum without variants, and every press on its page is a child's (`Press::BootImport`, `StickVisit`, `Zeroing`).

**Decision: `seeds.md` and `public-files.md` are features of the wallets composite, as 06 places
them, reached by routed outs.** Bip85 asks for a seed by `Out` through the device composite to
the wallets composite and on to seeds; Files shows a wallet's public files by asking wallets the
same way. Reason: each holds one state (the seed read sheet's, the slow kinds' jobs) that two
embeddings would split. Turned down: making both parts, which would put a feature's sheets and
jobs in a crate that never routes.

`faraday-device` exports `State`, `Msg`, `Out`, `Press`, `Page` (06 §4.2), `Body<'s>`, `Over<'s>`,
`Device` (its feature type), and in its `frame` module what the device app and the online app
share: `FrameSheetModel<'s>` (the power, devices and guide sheets' models) and

```rust
pub fn frame_model<'s>(state: &'s State, frame: &'s Frame<Page>, view: &View<'s>, show: &Show<'s>)
    -> FrameModel<'s, Press>;
/// Every child that may hold a secret, for the leak crate's exhaustive match (07 TS18).
/// One variant per feature crate above, named as the crate without `faraday-`, in Pascal case.
#[derive(Public, Copy, PartialEq, Eq)] pub enum Child { /* one per feature crate above */ }
```

A composite's `present` module holds, besides its `Worded` and `Refused` impls for its `Refusal`,
two functions, which the composite's parent and the product call:

```rust
/// The model of the page the frame's navigation shows.
pub fn body<'s>(state: &'s State, page: Page, view: &View<'s>, show: &Show<'s>) -> Body<'s>;
/// The sheets over the page, each with its anchor, the lowest first.
pub fn over<'s>(state: &'s State, view: &View<'s>, show: &Show<'s>) -> Vec<(Anchor, Over<'s>)>;
```

A composite with a page of its own (`faraday-checks-area`, `faraday-tools-area`,
`faraday-files-area`) exports its own `Model<'s>` for it, which `Body` holds in the variant of that
page; its fields come with the port. A composite with no sheet of its own has an `Over` that
cannot be made: `enum Over<'s> { #[doc(hidden)] None(Never, PhantomData<&'s ()>) }` (`faraday-checks-area`,
`faraday-signing-area`, `faraday-tools-area`).

Its normal edges: the feature base, its eleven direct children's crates. Dev: app-test.

### 4.7 The online composite, the Starter's composite and their features

| Crate | Directory | Role, metadata | Inventory | Adds (normal) |
|---|---|---|---|---|
| `faraday-online` | `app/composites/` | composite, `online` | none of its own | device, strip, transfer, organisms-online |
| `faraday-strip` | `app/features/` | feature, `online` | `practice-demo.md` | samples, organisms-online, intake, keys |
| `faraday-transfer` | `app/features/` | feature, `online` | `transfer.md` | text-field, codec, crypto, intake, qr, text |
| `faraday-starter` | `app/composites/` | composite, `online` | `starter-app.md` | its seven children, transfer, scan |
| `faraday-starter-home` | `app/features/` | feature, `online` | the Starter's Home of `starter-app.md` | guide |
| `faraday-install` | `app/features/` | feature, `online` | install of `starter-app.md` | none |
| `faraday-starter-appearance` | `app/features/` | feature, `online` | Appearance and About of `starter-app.md` | none |
| `faraday-print-templates` | `app/features/` | feature, `online` | templates of `starter-app.md` | paper |
| `faraday-starter-guide` | `app/features/` | feature, `online` | the Starter's guide of `starter-app.md` | guide |
| `faraday-make-stick` | `app/features/` | feature, `online` | `starter-make-stick.md` | boot, crypto, fat, files, payload, sb |
| `faraday-starter-start` | `app/features/` | feature, `online` | `starter-start.md` | codec, crypto, guide |

`faraday-online` exports `State`, `Msg`, `Out`, `Press`, `Page` (06 §4.2), `Body<'s>`, `Over<'s>`,
`FrameSheetModel` (the device composite's, re-exported), and the online app. An owner's page is
the product's to present (06 §4.2), so `Body` has no owner variant and the composite names no
owner and no `Tops`:

```rust
pub struct OnlineApp { online: State, frame: Frame<Page> }   // 06 §4.1
pub struct Online;                                           // Feature<State = OnlineApp, Out = Top>
pub enum Page { Device(device::Page), Transfer, Vaults(vaults::Page), Settings(settings::Page),
                SelfTestFailed }                             // the owners' pages: the kernel's types
pub enum Body<'s> { Device(device::Body<'s>), Transfer(transfer::Model<'s>),
                    SelfTestFailed(device::FailedModel<'s>) }
pub enum Over<'s> { Device(device::Over<'s>), Strip(strip::Model<'s>), Frame(FrameSheetModel<'s>) }
#[derive(Pressed)]
pub enum Press { Device(device::Press), Strip(strip::Press), Transfer(transfer::Press), Open(Page), Back }

pub fn shown(app: &OnlineApp) -> Page;
pub fn frame_model<'s, R: Pressed>(app: &'s OnlineApp, view: &View<'s>, show: &Show<'s>,
                                   lift: fn(Press) -> R) -> FrameModel<'s, R>;
pub fn body<'s>(app: &'s OnlineApp, view: &View<'s>, show: &Show<'s>) -> Option<Body<'s>>; // None: an owner's
pub fn over<'s>(app: &'s OnlineApp, view: &View<'s>, show: &Show<'s>) -> Vec<(Anchor, Over<'s>)>;
```

`faraday-starter` exports `State`, `Msg`, `Out`, `Press`, `Page` (06 §4.2), `Body<'s>`, `Over<'s>`.

### 4.8 The product crates

In `app/products/`, `role = "product"`. Dev edges: app-test, fixture-adapters (routing tests). A product
reaches the kit through `templates` and `organisms` only (01 §5); `TextSize` is `templates`'s
re-export. Its `present` module names no `Ui`: the `impl Drawn<Press>` of `Body` and `Over` (the
Starter's also of `DemoBody` and `DemoOver`) are in `page` (§4.4). Each product has a `MODES`
constant (S9) and an `l1` module with one `assert_not_clone!` per top state.

| Crate | Metadata | Normal edges |
|---|---|---|
| `faraday-device-product` | | runtime, device, session, settings, vaults, bip, the feature base |
| `faraday-demo-product` | `online` | runtime, online, session, settings, vaults, bip, the feature base |
| `faraday-starter-product` | `online` | runtime, online, starter, session, settings, vaults, bip, text, the feature base |

```rust
// faraday-device-product
pub struct DeviceApp { device: faraday_device::State, frame: Frame<Page> }  // 06 §4.1
pub enum Page { Device(faraday_device::Page), Vaults(faraday_vaults::Page),
                Settings(faraday_settings::Page), SelfTestFailed }
pub enum Body<'s> { /* 06 §4.2 */ }  pub enum Over<'s> { /* 06 §4.2 */ }
#[derive(Pressed)] pub enum Press { /* 06 §4.2 */ }
pub struct App;                                // Feature<State = DeviceApp, Out = Top>
pub struct Device;                             // the product value: impl faraday_runtime::Root
impl Root for Device {
    type Settings = faraday_settings::Settings; type Session = faraday_session::Session;
    type Vaults = faraday_vaults::Vaults;     type App = App; type Outside = Nothing;
    type Press = Press;                        type Options = NoOptions;
    /* 05 §1's functions */
}
pub const MODES: &[Mode];                      // S9: Mainnet, Testnet, Signet, Regtest
assert_not_clone!(DeviceApp);                  // L1, for each top state: a macro (a function cannot refuse `Clone`
                                               // on stable), by ambiguity of two blanket impls
#[derive(Public)] pub struct NoOptions;        // the device app takes no options (06 §2); `()` is not `Public`

// faraday-demo-product
pub struct Demo { options: DemoOptions }       // impl Root; App = faraday_online::Online
pub const MODES: &[Mode];                      // S9: Practice (Testnet, the default), Testnet, Signet, Regtest
#[derive(Public, Clone, Debug)] pub struct DemoOptions { pub text: Option<TextSize>, pub transfer: bool }

// faraday-starter-product
pub struct StarterApp { starter: faraday_starter::State, frame: Frame<faraday_starter::Page> }
pub struct Outside;                            // Feature<State = StarterApp, Out = Top>
pub struct Starter { options: StarterOptions } // impl Root; App = Online, Outside = Outside
pub const MODES: &[Mode];                      // as the Demo's
#[derive(Public, Clone, Debug)] pub struct StarterOptions { pub uninstall: bool, pub image: Option<Text> }
```

## 5. The runtime

### 5.1 The crate

`faraday-runtime`, `runtime/faraday-runtime/`, `layer = "runtime"`. Normal edges: kernel, port,
atoms, draw, crypto, selftest, strings. Dev edges: app-test, alloc-count, leak-scan.

| Module | Holds | Exports |
|---|---|---|
| `root` | the product's trait and what it is lent | `Root`, `Tops`, `Addr`, `Stream`, `Nothing`, `OwnerState<F>` (05 §1) |
| `runtime` | the top states and `Runtime::new` | `Runtime`, `Phase`, `Workers` (05 §1) |
| `queue` | one event at a time, the cascade (05 §2) | none |
| `dispatch` | orders, asks and replies, changes served (05 §3) | none |
| `facts` | `Facts` kept from port facts, `View` and `Show` built (05 §4) | none |
| `pool` | the two lanes (05 §5) | none |
| `memo` | the runtime's `Memo` and its frame marks (05 §6) | none |
| `lock` | the lock order, the carry in hand, the scope's build (05 §7) | none |
| `frame` | the wiping frame buffer, painting the call list, the last `FrameOut` (05 §2.5) | none |
| `log` | the ring of 512 lines (05 §8) | `LogLine` |
| `seen` | what a check reads (05 §9) | `Seen`, `Mark`, `KeptImage` |

```rust
impl<R: Root> Runtime<R> {
    // 05 §1: new, event, woken, frame, phase; 05 §9: seen, record_ink
    pub fn log(&mut self, on: bool);
    pub fn log_lines(&self) -> impl Iterator<Item = &LogLine> + '_;
}
pub type OwnerState<F> = <F as Feature>::State;
```

### 5.2 The two mint sites

This answers section 05's question. `costs-lint` lists exactly these, by path and function:

- `runtime/faraday-runtime/src/runtime.rs`, in `Runtime::new`: the root thread's token.
- `runtime/faraday-runtime/src/pool.rs`, in `fn lane(...)`, the lane thread's entry: one per lane.

## 6. The adapters

All in `adapters/`, `layer = "adapter"`, each a crate of its own, so its tests carry Owner
`adapter` (07 §4, check 3). This answers section 07's question. Each library adapter exports one
type that implements `port::Adapters` for its families, a builder from its options, and nothing
of the app's.

| Crate | Metadata | Holds | Normal edges | Dev edges | From `main` |
|---|---|---|---|---|---|
| `faraday-storage` | | `Ask::Storage`, `Keep`, `Forget`, `Memory`, `Host`, `Print`; the disk-process client (02 §8) | port, files, crypto, qr, strings, text | codec, vault, leak-scan | `faraday-storage`, ~~core~~ |
| `faraday-disk` | `process` | the disk process | boot, crypto, fat, files, strings, text | none | as it is, but its freed-memory test `tests/leak.rs`, not carried (07 §6.1, F336) |
| `faraday-grant` | `process` | the grant helper | boot | none | as it is |
| `faraday-stick-adapter` | | framebuffer, evdev and touch, held-back input, the arrow, turns, sticks attached, the clean marker, the read-only root, the dev reports, the capture thread (`ReadMode::Reduced`) | port, storage, camera, v4l2, scanner, crypto, boot, paper, text, tokens | leak-scan | `shells/stick/src/` but `main.rs`'s loop |
| `faraday-desktop-adapter` | `online` | sticks as folders (the folder `Place` from `faraday-files`), `ToggleStick`, Downloads, opening, the print folder, the camera list | port, storage, files, fat, crypto, strings, text | codec (the folder `Place`'s moved test builds a vault file's head) | `shells/demo`'s adapter part, `faraday-storage`'s `Dirs` |
| `faraday-window` | `online` | the window, mouse, touch, keys, clipboard, `MovesWindow`, OS randomness, the lens (`ReadMode::Full`) | port, text, camera, scanner, tokens, v4l2 (Linux), mediafoundation (Windows) | none | `shells/window`, ~~core~~, ~~host~~ |
| `faraday-os-adapter` | `online` | `SystemAsk`, `SystemFact`, `settings.txt`, the colour scheme, writing a stick (02 §4.10) | port, boot, crypto, fat, payload, sb, text; build: payload | sb | `shells/starter/src/` but `main.rs` |
| `faraday-camera` | | frames, NV12, scaling, the list | none | none | `shells/camera` |
| `faraday-v4l2` | | V4L2 capture | camera | none | `shells/v4l2` |
| `faraday-mediafoundation` | `online` | Media Foundation capture | camera | none | `shells/mediafoundation` |
| `faraday-scanner` | | reading codes from frames on its own thread | tokens | codec, crypto | `faraday-scanner`, ~~draw~~ |

```rust
// faraday-stick-adapter
pub struct StickOptions { pub fb: Text, pub size: Option<(u16, u16)>, pub depth: u8, pub dpi: u16,
    pub native: Option<Orientation>,          // None: detect
    pub mount: Text, pub pointer_speed: u8,
    pub touch_grid: Option<(u16, u16)>,       // None: the default grid
    pub touch_name: Option<Text>,
    pub calibration: Calibration,             // { swap, invert_x, invert_y }
    pub files: Text, pub verbose: bool, pub timings: bool, pub boot_report: bool,
    pub input: Vec<Text>,                     // files of recorded input: PATH or KIND:PATH
    pub frames: Option<u64>, pub camera: Option<Text>, pub no_camera: bool }
pub struct Stick { /* every device adapter but storage */ }
impl Stick { pub fn open(o: StickOptions, waker: Arc<dyn Waker>) -> Result<Self, Failure>;
             pub fn present(&mut self, f: &port::Frame<'_>);   // 02 §1, §5
             pub fn ended(&self) -> Option<u8>; }              // 06 §2.1: 0, 75, 76
impl Adapters for Stick { /* 02 §1 */ }
/// The device's adapters behind one value: the stick's and the storage adapter's.
pub struct DeviceAdapters { /* the stick, the storage adapter */ }
impl DeviceAdapters { pub fn new(stick: Stick, storage: faraday_storage::Storage, waker: Arc<dyn Waker>) -> Self;
                      pub fn stick(&mut self) -> &mut Stick; }   // 06 §5.1
impl Adapters for DeviceAdapters { /* each effect to the adapter of its family */ }

// faraday-storage
pub struct Storage { /* */ }
impl Storage {
    pub fn disk_process(fifo: Text, ram_dir: Text) -> Result<Self, Failure>;   // the device
    pub fn folders(sticks: Box<dyn Sticks>) -> Self;                           // the desktops
}
/// Where the sticks' files are: main's `Sticks`, one method for each storage ask over the port's
/// own ask types, and the stick listing.
pub trait Sticks: Send {
    fn list(&mut self) -> Vec<storage::StickInfo>;                       // every stick present now
    fn read(&mut self, ask: storage::Read) -> Result<Bytes, Failure>;
    fn read_codes(&mut self, ask: storage::ReadCodes) -> Result<Codes, Failure>;
    fn write(&mut self, ask: storage::Write) -> Result<Written, Failure>;
    fn wipe(&mut self, ask: storage::Wipe) -> Result<Wiped, Failure>;
    fn space(&mut self, ask: storage::Space) -> Result<Room, Failure>;
    fn zero_free(&mut self, ask: storage::ZeroFree) -> Result<Zeroing, Failure>;
    fn make_empty(&mut self, ask: storage::MakeEmpty) -> Result<(), Failure>;
    fn rename(&mut self, ask: storage::Rename) -> Result<(), Unrenamed>;
    fn name(&mut self, ask: storage::Name) -> Result<(), NameRefused>;
    fn hash(&mut self, ask: storage::Hash) -> Result<Sha256, Failure>;
    fn has(&self, stick: &StickId) -> bool;                              // the stick a folder stands for
}

// faraday-desktop-adapter
pub struct Desktop { /* folder sticks, host */ }
impl Desktop { pub fn new(sticks: Text, downloads: Option<Text>, waker: Arc<dyn Waker>) -> Self; }
pub struct FolderPlace { /* impl faraday_files::Place over std::fs */ }

// faraday-camera: no workspace edge, so a frame's planes are vectors and the frame wipes them on
// drop itself (`zeroize`'s ZeroizeOnDrop; 02 §6); not Clone
pub struct Frame { pub width: u16, pub height: u16, pub luma: Vec<u8>, pub chroma: Option<Vec<u8>> }

// faraday-window
/// The window's command line, as the adapter takes it (06 §5.2).
pub struct WindowOptions { pub size: Option<(u16, u16)>, pub panel: Option<Text>, pub aspect: Option<Text>,
    pub ppi: Option<u16>, pub timings: bool, pub no_camera: bool }
pub struct Window { /* */ }
impl Window { pub fn open(o: WindowOptions, waker: Arc<dyn Waker>) -> Result<Self, Failure>;
              pub fn paint(&mut self, f: &port::Frame<'_>); }
// faraday-os-adapter
pub struct System { /* */ }
impl System { pub fn new(waker: Arc<dyn Waker>) -> Self; }
```

Each adapter implements `port::Adapters` for what it carries and answers `Unsupported` for the
rest (02 §10); a root joins several behind one value as `DeviceAdapters` does (`DemoAdapters`,
`StarterAdapters` in the roots).

## 7. The roots

All in `roots/`, `layer = "root"`. Each is a binary with one `main.rs` and an `options.rs`
(the command line), and its journey tests in `tests/` (Owner `root`, 07 §2.5).

| Crate | Metadata | Normal edges | Dev edges | From `main` |
|---|---|---|---|---|
| `faraday-stick` | `offline` | device-product, runtime, port, stick-adapter, storage | fixture-adapters, testkit | `shells/stick`'s `main.rs` |
| `faraday-demo` | `online` | demo-product, runtime, port, window, desktop-adapter, storage | fixture-adapters, testkit | `shells/demo` |
| `faraday-starter-shell` | `online` | starter-product, runtime, port, window, desktop-adapter, storage, os-adapter | fixture-adapters, testkit | `shells/starter`'s `main.rs` |
| `faraday-snapshot` | `ships = false` | device-product, demo-product, starter-product, runtime, port, fixture-adapters, testkit | none | `shells/snapshot`, `faraday-starter`'s `snap` example |

The package names are `main`'s, so the image build, the release staging and the installer keep
their binaries' names. The tour root's modules: `main`, `tours` (one module per tour of
`tours.md`), `runner` (the driver and its command line), `screens` (the secret-leak check, 07
§5.3), `targets` (the touch-target walk, 07 §5.2), `png` (the picture written at each step, with
no metadata). It depends on a tool crate as a normal edge (asked of section 01 below).

Each root's `Options` holds its product's options and its adapters' options, and the paths that
are not flags (06 §5) as strings: the stick's `Options` has `stick: StickOptions` and the disk
process's `fifo` and `ram_dir`; the Demo's and the Starter's have `product`, `window:
WindowOptions`, `sticks` and `downloads: Option<String>`. The snapshot runner's `Product` is
`Device`, `Demo` or `Starter`, and its `Command` is the product, the width, the height, the dpi,
the tour (the default one when none) and where the pictures go; `parse() -> Command` reads it and
`run(&Command) -> ExitCode` drives it.

```rust
// each root's main.rs
fn main() -> ExitCode {
    let options = options::parse();                       // S6: no event made here
    let (adapters, waker) = /* the adapters of 06 §5 */;
    let mut rt = Runtime::new(/* the product value from options */, waker, Workers::Threads);
    run(&mut rt, adapters)                                // 02 §1's loop
}
/// The loop, shared in form by each root and by a journey (06 §8).
fn run<R: Root, A: Adapters>(rt: &mut Runtime<R>, adapters: A) -> ExitCode;
```

## 8. Tools and test support

All in `tools/`, `layer = "tool"`; nothing a root ships depends on one (T1, `lint-keys-tests`).

| Crate | Directory | Metadata | Normal edges | Purpose |
|---|---|---|---|---|
| `faraday-app-test` | `tools/faraday-app-test/` | `checks = "app"` | kernel, port, session, settings, vaults, testkit, crypto, keys, bip, psbt | the application's test support (07 §2.2): `TestView`, `test_compute`, `Seen`, `seen`, `answer`, `done` |
| `faraday-fixture-adapters` | `tools/faraday-fixture-adapters/` | `checks = "root"` | port, testkit, crypto, text, files, fat | the tour root's, the journeys' and the leak crate's adapters: fixture sticks, fixed randomness, scripted clock and input, recorded print |
| `faraday-leaks` | `tools/faraday-leaks/` | `checks = "app"` | none | the leak flows (07 §6.2); dev: runtime, device-product, demo-product, starter-product, device, session, vaults, settings, fixture-adapters, leak-scan, keys |
| `faraday-testkit` | `tools/faraday-testkit/` | `checks = "domain"` | bip, crypto, psbt, codec, qr, files, fat, vault, and the other domain crates its test data names | the test data (section 10 §6): the test wallets, seeds and words, the spends, the MuSig2 rounds, the test vaults, the test EFI image, over domain types; its edge to `faraday-core` cut, what it took from it rewritten over the domain; no application edge |
| `faraday-testkit-sticks` | `tools/testkit-sticks/` | | testkit, backup, and the domain crates the sticks need | writes the test kit's sticks; the backup stick holds what `faraday-backup` writes (section 10 §6) |
| `faraday-alloc-count` | `tools/faraday-alloc-count/` | | none | the counting allocator |
| `faraday-leak-scan` | `tools/faraday-leak-scan/` | `checks = "app"` | none | the leak allocator |
| `faraday-fontbake` | `tools/fontbake/` | | bip, crypto, draw | bakes the fonts into `faraday-draw` |
| `costs-lint` | `tools/costs-lint/` | | none | who mints and takes `Compute` (§5.2) |
| `op-cost` | `tools/op-cost/` | | bip, codec, crypto, entropy, files, intake, psbt | named costly operations' counts (F238); ~~core~~ |
| `faraday-crosscheck` | `tools/crosscheck/` | | the domain crates of `main`'s list but `faraday-core` (TS20); draw, for the picture requests alone | the cross-check server; `file_menu::sha256`'s use is its `hash` area's, now `faraday_crypto::sha256_whole` (section 10 §6) |

The counting allocator `Counting` (`allocations() -> u64`, `reset()`) and the leak allocator `Leak`
(`watch(form: &[u8])`; `arm()` and `disarm()`, each on the calling thread, 07 §6.2;
`live() -> bool`, a form in a live block of the armed threads;
`drop_checked<T>(value: T) -> bool`, a form in a block that dropping `value` freed, 07 §6.1) are
global allocators, each an `unsafe impl GlobalAlloc` as U2 allows. `faraday-testkit` has `TestSeed { name, words }` (all `&'static str`) and
`test_seeds() -> [TestSeed; 3]`, the three pairs of a name and its words (a function, since a
constant of the real words is a body's); its spends and MuSig2 rounds are serialized PSBTs, as
`Vec<u8>`, until `faraday-psbt` has a `Psbt` type.

```rust
// faraday-fixture-adapters
pub struct Fixtures { /* sticks from out/testkit, randomness, clock, input script */ }
impl Fixtures {
    pub fn device(kit: &Path) -> Self; pub fn demo(kit: &Path) -> Self; pub fn starter(kit: &Path) -> Self;
    pub fn input(&mut self, script: &[Scripted]);
    pub fn advance(&mut self, ms: u64);
    pub fn written(&self) -> &[(StickId, Text, Sha256)];    // what a journey asserts
}
impl Adapters for Fixtures { /* 02 §10's tour-root column */ }
pub enum Scripted { Touch(Point), Key(Key), Text(Text), Stick(StickId, bool), Wait(u64) }
```

`faraday-leaks` has no library code; its tests are `tests/leaks/flows.rs` (the exhaustive match
over `faraday_device::Child`, TS18) and `tests/leaks/reveals.rs` (TS19: the path the impact rule
`keys.new-reveal` names, `tools/faraday-leaks/tests/leaks/reveals.rs`), each its own binary with
the leak allocator.

**The copied-tests list** (07 §4.1) is `tools/test-headers.copied`. Its first list is every Rust
file of the copied crates that holds a test, at its path on `next`, made by
`git grep -l -E '#\[(test|proptest)|proptest!' -- <dirs> ':(glob)**/*.rs'` over the directories
of the domain crates (§2.1), `faraday-tokens`, `faraday-draw`, `faraday-atoms`,
`faraday-molecules`, `faraday-organisms` (the tests copied from `faraday-draw` and `faraday-ui`),
`faraday-disk`, `faraday-grant`, `faraday-camera`, `faraday-v4l2` and `faraday-mediafoundation`,
in the change that copies them. Measured on `main` at `d89dde5de` with the same command: 259
files (`faraday-bip` 47, `faraday-psbt` 24, `faraday-codec` 21, `faraday-vault` 21,
`faraday-samples` 21, `faraday-keys` 14, `faraday-entropy` 12, `faraday-files` 11,
`faraday-draw` and `faraday-ui` 13, the other 24 crates 75). The storage adapter, the scanner and
every ported crate are not on it: their tests are written with headers.

## 9. The wording, the guide and the tours

This answers section 08's questions.

- **Strings:** `main`'s `faraday/faraday-strings/src/en/` is `core/faraday-strings/src/en/` on
  `next`. `lint-strings`, `lint-guide`, `tools/impact.toml` and `CLAUDE.md` name that path.
- **The guide:** `core/faraday-guide/src/guide.rs` reads each page as
  `include_str!("../../../docs/guide/<page>.md")`, the same relative path as on `main`.
- **The tours' recipes:** `just tour SIZE [TOUR]` renders one tour of the device app at one size
  into `out/tours/device/WxH/[TOUR/]`; `just product=demo tour ...` and `just product=starter tour
  ...` take the other products, into `out/tours/demo/` and `out/tours/starter/`; `just tours`
  renders every tour of every product at each size, for a change to the kit's templates. The
  touch-target walk stays `just lint-touch-targets`. `CLAUDE.md`'s example is `just tour 480x640
  spend`, and its one-module test run is `cargo nextest run -p faraday-decode -E
  'test(/^update::/)'`.
- **The top level:** §1.1 is the list `README.md`'s "Repository layout" and `lint-tree`'s `TOP`
  carry.

## 10. Every inventory file in one crate

The crate that holds each of the 82 files of `docs/migration/inventory/features/`. Where a file's
rules reach other crates, the last column names them; the crate in the second column holds the
file's screens and its update tests.

| Slug | Crate | Also in |
|---|---|---|
| `add-key` | `faraday-add-key` | `faraday-ways-in`, `faraday-word-grid` |
| `address-check-explore` | `faraday-addresses` | |
| `app-shell` | `faraday-runtime` | the product crates (the gates, 06 §7) |
| `backup-board` | `faraday-backup` | `faraday-organisms` (`board`) |
| `backup-flow` | `faraday-backup-flow` | |
| `backups-list` | `faraday-backups-list` | |
| `boot-import` | `faraday-boot-import` | |
| `check-page` | `faraday-checks-area` | |
| `child-keys` | `faraday-bip85` | |
| `codex-backup` | `faraday-backup` | |
| `codex-shares` | `faraday-codex-shares` | `faraday-organisms` (`worksheet`) |
| `convert` | `faraday-convert` | |
| `core-wallet` | `faraday-core-import` | |
| `create-wallet` | `faraday-create` | |
| `decode` | `faraday-decode` | |
| `delete-and-zero` | `faraday-zeroing` | |
| `desktop-shells` | `faraday-window` | `faraday-desktop-adapter`, the capture adapters |
| `dev-diagnostics` | `faraday-stick-adapter` | `faraday-selftest`, the frame's `SelfTestFailed` page |
| `devices-inputs` | `faraday-devices` | `faraday-stick-adapter` (held-back input) |
| `draw-base` | `faraday-draw` | |
| `drawing-compute` | `faraday-runtime` | `costs-lint`, `faraday-present` |
| `files-inbox` | `faraday-files-area` | `faraday-session` (Inbox, Outbox) |
| `flow-container` | `faraday-templates` | `faraday-kernel` (`flow`), `faraday-present` (`flow`) |
| `gpg-keys` | `faraday-gpg` | |
| `guide` | `faraday-guide-sheet` | `faraday-guide`, `faraday-organisms` (`guide`) |
| `keyboard` | `faraday-organisms` | |
| `keygen-flow` | `faraday-keygen` | `faraday-key-making` |
| `krux-codes` | `faraday-krux` | |
| `layout-rules` | `faraday-templates` | the other kit crates |
| `lock` | `faraday-power` | `faraday-runtime` (`lock`), `faraday-session` (held for backup) |
| `menu-pages` | `faraday-home` | `faraday-templates` (`menu_page`) |
| `messages` | `faraday-messages` | |
| `motion` | `faraday-atoms` | `faraday-runtime` (the motion store's owner) |
| `pane-container` | `faraday-templates` | |
| `pointer-touch` | `faraday-stick-adapter` | `faraday-atoms` (focus), `faraday-organisms` (tooltips) |
| `practice-demo` | `faraday-strip` | each feature's `refuses`; `faraday-keys` (A33) |
| `public-files` | `faraday-public-files` | |
| `randomness` | `faraday-randomness` | `faraday-session` (the session's randomness) |
| `restore-wallet` | `faraday-restore` | |
| `scanner-qr` | `faraday-scan` | `faraday-scan-part`, `faraday-scanner`, `faraday-organisms` (`viewfinder`, `qr_grid`) |
| `screen-frame` | `faraday-templates` | the product crates' frames |
| `secrets` | `faraday-secrets` | `faraday-pass-field` |
| `secure-boot` | `faraday-secure-boot` | |
| `seed-checks` | `faraday-seed-checks` | |
| `seeds` | `faraday-seeds` | |
| `session-pane` | `faraday-organisms` | the frames, from `View::session` |
| `settings-time` | `faraday-settings` | |
| `sheets-parts` | `faraday-organisms` | |
| `silent-payments` | `faraday-silent` | |
| `small-panel` | `faraday-templates` | |
| `spend-backup` | `faraday-spend-backup` | |
| `spend-signing` | `faraday-spend` | |
| `starter-app` | `faraday-starter` | its features, `faraday-starter-product` |
| `starter-make-stick` | `faraday-make-stick` | `faraday-os-adapter` |
| `starter-start` | `faraday-starter-start` | |
| `stick-shell` | `faraday-stick` | `faraday-stick-adapter` |
| `stick-visit` | `faraday-stick-visit` | |
| `storage` | `faraday-storage` | `faraday-disk`, `faraday-grant` |
| `test-kit` | `faraday-testkit` | `faraday-app-test`, `faraday-leaks`, the allocators |
| `tokens` | `faraday-tokens` | |
| `tooling` | `costs-lint` | the scripts of `tools/`, `build/` |
| `tools` | `faraday-tools-area` | `faraday-calculators`, `faraday-lightning`, `faraday-ots-check` |
| `totp-codes` | `faraday-totp` | |
| `tour-runner` | `faraday-snapshot` | `faraday-fixture-adapters` |
| `tours` | `faraday-snapshot` | |
| `transfer` | `faraday-transfer` | |
| `typing` | `faraday-text-field` | |
| `ui-controls` | `faraday-atoms` | `faraday-molecules` (the appearance rows) |
| `ui-guide-text` | `faraday-molecules` | `faraday-organisms` (`guide`, `demo_only`) |
| `ui-navigation` | `faraday-organisms` | `faraday-molecules` (`tabs`) |
| `ui-press` | `faraday-atoms` | |
| `ui-regions` | `faraday-atoms` | |
| `ui-sheets` | `faraday-organisms` | |
| `ui-text` | `faraday-atoms` | `faraday-molecules` (list items) |
| `ui-theme` | `faraday-tokens` | |
| `vanity` | `faraday-vanity` | |
| `vault-contents` | `faraday-vault-contents` | |
| `vault-list-create` | `faraday-vault-list` | |
| `vault-seal` | `faraday-vault-seal` | |
| `vault-unlock` | `faraday-vault-unlock` | |
| `wallet-backup` | `faraday-backup` | |
| `wallets` | `faraday-wallets` | `faraday-session` (what the session holds) |

A script over the file list and this table's first column checks that each of the 82 files has
exactly one row, that no row names a file that does not exist, and that every crate in the second
column is a crate of this manifest; it passes on this text.

## 11. What holds the manifest

- The skeleton is this manifest compiled: every crate, module and export above, every body
  `todo!()` (the plan's §4, step 3). `lint-layers` reads each crate's metadata and checks its
  edges against section 01 and the tables here; a crate added later carries its row's metadata
  and the change adds its row to this section. Held by `lint-layers` and `lint-docs`.
- A path this section names (§1.1, §5.2, §8, §9) is read by the lint that names it
  (`lint-tree`, `costs-lint`, `lint-leak-reveals`, `lint-test-headers`, `lint-strings`). Held by
  those lints.

## Questions for later sections

- Section 10: the one file of `faraday-vault` that names a clock or I/O path (§2.2), and which of
  `faraday-samples`' copied tests are dropped with their `faraday-core` edge (§2.1).
- Section 10: where each of `faraday-testkit`'s uses of `faraday-core` (`wallet::Origin`,
  `picture::labelled_png`, `testkit::backup_files`, `file_menu::sha`) now comes from (§8).

What this section leaves to be written: each feature's own `Msg` variants beyond the routed `Out`s
of §4.4, each kit model's fields, and the `Tool` enum's variants, which are the porting's from the
inventory files named in §4.6 and §10.
