//! The device app's root: the command line, the stick and storage adapters, and the loop of
//! 02 §1 (09 §7).

mod options;

use std::process::ExitCode;
use std::sync::{Arc, Condvar, Mutex, PoisonError};

use faraday_device_product::Device;
use faraday_port::{Adapters, Effect, Text, Waker};
use faraday_runtime::{Phase, Root, Runtime, Workers};
use faraday_stick_adapter::{DeviceAdapters, Stick};
use faraday_storage::Storage;

/// Wakes the loop: the runtime's workers and every adapter thread hold it.
struct Gate {
    woken: Mutex<bool>,
    cv: Condvar,
}

impl Gate {
    fn new() -> Self {
        Gate {
            woken: Mutex::new(false),
            cv: Condvar::new(),
        }
    }

    /// Waits until something woke the loop, then clears the wake.
    fn wait(&self) {
        let mut woken = self.woken.lock().unwrap_or_else(PoisonError::into_inner);
        while !*woken {
            woken = self.cv.wait(woken).unwrap_or_else(PoisonError::into_inner);
        }
        *woken = false;
    }
}

impl Waker for Gate {
    fn wake(&self) {
        *self.woken.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.cv.notify_one();
    }
}

fn main() -> ExitCode {
    let options = options::parse();
    let gate = Arc::new(Gate::new());
    let waker: Arc<dyn Waker> = Arc::clone(&gate) as Arc<dyn Waker>;
    let adapters = match adapters(options, &waker) {
        Ok(adapters) => adapters,
        Err(why) => {
            eprintln!("faraday: {why}");
            return ExitCode::FAILURE;
        }
    };
    let mut rt = Runtime::new(product(), waker, Workers::Threads);
    run(&mut rt, adapters, &gate)
}

/// The device product value: it takes no options (06 §2).
fn product() -> Device {
    Device
}

/// The stick adapter and the storage adapter behind one value, both woken by the loop's waker.
fn adapters(options: options::Options, waker: &Arc<dyn Waker>) -> Result<DeviceAdapters, String> {
    let stick = Stick::open(options.stick, Arc::clone(waker))
        .map_err(|failure| format!("the stick adapter did not open: {failure:?}"))?;
    let storage = Storage::disk_process(Text::of(&options.fifo), Text::of(&options.ram_dir))
        .map_err(|failure| format!("the storage adapter did not open: {failure:?}"))?;
    Ok(DeviceAdapters::new(stick, storage, Arc::clone(waker)))
}

/// What one handling of the runtime gave: its effects to the adapters in the order made (P18),
/// the frame to the panel, and whether the process is over.
fn settle<R: Root>(
    rt: &mut Runtime<R>,
    adapters: &mut DeviceAdapters,
    effects: Vec<Effect>,
) -> Option<u8> {
    for effect in effects {
        adapters.carry(effect);
    }
    if let Some(frame) = rt.frame() {
        adapters.stick().present(&frame);
    }
    adapters.stick().ended()
}

/// The loop, shared in form by each root and by a journey (06 §8): events from the adapters, the
/// runtime's answer to each, the frame to the panel, and the process's end with the status the
/// adapter says. A lock ends it with the status the stick's startup reads (§2.1).
fn run<R: Root>(rt: &mut Runtime<R>, mut adapters: DeviceAdapters, gate: &Gate) -> ExitCode {
    loop {
        while let Some(event) = adapters.poll() {
            let effects = rt.event(event);
            if let Some(status) = settle(rt, &mut adapters, effects) {
                return ExitCode::from(status);
            }
        }
        let effects = rt.woken();
        if let Some(status) = settle(rt, &mut adapters, effects) {
            return ExitCode::from(status);
        }
        if rt.phase() == Phase::Ended {
            return ExitCode::SUCCESS;
        }
        gate.wait();
    }
}
