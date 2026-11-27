//! The thread the sticks' work runs on, so `carry` never waits on a stick (02 §1): asks go in
//! one at a time and in order (P19), answers and the listing of the sticks come back through
//! `poll`.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use faraday_port::event::SticksFact;
use faraday_port::storage::{StickInfo, StorageAsk};
use faraday_port::{Answer, AskId, Event, Waker};

use crate::Sticks;
use crate::serve::serve;

/// How often the sticks are looked at when no ask is waiting.
const SCAN: Duration = Duration::from_millis(700);

pub struct Worker {
    jobs: Sender<(AskId, StorageAsk)>,
    events: Receiver<Event>,
    waker: Arc<Mutex<Option<Arc<dyn Waker>>>>,
}

impl Worker {
    pub fn start(sticks: Box<dyn Sticks>) -> Worker {
        let (jobs, job_rx) = mpsc::channel();
        let (out, events) = mpsc::channel();
        let waker: Arc<Mutex<Option<Arc<dyn Waker>>>> = Arc::new(Mutex::new(None));
        let thread_waker = waker.clone();
        let spawned = std::thread::Builder::new()
            .name(String::from("sticks"))
            .spawn(move || run(sticks, &job_rx, &out, &thread_waker));
        drop(spawned);
        Worker {
            jobs,
            events,
            waker,
        }
    }

    pub fn wake_with(&mut self, waker: Arc<dyn Waker>) {
        if let Ok(mut slot) = self.waker.lock() {
            *slot = Some(waker);
        }
    }

    pub fn send(&self, id: AskId, ask: StorageAsk) {
        // A worker that has ended answers nothing; the runtime's own timeout reads that.
        let _ = self.jobs.send((id, ask));
    }

    pub fn poll(&mut self) -> Option<Event> {
        self.events.try_recv().ok()
    }
}

fn run(
    mut sticks: Box<dyn Sticks>,
    jobs: &Receiver<(AskId, StorageAsk)>,
    out: &Sender<Event>,
    waker: &Mutex<Option<Arc<dyn Waker>>>,
) {
    let say = |event: Event| -> bool {
        let sent = out.send(event).is_ok();
        if let Ok(slot) = waker.lock()
            && let Some(w) = slot.as_ref()
        {
            w.wake();
        }
        sent
    };
    let mut listed: Option<Vec<StickInfo>> = None;
    let mut problems: Option<Vec<(faraday_port::storage::StickId, bool)>> = None;
    let mut wait = Duration::ZERO;
    loop {
        match jobs.recv_timeout(wait) {
            Ok((id, ask)) => {
                let answer = serve(sticks.as_mut(), ask);
                if !say(Event::Answer(id, Answer::Storage(answer))) {
                    return;
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                let now = sticks.list();
                if listed.as_ref() != Some(&now) {
                    listed = Some(now.clone());
                    if !say(Event::Sticks(SticksFact::Listed(now))) {
                        return;
                    }
                }
                let bad = sticks.problems();
                if problems.as_ref() != Some(&bad) {
                    problems = Some(bad.clone());
                    for (stick, read_only) in bad {
                        if !say(Event::Sticks(SticksFact::Problem { stick, read_only })) {
                            return;
                        }
                    }
                }
                wait = SCAN;
            }
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}
