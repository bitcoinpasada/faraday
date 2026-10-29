//! The five-dice roller against the codex32 flow's own entry
//! (`docs/CODEX32.md` §5.5): a click fills the booklet's sheet exactly as
//! the same faces typed would, and the faces it made are marked.

use super::Sim;
use super::five::{Act, FiveRoller, Reach, Where};
use crate::Faraday;
use crate::codex32::{C32, Codex32Make, DIE_SIZES};
use crate::testkit;
use faraday_draw::tokens::{ROLLER_PAUSE_MS, ROLLER_TUMBLE_MS};
use faraday_ui::keyboard::OskPress;

/// The flow open on its entry at 2 of 3, 48 characters, and the faces
/// the host has entered into the character in progress.
struct Host {
    m: Codex32Make,
    seq: Vec<u8>,
    done: usize,
    faces: u8,
}

impl Host {
    fn new() -> Host {
        Host::with_faces(6)
    }

    /// The entry on dice of `faces` faces, chosen on the Entropy card.
    fn with_faces(faces: u8) -> Host {
        let c = testkit::compute();
        let mut m = Codex32Make::with_scheme(2, 3, Vec::new());
        for ch in "name".chars() {
            m.act(&c, C32::Key(OskPress::Char(ch)), None);
        }
        m.act(&c, C32::Next, None);
        let size = DIE_SIZES.iter().position(|&n| n == faces).unwrap();
        m.act(&c, C32::Faces(size as u8), None);
        m.act(&c, C32::Next, None);
        assert!(m.front().is_some(), "the entry is open");
        assert_eq!(m.dice_choice().faces, faces);
        Host {
            m,
            seq: Vec::new(),
            done: 0,
            faces,
        }
    }

    fn entered(&self) -> usize {
        let w = self.m.work().unwrap();
        (0..w.scheme().initial_indices().len())
            .map(|s| w.entered(s))
            .sum()
    }

    fn at(&self) -> Where<'_> {
        let w = self.m.work().unwrap();
        let data = w.scheme().length().data_len();
        let all_left = w.scheme().initial_indices().len() * data - self.entered();
        let share_left = self.m.front().map_or(0, |(_, n)| data - n);
        Where {
            seq: &self.seq,
            done: self.done,
            share_left,
            all_left,
        }
    }

    /// A face, as the pad's key: the six-face pad's digit, or the key of
    /// the face on the pad of other dice.
    fn press(&mut self, face: u8) {
        let c = testkit::compute();
        if self.faces == 6 {
            let ch = char::from(b'0' + face);
            self.m.act(&c, C32::Key(OskPress::Char(ch)), None);
        } else {
            self.m.act(&c, C32::Face(face), None);
        }
        self.seq.push(face);
        if self.entered() > self.done {
            self.done = self.entered();
            self.seq.clear();
        }
    }

    /// Backspace, then what the roller is told.
    fn back(&mut self, roller: &mut FiveRoller, restored: &[u8]) {
        let c = testkit::compute();
        self.m.act(&c, C32::Key(OskPress::Back), None);
        if self.seq.pop().is_none() {
            // The last character comes back less its last face.
            self.done = self.entered();
            self.seq = restored.to_vec();
        }
        roller.trim(self.done, self.seq.len());
    }
}

/// A stream of fresh answers that gives `faces`, then more.
fn stream(faces: &[u8]) -> impl FnMut() -> Option<[u8; 32]> + use<> {
    let mut next = faces.to_vec();
    let mut state = 0x2545_f491_u32;
    move || {
        let mut out = [0u8; 32];
        // The roller takes an answer's bytes from its end.
        for slot in out.iter_mut().rev() {
            *slot = if next.is_empty() {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                (state >> 8) as u8
            } else {
                next.remove(0) - 1
            };
        }
        Some(out)
    }
}

/// A stream of fresh answers whose bytes, in the order the roller uses
/// them, are `bytes`, then zeros.
fn raw(bytes: &[u8]) -> impl FnMut() -> Option<[u8; 32]> + use<> {
    let mut next = bytes.to_vec();
    move || {
        let mut out = [0u8; 32];
        for slot in out.iter_mut().rev() {
            *slot = if next.is_empty() { 0 } else { next.remove(0) };
        }
        Some(out)
    }
}

/// Ticks, entering what lands, until nothing is rolling; returns every
/// face that landed.
fn run(
    h: &mut Host,
    r: &mut FiveRoller,
    reduce: bool,
    draw: &mut impl FnMut() -> Option<[u8; 32]>,
) -> Vec<u8> {
    let mut all = Vec::new();
    let mut now = 0;
    for _ in 0..100_000 {
        let (faces, _) = r.tick(now, reduce, &h.at(), draw);
        for &f in &faces {
            h.press(f);
        }
        all.extend(faces);
        if !r.active() {
            break;
        }
        now += 16;
    }
    all
}

fn roll(h: &Host, r: &mut FiveRoller, now: u64) {
    r.act(Act::Roll, &h.at(), now);
}

/// The known answers (`tools/vectors/roller/`, whose README writes out
/// §5.5 and §5.6 and a few lines of Python that reproduce them).
const VECTORS: &str = include_str!("../../../../../tools/vectors/roller/vectors.json");

/// A known answer of the five dice: the faces of its dice, the bytes in
/// the order the roller uses them, the faces they land, and the value of
/// the character the sheet makes.
struct Known {
    faces: u8,
    bytes: Vec<u8>,
    rolls: Vec<u8>,
    value: u8,
}

fn known(name: &str) -> Known {
    let v: serde_json::Value = serde_json::from_str(VECTORS).expect("the vectors are JSON");
    let case = v["five_dice"]
        .as_array()
        .expect("the five dice")
        .iter()
        .find(|c| c["name"] == name)
        .unwrap_or_else(|| panic!("no case {name}"));
    let n = |x: &serde_json::Value| u8::try_from(x.as_u64().expect("a number")).unwrap();
    let list = |k: &str| case[k].as_array().expect("a list").iter().map(n).collect();
    let k = Known {
        faces: n(&case["faces"]),
        bytes: list("bytes"),
        rolls: list("rolls"),
        value: n(&case["value"]),
    };
    let tree = v["tree"].as_str().unwrap().as_bytes();
    assert_eq!(
        case["character"].as_str().unwrap().as_bytes(),
        &tree[usize::from(k.value)..=usize::from(k.value)]
    );
    k
}

/// A six-faced case's faces, whose bytes are each face less one, as
/// [`stream`] makes them.
fn six(name: &str) -> Vec<u8> {
    let k = known(name);
    assert_eq!(k.faces, 6);
    assert!(k.bytes.iter().zip(&k.rolls).all(|(b, f)| b + 1 == *f));
    k.rolls
}

/// First row 1 to 5, second row 2 to 6: every die higher, ten faces.
fn clean() -> Vec<u8> {
    six("clean")
}

/// Die 1's two values equal, so it is rolled again (6 then 2) before the
/// other dice's second values, as the pad takes them.
fn tied() -> Vec<u8> {
    six("tied")
}

#[test]
fn the_step_through_rolls_a_character_as_a_click_of_one_character_does() {
    // The same answers give the same faces whether a click lands them or
    // the step-through rolls them ahead (`docs/CODEX32.md` §5.5).
    for faces in [&clean()[..], &tied()[..]] {
        let mut h = Host::new();
        let mut clicked = FiveRoller::default();
        roll(&h, &mut clicked, 0);
        let landed = run(&mut h, &mut clicked, true, &mut stream(faces));
        let mut ahead = FiveRoller::default();
        let rolled = ahead.roll_character(&[], &mut stream(faces)).unwrap();
        assert_eq!(rolled.as_slice(), landed.as_slice());
        assert_eq!(rolled.as_slice(), faces);
    }
}

#[test]
fn a_character_click_gives_the_character_the_same_faces_typed_give() {
    let mut h = Host::new();
    let mut r = FiveRoller::default();
    r.act(Act::Reach(Reach::Character), &h.at(), 0);
    roll(&h, &mut r, 0);
    let clean = known("clean");
    let faces = run(&mut h, &mut r, true, &mut raw(&clean.bytes));
    assert_eq!(faces, clean.rolls);
    assert_eq!(h.entered(), 1);
    // Every die higher: 11111, "l".
    assert_eq!(h.m.made_of(0, 0).unwrap().value(), clean.value);
    let mut typed = Host::new();
    for f in clean.rolls {
        typed.press(f);
    }
    assert_eq!(
        h.m.made_of(0, 0).unwrap().value(),
        typed.m.made_of(0, 0).unwrap().value()
    );
}

#[test]
fn an_equal_pair_is_rolled_again() {
    let mut h = Host::new();
    let mut r = FiveRoller::default();
    roll(&h, &mut r, 0);
    let tied = known("tied");
    let faces = run(&mut h, &mut r, true, &mut raw(&tied.bytes));
    assert_eq!(faces, tied.rolls);
    assert_eq!(h.entered(), 1);
    // Die 1 rolled again: 6 then 2 gives bit 0; the others 1 0 1 0:
    // 01010, "2".
    assert_eq!(h.m.made_of(0, 0).unwrap().value(), tied.value);
    assert!(
        h.m.made_of(0, 0)
            .unwrap()
            .rows()
            .iter()
            .any(|row| row.bit().is_none()),
        "the struck pair stays on the sheet"
    );
}

#[test]
fn a_pass_is_the_five_dice_once() {
    let mut h = Host::new();
    let mut r = FiveRoller::default();
    r.act(Act::Reach(Reach::Pass), &h.at(), 0);
    let mut draw = stream(&tied());
    roll(&h, &mut r, 0);
    assert_eq!(run(&mut h, &mut r, true, &mut draw).len(), 5);
    assert_eq!(h.m.dice().len(), 5);
    roll(&h, &mut r, 0);
    assert_eq!(run(&mut h, &mut r, true, &mut draw), [1]);
    assert_eq!(h.entered(), 0, "the equal pair struck its die");
    assert_eq!(h.m.equal(), Some(0));
    // Only the tied die is rolled, first then second, and the rest.
    roll(&h, &mut r, 0);
    assert_eq!(run(&mut h, &mut r, true, &mut draw), [6]);
    roll(&h, &mut r, 0);
    assert_eq!(run(&mut h, &mut r, true, &mut draw), [2, 3, 2, 5, 4]);
    assert_eq!(h.entered(), 1);
}

#[test]
fn a_share_and_the_whole_set_fill_as_the_pad_would() {
    let mut h = Host::new();
    let mut r = FiveRoller::default();
    r.act(Act::Reach(Reach::Share), &h.at(), 0);
    roll(&h, &mut r, 0);
    let faces = run(&mut h, &mut r, true, &mut stream(&[]));
    assert_eq!(h.entered(), 26, "one share");
    assert_eq!(h.m.front(), Some((1, 0)));
    // The same faces typed give the same share.
    let mut typed = Host::new();
    for &f in &faces {
        typed.press(f);
    }
    assert_eq!(typed.entered(), 26);
    for n in 0..26 {
        assert_eq!(
            h.m.made_of(0, n).unwrap().value(),
            typed.m.made_of(0, n).unwrap().value()
        );
    }
    r.act(Act::Reach(Reach::End), &h.at(), 0);
    roll(&h, &mut r, 0);
    run(&mut h, &mut r, true, &mut stream(&[]));
    assert_eq!(h.entered(), 52);
    assert!(h.m.strings().is_some(), "the set is made");
    assert!(!r.rolling());
}

#[test]
fn every_face_it_made_is_marked_and_a_typed_one_is_not() {
    let mut h = Host::new();
    let mut r = FiveRoller::default();
    roll(&h, &mut r, 0);
    run(&mut h, &mut r, true, &mut stream(&clean()));
    assert!((0..10).all(|p| r.is_rolled(0, p)));
    assert!(r.char_rolled(0));
    assert_eq!(r.sim(10), Sim::All);
    // A face typed after: not marked, and the result says both.
    h.press(3);
    assert!(!r.is_rolled(1, 0));
    assert!(!r.char_rolled(1));
    assert_eq!(r.sim(11), Sim::Some);
    // Backspace takes marks back with their faces.
    h.back(&mut r, &[]);
    assert!((0..10).all(|p| r.is_rolled(0, p)));
    h.back(&mut r, &clean()[..9]);
    assert_eq!(h.entered(), 0);
    assert!(r.is_rolled(0, 8) && !r.is_rolled(0, 9));
    r.reset();
    assert_eq!(r.sim(0), Sim::No);
}

#[test]
fn the_dice_tumble_and_a_character_lands_with_a_pause() {
    let mut h = Host::new();
    let mut r = FiveRoller::default();
    let mut draw = stream(&clean());
    roll(&h, &mut r, 0);
    assert!(r.rolling() && r.tumbling(0));
    let (faces, redraw) = r.tick(ROLLER_TUMBLE_MS / 2, false, &h.at(), &mut draw);
    assert!(faces.is_empty() && redraw);
    // The first pass lands; the second tumbles at once.
    let (faces, _) = r.tick(ROLLER_TUMBLE_MS, false, &h.at(), &mut draw);
    assert_eq!(faces, &clean()[..5]);
    for f in faces {
        h.press(f);
    }
    assert!(r.tumbling(0));
    let t = 2 * ROLLER_TUMBLE_MS;
    let (faces, _) = r.tick(t, false, &h.at(), &mut draw);
    assert_eq!(faces, &clean()[5..]);
    for f in faces {
        h.press(f);
    }
    // The character landed: a pause, then rest.
    assert!(!r.rolling() && r.active() && !r.tumbling(0));
    let shown: Vec<u8> = (0..5).map(|d| r.shown(d)).collect();
    assert_eq!(shown, [2, 3, 4, 5, 6]);
    let _ = r.tick(t + ROLLER_PAUSE_MS, false, &h.at(), &mut draw);
    assert!(!r.active());
}

#[test]
fn reduce_motion_lands_everything_at_once() {
    let h = Host::new();
    let mut r = FiveRoller::default();
    roll(&h, &mut r, 0);
    let (faces, _) = r.tick(0, true, &h.at(), &mut stream(&clean()));
    assert_eq!(faces, clean());
    assert!(!r.active());
}

#[test]
fn stop_ends_the_rolls_and_a_full_entry_is_not_rolled() {
    let h = Host::new();
    let mut r = FiveRoller::default();
    r.act(Act::Reach(Reach::End), &h.at(), 0);
    roll(&h, &mut r, 0);
    r.stop();
    assert!(!r.active());
    let (faces, redraw) = r.tick(10_000, false, &h.at(), &mut stream(&[]));
    assert!(faces.is_empty() && !redraw);
    let full = Where {
        seq: &[],
        done: 52,
        share_left: 0,
        all_left: 0,
    };
    r.act(Act::Roll, &full, 0);
    assert!(!r.rolling());
}

#[test]
fn it_is_offered_only_where_the_shell_says_so() {
    let mut app = Faraday::new();
    assert!(!app.roller_offered(), "not on a device shell by default");
    app.set_roller_offered(false);
    assert!(!app.roller_offered());
}

#[test]
fn the_result_says_simulated_dice_alone_or_with_the_persons() {
    use super::five_screen::entropy_label;
    assert_eq!(entropy_label(Sim::No), None);
    assert_eq!(entropy_label(Sim::All), Some("Entropy · simulated dice"));
    assert_eq!(
        entropy_label(Sim::Some),
        Some("Entropy · simulated dice and your dice")
    );
}

#[test]
fn twenty_faced_dice_take_a_byte_below_240_as_its_face_plus_one() {
    // 240 to 255 are skipped, as 256 is not a multiple of 20: 239 is 20,
    // 0 and 20 are 1, 19 is 20.
    // The bytes 255, 240, 239, 0, 20, 19, 100, 241, 5, 7, 8, 9, 10 land
    // 20, 1, 1, 20, 1, 6, 8, 9, 10, 11.
    let twenty = known("twenty");
    assert_eq!(twenty.faces, 20);
    let mut h = Host::with_faces(20);
    let mut r = FiveRoller::default();
    r.set_faces(20);
    roll(&h, &mut r, 0);
    let faces = run(&mut h, &mut r, true, &mut raw(&twenty.bytes));
    assert_eq!(faces, twenty.rolls);
    // 20 then 6 lower, 1 then 8 higher, 1 then 9 higher, 20 then 10
    // lower, 1 then 11 higher: 01101, "d".
    assert_eq!(h.m.made_of(0, 0).unwrap().value(), twenty.value);
    let mut typed = Host::with_faces(20);
    for f in faces {
        typed.press(f);
    }
    assert_eq!(typed.m.made_of(0, 0).unwrap().value(), twenty.value);
}

#[test]
fn every_face_of_every_die_offered_comes_up_as_often() {
    // A long stream of bytes (xorshift), each die's faces counted: the
    // chi-square of the counts stays under its 0.1% point, and a face
    // past the die never comes.
    for &n in &DIE_SIZES {
        let mut state = 0x9e37_79b9_u32 ^ u32::from(n);
        let mut draw = move || {
            let mut out = [0u8; 32];
            for slot in &mut out {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                *slot = (state >> 24) as u8;
            }
            Some(out)
        };
        let mut pool = super::Pool::default();
        let total = 60_000u32;
        let mut count = [0u32; 21];
        for _ in 0..total {
            let f = pool.next(n, &mut draw).unwrap();
            count[usize::from(f)] += 1;
        }
        assert_eq!(count[0], 0);
        assert!(count[usize::from(n) + 1..].iter().all(|&c| c == 0), "d{n}");
        let expect = f64::from(total) / f64::from(n);
        let chi: f64 = count[1..=usize::from(n)]
            .iter()
            .map(|&c| (f64::from(c) - expect).powi(2) / expect)
            .sum();
        // The chi-square's 0.1% points for 3 to 19 degrees of freedom.
        // lint-tokens: sample data
        let limit = match n {
            4 => 16.27,
            6 => 20.52,
            8 => 24.32,
            10 => 27.88,
            12 => 31.26,
            _ => 43.82,
        };
        // lint-tokens: end sample data
        assert!(chi < limit, "d{n}: chi-square {chi}");
    }
}
