# The specification of Faraday next

**Kind:** Spec
**Open:** Not built.

The specification fills in `docs/plan/ARCHITECTURE-PLAN.md` from the inventory of `main`
(`docs/migration/inventory/`): the layers and what each may depend on, the port, the
application, presentation and the UI kit, the runtime, the composition roots, the tests, the
conventions, the skeleton manifest and the method of migration. Each section is a Spec with an
`**Open:**` line; later sections use the names and terms of earlier ones. Open the section that
answers the question and read only the part needed.

| Section | What it decides |
|---|---|
| [01 The layers](01-layers.md) | Every layer, what it may and may never depend on, which crates sit in each, how features reach one another, where key material may live and how it crosses layers, and the lint or compiler rule that holds each |
| [02 The port](02-port.md) | The one contract between app and shells: events in, effects out, storage and the camera behind it |
| [03 The application layer](03-application.md) | Reducers with effects, step flows as typed state machines, and worker answers as messages |
| [04 Presentation and the UI kit](04-presentation.md) | `present` and `draw`, and the kit's components by atomic level |
| [05 The runtime](05-runtime.md) | The queue, the memo, the workers, the lock and the restart |
| [06 The composition roots](06-roots.md) | The device app, Faraday Demo and the Starter, and the features each holds |
| [07 The test architecture](07-tests.md) | One invariant per test, the layer that owns each rule, and the test-header lint |
| [08 Conventions for next](08-conventions.md) | Its `CLAUDE.md`, its documents and its decisions |
| [09 The skeleton manifest](09-manifest.md) | Every crate and module, its public types and traits, and its function signatures |
| 10 The migration method | How each of `main`'s modules is copied, ported, rewritten or dropped |
