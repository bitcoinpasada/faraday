//! The Argon QR Vault's screens (`docs/AQRV.md`): a
//! `faraday_shell_api::App` with two tabs, Make and Open. Make is four
//! screens (the item, the settings, working, the code); Open is empty
//! until its own change. Argon2id and QR encoding run on one worker
//! thread (`worker.rs`); drawing takes `&State` and reads what it sent
//! back.

mod draw;
pub mod samples;
pub mod state;
pub mod worker;

use std::collections::VecDeque;

use faraday_draw::{Canvas, Rect};
use faraday_shell_api::{App, Command, DisplayInfo, Event, Frame, Key, TouchPhase};
use zeroize::Zeroizing;

pub use state::{Action, Field, OwnKind, Page, Source, State, Tab};
use worker::{Answer, Job, Worker};

/// How many 32-byte answers of the entropy channel one code draws from.
const ENTROPY_REQUESTS: usize = 8;

/// The app.
pub struct ArgonQrVault {
    state: State,
    worker: Worker,
    commands: VecDeque<Command>,
    canvas: Option<Canvas>,
    hits: Vec<(Rect, Action)>,
    pressed: Option<Action>,
    f: f32,
    size: (f32, f32),
    max: f32,
    phase: Option<f32>,
    dirty: bool,
    /// The request waiting for the shell's entropy, with what came.
    waiting: Option<u64>,
    pool: Zeroizing<Vec<u8>>,
}

impl ArgonQrVault {
    /// A new app. `wake` is called from the worker thread when it has an
    /// answer, so the shell sends a tick.
    pub fn new(wake: impl Fn() + Send + 'static) -> ArgonQrVault {
        let worker = Worker::start(wake);
        worker.ask(Job::Time);
        ArgonQrVault {
            state: State::new(),
            worker,
            commands: VecDeque::new(),
            canvas: None,
            hits: Vec::new(),
            pressed: None,
            f: 1.0,
            size: (0.0, 0.0),
            max: 0.0,
            phase: None,
            dirty: false,
            waiting: None,
            pool: Zeroizing::new(Vec::new()),
        }
    }

    /// What the screens show.
    pub fn state(&self) -> &State {
        &self.state
    }

    /// What a press on `action` does: the same as a touch on its control.
    pub fn tap(&mut self, action: Action) {
        self.act(action);
        self.changed();
    }

    /// Waits for the worker to answer what it was asked, as a shell's
    /// ticks do one answer at a time. Returns at once while the shell has
    /// not yet answered the entropy a code is waiting for.
    pub fn finish_work(&mut self) {
        while self.state.job.is_some() && self.waiting.is_none() || self.state.timing.is_none() {
            match self.worker.wait() {
                Some(answer) => self.take(answer),
                None => break,
            }
        }
        self.changed();
    }

    fn changed(&mut self) {
        self.dirty = true;
        self.commands.push_back(Command::Draw);
    }

    fn take(&mut self, answer: Answer) {
        match answer {
            Answer::Timed(d) => self.state.timing = Some(d.as_secs_f32()),
            Answer::Made(id, result) => {
                // A cancelled request's answer is discarded.
                if self.state.job != Some(id) {
                    return;
                }
                self.state.job = None;
                match *result {
                    Ok(made) => {
                        self.state.made = Some(made);
                        self.state.page = Page::Code;
                        self.state.offset = 0.0;
                    }
                    Err(why) => {
                        self.state.failed = Some(why);
                        self.state.page = Page::Settings;
                    }
                }
            }
        }
    }

    fn act(&mut self, action: Action) {
        let st = &mut self.state;
        match action {
            Action::Tab(t) => {
                st.tab = t;
                st.offset = 0.0;
            }
            Action::Sample(i) => {
                st.source = Source::Sample(i);
                st.focus = None;
                st.keep_size();
            }
            Action::Own => {
                st.source = Source::Own;
                st.focus = Some(Field::Own);
                st.keep_size();
            }
            Action::Kind(k) => {
                st.own_kind = k;
                st.focus = Some(Field::Own);
                st.keep_size();
            }
            Action::Size(s) => {
                if st.fits(s) {
                    st.size = s;
                }
            }
            Action::Step(s) => st.step = s,
            Action::Field(f) => st.focus = Some(f),
            Action::Eye => st.shown = !st.shown,
            Action::Next => {
                if st.page == Page::Item && st.item().is_some() {
                    st.keep_size();
                    st.page = Page::Settings;
                    st.focus = Some(Field::Pass);
                    st.offset = 0.0;
                }
            }
            Action::Back => match st.page {
                Page::Settings => st.page = Page::Item,
                Page::Code => st.page = Page::Settings,
                Page::Item | Page::Working => {}
            },
            Action::Make => {
                if st.page == Page::Settings && st.passphrase_ready() && st.fits(st.size) {
                    st.failed = None;
                    st.made = None;
                    st.page = Page::Working;
                    st.offset = 0.0;
                    st.focus = None;
                    st.next_job += 1;
                    let id = st.next_job;
                    st.job = Some(id);
                    self.waiting = Some(id);
                    self.pool = Zeroizing::new(Vec::new());
                    for _ in 0..ENTROPY_REQUESTS {
                        self.commands.push_back(Command::RequestEntropy);
                    }
                }
            }
            Action::Cancel => {
                // The run finishes on its thread; its answer is dropped.
                st.job = None;
                st.page = Page::Settings;
                self.waiting = None;
                self.pool = Zeroizing::new(Vec::new());
            }
            Action::Again => {
                st.made = None;
                st.pass = Zeroizing::new(String::new());
                st.again = Zeroizing::new(String::new());
                st.page = Page::Item;
                st.offset = 0.0;
            }
        }
    }

    /// Sends the job once the shell's entropy has all come.
    fn launch(&mut self) {
        let Some(id) = self.waiting else { return };
        if self.pool.len() < 32 * ENTROPY_REQUESTS {
            return;
        }
        self.waiting = None;
        let Some(item) = self.state.item() else {
            self.state.job = None;
            self.state.page = Page::Settings;
            return;
        };
        self.worker.ask(Job::Make {
            id,
            item,
            passphrase: self.state.pass.clone(),
            step: self.state.step,
            size: self.state.size,
            random: std::mem::take(&mut self.pool),
        });
    }

    fn display(&mut self, d: DisplayInfo) {
        let f = (f32::from(d.dpi.max(1)) / 160.0).max(0.5);
        let scaled = DisplayInfo {
            dpi: (160.0 * f).round().max(1.0) as u16,
            ..d
        };
        self.f = f;
        self.size = (f32::from(d.width) / f, f32::from(d.height) / f);
        self.canvas = Some(Canvas::new(&scaled));
    }

    fn hit_at(&self, x: i32, y: i32) -> Option<Action> {
        let bar = (faraday_draw::tokens::TOUCH * self.f) as i32;
        self.hits
            .iter()
            .rev()
            .find(|(r, a)| {
                let on_bar = matches!(a, Action::Tab(_));
                x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h && (on_bar || y >= bar)
            })
            .map(|(_, a)| *a)
    }

    fn key(&mut self, key: Key) {
        match key {
            Key::Char(c) if !c.is_control() => {
                self.state.type_char(c);
                self.state.keep_size();
            }
            Key::Backspace => {
                self.state.backspace();
                self.state.keep_size();
            }
            Key::Tab => {
                self.state.focus = match (self.state.page, self.state.focus) {
                    (Page::Settings, Some(Field::Pass)) => Some(Field::Again),
                    (Page::Settings, _) => Some(Field::Pass),
                    _ => self.state.focus,
                };
            }
            Key::Enter => match self.state.page {
                Page::Item => self.act(Action::Next),
                Page::Settings => self.act(Action::Make),
                _ => {}
            },
            Key::Escape => self.act(Action::Back),
            _ => return,
        }
        self.changed();
    }

    fn render(&mut self) {
        let Some(mut canvas) = self.canvas.take() else {
            return;
        };
        let mut hits = std::mem::take(&mut self.hits);
        hits.clear();
        let mut ui = faraday_ui::Ui::new(&mut canvas, self.f, &mut hits, self.pressed);
        self.max = draw::window(&mut ui, &self.state, self.size, self.phase);
        self.hits = hits;
        self.canvas = Some(canvas);
    }
}

impl App for ArgonQrVault {
    fn event(&mut self, event: Event) {
        match event {
            Event::Display(d) => self.display(d),
            Event::Tick { now_ms } => {
                let mut any = false;
                while let Some(answer) = self.worker.poll() {
                    self.take(answer);
                    any = true;
                }
                let moving = self.state.page == Page::Working && self.state.tab == Tab::Make;
                if moving {
                    self.phase = Some((now_ms % 1500) as f32 / 1500.0);
                }
                if !any && !moving {
                    return;
                }
            }
            Event::Touch { x, y, phase } => {
                let at = self.hit_at(i32::from(x), i32::from(y));
                match phase {
                    TouchPhase::Down => self.pressed = at,
                    TouchPhase::Move => return,
                    TouchPhase::Up => {
                        let was = self.pressed.take();
                        if let (Some(a), Some(b)) = (was, at)
                            && a == b
                        {
                            self.act(a);
                        }
                    }
                }
            }
            Event::Wheel { dy, .. } | Event::Scroll { dy, .. } => {
                self.state.offset =
                    (self.state.offset - f32::from(dy) / self.f).clamp(0.0, self.max);
            }
            Event::Back => self.act(Action::Back),
            Event::Key(k) => return self.key(k),
            Event::Entropy(e) => {
                if self.waiting.is_some() {
                    self.pool.extend_from_slice(&e.into_bytes());
                    self.launch();
                }
            }
            _ => return,
        }
        self.changed();
    }

    fn poll_command(&mut self) -> Option<Command> {
        self.commands.pop_front()
    }

    fn frame(&mut self) -> Frame<'_> {
        if self.dirty {
            self.dirty = false;
            self.render();
        }
        match self.canvas.as_ref() {
            Some(c) => c.frame(),
            None => Frame {
                width: 0,
                height: 0,
                rgba: &[],
            },
        }
    }
}
