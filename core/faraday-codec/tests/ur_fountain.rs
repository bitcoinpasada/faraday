//! `faraday_codec::ur::Decoder` puts a message together from the parts a
//! scanner happened to catch, as BCR-2020-005's decoder (URKit's) does:
//! a mix is reduced by another mix whose fragments are a subset of its
//! own, not only by fragments already resolved.

use faraday_codec::ur::{Decoder, Message};

/// Fifty parts of a 41-byte PSBT-typed message in 43 one-byte fragments,
/// six of the plain fragments missing and mixes in their place: a set
/// foundation-ur-py's decoder completes, found by the cross-check's `ur`
/// area (`tools/crosscheck/area_ur.py`).
const PARTS: [&str; 50] = [
    "ur:crypto-psbt/46-43/lpcsdmcsdncsdncytssfhnutfpqdfnwyvyam",
    "ur:crypto-psbt/57-43/lpcsescsdncsdncytssfhnutfptlvtrsclcs",
    "ur:crypto-psbt/33-43/lpcsclcsdncsdncytssfhnutfptlwdpyrdcy",
    "ur:crypto-psbt/40-43/lpcsdecsdncsdncytssfhnutfpntbsahgadt",
    "ur:crypto-psbt/35-43/lpcscncsdncsdncytssfhnutfpsalpfxoyfw",
    "ur:crypto-psbt/28-43/lpcscecsdncsdncytssfhnutfpolptclpmko",
    "ur:crypto-psbt/59-43/lpcsfrcsdncsdncytssfhnutfpaobbeoyawt",
    "ur:crypto-psbt/16-43/lpbecsdncsdncytssfhnutfpqdgrpdhkih",
    "ur:crypto-psbt/19-43/lpbwcsdncsdncytssfhnutfpcfbkecseia",
    "ur:crypto-psbt/3-43/lpaxcsdncsdncytssfhnutfplfhswtiygu",
    "ur:crypto-psbt/23-43/lpchcsdncsdncytssfhnutfpiediltylnt",
    "ur:crypto-psbt/12-43/lpbncsdncsdncytssfhnutfpryonaotyct",
    "ur:crypto-psbt/26-43/lpcscycsdncsdncytssfhnutfprorfbwecyk",
    "ur:crypto-psbt/61-43/lpcsfscsdncsdncytssfhnutfpsbcfrpdiaa",
    "ur:crypto-psbt/42-43/lpcsdrcsdncsdncytssfhnutfpsrjlwepyfe",
    "ur:crypto-psbt/32-43/lpcscxcsdncsdncytssfhnutfpfgcsayeelu",
    "ur:crypto-psbt/41-43/lpcsdtcsdncsdncytssfhnutfphgwsbydmvs",
    "ur:crypto-psbt/44-43/lpcsdwcsdncsdncytssfhnutfpyaehuyvdly",
    "ur:crypto-psbt/25-43/lpcscfcsdncsdncytssfhnutfpfhrogywnln",
    "ur:crypto-psbt/38-43/lpcsdscsdncsdncytssfhnutfphndaetbbmt",
    "ur:crypto-psbt/29-43/lpcscacsdncsdncytssfhnutfplfbaltahsp",
    "ur:crypto-psbt/20-43/lpbbcsdncsdncytssfhnutfpnybkcezsko",
    "ur:crypto-psbt/63-43/lpcsfhcsdncsdncytssfhnutfphgbneeiywk",
    "ur:crypto-psbt/37-43/lpcsdacsdncsdncytssfhnutfprpfscmpalt",
    "ur:crypto-psbt/17-43/lpbycsdncsdncytssfhnutfpgwieampyfs",
    "ur:crypto-psbt/36-43/lpcsdkcsdncsdncytssfhnutfpfzwzjnpkse",
    "ur:crypto-psbt/53-43/lpcseccsdncsdncytssfhnutfptkcstsmdvl",
    "ur:crypto-psbt/39-43/lpcsdicsdncsdncytssfhnutfpluldfeiaas",
    "ur:crypto-psbt/9-43/lpascsdncsdncytssfhnutfpbkisoxlpcx",
    "ur:crypto-psbt/4-43/lpaacsdncsdncytssfhnutfpfzhnaodwfz",
    "ur:crypto-psbt/6-43/lpamcsdncsdncytssfhnutfpzceswlkiwy",
    "ur:crypto-psbt/18-43/lpbgcsdncsdncytssfhnutfpprtimwyljz",
    "ur:crypto-psbt/15-43/lpbscsdncsdncytssfhnutfpwnpmynbdgh",
    "ur:crypto-psbt/47-43/lpcsdlcsdncsdncytssfhnutfppddpdmielp",
    "ur:crypto-psbt/22-43/lpcmcsdncsdncytssfhnutfpjyoymdpymt",
    "ur:crypto-psbt/48-43/lpcsdycsdncsdncytssfhnutfpbkjekntysf",
    "ur:crypto-psbt/21-43/lpbzcsdncsdncytssfhnutfpdsgujtgarn",
    "ur:crypto-psbt/5-43/lpahcsdncsdncytssfhnutfpehuooncllp",
    "ur:crypto-psbt/43-43/lpcsdncsdncsdncytssfhnutfppepfgssghk",
    "ur:crypto-psbt/58-43/lpcsftcsdncsdncytssfhnutfpctwpmhtpfg",
    "ur:crypto-psbt/49-43/lpcsehcsdncsdncytssfhnutfppygybabdut",
    "ur:crypto-psbt/24-43/lpcscscsdncsdncytssfhnutfpenhtdeahgt",
    "ur:crypto-psbt/11-43/lpbdcsdncsdncytssfhnutfpctwlfwzmgh",
    "ur:crypto-psbt/54-43/lpcsencsdncsdncytssfhnutfpnnjkdadsjs",
    "ur:crypto-psbt/52-43/lpcseecsdncsdncytssfhnutfpqzfyonjset",
    "ur:crypto-psbt/13-43/lpbtcsdncsdncytssfhnutfpryfmosmkjo",
    "ur:crypto-psbt/55-43/lpcsemcsdncsdncytssfhnutfplslulnamst",
    "ur:crypto-psbt/45-43/lpcsdpcsdncsdncytssfhnutfpsgidptzsjt",
    "ur:crypto-psbt/34-43/lpcscpcsdncsdncytssfhnutfpdlrthljkvs",
    "ur:crypto-psbt/30-43/lpcsckcsdncsdncytssfhnutfpweoxbbpywn",
];

/// The message those parts carry, as foundation-ur-py reads it.
const MESSAGE: &str =
    "824031fda0660a421fbdbdd3f1b34fb2199a267464363fb824a682edfb46d52fc240b6608b9d57c3af";

#[test]
fn a_message_missing_fragments_is_completed_from_mixes_of_mixes() {
    let mut d = Decoder::new();
    for p in PARTS {
        d.receive(p).expect("a part of this message");
    }
    assert!(d.is_complete());
    let Some(Message::Psbt(data)) = d.message() else {
        panic!("no PSBT message");
    };
    let hex: String = data.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(hex, MESSAGE);
}
