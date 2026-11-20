//! The open-tools tour (`docs/DESIGN.md` §4, Nothing to work on): Decode a
//! transaction and Check a signed message opened from Tools with nothing
//! waiting, with several files waiting, and a result with its ending; the
//! key tools (Explore a key, Child seeds and passwords, Vanity address, Silent payments)
//! opened with no key loaded.

use crate::tour::Tour;
use faraday_core::{Action, Screen, StorageEvent};

/// The first transaction ever sent, as the hex of its bytes.
const TX: &str = "0100000001c997a5e56e104102fa209c6a852dd90660a20b2d9c352423edce25857fcd3704000000004847304402204e45e16932b8af514961a1d3a1a25fdf3f4f7732e9d624c6c61548ab5fb8cd410220181522ec8eca07de4860a4acdd12909d831cc56cbbac4622082221a8768d1d0901ffffffff0200ca9a3b00000000434104ae1a62fe09c5f51b13905f07f06b99a2f7159b2225f374cd378d71302fa28414e7aab37397f554a7df5f142c21c1b7303b8a0626f1baded5c72a704f7e6cd84cac00286bee0000000043410411db93e1dcdb8a016b49840f8c53bc1eb68a382e97b1482ecad7b148a6909a5cb2e0eaddfb84ccf9744464f82e160bfa9b8b64f9d4c03f999b8643f656b412a3ac00000000";

/// A signed message in the three-line form.
const MESSAGE: &str = "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq\nH9L5yLFjti0QTHhPyFrZCT1V/MMnBtXKmoiKDZ78NDBjERki6ZTQZdSMCtkgoNmp17By9ItJr8o7ChX0XxY91nk=\nHello\n";

fn tile(name: &str) -> Result<u8, String> {
    faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == name)
        .map(|k| k as u8)
        .ok_or_else(|| format!("no {name} tile"))
}

pub(crate) fn open_tools_tour(t: &mut Tour) -> Result<(), String> {
    let decode = tile("Decode a transaction")?;
    let check = tile("Check a signed message")?;
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Catalog(decode));
    t.shot("decode-empty")?;
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Catalog(check));
    t.shot("message-empty")?;
    for (name, shot) in [
        ("Explore a key", "explore-no-key"),
        ("Child seeds and passwords", "bip85-no-key"),
        ("Vanity address", "vanity-no-key"),
        ("Silent payments", "silent-no-key"),
    ] {
        let k = tile(name)?;
        t.press(Action::Nav(Screen::Catalog));
        t.press(Action::Catalog(k));
        t.shot(shot)?;
    }
    t.app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            ("first.txt".to_string(), TX.as_bytes().to_vec()),
            ("second.txt".to_string(), TX.as_bytes().to_vec()),
            ("a.txt".to_string(), MESSAGE.as_bytes().to_vec()),
            ("b.txt".to_string(), MESSAGE.as_bytes().to_vec()),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Catalog(decode));
    t.shot("decode-list")?;
    t.press(Action::DecodeInbox(1));
    t.shot("decode-result")?;
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Catalog(check));
    t.shot("message-list")?;
    t.press(Action::CheckMessage(3));
    t.shot("message-result")?;
    Ok(())
}
