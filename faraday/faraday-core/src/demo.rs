//! The demo's public seeds (F20). A session set up for the demo
//! ([`crate::wallet::Session::set_demo`]) takes only these on mainnet;
//! on a test network it takes any seed.

/// The public seeds, as their words: each of these words, twenty-four
/// times, is a valid BIP-39 phrase.
pub const SEEDS: [&str; 11] = [
    "bacon", "flag", "gas", "great", "slice", "solution", "summer", "they", "trade", "trap",
    "zebra",
];

/// A public seed's phrase: its word twenty-four times.
pub fn words(word: &str) -> String {
    vec![word; 24].join(" ")
}
