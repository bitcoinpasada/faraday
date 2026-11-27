//! The channel the main loop waits on: a fixed ring of slots, guarded by a
//! mutex, in which a typed character is written over as soon as it is
//! read. std's channel frees and reuses its blocks unwiped, which would
//! leave a passphrase's characters behind in them.
//!
//! It blocks and wakes as std's bounded channel does: [`WakeTx::send`]
//! waits while the ring is full, [`WakeTx::try_send`] does not, and the
//! receiver reads what is queued before it hears that every sender is gone.

use std::sync::mpsc::{RecvTimeoutError, TryRecvError};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use faraday_crypto::Zeroize;
use faraday_port::Waker;
use faraday_port::event::Key;

use crate::{HeldInput, Wake};

/// The receiver is gone, or the ring is full (for `try_send`): the wake
/// was not queued.
#[derive(Debug, PartialEq, Eq)]
pub struct Rejected;

/// One place in the ring. A typed character is kept apart from the wake
/// it belongs to, in a plain integer, so that writing the integer over
/// leaves no copy of it in the slot.
struct Slot {
    /// The wake, with any typed character replaced by a placeholder.
    wake: Wake,
    /// The typed character, when the wake carries one.
    typed: u32,
}

impl Slot {
    const EMPTY: Slot = Slot {
        wake: Wake::Work,
        typed: 0,
    };

    fn of(wake: Wake) -> Slot {
        fn hide(key: Key, typed: &mut u32) -> Key {
            match key {
                Key::Char(c) => {
                    *typed = c as u32;
                    Key::Char('\0')
                }
                other => other,
            }
        }
        let mut typed = 0;
        let wake = match wake {
            Wake::Key(k) => Wake::Key(hide(k, &mut typed)),
            Wake::KeyUp(k) => Wake::KeyUp(hide(k, &mut typed)),
            Wake::Held(id, HeldInput::Key(k)) => {
                Wake::Held(id, HeldInput::Key(hide(k, &mut typed)))
            }
            Wake::Held(id, HeldInput::KeyUp(k)) => {
                Wake::Held(id, HeldInput::KeyUp(hide(k, &mut typed)))
            }
            other => other,
        };
        Slot { wake, typed }
    }

    /// The wake back as it was sent, the slot written over.
    fn take(&mut self) -> Wake {
        fn show(key: Key, typed: u32) -> Key {
            match key {
                Key::Char(_) => Key::Char(char::from_u32(typed).unwrap_or('\0')),
                other => other,
            }
        }
        let typed = self.typed;
        let wake = match self.wake {
            Wake::Key(k) => Wake::Key(show(k, typed)),
            Wake::KeyUp(k) => Wake::KeyUp(show(k, typed)),
            Wake::Held(id, HeldInput::Key(k)) => Wake::Held(id, HeldInput::Key(show(k, typed))),
            Wake::Held(id, HeldInput::KeyUp(k)) => Wake::Held(id, HeldInput::KeyUp(show(k, typed))),
            other => other,
        };
        self.typed.zeroize();
        self.wake = Wake::Work;
        wake
    }
}

struct State {
    /// The ring: `len` slots from `head`, the rest empty.
    slots: Box<[Slot]>,
    head: usize,
    len: usize,
    senders: usize,
    receiver_gone: bool,
}

impl State {
    fn push(&mut self, wake: Wake) {
        let at = (self.head + self.len) % self.slots.len();
        self.slots[at] = Slot::of(wake);
        self.len += 1;
    }

    fn pop(&mut self) -> Option<Wake> {
        if self.len == 0 {
            return None;
        }
        let wake = self.slots[self.head].take();
        self.head = (self.head + 1) % self.slots.len();
        self.len -= 1;
        Some(wake)
    }
}

impl Drop for State {
    fn drop(&mut self) {
        for slot in self.slots.iter_mut() {
            slot.typed.zeroize();
        }
    }
}

struct Shared {
    state: Mutex<State>,
    /// Signalled when a wake is queued or the last sender goes.
    readable: Condvar,
    /// Signalled when a slot is freed or the receiver goes.
    writable: Condvar,
    /// The root's loop, woken beside `readable` for each wake queued.
    waker: Option<Arc<dyn Waker>>,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn wake_root(&self) {
        if let Some(waker) = &self.waker {
            waker.wake();
        }
    }
}

/// The sending end; clones share the ring.
pub struct WakeTx(Arc<Shared>);

/// The receiving end.
pub struct WakeRx(Arc<Shared>);

/// A ring of `cap` slots (at least one); `waker` wakes the root's loop for each wake queued.
pub fn channel(cap: usize, waker: Option<Arc<dyn Waker>>) -> (WakeTx, WakeRx) {
    let slots = (0..cap.max(1)).map(|_| Slot::EMPTY).collect();
    let shared = Arc::new(Shared {
        state: Mutex::new(State {
            slots,
            head: 0,
            len: 0,
            senders: 1,
            receiver_gone: false,
        }),
        readable: Condvar::new(),
        writable: Condvar::new(),
        waker,
    });
    (WakeTx(shared.clone()), WakeRx(shared))
}

impl Clone for WakeTx {
    fn clone(&self) -> WakeTx {
        self.0.lock().senders += 1;
        WakeTx(self.0.clone())
    }
}

impl Drop for WakeTx {
    fn drop(&mut self) {
        let mut st = self.0.lock();
        st.senders -= 1;
        if st.senders == 0 {
            self.0.readable.notify_all();
        }
    }
}

impl Drop for WakeRx {
    fn drop(&mut self) {
        let mut st = self.0.lock();
        st.receiver_gone = true;
        // Nothing will read what is queued: write it over.
        while st.pop().is_some() {}
        self.0.writable.notify_all();
    }
}

impl WakeTx {
    /// Queues a wake, waiting while the ring is full.
    pub fn send(&self, wake: Wake) -> Result<(), Rejected> {
        let mut st = self.0.lock();
        loop {
            if st.receiver_gone {
                return Err(Rejected);
            }
            if st.len < st.slots.len() {
                st.push(wake);
                self.0.readable.notify_one();
                drop(st);
                self.0.wake_root();
                return Ok(());
            }
            st = self.0.writable.wait(st).unwrap_or_else(|e| e.into_inner());
        }
    }

    /// Queues a wake if there is room, without waiting.
    pub fn try_send(&self, wake: Wake) -> Result<(), Rejected> {
        let mut st = self.0.lock();
        if st.receiver_gone || st.len == st.slots.len() {
            return Err(Rejected);
        }
        st.push(wake);
        self.0.readable.notify_one();
        drop(st);
        self.0.wake_root();
        Ok(())
    }
}

impl WakeRx {
    /// The oldest wake, waiting up to `timeout` for one.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<Wake, RecvTimeoutError> {
        let deadline = Instant::now().checked_add(timeout);
        let mut st = self.0.lock();
        loop {
            if let Some(wake) = st.pop() {
                self.0.writable.notify_one();
                return Ok(wake);
            }
            if st.senders == 0 {
                return Err(RecvTimeoutError::Disconnected);
            }
            let left = match deadline {
                Some(d) => d.saturating_duration_since(Instant::now()),
                None => Duration::from_secs(3600),
            };
            if left.is_zero() {
                return Err(RecvTimeoutError::Timeout);
            }
            st = self
                .0
                .readable
                .wait_timeout(st, left)
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
    }

    /// The oldest wake, if one is queued.
    pub fn try_recv(&self) -> Result<Wake, TryRecvError> {
        let mut st = self.0.lock();
        match st.pop() {
            Some(wake) => {
                self.0.writable.notify_one();
                Ok(wake)
            }
            None if st.senders == 0 => Err(TryRecvError::Disconnected),
            None => Err(TryRecvError::Empty),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_order_and_the_characters_typed() {
        let (tx, rx) = channel(5, None);
        for c in "pass".chars() {
            tx.send(Wake::Key(Key::Char(c))).unwrap();
        }
        assert!(tx.try_send(Wake::Frame).is_ok());
        assert_eq!(tx.try_send(Wake::Frame), Err(Rejected));
        let got: Vec<Wake> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
        assert_eq!(
            got,
            vec![
                Wake::Key(Key::Char('p')),
                Wake::Key(Key::Char('a')),
                Wake::Key(Key::Char('s')),
                Wake::Key(Key::Char('s')),
                Wake::Frame,
            ]
        );
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
    }

    #[test]
    fn a_character_read_is_not_left_in_the_ring() {
        let (tx, rx) = channel(2, None);
        tx.send(Wake::Held(3, HeldInput::Key(Key::Char('z'))))
            .unwrap();
        tx.send(Wake::KeyUp(Key::Char('q'))).unwrap();
        assert_eq!(
            rx.try_recv(),
            Ok(Wake::Held(3, HeldInput::Key(Key::Char('z'))))
        );
        assert_eq!(rx.try_recv(), Ok(Wake::KeyUp(Key::Char('q'))));
        let st = tx.0.lock();
        assert!(
            st.slots
                .iter()
                .all(|s| s.typed == 0 && s.wake == Wake::Work)
        );
    }

    #[test]
    fn a_full_ring_waits_for_a_read_and_ends_with_the_senders() {
        let (tx, rx) = channel(1, None);
        tx.send(Wake::Frame).unwrap();
        let t2 = tx.clone();
        let h = std::thread::spawn(move || t2.send(Wake::Work));
        assert_eq!(rx.recv_timeout(Duration::from_secs(5)), Ok(Wake::Frame));
        assert_eq!(rx.recv_timeout(Duration::from_secs(5)), Ok(Wake::Work));
        h.join().unwrap().unwrap();
        assert_eq!(
            rx.recv_timeout(Duration::from_millis(10)),
            Err(RecvTimeoutError::Timeout)
        );
        drop(tx);
        assert_eq!(
            rx.recv_timeout(Duration::from_millis(10)),
            Err(RecvTimeoutError::Disconnected)
        );
    }

    #[test]
    fn sending_to_a_gone_receiver_fails() {
        let (tx, rx) = channel(1, None);
        drop(rx);
        assert_eq!(tx.send(Wake::Frame), Err(Rejected));
    }
}
