//! The constructions (`docs/CRYPTO-LEARN.md` §18) a Learn page carries
//! after its own sections: the Vaults page is followed by how the vault
//! is built, then by passphrase strength, and no other page carries one.

use faraday_learn::construction::Item;
use faraday_learn::{constructions_after, how, ids, page, vaults_page};

#[test]
fn the_vaults_page_is_followed_by_how_the_vault_is_built_then_passphrase_strength() {
    let titles: Vec<&str> = constructions_after(vaults_page())
        .map(|c| c.title)
        .collect();
    assert_eq!(titles, ["How the vault is built", "Passphrase strength"]);
    let ids: Vec<&str> = how::constructions_for_page("faraday:vaults")
        .map(|c| c.id)
        .collect();
    assert_eq!(ids, ["vault", "passphrase-strength"]);
}

#[test]
fn no_other_page_carries_a_construction() {
    for id in ids().filter(|i| *i != "vaults") {
        let p = page(id).expect("an own page");
        assert_eq!(constructions_after(p).count(), 0, "{id}");
    }
}

#[test]
fn the_vaults_page_no_longer_has_a_passphrase_strength_section_of_its_own() {
    assert!(
        vaults_page()
            .sections
            .iter()
            .all(|s| s.heading != "Passphrase strength")
    );
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
        planned.iter().any(|t| t.contains("seal count")),
        "the seal count is not built yet"
    );
}
