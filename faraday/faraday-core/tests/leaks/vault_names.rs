//! A vault's group names are held as wiping text, so a vault with a named
//! group, opened and then locked, leaves no copy of the name.

use crate::common::device;
use crate::forms::{VAULT_PASSPHRASE, vault_passphrase_forms};
use crate::seed_load::{lock, unlock};
use crate::{arm, finish};
use faraday_core::testkit;
use faraday_leak_scan::Form;

const GROUP: &str = "Quarry-Heron-Group-6619-Saffron-Pylon";

/// A vault file whose one group holds a person's entry, under
/// `VAULT_PASSPHRASE`.
fn vault_with_a_group() -> Vec<u8> {
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\"?>\n<KeePassFile><Meta></Meta><Root><Group><UUID>CQkJCQkJCQkJCQkJCQ==</UUID><Name>Cellar</Name><Group><UUID>CgoKCgoKCgoKCgoKCg==</UUID><Name>{GROUP}</Name><Entry><UUID>CwsLCwsLCwsLCwsLCw==</UUID><String><Key>Title</Key><Value>Mail</Value></String><String><Key>UserName</Key><Value>me</Value></String></Entry></Group></Group></Root></KeePassFile>\n\n"
    );
    let random = [3u8; faraday_vault::kdbx::RANDOM_LEN];
    let contents = faraday_vault::kdbx::Contents {
        document: xml.as_bytes(),
        binaries: faraday_vault::kdbx::Binaries::all(&[]),
        public_custom_data: None,
    };
    let cost = faraday_vault::Cost {
        memory_kib: 8 * 1024,
        passes: 1,
        lanes: 1,
    };
    faraday_vault::kdbx::seal(
        &testkit::compute(),
        VAULT_PASSPHRASE,
        cost,
        &random,
        &contents,
        xml.len() + 1024 * 1024,
    )
    .expect("sealed")
}

#[test]
fn a_vault_opened_with_a_named_group_leaves_no_copy_of_the_name_after_lock() {
    let file = vault_with_a_group();
    let mut forms = vault_passphrase_forms();
    forms.push(Form::new("group name", GROUP.as_bytes().to_vec()));
    let armed = arm(&forms);
    let mut app = device(vec![("cellar.kdbx".into(), file)]);
    unlock(&mut app);
    lock(&mut app);
    finish("vault group name", armed, true);
}
