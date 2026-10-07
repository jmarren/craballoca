pub fn run() {
    let x: i32 = 10;
    let y = 10.;
    let z: i8 = 2;
    let s1 = "hey there you!";
    let s2 = "hey there you! Literals like \"hello\" live in read-only data, and the compiler/linker places them wherever suits it. For plain char literals that's usually byte-aligned, though compilers sometimes align larger literals to 4, 8, or 16 bytes so that vectorized routines (like strlen) run faster.

Heap-allocated strings

malloc returns memory aligned to the platform's max fundamental alignment (typically 16 bytes on 64-bit), so a malloc'd string buffer starts well-aligned even though char doesn't require it. This is why strlen, memcpy, and friends can assume a decent starting alignment in practice, and why optimized libc versions handle the unaligned head and tail of a buffer separately, then process the aligned middle in word-sized or SIMD-sized chunks.";

    println!("x alignment = {:?}", Alignment::of_val(&x));
    println!("y alignment = {:?}", Alignment::of_val(&y));
    println!("z alignment = {:?}", Alignment::of_val(&z));
    println!("s1 alignment = {:?}", Alignment::of_val(s1));
    println!("s2 alignment = {:?}", Alignment::of_val(s2));
}
