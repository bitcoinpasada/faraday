//! The device app's root: the command line, the stick and storage adapters, and the loop of
//! 02 §1 (09 §7).

#![expect(dead_code, reason = "the fields are read once the bodies are written")]

mod options;

use std::process::ExitCode;
use std::sync::Arc;

use faraday_device_product::Device;
use faraday_port::{Adapters, Waker};
use faraday_runtime::{Root, Runtime, Workers};
use faraday_stick_adapter::DeviceAdapters;

fn main() -> ExitCode {
    let options = options::parse();
    let (adapters, waker) = adapters(&options);
    let mut rt = Runtime::new(product(&options), waker, Workers::Threads);
    run(&mut rt, adapters)
}

/// The device product value; its `Options` are `()`'s stand-in (06 §5.1).
fn product(_options: &options::Options) -> Device {
    todo!()
}

/// The stick adapter and the storage adapter behind one value, and the loop's waker.
fn adapters(_options: &options::Options) -> (DeviceAdapters, Arc<dyn Waker>) {
    todo!()
}

/// The loop, shared in form by each root and by a journey (06 §8).
fn run<R: Root, A: Adapters>(_rt: &mut Runtime<R>, _adapters: A) -> ExitCode {
    todo!()
}
