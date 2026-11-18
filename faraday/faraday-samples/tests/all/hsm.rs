//! Core Lightning's `hsm_secret` in Practice: Use a sample puts the three
//! sample files in Files, and each gives the node id the samples crate
//! names; a real node's file is refused.

use crate::common::demo_app;
use faraday_core::{Action, Faraday, testkit};
use faraday_samples::SAMPLES;
use faraday_shell_api::{App, Event, Key};

fn at_the_source() -> Faraday {
    let mut app = demo_app();
    app.press(Action::Lightning);
    app.press(Action::LCln(None));
    let _ = app.frame();
    app.press(Action::UseSample);
    app
}

/// The place of the `LFile` button the screen draws for the file `name`.
fn listed(app: &mut Faraday, name: &str) -> usize {
    let at = app
        .inbox()
        .iter()
        .position(|i| i.name == name)
        .expect("the sample file is in Files");
    let _ = app.frame();
    assert!(app.offers(Action::LFile(at)), "{name} is not listed");
    at
}

fn pick(app: &mut Faraday, name: &str) {
    let at = listed(app, name);
    app.press(Action::LFile(at));
    testkit::settle(app);
}

#[test]
fn use_a_sample_lists_three_files_and_each_gives_its_node_id() {
    let mut app = at_the_source();
    for (name, h) in ["hsm_secret", "hsm_secret-encrypted", "hsm_secret-words"]
        .into_iter()
        .zip(SAMPLES.inputs.hsm)
    {
        pick(&mut app, name);
        if name.ends_with("encrypted") {
            app.press(Action::LPassphrase);
            for c in "faraday".chars() {
                app.event(Event::Key(Key::Char(c)));
            }
            app.event(Event::Key(Key::Enter));
            let _ = app.frame();
            let now = app.now_ms() + 16;
            app.event(Event::Tick { now_ms: now });
            testkit::settle(&mut app);
        }
        let l = app.lightning().unwrap();
        let n = l
            .node
            .as_ref()
            .unwrap_or_else(|| panic!("{name}: {:?}", l.error));
        assert_eq!(n.public(), &h.node, "{name}");
    }
}

#[test]
fn a_file_that_is_not_a_sample_is_refused_in_practice() {
    let mut app = at_the_source();
    testkit::inbox_file(&mut app, "real", vec![1u8; 32]);
    let at = app.inbox().len() - 1;
    app.press(Action::LFile(at));
    testkit::settle(&mut app);
    let l = app.lightning().unwrap();
    assert!(l.node.is_none());
    assert!(
        l.error
            .as_deref()
            .is_some_and(|e| e.ends_with("is not a sample: Practice takes only samples"))
    );
}

#[test]
fn the_sample_words_give_a_public_node_with_none_and_with_faraday() {
    let mut app = at_the_source();
    app.press(Action::LResolve);
    testkit::settle(&mut app);
    let l = app.lightning().unwrap();
    assert!(l.node.is_some(), "none: {:?}", l.error);
    let _ = app.frame();
    app.press(Action::PassChip(faraday_core::demo::PassField::Aezeed));
    app.press(Action::LResolve);
    testkit::settle(&mut app);
    let l = app.lightning().unwrap();
    let n = l
        .node
        .as_ref()
        .unwrap_or_else(|| panic!("faraday: {:?}", l.error));
    assert!(SAMPLES.inputs.hsm_words_nodes.contains(n.public()));
}

#[test]
fn the_words_of_another_sample_key_give_a_public_node_typed() {
    let mut app = demo_app();
    app.press(Action::Lightning);
    app.press(Action::LCln(None));
    let words = SAMPLES.keys[1].parts[0];
    for ch in words.chars() {
        app.event(Event::Key(Key::Char(ch)));
    }
    app.event(Event::Key(Key::Enter));
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    testkit::settle(&mut app);
    let l = app.lightning().unwrap();
    let n = l.node.as_ref().unwrap_or_else(|| panic!("{:?}", l.error));
    assert!(SAMPLES.inputs.hsm_words_nodes.contains(n.public()));
}

/// Lightning's Core Lightning source on `network`, the sample file number
/// `n` in Files, chosen (its passphrase typed when it takes one).
fn sample_file_on(network: faraday_bip::keys::Network, n: usize) -> Faraday {
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, network).unwrap();
    testkit::inbox_file(&mut app, "hsm_secret", SAMPLES.inputs.hsm[n].bytes.to_vec());
    app.press(Action::Lightning);
    app.press(Action::LCln(None));
    let _ = app.frame();
    let at = app.inbox().len() - 1;
    app.press(Action::LFile(at));
    testkit::settle(&mut app);
    if n == 1 {
        app.press(Action::LPassphrase);
        for c in "faraday".chars() {
            app.event(Event::Key(Key::Char(c)));
        }
        app.event(Event::Key(Key::Enter));
        let _ = app.frame();
        let now = app.now_ms() + 16;
        app.event(Event::Tick { now_ms: now });
        testkit::settle(&mut app);
    }
    app
}

#[test]
fn core_lightnings_sample_files_are_refused_on_mainnet_and_taken_on_testnet() {
    use faraday_bip::keys::Network;
    for n in 0..SAMPLES.inputs.hsm.len() {
        let app = sample_file_on(Network::Mainnet, n);
        let l = app.lightning().unwrap();
        assert!(l.node.is_none(), "file {n}: Mainnet holds the node");
        assert_eq!(
            l.error.as_deref(),
            Some(faraday_core::strings::EN.published_sample_node),
            "file {n}"
        );
        let app = sample_file_on(Network::Testnet, n);
        let l = app.lightning().unwrap();
        let node = l
            .node
            .as_ref()
            .unwrap_or_else(|| panic!("file {n}: {:?}", l.error));
        assert_eq!(node.public(), &SAMPLES.inputs.hsm[n].node, "file {n}");
    }
}

#[test]
fn a_sample_keys_words_typed_as_core_lightning_words_are_refused_on_mainnet_and_read_on_testnet() {
    use faraday_bip::keys::Network;
    let typed = |network: Network, words: &str| {
        let mut app = testkit::started();
        testkit::move_to(&mut app.session, network).unwrap();
        app.press(Action::Lightning);
        app.press(Action::LCln(None));
        let _ = app.frame();
        for ch in words.chars() {
            app.event(Event::Key(Key::Char(ch)));
        }
        app.event(Event::Key(Key::Enter));
        let _ = app.frame();
        let now = app.now_ms() + 16;
        app.event(Event::Tick { now_ms: now });
        testkit::settle(&mut app);
        app
    };
    let words = SAMPLES.keys[1].parts[0];
    let app = typed(Network::Mainnet, words);
    let l = app.lightning().unwrap();
    assert!(l.node.is_none(), "Mainnet holds the node");
    assert_eq!(
        l.error.as_deref(),
        Some(faraday_core::strings::EN.published_sample_node)
    );
    let app = typed(Network::Testnet, words);
    let l = app.lightning().unwrap();
    let n = l.node.as_ref().unwrap_or_else(|| panic!("{:?}", l.error));
    assert!(SAMPLES.inputs.hsm_words_nodes.contains(n.public()));
}

#[test]
fn an_hsm_secret_file_of_a_sample_keys_words_is_refused_on_mainnet_and_read_on_testnet() {
    use faraday_bip::keys::Network;
    let mut file = vec![0u8; 32];
    file.extend_from_slice(SAMPLES.keys[1].parts[0].as_bytes());
    let opened = |network: Network| {
        let mut app = testkit::started();
        testkit::move_to(&mut app.session, network).unwrap();
        testkit::inbox_file(&mut app, "hsm_secret", file.clone());
        app.press(Action::Lightning);
        app.press(Action::LCln(None));
        let _ = app.frame();
        let at = app.inbox().len() - 1;
        app.press(Action::LFile(at));
        testkit::settle(&mut app);
        app
    };
    let app = opened(Network::Mainnet);
    let l = app.lightning().unwrap();
    assert!(l.node.is_none(), "Mainnet holds the node");
    assert_eq!(
        l.error.as_deref(),
        Some(faraday_core::strings::EN.published_sample_node)
    );
    let app = opened(Network::Testnet);
    let l = app.lightning().unwrap();
    let n = l.node.as_ref().unwrap_or_else(|| panic!("{:?}", l.error));
    assert!(SAMPLES.inputs.hsm_words_nodes.contains(n.public()));
}

#[test]
fn an_hsm_secret_file_of_the_seed_xor_samples_combined_words_is_refused_on_mainnet_and_read_on_testnet()
 {
    use faraday_bip::keys::Network;
    let mut file = vec![0u8; 32];
    file.extend_from_slice(crate::common::key("sample_xor").combined.as_bytes());
    let opened = |network: Network| {
        let mut app = testkit::started();
        testkit::move_to(&mut app.session, network).unwrap();
        testkit::inbox_file(&mut app, "hsm_secret", file.clone());
        app.press(Action::Lightning);
        app.press(Action::LCln(None));
        let _ = app.frame();
        let at = app.inbox().len() - 1;
        app.press(Action::LFile(at));
        testkit::settle(&mut app);
        app
    };
    let app = opened(Network::Mainnet);
    let l = app.lightning().unwrap();
    assert!(l.node.is_none(), "Mainnet holds the node");
    assert_eq!(
        l.error.as_deref(),
        Some(faraday_core::strings::EN.published_sample_node)
    );
    let app = opened(Network::Testnet);
    let l = app.lightning().unwrap();
    let n = l.node.as_ref().unwrap_or_else(|| panic!("{:?}", l.error));
    assert!(SAMPLES.inputs.hsm_words_nodes.contains(n.public()));
}
