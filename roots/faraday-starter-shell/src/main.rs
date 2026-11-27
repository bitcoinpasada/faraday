//! The Starter's root: the command line, the window, desktop, storage and operating-system
//! adapters, and the loop of 02 §1 (09 §7).

#![expect(dead_code, reason = "the fields are read once the bodies are written")]

mod options;

use std::process::ExitCode;
use std::sync::Arc;

use faraday_desktop_adapter::Desktop;
use faraday_os_adapter::System;
use faraday_port::{Adapters, Effect, Event, Waker};
use faraday_runtime::{Root, Runtime, Workers};
use faraday_starter_product::Starter;
use faraday_storage::Storage;
use faraday_window::Window;

fn main() -> ExitCode {
    let options = options::parse();
    let (adapters, waker) = adapters(&options);
    let mut rt = Runtime::new(product(&options), waker, Workers::Threads);
    run(&mut rt, adapters)
}

/// The Starter product value, with its `StarterOptions` from the command line.
fn product(_options: &options::Options) -> Starter {
    todo!()
}

fn adapters(_options: &options::Options) -> (StarterAdapters, Arc<dyn Waker>) {
    todo!()
}

/// The Starter's adapters behind one value: each effect to the adapter of its family.
struct StarterAdapters {
    window: Window,
    desktop: Desktop,
    storage: Storage,
    system: System,
}

impl Adapters for StarterAdapters {
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
