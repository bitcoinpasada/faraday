//! Every id the hub app and the Companion open a guide by has a page
//! here (`docs/HUB.md` §7.2, §7.6): every maker's, and the general
//! pages the stick's QR names an address for (`windows`, `linux`,
//! `mac-intel`).

#[test]
fn every_maker_and_general_guide_has_a_page() {
    for id in faraday_learn::MAKERS
        .iter()
        .map(|m| m.id)
        .chain(["windows", "linux", "mac-intel"])
    {
        assert!(faraday_guides::page(id).is_some(), "{id}");
    }
}

// The guides to starting a computer from the stick (docs/HUB.md §7.2),
// which the hub app's screens open by id: each id finds its page, and a
// guide puts what has to be in hand before any firmware step first.
// `replace-faraday` is faraday-learn's, found through `page`'s fallback.

fn guide(id: &str) -> &'static osk_learn::Page {
    faraday_guides::page(id).unwrap_or_else(|| panic!("no page {id}"))
}

#[test]
fn every_guide_the_hub_links_to_is_there() {
    let titles: Vec<&str> = faraday_guides::BOOT
        .iter()
        .map(|id| guide(id).title)
        .collect();
    assert_eq!(
        titles,
        [
            "Starting a Windows computer from the stick",
            "Starting a Linux computer from the stick",
            "BitLocker and starting from a stick",
            "What Secure Boot and BitLocker are",
            "Screens and devices that can record",
            "When the stick does not start",
            "Replacing Faraday on a stick",
            "Starting an Intel Mac from the stick",
        ]
    );
    assert!(faraday_guides::page("no-such-page").is_none());
}

#[test]
fn the_boot_entry_page_is_there() {
    assert_eq!(guide("boot-entry").title, "The Faraday boot entry on Linux");
}

#[test]
fn the_windows_guide_does_the_bitlocker_step_before_any_firmware_step() {
    let first = &guide("windows").sections[0];
    assert_eq!(first.heading, "Before you start");
    assert!(first.paragraphs[0].starts_with(
        "Do the BitLocker step in Faraday Starter on this computer before changing any firmware setting."
    ));
}

#[test]
fn the_linux_guide_asks_for_the_tpm_unlocked_disk_s_passphrase_first() {
    let first = &guide("linux").sections[0];
    assert_eq!(first.heading, "Before you start");
    assert!(first.paragraphs[0].contains("passphrase or recovery key"));
}

// The guides a person follows at the computer are numbered steps
// (docs/COMPANION.md §4.7): every maker's page, Other's, and the Windows
// and Linux guides.

/// The pages written as steps.
fn step_pages() -> impl Iterator<Item = &'static str> {
    faraday_learn::MAKERS
        .iter()
        .map(|m| m.id)
        .chain(["maker-other", "windows", "linux"])
}

#[test]
fn every_guide_followed_at_the_computer_is_numbered_steps_from_one() {
    for id in step_pages() {
        let steps = faraday_learn::steps(guide(id));
        assert!(steps.len() >= 4, "{id}: {steps:?}");
        let expected: Vec<u32> = (1..=steps.len() as u32).collect();
        assert_eq!(steps, expected, "{id}");
    }
}

#[test]
fn the_page_for_another_maker_gives_the_common_keys_and_says_to_search() {
    let p = guide("maker-other");
    let text: Vec<&str> = p
        .sections
        .iter()
        .flat_map(|s| s.paragraphs.iter().copied())
        .collect();
    let all = text.join(" ");
    for key in ["F2", "F12", "Delete", "Esc"] {
        assert!(all.contains(key), "{key}");
    }
    assert!(all.contains("Search online for your computer's model and \"boot menu key\""));
    // It names no maker's computer: it is for the ones without a page.
    for name in [
        "Dell",
        "HP",
        "Lenovo",
        "ThinkPad",
        "IdeaPad",
        "ASUS",
        "Acer",
        "MSI",
        "Framework",
        "Surface",
    ] {
        assert!(
            !all.split(|c: char| !c.is_ascii_alphanumeric())
                .any(|w| w == name),
            "{name}"
        );
    }
}
