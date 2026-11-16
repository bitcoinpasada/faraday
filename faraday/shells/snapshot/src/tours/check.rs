//! Check (`docs/DESIGN.md` §5, Check): the list of rows, the forms of a
//! backup in another form, and New key opened on its dice source.

use crate::tour::Tour;
use faraday_core::check::{Press, Row};
use faraday_core::{Action, Screen, testkit};
use faraday_shell_api::App;

pub(crate) fn check_tour(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Nav(Screen::Check));
    t.shot("check")?;
    t.press(Action::Check(Press::Row(Row::Backup)));
    t.shot("check-backups")?;
    t.press(Action::Cancel);
    t.press(Action::Check(Press::Row(Row::Dice)));
    t.shot("check-dice")?;
    address_found(t)?;
    message_at_path(t)
}

/// An address of a single-key wallet whose key is loaded, found: the
/// result offers Sign a message with this address.
fn address_found(t: &mut Tour) -> Result<(), String> {
    use faraday_bip::policy::WalletPolicy;
    let c = testkit::compute();
    t.app.session = testkit::session();
    t.app
        .session
        .keys
        .add_words(
            &c,
            &testkit::test_words(testkit::TEST_SEEDS[0].0),
            "Bacon",
            None,
        )
        .map_err(|e| format!("{e:?}"))?;
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .ok_or("no spending wallet")?;
    t.app
        .session
        .add_wallet_confirmed(
            &c,
            kit.name,
            &kit.descriptor,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .map_err(|e| format!("{e:?}"))?;
    let policy = WalletPolicy::parse_any(&c, &kit.descriptor).map_err(|e| format!("{e:?}"))?;
    let address = policy
        .address_at(&c, testkit::net(), false, 2)
        .map_err(|e| format!("{e:?}"))?
        .to_string();
    t.press(Action::Nav(Screen::Check));
    t.press(Action::Check(Press::Row(Row::Address)));
    crate::tour::type_text(t, &address);
    t.app
        .event(faraday_shell_api::Event::Key(faraday_shell_api::Key::Enter));
    t.pump();
    t.press(Action::Osk(faraday_core::OskPress::Hide));
    t.shot("check-address-found")
}

/// An address of a loaded key's own account with no wallet, found, then
/// Sign a message with this address pressed: Message opens on the path.
fn message_at_path(t: &mut Tour) -> Result<(), String> {
    use faraday_bip::policy::WalletPolicy;
    use faraday_core::address_check::AddressPress;
    use faraday_core::create::NewKind;
    use faraday_core::scan_read::Way;
    let c = testkit::compute();
    t.app.session = testkit::session();
    t.app
        .session
        .keys
        .add_words(
            &c,
            &testkit::test_words(testkit::TEST_SEEDS[0].0),
            "Bacon",
            None,
        )
        .map_err(|e| format!("{e:?}"))?;
    let kind = NewKind::NativeSegwit;
    let spec = kind
        .spec(t.app.session.network(), 0)
        .map_err(|e| format!("{e:?}"))?;
    let key = t
        .app
        .session
        .keys
        .key_text(&c, 0, &spec)
        .ok_or("no key")?
        .map_err(|e| format!("{e:?}"))?;
    let descriptor = String::from(&*kind.descriptor(1, &[key.as_str()]));
    let policy = WalletPolicy::parse_any(&c, &descriptor).map_err(|e| format!("{e:?}"))?;
    let address = policy
        .address_at(&c, testkit::net(), false, 0)
        .map_err(|e| format!("{e:?}"))?
        .to_string();
    t.press(Action::Nav(Screen::Check));
    t.press(Action::Check(Press::Row(Row::Address)));
    crate::tour::type_text(t, &address);
    t.app
        .event(faraday_shell_api::Event::Key(faraday_shell_api::Key::Enter));
    t.pump();
    t.press(Action::Osk(faraday_core::OskPress::Hide));
    let ways = t
        .app
        .address_check_state()
        .map(|p| p.ways().to_vec())
        .unwrap_or_default();
    let i = ways
        .iter()
        .position(|w| matches!(w, Way::SignAddress))
        .ok_or("no Sign a message way")?;
    t.press(Action::AddressCheck(AddressPress::Way(i as u8)));
    t.shot("check-message-at-path")?;
    t.press(Action::MStep(0));
    t.shot("check-message-address")
}
