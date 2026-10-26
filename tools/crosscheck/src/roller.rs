//! Faraday Demo's dice rollers (`faraday_core`'s `fresh/roller/`,
//! `docs/CODEX32.md` §5.5 and §5.6), driven through the testkit as a
//! host drives them, with Reduce motion on so that a click's values land
//! at once, and the system's answers given as fixed bytes.
//!
//! `roller.one` is the one die of New key's dice pad: clicks of a chosen
//! size into entries with a given room, and resets between them.
//! `roller.five` is the codex32 dice step's five dice: clicks of a chosen
//! reach into shares of a given length, of dice of `faces` faces (2 to
//! 20, six where not given), the host's worksheet kept with
//! `faraday_entropy::BookletChar` as the codex32 form keeps it.
//! `roller.faces` is every byte as a face of a die of `faces` faces
//! (`faraday_entropy::uniform_face`), and `roller.sheet` rolls entered
//! by hand on the worksheet: the characters they make and the pairs
//! struck (`docs/CODEX32.md` §5.1).

use faraday_core::testkit::five::{Act as FiveAct, FiveRoller, REACHES, Where};
use faraday_core::testkit::{PerClick, Roller, RollerAct};
use faraday_entropy::BookletChar;
use serde_json::{Value, json};

use crate::Answer;
use crate::json::{bytes_list, num, text};

/// Hands out the given answers in order, then none.
struct Answers {
    list: Vec<[u8; 32]>,
    next: usize,
}

impl Answers {
    fn new(req: &Value) -> Self {
        let list = bytes_list(req, "answers")
            .into_iter()
            .map(|a| a.try_into().expect("an answer is 32 bytes"))
            .collect();
        Answers { list, next: 0 }
    }

    fn draw(&mut self) -> Option<[u8; 32]> {
        let a = self.list.get(self.next).copied();
        if a.is_some() {
            self.next += 1;
        }
        a
    }
}

/// Handles `roller.<name>`.
pub fn handle(name: &str, req: &Value) -> Answer {
    match name {
        "one" => one(req),
        "five" => five(req),
        "faces" => {
            let n = num(req, "faces") as u8;
            let out: Vec<Value> = (0..=255u8)
                .map(|b| faraday_entropy::uniform_face(b, n).map_or(Value::Null, |f| json!(f)))
                .collect();
            Ok(json!({ "faces": out }))
        }
        "sheet" => sheet(req),
        _ => Err(format!("unknown roller op {name}")),
    }
}

fn per(step: &Value) -> PerClick {
    match text(step, "per") {
        "one" => PerClick::One,
        "five" => PerClick::Five,
        "ten" => PerClick::Ten,
        "end" => PerClick::ToEnd,
        _ => PerClick::Typed,
    }
}

/// The one die: each step a click or a reset, into entries that take
/// `room` values in all.
fn one(req: &Value) -> Answer {
    let mut answers = Answers::new(req);
    let room = num(req, "room") as usize;
    let mut roller = Roller::default();
    let mut values: Vec<u8> = Vec::new();
    for step in req["steps"].as_array().expect("steps is a list") {
        match text(step, "do") {
            "reset" => roller.reset(),
            "click" => {
                let p = per(step);
                roller.act(RollerAct::Per(p), 0, 0);
                if p == PerClick::Typed {
                    while !roller.typed().is_empty() {
                        roller.backspace();
                    }
                    for c in text(step, "typed").chars() {
                        roller.digit(c);
                    }
                }
                roller.act(RollerAct::Roll, room - values.len(), 0);
                loop {
                    let (got, _) =
                        roller.tick(0, true, values.len(), room - values.len(), &mut || {
                            answers.draw()
                        });
                    if got.is_empty() {
                        break;
                    }
                    values.extend(got);
                }
            }
            d => panic!("no step {d}"),
        }
    }
    Ok(json!({"values": values, "answers_used": answers.next}))
}

/// The host's codex32 entry as the form keeps it: the character in
/// progress on the worksheet, its faces with struck pairs, and the
/// characters made.
struct Host {
    faces: u8,
    sheet: BookletChar,
    seq: Vec<u8>,
    chars: String,
    share_len: usize,
    share_left: usize,
    all_left: usize,
}

impl Host {
    fn at(&self) -> Where<'_> {
        Where {
            seq: &self.seq,
            done: self.chars.len(),
            share_left: self.share_left,
            all_left: self.all_left,
        }
    }

    /// One face entered as the pad enters it.
    fn enter(&mut self, face: u8) {
        let second = (0..5).find(|&d| self.sheet.first(d).is_none()).is_none();
        let die = (0..5).find(|&d| self.sheet.second(d).is_none());
        self.sheet.push(face).expect("a face the sheet takes");
        self.seq.push(face);
        if let (true, Some(d)) = (second, die)
            && self.sheet.die_bit(d) == Some(None)
        {
            self.sheet.redo(d);
        }
        if let Ok(c) = self.sheet.character() {
            self.chars.push(char::from(c));
            self.sheet = BookletChar::with_faces(self.faces).expect("faces checked");
            self.seq.clear();
            self.all_left -= 1;
            self.share_left -= 1;
            if self.share_left == 0 {
                self.share_left = self.share_len.min(self.all_left);
            }
        }
    }
}

/// The five dice: each step a click of a reach, into `shares` shares
/// of `share_len` characters.
fn five(req: &Value) -> Answer {
    let mut answers = Answers::new(req);
    let share_len = num(req, "share_len") as usize;
    let all = share_len * num(req, "shares") as usize;
    let faces = req.get("faces").and_then(Value::as_u64).unwrap_or(6) as u8;
    let mut host = Host {
        faces,
        sheet: BookletChar::with_faces(faces).map_err(|e| format!("{e:?}"))?,
        seq: Vec::new(),
        chars: String::new(),
        share_len,
        share_left: share_len.min(all),
        all_left: all,
    };
    let mut roller = FiveRoller::default();
    roller.set_faces(faces);
    let mut faces: Vec<u8> = Vec::new();
    for step in req["steps"].as_array().expect("steps is a list") {
        let reach = REACHES[num(step, "reach") as usize];
        roller.act(FiveAct::Reach(reach), &host.at(), 0);
        roller.act(FiveAct::Roll, &host.at(), 0);
        loop {
            let (got, _) = roller.tick(0, true, &host.at(), &mut || answers.draw());
            if got.is_empty() {
                break;
            }
            for f in got {
                host.enter(f);
                faces.push(f);
            }
        }
    }
    Ok(json!({
        "faces": faces,
        "chars": host.chars,
        "pending": host.seq,
        "answers_used": answers.next,
    }))
}

/// Rolls entered by hand in the pad's order on dice of `faces` faces:
/// the characters they make, the pairs struck as equal (die from 0,
/// first, second) per character and for the one in progress, and the
/// bits of the one in progress.
fn sheet(req: &Value) -> Answer {
    let faces = num(req, "faces") as u8;
    let new = || BookletChar::with_faces(faces).map_err(|e| format!("{e:?}"));
    let mut sheet = new()?;
    let mut chars = String::new();
    let mut struck: Vec<Vec<[u8; 3]>> = vec![Vec::new()];
    for r in crate::json::nums(req, "rolls") {
        let face = r as u8;
        let second = (0..5).find(|&d| sheet.first(d).is_none()).is_none();
        let die = (0..5).find(|&d| sheet.second(d).is_none());
        sheet.push(face).map_err(|e| format!("{e:?}"))?;
        if let (true, Some(d)) = (second, die)
            && sheet.die_bit(d) == Some(None)
        {
            let first = sheet.first(d).unwrap_or(0);
            if let Some(list) = struck.last_mut() {
                list.push([d as u8, first, face]);
            }
            sheet.redo(d);
        }
        if let Ok(c) = sheet.character() {
            chars.push(char::from(c));
            sheet = new()?;
            struck.push(Vec::new());
        }
    }
    let bits: Vec<u8> = (0..5).filter_map(|d| sheet.die_bit(d).flatten()).collect();
    Ok(json!({ "chars": chars, "struck": struck, "bits": bits }))
}
