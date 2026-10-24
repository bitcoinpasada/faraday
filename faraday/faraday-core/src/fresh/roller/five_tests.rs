//! The five-dice roller against the codex32 flow's own entry
//! (`docs/CODEX32.md` §5.5): a click fills the booklet's sheet exactly as
//! the same faces typed would, and the faces it made are marked.

use super::Sim;
use super::five::{Act, FiveRoller, Reach, Where};
use crate::Faraday;
use crate::codex32::{C32, Codex32Make};
use crate::testkit;
use faraday_draw::tokens::{ROLLER_PAUSE_MS, ROLLER_TUMBLE_MS};
use faraday_ui::keyboard::OskPress;

/// The flow open on its entry at 2 of 3, 48 characters, and the faces
/// the host has entered into the character in progress.
struct Host {
    m: Codex32Make,
    seq: Vec<u8>,
    done: usize,
}

impl Host {
    fn new() -> Host {
        let c = testkit::compute();
        let mut m = Codex32Make::with_scheme(2, 3, Vec::new());
        for ch in "name".chars() {
            m.act(&c, C32::Key(OskPress::Char(ch)), None);
        }
        m.act(&c, C32::Next, None);
        m.act(&c, C32::Next, None);
        assert!(m.front().is_some(), "the entry is open");
        Host {
            m,
            seq: Vec::new(),
            done: 0,
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

    /// A face, as the pad's key.
    fn press(&mut self, face: u8) {
        let c = testkit::compute();
        let ch = char::from(b'0' + face);
        self.m.act(&c, C32::Key(OskPress::Char(ch)), None);
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
fn stream(faces: &[u8]) -> impl FnMut() -> Option<[u8; 32]> {
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

/// First row 1 to 5, second row 2 to 6: every die higher, ten faces.
const CLEAN: [u8; 10] = [1, 2, 3, 4, 5, 2, 3, 4, 5, 6];
/// Die 1's two values equal, so it is rolled again (6 then 2) before the
/// other dice's second values, as the pad takes them.
const TIED: [u8; 12] = [1, 2, 3, 4, 5, 1, 6, 2, 3, 2, 5, 4];

#[test]
fn a_character_click_gives_the_character_the_same_faces_typed_give() {
    let mut h = Host::new();
    let mut r = FiveRoller::default();
    r.act(Act::Reach(Reach::Character), &h.at(), 0);
    roll(&h, &mut r, 0);
    let faces = run(&mut h, &mut r, true, &mut stream(&CLEAN));
    assert_eq!(faces, CLEAN);
    assert_eq!(h.entered(), 1);
    let mut typed = Host::new();
    for f in CLEAN {
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
    let faces = run(&mut h, &mut r, true, &mut stream(&TIED));
    assert_eq!(faces, TIED);
    assert_eq!(h.entered(), 1);
    // Die 1: 2 then 6 gives bit 0; the others 1 0 1 0: 01010, "2".
    assert_eq!(h.m.made_of(0, 0).unwrap().value(), 10);
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
    let mut draw = stream(&TIED);
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
    run(&mut h, &mut r, true, &mut stream(&CLEAN));
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
    h.back(&mut r, &CLEAN[..9]);
    assert_eq!(h.entered(), 0);
    assert!(r.is_rolled(0, 8) && !r.is_rolled(0, 9));
    r.reset();
    assert_eq!(r.sim(0), Sim::No);
}

#[test]
fn the_dice_tumble_and_a_character_lands_with_a_pause() {
    let mut h = Host::new();
    let mut r = FiveRoller::default();
    let mut draw = stream(&CLEAN);
    roll(&h, &mut r, 0);
    assert!(r.rolling() && r.tumbling(0));
    let (faces, redraw) = r.tick(ROLLER_TUMBLE_MS / 2, false, &h.at(), &mut draw);
    assert!(faces.is_empty() && redraw);
    // The first pass lands; the second tumbles at once.
    let (faces, _) = r.tick(ROLLER_TUMBLE_MS, false, &h.at(), &mut draw);
    assert_eq!(faces, &CLEAN[..5]);
    for f in faces {
        h.press(f);
    }
    assert!(r.tumbling(0));
    let t = 2 * ROLLER_TUMBLE_MS;
    let (faces, _) = r.tick(t, false, &h.at(), &mut draw);
    assert_eq!(faces, &CLEAN[5..]);
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
    let (faces, _) = r.tick(0, true, &h.at(), &mut stream(&CLEAN));
    assert_eq!(faces, CLEAN);
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
