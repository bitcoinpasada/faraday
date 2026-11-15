//! The address check's result (`docs/DESIGN.md` §5, Check an address,
//! "The result"): what the card says of an address, worked out once when
//! the answer arrives and again as a look further goes on, from the
//! [`Look`] the card holds; its ways over come with it. Drawing reads
//! the [`Answer`] and works nothing out.

use super::look::FoundIn;
use super::{Look, LookWay};
use crate::scan_read::{ReadCard, Way};
use crate::strings::EN;
use faraday_bip::keys::ScriptType;
use faraday_text::Text;

/// One fact of the card: its label and the lines of its value, in the
/// mono face where it is an address, a path or a fingerprint.
pub(crate) struct Row {
    /// The fact's label.
    pub(crate) label: &'static str,
    /// The value's lines, each wrapped on its own.
    pub(crate) lines: Vec<Text>,
    /// Whether the value is in the mono face.
    pub(crate) mono: bool,
}

/// What the result card says.
pub(crate) struct Answer {
    /// Found (`OK`) or not (`WARN`).
    pub(crate) found: bool,
    /// The Result line.
    pub(crate) head: Text,
    /// The facts, the address first.
    pub(crate) rows: Vec<Row>,
    /// How far a look under way is, and its depth, for the indicator
    /// under **Searched**.
    pub(crate) looking: Option<(u32, u32)>,
}

fn row(label: &'static str, value: Text, mono: bool) -> Row {
    Row {
        label,
        lines: vec![value],
        mono,
    }
}

/// A key's line with a no-break space after the dot, so that where it
/// does not fit it wraps after the fingerprint.
fn keep_path(line: &Text) -> Text {
    Text::replaced(line, " · ", " ·\u{a0}")
}

/// "Native SegWit": a script's name as the account line says it.
fn script_name(t: ScriptType) -> Text {
    let name = match t {
        ScriptType::Legacy => EN.wallet_legacy,
        ScriptType::NestedSegwit => EN.wallet_nested_segwit,
        ScriptType::NativeSegwit => EN.wallet_native_segwit,
        ScriptType::Taproot => EN.wallet_taproot,
    };
    let mut chars = name.chars();
    let mut out = Text::new();
    if let Some(first) = chars.next() {
        out.extend(first.to_uppercase());
    }
    out.push_str(chars.as_str());
    out
}

/// What a miss says it searched: how far, and in what.
fn searched(look: &Look) -> Text {
    let looked = Text::num(u64::from(look.looked));
    let (w, a) = (look.wallets as u64, look.account_count() as u64);
    let wallets = if w == 1 {
        Text::of(EN.address_check_wallet_one)
    } else {
        Text::fill(EN.address_check_wallets, &[&Text::num(w)])
    };
    let accounts = if a == 1 {
        Text::of(EN.address_check_account_one)
    } else {
        Text::fill(EN.address_check_accounts, &[&Text::num(a)])
    };
    let of = match (w, a) {
        (1, 0) => look.only.clone().unwrap_or_default(),
        (_, 0) => wallets,
        (0, _) => accounts,
        _ => Text::fill(EN.address_check_wallets_accounts, &[&wallets, &accounts]),
    };
    Text::fill(EN.address_check_searched_value, &[&looked, &of])
}

/// The card's answer and ways over made again from the look it holds.
pub(crate) fn refresh(card: &mut ReadCard) {
    let Some(look) = card.look.as_ref() else {
        return;
    };
    let address = || {
        row(
            EN.address_check_address,
            faraday_ui::grouped(&look.address),
            true,
        )
    };
    let (answer, ways) =
        if let (Some((_, change, index)), Some(found)) = (look.found, look.found_in.as_ref()) {
            let at = Text::num(u64::from(index));
            let place = Text::fill(
                if change {
                    EN.address_check_change
                } else {
                    EN.address_check_receive
                },
                &[&at],
            );
            match found {
                FoundIn::Wallet { name, shape, keys } => {
                    let mut rows = vec![
                        address(),
                        row(EN.address_check_place, place, false),
                        row(EN.address_check_wallet, shape.clone(), false),
                    ];
                    let lines: Vec<Text> = keys.iter().map(keep_path).collect();
                    rows.push(Row {
                        label: if lines.len() == 1 {
                            EN.address_check_path
                        } else {
                            EN.address_check_keys
                        },
                        lines,
                        mono: true,
                    });
                    (
                        Answer {
                            found: true,
                            head: Text::fill(EN.address_check_found, &[name]),
                            rows,
                            looking: None,
                        },
                        Vec::new(),
                    )
                }
                FoundIn::Account {
                    label,
                    script,
                    number,
                    line,
                    ..
                } => (
                    Answer {
                        found: true,
                        head: Text::fill(EN.address_check_found_key, &[label]),
                        rows: vec![
                            address(),
                            row(EN.address_check_place, place, false),
                            row(
                                EN.address_check_account,
                                Text::fill(
                                    EN.address_check_account_value,
                                    &[&script_name(*script), &Text::num(u64::from(*number))],
                                ),
                                false,
                            ),
                            row(EN.address_check_path, keep_path(line), true),
                        ],
                        looking: None,
                    },
                    vec![look.made.map_or(Way::MakeWallet, Way::OpenWallet)],
                ),
            }
        } else {
            let progress = look.progress();
            let value = match progress {
                Some((done, depth)) => Text::fill(
                    EN.read_look_progress,
                    &[&Text::num(u64::from(done)), &Text::num(u64::from(depth))],
                ),
                None => searched(look),
            };
            let ways = if progress.is_some() {
                vec![Way::Look(LookWay::Stop)]
            } else if look.ended() {
                Vec::new()
            } else {
                look.offered()
                    .map(|(i, _)| Way::Look(LookWay::To(i)))
                    .collect()
            };
            (
                Answer {
                    found: false,
                    head: Text::of(EN.address_check_missed),
                    rows: vec![address(), row(EN.address_check_searched, value, false)],
                    looking: progress,
                },
                ways,
            )
        };
    card.answer = Some(answer);
    card.facts = Vec::new();
    card.ways = ways;
}
