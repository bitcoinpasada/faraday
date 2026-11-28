# Presentation (present and draw) and the UI kit by atomic level

**Kind:** Spec
**Open:** Not built.

This section gives presentation's two stages in full: `present`, which works out everything a
screen shows into a model, and `draw`, which reads the model and paints through the kit. It says
where the wording reaches `present` and nowhere else, what the runtime's memo of a `present` is,
how a frame's layout facts come back (to the next frame's drawing, or to `update` as messages),
where a component's motion lives between frames, how a frame's drawn text is cleared after it is
painted, and how a frame that shows a secret is never kept. It arranges the kit by atomic level
(tokens, atoms, molecules, organisms, templates), says what each level holds and may name, and
places `main`'s components in it, the flow container, the menu page container, the pane
container and the screen frame as templates. It is written for whoever writes the kit, a
feature's `present` and `page` modules, the presentation kernel crate (§2), the product crates'
`Root::frame` (section 06) and the runtime (section 05). It answers the questions sections 01 and 03 left for it. Rules continue section
01's numbering: V5 onward for presentation, K9 onward for the kit.

## 1. One frame, in order

What happens between the last event of a settled dispatch (03, A15) and the frame the root
paints. Section 05 owns the runtime's loop; this is the part presentation depends on.

1. The runtime builds `View` (03 §5) and `Show` (§3) for the top states it owns.
2. It builds a `Ui` over the canvas with the pointer and the focus as the last frame left them
   (`Pointer`, §7.1), the motion store (§8)
   and the theme from `view.settings`, and calls `Root::frame` (section 05 §1) with the top
   states it lends (`Tops`), `View`, `Show` and the `Ui`.
3. The product crate's `frame` presents the app's frame and the page its navigation shows, from
   the top state that holds it (an owner's page from `Tops`), into a `ScreenModel` (§4) that is a
   local of `frame`, and then draws that model through the screen template. No state changes; the
   memo (§6) may fill.
4. `Ui::finish` gives the frame's output (§7): hits, anchors, the page and sheets drawn, scrolled
   regions, laid facts, whether a secret was drawn, and whether motion wants another frame.
5. The model is dropped when `frame` returns. Any reveal guard it borrowed ends here (01 §8.2).
6. The canvas replays its call list in bands and wipes the list's text (§9).
7. The runtime compares the frame's laid facts with the last ones sent; each one that changed
   becomes a message, routed before the next event, and one more frame is drawn (§7.3).

- **Rule V5.** `present` and `draw` run once per frame drawn, in that order, with no `update`
  between them, and the model never outlives its frame. Held by `Root::frame` (the model is a
  local of it, borrowing the top states the runtime lends) and the compiler (`'s`).
- At a lock or an end the runtime drops with the model everything it keeps of the frames: the
  last frame's output (its hits and laid facts), the last laid fact kept per `Anchor` (V17), the
  pointer and focus state, the note queue (§10), the memo (§6) and the motion store (§8).

## 2. Where the stages live

The kernel never depends on the kit (01 §1), and a page's types are the kit's. So the traits
that join a feature to the kit, and the generic fill of the step-flow template, live in one
application crate between them.

**Decision: a presentation kernel crate (`layer = "app"`, `role = "present"`) holds `Present`,
`Page`, `Drawn`, `ScreenModel`, `ShowFlow`, `FlowPress` and the step-flow fill (§4, §5), and
the helpers every `present` shares (`offer`, `line`, `busy`).** It depends on the kernel, the
port, the domain and the kit (its templates and organisms, and the atoms' `Pressed`, `Ui`,
`Shown`, `Offered` and `Laid`); it never depends on a feature, the runtime, an adapter or a root.
Every feature crate and every product crate depends on it. The runtime does not: it calls
`Root::frame` (section 05 §1), whose arguments are the kernel's `View` and `Show` and the kit
atoms' `Ui`, and each product crate writes its `frame` with the presentation kernel. Reason:
`Present` must be in the crate that writes its blanket impl for step flows (the orphan rule), and
that crate must name the kit's models, which the kernel may not. Turned down: the step-flow fill written in each flow's `present` module, which lets one flow
forget to dim Back while a step runs.

```rust
/// In the presentation kernel. The first stage. The model may name the kit's model types
/// (`Offered`, `Content`, a template's model), never a drawing function.
pub trait Present: Feature {
    type Model<'s>;
    fn present<'s>(state: &'s Self::State, view: &View<'s>, show: &Show<'s>) -> Self::Model<'s>;
}

/// In the presentation kernel. The second stage. `Press` is what a press on this page sends (§2.1).
pub trait Page: Present {
    type Press: Pressed;
    fn draw(model: &Self::Model<'_>, ui: &mut Ui<'_, Self::Press>);
}

/// In the kit's atoms. A press: copied into the frame's hit list, compared for hover, focus and
/// the pressed layer. Not Public: a typed character is a press.
pub trait Pressed: Clone + PartialEq + Send + 'static {
    /// Given by `#[derive(Pressed)]` from the `#[hides]` marks (§11.1).
    fn hides(&self) -> bool;
}
// with `<Self as Feature>::Msg: From<Self::Press>`, checked where `Page` is implemented
```

### 2.1 Presses are their own type

A hit list holds a copy of each press for the frame, and the pointer's hover and the pressed
layer compare presses (`main`'s `is_hovered`, `is_pressed`, inventory `features/ui-press.md`).
That needs `Clone + PartialEq`. A feature's `Msg` cannot be `Clone`: its answers move secret
types (03 §7.4, the MuSig2 round; 01 K-3).

**Decision: a page draws on `Ui<Self::Press>`, a type of the feature's presses alone, and the
feature's `Msg` is `From<Press>`.** The runtime matches a press against the last frame's hits and
gives it to `Root::press`, which names the top state it is for (`Addr`) and converts it with
`From` into that state's message (a composite's `From` turns a child's press into the child's
message); it is then routed (03 A9). Reason: it keeps K-3 (no
secret-moving variant is ever cloned) and gives the kit what it compares. Turned down: `Ui<Msg>`
with a derived `again()` that clones only the pressable variants, which a page could break with a
hit on a variant that cannot be copied, found only at run time.

- **Rule V6.** A press type is not required to be `Public`. A press that carries typed input (an
  on-screen keyboard key, a dice face, a word chosen from the list) reaches `Msg` in a
  `#[withheld]` variant, so it is never logged or replayed (03 A14, 01 M2). Held by the
  `Message` derive (a withheld variant logs its kind alone) and by `lint-secrets`, which refuses a
  public variant of a `Msg` whose payload is a press type that names a key, a character or a word.
- A composite's `Press` has one variant per child's `Press`, and its page draws a child's page
  inside `ui.within(Press::Child, |ui| child::Page::draw(m, ui))` (§7.1). A step flow's press type
  is the presentation kernel's `FlowPress<S, P>` (§5), with `FlowMsg<S, M>: From<FlowPress<S, P>>`
  where `M: From<P>`.

### 2.2 The module rule, completed

Section 01 §5's table holds, with these additions, all in `lint-layers`' module rule:

| Module | Also never names |
|---|---|
| `state`, `msg`, `update` | `Show`, `Memo`, `faraday_strings` (any path, `EN` included), `Worded::words` |
| `present` | `Ui`, the kit's drawing functions |
| `page` | `View`, `Show`, `faraday_strings` |

- **Rule V7.** No crate re-exports `faraday_strings::Strings`, `faraday_strings::EN` or
  `faraday_strings::fill` under another path. Held by `lint-layers`, which refuses a `pub use` of
  the wording crate outside it, so the module rule on its paths cannot be dodged.

## 3. The wording reaches `present` alone

`update` receives `View` through `Ctx`. With the strings in `View`, an update could form text
through `cx.view.strings` without naming the wording crate, and A30 ("an update forms no text")
would not hold.

**Decision: `View` holds no strings. The wording reaches `present` only, in `Show`, which the
runtime builds and only `present` is given.** `Worded::words` takes `&Show` instead of
`&'static Strings`, so a value can be worded only where a `Show` is. Reason: it makes A30 a
rule of the types. Turned down: the strings in `View`, with A30 held by review, which no check
would catch an update breaking.

```rust
/// In the kernel (`cx`). What present has beside View. Built only by the runtime.
pub struct Show<'a> {
    strings: &'static faraday_strings::Strings,
    memo: &'a Memo,              // §6
    note: Option<NoteUp<'a>>,    // §10
}
impl<'a> Show<'a> {
    pub fn strings(&self) -> &'static faraday_strings::Strings;
    pub fn memo(&self) -> &'a Memo;
    pub fn note(&self) -> Option<NoteUp<'a>>;
    /// The runtime's and test support's only (clippy `disallowed-methods`).
    pub fn new(strings: &'static faraday_strings::Strings, memo: &'a Memo, note: Option<NoteUp<'a>>) -> Self;
}

pub trait Worded: Send + 'static {
    fn words(&self, show: &Show<'_>) -> Line;
}
```

- **Rule V8.** An update forms no user-facing text. Held by two things together: the type
  (`Ctx` and `View` hold no `Strings`, and `Worded::words` needs a `Show`, which `update` is never
  given and cannot build: `Show::new` is in `disallowed-methods` for every application directory
  but the runtime's and test support's), and the module rule of §2.2 (the update modules never
  name `faraday_strings`, so they cannot reach the `pub static EN`). A domain function that words
  its own output (01 §2.2: a printed sheet, a file's text) still words it in the domain; what an
  update keeps is the domain's typed answer, worded by `present`.
- The language of what `present` words is the runtime's choice (one static per language, `main`'s
  F68), given to `Show::new`. Text the domain words (01 §2.2) stays in the domain's language.

## 4. The screen model and the root's frame

The product crate's `Root::frame` presents a `ScreenModel` and draws it. Its body and the sheets
over it are the product crate's `Body` and `Over` enums, one variant per page model, and its
`Press` has one variant per top state's press (section 06 §4.2). Left generic here.

```rust
/// In the presentation kernel. The root's model for one frame.
pub struct ScreenModel<'s, B, O, R: Pressed> {
    /// Which page: the motion store's scope (§8) and the page turn's.
    pub page: Anchor,
    /// The screen template's model: navigation, top app bar, pills, busy line (§12.4). The top
    /// bar's `power` is the Power control a device app shows (a press and its label); a bar
    /// without one leaves it `None`.
    pub frame: kit::screen::FrameModel<'s, R>,
    /// The page under the sheets: one variant per page the root holds.
    pub body: B,
    /// Sheets and dialogs over it, each with its anchor, the lowest first; only the top one
    /// takes presses.
    pub over: Vec<(Anchor, O)>,
    /// The note on show (§10).
    pub note: Option<kit::snackbar::SnackbarModel<'s>>,
}

/// In the presentation kernel. A body or sheet enum draws the variant it holds, each page's
/// presses lifted into the root's `R` (`Ui::within`, §7.1).
pub trait Drawn<R: Pressed> {
    fn draw(&self, ui: &mut Ui<'_, R>);
}

/// In the presentation kernel. What a root's `frame` calls once it has the model.
pub fn draw<B: Drawn<R>, O: Drawn<R>, R: Pressed>(m: &ScreenModel<'_, B, O, R>, ui: &mut Ui<'_, R>) {
    kit::screen::draw(m.page, &m.frame, ui, |ui| m.body.draw(ui), &m.over, |o, ui| o.draw(ui), m.note.as_ref());
}
```

The screen template records the page's anchor and the sheets' anchors, lowest first, in
`FrameOut::page` and `FrameOut::sheets` (§7.1), which the runtime's `Seen` reads (section 05).

- **Rule V9.** A model holds no coordinate, no size and no rectangle: widths are classes
  (`Width::{Column, Wide, Full}`, 03 §9.1), rows are counts, and every place is the kit's. Held by
  the kit's model types (none has a field of `f32` units or a `Rect`) and by `lint-layers`, which
  refuses `faraday_draw::geom` and the tokens' dimensions in a `model` or `present` module.
- **Rule V10.** The model is the same at every window size but for what `present` chooses by the
  size class it reads from `view.facts.display` (which rows the Pi's Home lists, for example;
  inventory `features/small-panel.md`). The compact layouts, the pin bar and the page per step are
  the templates' (§12). Held by V9 and by the templates' signatures.
- **Rule V11.** A refused control is drawn from the one gate (03 A10): `present` sets each
  control's `Offered` with `offer::<F>(state, &msg, press, view)`, which asks `F::refuses` and
  gives `Yes(press)` for `None` and `Refused(press, drawn)` for a refusal, `drawn` being the
  refusal's `Refused::drawn` mapped to the kit's `Drawn`. A refused control is drawn dimmed, or as
  usual for `AsUsual` (an eye, a Show button); its press is still taken, and `route` answers it
  with the refusal's note (F292 on `main`). A control the model offers no press for is `No`: drawn
  dimmed, it takes none. Held by the kit (`Offered` is the only way a control model is enabled or
  not) and the update tests of each refusal.

```rust
/// In the kit's atoms: what a control offers.
pub enum Offered<R> { Yes(R), Refused(R, Drawn), No }
pub enum Drawn { Dimmed, AsUsual }       // `faraday_atoms::Drawn`: the kernel's (03 §10), mapped by
                                         // `offer`; not the presentation kernel's trait `Drawn<R>`
```

A control whose work is running is not a fourth case: the gate refuses its press while the job
runs (03 A29), and the busy line says what runs (§5's `busy`).

## 5. The step-flow template filled from `StepFlow`

Section 03 §9.2 lists what `StepFlow` reads out. `StepFlow` has no press type; the presentation
kernel adds one in `ShowFlow` and fills the step-flow template's model from both, once, for every
flow.

```rust
/// In the presentation kernel. What only the flow can say: written in its present module.
pub trait ShowFlow: StepFlow
where
    <Self as StepFlow>::Own: From<<Self as ShowFlow>::Press>,
{
    /// What a press on the flow's own controls sends.
    type Press: Pressed;
    fn title(&self, s: Self::Step, show: &Show<'_>) -> Line;
    /// One line of Your choices: never a secret (03: `Answer: Public`).
    fn choice(&self, a: &Self::Answer, show: &Show<'_>) -> ChoiceLine;
    /// The current step's card: the content organism's blocks.
    fn card<'s>(&'s self, view: &View<'s>, show: &Show<'s>)
        -> Content<'s, FlowPress<Self::Step, <Self as ShowFlow>::Press>>;
    /// What stands under Your choices and the foot pinned in the pane (FLOW-CONTAINER §1 on main).
    fn under<'s>(&'s self, view: &View<'s>, show: &Show<'s>)
        -> Option<Content<'s, FlowPress<Self::Step, <Self as ShowFlow>::Press>>> { None }
}

/// In the presentation kernel: a step flow's presses.
#[derive(Pressed)]
pub enum FlowPress<S, P> { Forward, Back, ForwardAgain, Open(S), Cancel, CancelSure, CancelNot, Own(P) }
impl<S, M: From<P>, P> From<FlowPress<S, P>> for FlowMsg<S, M> { /* each to its FlowMsg; Own(p) to Own(p.into()) */ }

type PressOf<F> = FlowPress<<F as StepFlow>::Step, <F as ShowFlow>::Press>;

impl<F: ShowFlow> Present for F {
    type Model<'s> = kit::flow::FlowModel<'s, PressOf<F>>;
    fn present<'s>(s: &'s Flow<F>, view: &View<'s>, show: &Show<'s>) -> Self::Model<'s>;
}
impl<F: ShowFlow> Page for F {
    type Press = PressOf<F>;
    fn draw(model: &Self::Model<'_>, ui: &mut Ui<'_, PressOf<F>>);   // the step-flow template
}
```

The fill, field by field. `refuses` below is `<F as Feature>::refuses(s, &msg, view)`, the one
gate (03 A10), so a control the runner would refuse (Back at or before `back_stop()`, Back or
Cancel while a step runs, 03 A29) is never drawn live:

| `FlowModel` field | Filled from |
|---|---|
| `heading` | `steps()`: "Step N of M" when the count is known, else "Step N" (the wording's templates) |
| `title` | `title(at())` |
| `card` | `card()` |
| `choices` | a `ChoiceRow` for each step of `steps()`: its title, `answer(step)` worded by `choice` or "Not chosen yet", and its `Offered` from `FlowMsg::Open(step)` |
| `under` | `under()` |
| `back` | the `Offered` of `FlowMsg::Back` |
| `cancel` | `cancel()`: `NotOffered` gives `None`, no button; `Now` and `Ask` give the button, with the `Offered` of `FlowMsg::Cancel` |
| `asking` | `Flow::asking()`: the cancel sheet's model, with the loss worded |
| `refused` | `Flow::refused()`: the refusal's line at the foot |
| `busy` | the `Running` of the flow's job kinds in `view.running` and `running()` (03 §8.2) |
| `width` | `width(at())`, the kernel's `Width` mapped to the template's |
| `step` | `at()` and its place in `steps()`: the anchor of the fade through (§8) |
| `words` | the wording's "Your choices", Back and Cancel |
| `forward` | the step's forward action: its buttons, kept out of `card`, so the small panel pins them |

The template's model, in `faraday-templates`' `flow` module:

```rust
pub struct FlowModel<'s, A> {
    pub heading: Shown<'s>,
    pub title: Shown<'s>,
    pub card: Content<'s, A>,
    pub choices: Vec<ChoiceRow<'s, A>>,
    pub under: Option<Content<'s, A>>,
    pub back: Offered<A>,
    pub cancel: Option<Offered<A>>,      // None: no Cancel button
    pub asking: Option<DialogModel<'s, A>>,
    pub refused: Option<Shown<'s>>,
    pub busy: Option<BusyModel<'s>>,
    pub width: Width,
    pub step: Anchor,                    // the anchor of the fade through
    pub words: FlowWords<'s>,            // Your choices, Back, Cancel: the template names no strings crate
    pub lead: Option<ChoiceRow<'s, A>>,  // the wallet or key the flow is on: Your choices' first row
    pub chip: Option<Chip<'s, A>>,       // at the head of the card: where the flow goes next
    pub note: Option<Shown<'s>>,         // under the chip, a primary bar at its left
    pub forward: Option<ButtonsModel<'s, A>>, // at the card's end; on the small panel the pin bar's
    pub instead: Option<Instead<'s, A>>, // a pick or the take-away, in the pane in place of Your choices
}
pub struct ChoiceRow<'s, A> { pub title: Shown<'s>, pub answer: Shown<'s>, pub offered: Offered<A>,
                              pub current: bool, pub mono: bool }
pub struct FlowWords<'s> { pub your_choices: Shown<'s>, pub back: Shown<'s>, pub cancel: Shown<'s> }
pub enum Width { Column, Wide, Full }
```

**Decision: the template has its own `Width`, the same three classes as the kernel's (03 §9.1),
and the presentation kernel maps one to the other.** Reason: the kit may not name the kernel and
the kernel may not name the kit (01 §1), as with `Drawn`. Turned down: one `Width` in the tokens crate, which
would make the kernel depend on the kit.

**Decision: `FlowModel` has private fields and is built by `FlowModel::new` and the `with_*`
methods, which `disallowed-methods` refuses outside the presentation kernel's fill.** Reason: a
struct that takes every field is no better than public fields, so `new` takes the fields every flow
has and the optional parts (choices, under, cancel, asking, refused, busy) are named by consuming
`with_*` methods; with that, V12 is held by the code and a feature cannot fill a flow's model by
hand. Turned down: public fields (nothing stops a feature building the model); a `new` taking all
twelve fields (the same as public ones).

**Decision: the words the template draws are a `FlowWords` that `FlowModel::new` takes, and the
forward action is a field (`with_forward`), not the card's last block.** Reason: the kit names no
strings crate, so the fill gives the words; the pin bar must know which buttons to hold at the
small panel's foot without reading the card's blocks, and a flow whose forward action stands in
its card would be pinned by accident. Turned down: the template reading the words from the
strings crate; pinning a card's last `Block::Buttons`.

- **Rule V12.** Every step flow's model is filled by the one blanket impl; a flow writes only
  `ShowFlow`. Held by `lint-layers`, which refuses an `impl Present for` a type that implements
  `StepFlow` (so no `Page` either, `Present` being its supertrait), by `FlowModel`'s
  private fields (built only by `FlowModel::new`, which takes the fields every flow has: heading,
  title, card, back, width, the step anchor and the words, and the consuming `with_choices`,
  `with_under`, `with_cancel`, `with_asking`, `with_refused`, `with_busy`, `with_forward`,
  `with_lead`, `with_chip`, `with_note`, `with_pick` and `with_take` for the optional parts), by those
  methods in `disallowed-methods` (in `kit/clippy.toml` and `app/clippy.toml`, the two files whose
  crates can name the template) with the fill carrying one `#[allow(clippy::disallowed_methods)]`,
  and by K8 (the step-flow template takes only `FlowModel`).
- `Flow<F>`'s fields are private to the kernel. The fill reads them through `Flow::inner()`,
  `asking()`, `refused()` and `again()`, read accessors the kernel gives (03 §9.1).

## 6. The memo of a `present`

Section 03 A18 keeps a costly derivation in state or a job. The memo is for a derivation that is
cheap but repeated each frame (rows put together and ranked, a wallet's quorum counts, the Home
jobs' order). Its type is in the kernel's `cx`, reached through `Show`; the runtime owns its one
instance (01 R1).

```rust
/// In the kernel (`cx`). Entries keyed by (slot, the value's type, key); interior mutability,
/// the runtime its only owner.
pub struct Memo { /* entries, each marked when a frame uses it */ }

#[derive(Public, Copy, PartialEq, Eq, Hash)]
pub struct Slot(pub &'static str);          // "<feature>.<what>", a constant of the feature's crate

impl Memo {
    /// The value kept for (`slot`, `V`, `key`), or `fill(input)` kept and returned.
    /// `fill` is a function, not a closure: it sees only what `input` gives it.
    #[cfg(not(debug_assertions))]
    pub fn get<K, I, V>(&self, slot: Slot, key: K, input: I, fill: fn(I) -> V) -> Rc<V>
    where K: Public + Eq + Hash, I: Public, V: Public + 'static;
    /// In debug builds `get` fills again on every call and asserts the value equals the kept one.
    #[cfg(debug_assertions)]
    pub fn get<K, I, V>(&self, slot: Slot, key: K, input: I, fill: fn(I) -> V) -> Rc<V>
    where K: Public + Eq + Hash, I: Public, V: Public + PartialEq + 'static;
}

/// In the kernel (`cx`): how many times an owner or a feature has changed what it holds.
#[derive(Public, Copy, PartialEq, Eq, Hash)]
pub struct Edits(u64);
```

- **Rule V13.** A memo's key holds every input its value is worked out from, as values or as
  `Edits` counts: the session's, the vaults' and the settings' (`session::Read::edits()` and the
  others, 03 §7.2) and the feature's own (a `#[lock(public)] Edits` field its `update` bumps). In
  debug builds `Memo::get` fills the entry again on every call and asserts the new value equals
  the kept one, so an input left out of a key fails every test that presents: the update tests
  that call `present` and the tours. Held by that check inside `get` and its `V: PartialEq`
  bound in debug builds.
- **Rule V14.** A memo entry is `Public` and owned (01 R5): no borrow, no secret, no reveal, and
  so is what `fill` is given (`I: Public`, which rules out a borrow, a reveal guard and a wiping
  type). A `fill` takes no `Compute`, so the memo holds nothing costly. Held by the bounds, V2,
  and `lint-leak-reveals`, which refuses a reveal's or a `SecretText`'s text reaching `Memo::get`
  (a check listed in 08 §8.1).
- An entry is kept while the last frame used it: at each frame's end the runtime drops every entry
  that frame did not use, so the memo is as large as what one frame shows. The value's type is
  part of the key, so two uses of one slot with different values never meet. The store is emptied
  at every lock and every end.

**Decision: the memo is keyed by inputs and kept across frames, not emptied after each frame
as `main`'s drawing memo was** (`docs/SPEED.md` §3.3 on `main`). Reason: a key of inputs is never
stale, and a frame drawn again works out nothing, which is what `main`'s kept values gave.
Turned down: a memo emptied each frame, which repeats the work at every frame of a scroll.

## 7. What a frame gives back

### 7.1 The `Ui` and its frame

```rust
/// In the kit's atoms. One frame's drawing context, generic over the press type.
pub struct Ui<'f, A: Pressed> { /* canvas, scale, theme, pointer state, motion store, lift */ }

impl<'f, A: Pressed> Ui<'f, A> {
    /// The runtime's only.
    pub fn new(canvas: &'f mut Canvas, theme: Theme, scale: f32, motion: &'f mut MotionStore,
               pointer: &'f Pointer<A>, ink: bool) -> Self;
    pub fn finish(self) -> FrameOut<A>;       // the runtime's only
    /// A child page draws here; its presses are lifted to this one's.
    pub fn within<B: Pressed>(&mut self, lift: fn(B) -> A, f: impl FnOnce(&mut Ui<'_, B>));
    /// A place a later Reveal or glide names (§8). Stable across frames.
    pub fn anchor(&mut self, id: Anchor);
    /// A layout fact an update reads (§7.3).
    pub fn laid(&mut self, id: Anchor, fact: Laid, to: fn(Laid) -> A);
    // controls, regions and presses are recorded through the kit's components, not by pages
}

/// What the runtime keeps of the last frame. Generic over the root's press type.
pub struct FrameOut<R: Pressed> {
    pub hits: Vec<Hit<R>>,            // rect, touch target, press, sheet layer; last drawn wins
    pub reach: Vec<usize>,            // the Tab order: indices into hits, in drawing order
    pub page: Anchor,                 // the page drawn, set by the screen template (§4)
    pub sheets: Vec<Anchor>,          // the sheets drawn over it, lowest first, set by the same
    pub anchors: Vec<(Anchor, Rect)>,
    pub regions: Vec<Scrolled>,       // id, view, extent: what a wheel or a finger moves
    pub laid: Vec<(Anchor, Laid, R)>,
    pub secret: bool,                 // §11
    pub pace: Pace,                   // the port's value (02 §1): Rest | Moving, motion wants another frame
    pub ink: Option<Ink>,             // only when the record is on (§9, V21)
}

/// A press recorded where it was drawn.
pub struct Hit<R> {
    pub rect: Rect,       // the control as drawn
    pub target: Rect,     // where a press is taken: the control grown to the touch floor
    pub press: R,         // lifted to the root's press
    pub layer: u8,        // 0 the page, 1 the lowest sheet over it, and up
}

/// A region a wheel or a finger moves.
pub struct Scrolled { pub id: Anchor, pub view: Rect, pub extent: Size }

/// The pointer and the focus as the last frame left them, kept by the runtime.
pub struct Pointer<R> {
    pub at: Option<Point>,       // while the pointer is over the window
    pub down: bool,
    pub focused: Option<R>,      // the press the keyboard is on: ringed; Enter or Space presses it
}

/// In the atoms' `focus` module: where Tab (`back` false) or Shift+Tab moves the focus among
/// the presses of `reach` in drawing order; the next or the one before, round at the ends, the
/// first or the last from none.
pub fn step<R: Clone + PartialEq>(order: &[R], at: Option<&R>, back: bool) -> Option<R>;
```

A press is taken by the top layer drawn alone, the last drawn hit first, at its `target`. Every
control that records a hit draws the focus ring round itself when its press is
`Pointer::focused`, so the focus is a press, as on `main` (`Ui::focused`), and stays on its
control when rows are added above it. The pointer and focus state is the runtime's, dropped at a
lock (§1).

`Ui` lifts each press to the root's type as it is recorded, so the frame's hit list is one type.
How the lift is stored is the kit's. A press is matched against the last frame drawn; the runtime
draws a frame after a press is handled before it matches the next press, so a press always meets
what the person saw (section 05 gives the loop).

A key goes to `reach` first: a key that moves the focus along the Tab order or presses the
focused target is taken there. A key the frame's focus does not take, and every other input,
goes to `Root::input` (section 05). Scrolling is the runtime's motion and sends no message
(§8.1).

### 7.2 Two kinds of layout fact

The inventory classes every write `main`'s drawing makes as a memo of the last frame's layout,
the guide's section jump and the word list's follow flag as real state, and some frames' work
(inventory `features/drawing-compute.md`, "What drawing writes, classed"). `next` sorts them by
who reads them.

- **Rule V15.** A layout fact read only by the next frame's drawing is kept in the motion store
  (§8), keyed by its component's `Anchor`, written and read only by kit components; no feature
  sees it. A layout fact an update decides on comes back as a message (§7.3). Neither is ever
  written into a feature's state by drawing. Held by the signatures (`draw` has no state; `Ui`
  gives a page no write but `laid`) and `lint-layers`' module rule.

**Decision: a measure read only by the next frame's drawing is the motion store's, and a measure
an update decides on comes back as a message.** Reason: a scroll extent or a keyboard lift
changes at every frame of a pan or a resize and is no fact of any feature, so a message for it
would route a dispatch per frame and put geometry in state (03 §9.2 keeps it out). Turned down:
every measure drawing finds comes back as a message, which 01 V2 first said.

| Fact on `main` | Inventory | In `next` |
|---|---|---|
| Scroll extents and clamped offsets of step columns, the guide, the word list | `drawing-compute.md`, `motion.md`, `guide.md`, `add-key.md` | motion store: the region's extent and offset |
| Keyboard lift, the Enter line, the caret's foot | `drawing-compute.md`, `typing.md` | motion store, read by the screen template |
| `pin_h` (the pin bar's height), drawn again in the same frame | `screen-frame.md`, `small-panel.md` | the screen template measures the pin bar before the page in the same frame; no memo |
| codex32's `dice_body_h`, `pane_all`, `sheet_room`, `sheet_short` | `codex-shares.md` | motion store, the worksheet organism's |
| Menu card split (`card_split_keep`) | `menu-pages.md` | motion store, the menu-page template's |
| `BackupState.drawn_check` | `backup-flow.md`, `wallet-backup.md` | neither: `present` works out the check once and gives it to the card and to Your choices in one model |
| Another frame wanted | `drawing-compute.md` | `FrameOut::pace` |
| Whether the page stood apart (`stood_apart`, the trail's `drawn_apart`) | `pane-container.md` | `Laid::Placed`, a message to the frame's `Nav` (`Addr::App`) |
| `Camera.pane_seen` (whether the viewfinder has a place beside this page) | `session-pane.md`, `scanner-qr.md` | `Laid::Room`, a message to the scan feature |
| The guide's section jump | `guide.md` | an update's `Fx::View(Reveal(anchor))` (§8.2) |
| The word list's follow flag | `add-key.md` | an update's `Fx::View(Reveal(anchor))` when the sheet opens at a mark |

### 7.3 Laid facts as messages

```rust
/// In the kit's atoms. A layout answer, never a coordinate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Laid { Placed(Placement), Room(bool), Fits(bool), Rows(u16) }
pub enum Placement { Stacked, Beside, OneColumn }
```

- **Rule V16.** A laid fact carries no coordinate: a placement, a yes or no, or a count. Held by
  the type (a new kind is a kit change, reviewed with this section).
- **Rule V17.** The runtime keeps the last fact sent for each `Anchor`; a fact that differs (or is
  the first) is routed as its message before the next event, and the frame is drawn once more.
  Facts that change again in that second frame are sent only after the next event; in debug builds
  that is an assertion ("layout settles in one frame"). Held by the runtime and a runtime test.
- A laid fact is routed as a laid fact: `refuses` never sees it, so a waiting feature still gets
  it (03 §2.2: an answer or a laid fact is never refused).
- A template reports its own facts from what its model asks: the pane container's model has
  `on_placed: Option<fn(Laid) -> A>`; a page that needs no answer leaves it `None`.

## 8. Motion between frames

### 8.1 The motion store

Section 01 K5 lets the kit keep across frames only what a component's motion needs, held by the
runtime. This is where and how.

```rust
/// In the port's values module (02 §1): a name the app gives a place it draws.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Anchor { pub scope: &'static str, pub n: u32 }

/// In the kit's atoms; owned by the runtime, lent to each frame's Ui.
pub struct MotionStore { /* entries keyed by (page Anchor, component Anchor) */ }
pub enum Motion {
    Region { offset: f32, velocity: f32, stretch: f32, extent: f32, bar_shown_ms: u64 },
    Glide { from: Rect, to: Rect, began_ms: u64 },   // the session pane, the guide's pane
    Shown { at: f32, began_ms: u64 },                 // a sheet, a menu, the dice sheet, the snackbar
    Hold { began_ms: u64 },                           // hold-to-confirm's fill
    Slide { from: u16, to: u16, began_ms: u64 },      // a segmented control's fill
    Fade { kind: Turn, began_ms: u64 },               // a page turn, a step's fade through
    Laid { measure: f32 },                            // a last-frame measure of §7.2's table
}
pub enum Turn { Page, Step }
```

Built: the store also keeps, per component anchor, a list of part heights under a key of the width,
the text size, the scale and the parts (`Ui::keep_heights`, `Ui::kept_heights`); the How blocks
write each part's measured height there while drawing and the next frame under the same key
passes over a part out of view without measuring it (V15's channel, not feature state). A wheel or a finger over a region writes its offset and `bar_shown_ms` from the frame's
clock (`MotionStore::scroll_region`), and a hold held to its end sends its press once.

- **Rule K9.** The motion store holds numbers, rectangles and times only: no text, no press, no
  model value. Held by the `Motion` enum (its fields are the kit's geometry and `u64` times). So
  it holds no secret and needs no wipe; a lock drops it with the model.
- **Rule K10.** A component's entry is keyed by an `Anchor` the page or template gives it, never
  by call order, so an entry survives a row added above it. Held by the kit's component
  signatures (a scrolled region, a sheet and a hold button take an `Anchor`).
- **Rule K11.** An entry no frame drew is dropped at that frame's end, except a page's own region
  offset, kept per page anchor for the last 32 pages (`main`'s trail holds 32 spots, inventory
  `features/flow-container.md`), so Back lands where the person was. A sheet therefore opens at
  its top each time. Held by the runtime's store and a runtime test that Back lands where the
  person was.
- **Built.** `Ui` holds the frame's time (`set_now_ms`, set by the runtime where it builds the
  `Ui`); `Ui::hold_fraction`, `Ui::shown_fraction` and the overlay scrollbar's fade read the
  store with it, and `Pointer` carries the press under the pointer and the press pressed
  (`Ui::is_hovered`, `Ui::is_pressed`). `FrameOut` carries the tooltip anchors the template left.
  `Ui::fade_later` notes the box of a refused row, link, chip or card and `Ui::fade_disabled` draws
  those boxes at the disabled content opacity once the page or sheet is drawn (`main`'s `keep_live`
  dimming, now decided by `Offered`).
  A moved frame is handed to `Ui` as the frame's time is: `Ui::set_rows`, called by the runtime
  where it builds the `Ui`, lends the rows (`KeptRows`) the runtime kept of the regions the frame
  before drew, and only when nothing but a region's motion asked for this frame; a region that
  opts in (`Region::moves`) and was drawn in the same view puts them back moved, paints the strip
  the move uncovered, sees its whole view for hits, and gives the rows it took (`MadeRows`, in
  `FrameOut::rows`) to be kept again (K16).
  The frame says what it moved to the shell: `FrameOut::slid` gives each moved region's view and
  rows, the runtime hands the biggest as `port::Frame::moved` (composed over frames the shell did
  not take, none after a whole one), and a frame only motion asked for draws nothing hovered, so a
  pointer at rest over a control, or a coast after a touch, still moves; the menu page opts in.
- Scrolling is motion, not state: a wheel notch, a pan, a flick and the stretch are handled by the
  runtime against the last frame's `regions`, change the store and send no message (inventory
  `features/motion.md`: slop, coast, notch, Reduce motion). Reason: a pan is up to 60 events a
  second of the person's view, not a fact of any feature, and 03 §9.2 keeps geometry out of
  state. Turned down: an offset in each feature's state, set by a message per pan event.
- Built: a pan (a trackpad or a finger past the slop) follows the fingers and stretches like rubber past an end; a lift coasts on at the speed the last moves saw, or springs a stretch back, advanced from the frame clock (`MotionStore::{pan_region, release_region, advance}`); Enter or Space held on a focused hold fills it and sends its press once at the end.
- Built: Reduce motion (`Settings::appearance.motion` Off) makes a sheet or menu fully in on its first frame, the overlay bar go without fading, and a pan stop at an end with no coast or stretch; a wheel notch and the arrow keys, Page Up/Down, Home and End glide a region to the offset a jump would reach (`MotionStore::glide_region`), or jump under Reduce motion.

### 8.2 What an update may ask of the view

Some updates decide where the view goes: the guide jumps to a section, the word list opens at a
mark, a step opened starts at its top, the codex32 worksheet keeps the work in view after entry
(inventory `features/guide.md`, `add-key.md`, `flow-container.md`, `codex-shares.md`).

```rust
/// In the kernel's effects (03 §4.1): to the runtime's motion store. Not answered.
pub enum ViewOrder {
    Reveal(Anchor),   // bring the anchor's place into view, gliding unless Reduce motion
    Top(Anchor),      // the region named starts at its top
}
```

- **Rule V18.** A one-shot or a timed change is decided in `update` or by the runtime's clock,
  never consumed by drawing: a jump or follow is an `Fx::View`, carried out by the runtime at the
  next frame that draws the anchor; a hold that completes is a message the runtime sends when the
  pointer has been down for the hold's time; a note's end and a fade's end are the runtime's
  clock. Held by the signatures (drawing holds no state to consume) and V15.

### 8.3 Fades and kept frames

A page turn and a step's fade through blend kept copies of whole frames (`main`'s `Fade`,
inventory `features/motion.md`). The runtime keeps them, beside the last frame, in wiping
buffers (02 P13).

- **Rule V19.** The runtime keeps one copy of a frame whose `FrameOut::secret` is set, and only
  one: the source of a step's fade through (`Turn::Step`) away from a card that shows a secret
  (F337). That copy is read only while the fade out runs (about 48 ms of `STEP_MS`, which the
  Motion speed setting does not scale), and is emptied when the fade out ends, when it is cut
  short (another change, a lock, a hide, the page left), whichever comes first. A step's fade
  through towards a card that shows a secret runs too (F337 point 3): its incoming card is drawn
  live, and no copy of it is kept. Every other kept copy of such a frame is refused: no page
  turn's source, no frost under a sheet, no moved-frame reuse. Held by the
  runtime, which reads the flag before keeping a copy, by §11, which sets it, and by the runtime
  test of 07 §8 (V19). On `main` the fade from such a card was drawn live from state (F318 point 7
  and F296 point 2); on `next` the step's state is dropped at the press (03 A29's `hide`), so the
  fade reads the kept copy instead and the card's secret leaves state at once.
- So the incoming card of the fade is drawn live each frame, and a card that shows a secret is
  never drawn live from state after the step has changed.

## 9. Drawn text after the frame is painted

- **Rule V20.** Every buffer that holds a frame's drawn text is a wiping type and is emptied once
  the frame is painted: the call list's text room is wiped at the end of each replay, as on `main`
  (`core/faraday-draw/src/canvas/list.rs`, `List::clear`); any per-band room that copies a
  call's text is wiped with it; the strings-cut-off list and the ink list are kept only when the
  frame is asked to record them. Held by the canvas's types (`Text` and the wiping buffers).
- **Rule V21.** In a shipped root no frame keeps its drawn text after it is painted. The tour
  root asks the runtime for `FrameOut::ink` (each string with its box, what `main`'s
  `drawn_texts` read, a secret's text among them marked secret), a wiping list replaced at the
  next frame and dropped at a lock. The tour runner's leak check reads it (01 R4, 07 TS15), and
  the runtime's `Seen` text reads it too, empty unless the root turns the record on (section 05).
  Held by the runtime (the record is off unless the root turns it on) and by `Runtime::record_ink`
  being in `disallowed-methods` outside the tour root, the leak crate and test support.
```rust
/// In the drawing base (`canvas`): what one frame drew, in drawing order, kept only while the
/// record is on. Its texts are `Text`, wiped when the record is dropped or replaced.
pub struct Ink(pub Vec<Inked>);
pub struct Inked {
    pub kind: InkKind,    // main's: a line of text (its face, colour, cut past its clip), an icon,
                          // a progress indicator, a badge's dot, the wordmark
    pub rect: Rect,       // the box its visible ink covers inside its clip
    pub secret: bool,     // drawn from a `Shown::secret` (K12)
}
```

`Inked` is `main`'s `canvas::Ink` with the secret mark; the record as a whole takes the name
`Ink`, since `FrameOut::ink` and the runtime's `Seen` hold one record a frame.

- The glyph and shape caches hold rasterised outlines keyed by font, size and glyph id, not
  strings. A glyph cache holds no secret beyond which glyphs were drawn, which is a side channel
  out of scope.

## 10. A note's words reach the snackbar

A note is a typed `Worded + Public` value an update returns (03 §4.4). `lift` keeps it, so every
note reaches the runtime from the top state it came out of.

- The runtime takes each `Fx::Note` at the top, keeps them in arrival order, and puts one up at
  a time for the snackbar's time (`TOAST_SHOW_MS`), a later one waiting its turn. The one up is
  `Show::note()`: `NoteUp { note: &Note, id: u32 }`.
- The product crate's `frame` words it, `note.words(show)`, into the snackbar organism's model in
  `ScreenModel::note`, and `draw` (§4) hands it to the screen template, which draws the snackbar
  above the floating action button where one stands. Its rise and fade are the store's
  `Motion::Shown` under `id`.
- A lock drops the queue with the model. A note holds no secret (`Public`).

**Decision: notes are words at the root, from a queue the runtime keeps.** Reason: a note's
timing is the runtime's clock (V18), and wording it in `present` keeps V8. Turned down: the
feature that made the note drawing its own snackbar, which would draw one per feature and lose a
note when the person moved away.

## 11. Secrets on the screen

A kit model's text says whether it is a secret, so that a frame that shows one is known and no
copy of it is kept but a step fade's source (V19). 01 §6.5 names the type.

```rust
/// In the kit's atoms: every text a kit model holds. Its variants are private.
#[derive(Clone)]
pub struct Shown<'s>(Kind<'s>);
enum Kind<'s> { Plain(&'s str), Made(Text), Secret(&'s str) }

impl<'s> Shown<'s> {
    /// Text that is no secret: a `Text` the app holds, or a fixed string.
    pub fn plain(text: impl Into<PlainRef<'s>>) -> Self;   // `From` only for `&'s Text` and `&'static str`
    /// A secret on show: a `SecretText`, a reveal's text or the passphrase part's view.
    pub fn secret(text: &'s str) -> Self;
    /// Plain text `present` made (the step heading "Step N of M", a `ShowFlow::choice` line, a
    /// filled refusal or label), held by the model for the frame. Never a secret.
    pub fn made(text: Text) -> Shown<'static>;
}
```

A kit model holds borrowed text or plain text it made, for one frame; a secret is only ever
borrowed (`Shown::secret` takes a `&'s str` and nothing owned). `Shown` is `Clone` and not `Copy`:
a model borrows it, or clones it where it must.

**Decision: `Shown` has one owned, plain-only form, `Shown::made`.** Reason: a text filled in
`present` is owned by nothing that lives for `'s`, so a model could not borrow it; the model
already lives for one frame, so the owned form keeps "for one frame", and no secret constructor
takes owned text, so a secret cannot be made plain or owned. Turned down: a frame's text room in
`Show` (`Show::keep(Text) -> &'s Text`), which needs new `unsafe` or an arena dependency.

- **Rule K12.** A kit model's text field is a `Shown`, and a secret on show (a reveal guard's
  words, typed seed words, a typed passphrase with its eye open, a vault record's password) is
  made by `Shown::secret`. Drawing one sets the frame's `secret` flag (V19); the ink record keeps
  its text marked secret, wiped like the rest, and the screens check reads both (§11.1). Held by
  the private variants (`plain` takes only a `Text` or a `&'static str`, so a borrowed secret
  cannot be made plain) and by `lint-leak-reveals`, which refuses a reveal's text turned into a
  `Text` (a check listed in 08 §8.1).
- `hide` stays immediate (03 A29): a step changed or left hides at once, and the next frame is
  drawn without the secret.

### 11.1 What the tour root's screens check reads

Section 07 §5.3 checks, at the frame after a lock, a hide, a page left and a sheet closed, that
no text of a planted secret is drawn and no kept image of a frame that showed one is still kept.

- **Rule V24.** A press that hides a secret (an eye closed, Show pressed again) is marked
  `#[hides]` on its variant of the page's press type. `#[derive(Pressed)]` gives
  `Pressed::hides(&self) -> bool` from the marks, and the runtime records `Mark::Hides` in
  `Seen::marks` (section 05) for the frame after a hiding press; 07 TS17 reads the mark from the
  press type. Held by the derive; a hiding press left unmarked is a gap the review of the
  feature's page checks.
- **Rule K16.** The kept images are the runtime's alone: the frosted copy under a sheet, a page
  turn's and a step fade's source frames, and a moved frame's reused rows. The motion store holds
  no pixels (K9). The runtime gives them read-only beside the last frame, each with the `secret`
  flag of the frame it was copied from, in wiping buffers (02 P13), so the check reads them with
  no hook in the kit (01 R4, 07 TS15). By V19 only a step fade's source may have the flag set,
  and none is left at the frame after its fade out, a lock, a hide or a page left; the check holds
  V19. Built: the frost under a sheet is made once when it opens and kept in the runtime
  (`Seen::kept`); a moved region's rows are replaced at each frame, kept only for a page with
  no sheet and no secret, and none outlives a lock, a hide or the page.

## 12. The UI kit by atomic level

### 12.1 What each level holds and may name

| Level | Holds | May name | Never |
|---|---|---|---|
| tokens | named dimensions, type roles, opacities, elevation levels, durations, easing curves; the palette: `Role` (45 roles), the scheme table (10 themes, light and dark, three contrasts), the fixed colours, the stick tones; `Theme`, `TextSize` | nothing | code beyond constants and their helpers (K2) |
| atoms | the canvas and its call list, surfaces, blending, geometry, text and fonts, shapes, icons, easing; `Ui`, `FrameOut`, hits and the Tab order, state layers and the focus ring, scrolled regions, `MotionStore`, `Shown`, `Offered`, `Laid`; every control: buttons in each `Style`, the floating action button, chips, pills, tags, badges, checkboxes, switches, segmented buttons, icon buttons, the stepper, progress, the hold button, a field's box and label, a typed line with caret and selection, rules, the wordmark and the mark, shadows and the frost | tokens, the port's display values and `Anchor`, `faraday-text`, `faraday-codec` (a QR code's modules) | the wording, the guide |
| molecules | rows and list items, kind rows and option lists, multi-select lists, chip sets, button rows, a code box, an address's lines, the passphrase field's drawing (its dots, eye and chips), word cells and the word pill, the dice and card pads, time zone rows, the appearance rows, tabs, a navigation destination with its badge, the stick pill, a status chip, a flow paragraph (bold runs), a guide block, a caution line, a QR code with its caption | atoms, tokens, the wording (a fixed label: Cancel, Show) | the guide's pages |
| organisms | the content organism (§12.3); the session pane (its rows, the Waiting band, pick rows, vault marks, row details, the scan in the pane); sheets, dialogs and the side sheet; menus; the snackbar; tooltips; ways-in; the navigation rail, the navigation bar and the top app bar; the viewfinder; the on-screen keyboard's panel; the guide's pages; the codex32 worksheet; the structure chart; the backup board; the QR copy grid; Faraday Demo's host strip; the "Demo only" card | molecules and below, the wording, the guide | another organism's internals (one organism may hold another only through its model) |
| templates | the screen template (§12.4), the pane container, the step-flow template, the menu-page template | organisms and below, the wording | a page's own drawing |

On `main`: the tokens are `core/faraday-draw/src/tokens.rs` and `faraday-ui`'s `pal` and `theme`;
the atoms are the rest of `core/faraday-draw` and `faraday-ui`'s `lib.rs`, `controls`, `press`,
`region`, `hold`, `mark`; the molecules are `faraday-ui`'s `lists`, `parts`, `roles`' list items,
`tabs`, `nav::chip`, `stick`, `appearance`, `flow`, and `faraday-core`'s `screens/parts.rs`; the
organisms are `faraday-ui`'s `sheet`, `menu`, `snackbar`, `tooltip`, `keyboard`, `nav`, `how`,
`demo`, and `faraday-core`'s `screens/session_pane/`, `ways_in`, `scan_frame.rs`, the worksheet,
chart and board drawing; the templates are `faraday-core`'s `screens/hub_layout/`, the flow
container (`flow.rs`, `flow/`) and `screens/menu_page.rs`, and the frame of `screens/mod.rs`
(inventory `features/ui-controls.md`, `ui-text.md`, `ui-sheets.md`, `ui-navigation.md`,
`ui-regions.md`, `ui-theme.md`, `ui-guide-text.md`, `keyboard.md`, `draw-base.md`, `tokens.md`,
`layout-rules.md`, `sheets-parts.md`). `docs/DESIGN.md` §4 lists the components under these
levels once it is carried (08 §4.4).

- Built: the address, caution, code box, destination, guide block, kind rows, multi-select, pad, QR and word molecules (`kit/faraday-molecules/src/`), each a model and a `draw` over its box.
- Built: the organisms `nav` (the navigation bar, the rail, the top app bar with its close button, Home's head with the mode pill, and the sidebar), `tooltip` (`Tips` and `TipModel`), `demo_only` (the "Demo only" card), and the content organism's address and caution blocks (`kit/faraday-organisms/src/`).

- **Rule K13.** A sheet of `main`'s `screens/sheets.rs` (Lock, Power, Idle, Devices, Scan, QR, New
  input, Guide, secret out, wallet confirm) is a page of the feature that owns it, drawn on the
  sheet organism. The kit holds the sheet, not what a sheet says. Held by K3.
- The palette and the type roles are tokens so that the adapters and `faraday-paper` (01 §6.3)
  can name them with the dimensions; the kit above tokens reads a colour only through
  `Paint::Role` or a named fixed colour.

### 12.2 Material 3 with Faraday's look

- **Rule K14.** Every component follows Material 3 (`main`'s F91, F92), drawn in Faraday's look:
  colours are palette roles, a dimension is a name in the tokens crate, elevation is Material's
  levels 1 to 5 with no shadow at High contrast. A difference from Material needs a decision, and a
  screen's review names the Material page it was checked against. Held by `lint-tokens` (a number
  used as a dimension or a colour outside the tokens crate; its counts only fall) and by review.
- Cobalt is the default theme; the floating action button takes Material 3's primary style; a
  control held down may read at 4:1 (6.5:1 at High) and a selected one about 3.8:1 in Dark
  Standard while pressed, full thresholds otherwise (the owner's calls on `main`).
- The theme is the person's (`view.settings`); the runtime maps it to the kit's `Theme` when it
  builds the `Ui`. No `present` and no page chooses a colour by theme.

### 12.3 The content organism: how a page draws controls

A page uses only templates and organisms (01 K6), and a step's card, a sheet's body and a
pane's rows are made of controls. The content organism draws them from data.

```rust
/// In the kit's organisms. A column of blocks, drawn in order in the width given.
pub struct Content<'s, A>(pub Vec<Block<'s, A>>);
pub enum Block<'s, A> {
    Heading(Shown<'s>),
    Para(FlowText<'s>),                 // bold and plain runs
    Fact { label: Shown<'s>, value: Shown<'s>, mono: bool },
    Rows(Vec<RowModel<'s, A>>),         // list items: icon, headline, line, tags, chevron, press
    Choose(ChoiceModel<'s, A>),         // one or several of a list; segmented
    Appearance(AppearanceModel<'s, A>), // Settings' Theme chips, Mode and Contrast rows
    Segmented(TextSizeModel<'s, A>),    // one labelled row of segmented choices; `dim` draws it dimmed, taking no press
    Zone(TimeZoneModel<'s, A>),         // the time zone as a row that opens the list, or as steppers
    Field(FieldModel<'s, A>),           // label, the typed line, its caret, an action in the field
    Pass(PassModel<'s, A>),             // the passphrase field part's drawing, from its read view
    Words(WordGridModel<'s, A>),
    Pad(PadModel<A>),                   // dice, cards, coin
    Code(QrModel<'s>),
    Address(Shown<'s>),
    Buttons(Vec<ButtonModel<'s, A>>),
    Hold(HoldModel<'s, A>),
    Busy(BusyModel<'s>),
    Caution(Shown<'s>),
    Part(Organism<'s, A>),              // a bespoke organism (09 §3.3)
}

/// In the kit's atoms (`button`).
pub struct ButtonModel<'s, A> { pub label: Shown<'s>, pub style: Style, pub offered: Offered<A> }
pub enum Style { Filled, Tonal, Outlined, Text, Elevated, Error }
```

- `Block::Pass` is filled from the passphrase part's read view (01 A2, A3): the dot count, the
  eye, the chips, and the text only while the eye is open. Only the part's crate names its state;
  the presentation kernel maps the view to `PassModel`, the text through `Shown::secret`.
- Built: the molecules `pass` (the passphrase field: dots, or the revealed text through `Shown::secret`; its view is private so a plain text cannot be made revealed), `appearance` with `chips` (theme chips, Mode and Contrast, Text size), `stick_pill`, `tabs`, `options` (`ChoiceModel`) and `time_zone` each draw from their box, `draw(m, ui, (x, y, w))` returning the height.
- **Rule K15.** A page shows a control only as a model the content organism or another organism
  draws; it calls no atom or molecule. Each organism re-exports its model's field types (K7).
  Held by K6 and K7.
- What `main` draws by hand on a page becomes an organism with its own model: the codex32
  worksheet (`features/codex-shares.md`), the structure chart and the wallet cards
  (`wallets.md`), the backup board (`backup-board.md`), the dice and card pads (`randomness.md`),
  the keyboard (`keyboard.md`), the viewfinder (`scanner-qr.md`), QR codes and their copy grid
  (`scanner-qr.md`, `public-files.md`), guide pages (`guide.md`, `ui-guide-text.md`), Faraday
  Demo's host strip and the "Demo only" card (`practice-demo.md`). A new one is a kit change and a
  row of DESIGN §4.
- Built: the keyboard (its layouts and keys), the ways in, the viewfinder, the QR copy grid and a row's details (`kit/faraday-organisms/src/{keyboard,ways_in,viewfinder,qr_grid,row_details}.rs`), each a model and a `draw` over its box.
- Built: the guide sheet (`kit/faraday-organisms/src/guide.rs`, `draw_sheet`: the tabs, the page's pinned title, the Find field, the page, the contents or the How tab in a scrolled region, Contents and Done at the foot, held or in its box; the target section's place registered with `Ui::anchor`). Open: the pane's own look, Find's matching (the feature's).
- Built: the How blocks (`kit/faraday-organisms/src/how.rs`, `how::draw`): blocks as list items, the cross-check, constructions with their tables, `PageLook`, all from a model of data; each part's height kept in the motion store, so a part out of view is not measured on the next frame at the same width and text size, and `how::draw_found` says where the target part begins.
- Built: the step-flow template (`kit/faraday-templates/src/flow.rs`, `flow/compact.rs`): the chip and note, the heading and title, the card, the forward action (at the card's end; on the small panel in the pin bar, `FLOW_PIN_H`), Your choices with the lead row and the current row's tint (beside the card where the room holds the pair, else under it), a pick or the take-away in the pane's place, `under`, the foot (refusal, busy line held still under Reduce motion, Back and Cancel), the asking dialog over the page room, the raised card (outlined at High contrast alone), and on the small panel the page per step. Not built: Your choices as the bar's button and sheet on the small panel; Back and Cancel in the top bar and the dialog over the whole window are the screen frame's to take over.
- Built: the screen template (`kit/faraday-templates/src/screen.rs`, `screen/pills.rs`): the navigation bar, the rail or the sidebar beside the page, the top app bar, the keyboard (at the small panel's foot, over the wide page), the tooltip, the stick pills (strip or in the bar), the busy line, the sheets and the note; and the menu page (`menu_page.rs`): section cards, cards of different heights, the box of rows, a typed search's results, Home's lead, the search row pinned or scrolling by the room, the raised padded tile in a pair or stack. Not built: Back and Cancel in the top bar, the dialog over the whole window, the Pi menu's Scan button and head (the feature's model).

### 12.4 The templates

Each template takes its own model and places everything in it. A page's model says what; the
template says where (K8).

- **The screen template** (`main`'s `screens/mod.rs` frame, `features/screen-frame.md`): the
  navigation rail or bar and their destinations, the top app bar with its title and way back, the
  page area, the sheets over it with the scrim (only the top one takes presses: `main`'s
  `clear_hits`), the snackbar, tooltips, the on-screen keyboard's place and the caret kept above
  it, and on a small panel the pin bar (measured before the page, in the same frame) and the page
  per step (`features/small-panel.md`). **Decision: the screen frame is a template.** Reason: it
  places the root's page and every sheet, which is a template's work, and the root's `frame` then
  uses only templates and organisms like any other. Turned down: the frame as the root's own
  drawing, which would put atoms in a root.
- **The pane container** (`docs/HUB-LAYOUT.md` on `main`, `features/pane-container.md`): a model
  of `Panes { left, tile, stacks, narrow }` as on `main`, the left pane's content (the session
  pane's model in its mode, or a pane the page gives), the tile's content and the guide's pane.
  It owns where the pair stands, the stack under This session's card, the level ends, the
  dividers and the time of a change of page under the Motion and Speed settings. It reports
  `Laid::Placed` where the model asks (§7.3).
  Built (`panes.rs`): every tile rule (card, fill, detail, equal), the pane at the right of the card, Guide's pane's place and room, the stack under This session's card, the level ends, `pair_card` (drawn to the foot measured this frame, so no kept foot), the dividers, `Laid::Placed` and `turn_at`, the turn's progress from the frame's time; the page gives its own pane, the row details, Guide's pane, `on_placed` and the tile's scroll in `Parts`, not in `Panes`.
  Built (`session_pane.rs`): the groups, the Waiting band, rows with a vault's mark and Save, the pane in a page's mode, the scan in the pane and `draw_pick`; the small panel's tinted box is not.
  Built (`snackbar.rs`): the close action and `draw_timed`, which rises on the store's `Motion::Shown` and fades over the last `MOTION_SHORT4` of the note's time the runtime counts.
  Built (`screen.rs`, `screen/selftest.rs`, `menu_page.rs`): the notices that stay (`snackbar::draw_held`, the first at the window's foot and the rest above it, the note above them), the wide window's keyboard button, the held frame (no side, top bar or navigation bar, the page the whole window, the holder's way back), the failed self-test frame, the pills' room in the bar taken by the frame itself (`draw_top_less`), the page's own left pane through the menu page (`MenuPageModel::own`) and a typed search on a wide page with no pane standing alone (`draw_alone`). Not built: `drop_edge` (Demo's drop target).
- **The step-flow template** (`docs/FLOW-CONTAINER.md` on `main`, `features/flow-container.md`):
  `FlowModel` (§5). It owns the card's place, a tall card's scroll, the step heading, the fade
  through (`STEP_MS`, a token), Your choices as the pane on the wide layout and as the bar's button
  and sheet on every other, and the foot with its refusal. Only the current step's card is drawn.
- **The menu-page template** (`docs/MENU-PAGE.md` on `main`, `features/menu-pages.md`): the page's
  name, its search bar and pills, its sections (tiles, cards, a block across), the column count by
  the room, the card split and the one-column and small-panel forms. A menu page's width and
  columns are the template's alone (08 §3.2).

## 13. Drawing stays free of costly work

The rule, from the inventory's settling of `main`'s writes while drawing (inventory
`features/drawing-compute.md`): draw reads a `ScreenModel`; layout measures return as messages
or stay in the motion store (§7); work goes through the compute path, its result kept by its
inputs.

```rust
/// In the kernel (`cx`): a costly value kept on a state by the inputs it came from.
/// Public, so a state field of it is marked `#[lock(public)]`.
#[derive(Public)]
pub struct Derived<K: Public + Eq, V: Public> { key: Option<K>, value: Option<V> }
impl<K: Public + Eq, V: Public> Derived<K, V> {
    pub fn get(&self, key: &K) -> Option<&V>;   // present reads; None when missing or stale
    pub fn put(&mut self, key: K, value: V);    // update: worked out with cx's Compute, or a job's answer
}
```

- **Rule V22.** Costly work (anything that takes `Compute`) runs only in `update` with `Ctx`'s
  token or in a job, and its result is state, kept by the inputs it was worked out from. `present`
  reads the kept value and, where it is missing or stale, shows the step as busy; it never works
  it out. A feature works it out when its inputs change: the press that changes them, the
  `Notice` that says a shared input changed (03 §2.4), or the answer of the job that made it; it
  then `put`s it. A derivation from a key is a `faraday-keys` public derivation over
  `view.session.keys()` in `update`, so the key's handle is the key and no key material is.
  Held by V2 (no `Compute` in `present`) and `costs-lint`.
- A costly secret on show (Explore's xprv or WIF) is never `Derived`, whose bounds refuse it: it
  lives in a `#[lock(shown)]` wiping field of the feature's state, worked out on the press that
  shows it and wiped by `hide`.
- **Rule V23.** Kit layout arithmetic (measuring, wrapping, fitting text, placing panes) is drawing
  work, not costly work: it takes no token, is bounded by what the model holds, and is held by the
  frame's time on the Pi (16.7 ms) and by the timing of a late-flow frame (08 §3.3). The canvas
  caches a text's line breaks keyed by (a hash of the text, the wrap basis), holding line offsets
  only, and never caches the lines of a `Shown::secret` text; nothing of it is kept in state.

How `main`'s cases land (inventory file in brackets):

- **Explore** (`address-check-explore.md`): the public reading (xpub, script, address at the path)
  and the address list are `Derived` on Explore's state, keyed by the key's handle, path, script,
  network and the session's `Edits`, and by the account and the count derived; worked out on the
  press and on `Notice::Shared`. A private key it shows is the wiping field above.
- **Silent payments** (`silent-payments.md`): what the page shows of its key (two derivations and
  an address) is `Derived`, keyed by the key's handle, label, wallet, record, the session's
  `Edits` and the network.
- **Restore** (`restore-wallet.md`): the pool of shares and its merge are `Derived` by the chosen
  shares, the session's and the vaults' `Edits`.
- **`public_qr`** (`public-files.md`, `scanner-qr.md`): each public file's codes are `Derived` by
  the file's SHA-256, put when the file is made.
- **Spend from backup** (`spend-backup.md`), which `main` recomputes every frame with no token and
  no cache, and which `next` must not repeat: the session reads each Inbox file once, when it comes
  in (a key file read with its BIP 129 signature checked, a share file parsed), and keeps the
  result with the file, keyed by its SHA-256 (`main`'s `docs/SPEED.md` §10, "what the Inbox gives
  is kept with the Inbox"). The person's text of a file is kept only for a file of a kind that
  holds no secret, and is never in `FileView`. Spend from backup reads the result through
  `session::Read::inbox()`'s `FileView` (03 §7.2), merges the shares in its `update` on
  `Notice::Shared(FileCame)` and `FileGone` into a `Derived` keyed by the session's and the vaults'
  `Edits` and its chosen shares, and counts each wallet's keys toward its quorum in a memo (§6).
- Receive addresses stay kept on the wallet by the session, as on `main`; the shown line is a
  lookup.

## 14. What holds each rule

| Rule | What it says, short | Held by |
|---|---|---|
| V5 | present then draw, once a frame; the model never outlives it | `Root::frame`; the compiler (`'s`) |
| V6 | presses are their own type; typed input is withheld | `Pressed`; the `Message` derive; `lint-secrets` |
| V7 | no re-export of the wording | `lint-layers` |
| V8 | an update forms no text | `Show` and `Worded::words(&Show)`; `disallowed-methods` on `Show::new`; the module rule |
| V9, V10 | no geometry in a model; the size class only by `present`'s choice of content | the kit's model types; `lint-layers` |
| V11 | a control's enabled state from the one gate | `Offered`; update tests |
| V12 | every flow's model filled once | the blanket impls; `lint-layers`; `disallowed-methods` on `FlowModel::new`; K8 |
| V13, V14 | a memo key holds every input; entries and inputs Public, no `Compute` | the refill check inside `Memo::get`; the bounds; `lint-leak-reveals` |
| V15 to V17 | layout facts to the next frame or as messages; no coordinate; settle in one frame | the signatures; `Laid`; the runtime; a runtime test |
| V18 | one-shot and timed changes outside drawing | `Fx::View`; the runtime's clock |
| V19, K12 | no kept copy of a frame that shows a secret but a step fade's source, emptied when its fade out ends; secret text is marked | `FrameOut::secret`; `Shown`'s constructors; `lint-leak-reveals` |
| V20, V21 | drawn text wiped after paint; the ink record only for tours | the canvas's wiping types; the runtime; `disallowed-methods` on `Runtime::record_ink` |
| V22, V23 | costly work in update or a job, kept by inputs; layout arithmetic is drawing | V2; `costs-lint`; `Derived`'s bounds; timings |
| K9 to K11 | the motion store: numbers only, keyed by anchor, bounded | `Motion`; the kit's signatures; a runtime test |
| V24, K16 | hiding presses marked; kept images the runtime's, flagged | `#[derive(Pressed)]`; `Mark::Hides` in `Seen::marks`; the runtime; the screens check (07 §5.3) |
| K13, K15 | sheets are features' pages; pages draw controls only as models | K3; K6; K7 |
| K14 | Material 3 in Faraday's look | `lint-tokens`; review |

## Questions for later sections

- Section 07: the tests this section names: the runtime's laid-facts settling (V17), the memo's
  refill check inside `Memo::get` (V13) run by the update tests that present and by the tours,
  and the runtime test that Back lands where the person was (K11).
