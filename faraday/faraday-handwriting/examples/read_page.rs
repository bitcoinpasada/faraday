//! Reads the page in a binary PGM (P5, 8 bits): `read_page <file.pgm>`.
//!
//! Prints a line about the page, then for each list its form (or `none`),
//! its questions and its words in the order read, each as `clear <word>` or
//! `unclear <word> ...` (against the form's list, or against every list when
//! the form is not decided), with the line number read beside it (`#n`) and
//! `edge` if its box touches the frame's edge. `across` lists the other
//! order of a list when the order is a question.

use faraday_handwriting::{Form, Page, Question, Reading, read_page};

/// Next whitespace-separated token of a PGM header, skipping `#` comments.
fn token(data: &[u8], at: &mut usize) -> Option<String> {
    loop {
        while data.get(*at).is_some_and(u8::is_ascii_whitespace) {
            *at += 1;
        }
        if data.get(*at) == Some(&b'#') {
            while data.get(*at).is_some_and(|&c| c != b'\n') {
                *at += 1;
            }
        } else {
            break;
        }
    }
    let start = *at;
    while data.get(*at).is_some_and(|c| !c.is_ascii_whitespace()) {
        *at += 1;
    }
    let t = data.get(start..*at)?;
    (!t.is_empty()).then(|| String::from_utf8_lossy(t).into_owned())
}

/// Width, height and pixels of a binary 8-bit PGM.
fn parse_pgm(data: &[u8]) -> Option<(usize, usize, &[u8])> {
    let mut at = 0;
    if token(data, &mut at)? != "P5" {
        return None;
    }
    let width: usize = token(data, &mut at)?.parse().ok()?;
    let height: usize = token(data, &mut at)?.parse().ok()?;
    if token(data, &mut at)?.parse::<usize>().ok()? != 255 {
        return None;
    }
    // One whitespace byte ends the header.
    let pixels = data.get(at + 1..)?;
    let n = width.checked_mul(height)?;
    Some((width, height, pixels.get(..n)?))
}

fn describe(r: &Reading) -> String {
    match r {
        Reading::Clear(c) => format!("clear {}", c.word()),
        Reading::Unclear(list) => {
            let names: Vec<&str> = list.iter().map(|c| c.word()).collect();
            format!("unclear {}", names.join(" "))
        }
    }
}

fn form_name(f: Option<Form>) -> &'static str {
    match f {
        Some(Form::Bip39) => "bip39",
        Some(Form::Slip39) => "slip39",
        Some(Form::Extra) => "extra",
        None => "none",
    }
}

fn question_name(q: &Question) -> &'static str {
    match q {
        Question::Order => "order",
        Question::Count => "count",
        Question::Form => "form",
        Question::Numbers => "numbers",
        Question::Split => "split",
        Question::Missing => "missing",
    }
}

fn print(page: &Page) {
    println!(
        "page lists={} cut_off={} unattached_numbers={} too_many={}",
        page.lists.len(),
        page.maybe_cut_off,
        page.unattached_numbers,
        page.too_many
    );
    for (k, list) in page.lists.iter().enumerate() {
        let qs: Vec<&str> = list.questions.iter().map(question_name).collect();
        println!(
            "list {k} form={} words={} questions={}",
            form_name(list.form),
            list.words.len(),
            qs.join(",")
        );
        let fit: Vec<String> = list.fit.iter().map(|f| format!("{f:.1}")).collect();
        println!("  fit {}", fit.join(" "));
        for (i, w) in list.words.iter().enumerate() {
            let number = w
                .number
                .and_then(|n| n.value)
                .map_or(String::new(), |v| format!(" #{v}"));
            let edge = match (w.edge, w.tall) {
                (true, true) => " edge tall",
                (true, false) => " edge",
                (false, true) => " tall",
                (false, false) => "",
            };
            let lists: Vec<usize> = match list.list {
                Some(l) => vec![l],
                None => (0..page.list_names.len()).collect(),
            };
            for l in lists {
                if let Some(r) = w.reading(l) {
                    let name = form_name(page.list_names.get(l).copied());
                    println!("  {i}{number}{edge} [{name}] {}", describe(r));
                }
            }
        }
        if let Some(a) = &list.across {
            let a: Vec<String> = a.iter().map(usize::to_string).collect();
            println!("  across {}", a.join(" "));
        }
    }
}

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: read_page <file.pgm>");
        std::process::exit(2);
    };
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{path}: {e}");
            std::process::exit(1);
        }
    };
    let Some((width, height, gray)) = parse_pgm(&data) else {
        eprintln!("{path}: not a binary 8-bit PGM");
        std::process::exit(1);
    };
    match read_page(width, height, gray) {
        Ok(page) => print(&page),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
