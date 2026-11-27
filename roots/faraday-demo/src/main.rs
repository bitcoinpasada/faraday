//! Faraday Demo's root: the command line, the window, desktop and storage adapters, and the loop
//! of 02 §1 (09 §7).

#![expect(dead_code, reason = "the fields are read once the bodies are written")]

mod options;

use std::process::ExitCode;
use std::sync::Arc;

use faraday_demo_product::Demo;
use faraday_desktop_adapter::Desktop;
use faraday_port::{Adapters, Effect, Event, Waker};
use faraday_runtime::{Root, Runtime, Workers};
use faraday_storage::Storage;
use faraday_window::Window;

fn main() -> ExitCode {
    let options = options::parse();
    let (adapters, waker) = adapters(&options);
    let mut rt = Runtime::new(product(&options), waker, Workers::Threads);
    run(&mut rt, adapters)
}

/// The Demo product value, with its `DemoOptions` from the command line.
fn product(_options: &options::Options) -> Demo {
    todo!()
}

fn adapters(_options: &options::Options) -> (DemoAdapters, Arc<dyn Waker>) {
    todo!()
}

/// The Demo's adapters behind one value: each effect to the adapter of its family.
struct DemoAdapters {
    window: Window,
    desktop: Desktop,
    storage: Storage,
}

impl Adapters for DemoAdapters {
    fn carry(&mut self, _effect: Effect) {
        todo!()
    }

    fn poll(&mut self) -> Option<Event> {
        todo!()
    }
}

/// The loop, shared in form by each root and by a journey (06 §8).
fn run<R: Root, A: Adapters>(_rt: &mut Runtime<R>, _adapters: A) -> ExitCode {
    todo!()
}
