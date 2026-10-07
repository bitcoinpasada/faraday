//! Secret storage the kernel is asked not to write to swap
//! (`docs/PLANNING.md` §5.4, §5.5, §16.49).
//!
//! A page that holds a seed can be written to swap in the clear, where it
//! survives a power-off. [`Pinned<T>`] puts `T` on pages of its own and
//! asks the kernel to keep them in RAM: `mlock` when the value is
//! created, `munlock` after it is zeroized on drop.
//!
//! **Whole pages, and nothing else on them.** `mlock` and `munlock` take
//! addresses but act on the pages that contain them, and they do not
//! nest: if two secrets shared a page, dropping either would unpin the
//! page the other still lives on. So the allocation is page-aligned and
//! its size is `size_of::<T>()` rounded up to a whole number of pages,
//! which leaves nothing else on them. The alignment is the reason the
//! type exists rather than a detail of it.
//!
//! **An allocation, not a field.** The value lives in an allocation this
//! module makes and frees itself, and `Pinned<T>` is the pointer to it.
//! Not a `Box`: `Box::from_raw` promises the pointer came from the global
//! allocator under `Layout::new::<T>()`, and this one deliberately did
//! not — its size is rounded up to whole pages and its alignment raised
//! to one. Rounding up is the point of the type, so the pointer is held
//! raw and freed under the layout it was made with.
//! That is what makes the address stable: the pages are pinned once, at
//! construction, and released once, on drop, with no re-checking and
//! nothing to keep in step. Storing the pages inside the value would
//! undo all of it — moving a `Pinned` would copy the whole page and
//! leave the plaintext behind at the old address, which is the exposure
//! `mlock` exists to prevent, and every type containing one would
//! inherit the page's size and alignment. Moving this one copies a
//! pointer.
//!
//! **Nothing is left where the value came from.** Construction copies
//! the bytes onto the page and then zeroizes the source, so a value
//! handed to [`Pinned::new`] does not survive in the caller's frame as a
//! moved-from copy. [`Pinned::new_with`] goes further and makes no copy
//! at all: the page starts zeroed and the caller fills it in place,
//! which is how a key assembled from entropy should be built.
//!
//! **The syscalls are a feature.** Without `pin-pages` — the browser and
//! Android builds, which must not link `libc` — the type behaves exactly
//! as it does with it, minus the two calls: same allocation, same
//! alignment, same zeroization. There is one code path either way.
//!
//! **Failure is carried on from.** A machine whose `RLIMIT_MEMLOCK` is
//! too small to satisfy still runs; its pages are simply not pinned.
//! [`pin_failure_to_report`] hands the first such failure to the shell
//! once, for one line on stderr. Nothing on the device changes. A
//! refused *allocation* is not carried on from: it is
//! `handle_alloc_error`, as everywhere else that allocates.

use core::marker::PhantomData;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicU8, Ordering};

use alloc::alloc::Layout;
use subtle::ConstantTimeEq;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// The page size, which is the alignment of every pinned allocation.
///
/// 4096 on the targets that can turn `pin-pages` on (Linux on x86-64 and
/// on the Pi's armv7), and 16384 on 64-bit macOS, whose pages are that
/// size. An alignment larger than the running kernel's page size is
/// still whole pages; a smaller one would share.
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
const PAGE: usize = 4096;

/// The page size on 64-bit macOS. See the other definition.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const PAGE: usize = 16384;

/// Whole pages for one `T`: page-aligned, and at least one page even for
/// a zero-sized value, since an allocation of no bytes is not one. A `T`
/// that wants more alignment than a page gets it, and its size rounds up
/// to that instead.
fn layout<T>() -> Layout {
    let align = PAGE.max(align_of::<T>());
    let bytes = size_of::<T>().next_multiple_of(align).max(align);
    Layout::from_size_align(bytes, align).expect("a whole number of pages is a valid layout")
}

/// Whether a page could not be pinned: 0 nothing, 1 failed and not yet
/// reported, 2 reported.
static PIN_FAILURE: AtomicU8 = AtomicU8::new(0);

/// Whether a secret page could not be pinned. True once, for the first
/// failure of the process, so that a shell can put one line on stderr and
/// then leave the subject alone.
pub fn pin_failure_to_report() -> bool {
    PIN_FAILURE
        .compare_exchange(1, 2, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
}

/// A value that is zeroized when dropped and lives on pages of its own
/// that the kernel is asked to keep out of swap.
///
/// The API is [`Secret`](crate::Secret)'s: no `Debug`, `Display`, `Clone`
/// or serialisation, plaintext only through [`expose`](Self::expose),
/// [`expose_mut`](Self::expose_mut) and [`with`](Self::with), and
/// constant-time equality where `T: AsRef<[u8]>`. Use it for the secrets
/// a session holds for as long as a key is loaded; a value that lives for
/// the length of one function is not worth a page.
pub struct Pinned<T: Zeroize> {
    /// One initialised `T`, in an allocation of whole pages made by
    /// [`raw::hold`] and freed by [`raw::release`] under the same
    /// [`layout`]. Nothing else points at it or frees it, so it is live
    /// and uniquely owned for as long as this value is.
    ptr: NonNull<T>,
    /// `Pinned<T>` owns its `T`, which `NonNull` alone does not say.
    owned: PhantomData<T>,
}

impl<T: Zeroize> Pinned<T> {
    /// Puts `value` on pages of its own and pins them, and zeroizes
    /// `value` where it stood.
    ///
    /// The caller's binding is moved from, so the bytes it held are
    /// unreachable either way; wiping them is what keeps them out of a
    /// memory image of the process. One copy is still made — the move
    /// into this call — and a caller that can fill the page instead of
    /// handing over a value should use [`new_with`](Self::new_with),
    /// which makes none.
    #[inline]
    pub fn new(value: T) -> Self {
        Pinned {
            ptr: raw::hold(value),
            owned: PhantomData,
        }
    }

    /// Puts a zeroed `T` on pages of its own, pins them, and lets `init`
    /// fill it in place.
    ///
    /// Nothing is copied: `init` writes through the pinned page, so the
    /// value never exists anywhere else. This is the way to build a
    /// secret that is assembled byte by byte, such as a key mixed from
    /// entropy the caller holds.
    #[inline]
    pub fn new_with(init: impl FnOnce(&mut T)) -> Self
    where
        T: Default,
    {
        let mut held = Self::new(T::default());
        init(held.expose_mut());
        held
    }

    /// Borrows the plaintext.
    #[inline]
    #[allow(unsafe_code)]
    pub fn expose(&self) -> &T {
        // SAFETY: `self.ptr` is non-null and aligned to at least
        // `align_of::<T>()` — `raw::hold` allocated it at `PAGE` or more
        // — and points to one `T` that `raw::hold` initialised by
        // writing the value into it. Only `raw::release` frees it, from
        // `drop`, so it is live for the whole life of `self`, and no
        // other value points at it, so this shared borrow is the only
        // access for as long as it lasts.
        unsafe { self.ptr.as_ref() }
    }

    /// Mutably borrows the plaintext, for filling a buffer in place.
    #[inline]
    #[allow(unsafe_code)]
    pub fn expose_mut(&mut self) -> &mut T {
        // SAFETY: as in `expose` — non-null, aligned, one initialised
        // `T`, live until `drop` frees it — and `self` is borrowed
        // mutably, so the unique ownership of the allocation makes this
        // the only reference to that `T` while it lasts.
        unsafe { self.ptr.as_mut() }
    }

    /// Runs `f` on the plaintext and returns its result.
    #[inline]
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(self.expose())
    }
}

impl<T: Zeroize> Drop for Pinned<T> {
    fn drop(&mut self) {
        self.expose_mut().zeroize();
        raw::release(self.ptr);
    }
}

impl<T: Zeroize> Zeroize for Pinned<T> {
    fn zeroize(&mut self) {
        self.expose_mut().zeroize();
    }
}

impl<T: Zeroize> ZeroizeOnDrop for Pinned<T> {}

impl<T: Zeroize + AsRef<[u8]>> PartialEq for Pinned<T> {
    /// Constant-time comparison of the byte representations, as
    /// [`Secret`](crate::Secret) compares.
    fn eq(&self, other: &Self) -> bool {
        self.expose().as_ref().ct_eq(other.expose().as_ref()).into()
    }
}

impl<T: Zeroize + AsRef<[u8]>> Eq for Pinned<T> {}

/// The pages themselves: taken, pinned, released.
mod raw {
    use core::ptr::NonNull;
    use core::sync::atomic::Ordering;

    use alloc::alloc::{alloc, dealloc, handle_alloc_error};
    use zeroize::Zeroize;

    use super::{PIN_FAILURE, layout};

    /// Moves `value` into whole pages of its own and pins them, leaving
    /// nothing behind: the bytes are copied to the page, the source is
    /// zeroized where it stands, and its destructor is not run, because
    /// what it owned belongs to the page now. A plain `ptr::write` would
    /// leave the caller's copy in the frame for a memory image to find.
    ///
    /// This is why `T: Zeroize` means what it says here: `zeroize` must
    /// overwrite the bytes of the `T` itself and not reach through a
    /// pointer it owns, since the page's copy holds the same pointer.
    /// Every `T` pinned in this crate is plain bytes.
    ///
    /// A refused allocation aborts, as any other allocation here does; a
    /// refused `mlock` is noted and carried on from.
    #[allow(unsafe_code)]
    pub fn hold<T: Zeroize>(mut value: T) -> NonNull<T> {
        let layout = layout::<T>();
        // SAFETY: `layout` has a non-zero size — a whole number of pages,
        // at least one — so `alloc` may be called with it. It returns
        // either null, which is handled before anything touches it, or a
        // pointer to `layout.size()` uninitialised bytes aligned to
        // `PAGE`, which is at least `size_of::<T>()` bytes aligned to at
        // least `align_of::<T>()`: enough to hold one `T`. `value` is a
        // live, initialised, owned `T`, so `size_of::<T>()` bytes may be
        // read from it, and it cannot overlap an allocation made after
        // it. After the copy the allocation holds one initialised `T`,
        // which is what the returned pointer promises its caller, and
        // the source is forgotten below rather than dropped, so only one
        // of the two is ever owned.
        let held = unsafe {
            let ptr = alloc(layout).cast::<T>();
            if ptr.is_null() {
                handle_alloc_error(layout);
            }
            core::ptr::copy_nonoverlapping(&raw const value, ptr, 1);
            NonNull::new_unchecked(ptr)
        };
        value.zeroize();
        core::mem::forget(value);
        if !sys::lock(held.as_ptr().cast(), layout.size()) {
            let _ = PIN_FAILURE.compare_exchange(0, 1, Ordering::Relaxed, Ordering::Relaxed);
        }
        held
    }

    /// Unpins and frees the pages `held` occupies. The caller has
    /// zeroized the value; nothing else reads it afterwards.
    #[allow(unsafe_code)]
    pub fn release<T: Zeroize>(held: NonNull<T>) {
        let layout = layout::<T>();
        // SAFETY: `held` came from `hold` and names an allocation of
        // `layout.size()` bytes aligned to `PAGE`, made by the global
        // allocator under this same layout, so it may be freed under it.
        // It holds one `T`, initialised by `hold` and zeroized by the
        // caller, which is dropped in place before the allocation goes:
        // zeroizing a `T` erases what it holds, not what it owns, so a
        // `T` with a destructor still needs it run. `release` runs once,
        // from `Pinned::drop`, on a value that is going away, so nothing
        // reads the allocation afterwards.
        unsafe {
            core::ptr::drop_in_place(held.as_ptr());
            sys::unlock(held.as_ptr().cast(), layout.size());
            dealloc(held.as_ptr().cast(), layout);
        }
    }

    /// `mlock` and `munlock`, where there is a `libc` to call them in.
    #[cfg(all(feature = "pin-pages", unix))]
    mod sys {
        /// Pins the `len` bytes at `ptr`; `false` if the kernel refused.
        pub fn lock(ptr: *const u8, len: usize) -> bool {
            call(true, ptr, len)
        }

        /// Unpins the `len` bytes at `ptr`.
        pub fn unlock(ptr: *const u8, len: usize) {
            let _ = call(false, ptr, len);
        }

        /// One call over one pinned allocation.
        #[allow(unsafe_code)]
        fn call(lock: bool, ptr: *const u8, len: usize) -> bool {
            let ptr = ptr.cast::<core::ffi::c_void>();
            // SAFETY: `ptr` addresses `len` bytes of one live,
            // page-aligned allocation owned by the `Pinned<T>` this was
            // called for, `len` being that allocation's whole size. Both
            // functions read the address and the length as a range and
            // change only whether the kernel may page it out; neither
            // reads or writes the memory, so no initialisation or
            // aliasing requirement applies to it.
            let rc = unsafe {
                if lock {
                    libc::mlock(ptr, len)
                } else {
                    libc::munlock(ptr, len)
                }
            };
            rc == 0
        }
    }

    /// The same two functions where there is no `libc` to call: the
    /// browser, Android, and any build with `pin-pages` off. The pages
    /// are allocated and freed as they are everywhere else; only the
    /// syscalls go.
    #[cfg(not(all(feature = "pin-pages", unix)))]
    mod sys {
        pub fn lock(_ptr: *const u8, _len: usize) -> bool {
            true
        }

        pub fn unlock(_ptr: *const u8, _len: usize) {}
    }
}
