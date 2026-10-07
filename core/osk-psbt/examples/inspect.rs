//! PSBT inspector and signer at the terminal, in the order the Sign wizard
//! shows things (`docs/UX.md` §7.3): summary, outputs, inputs and
//! warnings; then, with `--sign`, per-input signature status and the
//! signed PSBT or the final transaction.
//!
//! ```text
//! cargo run -p osk-psbt --example inspect -- --psbt <base64 or file> [--words "..."] [--passphrase X] [--network regtest] [--sign] [--force] [--out file]
//! cargo run -p osk-psbt --example inspect -- --make-test-psbt <file> [--network regtest] [--script-type native-segwit]
//! ```
//!
//! Demo tool: prints secrets to the terminal. Use test seeds only.
//! Examples are outside the secret lint's scope (`src/` only) and may use
//! `std`.

use std::process::ExitCode;

use bitcoin::consensus::encode::serialize_hex;
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{Fingerprint, MasterKey, Network, ScriptType};
use osk_psbt::{
    Aux, Context, Inspection, KeyRef, Nonce, OutputKind, Psbt, finalize, inspect, sign,
};

#[path = "../tests/common/builder.rs"]
mod builder;

const USAGE: &str = "\
usage: inspect --psbt <base64|file> [--words \"<12..24 words>\"] [--passphrase <ascii>]
               [--network <mainnet|testnet|signet|regtest>] [--sign] [--force] [--out <file>]
       inspect --make-test-psbt <file> [--network <name>] [--script-type <legacy|nested-segwit|native-segwit|taproot>]

Demo tool: prints secrets to the terminal. Use test seeds only.";

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

struct Args {
    psbt: Option<String>,
    make: Option<String>,
    words: Option<String>,
    passphrase: String,
    network: Network,
    script_type: ScriptType,
    sign: bool,
    force: bool,
    out: Option<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = std::env::args().skip(1);
    let mut a = Args {
        psbt: None,
        make: None,
        words: None,
        passphrase: String::new(),
        network: Network::Regtest,
        script_type: ScriptType::NativeSegwit,
        sign: false,
        force: false,
        out: None,
    };
    while let Some(flag) = args.next() {
        let mut value = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        match flag.as_str() {
            "--psbt" => a.psbt = Some(value("--psbt")?),
            "--make-test-psbt" => a.make = Some(value("--make-test-psbt")?),
            "--words" => a.words = Some(value("--words")?),
            "--passphrase" => a.passphrase = value("--passphrase")?,
            "--network" => {
                let name = value("--network")?;
                a.network = Network::from_name(&name).ok_or(format!("unknown network {name}"))?;
            }
            "--script-type" => {
                let name = value("--script-type")?;
                a.script_type = ScriptType::ALL
                    .into_iter()
                    .find(|t| t.name() == name)
                    .ok_or(format!("unknown script type {name}"))?;
            }
            "--sign" => a.sign = true,
            "--force" => a.force = true,
            "--out" => a.out = Some(value("--out")?),
            "-h" | "--help" => return Err(USAGE.into()),
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
    }
    if a.psbt.is_none() && a.make.is_none() {
        return Err(USAGE.into());
    }
    Ok(a)
}

fn load_psbt(arg: &str) -> Result<Psbt, String> {
    if let Ok(bytes) = std::fs::read(arg) {
        if bytes.starts_with(b"psbt\xff") {
            return Psbt::parse_bytes(&bytes).map_err(|e| e.to_string());
        }
        let text = String::from_utf8(bytes).map_err(|_| "file is neither binary PSBT nor text")?;
        return Psbt::parse_base64(&text).map_err(|e| e.to_string());
    }
    Psbt::parse_base64(arg).map_err(|e| e.to_string())
}

fn master_from(words: &str, passphrase: &str, network: Network) -> Result<MasterKey, String> {
    let m = Mnemonic::parse(Language::English, words).map_err(|e| e.to_string())?;
    let seed = m
        .to_seed(passphrase.as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(MasterKey::from_seed(&seed, network))
}

fn btc(a: bitcoin::Amount) -> String {
    format!("{} sat ({:.8} BTC)", a.to_sat(), a.to_btc())
}

fn print_inspection(insp: &Inspection, ctx: &Context) {
    println!("== Summary ==");
    println!("network:          {}", ctx.network);
    println!("amount to others: {}", btc(insp.amount_to_others));
    println!(
        "recipients:       {}",
        insp.outputs.iter().filter(|o| !o.is_ours()).count()
    );
    match insp.fee_rate_sat_vb {
        Some(rate) => println!(
            "fee:              {} at ~{rate:.1} sat/vB (est. {} vB)",
            btc(insp.fee),
            insp.vsize_estimate
        ),
        None => println!("fee:              unknown (missing UTXO data)"),
    }
    let change = insp.outputs.iter().filter(|o| o.is_ours()).count();
    let unverified = insp
        .outputs
        .iter()
        .filter(|o| matches!(o.kind, OutputKind::UnverifiedChange { .. }))
        .count();
    println!(
        "change:           {} verified output(s), {}; {unverified} NOT verified",
        change,
        btc(insp.change_total)
    );
    if insp.is_self_transfer {
        println!("self-transfer:    every output is yours");
    }
    let keys: Vec<String> = insp
        .participating_keys
        .iter()
        .map(|f| f.to_string())
        .collect();
    println!(
        "participating:    {}",
        if keys.is_empty() {
            "NONE of your keys can sign this".to_string()
        } else {
            keys.join(", ")
        }
    );
    if let Some(ms) = &insp.multisig {
        println!(
            "multisig:         {}-of-{}, {} signed, {}",
            ms.m,
            ms.n,
            ms.signed(),
            if ms.ours {
                "you are a cosigner"
            } else {
                "you are NOT a cosigner"
            }
        );
        for c in &ms.cosigners {
            println!(
                "  cosigner {}{}{}",
                c.fingerprint
                    .map_or("(no origin)".to_string(), |f| f.to_string()),
                if c.ours { " (yours)" } else { "" },
                if c.signed { " signed" } else { "" }
            );
        }
    }

    println!("\n== Outputs ==");
    for o in &insp.outputs {
        let kind = match &o.kind {
            OutputKind::Recipient => "recipient".to_string(),
            OutputKind::Change { key, change, index } => format!(
                "CHANGE verified: key {key} {}/{index}",
                if *change { "1" } else { "0" }
            ),
            OutputKind::WalletChange {
                wallet,
                change,
                index,
            } => format!(
                "CHANGE verified: wallet {wallet} {}/{index}",
                if *change { "1" } else { "0" }
            ),
            OutputKind::UnverifiedChange { key } => format!("claims key {key}, NOT verified"),
        };
        println!(
            "#{} {}\n    {}  {kind}{}",
            o.index,
            o.address,
            btc(o.amount),
            if o.is_dust { "  DUST" } else { "" }
        );
    }

    println!("\n== Inputs ==");
    println!(
        "version {}, locktime {}, rbf {}",
        insp.version,
        insp.locktime,
        if insp.rbf { "yes" } else { "no" }
    );
    for i in &insp.inputs {
        println!(
            "#{} {}:{}  {}  {}{}",
            i.index,
            i.txid,
            i.vout,
            i.amount.map_or("unknown amount".to_string(), btc),
            i.script_type,
            if i.is_ours { "  yours" } else { "" }
        );
        if let Some(o) = &i.origin {
            println!("    origin [{}/{}]", o.fingerprint, o.path);
        }
        println!(
            "    sighash {}  signatures {}{}{}",
            i.sighash,
            i.signatures,
            if i.already_signed_by.is_empty() {
                String::new()
            } else {
                format!(
                    " ({})",
                    i.already_signed_by
                        .iter()
                        .map(|f| f.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            },
            if i.finalized { "  finalized" } else { "" }
        );
    }

    println!("\n== Warnings ==");
    if insp.warnings.is_empty() {
        println!("none");
    }
    for w in &insp.warnings {
        println!("[{}] {}", w.level.to_string().to_uppercase(), w.text);
    }
}

fn run(a: Args) -> Result<(), String> {
    if let Some(path) = &a.make {
        let master = master_from(ABANDON, "", a.network)?;
        let other = master_from(
            "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong",
            "",
            a.network,
        )?;
        let recipient = other
            .account_xpub(ScriptType::NativeSegwit, 0)
            .map_err(|e| e.to_string())?
            .address(false, 0)
            .map_err(|e| e.to_string())?
            .script_pubkey();
        let mut b = builder::Build::new(a.script_type, a.network);
        b.recipients.push((recipient, 60_000));
        b.change = Some(39_000);
        let psbt = builder::single_sig(&master, &b);
        std::fs::write(path, psbt.to_base64()).map_err(|e| e.to_string())?;
        println!(
            "wrote {path}: {} {} spend of 100 000 sat from the \"abandon … about\" seed, fingerprint {}",
            a.network,
            a.script_type,
            master.fingerprint()
        );
        return Ok(());
    }

    let mut psbt = load_psbt(a.psbt.as_deref().expect("checked in parse_args"))?;
    let master = match &a.words {
        Some(words) => Some(master_from(words, &a.passphrase, a.network)?),
        None => None,
    };
    let keys: Vec<KeyRef> = match &master {
        Some(m) => vec![KeyRef::from_master(m, 0).map_err(|e| e.to_string())?],
        None => Vec::new(),
    };
    let ctx = Context {
        network: a.network,
        keys: &keys,
        wallets: &[],
        musig_session: None,
        shares: &[],
        carry: None,
    };
    let insp = inspect(&psbt, &ctx);
    print_inspection(&insp, &ctx);

    if !a.sign {
        return Ok(());
    }
    let Some(master) = &master else {
        return Err("--sign needs --words".into());
    };
    let fp: Fingerprint = master.fingerprint();
    println!("\n== Signing with {fp} ==");
    let result = sign(
        &mut psbt,
        &[master],
        &[fp],
        &ctx,
        a.force,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        [0u8; 32],
        &[],
        &[],
    )
    .map_err(|e| {
        if insp.has_danger() && !a.force {
            format!("{e}; pass --force after reading the danger warnings")
        } else {
            e.to_string()
        }
    })?;
    for s in &result.signed_inputs {
        println!(
            "input {}: {:?} signature, verified {}, nonce OK (deterministic) {}, {} bytes: {}",
            s.index,
            s.kind,
            s.verified,
            s.deterministic_ok,
            s.sig_bytes.len(),
            s.sig_bytes
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
    }
    if result.signed_inputs.is_empty() {
        println!("nothing to sign");
    }
    let output = if result.complete {
        let tx = finalize(&mut psbt)
            .map_err(|e| e.to_string())?
            .expect("complete");
        println!("complete: finalized transaction {}", tx.compute_txid());
        serialize_hex(&tx)
    } else {
        println!("not complete: signed PSBT follows");
        psbt.to_base64()
    };
    match &a.out {
        Some(path) => {
            std::fs::write(path, &output).map_err(|e| e.to_string())?;
            println!("wrote {path}");
        }
        None => println!("{output}"),
    }
    Ok(())
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    if args.psbt.is_some() {
        eprintln!("Demo tool: prints secrets to the terminal. Use test seeds only.\n");
    }
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
