//! Writes the files the test sticks carry, and proves the spend works
//! end to end before a screen is involved.
//!
//! ```text
//! faraday-testkit OUT_DIR
//! faraday-testkit --backup OUT_DIR
//! ```
//!
//! With `--backup`, only the backup test stick
//! (`faraday_core::testkit::backup_files`): what backing up the 2-of-3
//! Taproot multisig leaves, with the three seeds in a vault.
//!
//! Into `OUT_DIR`: every file of the test stick (`faraday_core::testkit`),
//! all on testnet: each test wallet in every backup form, its unsigned
//! spend and a cosigner's signed copy, the test keys in every form a key
//! arrives in, and the test vault.

use std::path::Path;
use std::process::ExitCode;

use faraday_bip::bitcoin::absolute::LockTime;
use faraday_core::testkit::{self, SPEND_TXID};
use faraday_core::wallet::Spend;
use faraday_crypto::work::Compute;
use faraday_psbt::Psbt;

/// Costs what `add_words`, `add_wallet`, `inspect` and the rest it
/// calls cost.
fn run(c: &Compute, out: &Path) -> Result<(), String> {
    let (unsigned, cosigned) = testkit::spend()?;
    println!("spend txid {}", unsigned.unsigned_tx().compute_txid());
    // The flow the screens run: the test wallet, sign here, collect the
    // cosigner's copy, finish.
    let mut session = testkit::session();
    let (word, label) = testkit::TEST_SEEDS[0];
    session
        .keys
        .add_words(c, &testkit::test_words(word), label, None)
        .map_err(|e| e.text())?;
    session
        .add_wallet(
            c,
            "Savings",
            &testkit::savings(),
            "Test wallet",
            faraday_core::wallet::Origin::Named,
        )
        .map_err(|e| e.text())?;
    let inspection = session.inspect(c, &unsigned);
    println!(
        "fee {} sat, {:?} sat/vB, warnings {:?}",
        inspection.fee.to_sat(),
        inspection.fee_rate_sat_vb,
        inspection.warnings
    );
    for o in &inspection.outputs {
        println!("  output {} {} {:?}", o.address, o.amount.to_sat(), o.kind);
    }
    let mut flow = Spend::new(unsigned.clone(), "unsigned.psbt");
    let here = session.sign(c, &mut flow.psbt, [0; 32], &mut None)?;
    println!(
        "signed here with {here:?}; signers now {:?}",
        flow.signers(c, &session)
    );
    let added = flow.collect(c, &session, &cosigned.to_bytes(), "cosigner-5d388376.psbt")?;
    println!(
        "collected {added:?}; signers now {:?}",
        flow.signers(c, &session)
    );
    flow.finish()?;
    let tx = flow.finished.as_ref().expect("finished");
    let finished_txid = tx.compute_txid().to_string();
    println!("finished txid {finished_txid} ({} vB)", tx.vsize());

    // A copy for another transaction must be refused.
    let mut other = unsigned.inner().clone();
    other.unsigned_tx.lock_time = LockTime::from_consensus(1);
    let refused = flow.collect(c, &session, &Psbt::from(other).to_bytes(), "other.psbt");
    println!("another transaction: {refused:?}");
    if refused.is_ok() {
        return Err("a PSBT for another transaction was merged".into());
    }

    if finished_txid != SPEND_TXID {
        return Err("the finished transaction has another txid".into());
    }
    // Every other test wallet: its spend signs with the test keys, and
    // finishes once enough of them have signed.
    for kit in testkit::kits() {
        let (u, cosigned) = testkit::spend_of(&kit)?;
        let mut all = testkit::session();
        for (word, label) in testkit::TEST_SEEDS {
            all.keys
                .add_words(c, &testkit::test_words(word), label, None)
                .map_err(|e| e.text())?;
        }
        all.add_wallet(
            c,
            kit.name,
            &kit.descriptor,
            "Test wallet",
            faraday_core::wallet::Origin::Named,
        )
        .map_err(|e| e.text())?;
        let mut flow = Spend::new(u.clone(), "x");
        let inspection = all.inspect(c, &u);
        let by = all
            .sign(c, &mut flow.psbt, [0; 32], &mut None)
            .map_err(|e| format!("{}: {e}", kit.id))?;
        let done = flow.finish();
        println!(
            "{:<17} fee {:>5} sat  change {:<8} signed by {:?}  {}  cosigner copy {}",
            kit.id,
            inspection.fee.to_sat(),
            if inspection.outputs.iter().any(|o| o.is_ours()) {
                "verified"
            } else {
                "NOT"
            },
            by.iter()
                .map(|f| faraday_core::wallet::fp_text(*f))
                .collect::<Vec<_>>(),
            if done.is_ok() {
                "finished".to_string()
            } else {
                format!("not finished: {:?}", done)
            },
            if cosigned.is_some() { "yes" } else { "no" },
        );
        if done.is_err() {
            return Err(format!("{} did not finish", kit.id));
        }
    }
    // The Inheritance wallet down its recovery path: key 2 alone, once the
    // sequence says the coins have waited.
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "inheritance")
        .ok_or("no inheritance kit")?;
    let mut second = testkit::session();
    let (word, label) = testkit::TEST_SEEDS[1];
    second
        .keys
        .add_words(c, &testkit::test_words(word), label, None)
        .map_err(|e| e.text())?;
    second
        .add_wallet(
            c,
            kit.name,
            &kit.descriptor,
            "Test wallet",
            faraday_core::wallet::Origin::Named,
        )
        .map_err(|e| e.text())?;
    let mut flow = Spend::new(testkit::recovery(&kit)?, "x");
    let by = second
        .sign(c, &mut flow.psbt, [0; 32], &mut None)
        .map_err(|e| format!("recovery: {e}"))?;
    flow.finish()
        .map_err(|e| format!("recovery did not finish: {e}"))?;
    println!(
        "inheritance recovery path: signed by {:?}, finished",
        by.iter()
            .map(|f| faraday_core::wallet::fp_text(*f))
            .collect::<Vec<_>>()
    );
    // MuSig2: the app's order when it signs last.
    let (mu, nonce_files, partial_files) = testkit::musig_rounds()?;
    let mut s1 = testkit::session();
    let (word, label) = testkit::TEST_SEEDS[0];
    s1.keys
        .add_words(c, &testkit::test_words(word), label, None)
        .map_err(|e| e.text())?;
    s1.add_wallet(
        c,
        "MuSig2",
        &testkit::musig_policy()?.to_descriptor_checksummed(),
        "Test wallet",
        faraday_core::wallet::Origin::Named,
    )
    .map_err(|e| e.text())?;
    let mut flow = Spend::new(mu, "musig-unsigned.psbt");
    for (fp, copy) in &nonce_files {
        flow.collect(c, &s1, &copy.to_bytes(), &format!("musig-nonce-{fp}.psbt"))?;
    }
    s1.sign(c, &mut flow.psbt, [3; 32], &mut None)
        .map_err(|e| format!("musig round 2: {e}"))?;
    for (fp, copy) in &partial_files {
        flow.collect(
            c,
            &s1,
            &copy.to_bytes(),
            &format!("musig-partial-{fp}.psbt"),
        )?;
    }
    s1.sign(c, &mut flow.psbt, [3; 32], &mut None)
        .map_err(|e| format!("musig aggregate: {e}"))?;
    flow.finish()
        .map_err(|e| format!("musig did not finish: {e}"))?;
    println!(
        "musig2 3 of 3: nonces collected, signed last, partials collected, aggregated, finished"
    );
    // FROST 2 of 3: test key 1 signed as share 1 and chose share 2; test
    // key 2, at the second location, reads the carry file and finishes.
    let file = testkit::threshold_carry()?;
    let (section, bytes) = faraday_keys::carry::parse(&file).map_err(|e| e.reason().to_string())?;
    let psbt = faraday_psbt::Psbt::parse_bytes(c, &bytes).map_err(|e| e.to_string())?;
    let mut second = testkit::session();
    let (word, label) = testkit::TEST_SEEDS[1];
    second
        .keys
        .add_words(c, &testkit::test_words(word), label, None)
        .map_err(|e| e.text())?;
    second
        .add_wallet(
            c,
            "Threshold",
            &testkit::threshold_record()?.to_text(),
            "Test wallet",
            faraday_core::wallet::Origin::Named,
        )
        .map_err(|e| e.text())?;
    let seen = second.inspect_with(c, &psbt, Some(&section));
    if !seen.outputs.iter().any(|o| o.is_ours()) || seen.threshold.is_none() {
        return Err("the threshold spend's change or wallet was not recognised".into());
    }
    let mut flow = Spend::new(psbt, "threshold-partly-signed.osk");
    second.sign_with(
        c,
        &mut flow.psbt,
        [6; 32],
        &mut None,
        &[],
        Some(&section),
        None,
    )?;
    flow.finish()
        .map_err(|e| format!("threshold did not finish: {e}"))?;
    println!(
        "frost 2 of 3: share 1 signed and chose share 2, carry file read, share 2 signed, aggregated, finished"
    );
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let files = testkit::files()?;
    // A directory an earlier kit wrote loses the files this kit no longer
    // has, so a stick made from it carries this kit alone.
    if out.join("savings-wallet.txt").exists() {
        for entry in std::fs::read_dir(out).map_err(|e| e.to_string())?.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.path().is_file() && !files.iter().any(|(n, _)| *n == name) {
                std::fs::remove_file(entry.path()).map_err(|e| format!("{name}: {e}"))?;
            }
        }
    }
    for (name, bytes) in files {
        std::fs::write(out.join(&name), bytes).map_err(|e| format!("{name}: {e}"))?;
    }
    println!("wrote {}", out.display());
    Ok(())
}

/// Writes the backup test stick's files.
fn run_backup(out: &Path) -> Result<(), String> {
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    for (name, bytes) in testkit::backup_files()? {
        std::fs::write(out.join(&name), bytes).map_err(|e| format!("{name}: {e}"))?;
        println!("{name}");
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // The test stick's files are made as the tests make theirs.
    let compute = testkit::compute();
    let result = match args.as_slice() {
        [flag, out] if flag == "--backup" => run_backup(Path::new(out)),
        [out] => run(&compute, Path::new(out)),
        _ => {
            eprintln!("usage: faraday-testkit [--backup] OUT_DIR");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("faraday-testkit: {e}");
            ExitCode::FAILURE
        }
    }
}
