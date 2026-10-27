//! The inbox tour.

use crate::tour::{Tour, scroll};
use faraday_core::{Action, Screen};
use faraday_shell_api::App;

/// Stick visit with files copied in from another stick offered under
/// "From the Inbox".
pub(crate) fn inbox_tour(t: &mut Tour) -> Result<(), String> {
    use faraday_core::wallet::FileKind as K;
    let item = |name: &str, kind: K, len: usize, picture: Option<K>| faraday_core::Item {
        name: name.into(),
        bytes: vec![b'#'; len],
        kind,
        secret: false,
        picture,
        sha: None,
        wallet: None,
        psbt: None,
    };
    t.app
        .outbox
        .push(item("savings-signed.psbt", K::Psbt, 1400, None));
    for i in [
        item("family-savings-descriptor.txt", K::Wallet, 620, None),
        item("household.ofv", K::Vault, 4_200_000, None),
        item("savings-photo-qr.png", K::Other, 2900, Some(K::Wallet)),
        item("seed-3-words.txt", K::Words, 160, None),
        item("recovery-codes.txt", K::Text, 210, None),
    ] {
        faraday_core::testkit::inbox_push(&mut t.app, i);
    }
    t.sticks(true);
    t.press(Action::Nav(Screen::Visit));
    if t.app.is_compact() {
        // Down past the stick's files to the write list.
        for _ in 0..400 {
            let _ = t.app.frame();
            if t.app.offers(Action::VisitInbox(4)) {
                break;
            }
            scroll(t, 200);
            t.app.settle();
        }
    }
    t.shot("inbox-offered")?;
    t.press(Action::VisitOutAll);
    t.shot("inbox-select-all")?;
    t.press(Action::VisitInbox(4));
    t.shot("inbox-text-sheet")?;
    if t.app.is_compact() {
        // The small panel's sheet scrolls to its acknowledgement.
        scroll(t, 400);
        t.shot("inbox-text-sheet-foot")?;
    }
    t.press(Action::SecretUnprotected);
    t.shot("inbox-text-plain")?;
    t.press(Action::SecretHold);
    t.press(Action::VisitInbox(3));
    t.shot("inbox-words-sheet")?;
    t.press(Action::Cancel);
    t.shot("inbox-chosen")?;
    Ok(())
}
