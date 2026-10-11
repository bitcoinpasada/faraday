//! Drives Faraday's own flows from the command line, for
//! `faraday/tools/core-check.py`, which checks them against Bitcoin Core
//! on regtest.
//!
//! ```text
//! cargo run -p faraday-core --example interop -- kits
//! cargo run -p faraday-core --example interop -- export <kit> <dir>
//! cargo run -p faraday-core --example interop -- addresses <kit> <n>
//! cargo run -p faraday-core --example interop -- sign <kit> <seeds> <in.psbt> <dir>
//! cargo run -p faraday-core --example interop -- read <file>
//! ```
//!
//! `kits` lists the test wallets as `id descriptor`. `export` writes
//! every public file the wallet's Back up step puts in the Outbox.
//! `addresses` prints the first `n` receive and change addresses on
//! regtest, as Faraday derives them. `sign` starts the app as a shell
//! does, puts the PSBT and the wallet's descriptor in the Inbox, loads
//! the wallet, adds the test seeds named (comma-separated: bacon,
//! zebra, summer), opens the spend, presses Sign, and writes what the
//! Outbox then holds: `<stem>-final.txn` when the transaction is
//! finished, else `<stem>-signed.psbt`.
//! `read` prints what Files says a file is and, when it is a wallet,
//! the descriptor Faraday reads from it and its first receive address on
//! regtest.

use std::path::Path;
use std::process::ExitCode;

use faraday_core::testkit::{self, Kit};
use faraday_core::{Action, Faraday, StorageEvent};
use osk_bip::keys::Network;
use osk_bip::policy::WalletPolicy;
use osk_shell_api::{App, Event, Key};

fn kit(id: &str) -> Result<Kit, String> {
    testkit::kits()
        .into_iter()
        .find(|k| k.id == id)
        .ok_or_else(|| format!("no test wallet {id}"))
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(name), bytes).map_err(|e| format!("{name}: {e}"))
}

fn sign(k: &Kit, seeds: &str, psbt: &Path, dir: &Path) -> Result<String, String> {
    let bytes = std::fs::read(psbt).map_err(|e| format!("{}: {e}", psbt.display()))?;
    let wallet_file = format!("{}-wallet.txt", k.id);
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: vec![
            ("spend.psbt".to_string(), bytes),
            (wallet_file.clone(), k.descriptor.clone().into_bytes()),
        ],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let at = |app: &Faraday, name: &str| app.inbox.iter().position(|i| i.name == name);
    let w = at(&app, &wallet_file).ok_or("the wallet did not arrive")?;
    app.press(Action::LoadWallet(w));
    for seed in seeds.split(',').filter(|s| !s.is_empty()) {
        app.press(Action::Entry(None));
        for c in testkit::test_words(seed).chars() {
            app.event(Event::Key(Key::Char(c)));
        }
        app.press(Action::EntryAdd);
    }
    let p = at(&app, "spend.psbt").ok_or("the PSBT was not read as one")?;
    app.press(Action::StartSpend(p));
    if app.spend.is_none() {
        return Err("Faraday did not open the spend".into());
    }
    app.press(Action::SignHere);
    let finished = app
        .spend
        .as_ref()
        .is_some_and(|s| s.spend.finished.is_some());
    app.press(if finished {
        Action::TxToOutbox
    } else {
        Action::SignedToOutbox
    });
    let out = app.outbox.last().ok_or("nothing reached the Outbox")?;
    write(dir, &out.name, &out.bytes)?;
    Ok(out.name.clone())
}

fn run(args: &[String]) -> Result<(), String> {
    match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["kits"] => {
            for k in testkit::kits() {
                println!("{} {}", k.id, k.descriptor);
            }
        }
        ["export", id, dir] => {
            let k = kit(id)?;
            for (name, bytes) in testkit::public_files(k.name, &k.descriptor)? {
                write(Path::new(dir), &name, &bytes)?;
                println!("{name}");
            }
        }
        ["addresses", id, n] => {
            let k = kit(id)?;
            let n: u32 = n.parse().map_err(|_| "n is a number")?;
            let policy = WalletPolicy::parse_any(&k.descriptor).map_err(|e| e.to_string())?;
            for change in [false, true] {
                for i in 0..n {
                    let a = policy
                        .address_at(Network::Regtest, change, i)
                        .map_err(|e| e.to_string())?;
                    println!("{} {i} {a}", u8::from(change));
                }
            }
        }
        ["read", file] => {
            let bytes = std::fs::read(file).map_err(|e| format!("{file}: {e}"))?;
            let name = Path::new(file)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            println!("{:?}", faraday_core::wallet::classify(&name, &bytes));
            match faraday_core::wallet::read_wallet(&String::from_utf8_lossy(&bytes)) {
                Ok(w) => {
                    println!("{}", w.to_descriptor_checksummed());
                    if let Ok(a) = w.address_at(Network::Regtest, false, 0) {
                        println!("{a}");
                    }
                }
                Err(e) => println!("refused: {e:?}"),
            }
        }
        ["sign", id, seeds, psbt, dir] => {
            let name = sign(&kit(id)?, seeds, Path::new(psbt), Path::new(dir))?;
            println!("{name}");
        }
        _ => return Err("usage: kits | export <kit> <dir> | addresses <kit> <n> | sign <kit> <seeds> <in.psbt> <dir> | read <file>".into()),
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("interop: {e}");
            ExitCode::FAILURE
        }
    }
}
