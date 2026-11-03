//! The XML document of a KDBX file (`faraday_vault::kdbx::xml`,
//! `docs/VAULT.md` §7, §7.1): what pykeepass wrote read to its entry, a
//! document written again as it was, what the model does not read kept, a
//! file read back by pykeepass, history and the recycle bin dropped and
//! counted, and each refusal named.

use std::path::{Path, PathBuf};
use std::process::Command;

use faraday_crypto::work::{Compute, Work};
use faraday_vault::Cost;
use faraday_vault::kdbx::xml::{Document, Entry, Group, Refused};
use faraday_vault::kdbx::{Contents, RANDOM_LEN, inner_key_of, open, seal};

const PASSPHRASE: &str = "kdbx vector passphrase";
const CHEAP: Cost = Cost {
    memory_kib: 8 * 1024,
    passes: 1,
    lanes: 1,
};

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-vault's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_: Work| {})
}

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn opened(name: &str) -> faraday_vault::kdbx::Opened {
    let bytes = std::fs::read(repo(&format!("tools/vectors/kdbx/pykeepass-{name}.kdbx"))).unwrap();
    open(&compute(), &bytes, PASSPHRASE).expect("opens")
}

fn trimmed(document: &[u8]) -> &[u8] {
    let end = document.iter().rposition(|&b| b != b'\n').unwrap() + 1;
    &document[..end]
}

fn has(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

const OPENABLE: [&str; 7] = [
    "aes-argon2d-gzip",
    "aes-argon2id-gzip",
    "aes-argon2id-gzip-v41",
    "chacha20-argon2d-gzip",
    "chacha20-argon2id-gzip",
    "chacha20-argon2id-plain",
    "chacha20-argon2id-plain-large",
];

fn example(doc: &Document) -> &Entry {
    doc.root()
        .entries()
        .find(|e| e.field("Title").map(|f| f.value()) == Some("Example"))
        .expect("the entry Example")
}

fn check_example(doc: &Document) {
    let e = example(doc);
    assert_eq!(e.field("UserName").unwrap().value(), "alice");
    let password = e.field("Password").unwrap();
    assert_eq!(password.value(), "hunter2");
    assert!(password.protected());
    assert_eq!(e.field("URL").unwrap().value(), "https://example.org/");
}

#[test]
fn every_file_pykeepass_wrote_opens_to_its_entry() {
    for name in OPENABLE {
        let o = opened(name);
        let doc = Document::read(&o.document, &o.inner_key).unwrap_or_else(|_| panic!("{name}"));
        check_example(&doc);
        assert_eq!(doc.history_entries(), 0, "{name}");
        assert_eq!(doc.recycle_bin_entries(), 0, "{name}");
    }
}

#[test]
fn a_document_not_changed_is_written_as_it_was() {
    for name in OPENABLE {
        let o = opened(name);
        let doc = Document::read(&o.document, &o.inner_key).unwrap();
        assert_eq!(&doc.write(&o.inner_key)[..], trimmed(&o.document), "{name}");
    }
    // The attachment's reference is among what is kept.
    let o = opened("chacha20-argon2id-plain-large");
    assert_eq!(o.binaries.len(), 1);
    assert!(has(&o.document, b"attachment.bin"));
}

// --- Read back by pykeepass --------------------------------------------

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Seals `doc` as a file and reads it with pykeepass: its lines of output,
/// or None where pykeepass is not installed (said, not silent).
fn pykeepass_reads(
    name: &str,
    doc: &Document,
    binaries: &[faraday_vault::kdbx::Binary],
) -> Option<String> {
    let random: [u8; RANDOM_LEN] = std::array::from_fn(|i| (i * 7 + 1) as u8);
    let document = doc.write(inner_key_of(&random));
    let total: usize = binaries.iter().map(|b| b.data.len()).sum();
    let file = seal(
        &compute(),
        PASSPHRASE,
        CHEAP,
        &random,
        &Contents {
            document: &document,
            binaries,
            public_custom_data: None,
        },
        total + document.len() + 4096,
    )
    .expect("sealed");
    let dir = std::env::temp_dir().join(format!("faraday-kdbx-xml-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let _guard = Scratch(dir.clone());
    let path = dir.join("vault.kdbx");
    std::fs::write(&path, file).unwrap();
    let out = Command::new("python3")
        .args(["-I", "-B"])
        .arg(repo("tools/vectors/kdbx/reread.py"))
        .arg(&path)
        .arg(PASSPHRASE)
        .output()
        .ok()?;
    let err = String::from_utf8_lossy(&out.stderr);
    if !out.status.success() && (err.contains("ModuleNotFoundError") || err.contains("ImportError"))
    {
        eprintln!("pykeepass is not installed: skipped");
        return None;
    }
    assert!(out.status.success(), "pykeepass: {err}");
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[test]
fn a_document_written_again_is_read_by_pykeepass() {
    for name in ["chacha20-argon2id-plain", "aes-argon2d-gzip"] {
        let o = opened(name);
        let doc = Document::read(&o.document, &o.inner_key).unwrap();
        if let Some(text) = pykeepass_reads(name, &doc, &o.binaries) {
            assert!(
                text.contains("/Example\talice\thunter2\thttps://example.org/\t0\t\n"),
                "{text}"
            );
        }
    }
    let o = opened("chacha20-argon2id-plain-large");
    let doc = Document::read(&o.document, &o.inner_key).unwrap();
    if let Some(text) = pykeepass_reads("large", &doc, &o.binaries) {
        assert!(
            text.contains("\thunter2\thttps://example.org/\t0\tattachment.bin\n"),
            "{text}"
        );
    }
}

// --- History and the recycle bin -----------------------------------------

#[test]
fn history_and_the_recycle_bin_are_dropped_and_counted() {
    let o = opened("chacha20-argon2id-plain-history");
    let doc = Document::read(&o.document, &o.inner_key).unwrap();
    assert_eq!(doc.history_entries(), 1);
    assert_eq!(doc.recycle_bin_entries(), 1);
    check_example(&doc);
    let key = [9u8; 64];
    let written = doc.write(&key);
    assert!(!has(&written, b"<History>"));
    assert!(!has(&written, b"<Value>Deleted</Value>"));
    assert!(!has(&written, b"Recycle Bin"));
    let again = Document::read(&written, &key).unwrap();
    assert_eq!(
        (again.history_entries(), again.recycle_bin_entries()),
        (0, 0)
    );
    check_example(&again);
    if let Some(text) = pykeepass_reads("history", &doc, &[]) {
        assert!(
            text.contains("/Example\talice\thunter2\thttps://example.org/\t0\t\n"),
            "{text}"
        );
        assert!(!text.contains("Deleted"), "{text}");
        assert!(text.ends_with("bin: no\n"), "{text}");
    }
}

// --- What the model does not read ----------------------------------------

const ID: &str = "AQEBAQEBAQEBAQEBAQEBAQ==";
const OTHER: &str = "AgICAgICAgICAgICAgICAg==";

fn skeleton(meta: &str, group: &str) -> Vec<u8> {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\"?>\n<KeePassFile><Meta>{meta}</Meta><Root><Group><UUID>{ID}</UUID><Name>V</Name>{group}</Group></Root></KeePassFile>\n\n"
    )
    .into_bytes()
}

const KEY: &[u8] = b"an inner stream key";

#[test]
fn what_a_program_wrote_and_is_not_read_is_kept_in_its_place() {
    let meta = "<Generator>Other</Generator><DatabaseName>Vault</DatabaseName>\n\t<MemoryProtection><ProtectPassword>True</ProtectPassword></MemoryProtection><CustomData><Item><Key>KPXC_RANDOM_SLUG</Key><Value>abc</Value></Item><Item><Key>faraday.format</Key><Value>1</Value></Item></CustomData><Unknown a=\"1\">x<Child/></Unknown>";
    let first = format!(
        "<Entry><UUID>{ID}</UUID><IconID>3</IconID><Tags>a;b</Tags><Times><CreationTime>X</CreationTime><LastModificationTime>Y</LastModificationTime></Times><String><Key>Title</Key><Value>T &amp; &#65; &apos;q&apos;</Value></String><Binary><Key>f.bin</Key><Value Ref=\"0\"/></Binary><AutoType><Enabled>True</Enabled></AutoType><CustomData><Item><Key>plugin</Key><Value>p</Value></Item></CustomData></Entry>"
    );
    let second = format!(
        "<Entry><UUID>{OTHER}</UUID><String><Key>Title</Key><Value>Second</Value></String></Entry>"
    );
    let group = format!("<IconID>48</IconID>{first}{second}");
    let text = skeleton(meta, &group);
    let mut doc = Document::read(&text, KEY).unwrap();
    // Unchanged, the document is written as it was.
    assert_eq!(&doc.write(KEY)[..], trimmed(&text));
    assert_eq!(doc.label(), "Vault");
    assert_eq!(doc.custom().get("faraday.format"), Some("1"));
    assert_eq!(doc.custom().get("KPXC_RANDOM_SLUG"), None);
    assert!(doc.custom_mut().set("plugin", "x").is_err());

    doc.custom_mut().set("faraday.seals", "2").unwrap();
    let entry = doc.root_mut().entries_mut().next().unwrap();
    assert_eq!(entry.field("Title").unwrap().value(), "T & A 'q'");
    entry.set_field("Title", "New <title>", false).unwrap();
    entry.set_field("Password", "pw", true).unwrap();
    entry.custom_mut().set("faraday.kind", "note").unwrap();
    entry.set_modified(1_700_000_000).unwrap();
    let written = doc.write(KEY);
    for kept in [
        "<Generator>Other</Generator>",
        "<MemoryProtection><ProtectPassword>True</ProtectPassword></MemoryProtection>",
        "<Item><Key>KPXC_RANDOM_SLUG</Key><Value>abc</Value></Item>",
        "<Unknown a=\"1\">x<Child/></Unknown>",
        "<IconID>48</IconID>",
        "<IconID>3</IconID><Tags>a;b</Tags>",
        "<CreationTime>X</CreationTime>",
        "<Binary><Key>f.bin</Key><Value Ref=\"0\"/></Binary>",
        "<AutoType><Enabled>True</Enabled></AutoType>",
        "<Item><Key>plugin</Key><Value>p</Value></Item>",
        // The entry not changed is written as it was.
        &second,
    ] {
        assert!(has(&written, kept.as_bytes()), "{kept}");
    }
    assert!(has(&written, b"<Value>New &lt;title&gt;</Value>"));
    assert!(has(&written, b"<Key>faraday.seals</Key><Value>2</Value>"));
    assert!(!has(&written, b"<LastModificationTime>Y"));
    assert!(!has(&written, b"pw</Value>"));
    let again = Document::read(&written, KEY).unwrap();
    let entry = again.root().entries().next().unwrap();
    assert_eq!(entry.field("Password").unwrap().value(), "pw");
    assert_eq!(entry.custom().get("faraday.kind"), Some("note"));
    assert_eq!(entry.modified(), Some(1_700_000_000));
    assert_eq!(again.custom().get("faraday.seals"), Some("2"));
}

#[test]
fn a_new_vault_is_written_and_reads_back() {
    let id = [0xA5; 16];
    let mut doc = Document::new(id, "My <vault>");
    assert_eq!(&doc.vault_id(), b"a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5");
    assert_eq!(doc.label(), "My <vault>");
    assert_eq!(doc.root().name(), "My <vault>");
    let mut entry = Entry::new([3; 16]);
    entry.set_field("Title", "Example", false).unwrap();
    entry.set_field("Password", "s3cret", true).unwrap();
    doc.root_mut().push_entry(entry).unwrap();
    doc.custom_mut().set("faraday.format", "1").unwrap();
    let written = doc.write(KEY);
    assert!(!has(&written, b"s3cret"));
    assert!(!has(&written, b"<History>"));
    assert!(has(&written, b"<Generator>Faraday</Generator>"));
    let again = Document::read(&written, KEY).unwrap();
    assert_eq!(again.vault_id(), doc.vault_id());
    assert_eq!(
        again
            .root()
            .entries()
            .next()
            .unwrap()
            .field("Password")
            .unwrap()
            .value(),
        "s3cret"
    );
}

// --- Refusals --------------------------------------------------------------

fn refusal(text: &[u8]) -> Refused {
    Document::read(text, KEY).err().expect("refused")
}

fn in_name(name: &str) -> Vec<u8> {
    format!("<?xml version=\"1.0\"?><KeePassFile><Meta></Meta><Root><Group><UUID>{ID}</UUID><Name>{name}</Name></Group></Root></KeePassFile>").into_bytes()
}

#[test]
fn a_comment_before_the_root_and_line_ends_in_text_are_read_as_xml_has_them() {
    let text = format!(
        "<?xml version=\"1.0\"?>\n<!-- made by an app -->\n<KeePassFile><Meta><DatabaseName>a\r\nb\rc\nd&#13;e</DatabaseName></Meta><Root><Group><UUID>{ID}</UUID><Name>V</Name></Group></Root></KeePassFile>\n"
    );
    let doc = Document::read(text.as_bytes(), KEY).expect("reads");
    // CR LF and a lone CR are one LF each; a reference to CR stays.
    assert_eq!(doc.label(), "a\nb\nc\nd\re");
    assert!(has(&doc.write(KEY), b"<!-- made by an app -->"));
}

#[test]
fn comments_and_processing_instructions_are_kept_where_they_are_and_skipped_in_text() {
    let text = format!(
        "<?xml version=\"1.0\"?>\n<?before x?><KeePassFile><?in y?><Meta><DatabaseName>V<?pi z?>ault</DatabaseName></Meta><Root><Group><UUID>{ID}</UUID><Name>V</Name></Group></Root></KeePassFile>\n<!-- c --><?after w?>\n"
    );
    let doc = Document::read(text.as_bytes(), KEY).expect("reads");
    assert_eq!(doc.label(), "Vault");
    let out = doc.write(KEY);
    for kept in [
        &b"<?before x?>"[..],
        b"<?in y?>",
        b"V<?pi z?>ault",
        b"<!-- c --><?after w?>",
    ] {
        assert!(has(&out, kept), "{}", String::from_utf8_lossy(kept));
    }
    assert_eq!(Document::read(&out, KEY).unwrap().write(KEY), out);
}

#[test]
fn what_is_refused_is_named() {
    assert_eq!(
        refusal(b"<a><b></a>"),
        Refused::Structure("the root element is not KeePassFile")
    );
    assert_eq!(
        refusal(b"<KeePassFile><Meta></KeePassFile>"),
        Refused::NotWellFormed
    );
    assert_eq!(
        refusal(b"<!DOCTYPE x [<!ENTITY e \"v\">]><KeePassFile/>"),
        Refused::DocType
    );
    assert_eq!(refusal(&in_name("&foo;")), Refused::Entity(b"foo".to_vec()));
    assert_eq!(refusal(&in_name("&#0;")), Refused::Entity(b"#0".to_vec()));
    // A sign is not a digit.
    assert_eq!(
        refusal(&in_name("&#+65;")),
        Refused::Entity(b"#+65".to_vec())
    );
    assert_eq!(
        refusal(&in_name("&#x+41;")),
        Refused::Entity(b"#x+41".to_vec())
    );
    // A character XML 1.0 forbids, raw.
    assert_eq!(refusal(&in_name("a\u{1b}b")), Refused::NotWellFormed);
    assert_eq!(refusal(&in_name("a\u{FFFE}b")), Refused::NotWellFormed);
    assert!(Document::read(&in_name("&#65;&lt;&gt;&amp;&quot;&apos;&#x41;"), KEY).is_ok());
    assert_eq!(
        refusal(b"<KeePassFile><?xml version=\"1.0\"?></KeePassFile>"),
        Refused::ProcessingInstruction(b"xml".to_vec())
    );
    assert_eq!(
        refusal(b"<?xml version=\"1.0\" encoding=\"iso-8859-1\"?><KeePassFile/>"),
        Refused::Encoding(b"iso-8859-1".to_vec())
    );
    let mut after = in_name("V");
    after.extend_from_slice(b"\n\n\n");
    assert!(Document::read(&after, KEY).is_ok());
    // XML's whitespace after the root: space, tab, CR and LF.
    let mut blanks = in_name("V");
    blanks.extend_from_slice(b" \t\r\n");
    assert!(Document::read(&blanks, KEY).is_ok());
    after.extend_from_slice(b"x");
    assert_eq!(refusal(&after), Refused::AfterRoot);
    let mut after = in_name("V");
    after.extend_from_slice(b"<a/>");
    assert_eq!(refusal(&after), Refused::AfterRoot);
    let deep = format!("{}{}", "<a>".repeat(70), "</a>".repeat(70));
    assert_eq!(refusal(&skeleton(&deep, "")), Refused::TooDeep);
    assert_eq!(refusal(&[0xFF, 0xFE]), Refused::NotUtf8);
}

#[test]
fn a_value_a_uuid_or_a_key_that_is_wrong_is_refused() {
    let entry = |inner: &str| skeleton("", &format!("<Entry><UUID>{OTHER}</UUID>{inner}</Entry>"));
    // Not padded, trailing bits set, not Base64 at all.
    for bad in ["AA", "AB==", "A!A="] {
        let text = entry(&format!(
            "<String><Key>Password</Key><Value Protected=\"True\">{bad}</Value></String>"
        ));
        assert_eq!(
            refusal(&text),
            Refused::Base64("a protected value"),
            "{bad}"
        );
    }
}

// --- Marked, not refused ---------------------------------------------------

#[test]
fn an_entry_with_two_fields_of_one_key_opens_marked_and_is_not_edited() {
    let text = skeleton(
        "",
        &format!(
            "<Entry><UUID>{OTHER}</UUID><String><Key>Title</Key><Value>A</Value></String><String><Key>Title</Key><Value>B</Value></String><String><Key>URL</Key><Value>u</Value></String></Entry>"
        ),
    );
    let mut doc = Document::read(&text, KEY).unwrap();
    assert_eq!(&doc.write(KEY)[..], trimmed(&text));
    let entry = doc.root_mut().entries_mut().next().unwrap();
    assert_eq!(entry.duplicate_keys().collect::<Vec<_>>(), ["Title"]);
    assert!(entry.set_field("URL", "x", false).is_err());
    assert!(entry.set_field("Notes", "x", false).is_err());
    assert!(entry.remove_field("URL").is_err());
    assert_eq!(&doc.write(KEY)[..], trimmed(&text));
}

#[test]
fn a_group_or_an_entry_with_another_ones_uuid_opens_marked() {
    let entry = |t: &str| {
        format!(
            "<Entry><UUID>{OTHER}</UUID><String><Key>Title</Key><Value>{t}</Value></String></Entry>"
        )
    };
    let group = |inner: &str| format!("<Group><UUID>{OTHER}</UUID><Name>G</Name>{inner}</Group>");
    let text = skeleton(
        "",
        &format!(
            "{}{}{}{}",
            entry("1"),
            entry("2"),
            group(&entry("3")),
            group("")
        ),
    );
    let doc = Document::read(&text, KEY).unwrap();
    assert_eq!(&doc.write(KEY)[..], trimmed(&text));
    assert!(!doc.root().duplicate_uuid());
    assert!(doc.root().entries().all(Entry::duplicate_uuid));
    assert!(doc.root().groups().all(|g| g.duplicate_uuid()));
    assert!(
        doc.root()
            .groups()
            .next()
            .unwrap()
            .entries()
            .all(Entry::duplicate_uuid)
    );
    // Both namespaces are separate: this entry's UUID is the group's only
    // among groups.
    let text = skeleton("", &format!("{}{}", entry("1"), group("")));
    let doc = Document::read(&text, KEY).unwrap();
    assert!(!doc.root().entries().next().unwrap().duplicate_uuid());
}

// --- Characters XML forbids -------------------------------------------------

#[test]
fn characters_xml_forbids_are_left_out_of_what_is_written() {
    let mut doc = Document::new([0xA5; 16], "La\u{1}bel");
    let mut entry = Entry::new([3; 16]);
    for (k, v, p) in EXAMPLE {
        entry.set_field(k, v, p).unwrap();
    }
    entry
        .set_field("URL", "https://exa\u{1b}mple.org/\u{0}\u{FFFF}", false)
        .unwrap();
    entry.set_field("No\u{c}te", "n", false).unwrap();
    doc.root_mut().push_entry(entry).unwrap();
    doc.custom_mut().set("faraday.k\u{2}ey", "v\u{1f}").unwrap();
    let written = doc.write(KEY);
    assert!(
        std::str::from_utf8(&written)
            .unwrap()
            .chars()
            .all(|c| !c.is_control() || c == '\n')
    );
    let again = Document::read(&written, KEY).unwrap();
    assert_eq!(again.label(), "Label");
    assert_eq!(again.custom().get("faraday.key"), Some("v"));
    let e = again.root().entries().next().unwrap();
    assert_eq!(e.field("URL").unwrap().value(), "https://example.org/");
    assert!(e.field("Note").is_some());
    if let Some(text) = pykeepass_reads("forbidden", &doc, &[]) {
        assert!(
            text.contains("/Example\talice\thunter2\thttps://example.org/\t0\t\n"),
            "{text}"
        );
    }
}

// --- The inner stream's order ----------------------------------------------

const EXAMPLE: [(&str, &str, bool); 4] = [
    ("Title", "Example", false),
    ("UserName", "alice", false),
    ("Password", "hunter2", true),
    ("URL", "https://example.org/", false),
];

const OTHER_KEY: &[u8] = b"another inner stream key";

fn entry_of(uuid: u8, fields: &[(&str, &str, bool)]) -> Entry {
    let mut e = Entry::new([uuid; 16]);
    for &(k, v, p) in fields {
        e.set_field(k, v, p).unwrap();
    }
    e
}

/// The field `key`'s `<String>` element (the first), with `before` and
/// `after` put around it.
fn wrap(text: &mut String, key: &str, before: &str, after: &str) {
    let start = text.find(&format!("<String><Key>{key}</Key>")).unwrap();
    let end = start + text[start..].find("</String>").unwrap() + "</String>".len();
    text.insert_str(end, after);
    text.insert_str(start, before);
}

/// `text` is read under KEY, written under another key and read again, and
/// pykeepass reads the written file: `check` holds each time.
fn survives_another_key(name: &str, text: &str, check: impl Fn(&Document)) {
    let doc = Document::read(text.as_bytes(), KEY).unwrap();
    check(&doc);
    let again = Document::read(&doc.write(OTHER_KEY), OTHER_KEY).unwrap();
    check(&again);
    if let Some(out) = pykeepass_reads(name, &doc, &[]) {
        assert!(
            out.contains("/Example\talice\thunter2\thttps://example.org/\t0\t\n"),
            "{out}"
        );
    }
}

fn written(doc: &Document) -> String {
    String::from_utf8(doc.write(KEY).to_vec()).unwrap()
}

fn example_password(doc: &Document) {
    let e = example_in(doc.root()).expect("the entry Example");
    assert_eq!(e.field("Password").unwrap().value(), "hunter2");
}

fn example_in(g: &Group) -> Option<&Entry> {
    g.entries()
        .find(|e| e.field("Title").map(|f| f.value()) == Some("Example"))
        .or_else(|| g.groups().find_map(example_in))
}

#[test]
fn a_carried_protected_value_before_a_field_keeps_the_stream_in_step() {
    let mut doc = Document::new([1; 16], "V");
    let mut fields = vec![("Secret", "carried", true)];
    fields.extend(EXAMPLE);
    doc.root_mut().push_entry(entry_of(2, &fields)).unwrap();
    let mut text = written(&doc);
    wrap(&mut text, "Secret", "<Foo>", "</Foo>");
    survives_another_key("carried", &text, example_password);
    let out = Document::read(text.as_bytes(), KEY)
        .unwrap()
        .write(OTHER_KEY);
    assert!(has(
        &out,
        b"<Foo><String><Key>Secret</Key><Value Protected=\"True\">"
    ));
    assert!(!has(&out, b"carried"));
}

#[test]
fn an_entrys_history_before_a_later_entry_keeps_the_stream_in_step() {
    let mut doc = Document::new([1; 16], "V");
    doc.root_mut()
        .push_entry(entry_of(
            2,
            &[("Title", "Old", false), ("Password", "oldpw", true)],
        ))
        .unwrap();
    doc.root_mut().push_entry(entry_of(3, &EXAMPLE)).unwrap();
    let mut text = written(&doc);
    wrap(
        &mut text,
        "Password",
        &format!("<History><Entry><UUID>{OTHER}</UUID>"),
        "</Entry></History>",
    );
    survives_another_key("history-order", &text, example_password);
    assert_eq!(
        Document::read(text.as_bytes(), KEY)
            .unwrap()
            .history_entries(),
        1
    );
}

#[test]
fn a_protected_field_after_the_recycle_bin_keeps_the_stream_in_step() {
    let mut doc = Document::new([1; 16], "V");
    let mut bin = Group::new([4; 16], "Recycle Bin");
    bin.push_entry(entry_of(
        2,
        &[("Title", "Gone", false), ("Password", "deleted", true)],
    ))
    .unwrap();
    doc.root_mut().push_group(bin).unwrap();
    let mut later = Group::new([5; 16], "Later");
    later.push_entry(entry_of(3, &EXAMPLE)).unwrap();
    doc.root_mut().push_group(later).unwrap();
    let mut text = written(&doc);
    let meta = "<RecycleBinUUID>BAQEBAQEBAQEBAQEBAQEBA==</RecycleBinUUID><RecycleBinEnabled>True</RecycleBinEnabled>";
    const NONE: &str = "<RecycleBinUUID>AAAAAAAAAAAAAAAAAAAAAA==</RecycleBinUUID>";
    text = text.replace(NONE, meta);
    survives_another_key("bin-order", &text, |d| {
        assert_eq!(d.root().groups().count(), 1);
        example_password(d);
    });
    let doc = Document::read(text.as_bytes(), KEY).unwrap();
    assert_eq!(doc.recycle_bin_entries(), 1);
    // The bin is a group like any other when the bin is not enabled.
    let text = text.replace("<RecycleBinEnabled>True", "<RecycleBinEnabled>False");
    let doc = Document::read(text.as_bytes(), KEY).unwrap();
    assert_eq!(doc.recycle_bin_entries(), 0);
    assert_eq!(doc.root().groups().count(), 2);
    example_password(&doc);
    // The writer says there is no bin.
    let expected = text.replace(
        "<RecycleBinUUID>BAQEBAQEBAQEBAQEBAQEBA==</RecycleBinUUID>",
        NONE,
    );
    assert_eq!(String::from_utf8_lossy(&doc.write(KEY)), expected);
}

/// A protected Base64 text for a plaintext of one byte, at the start of the
/// stream under KEY.
fn one_byte(plain: &str) -> String {
    let mut doc = Document::new([1; 16], "V");
    doc.root_mut()
        .push_entry(entry_of(2, &[("Password", plain, true)]))
        .unwrap();
    let text = written(&doc);
    let from = text.find("Protected=\"True\">").unwrap() + "Protected=\"True\">".len();
    text[from..from + text[from..].find('<').unwrap()].to_string()
}

#[test]
fn every_text_element_that_is_protected_is_opened_as_keepassxc_does() {
    let ct = one_byte("Vault name");
    let name = |attr: &str, inner: &str| {
        format!(
            "<?xml version=\"1.0\"?><KeePassFile><Meta></Meta><Root><Group><UUID>{ID}</UUID><Name {attr}>{inner}</Name></Group></Root></KeePassFile>"
        )
    };
    for attr in [
        "Protected=\"True\"",
        "Protected=\"true\"",
        "Protected=\"1\"",
        "Protected=\"&#84;rue\"",
    ] {
        let doc = Document::read(name(attr, &ct).as_bytes(), KEY).unwrap();
        assert_eq!(doc.root().name(), "Vault name", "{attr}");
        // Written unprotected, and the same name reads back.
        let out = doc.write(OTHER_KEY);
        assert!(has(&out, b"<Name>Vault name</Name>"), "{attr}");
    }
    for attr in [
        "Protected=\"False\"",
        "Protected=\"0\"",
        "Protected=\"yes\"",
        "",
    ] {
        let doc = Document::read(name(attr, &ct).as_bytes(), KEY).unwrap();
        assert_eq!(doc.root().name(), ct, "{attr}");
    }
    // A protected attribute on an element with elements in it means nothing.
    let text = skeleton("<Foo Protected=\"True\"><Bar>x</Bar></Foo>", "");
    let doc = Document::read(&text, KEY).unwrap();
    assert_eq!(&doc.write(KEY)[..], trimmed(&text));
}

#[test]
fn a_protected_value_that_is_not_text_is_damage() {
    // 'a' with its high bit flipped by the stream's cipher text: the first
    // Base64 digit's top bit is that bit.
    const ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let ct = one_byte("a");
    let first = ALPHABET.find(&ct[..1]).unwrap() ^ 32;
    let flipped = format!("{}{}", &ALPHABET[first..=first], &ct[1..]);
    let text = skeleton(
        "",
        &format!(
            "<Entry><UUID>{OTHER}</UUID><String><Key>Password</Key><Value Protected=\"True\">{flipped}</Value></String></Entry>"
        ),
    );
    assert_eq!(refusal(&text), Refused::NotText);
}

// --- Ambiguous, not refused -------------------------------------------------

const FIELD: &str = "<String><Key>Title</Key><Value>T</Value></String>";

#[test]
fn what_the_model_cannot_place_is_marked_kept_and_not_edited() {
    let entry = |inner: &str| format!("<Entry><UUID>{OTHER}</UUID>{inner}</Entry>");
    let group = |inner: &str| format!("<Group><UUID>{OTHER}</UUID><Name>G</Name>{inner}</Group>");
    let item = |k: &str| format!("<Item><Key>{k}</Key><Value>1</Value></Item>");
    let cases: Vec<(String, String, &str)> = vec![
        (String::new(), entry(&format!("{FIELD}<UUID>{ID}</UUID>")), "an entry with two UUIDs"),
        (String::new(), "<Entry><UUID>AAAA</UUID></Entry>".into(), "a UUID that is not 16 bytes"),
        (String::new(), "<Entry></Entry>".into(), "an entry without a UUID"),
        (
            String::new(),
            entry("<Times><LastModificationTime>A</LastModificationTime></Times><Times/>"),
            "an entry with two Times",
        ),
        (
            String::new(),
            entry("<Times><LastModificationTime>A</LastModificationTime><LastModificationTime>B</LastModificationTime></Times>"),
            "LastModificationTime twice",
        ),
        (
            String::new(),
            entry("<CustomData/><CustomData/>"),
            "an entry with two lists of custom data",
        ),
        (
            String::new(),
            entry("<String><Key>A</Key><Key>B</Key><Value>v</Value></String>"),
            "a field with two keys",
        ),
        (
            String::new(),
            entry("<String><Key>A</Key><Value>v</Value><Value>w</Value></String>"),
            "a field with two values",
        ),
        (String::new(), entry("<String><Value>x</Value></String>"), "a field without a key"),
        (String::new(), entry("<String><Key>A</Key>text</String>"), "text inside a field"),
        (
            String::new(),
            entry(&format!("<CustomData>{}{}</CustomData>", item("faraday.a"), item("faraday.a"))),
            "two custom data items with one key",
        ),
        (
            String::new(),
            group(&format!("<CustomData>{}{}</CustomData>", item("faraday.a"), item("faraday.a"))),
            "two custom data items with one key",
        ),
        (String::new(), group("<Name>H</Name>"), "a group with two names"),
        (String::new(), group(&format!("<UUID>{ID}</UUID>")), "a group with two UUIDs"),
        (String::new(), "<Group><Name>G</Name></Group>".into(), "a group without a UUID"),
        (
            format!("<CustomData>{}{}</CustomData>", item("faraday.a"), item("faraday.a")),
            String::new(),
            "two custom data items with one key",
        ),
        ("<DatabaseName>A</DatabaseName><DatabaseName>B</DatabaseName>".into(), String::new(), "DatabaseName twice"),
        ("<CustomData/><CustomData/>".into(), String::new(), "CustomData twice"),
        (
            "<RecycleBinUUID>AAAAAAAAAAAAAAAAAAAAAA==</RecycleBinUUID><RecycleBinUUID>AAAAAAAAAAAAAAAAAAAAAA==</RecycleBinUUID>".into(),
            String::new(),
            "RecycleBinUUID twice",
        ),
    ];
    for (meta, inner, why) in cases {
        let text = skeleton(&meta, &inner);
        let mut doc = Document::read(&text, KEY).unwrap_or_else(|e| panic!("{why}: {e:?}"));
        assert_eq!(&doc.write(KEY)[..], trimmed(&text), "{why}");
        let found = doc
            .ambiguous()
            .into_iter()
            .chain(doc.root().entries().filter_map(Entry::ambiguous))
            .chain(doc.root().groups().filter_map(|g| g.ambiguous()))
            .collect::<Vec<_>>();
        assert!(found.contains(&why), "{why}: {found:?}");
        // Every edit of it refuses.
        if let Some(e) = doc.root_mut().entries_mut().next() {
            assert!(e.set_field("Title", "x", false).is_err(), "{why}");
            assert!(e.remove_field("Title").is_err(), "{why}");
            assert!(e.set_modified(1).is_err(), "{why}");
            assert!(e.custom_mut().set("faraday.k", "v").is_err(), "{why}");
        }
        if let Some(g) = doc.root_mut().groups_mut().next() {
            assert!(g.set_name("x").is_err(), "{why}");
            assert!(g.custom_mut().set("faraday.k", "v").is_err(), "{why}");
            assert!(g.push_entry(Entry::new([9; 16])).is_err(), "{why}");
        }
        if doc.ambiguous().is_some() {
            assert!(doc.custom_mut().set("faraday.k", "v").is_err(), "{why}");
        }
        assert_eq!(&doc.write(KEY)[..], trimmed(&text), "{why}");
    }
}

#[test]
fn the_recycle_bin_is_dropped_unless_it_is_disabled() {
    let bin = "BAQEBAQEBAQEBAQEBAQEBA==";
    let group = format!(
        "<Group><UUID>{bin}</UUID><Name>Recycle Bin</Name><Entry><UUID>{OTHER}</UUID><String><Key>Title</Key><Value>Gone</Value></String></Entry></Group>"
    );
    for (enabled, kept) in [
        ("", false),
        ("<RecycleBinEnabled>True</RecycleBinEnabled>", false),
        ("<RecycleBinEnabled>true</RecycleBinEnabled>", false),
        ("<RecycleBinEnabled>False</RecycleBinEnabled>", true),
    ] {
        let meta = format!("<RecycleBinUUID>{bin}</RecycleBinUUID>{enabled}");
        let doc = Document::read(&skeleton(&meta, &group), KEY).unwrap();
        assert_eq!(doc.root().groups().count(), usize::from(kept), "{enabled}");
        assert_eq!(doc.recycle_bin_entries(), usize::from(!kept), "{enabled}");
    }
}

#[test]
fn a_comment_in_a_text_element_is_skipped_and_kept() {
    let text = skeleton("<DatabaseName>Va<!-- c -->ult</DatabaseName>", "");
    let doc = Document::read(&text, KEY).unwrap();
    assert_eq!(doc.label(), "Vault");
    assert_eq!(&doc.write(KEY)[..], trimmed(&text));
    // Carried and protected: the comment stays and the stream stays in step.
    let ct = one_byte("carried");
    let text = skeleton(&format!("<Foo Protected=\"True\">{ct}<!-- c --></Foo>"), "");
    let doc = Document::read(&text, KEY).unwrap();
    let out = doc.write(OTHER_KEY);
    assert!(has(&out, b"<!-- c -->"));
    assert!(Document::read(&out, OTHER_KEY).is_ok());
}
