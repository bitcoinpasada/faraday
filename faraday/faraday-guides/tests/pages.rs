//! Every id the Starter opens a guide by has a page
//! here (`docs/STARTER.md` §7.2, §7.6): every maker's, and the general
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

// The guides to starting a computer from the stick (docs/STARTER.md §7.2),
// which the Starter's screens open by id: each id finds its page, and a
// guide puts what has to be in hand before any firmware step first.
// `replace-faraday` is faraday-learn's, found through `page`'s fallback.

fn guide(id: &str) -> &'static osk_learn::Page {
    faraday_guides::page(id).unwrap_or_else(|| panic!("no page {id}"))
}

#[test]
fn every_guide_the_starter_links_to_is_there() {
    let titles: Vec<&str> = faraday_guides::BOOT
        .iter()
        .map(|id| guide(id).title)
        .collect();
    assert_eq!(
        titles,
        [
            "Starting a Windows computer from the stick",
            "Starting a Linux computer from the stick",
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
fn the_windows_guide_says_the_last_step_turns_secure_boot_back_on_before_any_bios_step() {
    let first = &guide("windows").sections[0];
    assert_eq!(first.heading, "Before you start");
    assert_eq!(
        first.paragraphs[0],
        "The last step turns Secure Boot back on, before Windows starts. With Secure Boot back on, \
         Windows starts as usual and asks for nothing."
    );
}

#[test]
fn the_linux_guide_asks_for_the_tpm_unlocked_disk_s_passphrase_first() {
    let first = &guide("linux").sections[0];
    assert_eq!(first.heading, "Before you start");
    assert!(first.paragraphs[0].contains("passphrase or recovery key"));
}

// The guides a person follows at the computer are numbered steps
// (docs/STARTER.md §7): every maker's page, Other's, and the Windows
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

#[test]
fn every_guide_ends_with_the_step_that_turns_secure_boot_back_on() {
    for id in step_pages() {
        let page = guide(id);
        let last = faraday_learn::steps(page).len();
        let text = page
            .sections
            .iter()
            .flat_map(|s| s.paragraphs.iter().copied())
            .find(|p| p.starts_with(&format!("{last}. ")))
            .unwrap_or_else(|| panic!("{id}: no step {last}"));
        let body = text.split_once(". ").expect("a step").1;
        assert!(body.starts_with("Turn Secure Boot back on"), "{id}: {text}");
        let ending = if id == "linux" {
            "Do it before the computer's own system starts."
        } else {
            "Do it before Windows starts."
        };
        assert!(text.ends_with(ending), "{id}: {text}");
    }
}

#[test]
fn no_guide_offers_a_key_backup_a_suspension_or_a_resume() {
    for id in faraday_guides::ids() {
        let all = guide(id)
            .sections
            .iter()
            .flat_map(|s| s.paragraphs.iter().copied())
            .collect::<Vec<_>>()
            .join(" ");
        for word in [
            "Resume BitLocker",
            "suspended BitLocker",
            "BitLocker page",
            "back it up",
        ] {
            assert!(!all.contains(word), "{id}: {word}");
        }
    }
    assert!(faraday_guides::page("bitlocker").is_none());
}

#[test]
fn the_windows_guide_and_each_maker_page_say_where_windows_keeps_the_recovery_key() {
    for id in faraday_learn::MAKERS
        .iter()
        .map(|m| m.id)
        .chain(["maker-other", "windows"])
    {
        let different = guide(id)
            .sections
            .iter()
            .find(|s| s.heading == "If something is different")
            .unwrap_or_else(|| panic!("{id}: no such section"));
        assert!(
            different.paragraphs[0].starts_with("If Windows asks for the BitLocker recovery key:")
                && different.paragraphs[0].contains("aka.ms/myrecoverykey"),
            "{id}"
        );
    }
}

// The BIOS key the power sheet shows for a computer (docs/FLOWS.md,
// Restart; F42) is the one its maker page gives.

/// The keys a table value names: "Esc, then F10" is Esc and F10;
/// "F2 (or Fn+F2)" is F2 and Fn+F2; ", held" asks for "Hold <key>".
fn keys_of(value: &str) -> (Vec<String>, bool) {
    let held = value.ends_with(", held");
    let value = value.trim_end_matches(", held");
    let keys = value
        .replace(" (or ", ", then ")
        .replace(')', "")
        .split(", then ")
        .map(str::to_string)
        .collect();
    (keys, held)
}

#[test]
fn the_bios_key_table_agrees_with_the_maker_pages() {
    for maker in faraday_learn::MAKERS {
        let (keys, held) = keys_of(maker.bios_key);
        let page = guide(maker.id);
        let steps: Vec<&str> = page
            .sections
            .iter()
            .flat_map(|s| s.paragraphs.iter().copied())
            .filter(|p| faraday_learn::step(p).is_some())
            .collect();
        assert!(
            steps.iter().any(|p| {
                keys.iter().all(|k| p.contains(k.as_str()))
                    && (!held || p.contains(&format!("Hold {}", keys[0])))
            }),
            "{}: no step gives {:?}",
            maker.id,
            maker.bios_key
        );
    }
}
