#![feature(allocator_api)]
use std::alloc::{AllocError, Allocator, GlobalAlloc, Layout};
use std::ptr::NonNull;
use std::sync::Mutex;
mod simple;

// a chunk of bytes, guarded by a mutex
pub struct BumpAllocator {
    memory: Mutex<BumpMemory>,
}

// #[global_allocator]
// static ALLOCATOR: BumpAllocator = BumpAllocator::new();
//
// a chunk of bytes and an offset
struct BumpMemory {
    buffer: [u8; 1024], // Pre-allocated memory buffer
    offset: usize,      // Current allocation offset
}

impl BumpAllocator {
    // initialize with zeroed out buffer and offset of 0
    pub const fn new() -> Self {
        Self {
            memory: Mutex::new(BumpMemory {
                buffer: [0; 1024],
                offset: 0,
            }),
        }
    }
}

impl Drop for BumpAllocator {
    fn drop(&mut self) {
        println!("freed bumpallocator");
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        panic!("failed");
        println!("allocating");
        println!("allocating for layout = {:?}", layout);

        // lock memory
        let mut memory = self.memory.lock().unwrap();
        // set start to current offset
        let start = memory.offset;
        // set end to start plus requested Layout size
        let end = start + layout.size();

        // if overflow, panic
        if end > memory.buffer.len() {
            panic!("buffer overflow");
        } else {
            // set offset to end of allocated region
            memory.offset = end;
            println!("Allocated {} from {start} to {}", end - start, end - 1);
            let slice = &mut memory.buffer[start..end];
            // slice.get(0)
            slice[0] as *mut u8
            // &self.memory as usize + start as *mut u8
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        panic!("cannot dealloc from BumpAllocator");
    }
}

unsafe impl Allocator for BumpAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let mut memory = self.memory.lock().unwrap();
        let start = memory.offset;
        let end = start + layout.size();

        if end > memory.buffer.len() {
            Err(AllocError)
        } else {
            memory.offset = end;
            println!("Allocated {} from {start} to {}", end - start, end - 1);
            let slice = &mut memory.buffer[start..end];
            Ok(NonNull::from(slice))
        }
    }

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {
        // No-op: deallocation is unsupported in a bump allocator.
    }
}
//
// fn bump_local() {
//     let bump_allocator = BumpAllocator::new();
//     let mut my_vec: Vec<u8, &BumpAllocator> = Vec::with_capacity_in(1, &bump_allocator);
//     for i in 0u32..128 {
//         my_vec.push((i % 255).try_into().unwrap());
//     }
//
//     let my_vec_2: Vec<u8, &BumpAllocator> =
//         Vec::try_with_capacity_in(10000, &bump_allocator).expect("failed to allocate");
//
//     println!("{:?}", my_vec_2); // Outputs: [1, 2, 3, 4, 5]
// }

fn main() {
    println!("starting up");

    let my_vec = vec![0; 100];

    println!("my_vec = {:?}", my_vec);

    simple::run();

    let mut remaining: usize = 1024;

    println!("remaining = {:b}", remaining);
    let align: usize = 4;
    let align_mask_to_round_down = !(align - 1);
    let size: usize = 4 * 4;

    remaining -= size;

    println!("size = {:b}", size);
    println!("align = {:b}", align);
    println!("align_mask_to_round_down = {:b}", align_mask_to_round_down);
    println!("remaining = {:b}", remaining);
    println!("remaining = {}", remaining);

    remaining &= align_mask_to_round_down;

    println!("remaining = {:b}", remaining);
    println!("remaining = {}", remaining);

    let align: usize = 2;
    let align_mask_to_round_down = !(align - 1);
    let size: usize = 2 * 300;

    remaining -= size;

    println!("size = {:b}", size);
    println!("align = {:b}", align);
    println!("align_mask_to_round_down = {:b}", align_mask_to_round_down);
    println!("remaining = {:b}", remaining);
    println!("remaining = {}", remaining);

    remaining &= align_mask_to_round_down;

    println!("remaining = {:b}", remaining);
    println!("remaining = {}", remaining);
    // drop(bump_allocator);
}
