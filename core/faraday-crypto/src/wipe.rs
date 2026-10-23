//! A global allocator wrapper that zeroes every block before it is
//! freed.
//!
//! `zeroize` wipes what Faraday's own types hold. It cannot reach the
//! copies other code makes on the heap and frees: a `Vec` that grew and
//! left its old buffer behind, the buffers a QR decoder fills while it
//! reads a SeedQR from a camera frame, the frame itself, a parser's
//! temporaries. Freed, those bytes stay in the process's heap until the
//! allocator hands the block out again. [`WipeOnFree`] wraps the
//! allocator a binary installs and writes zeros over every block as it
//! is freed, and over the old block when one is moved to grow or
//! shrink.
//!
//! **Opt-in, by a binary.** A library never installs a global
//! allocator; a shell's `main.rs` does, once:
//!
//! ```ignore
//! #[global_allocator]
//! static ALLOC: faraday_crypto::WipeOnFree<std::alloc::System> =
//!     faraday_crypto::WipeOnFree::new(std::alloc::System);
//! ```
//!
//! **What it costs.** A write of every freed byte, and a reallocation
//! is always a move: a block is never grown in place, because growing in
//! place cannot wipe what the allocator does with the old one. Frees are
//! as frequent as allocations, so a binary that installs it is timed in
//! a late-flow session first (`docs/SPEED.md` §7).
//!
//! **What it does not do.** It wipes what was asked for, `layout.size()`
//! bytes, which is all Rust code can have written; the stack, registers
//! and copies inside C libraries' own allocators are out of its reach,
//! as before. It changes nothing about what a block holds while it is
//! live.
//!
//! The writes are volatile, a word at a time where the block is
//! aligned to one, so that the compiler cannot drop them as stores to
//! memory about to be freed.

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{Ordering, compiler_fence};

/// A [`GlobalAlloc`] that zeroes each block before the allocator it
/// wraps frees it.
pub struct WipeOnFree<A> {
    inner: A,
}

impl<A> WipeOnFree<A> {
    /// Wraps `inner`, the allocator that does the allocating.
    pub const fn new(inner: A) -> Self {
        WipeOnFree { inner }
    }
}

/// Writes zeros over `len` bytes at `ptr`, volatile, a `u64` at a time
/// over the aligned middle and a byte at a time at either end.
///
/// # Safety
///
/// `ptr` must be valid for writes of `len` bytes.
#[allow(unsafe_code)]
unsafe fn wipe(ptr: *mut u8, len: usize) {
    let head = ptr.align_offset(align_of::<u64>()).min(len);
    let words = (len - head) / size_of::<u64>();
    let tail = head + words * size_of::<u64>();
    // SAFETY: every write below is at an offset under `len`, which the
    // caller promises is writable; the word writes start at `ptr + head`,
    // which `align_offset` made aligned to `u64`, and end at `tail`,
    // within `len`.
    unsafe {
        for i in 0..head {
            ptr.add(i).write_volatile(0);
        }
        let body = ptr.add(head).cast::<u64>();
        for i in 0..words {
            body.add(i).write_volatile(0);
        }
        for i in tail..len {
            ptr.add(i).write_volatile(0);
        }
    }
    compiler_fence(Ordering::SeqCst);
}

// SAFETY: every method hands the caller's layout to the inner
// allocator unchanged, so what it returns meets `GlobalAlloc`'s
// contract as the inner allocator's does. `dealloc` only writes the
// `layout.size()` bytes the caller owns until the call returns, then
// frees them as the inner allocator's. `realloc` allocates the new
// block under the same alignment, copies the smaller of the two sizes,
// and frees the old block through `dealloc`, which leaves the old block
// untouched when the new allocation fails, as the contract asks.
#[allow(unsafe_code)]
unsafe impl<A: GlobalAlloc> GlobalAlloc for WipeOnFree<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's contract, passed on.
        unsafe { self.inner.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's contract, passed on.
        unsafe { self.inner.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller allocated `ptr` with `layout`, so its
        // `layout.size()` bytes are writable until they are freed here.
        unsafe {
            wipe(ptr, layout.size());
            self.inner.dealloc(ptr, layout);
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller promises `new_size`, rounded up to
        // `layout.align()`, does not overflow `isize`, which is what
        // `from_size_align_unchecked` needs, and that it is not zero.
        let new_layout = unsafe { Layout::from_size_align_unchecked(new_size, layout.align()) };
        // SAFETY: a non-zero size, as above.
        let new = unsafe { self.inner.alloc(new_layout) };
        if !new.is_null() {
            // SAFETY: `ptr` holds `layout.size()` readable bytes and
            // `new` `new_size` writable ones, in two different blocks;
            // the old block is then freed as the caller allocated it.
            unsafe {
                core::ptr::copy_nonoverlapping(ptr, new, layout.size().min(new_size));
                self.dealloc(ptr, layout);
            }
        }
        new
    }
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    extern crate std;

    use super::*;
    use core::sync::atomic::AtomicUsize;
    use std::alloc::System;

    /// The system allocator, counting the blocks it is asked to free and
    /// how many of them still held a byte that was not zero.
    struct Check {
        freed: AtomicUsize,
        dirty: AtomicUsize,
    }

    // SAFETY: `System`'s, with a read of the block before it is freed.
    unsafe impl GlobalAlloc for Check {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            unsafe { System.alloc(layout) }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            // SAFETY: the block is live and `layout.size()` long.
            let bytes = unsafe { core::slice::from_raw_parts(ptr, layout.size()) };
            self.freed.fetch_add(1, Ordering::SeqCst);
            if bytes.iter().any(|b| *b != 0) {
                self.dirty.fetch_add(1, Ordering::SeqCst);
            }
            unsafe { System.dealloc(ptr, layout) }
        }
    }

    fn checked() -> WipeOnFree<Check> {
        WipeOnFree::new(Check {
            freed: AtomicUsize::new(0),
            dirty: AtomicUsize::new(0),
        })
    }

    fn filled(a: &WipeOnFree<Check>, layout: Layout, byte: u8) -> *mut u8 {
        let ptr = unsafe { a.alloc(layout) };
        assert!(!ptr.is_null());
        unsafe { ptr.write_bytes(byte, layout.size()) };
        ptr
    }

    #[test]
    fn a_freed_block_holds_only_zeros() {
        let a = checked();
        for (size, align) in [(1, 1), (7, 1), (64, 8), (1000, 16), (4099, 4096)] {
            let layout = Layout::from_size_align(size, align).unwrap();
            let ptr = filled(&a, layout, 0xa5);
            unsafe { a.dealloc(ptr, layout) };
        }
        assert_eq!(a.inner.freed.load(Ordering::SeqCst), 5);
        assert_eq!(a.inner.dirty.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_moved_block_keeps_its_bytes_and_leaves_zeros_behind() {
        let a = checked();
        let layout = Layout::from_size_align(64, 8).unwrap();
        for new_size in [4096, 10] {
            let ptr = filled(&a, layout, 0x5a);
            let moved = unsafe { a.realloc(ptr, layout, new_size) };
            assert!(!moved.is_null());
            let kept = unsafe { core::slice::from_raw_parts(moved, new_size.min(64)) };
            assert!(kept.iter().all(|b| *b == 0x5a));
            let new_layout = Layout::from_size_align(new_size, 8).unwrap();
            unsafe { moved.write_bytes(0x77, new_size) };
            unsafe { a.dealloc(moved, new_layout) };
        }
        assert_eq!(a.inner.freed.load(Ordering::SeqCst), 4);
        assert_eq!(a.inner.dirty.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_wipe_stays_inside_its_bytes() {
        // Unaligned at both ends: the byte head, the word body and the
        // byte tail are all written, and nothing outside them.
        let mut buf = [0xffu8; 64];
        unsafe { wipe(buf.as_mut_ptr().add(3), 45) };
        assert!(buf[..3].iter().all(|b| *b == 0xff));
        assert!(buf[3..48].iter().all(|b| *b == 0));
        assert!(buf[48..].iter().all(|b| *b == 0xff));
    }
}
