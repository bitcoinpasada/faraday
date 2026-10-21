//! A global allocator that counts the allocations each thread makes, for
//! faraday-core's frame budgets (`docs/SPEED.md` §7): a test binary
//! declares it as its allocator, and a test reads [`allocations`] before
//! and after a frame. Every call goes to the system allocator; the count
//! is a thread-local add, so tests running side by side in one process do
//! not see each other's.
//!
//! ```
//! #[global_allocator]
//! static ALLOC: faraday_alloc_count::Counting = faraday_alloc_count::Counting;
//!
//! let before = faraday_alloc_count::allocations();
//! let v = vec![1u8; 64];
//! assert!(faraday_alloc_count::allocations() > before);
//! drop(v);
//! ```

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static COUNT: Cell<u64> = const { Cell::new(0) };
}

/// The allocator: the system's, counting each allocation and
/// reallocation on the thread that asks.
pub struct Counting;

fn note() {
    // `try_with`: a thread being torn down still allocates, after its
    // count is gone.
    let _ = COUNT.try_with(|c| c.set(c.get().wrapping_add(1)));
}

#[allow(unsafe_code)]
// SAFETY: every call is passed to `System` unchanged; the count touches
// no memory the caller owns and allocates nothing itself.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note();
        // SAFETY: the caller's contract for `alloc` is `System`'s.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        note();
        // SAFETY: as `alloc`.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from `System` through this allocator.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        note();
        // SAFETY: `ptr` came from `System` through this allocator.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

/// The allocations and reallocations this thread has made so far.
pub fn allocations() -> u64 {
    COUNT.try_with(Cell::get).unwrap_or(0)
}
