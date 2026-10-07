// A memory allocator that can be registered as the standard library’s default
//  through the `#[global_allocator]` attribute.
//
//  Some of the methods require that a memory block be *currently
//  allocated* via an allocator. This means that:
//
//  * the starting address for that memory block was previously
//    returned by a previous call to an allocation method
//    such as `alloc`, and
//
//  * the memory block has not been subsequently deallocated, where
//    blocks are deallocated either by being passed to a deallocation
//    method such as `dealloc` or by being
//    passed to a reallocation method that returns a non-null pointer.
//
//  # Example
//
//  ```standalone_crate
use std::alloc::{GlobalAlloc, Layout};
use std::cell::UnsafeCell;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

const ARENA_SIZE: usize = 128 * 1024;
const MAX_SUPPORTED_ALIGN: usize = 4096;
#[repr(C, align(4096))] // 4096 == MAX_SUPPORTED_ALIGN
struct SimpleAllocator {
    arena: UnsafeCell<[u8; ARENA_SIZE]>,
    remaining: AtomicUsize, // we allocate from the top, counting down
}

#[global_allocator]
static ALLOCATOR: SimpleAllocator = SimpleAllocator {
    arena: UnsafeCell::new([0x55; ARENA_SIZE]),
    remaining: AtomicUsize::new(ARENA_SIZE),
};

unsafe impl Sync for SimpleAllocator {}

unsafe impl GlobalAlloc for SimpleAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        if align > MAX_SUPPORTED_ALIGN {
            return null_mut();
        }
        // `Layout` contract forbids making a `Layout` with align=0, or align not power of 2.
        // So we can safely use a mask to ensure alignment without worrying about UB.
        let align_mask_to_round_down = !(align - 1);

        let mut allocated = 0;
        if self
            .remaining
            .try_update(Relaxed, Relaxed, |mut remaining| {
                // if there is not enough space return none
                if size > remaining {
                    return None;
                }
                // subtract allocated size from remaining
                remaining -= size;
                // apply the align mask to round down to
                // the nearest valid offset
                remaining &= align_mask_to_round_down;
                allocated = remaining;
                Some(remaining)
            })
            .is_err()
        {
            return null_mut();
        };

        unsafe { self.arena.get().cast::<u8>().add(allocated) }
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

pub fn run() {
    let _s = format!("allocating a string!");
    let currently = ALLOCATOR.remaining.load(Relaxed);
    println!("allocated so far: {}", ARENA_SIZE - currently);
}
// ```
//
// # The `#[global_allocator]` attribute
//
// As the example above demonstrates, the `#[global_allocator]` attribute can be used to register a
// concrete `static` of a type that implements this trait to become *the* global allocator
// for the current program. That global allocator can be invoked via the functions [`alloc`],
// [`alloc_zeroed`], [`dealloc`], [`realloc`]). Note, however, that invoking those functions is
// *not* equivalent to directly invoking the underlying methods on the declared global allocator!
// Users of the global allocator cannot assume anything about what the allocator does (even if they know which allocator is being used),
// and implementors of the allocator cannot assume anything about what the program does (even if they know how the allocator is being used).
// Both can only assume the documented requirements for the respective other party of this contract.
// This means:
//
// - Allocation functions may non-deterministically entirely skip the underlying allocator, e.g. if the
//   compiler can show that this allocation can be replaced by a stack variable. The compiler may
//   also merge multiple allocation operations into one, as long as it can also adjust all
//   corresponding deallocation operations accordingly.
// - An allocation created by invoking [`alloc`], [`alloc_zeroed`], or [`realloc`] has exactly the
//   size and minimum alignment defined by `layout`, even if the underlying allocator makes
//   stronger promises.
// - An allocation created by invoking [`alloc`], [`alloc_zeroed`], or [`realloc`] can only be
//   freed by invoking [`dealloc`] or [`realloc`]. In particular, passing a pointer to such an
//   allocation directly to the underlying method on [`GlobalAlloc`] is not permitted. Until one of
//   those functions is called, it is undefined behavior to access the memory that backs this
//   allocation with any pointer not derived from the return value of this function (e.g., with
//   internal pointers the allocator might keep around).
// - The pointer passed to [`dealloc`] or [`realloc`] must have been obtained by invoking [`alloc`],
//   [`alloc_zeroed`], or [`realloc`]. In particular, passing a pointer returned by the underlying
//   methods on [`GlobalAlloc`] is not permitted.
// - [`alloc`] de-initializes the contents of the allocation before handing it to the user. So even
//   if you control the underlying allocator and know that it explicitly initialized this memory,
//   you cannot rely on it being initialized. For a [`realloc`] that grows an allocation, this
//   applies to the newly allocated part.
// - [`dealloc`] de-initializes the contents of the allocation before handing it to the allocator.
//   So even if you know that the program previously initialized that memory, the allocator cannot
//   rely on it being initialized. For a [`realloc`] that shrinks an allocation, this applies to
//   the part being removed.
//
// [`alloc`]: ../../std/alloc/fn.alloc.html
// [`alloc_zeroed`]: ../../std/alloc/fn.alloc_zeroed.html
// [`dealloc`]: ../../std/alloc/fn.dealloc.html
// [`realloc`]: ../../std/alloc/fn.realloc.html
//
// The first point means that you cannot rely on global allocations actually happening, even if
// there are explicit global allocations in the source. The optimizer may detect unused global
// allocations that it can either eliminate entirely or move to the stack and thus never invoke the
// global allocator. The optimizer may further assume that allocation is infallible, so code that
// used to fail due to allocator failures may now suddenly work because the optimizer worked around
// the need for an allocation. More concretely, the following code example is unsound, irrespective
// of whether your custom allocator allows counting how many allocations have happened.
//
// ```rust,ignore (unsound and has placeholders)
// drop(Box::new(42));
// let number_of_heap_allocs = /* call private allocator API */;
// unsafe { std::hint::assert_unchecked(number_of_heap_allocs > 0); }
// ```
//
// Note that the optimizations mentioned above are not the only
// optimization that can be applied. You may generally not rely on global allocations
// happening if they can be removed without changing program behavior.
// Whether allocations happen or not is not part of the program behavior, even if it
// could be detected via an allocator that tracks allocations by printing or otherwise
// having side effects.
//
// # Safety
//
// The `GlobalAlloc` trait is an `unsafe` trait for a number of reasons, and
// implementors must ensure that they adhere to these contracts:
//
// * It is undefined behavior for the allocator to read, write, or deallocate any memory that
//   is *currently allocated*. This memory is owned by the user, the allocator must not touch it.
//
// * It's undefined behavior if global allocators unwind. This restriction may
//   be lifted in the future, but currently a panic from any of these
//   functions may lead to memory unsafety.
//
// * Callers of this trait are allowed to rely on the contracts defined on each method, and
//   implementors must ensure such contracts remain true.
//
// # Re-entrance
//
// When implementing a global allocator, one has to be careful not to create an infinitely recursive
// implementation by accident, as many constructs in the Rust standard library may allocate in
// their implementation. For example, on some platforms, [`std::sync::Mutex`] may allocate, so using
// it is highly problematic in a global allocator.
//
// For this reason, one should generally stick to library features available through
// [`core`], and avoid using [`std`] in a global allocator. A few features from [`std`] are
// guaranteed to not use `#[global_allocator]` to allocate:
//
//  - [`std::thread_local`],
//  - [`std::thread::current`],
//  - [`std::thread::park`] and [`std::thread::Thread`]'s [`unpark`] method and
//    [`Clone`] implementation.
//
// [`std`]: ../../std/index.html
// [`std::sync::Mutex`]: ../../std/sync/struct.Mutex.html
// [`std::thread_local`]: ../../std/macro.thread_local.html
// [`std::thread::current`]: ../../std/thread/fn.current.html
// [`std::thread::park`]: ../../std/thread/fn.park.html
// [`std::thread::Thread`]: ../../std/thread/struct.Thread.html
// [`unpark`]: ../../std/thread/struct.Thread.html#method.unpark
