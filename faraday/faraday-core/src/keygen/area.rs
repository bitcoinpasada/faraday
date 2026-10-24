//! New key's area: the flow being made, held in one field of `Faraday`,
//! and what other areas reach it through. The flow's state is [`KeyGen`]
//! in `mod.rs`.

use super::*;

/// New key's area: the flow being made, the keys made this session, the
/// camera and the shell's randomness source.
#[derive(Default)]
pub(crate) struct KeyGenArea {
    pub(super) state: Option<KeyGen>,
    /// New keys made this session, which number their labels.
    pub(super) new_keys: u32,
    /// The camera is on for camera noise.
    pub(super) camera_on: bool,
    /// The shell's randomness source (`Faraday::entropy_source_named`).
    pub(super) entropy_source: Option<&'static str>,
}

impl KeyGenArea {
    /// The flow being made, while it is open.
    pub(crate) fn state(&self) -> Option<&KeyGen> {
        self.state.as_ref()
    }

    /// Forgets the flow, with everything in it.
    pub(crate) fn forget(&mut self) {
        self.state = None;
    }

    /// Whether the camera is on for camera noise.
    pub(crate) fn camera_on(&self) -> bool {
        self.camera_on
    }

    /// The column's scroll, while the flow is open.
    pub(crate) fn scroll_mut(&mut self) -> Option<&mut flow::Scroll> {
        self.state.as_mut().map(|k| &mut k.scroll)
    }

    /// Takes the column's scroll as the pass left it.
    pub(crate) fn set_scroll(&mut self, to: flow::Scroll) {
        if let Some(k) = self.state.as_mut() {
            k.scroll = to;
        }
    }

    /// Takes a camera frame as noise, forgetting the one before.
    pub(crate) fn camera_frame(&mut self, luma: Vec<u8>) {
        if let Some(k) = self.state.as_mut() {
            if let Some(old) = k.frame.as_mut() {
                zeroize::Zeroize::zeroize(old);
            }
            k.frame = Some(luma);
        }
    }

    /// Demo's passphrase typed into field `i` of the Key card.
    pub(crate) fn demo_passphrase(&mut self, i: u8, text: &str) {
        if let Some(k) = self.state.as_mut() {
            let t = if i == 0 {
                &mut k.passphrase
            } else {
                &mut k.passphrase2
            };
            t.set(text);
            k.focus = Some(i);
            k.note = None;
        }
    }

    /// Whether the person has added randomness of their own changed
    /// (`docs/OWN-RANDOMNESS.md` §2.3): an open flow's This device says
    /// so. Bytes it holds from before, not yet made into words, go and
    /// are asked for again, so that what it says stays what they are.
    pub(crate) fn own_changed(&mut self, any: bool) {
        if let Some(k) = self.state.as_mut()
            && k.mnemonic.is_none()
        {
            k.with_own = any;
            k.device.clear();
            k.device_asked = false;
        }
    }

    /// The randomness source the shell named, if it named one.
    pub(crate) fn entropy_source(&self) -> Option<&'static str> {
        self.entropy_source
    }

    /// The shell named its randomness source: kept, and copied into an
    /// open flow, as a new one copies it when it opens.
    pub(crate) fn entropy_source_named(&mut self, name: &'static str) {
        self.entropy_source = Some(name);
        if let Some(k) = self.state.as_mut() {
            k.entropy_source = Some(name);
        }
    }

    /// The shell is asked for fresh bytes when this device's generator
    /// is the source being entered; they go to the key and nowhere else.
    pub(crate) fn device(&mut self, out: &mut crate::out::Out) {
        let Some(k) = self.state.as_mut() else {
            return;
        };
        if k.open == Some(kstep::ENTER)
            && k.active() == Some(Source::Device)
            && !k.device.is_enough(k.strength())
            && !k.device_asked
        {
            k.device_asked = true;
            out.command(faraday_shell_api::Command::RequestEntropy);
        }
    }

    /// Takes the shell's answer when the flow asked for it. Returns
    /// whether it did.
    pub(crate) fn entropy(&mut self, bytes: &mut [u8; 32], own: Option<&[u8; 32]>) -> bool {
        let Some(k) = self.state.as_mut().filter(|k| k.device_asked) else {
            return false;
        };
        match own {
            Some(acc) => k.device.set(&device_own(bytes, acc)),
            None => k.device.set(bytes),
        }
        k.device_asked = false;
        zeroize::Zeroize::zeroize(bytes);
        true
    }

    /// The camera is on while camera noise is being taken, and off
    /// otherwise.
    pub(crate) fn camera(&mut self, on_screen: bool, out: &mut crate::out::Out) {
        let want = on_screen
            && self.state.as_ref().is_some_and(|k| {
                k.open == Some(kstep::ENTER) && k.active() == Some(Source::Camera)
            });
        if want != self.camera_on {
            self.camera_on = want;
            out.command(if want {
                faraday_shell_api::Command::CameraOn
            } else {
                faraday_shell_api::Command::CameraOff
            });
        }
        if !want
            && let Some(k) = self.state.as_mut()
            && let Some(f) = k.frame.as_mut()
        {
            zeroize::Zeroize::zeroize(f);
            k.frame = None;
        }
    }

    /// Costs a seed's fingerprint and a share deal when the entry is
    /// finished. A press that changes only the flow. `draw` is the fresh
    /// draw dealing shares takes, asked for before the flow is borrowed.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub(crate) fn act(
        &mut self,
        action: crate::Action,
        c: &Compute,
        draw: Option<[u8; 32]>,
        now_ms: u64,
    ) {
        use crate::Action as A;
        let Some(k) = self.state.as_mut() else {
            return;
        };
        let open = |k: &mut KeyGen, step: u8| {
            k.open = Some(step);
            k.scroll.follow = true;
        };
        match action {
            A::KOthers => k.others_open = !k.others_open,
            A::KPassField(f) => {
                if k.open == Some(kstep::KEY) && k.asks_passphrase() && f < 2 {
                    k.focus = Some(f);
                    k.note = None;
                }
            }
            A::KStep(s) => {
                // A later card opens once the ones before it in this
                // flow are done.
                let steps = k.steps();
                let reachable = steps
                    .iter()
                    .take_while(|&&i| i != s)
                    .all(|&i| k.done[usize::from(i)])
                    && steps.contains(&s);
                // The cards of codex32 are opened by its own presses;
                // Start's and a closing are leaving it (§6.4.6).
                if let Some(m) = k.codex32.as_mut() {
                    m.mask();
                }
                if k.open == Some(s) {
                    k.open = None;
                } else if reachable {
                    // A step opens with its secret hidden (F104).
                    k.shown = false;
                    open(k, s);
                }
            }
            A::KForm(on) => k.set_slip39(on),
            A::KSlipWords(n) => {
                if k.slip39 && k.counts().contains(&n) && usize::from(n) != k.words {
                    k.words = usize::from(n);
                    k.clear_entries();
                }
            }
            A::KSlipM(d) => {
                k.slip_m = (i16::from(k.slip_m) + i16::from(d)).clamp(2, i16::from(k.slip_n)) as u8;
            }
            A::KSlipN(d) => {
                let max = faraday_bip::slip39::MAX_COUNT as i16;
                k.slip_n = (i16::from(k.slip_n) + i16::from(d)).clamp(2, max) as u8;
                k.slip_m = k.slip_m.min(k.slip_n);
            }
            A::KShare(d) => {
                let last = k.shares.len().saturating_sub(1) as i64;
                k.share_at = (k.share_at as i64 + i64::from(d)).clamp(0, last) as usize;
                k.shown = false;
            }
            A::KWords(n) => {
                let n = usize::from(n);
                if k.counts().iter().any(|c| usize::from(*c) == n) {
                    k.words = n;
                    k.clear_entries();
                    k.done[usize::from(kstep::LENGTH)] = true;
                    open(k, kstep::SOURCE);
                }
            }
            A::KWay(i) => {
                if let Some(&w) = WAYS.get(usize::from(i)) {
                    k.choose(w);
                    // A single choice moves on; a mix opens its sources
                    // to tick and keeps its Continue (F103).
                    if k.open == Some(kstep::SOURCE)
                        && k.source == Some(w.source())
                        && w != Way::Mix
                    {
                        k.source_next();
                    }
                }
            }
            A::KGroup(i) => {
                if let Some(open) = k.groups_open.get_mut(usize::from(i)) {
                    *open = !*open;
                }
            }
            A::KMix(i) => {
                if let Some(on) = k.mix.get_mut(usize::from(i)) {
                    *on = !*on;
                    k.clear_entries();
                }
            }
            A::KNext => match k.open {
                Some(kstep::SOURCE) => k.source_next(),
                Some(kstep::ENTER) => {
                    if k.slip39 && draw.is_none() {
                        k.note = Some(EN.keygen_no_randomness.to_string());
                        return;
                    }
                    if k.finish_entry() {
                        if k.slip39 {
                            if let Err(e) = draw.map_or(Ok(()), |r| k.deal(c, r)) {
                                k.note = Some(e);
                                return;
                            }
                            k.fingerprint = k.secret.as_ref().and_then(|s| {
                                faraday_keys::identity::fingerprint_of_seed(c, s).map(|f| f.0)
                            });
                        } else {
                            k.fingerprint = k.phrase().and_then(|w| {
                                faraday_keys::identity::fingerprint_of_words(c, &w, "")
                                    .ok()
                                    .map(|f| f.0)
                            });
                        }
                        k.done[usize::from(kstep::ENTER)] = true;
                        if k.slip39 {
                            open(k, kstep::CHECK);
                        } else {
                            // Words that showed as they came in stay on
                            // screen; hashed ones wait for Show words.
                            k.shown = k.reveals();
                            open(k, kstep::KEY);
                        }
                    }
                }
                Some(kstep::CHECK) => {
                    k.done[usize::from(kstep::CHECK)] = true;
                    open(k, kstep::WORDS);
                }
                Some(kstep::WORDS) if k.slip39 => {
                    // Each share is quizzed in turn, from the first.
                    if let Some(ind) = k.share_indices(0) {
                        let seed = crate::quiz::Quiz::seed(now_ms, ind);
                        k.quiz = Some(crate::quiz::Quiz::new(
                            ind,
                            crate::word_entry::EntryList::Slip39,
                            seed,
                        ));
                        k.quiz_share = 0;
                        k.shown = false;
                        k.done[usize::from(kstep::WORDS)] = true;
                        open(k, kstep::QUIZ);
                    }
                }
                Some(kstep::WORDS) => {
                    // The quiz asks every word in a random order; the
                    // words are hidden while it runs.
                    if let Some(m) = k.mnemonic.as_ref() {
                        let seed = crate::quiz::Quiz::seed(now_ms, m.indices());
                        k.quiz = Some(crate::quiz::Quiz::new(
                            m.indices(),
                            crate::word_entry::EntryList::Bip39(Language::English),
                            seed,
                        ));
                        k.shown = false;
                        k.done[usize::from(kstep::WORDS)] = true;
                        open(k, kstep::QUIZ);
                    }
                }
                _ => {}
            },
            A::KRoll(f) => k.roll(f),
            A::KFlip(h) => k.flip(h),
            A::KDie(f) => k.die_flip(f),
            A::KByDie(on) => k.by_die = on,
            A::KTyping(on) => k.typing = on,
            A::KRank(r) => k.card(Some(r), None),
            A::KSuit(s) => k.card(None, Some(s)),
            A::KHex(v) => {
                if let Some(c) = char::from_digit(u32::from(v), 16) {
                    k.hex_digit(c);
                }
            }
            A::KFrame => k.take_frame(),
            A::KUndo => k.undo(),
            A::KClear => k.clear_active(),
            A::KShow => {
                if matches!(k.open, Some(kstep::WORDS | kstep::KEY)) {
                    k.shown = !k.shown;
                }
            }
            A::KQuiz(slot) if k.slip39 => {
                let list = crate::word_entry::EntryList::Slip39;
                let at = k.quiz_share;
                let ind: Vec<u16> = k.share_indices(at).map(<[u16]>::to_vec).unwrap_or_default();
                let passed = match k.quiz.as_mut() {
                    Some(q) => {
                        q.pick(usize::from(slot));
                        q.confirm(&ind, list);
                        q.state() == crate::quiz::QuizState::Passed
                    }
                    None => false,
                };
                if passed {
                    // The next share, or done.
                    match k.share_indices(at + 1).map(<[u16]>::to_vec) {
                        Some(next) => {
                            let seed = crate::quiz::Quiz::seed(now_ms, &next);
                            k.quiz = Some(crate::quiz::Quiz::new(&next, list, seed));
                            k.quiz_share = at + 1;
                        }
                        None => k.done[usize::from(kstep::QUIZ)] = true,
                    }
                }
            }
            A::KQuizRetry if k.slip39 => {
                let ind: Vec<u16> = k
                    .share_indices(k.quiz_share)
                    .map(<[u16]>::to_vec)
                    .unwrap_or_default();
                if let Some(q) = k.quiz.as_mut() {
                    q.retry(&ind, crate::word_entry::EntryList::Slip39);
                }
            }
            A::KQuiz(slot) => {
                if let (Some(q), Some(m)) = (k.quiz.as_mut(), k.mnemonic.as_ref()) {
                    let list = crate::word_entry::EntryList::Bip39(Language::English);
                    q.pick(usize::from(slot));
                    q.confirm(m.indices(), list);
                    if q.state() == crate::quiz::QuizState::Passed {
                        k.done[usize::from(kstep::QUIZ)] = true;
                    }
                }
            }
            A::KQuizRetry => {
                if let (Some(q), Some(m)) = (k.quiz.as_mut(), k.mnemonic.as_ref()) {
                    q.retry(
                        m.indices(),
                        crate::word_entry::EntryList::Bip39(Language::English),
                    );
                }
            }
            A::KSkip => {
                if k.skip_ask {
                    k.skipped = true;
                    k.done[usize::from(kstep::QUIZ)] = true;
                } else {
                    k.skip_ask = true;
                }
            }
            A::KAgain => {
                k.clear_entries();
                open(k, kstep::ENTER);
            }
            _ => {}
        }
    }
}
