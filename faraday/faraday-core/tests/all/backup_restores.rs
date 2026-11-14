//! A backup Faraday writes restores the wallet it was written for: split
//! sheets and the Coldcard config are offered only for wallets they
//! rebuild, and any quorum of the sheets rebuilds the wallet's own
//! addresses through the code Restore uses.

use faraday_core::testkit;
use faraday_core::testkit::Plain;
use faraday_core::wallet::{Origin, Session, Wallet, read_wallet};
use faraday_core::wallet_backup::{multisig_config, split_plan, split_share, splits};

/// A kit's descriptor, without its checksum.
fn kit(id: &str) -> String {
    let k = testkit::kits()
        .into_iter()
        .find(|k| k.id == id)
        .expect("kit");
    k.descriptor.split('#').next().unwrap().to_string()
}

/// The wallet `descriptor` describes, loaded into a session.
fn load(descriptor: &str) -> (Session, usize) {
    let c = testkit::compute();
    let mut s = testkit::session();
    let i = s
        .add_wallet(&c, "Test", descriptor, "Test wallet", Origin::Named)
        .expect("reads");
    (s, i)
}

/// The descriptors of wallets whose keys sit in the script in the order
/// written: `multi` and `multi_a`, unsorted.
fn unsorted() -> Vec<String> {
    vec![
        kit("savings").replace("sortedmulti(", "multi("),
        kit("vault-nested").replace("sortedmulti(", "multi("),
        kit("vault-legacy").replace("sortedmulti(", "multi("),
        kit("taproot-multisig").replace("sortedmulti_a(", "multi_a("),
        twice(),
    ]
}

/// A sorted 2-of-3 two of whose keys come from one seed, at BIP-48
/// accounts 0 and 1.
fn twice() -> String {
    let (a, b) = ("m/48'/1'/0'/2'", "m/48'/1'/1'/2'");
    format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
        testkit::key(0, a),
        testkit::key(0, b),
        testkit::key(1, a)
    )
}

/// A wallet whose keys keep the order written (`wsh(multi(…))`,
/// `sh(wsh(multi(…)))`, `sh(multi(…))`, `tr(NUMS, multi_a(…))`), or a
/// `sortedmulti` two of whose keys come from one seed, offers no
/// split sheets and no Coldcard config: its public files hold neither,
/// and its whole-wallet sheet is still offered.
#[test]
fn a_wallet_of_unsorted_keys_or_one_seed_twice_offers_no_split_sheets_and_no_multisig_config() {
    for d in unsorted() {
        let (s, i) = load(&d);
        let w = &s.wallets[i];
        assert!(!splits(w), "{d}: split sheets offered");
        assert!(multisig_config(w, None).is_none(), "{d}: config offered");
        let (m, n) = Session::quorum(w);
        for row in split_plan(n, m, m - 1) {
            assert!(split_share(w, &row).is_none(), "{d}: a sheet made");
        }
        let files = testkit::public_files("Test", &d).expect("files");
        let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
        assert!(
            !names.iter().any(|n| n.contains("-share-")),
            "{d}: {names:?}"
        );
        assert!(
            !names.iter().any(|n| n.contains("multisig-config")),
            "{d}: {names:?}"
        );
        assert!(names.contains(&"test-backup.pdf"), "{d}: {names:?}");
    }
}

/// The first receive and change addresses of `w`, on `s`'s network.
fn firsts(s: &Session, w: &faraday_bip::policy::WalletPolicy) -> (String, String) {
    let c = testkit::compute();
    let net = s.network();
    (
        w.address_at(&c, net, false, 0)
            .expect("receive")
            .to_string(),
        w.address_at(&c, net, true, 0).expect("change").to_string(),
    )
}

/// The wallet a set of sheets rebuilds, as Restore rebuilds it.
fn rebuilt(sheets: &[String]) -> faraday_bip::policy::WalletPolicy {
    let c = testkit::compute();
    let whole = faraday_core::restore::merge(sheets)
        .expect("merge")
        .whole
        .expect("every key in hand");
    read_wallet(&c, &whole).expect("reads")
}

/// Every set of `m` of `n`, as bit masks.
fn quorums(n: usize, m: usize) -> Vec<Vec<usize>> {
    (0u32..1 << n)
        .filter(|mask| mask.count_ones() as usize == m)
        .map(|mask| (0..n).filter(|i| mask & (1 << i) != 0).collect())
        .collect()
}

/// A sorted multisig in each wrapper and a `tr(NUMS, sortedmulti_a)`
/// wallet: every quorum of its split sheets, at every number of keys
/// left off, rebuilds its first receive and change addresses; its
/// Coldcard config, where it has one, does too.
#[test]
fn any_quorum_of_split_sheets_rebuilds_the_wallets_first_addresses() {
    for id in [
        "savings",
        "vault-nested",
        "vault-legacy",
        "taproot-multisig",
    ] {
        let (s, i) = load(&kit(id));
        let w: &Wallet = &s.wallets[i];
        assert!(splits(w), "{id}");
        let want = firsts(&s, w.policy());
        let (m, n) = Session::quorum(w);
        for omit in 0..m {
            let plan = split_plan(n, m, omit);
            let sheets: Vec<String> = plan
                .iter()
                .map(|row| String::from(&*split_share(w, row).expect("a sheet")))
                .collect();
            for q in quorums(n, m) {
                let some: Vec<String> = q.iter().map(|&j| sheets[j].clone()).collect();
                assert_eq!(
                    firsts(&s, &rebuilt(&some)),
                    want,
                    "{id}, {omit} off, sheets {q:?}"
                );
            }
        }
        if let Some(config) = multisig_config(w, None) {
            let c = testkit::compute();
            let back = read_wallet(&c, &config).expect("config reads");
            assert_eq!(firsts(&s, &back), want, "{id}: config");
        }
    }
}

/// Sheets that rebuild another wallet are refused before anything is
/// written: the sorted wallet's sheets, checked against the wallet of
/// the same keys in the order written, do not rebuild it.
#[test]
fn sheets_that_rebuild_another_wallet_are_refused() {
    let c = testkit::compute();
    let (s, i) = load(&kit("savings"));
    let sorted = &s.wallets[i];
    let (m, n) = Session::quorum(sorted);
    let sheets: Vec<String> = split_plan(n, m, 1)
        .iter()
        .map(|row| String::from(&*split_share(sorted, row).expect("a sheet")))
        .collect();
    assert!(faraday_core::restore::rebuilds(
        &c,
        s.network(),
        sorted,
        &sheets[..m]
    ));
    let (u, j) = load(&kit("savings").replace("sortedmulti(", "multi("));
    assert!(!faraday_core::restore::rebuilds(
        &c,
        u.network(),
        &u.wallets[j],
        &sheets[..m]
    ));
}

/// The backup board of a multisig whose description does not split says
/// why partial descriptions are not offered (its keys unsorted, or two of
/// one seed); a MuSig2 wallet's says nothing of them, and a sorted
/// multisig's offers them.
#[test]
fn the_board_says_why_partial_descriptions_are_not_offered_for_a_multisig_that_does_not_split() {
    use faraday_core::Action;
    use faraday_shell_api::App;
    let (only, one_seed) = (
        "Partial descriptions: only for",
        "Partial descriptions: not for",
    );
    let add = "Add partial descriptions to this backup";
    let musig = testkit::musig_policy()
        .expect("the MuSig2 kit")
        .to_descriptor_checksummed()
        .as_str()
        .to_owned();
    let cases = [
        (
            kit("savings").replace("sortedmulti(", "multi("),
            Some(only),
            false,
        ),
        (
            kit("taproot-multisig").replace("sortedmulti_a(", "multi_a("),
            Some(only),
            false,
        ),
        (twice(), Some(one_seed), false),
        (kit("savings"), None, true),
        (musig, None, false),
    ];
    for (d, says, offers) in cases {
        let c = testkit::compute();
        let mut app = crate::common::device(Vec::new());
        let i = app
            .session
            .add_wallet(&c, "Test", &d, "Test wallet", Origin::Named)
            .expect("reads");
        app.press(Action::Backup(i));
        testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
        let _ = app.frame();
        let texts = app.drawn_texts().plain();
        let said = texts
            .iter()
            .find(|t| t.starts_with("Partial descriptions:"))
            .map(String::as_str);
        assert_eq!(
            said.map(|t| says.is_some_and(|s| t.starts_with(s))),
            says.map(|_| true),
            "{d}: {texts:?}"
        );
        assert_eq!(texts.iter().any(|t| t == add), offers, "{d}: {texts:?}");
    }
}

/// The default plan of a wallet whose seeds must all sign does not say
/// that losing any one place leaves a wallet that rebuilds, nor that one
/// place with one seed and the sheet can spend; a 2-of-3 reads as it
/// always did.
#[test]
fn a_plan_of_a_wallet_whose_keys_must_all_sign_does_not_say_one_place_can_be_lost() {
    use faraday_core::plan::{Answers, Found, Lost, check};
    let path = "m/48'/1'/0'/2'";
    let both = format!(
        "wsh(and_v(v:pk({}/<0;1>/*),pk({}/<0;1>/*)))",
        testkit::key(0, path),
        testkit::key(1, path)
    );
    let musig = testkit::musig_policy()
        .expect("the MuSig2 kit")
        .to_descriptor_checksummed()
        .as_str()
        .to_owned();
    let c = testkit::compute();
    let read = |d: &str| {
        let mut app = crate::common::device(Vec::new());
        let i = app
            .session
            .add_wallet(&c, "Test", d, "Test wallet", Origin::Named)
            .expect("reads");
        let mut shape = app.plan_shape_now(i);
        shape.seeds.iter_mut().for_each(|s| s.here = true);
        let check = check(&c, &shape, &Answers::defaults(&shape));
        (shape.seeds.len(), check)
    };
    for d in [&musig, &both] {
        let (seeds, got) = read(d);
        assert!(seeds >= 2, "{d}");
        assert_ne!(got.lost, Lost::Yes, "{d}");
        assert_eq!(got.spend, Found::No, "{d}");
    }
    let (_, got) = read(&kit("savings"));
    assert_eq!(got.lost, Lost::Yes);
    assert_eq!(got.spend, Found::No);
}

/// A way to spend that needs a hash's preimage is not a way a backup of
/// seeds can spend: the plan does not say the key beside the preimage can
/// spend alone, nor reads that way as needing no seed.
#[test]
fn a_plan_does_not_count_a_way_that_needs_a_preimage_as_spendable_from_seeds() {
    use faraday_core::plan::{Answers, Found, Lost, check};
    let path = "m/48'/1'/0'/2'";
    let (a, b, k) = (
        testkit::key(0, path),
        testkit::key(1, path),
        testkit::key(2, path),
    );
    let h = "11".repeat(32);
    let c = testkit::compute();
    let wallets = [format!(
        "wsh(or_d(multi(2,{a}/<0;1>/*,{b}/<0;1>/*),and_v(v:pk({k}/<0;1>/*),sha256({h}))))"
    )];
    for d in wallets {
        let mut app = crate::common::device(Vec::new());
        let i = app
            .session
            .add_wallet(&c, "Test", &d, "Test wallet", Origin::Named)
            .unwrap_or_else(|e| panic!("{d}: {e:?}"));
        let mut shape = app.plan_shape_now(i);
        shape.seeds.iter_mut().for_each(|s| s.here = true);
        assert!(shape.paths.iter().all(|&p| p != 0), "{d}");
        let got = check(&c, &shape, &Answers::defaults(&shape));
        assert_ne!(got.lost, Lost::Yes, "{d}");
        assert_eq!(got.spend, Found::No, "{d}");
    }
}

/// The board's line for a place in a wallet with several ways to spend
/// never says it keeps more seeds than the way it is closest to needs.
#[test]
fn the_board_line_for_a_wallet_with_several_ways_never_says_more_seeds_than_a_way_needs() {
    use faraday_core::plan::{At, Seed, Shape, Spot, Tag, What};
    let c = testkit::compute();
    let seeds: Vec<Seed> = (0..3)
        .map(|i| Seed {
            name: faraday_text::text!("{:08x}", 0x1000_0000u32 + i),
            here: true,
            passphrase: false,
        })
        .collect();
    let shape = Shape {
        paths: vec![0b011, 0b100],
        m: 1,
        keys: 3,
        seeds,
        splits: false,
        vaults: 0,
    };
    let mut a = faraday_core::plan::Answers::default();
    a.set_map(
        (0..3)
            .map(|i| Spot {
                at: At::Place(i),
                holds: vec![(What::Words(i), Tag::Secret)],
            })
            .collect(),
    );
    for p in 0..3 {
        let r = faraday_core::plan::reach(&c, &shape, &a, p).expect("keeps a seed");
        assert!(r.seeds <= r.of, "place {p}: {} of {}", r.seeds, r.of);
    }
    let r = faraday_core::plan::reach(&c, &shape, &a, 0).expect("keeps a seed");
    assert_eq!((r.seeds, r.of), (1, 2));
}
