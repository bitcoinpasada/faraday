//! Entries from text (`PLAN.md` §6.4): an Inbox text file of
//! `otpauth://` URIs and `field: value` lines, or a TOTP setup code read
//! by the camera. CSV and KeePass are not read.
//!
//! A blank line ends an entry. Within one, `title`, `username`, `password`,
//! `url`, `notes` and `totp` lines (with a few common other names for
//! each) fill those fields; an `otpauth://` line is the TOTP secret, and
//! names the entry from its label when no title line does. Any other
//! `field: value` line is kept in the notes, so nothing typed is lost.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::records::{Record, field, kind};

/// The field a `name:` line fills.
fn field_of(name: &str) -> Option<u8> {
    Some(match name.trim().to_ascii_lowercase().as_str() {
        "title" | "name" => field::TITLE,
        "username" | "user" | "login" | "email" => field::USERNAME,
        "password" | "pass" => field::PASSWORD,
        "url" | "website" | "site" => field::URL,
        "notes" | "note" => field::NOTES,
        "totp" | "otp" | "otpauth" => field::TOTP,
        _ => return None,
    })
}

/// `%XX` escapes undone, as an `otpauth://` label carries them.
fn percent_decoded(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Ok(v) =
                u8::from_str_radix(core::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("zz"), 16)
        {
            out.push(v);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// What an `otpauth://` URI names: the issuer, or the label before its
/// colon, and the account after it.
pub fn otpauth_names(uri: &str) -> Option<(String, String)> {
    let rest = uri.trim().strip_prefix("otpauth://")?;
    let (_, rest) = rest.split_once('/')?;
    let (label, query) = rest.split_once('?').unwrap_or((rest, ""));
    let label = percent_decoded(label);
    let (from_label, account) = match label.split_once(':') {
        Some((i, a)) => (i.trim().to_string(), a.trim().to_string()),
        None => (String::new(), label.trim().to_string()),
    };
    let issuer = query
        .split('&')
        .find_map(|p| p.strip_prefix("issuer="))
        .map(percent_decoded)
        .unwrap_or_default();
    let title = if !issuer.is_empty() {
        issuer
    } else if !from_label.is_empty() {
        from_label
    } else {
        account.clone()
    };
    Some((title, account))
}

/// Whether text looks like entries: an `otpauth://` line, or a `title:`
/// line.
pub fn looks_like_entries(text: &str) -> bool {
    text.lines()
        .map(str::trim)
        .any(|l| l.starts_with("otpauth://") || l.to_ascii_lowercase().starts_with("title:"))
}

/// The entries in text, as records. A block with nothing to call it by is
/// left out.
pub fn parse(text: &str) -> Vec<Record> {
    let mut out = Vec::new();
    let mut fields: Vec<(u8, String)> = Vec::new();
    let mut extra: Vec<String> = Vec::new();
    let finish = |fields: &mut Vec<(u8, String)>,
                  extra: &mut Vec<String>,
                  out: &mut Vec<Record>| {
        if fields.is_empty() && extra.is_empty() {
            return;
        }
        let get =
            |f: &Vec<(u8, String)>, n: u8| f.iter().find(|(k, _)| *k == n).map(|(_, v)| v.clone());
        let mut title = get(fields, field::TITLE);
        let mut user = get(fields, field::USERNAME);
        if let Some(t) = get(fields, field::TOTP)
            && let Some((name, account)) = otpauth_names(&t)
        {
            if title.is_none() && !name.is_empty() {
                title = Some(name);
            }
            if user.is_none() && !account.is_empty() {
                user = Some(account);
            }
        }
        if let Some(title) = title {
            let mut r = Record::new(kind::ENTRY).with(field::TITLE, title.as_bytes());
            if let Some(u) = user {
                r.push(field::USERNAME, u.as_bytes());
            }
            for n in [field::PASSWORD, field::URL] {
                if let Some(v) = get(fields, n) {
                    r.push(n, v.as_bytes());
                }
            }
            let mut notes: Vec<String> = get(fields, field::NOTES).into_iter().collect();
            notes.append(extra);
            if !notes.is_empty() {
                r.push(field::NOTES, notes.join("\n").as_bytes());
            }
            if let Some(t) = get(fields, field::TOTP) {
                r.push(field::TOTP, t.as_bytes());
            }
            out.push(r);
        }
        fields.clear();
        extra.clear();
    };
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() {
            finish(&mut fields, &mut extra, &mut out);
            continue;
        }
        if l.starts_with("otpauth://") {
            // A second code in one block starts a new entry.
            if fields.iter().any(|(k, _)| *k == field::TOTP) {
                finish(&mut fields, &mut extra, &mut out);
            }
            fields.push((field::TOTP, l.to_string()));
            continue;
        }
        match l.split_once(':') {
            Some((name, value)) if field_of(name).is_some() => {
                let n = field_of(name).expect("checked");
                let value = value.trim().to_string();
                if n == field::TITLE && fields.iter().any(|(k, _)| *k == field::TITLE) {
                    finish(&mut fields, &mut extra, &mut out);
                }
                fields.retain(|(k, _)| *k != n);
                fields.push((n, value));
            }
            _ => extra.push(l.to_string()),
        }
    }
    finish(&mut fields, &mut extra, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two forms §6.4 names, mixed, with a line no field takes.
    #[test]
    fn a_text_file_of_entries_reads() {
        let text = "otpauth://totp/Example%20Mail:you%40example.com?secret=JBSWY3DPEHPK3PXP&issuer=Example\n\
                    \n\
                    title: Bank\n\
                    username: me\n\
                    password: hunter2\n\
                    PIN: 1234\n\
                    \n\
                    otpauth://totp/GitHub:octo?secret=ABC\n";
        let e = parse(text);
        assert_eq!(e.len(), 3);
        assert_eq!(e[0].text(field::TITLE), Some("Example"));
        assert_eq!(e[0].text(field::USERNAME), Some("you@example.com"));
        assert_eq!(e[1].text(field::PASSWORD), Some("hunter2"));
        assert_eq!(e[1].text(field::NOTES), Some("PIN: 1234"));
        assert_eq!(e[2].text(field::TITLE), Some("GitHub"));
        assert!(e.iter().all(|r| r.check().is_ok()));
    }
}
