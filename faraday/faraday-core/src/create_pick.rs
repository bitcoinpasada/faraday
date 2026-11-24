//! Make a new wallet's Keys step as a pick: the open slot's keys and ways
//! in as rows (`docs/WORKSPACE.md` §3). Labels and fingerprints only.

use crate::pick::PickRow;
use crate::strings::{EN, fill};
use faraday_draw::widgets::Icon;

/// The keys held here as the pick's rows for slot `slot`: a key that stands
/// in another slot stays, dimmed, its line "In slot n" (n from 1) and no
/// press.
pub(crate) fn key_rows(app: &crate::Faraday, c: &crate::CreateState, slot: usize) -> Vec<PickRow> {
    use crate::create::Source;
    app.session
        .keys
        .all()
        .map(|k| {
            let fp = crate::wallet::fp_text(k.fingerprint);
            let label = k.label.trim();
            let line = if label.eq_ignore_ascii_case(&fp) {
                ""
            } else {
                label
            };
            let row = PickRow::pick(
                Icon::Keys,
                String::from(&*fp),
                line.to_string(),
                c.slots[slot] == Source::Here(k.fingerprint.0),
                crate::Action::CSlotHere(slot as u8, k.fingerprint.0),
            );
            match c
                .slots
                .iter()
                .position(|s| *s == Source::Here(k.fingerprint.0))
            {
                Some(i) if i != slot => {
                    row.dimmed_for(&fill(EN.hub_in_slot, &[&(i + 1).to_string()]))
                }
                _ => row,
            }
        })
        .collect()
}

/// The ways in of slot `slot`, as the card offers them for its state:
/// Make a new key, Scan its xpub, Type or paste an xpub, Someone else adds
/// it later. The Inbox's files, the open vaults' keys and the passphrase
/// stay in the card (they are worked out in the drawing's memo, which the
/// pane's rows cannot reach).
pub(crate) fn way_rows(app: &crate::Faraday, c: &crate::CreateState, slot: usize) -> Vec<PickRow> {
    use crate::create::Source;
    let k = slot as u8;
    let src = &c.slots[slot];
    let making = app.keygen().is_some_and(|g| g.on_card && g.slot == Some(k));
    let multi = c.kind.multi();
    let scan = |rows: &mut Vec<PickRow>| {
        rows.push(PickRow::way_in(
            Icon::Scan,
            EN.create_scan_its_xpub,
            crate::Action::ScanFor(crate::Ask::Xpub(k + 1)),
        ));
    };
    let mut rows = Vec::new();
    if multi && *src == Source::Empty && !making {
        scan(&mut rows);
        rows.push(PickRow::way_in(
            Icon::Add,
            EN.create_new_words,
            crate::Action::KeyOnCard(k),
        ));
    } else if !making {
        rows.push(PickRow::way_in(
            Icon::Add,
            EN.create_new_key,
            crate::Action::KeyGen(Some(k)),
        ));
    }
    if multi && (*src == Source::Later || (*src == Source::Empty && making)) {
        scan(&mut rows);
    }
    if multi && matches!(src, Source::Empty | Source::Later) {
        rows.push(PickRow::way_in(
            Icon::Keyboard,
            EN.create_type_xpub,
            crate::Action::CXpubOpen(k),
        ));
    }
    if multi && *src == Source::Empty {
        rows.push(PickRow::way_in(
            Icon::Envelope,
            EN.create_someone_else,
            crate::Action::CSlotLater(k),
        ));
    }
    rows
}
