//! §4.7 Amounts: one unit per build of the screen, never both.
//!
//! "Amounts are shown in sats or in BTC, by setting, never both" (§2.9).
//! [`amount`] takes the setting and returns one string; there is no
//! function that returns both, which is how the echo cannot come back.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::records::Record;

/// The unit a screen shows amounts in. Settled for this pass
/// (`docs/DESIGN.md` §7.5): the default is [`Unit::Sat`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Unit {
    /// Satoshis, grouped with a no-break space: "60 000 sats".
    #[default]
    Sat,
    /// Whole bitcoin at eight decimal places: "0.00060000 BTC".
    Btc,
}

/// Satoshi per bitcoin.
const SAT_PER_BTC: u64 = 100_000_000;

/// §4.7 Amount: "In the unit the setting names: '60 000 sats' or
/// '0.00060000 BTC', grouped with a no-break space. The setting's rows
/// read 'sats' and 'BTC'. No echo in the other unit."
pub fn amount(sat: u64, unit: Unit) -> String {
    match unit {
        Unit::Sat => {
            let mut out = group(sat);
            out.push(' ');
            out.push_str("sats");
            out
        }
        Unit::Btc => {
            let whole = sat / SAT_PER_BTC;
            let rest = sat % SAT_PER_BTC;
            format!("{}.{rest:08} BTC", group(whole))
        }
    }
}

/// Satoshi per millibitcoin.
const SAT_PER_MBTC: u64 = 100_000;

/// Satoshi per bit (microbitcoin).
const SAT_PER_BIT: u64 = 100;

/// A unit an amount can be written in, for the Units calculator. The
/// two a review screen shows are [`Unit`]'s; these are the four the
/// calculator converts between, and nothing outside it reads them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Denomination {
    /// Satoshis.
    #[default]
    Sat,
    /// Whole bitcoin, eight decimal places.
    Btc,
    /// Millibitcoin, five decimal places.
    MBtc,
    /// Bits (microbitcoin), two decimal places.
    Bits,
}

impl Denomination {
    /// The four, in the order the mode row lists them.
    pub const ALL: [Denomination; 4] = [
        Denomination::Sat,
        Denomination::Btc,
        Denomination::MBtc,
        Denomination::Bits,
    ];

    /// How many satoshi one of these is.
    pub fn sat(self) -> u64 {
        match self {
            Denomination::Sat => 1,
            Denomination::Btc => SAT_PER_BTC,
            Denomination::MBtc => SAT_PER_MBTC,
            Denomination::Bits => SAT_PER_BIT,
        }
    }
}

/// `sat` written in `denomination`, grouped the way [`amount`] groups
/// the two units a review screen shows, and carrying no unit name: the
/// row's label says which unit it is.
pub fn denominated(sat: u64, denomination: Denomination) -> String {
    let per = denomination.sat();
    if per == 1 {
        return group(sat);
    }
    let places = match denomination {
        Denomination::Btc => 8,
        Denomination::MBtc => 5,
        Denomination::Bits => 2,
        Denomination::Sat => 0,
    };
    format!("{}.{:0width$}", group(sat / per), sat % per, width = places)
}

/// §4.7 Fee, Rate: "Two record rows: 'Fee · 1 000 sats' and 'Rate · 7.1
/// sat/vB'. The rate row is absent when unknown."
///
/// Two facts, two rows: §4.11 says a value never carries two, and a fee
/// and a rate are read for different reasons — one is what this
/// transaction costs, the other is whether it will confirm. The labels
/// are the caller's, because this crate has no wording of its own.
pub fn fee_rows(
    fee_label: impl Into<String>,
    rate_label: impl Into<String>,
    sat: u64,
    unit: Unit,
    rate_sat_per_vb: Option<f32>,
) -> Vec<Record> {
    let mut rows = alloc::vec![Record::mono(fee_label, amount(sat, unit))];
    if let Some(r) = rate_sat_per_vb {
        rows.push(Record::mono(rate_label, rate(r)));
    }
    rows
}

/// The fee rate as §4.7 writes it: "7.1 sat/vB", one decimal place.
pub fn rate(sat_per_vb: f32) -> String {
    format!("{sat_per_vb:.1} sat/vB")
}

/// A whole number in groups of three, thin-space separated.
fn group(value: u64) -> String {
    let digits = format!("{value}");
    let mut out = String::new();
    let n = digits.len();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (n - i) % 3 == 0 {
            out.push(crate::tokens::GROUP_SEPARATOR);
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The label of a record row, for a test that reads the rows a fee
    /// turns into.
    fn label_of(row: &Record) -> &str {
        match row {
            Record::Fact { label, .. }
            | Record::Reference { label, .. }
            | Record::Value { label, .. }
            | Record::OneLine { label, .. }
            | Record::Stacked { label, .. }
            | Record::Toggle { label, .. }
            | Record::Glyph { label, .. } => label,
            Record::Action { label, .. } | Record::Dimmed { label, .. } => label,
            Record::Badge(_) => "",
        }
    }

    /// §2.9 "One unit": nothing prints both.
    #[test]
    fn an_amount_names_one_unit_and_never_both() {
        for sat in [0, 1, 999, 60_000, 100_000_000, 2_100_000_000_000_000] {
            for unit in [Unit::Sat, Unit::Btc] {
                let s = amount(sat, unit);
                assert!(
                    !(s.contains("sats") && s.contains("BTC")),
                    "{s} names two units"
                );
            }
            for row in fee_rows("Fee", "Rate", sat, Unit::Sat, Some(7.05)) {
                assert!(!label_of(&row).contains("BTC"), "{}", label_of(&row));
            }
        }
    }

    #[test]
    fn amounts_group_in_threes_and_keep_eight_decimals() {
        assert_eq!(amount(60_000, Unit::Sat), "60\u{00A0}000 sats");
        assert_eq!(amount(999, Unit::Sat), "999 sats");
        assert_eq!(amount(1_234_567, Unit::Sat), "1\u{00A0}234\u{00A0}567 sats");
        assert_eq!(amount(60_000, Unit::Btc), "0.00060000 BTC");
        assert_eq!(amount(100_000_000, Unit::Btc), "1.00000000 BTC");
        assert_eq!(
            amount(1_234_500_000_000, Unit::Btc),
            "12\u{00A0}345.00000000 BTC"
        );
    }

    /// §4.7: "Two record rows ... The rate row is absent when unknown."
    #[test]
    fn the_rate_row_goes_when_the_rate_is_unknown() {
        assert_eq!(rate(7.14), "7.1 sat/vB");
        let one = fee_rows("Fee", "Rate", 1_000, Unit::Sat, None);
        assert_eq!(one.len(), 1);
        assert_eq!(label_of(&one[0]), "Fee");
    }
}
