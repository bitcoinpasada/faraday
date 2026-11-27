//! The test vault: one passphrase at the test cost, holding test key 1, an entry, a note and
//! every test wallet.

#![expect(
    clippy::disallowed_methods,
    reason = "the test kit makes dummy seeds and vault files directly and stays out of every shipped binary (docs/KEYS.md)"
)]

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_keys::vault::{Names, OpenVaults};
use faraday_strings::EN;
use faraday_text::{Text, text};
use faraday_vault::Record;
use faraday_vault::records::{self, field, kind};

use crate::psbts::musig_policy;
use crate::{VAULT_PASSPHRASE, compute, kits, test_seeds};

/// The names Faraday writes into a vault, in English.
fn names() -> Names {
    Names {
        group_keys: EN.vault_group_keys,
        group_wallets: EN.vault_group_wallets,
        group_notes: EN.vault_group_notes,
        group_sheets: EN.vault_group_sheets,
        group_gpg: EN.vault_group_gpg,
        group_secure_boot: EN.vault_group_secure_boot,
        group_wallet_shares: EN.vault_group_wallet_shares,
        group_passphrases: EN.vault_group_passphrases,
        group_shares: EN.vault_group_shares,
        title_key: EN.vault_title_key,
        title_note: EN.vault_title_note,
        title_passphrase: EN.vault_title_passphrase,
        title_share: EN.vault_title_share,
        title_secure_boot: EN.vault_title_secure_boot,
    }
}

/// A vault file labelled `label` under `passphrase` holding `records`, at the test cost (8 MiB,
/// one pass, one lane). `id` makes its identifier and the randomness of its seals.
fn kdbx_vault(
    label: &str,
    passphrase: &str,
    id: u8,
    records: Vec<Record>,
) -> Result<Vec<u8>, Text> {
    let c = compute();
    let cost = faraday_vault::Cost {
        memory_kib: *faraday_vault::kdbx::MEMORY_KIB.start(),
        passes: 1,
        lanes: 1,
    };
    let file = faraday_vault::file::create(
        &c,
        [id; 16],
        label,
        cost,
        passphrase,
        &[id; faraday_vault::kdbx::RANDOM_LEN],
        None,
    )
    .map_err(|e| text!("{e:?}"))?;
    if records.is_empty() {
        return Ok(file);
    }
    let mut store = OpenVaults::default();
    store.set_names(names());
    let (vid, _) = store
        .unlock(&c, file, passphrase)
        .map_err(|e| text!("{e:?}"))?;
    for r in records {
        store.insert(vid, r).ok_or_else(|| Text::of("not kept"))?;
    }
    let n = store.uuid_bytes(vid).ok_or_else(|| Text::of("not held"))?;
    let uuids: Vec<u8> = (0..n).map(|i| id.wrapping_add((i / 16) as u8)).collect();
    store
        .seal_job(vid, &[id; faraday_vault::kdbx::RANDOM_LEN], &uuids, None)
        .ok_or_else(|| Text::of("no job"))?
        .run(&c)
        .into_file()
        .map_err(|e| text!("{e:?}"))
}

/// The test vault, sealed.
pub(crate) fn test_vault() -> Result<Vec<u8>, Text> {
    let c = compute();
    let seed = test_seeds()[0];
    let words = Text::of(seed.words);
    let m = Mnemonic::parse(&c, Language::English, &words).map_err(|e| text!("{e:?}"))?;
    let mut wallets: Vec<(Text, Text)> = kits()
        .into_iter()
        .map(|k| (Text::of(k.name), Text::of(&k.descriptor)))
        .collect();
    wallets.push((
        Text::of("MuSig2"),
        Text::of(&musig_policy()?.to_descriptor_checksummed()),
    ));
    let mut records = vec![
        Record::new(kind::KEY)
            .with(field::KEY, &records::words_payload(&m))
            .with(field::KEY_LABEL, seed.name.as_bytes()),
        Record::new(kind::ENTRY)
            .with(field::TITLE, b"Email")
            .with(field::USERNAME, b"you@example.com")
            .with(field::PASSWORD, b"correct-horse-battery-staple")
            .with(field::URL, b"https://mail.example.com")
            .with(
                field::TOTP,
                b"otpauth://totp/Example:you?secret=JBSWY3DPEHPK3PXP",
            ),
        Record::new(kind::NOTE).with(field::NOTE, b"Recovery codes\n1111-2222\n3333-4444"),
    ];
    for (name, text) in &wallets {
        records.push(
            Record::new(kind::WALLET)
                .with(field::WALLET, text.as_bytes())
                .with(field::WALLET_NAME, name.as_bytes()),
        );
    }
    kdbx_vault("Test vault", VAULT_PASSPHRASE, 0x7a, records)
}
