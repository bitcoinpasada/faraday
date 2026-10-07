//! Tools › Lightning node key (`docs/PLANNING.md` §16.116): what a
//! person typing an LND cipher seed, or choosing a key already loaded,
//! is told.

mod common;

use common::{ABANDON, Harness, PANEL};
use opensigner_core::{ScreenKind, ids};

/// An LND cipher seed at the scrypt cost a released LND uses, from
/// `tools/reference/aezeed/README.md`: no passphrase, birthday 0,
/// entropy `81b637d8…`.
const CIPHER_SEED: &str = "above judge emerge veteran reform crunch system all snap please \
    shoulder vault hurt city quarter cover enlist swear success suggest drink wagon enrich body";

/// The same seed under a passphrase, with a birthday of 3365 days.
const CIPHER_SEED_2: &str = "absorb century submit father path glove gloom super divert garden \
    ice mirror wisdom grass dice kit ugly castle success suggest drink monster congress flight";

/// What LND derives at `m/1017'/0'/6'/0/0` from that seed's entropy.
const NODE_KEY: &str = "024c7005923a074fd38b16ace7be4914ec9f929717692bfb585019be13290c9b1d";

/// Home › Tools › Lightning node key, at the source Choice.
fn open_tool(h: &mut Harness) {
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    assert_eq!(h.app.screen(), ScreenKind::Tools);
    h.tap(ids::TOOLS_LIGHTNING);
}

/// Types the twenty-four words of a cipher seed, then the passphrase.
fn read_seed(h: &mut Harness, words: &str, passphrase: &str) {
    open_tool(h);
    h.choose(ids::at(ids::PICK_BASE, 0), ids::PICK_CONTINUE);
    let words: Vec<&str> = words.split_whitespace().collect();
    // The last word ends the entry: the wizard hands its indices over
    // and is dropped, so the harness's own accept helper has no wizard
    // left to look at.
    for w in &words[..words.len() - 1] {
        h.type_word(w);
    }
    h.type_text(words[words.len() - 1]);
    if h.app.load_step().is_some() {
        h.key(osk_shell_api::Key::Enter);
    }
    assert_eq!(h.app.screen(), ScreenKind::Lightning);
    assert!(h.app.load_step().is_none(), "the word entry is done");
    if !passphrase.is_empty() {
        h.type_text(passphrase);
    }
    h.key(osk_shell_api::Key::Enter);
}

/// A key as a row or a panel draws it: groups of four (§4.5).
fn chunked(hex: &str) -> String {
    hex.as_bytes()
        .chunks(4)
        .map(|c| core::str::from_utf8(c).unwrap())
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn an_lnd_cipher_seed_names_its_node() {
    let mut h = Harness::new(PANEL);
    read_seed(&mut h, CIPHER_SEED, "");
    let head = chunked(&NODE_KEY[..8]);
    let labels = h.app.texts();
    assert!(
        labels.iter().any(|l| l.starts_with(&head)),
        "the node public key is on the Result: {labels:?}"
    );
    // The reference row opens it whole.
    h.tap(ids::LIGHTNING_NODE_KEY);
    assert!(
        h.app.texts().iter().any(|l| l.contains(NODE_KEY)),
        "the key, whole: {:?}",
        h.app.texts()
    );
    h.tap(ids::COMPARE_DONE);
    // Nothing was loaded: the tool reads, it does not add a key.
    h.tap(ids::LIGHTNING_DONE);
    assert!(h.app.fingerprints().is_empty(), "the tool adds no key");
}

#[test]
fn the_birthday_and_the_version_are_stated() {
    let mut h = Harness::new(PANEL);
    read_seed(&mut h, CIPHER_SEED_2, "!very_safe_55345_password*");
    let labels = h.app.texts();
    assert!(
        labels.iter().any(|l| l.contains("2018-03-22")),
        "the birthday is a day: {labels:?}"
    );
    assert!(
        labels.iter().any(|l| l.contains("3365")),
        "and the count the seed holds"
    );
}

#[test]
fn a_wrong_passphrase_is_refused() {
    let mut h = Harness::new(PANEL);
    read_seed(&mut h, CIPHER_SEED_2, "not the passphrase");
    let labels = h.app.texts();
    assert!(
        !labels.iter().any(|l| l.contains(&NODE_KEY[..16])),
        "no node key is offered"
    );
    assert!(
        labels.iter().any(|l| l.contains("Wrong passphrase")),
        "the screen says why: {labels:?}"
    );
    assert!(
        h.app.rect_of(ids::LIGHTNING_SECRET).is_none(),
        "and no secret behind it"
    );
}

#[test]
fn a_loaded_key_gives_the_key_its_lightning_node_would_use() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    open_tool(&mut h);
    h.choose(ids::at(ids::PICK_BASE, 1), ids::PICK_CONTINUE);
    assert_eq!(h.app.screen(), ScreenKind::Lightning);
    // ldk-node's node id for the `abandon…about` phrase with no
    // passphrase, from `tools/reference/aezeed/README.md`.
    let want = "027cf7c81dc777e46572ff964f17650a0f6f783319fcc140fb9a69425e2fbdc93c";
    let head = chunked(&want[..8]);
    assert!(
        h.app.texts().iter().any(|l| l.starts_with(&head)),
        "the node id is on the Result: {:?}",
        h.app.texts()
    );
    // The secret behind it is the node private key, not a seed: the
    // tool holds no words for this key beyond the ones already loaded.
    h.tap(ids::LIGHTNING_SECRET);
    assert!(
        h.app.rect_of(ids::LIGHTNING_ENTROPY).is_none(),
        "a loaded key has no cipher seed entropy"
    );
}
