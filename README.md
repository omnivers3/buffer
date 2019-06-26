# omni-buffer

Fixed size, contiguous u8 buffer with optional cache line padding support

--

Some errors, related to memory size alloc for ex, are hard to reproduce in unit tests and, so, aren't represented.

## References

[Hands-On Concurrency with Rust: Confidently build memory-safe, parallel, and efficient software in Rust](https://www.amazon.com/Hands-Concurrency-Rust-Brian-Troutwine/dp/1788399978)

[Vecio:Vector IO, scatter/gather, readv, writev](https://github.com/seanmonstar/vecio)

> Fast copy array values in/out from/to multiple sources

[Rust FFI - Transmute](https://www.reddit.com/r/rust/comments/2fmvcy/rust_ffi_and_opaque_pointer_idiom/)

[Rust Audio/Video Library - Using Heap & Opaque](https://github.com/rust-av/rust-av/issues/56)

[IoVec - Slice copying from sets of Vectors](https://carllerche.github.io/bytes/iovec/index.html)

[Heapless Data Structures in Rust](https://docs.rs/heapless/0.3.6/heapless/)

[Liballoc - Raw Vec](https://github.com/rust-lang/rust/blob/master/src/liballoc/raw_vec.rs#L90)

[List of CFG features](https://stackoverflow.com/questions/41742046/is-there-a-list-of-all-cfg-features/41743950)

[Gallery of Processor Cache Effects](http://igoro.com/archive/gallery-of-processor-cache-effects/)

[CppCon 2017: Carl Cook “When a Microsecond Is an Eternity: High Performance Trading Systems in C++”](https://www.youtube.com/watch?v=NH1Tta7purM)

[Rust Cache Alignment Issue](https://github.com/rust-lang/rust/issues/33626)

[Data alignment structure padding and sizes](https://github.com/kaisellgren/comp_sci.rs/wiki/Data-alignment,-structure-padding-and-sizes)