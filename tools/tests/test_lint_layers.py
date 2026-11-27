"""Tests of tools/lint-layers.py: each rule has a fixture workspace that breaks it, which the lint
must refuse under that rule's number, and the same workspace mended, which must pass whole."""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fixture import Crate, Workspace, app, kit, load  # noqa: E402

ll = load("lint-layers")


def dom(name, **kw):
    return Crate(name, "domain", **kw)


class Base(unittest.TestCase):
    def rules(self, crates):
        ws = Workspace(crates)
        self.addCleanup(ws.close)
        _, items = ll.run(ws.meta, ws.root)
        return items

    def refused(self, rule, crates, where=None):
        items = self.rules(crates)
        hit = [i for i in items if i[0] == rule and (where is None or where in i[1])]
        self.assertTrue(hit, f"{rule} not refused; got {items}")

    def clean(self, crates):
        self.assertEqual(self.rules(crates), [])


class Metadata(Base):
    def test_crate_without_layer(self):
        """Invariant: A workspace crate that declares no layer is refused.
        Regression: A new crate lands with no `layer` and no rule applies to it.
        Owner: tool"""
        bad = Crate("a", "domain")
        bad.md = {}
        self.refused("§1", [bad], "a")
        self.clean([dom("a")])

    def test_unknown_key_and_misplaced_key(self):
        """Invariant: A metadata key the table does not allow, or one on the wrong layer, is refused.
        Regression: A typo such as `no-std` or `owner` on a kit crate passes and holds nothing.
        Owner: tool"""
        self.refused("§1", [dom("a", nostd=True)])
        self.refused("§1", [dom("a", owner=True)])
        self.refused("§1", [dom("a", wording="yes")])
        self.clean([dom("a", wording=True)])

    def test_kit_needs_level_and_app_needs_role(self):
        """Invariant: A kit crate names its level and an application crate its role.
        Regression: A crate without them is checked against no row of the table.
        Owner: tool"""
        self.refused("K1", [Crate("k", "kit")])
        self.refused("§1", [Crate("a", "app")])

    def test_renamed_workspace_dependency(self):
        """Invariant: No workspace dependency is renamed.
        Regression: A rename hides a crate's path from the module rule, which reads exact paths.
        Owner: tool"""
        a, b = dom("a", deps=["b"], rename={"b": "bee"}), dom("b")
        self.refused("09 §1.2", [a, b])
        self.clean([dom("a", deps=["b"]), b])

    def test_no_std_crates_carry_the_key_and_the_attribute(self):
        """Invariant: A crate that is `no_std` on `main` says so, and its root has the attribute.
        Regression: The metadata says `no_std` while the root drops the attribute.
        Owner: tool"""
        self.refused("D2", [dom("faraday-text")])
        ok = dom("faraday-text", no_std=True)
        self.clean([ok])
        w = Workspace([ok])
        self.addCleanup(w.close)
        (Path(w.root) / "domain/faraday-text/src/lib.rs").write_text("")
        _, items = ll.run(w.meta, w.root)
        self.assertTrue([i for i in items if i[0] == "D2" and "#![no_std]" in i[2]])


class Edges(Base):
    def test_domain_edges(self):
        """Invariant: A domain crate depends on domain crates and the tokens only.
        Regression: A domain crate takes a type from the port, the kit or the application.
        Owner: tool"""
        self.refused("D1", [dom("a", deps=["p"]), Crate("p", "port")])
        self.refused("§6.3", [dom("a", deps=["k"]), kit("k", "atoms")])
        self.clean([dom("a", deps=["k"]), kit("k", "tokens")])

    def test_port_edges(self):
        """Invariant: The port depends on faraday-text and faraday-crypto and nothing else.
        Regression: The port grows a dependency on another domain crate or on the kit.
        Owner: tool"""
        self.refused("P2", [Crate("p", "port", deps=["k"]), kit("k", "tokens")])
        self.refused("P5", [Crate("p", "port", deps=["d"]), dom("d")])
        self.clean([Crate("p", "port", deps=["faraday-text"]), dom("faraday-text", no_std=True)])

    def test_kit_levels(self):
        """Invariant: A kit crate depends on lower levels, the tokens depend on nothing and an
        edge between crates of one level is one the manifest names.
        Regression: A molecule takes an organism, or a level grows a sideways edge.
        Owner: tool"""
        self.refused("K1", [kit("m", "molecules", deps=["o"]), kit("o", "organisms")])
        self.refused("K1", [kit("a", "atoms", deps=["b"]), kit("b", "atoms")])
        self.refused("K2", [kit("t", "tokens", deps=["a"]), kit("a", "atoms")])
        self.clean([kit("m", "molecules", deps=["a"]), kit("a", "atoms")])

    def test_kit_names_no_app_type(self):
        """Invariant: No kit crate depends on the application, the runtime, an adapter or a root.
        Regression: An organism takes a feature's state type.
        Owner: tool"""
        self.refused("K3", [kit("o", "organisms", deps=["f"]), app("f", "kernel")])

    def test_kernel_and_parts(self):
        """Invariant: The kernel depends on no feature and a part on no kit crate or feature.
        Regression: The kernel names a feature's type, or a part draws with the kit.
        Owner: tool"""
        self.refused("A1", [app("k", "kernel", deps=["f"]), app("f", "feature")])
        self.refused("A2", [app("p", "part", deps=["f"]), app("f", "feature")])
        self.refused("A2", [app("p", "part", deps=["m"]), kit("m", "molecules")])
        self.clean([app("p", "part", deps=["k", "q"]), app("k", "kernel"), app("q", "part")])

    def test_feature_never_depends_on_feature(self):
        """Invariant: A feature crate never depends on another feature crate.
        Regression: One feature reaches into another's state through a direct dependency.
        Owner: tool"""
        self.refused("A5", [app("a", "feature", deps=["b"]), app("b", "feature")])
        self.refused("A5", [app("a", "feature", dev=["b"]), app("b", "feature")])
        self.clean([app("a", "feature"), app("b", "feature")])

    def test_only_composites_and_products_depend_on_features(self):
        """Invariant: Only a composite or a product depends on a feature, and only a root or a tool on a product.
        Regression: The kernel or a runtime crate names a feature, or a composite names a product.
        Owner: tool"""
        self.refused("A6", [Crate("r", "runtime", deps=["f"]), app("f", "feature")])
        self.refused("A6", [app("c", "composite", deps=["p"]), app("p", "product")])
        self.clean([app("c", "composite", deps=["f"]), app("f", "feature")])

    def test_owner_crates_are_named_by_owners_products_and_tools(self):
        """Invariant: An `owner` crate is named only by another owner, a product or a tool.
        Regression: A feature or an area composite depends on the session's crate.
        Owner: tool"""
        o = app("o", "feature", owner=True)
        self.refused("A8", [app("c", "composite", deps=["o"]), o])
        self.clean([app("c", "composite", deps=["o"], owner=True), o])

    def test_feature_kit_edges(self):
        """Invariant: A feature depends on the kit's templates and organisms only.
        Regression: A feature takes the molecules or atoms and draws by hand.
        Owner: tool"""
        self.refused("K6", [app("f", "feature", deps=["m"]), kit("m", "molecules")])
        self.clean([app("f", "feature", deps=["m"]), kit("m", "organisms")])

    def test_runtime_edges(self):
        """Invariant: The runtime depends on the kernel, the port, the domain and the kit's atoms, no feature and no presentation kernel.
        Regression: The runtime names the presentation kernel or a template.
        Owner: tool"""
        self.refused("R1", [Crate("r", "runtime", deps=["p"]), app("p", "present")])
        self.refused("R1", [Crate("r", "runtime", deps=["t"]), kit("t", "templates")])
        self.clean([Crate("r", "runtime", deps=["k", "a"]), app("k", "kernel"), kit("a", "atoms")])

    def test_application_and_adapters_do_not_meet(self):
        """Invariant: No application crate depends on an adapter or a root, and no adapter on the application or the runtime.
        Regression: A storage adapter names an app type.
        Owner: tool"""
        self.refused("P3", [app("f", "feature", deps=["a"]), Crate("a", "adapter")])
        self.refused("P3", [Crate("a", "adapter", deps=["k"]), app("k", "kernel")])
        self.refused("P3", [Crate("a", "adapter", deps=["r"]), Crate("r", "runtime")])
        self.clean([Crate("a", "adapter", deps=["b", "t"]), Crate("b", "adapter"), kit("t", "tokens")])

    def test_process_adapters(self):
        """Invariant: A disk or grant process depends on the domain only, and nothing depends on it.
        Regression: The storage adapter links the disk process instead of speaking to it over a pipe.
        Owner: tool"""
        proc = Crate("d", "adapter", process=True)
        self.refused("S2", [Crate("s", "adapter", deps=["d"]), proc])
        self.refused("S2", [Crate("d", "adapter", deps=["p"], process=True), Crate("p", "port")])

    def test_roots(self):
        """Invariant: A root never depends on another root, and a root that ships names its product, the runtime, the port and its adapters.
        Regression: The Starter's root takes the Demo's root, or a root names a feature.
        Owner: tool"""
        prod = app("p", "product")
        self.refused("S3", [Crate("a", "root", deps=["b"]), Crate("b", "root")])
        self.refused("S5", [Crate("a", "root", deps=["p", "f"]), prod, app("f", "feature")])
        self.refused("S5", [Crate("a", "root"), prod])
        self.refused("S5", [Crate("a", "root", deps=["p", "q"]), prod, app("q", "product")])
        self.clean([Crate("a", "root", deps=["p", "r"]), prod, Crate("r", "runtime")])

    def test_tools(self):
        """Invariant: Nothing that ships depends on a tool except in its dev-dependencies, and a tool depends on no root.
        Regression: A shipped crate links a test allocator.
        Owner: tool"""
        t = Crate("t", "tool")
        self.refused("T1", [dom("a", deps=["t"]), t])
        self.refused("T1", [Crate("t", "tool", deps=["r"]), Crate("r", "root", ships=False)])
        self.clean([dom("a", dev=["t"]), t])
        self.clean([Crate("r", "root", ships=False, deps=["t"]), t])

    def test_dev_edges(self):
        """Invariant: A crate's dev-dependencies are its own layer or below and test support, nothing above.
        Regression: A domain crate's test pulls in the application.
        Owner: tool"""
        self.refused("D1", [dom("a", dev=["k"]), app("k", "kernel")])
        self.clean([dom("a", dev=["b"]), dom("b")])

    def test_wording_crates(self):
        """Invariant: A wording crate depends only on faraday-text, faraday-crypto, faraday-bip and the other wording crate.
        Regression: The strings crate takes a parser, which then depends on its own wording.
        Owner: tool"""
        self.refused("§2.2", [dom("w", wording=True, deps=["x"]), dom("x")])
        self.clean([dom("w", wording=True, deps=["faraday-text"]), dom("faraday-text", no_std=True)])

    def test_boundary_crates(self):
        """Invariant: A boundary crate depends on no input parser and no crate that depends on one.
        Regression: The key store takes the registry of key forms, as it did on `main`.
        Owner: tool"""
        parser = dom("faraday-intake")
        self.refused("§2.3", [dom("k", boundary=True, deps=["faraday-intake"]), parser])
        self.refused("§2.3", [dom("k", boundary=True, deps=["m"]), dom("m", deps=["faraday-intake"]), parser])
        self.clean([dom("k", boundary=True, deps=["m"]), dom("m")])

    def test_wording_and_boundary_crates_say_so(self):
        """Invariant: The strings and guide crates say `wording` and the keys and vault crates say `boundary`, so the rules that read the keys apply to them.
        Regression: The key is dropped from a crate and its edge rules stop applying.
        Owner: tool"""
        self.refused("§2.2", [dom("faraday-strings")])
        self.refused("§2.3", [dom("faraday-keys")])
        self.clean([dom("faraday-strings", wording=True), dom("faraday-keys", boundary=True)])

    def test_cycles(self):
        """Invariant: Parts, adapters and kit levels have no dependency cycle.
        Regression: Two parts embed each other.
        Owner: tool"""
        self.refused("A2", [app("p", "part", deps=["q"]), app("q", "part", deps=["p"])])
        self.refused("S1", [Crate("a", "adapter", deps=["b"]), Crate("b", "adapter", deps=["a"])])

    def test_graphs_of_roots(self):
        """Invariant: No tool is in a shipping root's graph, no online crate in the device root's, and no product's root holds another product.
        Regression: A tool or the strip reaches the device through a crate the root names.
        Owner: tool"""
        prod, run = app("p", "product", deps=["c"]), Crate("r", "runtime")
        self.refused("T1", [Crate("a", "root", deps=["p", "r"]), prod, run, app("c", "composite", dev=[], deps=["x"]), Crate("x", "app", role="kernel", deps=["t"]), Crate("t", "tool")])
        online = app("c", "composite", online=True)
        self.refused("S8", [Crate("a", "root", offline=True, deps=["p", "r"]), prod, run, online])
        self.clean([Crate("a", "root", deps=["p", "r"]), prod, run, online])
        p2 = app("q", "product")
        self.refused("S7", [Crate("a", "root", deps=["p", "r"]), Crate("b", "root", deps=["q", "r"]), prod, p2, run, app("c", "composite", deps=["q"])])

    def test_device_composite_names_no_product_or_online_crate(self):
        """Invariant: The device composite holds no online crate, no Starter feature and no product.
        Regression: The strip is added to the device composite instead of the online one.
        Owner: tool"""
        strip = app("faraday-strip", "feature", online=True)
        self.refused("S10", [app("faraday-device", "composite", deps=["faraday-strip"]), strip])
        self.clean([app("faraday-device", "composite", deps=["f"]), app("f", "feature")])


class Sources(Base):
    def test_every_crate_inherits_the_workspace_lints(self):
        """Invariant: Every crate inherits the workspace lints but those on the unsafe list, which set their own.
        Regression: A crate opts out of `unsafe_code = "forbid"` without being on the list.
        Owner: tool"""
        self.refused("U1", [Crate("a", "domain", lints="")])
        self.clean([Crate("faraday-crypto", "domain", lints='[lints.rust]\nunsafe_code = "deny"\n', no_std=True)])
        self.refused("U2", [Crate("faraday-crypto", "domain", lints="", no_std=True)])

    def test_unsafe_list_excludes_the_application(self):
        """Invariant: No crate of the application, the kit, the port or the runtime is on the unsafe list.
        Regression: A feature is added to the list to hold an unsafe block.
        Owner: tool"""
        deny = '[lints.rust]\nunsafe_code = "deny"\n'
        self.refused("U2", [Crate("faraday-window", "app", role="feature", lints=deny)])

    def test_crate_root_denies_panics(self):
        """Invariant: The root of every port, application and runtime crate denies unwrap, expect and panic outside tests.
        Regression: A new crate is written without the attribute.
        Owner: tool"""
        a = app("a", "kernel")
        w = Workspace([a])
        self.addCleanup(w.close)
        (Path(w.root) / "app/a/src/lib.rs").write_text("pub fn f() {}\n")
        _, items = ll.run(w.meta, w.root)
        self.assertTrue([i for i in items if i[0] == "P22"])
        self.clean([a])

    def test_io_in_domain_library_code(self):
        """Invariant: A domain crate's library code names no file, network, process, environment, thread or clock path, but its tests may.
        Regression: A parser reads a file or the clock instead of taking it as an argument.
        Owner: tool"""
        bad = 'use std::fs;\npub fn f() { let _ = fs::read("x"); let _ = std::time::Instant::now(); }\n'
        self.refused("D1", [dom("a", files={"src/lib.rs": bad})])
        self.refused("D1", [dom("a", files={"src/lib.rs": "use std::time::SystemTime;\npub fn f() { SystemTime::now(); }\n"})])
        self.refused("D1", [dom("a", files={"src/lib.rs": "use std::{env, thread};\nfn f() { thread::spawn(|| {}); env::var(\"A\"); }\n"})])
        good = '#[cfg(test)]\nmod tests { use std::fs; #[test] fn t() { let _ = fs::read("x"); } }\n'
        self.clean([dom("a", files={"src/lib.rs": good})])
        self.clean([dom("a", files={"src/lib.rs": '#[cfg(test)]\nmod tests;\n', "src/tests.rs": "use std::env;\n"})])
        self.clean([dom("a", files={"src/lib.rs": '// std::fs::read\npub const S: &str = "std::net";\n'})])

    def test_no_reexport_of_the_wording(self):
        """Invariant: No crate re-exports the wording crate under another path.
        Regression: A kit crate writes `pub use faraday_strings::EN` and a module names the wording through it.
        Owner: tool"""
        w = dom("faraday-strings", wording=True)
        bad = kit("k", "organisms", deps=["faraday-strings"], files={"src/lib.rs": "pub use faraday_strings::EN;\n"})
        self.refused("V7", [w, bad])
        ok = kit("k", "organisms", deps=["faraday-strings"], files={"src/lib.rs": "use faraday_strings::EN;\n"})
        self.clean([w, ok])

    def test_kit_reexports_are_listed(self):
        """Invariant: A kit crate's `pub use` of a lower level is named in its `reexports`.
        Regression: An organism re-exports a molecule's type that its metadata does not list.
        Owner: tool"""
        atoms = kit("faraday-atoms", "atoms")
        src = {"src/lib.rs": "pub use faraday_atoms::Ui;\n"}
        self.refused("K7", [atoms, kit("o", "organisms", deps=["faraday-atoms"], files=src)])
        self.clean([atoms, kit("o", "organisms", deps=["faraday-atoms"], files=src, reexports=["faraday_atoms::Ui"])])

    def test_kit_names_only_the_ports_values(self):
        """Invariant: A kit crate names only the port's values module.
        Regression: The kit takes the port's `Event` to react to input itself.
        Owner: tool"""
        port = Crate("faraday-port", "port", values="values", no_std=True)
        k = lambda src: kit("k", "atoms", deps=["faraday-port"], files={"src/lib.rs": src})  # noqa: E731
        self.refused("P4", [port, k("use faraday_port::event::Event;\n")])
        self.refused("P4", [port, k("fn f(_: faraday_port::order::Order) {}\n")])
        self.clean([port, k("use faraday_port::values::Display;\n")])

    def test_runtime_orders_and_owner_names(self):
        """Invariant: Only the runtime names `Order::Exit`, `Order::Clean`, `Ask::Keep` and `Ask::Forget`, only an owner names `Top::Shared`, `Fx::Done` and `Effects::done`, and only the vault owner `vaults::Close`.
        Regression: A feature marks the process clean or ends the scope itself.
        Owner: tool"""
        port, kernel = Crate("faraday-port", "port", no_std=True), app("faraday-kernel", "kernel", deps=["faraday-port"])

        def f(src, **kw):
            return app("f", "feature", deps=["faraday-kernel", "faraday-port"], files={"src/lib.rs": src}, **kw)

        for src in (
            "use faraday_port::Order;\nfn a() { let _ = Order::Exit; }\n",
            "use faraday_port as port;\nfn a() { let _ = port::Order::Clean(true); }\n",
            "use faraday_port::Ask::{Keep as K};\nfn a() { let _ = K; }\n",
            "fn a(_: faraday_port::Exit) {}\n",
        ):
            self.refused("R6", [port, kernel, f(src)])
        self.clean([port, kernel, f("use faraday_port::Order;\nfn a() { let _ = Order::Print; }\n")])
        self.refused("R6", [port, kernel, f("use faraday_kernel::Top;\nfn a() { let _ = Top::Shared; }\n")])
        self.clean([port, kernel, f("use faraday_kernel::Top;\nfn a() { let _ = Top::Shared; }\n", owner=True)])
        self.refused("A24", [port, kernel, f("use faraday_kernel::Fx;\nfn a() { let _ = Fx::Done; }\n")])
        self.refused("A24", [port, kernel, f("use faraday_kernel::Effects;\nfn a(e: Effects<u8, u8>) { let _ = e.done(1, 2); }\n")])
        self.refused("A24", [port, kernel, f("use faraday_kernel::Effects;\nfn a() { let _ = Effects::done; }\n")])
        self.clean([port, kernel, f("use faraday_kernel::Effects;\nfn a(e: Effects<u8, u8>) { let _ = e.done(1, 2); }\n", owner=True)])
        self.refused("A22", [port, kernel, f("use faraday_kernel::shared::vaults;\nfn a() { let _ = vaults::Close; }\n")])

    def test_root_builds_no_event(self):
        """Invariant: A root crate makes no `port::Event`.
        Regression: A root injects a Tick event of its own instead of the adapters' polls.
        Owner: tool"""
        port = Crate("faraday-port", "port", no_std=True)

        def r(src):
            return Crate("r", "root", ships=False, deps=["faraday-port"], files={"src/lib.rs": src})

        self.refused("S6", [port, r("use faraday_port::Event;\nfn f() -> Event { Event::Tick }\n")])
        self.clean([port, r("use faraday_port::Event;\nfn f() -> Option<Event> { None }\n")])

    def test_only_own_crate_names_update(self):
        """Invariant: Outside the kernel, only a feature's own crate names its `update`, its tests included.
        Regression: A composite calls a child's `update` directly and skips `refuses`.
        Owner: tool"""
        child = app("faraday-child", "feature")
        bad = app("c", "composite", deps=["faraday-child"], files={"src/lib.rs": "use faraday_child::update;\n"})
        self.refused("A9", [child, bad])
        test_use = app("c", "composite", deps=["faraday-child"], files={"tests/t.rs": "fn t() { faraday_child::update::step(); }\n"})
        self.refused("A9", [child, test_use])
        self.clean([child, app("c", "composite", deps=["faraday-child"], files={"src/lib.rs": "use faraday_child::Msg;\n"})])

    def test_flow_models_are_filled_once(self):
        """Invariant: A type that implements `StepFlow` implements neither `Present` nor `Page`.
        Regression: A flow writes its own `Present`, so its model is filled twice.
        Owner: tool"""
        flow = "struct S;\nimpl StepFlow for S {}\n"
        self.refused("V12", [app("f", "feature", files={"src/lib.rs": flow + "impl<'a> Present for S {}\n"})])
        self.refused("V12", [app("f", "feature", files={"src/lib.rs": flow + "impl faraday_present::Page for S {}\n"})])
        self.clean([app("f", "feature", files={"src/lib.rs": flow + "impl ShowFlow for S {}\n"})])

    def test_state_fields_are_private(self):
        """Invariant: A type that implements `AtLock` outside the kernel has no `pub` field.
        Regression: A feature's `State` publishes a field so its composite reads it.
        Owner: tool"""
        bad = "pub struct State { pub keys: u8 }\nimpl AtLock for State {}\n"
        self.refused("A7", [app("f", "feature", files={"src/lib.rs": bad})])
        derived = "#[derive(AtLock)]\npub struct State(pub u8);\n"
        self.refused("A7", [app("f", "feature", files={"src/lib.rs": derived})])
        good = "pub struct State { pub(crate) keys: u8, c: u8 }\nimpl AtLock for State {}\n"
        self.clean([app("f", "feature", files={"src/lib.rs": good})])


class ModuleRule(Base):
    KERNEL = app("faraday-kernel", "kernel")

    def feat(self, module, src, **kw):
        files = {f"src/{module}.rs": src}
        return [self.KERNEL, app("f", "feature", deps=["faraday-kernel"], files=files, **kw)]

    def test_update_names_neither_wording_nor_show(self):
        """Invariant: `state`, `msg` and `update` name no `Show`, `Memo`, `Worded` or `faraday_strings`.
        Regression: An update forms text through the wording, so a refusal is worded where it is made.
        Owner: tool"""
        for src in (
            "use faraday_kernel::Show;\nfn f(_: &Show) {}\n",
            "fn f(_: &faraday_kernel::Show) {}\n",
            "use faraday_kernel::*;\nfn f(_: &Show) {}\n",
            "use faraday_kernel::{Memo as M};\nfn f(_: &M) {}\n",
            "use faraday_kernel::Worded;\n",
        ):
            for module in ("state", "msg", "update"):
                self.refused("A30" if "Show" in src or "Worded" in src else "§5", self.feat(module, src))
        self.refused("A30", self.feat("update", "use faraday_strings::EN;\n"))
        self.clean(self.feat("update", "use faraday_kernel::Ctx;\nenum Msg { Show }\nfn f() { let _ = Msg::Show; }\n"))

    def test_siblings(self):
        """Invariant: Each module of a feature names only the siblings the stage table lets it: `page` not `state`, `update` or `present`; `present` not `update` or `page`; `update` not `present`.
        Regression: A page reads the feature's state, so drawing works things out.
        Owner: tool"""
        self.refused("V1", self.feat("page", "use crate::state::State;\n"))
        self.refused("V1", self.feat("page", "fn f() { let _ = super::present::go(); }\n"))
        self.refused("§5", self.feat("present", "use crate::update::step;\n"))
        self.refused("§5", self.feat("update", "use crate::{present::go, msg::Msg};\n"))
        self.refused("§5", self.feat("model", "use super::state::State;\n"))
        self.clean(self.feat("page", "use crate::model::Model;\nuse crate::msg::Msg;\n"))
        self.clean(self.feat("present", "use crate::state::State;\nuse crate::model::Model;\n"))

    def test_page_and_present_stages(self):
        """Invariant: `page` names no `View`, `Show` or wording; `present` names no `Ui` and no `Compute`; the model and present name no geometry or tokens.
        Regression: A page asks the view for a value, or `present` does costly work.
        Owner: tool"""
        self.refused("V1", self.feat("page", "use faraday_kernel::View;\nfn f(_: &View) {}\n"))
        self.refused("V1", self.feat("page", "fn f(_: &faraday_kernel::Show) {}\n"))
        self.refused("V4", self.feat("page", "use faraday_strings::EN;\n"))
        kitcrate = kit("faraday-templates", "templates")
        ui = app("f", "feature", deps=["faraday-templates"], files={"src/present.rs": "use faraday_templates::Ui;\nfn f(_: &mut Ui) {}\n"})
        self.refused("§5", [kitcrate, ui])
        self.refused("V2", self.feat("present", "use faraday_crypto::Compute;\nfn f(_: &Compute) {}\n"))
        draw = kit("faraday_draw", "atoms")
        draw.name = "faraday-draw"
        geom = app("f", "feature", deps=["faraday-draw"], files={"src/model.rs": "use faraday_draw::geom::Rect;\nstruct M(Rect);\n"})
        self.refused("V9", [draw, geom])
        self.clean(self.feat("present", "use faraday_kernel::View;\nfn f(_: &View) {}\n"))

    def test_passphrase_state_stays_in_its_part(self):
        """Invariant: A feature's `present` and `page` read the passphrase field through its read view alone.
        Regression: A page takes the field's state and shows what was typed.
        Owner: tool"""
        part = app("faraday-pass-field", "part")
        f = lambda src: app("f", "feature", deps=["faraday-pass-field"], files={"src/page.rs": src})  # noqa: E731
        self.refused("A3", [part, f("use faraday_pass_field::State;\n")])
        self.clean([part, f("use faraday_pass_field::Read;\n")])

    def test_cfg_test_items_are_not_the_modules(self):
        """Invariant: A test module inside a feature's `update` may name its `present`.
        Regression: The module rule refuses the update tests that check a rule through `present`.
        Owner: tool"""
        self.clean(self.feat("update", "#[cfg(test)]\nmod tests { use crate::present::go; }\n"))


if __name__ == "__main__":
    unittest.main()
