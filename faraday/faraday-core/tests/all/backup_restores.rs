//! A backup Faraday writes restores the wallet it was written for: split
//! sheets and the Coldcard config are offered only for wallets they
//! rebuild, and any quorum of the sheets rebuilds the wallet's own
//! addresses through the code Restore uses.

use faraday_core::testkit;
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
                .map(|row| split_share(w, row).expect("a sheet"))
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
        .map(|row| split_share(sorted, row).expect("a sheet"))
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
        .to_descriptor_checksummed();
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
        let texts = app.drawn_texts();
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
