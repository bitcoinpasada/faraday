//! rPGP, as the cross-check's `qrvault` area calls it (`tools/crosscheck/
//! area_qrvault.py`): one call per process, arguments in hex, the answer
//! in hex on one line.
//!
//!   seal PASSPHRASE EXPONENT CHUNK PLAINTEXT
//!                                        the RFC 9580 message of a QR vault
//!                                        (SKESK v6 and SEIPD v2, AES-256, OCB,
//!                                        Argon2 at 3 passes, 4 lanes, memory 2^EXPONENT KiB;
//!                                        CHUNK is the SEIPD's chunk size octet, 0 or 10)
//!   open PASSPHRASE MESSAGE              the literal data under the passphrase
//!
//! A refusal prints `refused: <why>` and exits with 1. Test-only: rPGP is
//! a dependency of this folder's own workspace and of nothing that ships.

use std::process::ExitCode;

use pgp::composed::{Message, MessageBuilder};
use pgp::crypto::aead::{AeadAlgorithm, ChunkSize};
use pgp::crypto::sym::SymmetricKeyAlgorithm;
use pgp::types::{Password, StringToKey};

fn unhex(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err("odd hex".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn run(args: &[String]) -> Result<String, String> {
    let e = |e: pgp::errors::Error| e.to_string();
    let mut rng = rand::thread_rng();
    match args.first().map(String::as_str) {
        Some("seal") if args.len() == 5 => {
            let pass = String::from_utf8(unhex(&args[1])?).map_err(|e| e.to_string())?;
            let exponent: u8 = args[2].parse().map_err(|_| "exponent".to_string())?;
            let chunk = match args[3].as_str() {
                "0" => ChunkSize::C64B,
                "10" => ChunkSize::C64KiB,
                _ => return Err("chunk size octet: 0 or 10".into()),
            };
            let plain = unhex(&args[4])?;
            let s2k = StringToKey::new_argon2(&mut rng, 3, 4, exponent);
            let mut b = MessageBuilder::from_bytes("", plain).seipd_v2(
                &mut rng,
                SymmetricKeyAlgorithm::AES256,
                AeadAlgorithm::Ocb,
                chunk,
            );
            b.encrypt_with_password(&mut rng, s2k, &Password::from(pass.as_str()))
                .map_err(e)?;
            Ok(hex(&b.to_vec(&mut rng).map_err(e)?))
        }
        Some("open") if args.len() == 3 => {
            let pass = String::from_utf8(unhex(&args[1])?).map_err(|e| e.to_string())?;
            let data = unhex(&args[2])?;
            let msg = Message::from_bytes(&data[..]).map_err(e)?;
            let mut msg = msg
                .decrypt_with_password(&Password::from(pass.as_str()))
                .map_err(e)?;
            Ok(hex(&msg.as_data_vec().map_err(|e| e.to_string())?))
        }
        _ => {
            Err("usage: seal PASSPHRASE EXPONENT CHUNK PLAINTEXT | open PASSPHRASE MESSAGE".into())
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(out) => {
            println!("{out}");
            ExitCode::SUCCESS
        }
        Err(why) => {
            println!("refused: {why}");
            ExitCode::from(1)
        }
    }
}
