//! The test kit's public fixtures: the wallets' descriptors, their keys
//! as extended public keys, the names of the files and the errors of
//! building them. Nothing here is secret, so it is plain text; the seeds,
//! their words and what is made from them stay in the parent module, which
//! hands this one only the public text it makes.

use super::{NUMS, compute, key_t, net};
use faraday_bip::keys::MasterKey;
use faraday_bip::policy::WalletPolicy;
use faraday_psbt::Psbt;
use faraday_text::Text;

/// `[fingerprint/path]tpub` for test seed `n` at `path`, its coin type
/// [`net`]'s ([`on_net`]): an `xpub` at coin type 0 on Mainnet.
pub fn key(n: usize, path: &str) -> String {
    String::from(&*key_t(n, path))
}

/// A 3-of-5 native SegWit multisig at BIP-48 account 0, over five keys
/// made from the seeds 01…01 to 05…05 (64 bytes each), with its checksum:
/// for the backup's shares, which a 3-of-5 can leave up to two keys off.
pub fn three_of_five() -> String {
    String::from(&*super::three_of_five_t())
}

/// `path`, an account path written at coin type 1 as this kit writes
/// them (`m/48'/1'/0'/2'`), at [`net`]'s coin type: unchanged on the test
/// networks, `m/48'/0'/0'/2'` on Mainnet.
pub fn on_net(path: &str) -> String {
    let coin = net().coin_type();
    path.split('/')
        .enumerate()
        .map(|(i, step)| match (i, step) {
            (2, "1'") => format!("{coin}'"),
            (2, "1h") => format!("{coin}h"),
            _ => step.to_string(),
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// One test wallet.
pub struct Kit {
    /// The file stem on the test stick and the wallet's short name.
    pub id: &'static str,
    /// The name the wallet is loaded under.
    pub name: &'static str,
    /// The descriptor.
    pub descriptor: String,
    /// Which test seed signs the cosigner's copy, when there is one.
    pub cosigner: Option<usize>,
    /// The fee, in sat.
    pub fee: u64,
}

/// The test wallet: a 2-of-3 native SegWit multisig over the three test
/// seeds at BIP-48 account 0, with its checksum.
pub fn savings() -> String {
    let c = compute();
    let p = "m/48'/1'/0'/2'";
    let d = format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
        key(0, p),
        key(1, p),
        key(2, p)
    );
    WalletPolicy::parse_any(&c, &d)
        .map(|w| w.to_descriptor_checksummed())
        .unwrap_or(d)
}

/// Every test wallet, in the order the session loads them.
pub fn kits() -> Vec<Kit> {
    let k1 = |p: &str| key(0, p);
    let multi = |p: &str| {
        format!(
            "{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*",
            key(0, p),
            key(1, p),
            key(2, p)
        )
    };
    vec![
        Kit {
            id: "savings",
            name: "Savings",
            descriptor: savings(),
            cosigner: Some(1),
            fee: 2_140,
        },
        Kit {
            id: "spending",
            name: "Spending",
            descriptor: format!("wpkh({}/<0;1>/*)", k1("m/84'/1'/0'")),
            cosigner: None,
            fee: 1_410,
        },
        Kit {
            id: "taproot",
            name: "Taproot",
            descriptor: format!("tr({}/<0;1>/*)", k1("m/86'/1'/0'")),
            cosigner: None,
            fee: 1_540,
        },
        Kit {
            id: "nested",
            name: "Nested SegWit",
            descriptor: format!("sh(wpkh({}/<0;1>/*))", k1("m/49'/1'/0'")),
            cosigner: None,
            fee: 1_660,
        },
        Kit {
            id: "legacy",
            name: "Legacy",
            descriptor: format!("pkh({}/<0;1>/*)", k1("m/44'/1'/0'")),
            cosigner: None,
            fee: 2_260,
        },
        Kit {
            id: "vault-nested",
            name: "Nested multisig",
            descriptor: format!("sh(wsh(sortedmulti(2,{})))", multi("m/48'/1'/0'/1'")),
            cosigner: Some(1),
            fee: 2_560,
        },
        Kit {
            id: "vault-legacy",
            name: "Legacy multisig",
            descriptor: format!("sh(sortedmulti(2,{}))", multi("m/45'/1'/0'")),
            cosigner: Some(2),
            fee: 3_380,
        },
        Kit {
            id: "taproot-multisig",
            name: "Taproot multisig",
            descriptor: format!("tr({NUMS},sortedmulti_a(2,{}))", multi("m/48'/1'/0'/3'")),
            cosigner: Some(1),
            fee: 1_820,
        },
        Kit {
            id: "inheritance",
            name: "Inheritance",
            descriptor: format!(
                "wsh(or_d(pk({}/<0;1>/*),and_v(v:pkh({}/<0;1>/*),older(52560))))",
                key(0, "m/48'/1'/1'/2'"),
                key(1, "m/48'/1'/1'/2'")
            ),
            cosigner: None,
            fee: 1_980,
        },
        Kit {
            id: "tree",
            name: "Taproot tree",
            descriptor: format!(
                "tr({}/<0;1>/*,{{and_v(v:pk({}/<0;1>/*),older(4320)),pk({}/<0;1>/*)}})",
                key(0, "m/86'/1'/1'"),
                key(1, "m/86'/1'/1'"),
                key(2, "m/86'/1'/1'")
            ),
            cosigner: None,
            fee: 1_540,
        },
    ]
}

/// A kit's wallet file: its name and bytes.
pub fn wallet_file(kit: &Kit) -> (String, Vec<u8>) {
    (
        format!("{}-wallet.txt", kit.id),
        format!(
            "# {}: test wallet over the dummy test seeds\n{}\n",
            kit.name, kit.descriptor
        )
        .into_bytes(),
    )
}

fn named(files: Vec<(Text, Vec<u8>)>) -> Vec<(String, Vec<u8>)> {
    files
        .into_iter()
        .map(|(n, b)| (String::from(&*n), b))
        .collect()
}

/// Every file of the test stick: its name and bytes.
pub fn files() -> Result<Vec<(String, Vec<u8>)>, Text> {
    super::files_t().map(named)
}

/// The public files of one wallet.
pub fn public_files(name: &str, descriptor: &str) -> Result<Vec<(String, Vec<u8>)>, Text> {
    super::public_files_t(name, descriptor).map(named)
}

/// The backup test stick's files.
pub fn backup_files() -> Result<Vec<(String, Vec<u8>)>, Text> {
    super::backup_files_t().map(named)
}

/// Files as the app queued them for the shell, with their bytes copied out
/// to plain vectors, for a test to compare.
pub fn plain_files(files: crate::Files) -> Vec<(String, Vec<u8>)> {
    named(super::plain_files_t(files))
}

/// A kit's spend and its cosigner's copy, with the cosigner's fingerprint.
pub fn spend_of(kit: &Kit) -> Result<(Psbt, Option<(String, Psbt)>), Text> {
    super::spend_of_t(kit).map(|(u, c)| (u, c.map(|(fp, p)| (String::from(&*fp), p))))
}

/// A MuSig2 round: each signer's copy, named by fingerprint.
pub type Round = Vec<(String, Psbt)>;

/// The MuSig2 spend: the unsigned PSBT, the nonce round and the partial round.
pub type Rounds = (Psbt, Round, Round);

/// The MuSig2 spend's three rounds: the unsigned spend, then each
/// signer's nonce copy and partial-signature copy, named by fingerprint.
pub fn musig_rounds() -> Result<Rounds, Text> {
    let (u, nonces, partials) = super::musig_rounds_t()?;
    let named = |v: Vec<(Text, Psbt)>| -> Vec<(String, Psbt)> {
        v.into_iter().map(|(n, p)| (String::from(&*n), p)).collect()
    };
    Ok((u, named(nonces), named(partials)))
}

/// What **Save without encryption?** asks the hold for (F129): the
/// file's name and where it goes, if it is up.
pub fn plain_asked(app: &crate::Faraday) -> Option<(String, String)> {
    super::plain_asked_t(app).map(|(a, b)| (String::from(&*a), String::from(&*b)))
}

/// What the Name taken sheet says of the file it asks about.
pub fn name_taken_body(app: &crate::Faraday) -> Option<String> {
    super::name_taken_body_t(app).map(|t| String::from(&*t))
}

/// Replaces the path Explore holds as typed with `path`, returning the
/// one it replaced.
pub fn explore_swap_path(app: &mut crate::Faraday, path: String) -> Option<String> {
    super::explore_swap_path_t(app, &path).map(|t| String::from(&*t))
}

/// A vault file, at the test cost, whose empty document `edit` changes and
/// whose XML `tweak` then rewrites as text: what a KeePass app or a newer
/// Faraday could leave (history, a recycle bin, a damaged item, a newer
/// format). The XML holds its protected values sealed, so it is plain
/// text here.
pub fn kdbx_edited(
    label: &str,
    passphrase: &str,
    id: u8,
    edit: impl FnOnce(&mut faraday_vault::kdbx::xml::Document),
    tweak: impl FnOnce(String) -> String,
) -> Result<Vec<u8>, Text> {
    super::kdbx_edited_t(label, passphrase, id, edit, |xml| {
        Text::of(&tweak(String::from(&*xml)))
    })
}

/// A key kind's key text for a master key a test holds
/// (`Keys::master_for_tests`); the session's keys go through
/// [`crate::wallet::KeyTexts`]. Bring it in with `use
/// faraday_core::testkit::KindText`.
pub trait KindText {
    /// Costs a key derivation. `[fingerprint/path]xpub` of `master` at the
    /// kind's path, account 0.
    fn key_text(
        self,
        c: &faraday_crypto::work::Compute,
        master: &MasterKey,
    ) -> Result<String, String>;

    /// Costs a key derivation. `[fingerprint/path]xpub` of `master` at the
    /// kind's path, at `account`.
    fn key_text_at(
        self,
        c: &faraday_crypto::work::Compute,
        master: &MasterKey,
        account: u32,
    ) -> Result<String, String>;
}

impl KindText for crate::create::NewKind {
    fn key_text(
        self,
        c: &faraday_crypto::work::Compute,
        master: &MasterKey,
    ) -> Result<String, String> {
        self.key_text_at(c, master, 0)
    }

    fn key_text_at(
        self,
        c: &faraday_crypto::work::Compute,
        master: &MasterKey,
        account: u32,
    ) -> Result<String, String> {
        faraday_keys::texts::key_text(c, master, &self.spec(master.network(), account)?)
            .map(|t| String::from(&*t))
            .map_err(|e| String::from(&*e))
    }
}
