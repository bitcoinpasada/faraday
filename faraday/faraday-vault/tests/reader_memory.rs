//! The memory the document reader's charge stands for, measured: a
//! counting allocator holds what a read keeps, against what the reader
//! charged and what the document's length allows.

use base64ct::{Base64, Encoding};
use faraday_crypto::work::{Compute, Work};
use faraday_vault::kdbx::items;
use faraday_vault::kdbx::xml::{Document, MODEL_PER_BYTE, Refused};

#[global_allocator]
static ALLOC: faraday_alloc_count::Counting = faraday_alloc_count::Counting;

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-vault's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_: Work| {})
}

const KEY: &[u8] = b"an inner stream key";

fn id(n: usize) -> String {
    let mut u = [7u8; 16];
    u[..8].copy_from_slice(&(n as u64 + 1).to_le_bytes());
    Base64::encode_string(&u)
}

fn skeleton(children: &str) -> Vec<u8> {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\"?>\n<KeePassFile><Meta></Meta><Root><Group><UUID>{}</UUID><Name>V</Name>{children}</Group></Root></KeePassFile>\n\n",
        id(1 << 40)
    )
    .into_bytes()
}

const TIMES: &str = "\n\t\t<Times>\n\t\t\t<CreationTime>2024-01-01T00:00:00Z</CreationTime>\n\t\t\t<LastModificationTime>2024-01-01T00:00:00Z</LastModificationTime>\n\t\t\t<LastAccessTime>2024-01-01T00:00:00Z</LastAccessTime>\n\t\t\t<ExpiryTime>2024-01-01T00:00:00Z</ExpiryTime>\n\t\t\t<Expires>False</Expires>\n\t\t\t<UsageCount>0</UsageCount>\n\t\t\t<LocationChanged>2024-01-01T00:00:00Z</LocationChanged>\n\t\t</Times>";

/// A group as KeePassXC writes an empty one.
fn xc_group(n: usize) -> String {
    format!(
        "\n\t<Group>\n\t\t<UUID>{}</UUID>\n\t\t<Name>g{n}</Name>\n\t\t<Notes></Notes>\n\t\t<IconID>48</IconID>{TIMES}\n\t\t<IsExpanded>True</IsExpanded>\n\t\t<DefaultAutoTypeSequence></DefaultAutoTypeSequence>\n\t\t<EnableAutoType>null</EnableAutoType>\n\t\t<EnableSearching>null</EnableSearching>\n\t\t<LastTopVisibleEntry>AAAAAAAAAAAAAAAAAAAAAA==</LastTopVisibleEntry>\n\t</Group>",
        id(n)
    )
}

/// An entry as a KeePass app writes a real one.
fn real_entry(n: usize) -> String {
    format!(
        "\n\t<Entry>\n\t\t<UUID>{}</UUID>\n\t\t<IconID>0</IconID>\n\t\t<ForegroundColor></ForegroundColor>\n\t\t<BackgroundColor></BackgroundColor>\n\t\t<OverrideURL></OverrideURL>\n\t\t<Tags></Tags>{TIMES}\n\t\t<String>\n\t\t\t<Key>Title</Key>\n\t\t\t<Value>A bank</Value>\n\t\t</String>\n\t\t<String>\n\t\t\t<Key>UserName</Key>\n\t\t\t<Value>someone</Value>\n\t\t</String>\n\t\t<String>\n\t\t\t<Key>Password</Key>\n\t\t\t<Value>correct horse</Value>\n\t\t</String>\n\t\t<String>\n\t\t\t<Key>URL</Key>\n\t\t\t<Value>https://example.org/login</Value>\n\t\t</String>\n\t\t<String>\n\t\t\t<Key>Notes</Key>\n\t\t\t<Value></Value>\n\t\t</String>\n\t\t<AutoType>\n\t\t\t<Enabled>True</Enabled>\n\t\t\t<DataTransferObfuscation>0</DataTransferObfuscation>\n\t\t</AutoType>\n\t</Entry>",
        id(n)
    )
}

/// An entry with a title and a password, nothing else.
fn small_entry(n: usize) -> String {
    format!(
        "<Entry><UUID>{}</UUID><String><Key>Title</Key><Value>t</Value></String><String><Key>Password</Key><Value>p</Value></String></Entry>",
        id(n)
    )
}

/// A chain of `depth` groups, and at its foot `many` groups with one
/// entry each.
fn deep(depth: usize, many: usize) -> Vec<u8> {
    let mut inner = String::new();
    for n in 0..many {
        inner.push_str(&format!(
            "<Group><UUID>{}</UUID><Name>s</Name>{}</Group>",
            id(2 * n + 1000),
            small_entry(2 * n + 1001)
        ));
    }
    for d in (0..depth).rev() {
        inner = format!(
            "<Group><UUID>{}</UUID><Name>d{d}</Name>{inner}</Group>",
            id(d + 2)
        );
    }
    skeleton(&inner)
}

/// Reads `text`; what the document holds, what the reader charged, and
/// what the items read from it hold against what [`items::Read::held`] says.
fn measure(text: &[u8]) -> (usize, usize, usize, usize) {
    let c = compute();
    let before = faraday_alloc_count::live_bytes();
    let doc = Document::read(text, KEY).expect("reads");
    let held = (faraday_alloc_count::live_bytes() - before) as usize;
    let before = faraday_alloc_count::live_bytes();
    let read = items::read(&c, &doc);
    let items_held = (faraday_alloc_count::live_bytes() - before) as usize;
    (held, doc.charged(), items_held, read.held())
}

/// What the charge stands for is what the read holds: never less, for the
/// shapes a crafted file would use and the ones real apps write, and a real
/// shape is charged under the limit for its length.
#[test]
fn the_reader_charges_at_least_what_it_holds() {
    let n = 2000;
    let shapes: Vec<(&str, Vec<u8>, bool)> = vec![
        (
            "crafted group",
            skeleton(&"<Group a=\"xxxxxxxxxx\"/>".repeat(n)),
            false,
        ),
        ("empty group", skeleton(&"<Group/>".repeat(n / 2)), false),
        ("empty entry", skeleton(&"<Entry/>".repeat(n / 2)), false),
        (
            "empty fields",
            skeleton(&format!("<Entry>{}</Entry>", "<String/>".repeat(n))),
            false,
        ),
        (
            "empty items",
            skeleton(&format!("<CustomData>{}</CustomData>", "<Item/>".repeat(n))),
            false,
        ),
        (
            "empty times",
            skeleton(&"<Entry><Times/></Entry>".repeat(n / 2)),
            false,
        ),
        (
            "protected history",
            skeleton(&format!(
                "<Entry><UUID>{}</UUID><String><Key>Title</Key><Value>t</Value></String><History>{}</History></Entry>",
                id(0),
                "<a Protected=\"1\"></a>".repeat(n)
            )),
            false,
        ),
        (
            "deep chain of groups, entries beside it",
            deep(54, n / 4),
            false,
        ),
        (
            "xc groups",
            skeleton(&(0..n).map(xc_group).collect::<String>()),
            true,
        ),
        (
            "real entries",
            skeleton(&(0..n).map(real_entry).collect::<String>()),
            true,
        ),
    ];
    let mut bad = Vec::new();
    for (name, text, real) in shapes {
        let (held, charged, items_held, items_said) = measure(&text);
        println!(
            "{name}: {} bytes, held {held} ({:.1} a byte), charged {charged} ({:.1} a byte)",
            text.len(),
            held as f64 / text.len() as f64,
            charged as f64 / text.len() as f64
        );
        println!("  items: held {items_held}, said {items_said}");
        if held > charged
            || items_held > items_said
            || (real && charged > text.len() * MODEL_PER_BYTE)
        {
            bad.push(name);
        }
    }
    assert!(bad.is_empty(), "{bad:?}");
    // A flood of empty groups is refused, not read.
    let flood = skeleton(&"<Group/>".repeat(200_000));
    assert!(matches!(
        Document::read(&flood, KEY).err(),
        Some(Refused::TooLarge)
    ));
}
