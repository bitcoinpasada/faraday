//! The constructions (`docs/CRYPTO-LEARN.md` §18) the Cryptography tab
//! draws after the blocks of the actions they belong to.

use faraday_guide::construction::Item;
use faraday_guide::how;

#[test]
fn a_construction_follows_the_last_block_of_its_actions_shown() {
    let shown: Vec<_> = ["vault-create", "vault-unlock", "vault-seal"]
        .iter()
        .map(|id| how::BLOCKS.iter().find(|b| b.id == *id).expect("a block"))
        .collect();
    let after = |at| -> Vec<&str> {
        how::constructions_after_block(&shown, at)
            .map(|c| c.id)
            .filter(|id| *id == "vault")
            .collect()
    };
    assert!(after(0).is_empty());
    assert!(after(1).is_empty());
    assert_eq!(after(2), ["vault"]);
}

#[test]
fn every_table_row_has_a_cell_for_each_head() {
    for c in how::CONSTRUCTIONS {
        for part in c.sections {
            for it in part.items {
                if let Item::Table(t) = it {
                    for r in t.rows {
                        assert_eq!(r.len(), t.head.len(), "{}: {}", c.id, part.heading);
                    }
                }
            }
        }
    }
}

#[test]
fn work_not_built_reads_as_planned() {
    let vault = how::construction("vault").expect("the vault's construction");
    let planned: Vec<&str> = vault
        .sections
        .iter()
        .flat_map(|p| p.items.iter())
        .filter_map(|i| match i {
            Item::Para(t) if t.starts_with("Planned, not built: ") => Some(*t),
            _ => None,
        })
        .collect();
    assert!(
        !planned.iter().any(|t| t.contains("Normalization Form C")),
        "the passphrase in NFC is built"
    );
    assert!(
        !planned.iter().any(|t| t.contains("seal count")),
        "the seal count is built"
    );
}
