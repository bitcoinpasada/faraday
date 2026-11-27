# The composition roots: the device app, Faraday Demo and the Starter

**Kind:** Spec
**Open:** Not built.

This section fixes the three products and the tour root: for each, the root binary, the product
crate that holds its model, the features and composites it holds (every inventory feature is
placed in §3), its adapters, its pages and navigation, how each port fact that is not in `Facts`
becomes a feature's message, and what it carries that the others do not. Faraday Demo and the
Starter are their own composition roots, and the Starter does not depend on Faraday Demo: the
Starter's model holds a Faraday Demo of its own as its lock scope, beside the Starter's own
composite. The section ends with what a root may and may not do, each rule with what holds it. It
answers the questions sections 02 and 03 left for it and three of section 07's. It is written for
whoever writes a root, a product crate or the device composite, and for whoever writes the
runtime or the manifest. Rules continue section 01's numbering for adapters and roots: S5 onward.

## 1. A product is a root binary and a product crate

**Decision: each product's model is a library, its product crate, which implements the runtime's
`Root` (section 05, §1); its root is a binary that holds only the command line, the adapters and
the loop of section 02, §1.**

| Product | Root binary (`layer = "root"`) | Product crate (`layer = "app"`, `role = "product"`) | On `main` |
|---|---|---|---|
| The device app | the stick root | the device product: the owners, the device app (its frame and the device composite) | `shells/stick`, `faraday-core` built with no feature |
| Faraday Demo | the Demo root | the Demo product: the owners, the online app (its frame and the online composite) | `shells/demo`, `faraday-host`, `faraday-core` with `demo` |
| The Starter | the Starter root | the Starter product: the settings owner and the Starter's composite outside the lock scope; the session, the vaults and the online app inside it | `shells/starter`, `faraday-starter` |
| Tour root (`ships = false`) | the tour root | none of its own: it builds the three product crates' models | `shells/snapshot`, `faraday-starter`'s `snap` example |

A product crate may depend on the runtime; only roots and tools depend on a product crate.
Reason: the tour root, the leak crate and each product's journey tests build the same models as
the products do without an edge from one root to another (S3), and the binary is left with
nothing but I/O. Turned down: the model written inside each root binary, which the tour root
could only copy.

Two library composites sit below the product crates:

- **The device composite** holds every feature of the device app that is not a shared owner
  (§3.1). It is the same crate in all three products. It has no build options: what differs by
  product reaches it only through `View` (`mode`, `facts.abilities`).
- **The online composite** holds the device composite, the Demo's strip (with the samples
  loader) and Transfer. With the frame it is the online app, which Faraday Demo's product and the
  Starter's product both hold as their `Root::App`. It routes between its three children once: a
  roll on the strip to New key or a vault's dice, the codex32 sample to the key form, a sample
  loaded to the session, a scan of Transfer's Receive.

**Decision: the Starter's Faraday Demo is the online app, a library Faraday Demo's and the
Starter's products both hold.** Reason: the routing between the strip, Transfer and the device's
features is written and tested once. Turned down: the Starter's composite holding the device
composite and the strip as two children of its own, which writes that routing twice. The Starter
depends on no crate of Faraday Demo's product or root, and nothing depends on a root (S7).

## 2. What a root gives the runtime

Each product crate implements `Root`, the one trait of the runtime crate (section 05, §1). The
runtime holds the three owners and the app as top states, and in the Starter the Starter's
composite as a fifth; every top state's `Out` is the kernel's `Top`. A message reaches the top
state its `Addr` variant names (`Settings`, `Session`, `Vaults`, `App`, `Outside`). The product
crate's part is its associated types and the functions that map presses, input, facts and the
window's close to an `Addr`, and its frame (`Root::frame`, present then draw over `Tops`).

| | The device app | Faraday Demo | The Starter |
|---|---|---|---|
| `Settings` | the settings owner | the settings owner | the settings owner, outside the lock scope |
| `Session` | the session owner | the session owner | the session owner, inside the scope |
| `Vaults` | the vault composite | the vault composite | the vault composite, inside the scope |
| `App` | the device app: the frame over the device composite | the online app: the frame over the online composite | the online app, inside the scope |
| `Outside` | `Nothing` | `Nothing` | the Starter's composite, with its own frame |
| `Press` | §4.2 | §4.2 | §4.2 |
| `Options` | none (`()`) | `--text`, `--transfer` | `--uninstall`, `--image` |

The product value carries its `Options`; the root binary builds it from the command line and
gives it to `Runtime::new`. `Root::mode` gives the mode a scope starts in (S9).

### 2.1 How a root ends

A feature never orders an exit. It sends its ending up as its `Out` (the power feature's Lock,
Quit, Power off and Restart), and each composite on the way lifts it into the top state's
`Top`, which the runtime handles (section 05, §1.1):

| `Top` | The device app | Faraday Demo | The Starter (from inside the scope) |
|---|---|---|---|
| `Lock(Lock { seal, idle })` | the lock order (05 §7.2), `Ask::Keep`, `Order::Exit(Exit::Lock)`: the process ends | the lock order; the carry stays in the runtime's hand; `Order::Exit(Exit::Reset(generation))`; the scope built again | the same as Faraday Demo, over the scope |
| `End(PowerOff)` | the end order (05 §7.7), no seal; power off | never | never |
| `End(RestartComputer)` | the end order, no seal; the computer restarts | never | never |
| `End(Quit)` | never | the end order, no seal; the window closes | the scope closed: the end order over the scope, no seal, then `Order::Exit(Exit::Reset(generation))` in place of the end's exit |
| `Open` | never | never | sent by the Starter's composite after `SystemAsk::PrepareDemoSticks` answered `Ok`: the scope built |

The window's close (`CloseAsked`) in the Starter is the Starter's own end (`End(Quit)` from the
Starter's composite, outside the scope), which ends the process.

**Decision: only the runtime orders `Exit`, on a top state's `Top`.** Reason: a lock and an end
must pass through the runtime's order (the hide, the seal at a lock, `at_lock`), and a `Top`
from inside the scope is told apart from one from the Starter's composite by the `Addr` it came
from. Turned down: an adapter deciding whether an end closes the Demo or the Starter (section 02,
§4.9 and §10), which puts an app decision in an adapter. A close of the Starter's Demo seals
nothing, as an end seals nothing: the person is asked first where something would be lost.

### 2.2 The lock scope

The lock scope is the part of the model a lock drops and builds again.

- The device app and Faraday Demo: the whole model (the three owners and the app).
- The Starter: the session, the vaults and the online app. The settings owner and the Starter's
  composite are outside it: the Starter has no lock (inventory `features/starter-app.md`).

At every lock, `carry` is called on the scope's top states and, always, on the settings owner, so
the mode a person picks inside the Starter's Demo is carried into the scope built next. Only the
scope's states are wiped, dropped and built again.

`Start` is the first event of every process, exactly once. A scope built in the same process
(Faraday Demo's lock, the Starter's Demo opened or locked, a failed keep on the device) is started
by the runtime from the facts it keeps and the carry in its hand (section 05, §7.6); `Ask::Keep`
is asked only when the lock ends the process (`Abilities::lock_ends_process`). The generation of
`AskId`, `ChangeId` and `JobId` counts builds of the scope. Whatever the part outside the scope
asks for, changes or starts carries generation 0: it is never stale at a lock and never dropped
by `Exit::Reset`, so a lock in the Starter's Demo leaves a stick being written by the Starter
alone (section 05, R26).

## 3. The features each product holds

### 3.1 The device composite

The device composite is one crate. It holds these children (each a composite or a feature crate;
which features share a crate is section 09's). Its children reach one another only through it
(A6), and the shared owners only through `View` and changes (03 §7).

| Child | Kind | Features (inventory file) | Pages (main's `Screen`) |
|---|---|---|---|
| home | feature | Home and the bar search (`menu-pages.md`) | Home |
| keys | composite | New key and the key-making part (`keygen-flow.md`); Add randomness and the rollers (`randomness.md`); codex32 shares (`codex-shares.md`); Add a key and the ways in (`add-key.md`); Krux codes (`krux-codes.md`); BIP-85 (`child-keys.md`); Convert (`convert.md`); Vanity (`vanity.md`); Secure Boot keys (`secure-boot.md`) | KeyGen, Randomness, Entry, KefTool, Bip85, Convert, Vanity |
| wallets | composite | the Wallets tab and cards (`wallets.md`); Make a new wallet (`create-wallet.md`); Restore (`restore-wallet.md`); seeds to a wallet and Sweep (`seeds.md`); public files (`public-files.md`); a Core wallet (`core-wallet.md`); addresses (`address-check-explore.md`); silent payments (`silent-payments.md`); Back up (`wallet-backup.md`); the backup flow (`backup-flow.md`); Backups (`backups-list.md`); the codex32 item (`codex-backup.md`); the backup board (`backup-board.md`); Spend from a backup (`spend-backup.md`) | WalletsStart, Wallets, Create, Restore, CoreWallet, Explore, CheckAddress, Silent, Backup, BackupFlow, Backups, SpendBackup |
| signing | composite | Spend and its review (`spend-signing.md`); Decode, the nonce and signer checks (`decode.md`); messages (`messages.md`) | Spend, Decode, NonceCheck, SignerCheck, Message, CheckMessage |
| checks | composite | the Check page (`check-page.md`); the seed checks (`seed-checks.md`) | Check, SeedCheck, MultisigCheck |
| tools | composite | the Tools catalog, calculators, Lightning, invoices, OTS (`tools.md`) | Catalog, Tools, Lightning, Invoice, OtsCheck |
| files | composite | Files, the file menu (`files-inbox.md`); the boot import (`boot-import.md`); sticks coming and going (`stick-visit.md`); delete and zeroing (`delete-and-zero.md`) | Files |
| power | feature | Lock, Power, the idle lock and warning, held for backup (`lock.md`) | the Lock, LockAsk, Locked, Power, IdleWarn sheets |
| devices | feature | the device check and held-back inputs (`devices-inputs.md`) | the Devices and NewInput sheets |
| scan | feature | the scan sheet, its read card and the QR view (`scanner-qr.md`), over the scanner part | the Scan and Qr sheets |
| guide | feature | the Guide button and its side pane (`guide.md`) | the Guide sheet |

The shared owners, top states beside the app in each model (03 §7.1):

| Owner | Features (inventory file) | Pages |
|---|---|---|
| session | the keys, wallets and what waits (`wallets.md`, its session part); the Inbox and Outbox (`files-inbox.md`); the session's randomness (`randomness.md`); keys held for their backup (`lock.md`) | none: the session pane draws it |
| vaults | the list and Create (`vault-list-create.md`); Unlock (`vault-unlock.md`); inside (`vault-contents.md`); the seal (`vault-seal.md`); one-time codes (`totp-codes.md`); GPG keys (`gpg-keys.md`) | Vaults, CreateVault, Unlock, VaultContents, Codes |
| settings | Settings, Set the time, the settings file (`settings-time.md`) | Settings, SetTime |

### 3.2 The other features and where the rest of the inventory goes

| Inventory file | Where it goes |
|---|---|
| `transfer.md` | the transfer feature: in the online composite (Faraday Demo and the Starter's Demo) and, a second instance, in the Starter's composite |
| `practice-demo.md` | Practice's refusals: each feature's `refuses` and the key boundary (A32, A33); the strip and the Samples sheet: the strip feature of the online composite, over `faraday-samples` |
| `starter-app.md` | the Starter's composite: home (This computer, the checks, the Demo card), install, appearance (Appearance and About, as changes to the settings owner), templates, the Starter's guide; its frame |
| `starter-make-stick.md` | the make-stick flow of the Starter's composite |
| `starter-start.md` | the start feature of the Starter's composite |
| `secrets.md`, `typing.md` | parts: the passphrase field (A3), the secret checks, the text field and caret |
| `flow-container.md` | the kernel's step-flow runner (A29) and the kit's step-flow template |
| `app-shell.md` | the runtime (the event loop, section 05) and the product crates (the gates, §7) |
| `screen-frame.md`, `small-panel.md` | the app's frame (§4) over the kit's templates |
| `menu-pages.md`, `pane-container.md`, `session-pane.md`, `sheets-parts.md` | the kit's templates and organisms; the session pane is filled by the frame from `View::session` |
| `motion.md`, `drawing-compute.md` | the runtime and the kit (sections 04, 05) |
| `dev-diagnostics.md` | the self-test: `Facts::selftest` and the frame's page switch (§7); the dev reports: the stick adapter and the root's dev flags |
| `stick-shell.md` | the stick root and the stick adapter |
| `storage.md` | the storage adapter, the disk process and the grant helper (section 02, §8) |
| `pointer-touch.md` | the stick adapter (the cursor, touch decoding) and the kit (tooltips, focus) |
| `desktop-shells.md` | the window adapter, the desktop adapter (sticks as folders, the host) and the camera adapters |
| `tours.md`, `tour-runner.md` | the tour root and its fixture adapters (§8) |
| `ui-press.md`, `ui-controls.md`, `ui-text.md`, `ui-sheets.md`, `ui-navigation.md`, `ui-regions.md`, `ui-theme.md`, `ui-guide-text.md`, `keyboard.md`, `draw-base.md`, `tokens.md`, `layout-rules.md` | the kit (section 01, §6) |
| `test-kit.md`, `tooling.md` | tools (section 01, §7.3) |

With §3.1 this places all 82 files of the inventory.

### 3.3 The tools a menu page opens

This answers section 07's question (§5.4 there). The tools Check and Tools both open are one
enum, `Tool`, a handle in the kernel (`handle` module), since two features name it (A1). The
checks and tools composites each send `Out::Go(Tool)` up; the device composite routes each `Tool`
to the feature that holds it with one exhaustive `match`. The Tools catalog's tile list is an
exhaustive `match` over `Tool` in the tools composite's `present`, so a Check row whose `Tool` has
no tile does not compile.

## 4. Models, pages and navigation

### 4.1 The models

The runtime holds the top states (section 05, §1); the product crate writes the app's state and,
in the Starter, the Starter's composite's:

```rust
/// The frame's own state, in each app (and in the Starter's composite): what no child holds and
/// the frame needs to route.
pub struct Frame<P> {
    nav: Nav<P>,                      // kernel (§4.3)
    sheets: Vec<FrameSheet>,          // the frame sheets on show, kept in the order of §4.4
    camera: Option<Holder>,           // the camera's holder, from its feature's Out (§6.2)
}

/// The device app's `App`.
pub struct DeviceApp { device: device::State, frame: Frame<device_app::Page> }
/// Faraday Demo's `App`, and the Starter's inside its lock scope.
pub struct OnlineApp { online: online::State, frame: Frame<online::Page> }
/// The Starter's `Outside`.
pub struct StarterApp { starter: starter::State, frame: Frame<starter::Page> }
```

The Starter's Demo is its lock scope: its session, vaults and `OnlineApp` are top states of the
Starter's model that exist only while the scope is built. Its frame records (its `Nav`, its sheet
stack, its camera holder) are in its `OnlineApp`, so they are dropped and built again with it.

The runtime lends `View` (section 05, §4.1): an owner's own slot is the copy or empty reader of
R13 there. The Starter's composite, outside the scope, is always lent the empty session and
vaults (A19), whether the scope is built or not (section 05, R26): no Starter feature reads the
Demo's keys or Inbox, and none can send them a change.

**Decision: in the Starter, the settings owner sits outside the lock scope, beside the Starter's
composite.** The Starter's Appearance and About pages and the Demo's Settings page change the same
owner. Reason: `View` lends one `settings`, and the Demo takes the Starter's theme and text size
(inventory `features/starter-app.md`); one owner gives one appearance with no copying between
two. Turned down: a settings owner in the scope as well, which the Starter would have to keep in
step by changes at every build of the scope. It follows that in the Starter the settings are kept
by the Starter (`Ask::Settings`, its text size, only while a Starter is installed) and not read
from the Demo's folder boot stick.

**Decision: the Starter's Faraday Demo is the lock scope of the Starter's one model, beside the
Starter's composite.** Reason: the runtime holds the owners, so the Demo's session and vaults can
only be its top states; as a scope beside the Starter's composite, the part outside it is lent the
empty session and vaults by construction, the Demo's frame records are rebuilt with it, and the
Starter names only `Page::Demo`. Turned down: the Demo as a child state inside the Starter's
composite, which would hold owners the runtime must hold and the lock's whole order inside a
composite; and a second model hosted by the runtime, which would give the Demo a settings owner
of its own.

### 4.2 Pages, bodies, sheets and presses

```rust
// The device composite's pages: each a child's own page type.
pub enum device::Page {
    Home, Keys(keys::Page), Wallets(wallets::Page), Signing(signing::Page),
    Checks(checks::Page), Tools(tools::Page), Files(files::Page),
}

// The device app.
pub enum device_app::Page { Device(device::Page), Vaults(vaults::Page), Settings(settings::Page), SelfTestFailed }

// Faraday Demo, and the Starter's Demo (the online composite's crate).
pub enum online::Page { Device(device::Page), Transfer, Vaults(vaults::Page), Settings(settings::Page), SelfTestFailed }

// The Starter's composite.
pub enum starter::Page {
    Home, Make(make_stick::Page), Start, Tools, Templates(templates::Page), Guide,
    Install(install::Page), Appearance, About, Transfer,
    Demo,                             // the lock scope's own page, shown by its own Nav
}
```

A page value names a feature and which of its screens; a step flow's steps are the flow's state,
not pages. A feature with one screen has no `Page` of its own (09 §4.4); the variants of those
named here (`make_stick::Page`, `templates::Page`, `install::Page`, `backup::Page`) are the
porting's, from the feature's inventory file; each holds the variant of its first screen. The
Starter's `Tools` is the Starter's own page (its
tools list, inventory `features/starter-app.md`), held by its composite, with no feature of its
own and its model the composite's own `Model` (`Body::Tools(Model<'s>)`); `About` is the appearance feature's second page. Sheets are the features' own state (inventory `features/sheets-parts.md`), but for the
frame sheets in §4.4. The Starter names only `Page::Demo`: which Demo page shows is the online
app's `Nav`, so no Starter crate names `online::Page`.

**Decision: an owner's page is presented and drawn by the product crate, in every product; its
page value is the kernel's.** `vaults::Page` and `settings::Page` are types of the kernel's
`shared` module, as the owners' `Change` types are (03 §7.3), and the two owners re-export them.
The online composite's `Nav` therefore names an owner's page with no edge to an owner (09 §4.5),
and the app's press that begins an owner's page (§4.3) is a value the app can build. The online
composite presents what it holds and nothing of an owner: its `Body` has no owner variant, and it
gives the two products that hold it the shown page, the frame's model, its body and its sheets
(09 §4.7). The product crate, which may depend on the owners, presents the owner's page from
`Tops` when the shown page is one, and builds the screen model. Reason: presenting an owner's page
calls the owner's `present` over its state, which only a crate with an edge to the owner can
name, and the owners' presses are the product's `Press` (below). Turned down: an edge from the
online composite to the owners, which would let a composite shared by two products hold an
owner's state; and an index of the owner's page that the product maps, which no type holds.

Each product crate gives the `ScreenModel<'s, B, O, R>` of section 04 (§4 there) a `Body` and an
`Over` enum, one variant per page or sheet model, and a `Press`, one variant per top state's
press (`Root::Press`):

```rust
// The device app's product crate.
pub enum Body<'s> {
    Device(device::Body<'s>), Vaults(vaults::Model<'s>), Settings(settings::Model<'s>),
    SelfTestFailed(device::FailedModel<'s>),
}
pub enum Over<'s> { Device(device::Over<'s>), Vaults(vaults::SheetModel<'s>), Frame(FrameSheetModel<'s>) }
#[derive(Pressed)]
pub enum Press { Settings(settings::Press), Session(session::Press), Vaults(vaults::Press), App(AppPress) }
/// `device_app::Press`: what a press on the app's frame or the device composite's page sends.
#[derive(Pressed)]
pub enum AppPress { Device(device::Press), Open(device_app::Page), Back }  // a destination; the top bar's Back

// Faraday Demo's: the online app's page or an owner's, presented by the product.
pub enum Body<'s> { Online(online::Body<'s>), Vaults(vaults::Model<'s>), Settings(settings::Model<'s>) }
pub enum Over<'s> { Online(online::Over<'s>), Vaults(vaults::SheetModel<'s>) }
// `Press` as the device app's, with `App(online::Press)`.

// The Starter's: the Starter's own pages, and the Demo's whole screen under the Starter's bar,
// presented by the Starter's product as Faraday Demo's product presents its own.
pub enum Body<'s> { Starter(starter::Body<'s>), Demo(DemoScreen<'s>) }
pub enum Over<'s> { Starter(starter::Over<'s>), Frame(FrameSheetModel<'s>) }
pub type DemoScreen<'s> = ScreenModel<'s, DemoBody<'s>, DemoOver<'s>, Press>;
pub enum DemoBody<'s> { Online(online::Body<'s>), Vaults(vaults::Model<'s>), Settings(settings::Model<'s>) }
pub enum DemoOver<'s> { Online(online::Over<'s>), Vaults(vaults::SheetModel<'s>) }
#[derive(Pressed)]
pub enum Press {
    Settings(settings::Press), Session(session::Press), Vaults(vaults::Press),
    App(online::Press), Outside(starter::Press),
}
```

`Root::press` maps each variant to the `Addr` of the same name. `ScreenModel::over` is a
`Vec<(Anchor, O)>`, the lowest first. The vault composite exports `SheetModel` and
`sheets(state, view, show) -> Vec<(Anchor, SheetModel)>` beside its `Model`, for the sheets of the
vault area over any page. The device composite's `FailedModel` is the power feature's
(`power::FailedModel`, whose fields come with the port), re-exported by the device composite's
`frame` module for the products,
which do not depend on power. The Demo's and the Starter's products each write the three-variant
Demo body; neither depends on the other (§1).

- **Anchors.** Each page and each sheet has an `Anchor` (port `values`), a constant of the crate
  that holds it: `scope` is the crate and page or sheet name, `n` the variant's index. The screen
  template sets `FrameOut::page` and `FrameOut::sheets` from them.
- **Laid facts.** `Laid::Placed` (whether the page stood apart) goes to the frame's `Nav`, a
  message of the app (`Addr::App`; in the Starter, `Addr::Outside` for its own pages). `Laid::Room`
  (whether the viewfinder has a place beside the page) goes to the scan feature.

### 4.3 Navigation

The kernel holds one `Nav<P>`, used by every frame. Reason: Back, Forward, the page a flow was
begun from and the resume on a tab are then written and tested once. Turned down: each product
writing its own, three copies of the same rules.

- **Opening a page.** A feature sends `Out::Open(its page)`; each composite lifts it into its own
  page type; the frame's `Nav` shows it and sends `Notice::Left` to the feature whose page was
  left, through its composites (A13).
- **Reaching another feature.** A feature never opens another's page: it sends what it wants
  (`home::Out::CreateWallet`, `Out::Go(Tool)`) and the composite that holds both routes it (A6).
- **An owner's page.** A press that begins an owner's page (Unlock this file, a Settings section)
  is the app's: its `Nav` shows the owner's page and, where the page begins with something given
  (the file to unlock), the app sends the owner a change (03 §7.3). Presses on an owner's page are
  the owner's (`Press::Vaults`, `Addr::Vaults`), and what the owner did reaches the app as
  `Notice::Shared`.
- **Destinations.** The device app: Home, Check, Wallets, Files, Vaults, Tools, Settings. Faraday
  Demo and the Starter's Demo: the same and Transfer (shown when `mode.online`, which is always).
  The Starter: Home, Make, Tools, Guide. A destination pressed while a flow of it is begun resumes
  the flow (inventory `features/screen-frame.md`).
- **Back.** Back goes first to the top sheet of the frame's stack, then to the shown page's
  feature (a step flow's `FlowMsg::Back`); a flow that ends with `Ending::Left` returns to the
  page it was first begun from, kept by `Nav` (F77 as amended); otherwise `Nav` goes back one
  page. In the Starter, Back over `Page::Demo` goes to the Demo's own frame (`Addr::App`); at the
  Demo's Home it is the power feature's Quit, which asks first where the Lock sheet would
  (`asks_first`) and then sends `Top::End(End::Quit)`, which closes the scope (§2.1). Back while a
  stick is written is refused by make-stick.
- **Forward.** Reopens the page Back left; in the Starter not from a confirmation and not under a
  sheet (inventory `features/starter-app.md`).
- **A lock scope built again.** The scope's `Nav` is built with it and shows the scope's Home,
  `Device(Home)`. In the Starter, the Starter's own `Nav` keeps `Page::Demo` across a lock in the
  scope; when the scope is closed, the runtime gives the Starter's composite `Notice::Left`, and its
  `Nav` drops every `Page::Demo` entry and shows Home.

### 4.4 The frame

The frame is the app's `present` and `page`: the navigation rail or bar and the top app bar by
size class, the page area, the notices (busy line, remove the stick, the snackbar), the session
pane filled from `View::session`, and the sheets over the page. `Root::frame` presents the page
the shown `Nav` names from the top state that holds it (an owner's page from `Tops`, §4.2) and
draws the screen template (section 04, §12.4). A frame model shared by two products (the online
app's) is built in the product's press by a `lift: fn(online::Press) -> R` the product gives, as
`Ui::within` lifts a body's presses. What differs by product:

| | The device app | Faraday Demo and the Starter's Demo | The Starter |
|---|---|---|---|
| App name | FARADAY OFFLINE | FARADAY DEMO | the Starter's top bar; the Demo's frame under it |
| Power control | Power off | Quit | the window's close (Linux) |
| Strip at the foot | none | the strip (not with `--panel`) | the Demo's strip while it shows |
| Frame sheets | IdleWarn, NewInput, Devices, Lock and Power, Guide | the same, NewInput never shown | the Starter's Restart, Uninstall, LetThrough, How, Guide sheets |

The frame sheets are those of the features held directly by the device composite (power, devices,
guide) and of the Starter's own. Their feature says it shows or puts away a sheet by
`Out::Front(Some(sheet))` and `Out::Front(None)`; the frame keeps the stack, never reads it from
the feature (A7). The stack's order, top first: the Lock sheet (and LockAsk), then IdleWarn, then
NewInput, Devices, Power and Guide, then a feature's own sheet. The idle warning covers every
sheet but a lock's own; the sheets under it come back when it is put away.

## 5. Each product

### 5.1 The device app

- **Modes:** Mainnet (the default), Testnet, Signet, Regtest; `practice` and `online` false.
- **Adapters:** the stick adapter (framebuffer, input devices each grabbed, held-back input, the
  arrow, turns, attached sticks, the clean marker, the read-only root, the dev reports), the
  storage adapter over the disk process (the carry in the RAM directory, the memory figures), the
  V4L2 capture adapter and the scanning adapter with `ReadMode::Reduced`. The disk process and the
  grant helper are their own processes (S2).
- **Carries alone:** held-back inputs and the device check's facts (`DevicesFact`), the clean
  marker, `Turn` and rotation offered, `RestartComputer`, the boot medium and the SD card medium,
  the read-only root, a lock that ends the process (`lock_ends_process`).
- **Never holds:** Transfer, the strip, the samples, a host, a window, a clipboard or a print
  (S8).
- **Settings:** read from the boot stick's settings file at the first look, kept across a lock in
  the carry.
- **A failed keep:** as section 05, §7.6: the runtime builds the scope again in process from the
  carry handed back when it holds an only copy, and ends the process otherwise.
- **Options:** none reach the model. The command line (`--fb`, `--size`, `--depth`, `--dpi`,
  `--native`, `--mount`, `--pointer-speed`, `--touch-grid`, `--files`, `--verbose`, `--timings`,
  `--boot-report`) configures the adapters and the dev reports. The disk process's two FIFO paths
  and its RAM directory are fixed paths that the startup script makes, not flags; the root's
  `Options` holds them as strings.

### 5.2 Faraday Demo

- **Modes:** Practice (the default; Testnet), Testnet, Signet, Regtest; `online` true. Never
  Mainnet.
- **The session** is built in Practice taking only the published samples (A33); the strip's
  Samples sheet loads them by `AddKey` changes.
- **Adapters:** the window adapter (window, mouse, touch, keys, clipboard, `MovesWindow`
  regions, randomness from the operating system), the desktop adapter (sticks as folders, the
  pills' `ToggleStick`, Downloads, opening, the print folder, the memory figures), the capture
  adapter (V4L2 or Media Foundation) and the scanning adapter with `ReadMode::Full`.
- **Carries alone:** the strip, `PreviewSize`, sticks as folders made at start.
- **Settings:** as the device app, over the folder boot stick and the carry.
- **Options:** `--text` (the starting text size) and `--transfer` (Transfer shown first) are
  `Options`; `--sticks`, `--downloads`, `--size`, `--panel`, `--aspect`, `--ppi`, `--timings`,
  `--no-camera` configure the adapters.
- **A lock** builds the scope again in process: the carry stays in the runtime's hand, the
  adapters reset on `Exit::Reset`, and the scope starts in the carried mode (section 05, §7.8).

### 5.3 The Starter

- **Its own features:** home, install, appearance, templates, its guide, make-stick, start,
  transfer and a scan feature for Transfer's Receive (§3.2). No session and no vault of its own.
- **Adapters:** the window adapter, the desktop adapter (for its Demo's folder sticks, and its
  own Downloads, saving, the print folder, opening a saved PDF), the Starter's operating-system
  adapter (`SystemAsk`, `SystemFact`, `settings.txt`, the colour scheme at start), the capture and
  scanning adapters as Faraday Demo.
- **Carries alone:** `Ask::System` and `SystemFact`, `Ask::Settings`, the templates' printing, the
  lock scope beside its composite.
- **Options:** `--uninstall` (the Installed page with Uninstall asked) and `--image` (an image
  for make-stick, as if dropped) are `Options`; `--downloads`, `--size`, `--timings`, `--no-camera` configure
  the adapters. Its Demo's sticks folder is a fixed one in the home directory, not a flag; the root's
  `Options` holds its path as a string.

The Starter's Demo, in order (answering section 02's question):

1. Open Faraday Demo on the Starter's Home: home sends `Out::OpenDemo`. With the scope built, the
   Starter's `Nav` shows `Page::Demo`. Otherwise the Starter's composite asks
   `SystemAsk::PrepareDemoSticks`.
2. The desktop adapter makes the sticks folder, `BOOTSTICK` once (its marker) and `STICK-2`, and
   answers `Ok`, or a `Failure` naming the folder, which home shows on the Demo card.
3. On `Ok` the Starter's composite sends `Top::Open`. The runtime builds the scope from the facts
   it keeps (the session in Practice, empty vaults, the online app) in the mode `Root::mode` gives,
   and the Starter's `Nav` shows `Page::Demo`, whose own `Nav` shows `Device(Home)`. The folder
   sticks come as `SticksFact::Listed`.
4. A lock in the Demo: its power feature's Lock reaches the runtime as `Top::Lock`; the runtime
   runs the lock order over the scope, carries on the scope's top states and the settings owner,
   orders `Exit::Reset(generation)`, and builds the scope again in the carried mode.
5. Closing the Demo (the Starter's Back over it, or the Demo's Quit): `Top::End(End::Quit)` from
   inside the scope; the runtime runs the end order over the scope with no seal and orders
   `Exit::Reset(generation)`, so every adapter drops what the scope had (its picture copies, its
   camera buffers, its folder-stick writes not begun). The Starter's `Nav` shows Home (§4.3).
6. While the Demo shows, the Starter's bar takes only Back; presses on the Demo's frame are the
   Demo's (V3); ticks and facts reach both.

## 6. From port facts to messages

### 6.1 Facts, streams and input

A port fact that is a state of the world is kept by the runtime in `Facts` and read through
`View` (03 §5). Every other fact is a `Stream` and goes through `Root::fact` to one feature. This
answers section 03's question.

```rust
/// The runtime's: the port facts that are not states of the world.
pub enum Stream { Devices(DevicesFact), Camera(CameraFact), Host(HostFact), System(SystemFact) }
```

| Fact | Reaches | The device app | Faraday Demo | The Starter |
|---|---|---|---|---|
| `CameraFact::Listed`, `HostFact::Downloads`, `SticksFact::*`, `Display`, `Time` | `Facts` | `Facts` | `Facts` | `Facts` |
| `SystemFact`, but `WriteProgress` | `Facts::system` | never sent | never sent | `Facts::system` |
| `DevicesFact::*` | `Root::fact` | devices | devices (only `Trust`, at start) | the Demo's devices; dropped while the scope is not built |
| `CameraFact::Frame`, `Code`, `Seen`, `Ended` | `Root::fact` | the camera's holder (§6.2) | the same | the same: the Starter's scan, or one in the Demo |
| `HostFact::Hovered`, `Dropped` | `Root::fact` | never sent | transfer; `Dropped` also opens Transfer | the Demo's transfer while it shows, else make-stick (an image file) |
| `SystemFact::WriteProgress` | `Root::fact` | never sent | never sent | make-stick, the only feature that writes a stick |

Input does not pass through `Root::fact`:

| Input | Reaches |
|---|---|
| `Input::Touch`, `Hover` | the message of the frame's hit target (V3) |
| `Input::Scroll`, `Wheel`, `ScrollEnd` | the runtime's motion; no message (section 04) |
| `Input::Key` | section 04's reach order (the focused hit, then the frame's keys), then `Root::input` |
| `Input::Back`, `Forward` | `Root::input`, to the shown frame's `Nav` (§4.3) |
| `CloseAsked` | `Root::close_asked`: Faraday Demo's power feature (Quit); the Starter's make-stick first (refused while a stick is written), then the Starter's end |

### 6.2 The camera's holder

A feature that turns the camera on (`camera::On`) also sends `Out::Camera(true)`, and
`Out::Camera(false)` when it turns it off. Each composite on the way records which child holds it
in its own state, and the frame records the top of the path. `Root::fact` routes every
`CameraFact` of a stream down that path, so a frame (`Bytes`, not `Clone`) reaches exactly one
feature. The holder takes only facts of the stream its `Started` answer named; the runtime has
already dropped those of a stream turned off (P21). In the device app the holders are the scan
feature, New key's camera noise and Add randomness's camera; in the Starter also the Starter's own
scan, whose record is in the Starter's frame and not in the scope.

**Decision: stream facts are routed by records the composites keep from their children's outs.**
Reason: it holds inside section 03's routing (A6, A7) with no new kernel type. Turned down: a
kernel stream ask whose reply the runtime keeps for every fact of the stream, which would be a
second kind of reply for one family of facts.

## 7. The gates main's dispatch held

`main` checked these before its press dispatch (inventory `features/app-shell.md`, "Dispatch
order"). Each has one place:

| Gate | Where | Held by |
|---|---|---|
| A failed self-test: only Power off | the frame shows the `SelfTestFailed` page alone while `facts.selftest` is `Failed`; its one control is the power feature's Power off (Faraday Demo: Quit) | the frame's `present`: no other page is drawn, so no other hit target exists (V3); a routing test of each product crate |
| Practice | each feature's `refuses` (A32); the key boundary (A33) | update tests; `faraday-keys` |
| The idle warning takes the next input | the IdleWarn sheet, in front of every sheet but a lock's own, takes every key and press and puts itself away | the frame's stack (§4.4); an update test of power |
| A stick in the way of a key load | the press of a key-loading feature is taken, and its `update` opens the Pull sheet while `facts.sticks.attached` is set; the session refuses `AddKey` while a stick is attached | update tests; the session's update test |
| The device check | a press that would show a secret is the feature's `Out::Reveal(r)`; the device composite routes it to devices, which answers `Cleared(r)` back to the feature, at once or after the Devices sheet | the device composite's routing test; update tests of devices |

## 8. The tour root

The tour root builds any of the three product crates' models over the tour root's fixture
adapters, which answer storage, randomness, camera, host and system asks from fixtures and send
scripted `Input`. It reads the runtime's frame (R4) and writes a picture at each named step.

- The fixture adapters are a library crate (`layer = "tool"`), which the leak crate and each
  product's journey tests use too (section 07, §6.2). This answers section 07's question.
- A root binary's loop (section 02, §1) and a journey test drive the same `Runtime<R>` over the
  same product value: the binary over its real adapters, the journey over the fixture adapters
  (section 07, §2.5). This answers section 07's question.

## 9. What a root may and may not do

- **Rule S5.** A product that ships is one root binary and one product crate. The root binary
  holds no library target and depends on its product crate, the runtime, the port and the
  adapters it builds, and on nothing else: no feature, part or kit crate. Held by `lint-layers`
  (a shipping `root` crate's edges; `role = "product"` on exactly one dependency). The tour root
  (`ships = false`) is exempt from the one-product rule.
- **Rule S6.** A root makes no event. It parses its command line, builds its adapters and its
  product value, and runs the loop of section 02, §1; every event the model sees came from an
  adapter's `poll`, and the command line reaches the model only as the product value's `Options`.
  Held by `lint-layers`' module rule (a root crate builds no `port::Event` value).
- **Rule S7.** The Starter depends on no crate of Faraday Demo's product or root; it holds the
  online app, a library, as its lock scope. Held by `lint-layers` (S3, and its edge rule refusing
  an edge from the Starter's crates to any crate of Faraday Demo's product).
- **Rule S8.** No crate marked `online = true` (transfer, the strip, `faraday-samples`, the window
  and desktop adapters) is in the device root's resolved graph. Held by `lint-layers` over each
  root's resolved graph (as T1), refusing an `online` crate in a root marked `offline = true`.
- **Rule S9.** Each product crate fixes the modes its product offers (§5). An online product
  never offers Mainnet; the device never offers Practice or `online`. A carried mode outside the
  list starts the product's default. Held by `Root::mode`'s list, `Mode` being built only by a
  product crate, the session's refusals (A33), and routing and update tests of the product crate.
- **Rule S10.** The device composite names no product: it holds no Transfer, no strip and no
  Starter feature, and has no build options; what differs by product reaches it through `View`.
  Held by `lint-layers` (its dependencies) and the compiler.
- **Rule S11.** Only the runtime orders `Exit`, on a top state's `Top` (§2.1). No application
  crate names `port::Order::Exit`. Held by `lint-layers`' module rule.
- **Rule S12.** A lock drops and builds again only the lock scope (§2.2); whatever the part
  outside the scope asks, changes or starts carries generation 0 and is never stale at a lock, and
  that part is lent the empty session and vaults. Held by `Addr`'s variant (the runtime knows the
  part a message is for) and a runtime test over a test root with one scope and one outside part.
- **Rule S13.** A product crate routes from its children's outs and its own records (`Nav`, the
  frame's sheet stack, the camera's holder) and reads nothing in a child's state. Held by A7 (the
  compiler).
- **Rule S14.** Each fact of §6.1 reaches the one feature its row names, and none reaches a part
  of the model that is not built. Held by `Root::fact`'s exhaustive `match` (clippy's
  `wildcard_enum_match_arm`, denied in product crates) over `Tops`, whose scope's states are
  `None` while the scope is not built, and routing tests of the product crate.
- **Rule S15.** Each root's adapters carry its row of section 02's §10 and its `Start::abilities`
  says so. Held by A17 and the adapters' own tests.
- **Rule S16.** The tour root and the journey tests build the product crates' models, never a
  root binary. Held by S5 and `lint-layers` (the tour root `ships = false`, T1).

## 10. What holds each rule

| Rule | What it says, short | Held by |
|---|---|---|
| S5 | a product that ships is a root binary and a product crate | `lint-layers` |
| S6 | events only from adapters; options only in the product value | `lint-layers`' module rule |
| S7 | the Starter depends on no crate of Faraday Demo's product or root | `lint-layers` |
| S8 | no online crate in the device's graph | `lint-layers` over each root's resolved graph |
| S9 | each product's modes; no Mainnet online, no Practice offline | `Root::mode`; A33; routing and update tests of the product crate |
| S10 | the device composite names no product and has no build options | `lint-layers`; the compiler |
| S11 | only the runtime orders `Exit` | `lint-layers`' module rule |
| S12 | a lock is the scope's; the outside part's asks never go stale and it is lent no keys | `Addr`; a runtime test over a test root |
| S13 | a product crate reads no child's state | A7 |
| S14 | each stream fact to one feature | an exhaustive `match`; routing tests of the product crate |
| S15 | each root carries its abilities | A17; the adapters' tests |
| S16 | tours and journeys build product crates' models | S5; `lint-layers` |

## Questions for later sections

- Section 09: the crate names of the product crates, the online composite and the device
  composite's children, and which features of §3.1 share a crate.
