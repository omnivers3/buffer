#![feature(allocator_api)]
#![feature(const_fn_union)]

extern crate core;

#[macro_use]
extern crate log;

// TODO: more robust cache line detection
// #[cfg(target_pointer_width = "32")]
// const CACHE_LINE_SIZE: usize = 56;
// #[cfg(target_pointer_width = "64")]
const CACHE_LINE_SIZE: usize = 64;

extern crate memsec;

use std::cmp::{ max };
use std::alloc::{ Alloc, Global, Layout };
use std::mem;

#[derive(Debug)]
pub enum Errors {
    BufferSizeOverflow,
    AllocCapacityOverflow,
    InvalidLayout,
    InsufficientMemory,
    ZeroBufferNotSupported,
}

#[derive(Debug)]
pub struct Buffer<'a, T: 'a> {
    ptr: *mut u8,
    cap: usize,
    size: usize,
    padded_size: usize,
    pub entries: Vec<&'a mut T>,
}

fn zero_buffer<'a, T>() -> Result<Buffer<'a, T>, Errors> {
    // handles ZSTs and `cap = 0` alike
    // NonNull::<T>::dangling()
    Err (Errors::ZeroBufferNotSupported)
}

fn buffer_from<'a, T>(cap: usize, size: usize, padded_size: usize, alloc_size: usize) -> Result<Buffer<'a, T>, Errors> {
    let align = mem::align_of::<T>();
    Layout::from_size_align(alloc_size, align)
        .map_err(|err| {
            error!("Error creating layout for Buffer: {:?}", err);
            Errors::InvalidLayout
        })
        .and_then(|layout| {
            unsafe {
                Global.alloc_zeroed(layout) // Heap allocation
            }
            .map_err(|err| {
                error!("Error allocating heap for Buffer: {:?}", err);
                Errors::InsufficientMemory
            })
        })
        .map(|ptr| {
            let ptr: *mut u8 = ptr.as_ptr() as *mut u8;
            let mut entries: Vec<&mut T> = Vec::with_capacity(cap);
            for i in 0..cap {
                entries.push(
                    unsafe {
                        mem::transmute(ptr.add(i * padded_size))
                    }
                );
            }
            Buffer {
                ptr,
                cap,
                size,
                padded_size,
                entries,
            }
        })
}

enum Padding {
    None,
    Padded (usize),
    CacheAligned,
}

fn new<'a, T>(cap: usize, padding: Padding) -> Result<Buffer<'a, T>, Errors> {
    let size = mem::size_of::<T>();
    let padded_size: usize = match padding {
        Padding::None => size,
        Padding::Padded(padded_size) => {
            let padded_size = max(padded_size, size); // Must be at least as big as the things being contained
            let padded_size = max(padded_size, 1); // Size must also be at least one
            padded_size
        },
        Padding::CacheAligned => {
            if size % CACHE_LINE_SIZE == 0 { // Naturally aligned
                size
            } else {
                (size / CACHE_LINE_SIZE + 1) * CACHE_LINE_SIZE
            }
        }
    };

    cap
        .checked_mul(padded_size)
        .ok_or(Errors::BufferSizeOverflow)
        .and_then(alloc_guard)
        .and_then(|alloc_size| {
            if alloc_size == 0 {
                // NonNull::<T>::dangling()
                zero_buffer()
            } else {
                buffer_from::<T>(cap, size, padded_size, alloc_size)
            }
        })
}

impl <'a, T: 'a> Buffer<'a, T> {
    pub fn new(cap: usize) -> Result<Self, Errors> {
        new(cap, Padding::None)
    }

    pub fn padded(cap: usize, padded_size: usize) -> Result<Self, Errors> {
        new(cap, Padding::Padded(padded_size))
    }

    pub fn cache_aligned(cap: usize) -> Result<Self, Errors> {
        new(cap, Padding::CacheAligned)
    }

    pub fn cap(&self) -> usize {
        self.cap
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn padded_size(&self) -> usize {
        self.padded_size
    }

    pub fn data_size(&self) -> usize {
        self.cap * self.padded_size
    }

    pub fn entries(&self) -> &Vec<&'a mut T> {
        &self.entries
    }

    pub fn buffers(&self) -> Vec<Vec<u8>> {
        let mut buffers: Vec<Vec<u8>> = Vec::with_capacity(self.cap);
        for i in 0..self.cap {
            buffers.push(
                unsafe {
                    Vec::from_raw_parts(self.ptr.add(i * self.padded_size), self.size, self.size)
                }
            );
        }
        buffers
    }

    pub fn data(&self) -> Vec<u8> {
        let data_size = self.data_size();
        let data: Vec<u8> = unsafe {
            Vec::from_raw_parts(self.ptr, data_size, data_size)
        };
        data
    }
}

// We need to guarantee the following:
// * We don't ever allocate `> isize::MAX` byte-size objects
// * We don't overflow `usize::MAX` and actually allocate too little
//
// On 64-bit we just need to check for overflow since trying to allocate
// `> isize::MAX` bytes will surely fail. On 32-bit and 16-bit we need to add
// an extra guard for this in case we're running on a platform which can use
// all 4GB in user-space. e.g. PAE or x32
#[inline]
fn alloc_guard(alloc_size: usize) -> Result<usize, Errors> {
    if mem::size_of::<usize>() < 8 && alloc_size > ::core::isize::MAX as usize {
        Err(Errors::AllocCapacityOverflow)
    } else {
        Ok(alloc_size)
    }
}

#[cfg(test)]
mod tests {
    extern crate env_logger;
    // extern crate heapless;

    use super::*;

    // use self::heapless::consts::*;

    // #[derive(Debug)]
    // struct U8String {
    //     value: heapless::String<U8>,
    // }

    // #[derive(Debug)]
    // struct U32String {
    //     value: heapless::String<U32>,
    // }

    #[derive(Debug)]
    struct Thing {
        value1: usize,
        value2: usize,
    }

    #[test]
    fn should_capture_correct_properties_for_u8() {
        let buf = Buffer::<u8>::new(1).unwrap();
        assert_eq!(buf.cap(), 1);
        assert_eq!(buf.size(), 1);
        assert_eq!(buf.padded_size(), 1);
    }

    #[test]
    fn should_capture_correct_properties_for_u32() {
        let buf = Buffer::<u32>::new(1).unwrap();
        assert_eq!(buf.cap(), 1);
        assert_eq!(buf.size(), 4);
        assert_eq!(buf.padded_size(), 4);
    }

    // #[test]
    // fn should_capture_correct_properties_for_struct_with_u8string() {
    //     let buf = Buffer::<U8String>::new(1).unwrap();
    //     assert_eq!(buf.cap(), 1);
    //     assert_eq!(buf.size(), 16);
    //     assert_eq!(buf.padded_size(), 16);
    // }

    // #[test]
    // fn should_capture_correct_properties_for_struct_with_u32string() {
    //     let buf = Buffer::<U32String>::new(1).unwrap();
    //     assert_eq!(buf.cap(), 1);
    //     assert_eq!(buf.size(), 40);
    //     assert_eq!(buf.padded_size(), 40);
    // }

    #[test]
    fn should_capture_correct_properties_for_struct_thing() {
        let buf = Buffer::<Thing>::new(1).unwrap();
        assert_eq!(buf.cap(), 1);
        assert_eq!(buf.size(), 16);
        assert_eq!(buf.padded_size(), 16);
    }

    #[test]
    fn should_capture_correct_properties_for_struct_thing_with_padding() {
        let buf = Buffer::<Thing>::padded(1, 64).unwrap();
        assert_eq!(buf.cap(), 1);
        assert_eq!(buf.size(), 16);
        assert_eq!(buf.padded_size(), 64);
    }

    #[test]
    fn should_expand_buffer_entries_in_memory_but_not_views() {
        let buf = Buffer::<Thing>::padded(1, 64).unwrap();
        let buffers = buf.buffers();
        assert_eq!(buffers[0].len(), 16);
    }

    #[test]
    fn should_calculate_data_size_to_match_underying_buffer_size_for_unpadded() {
        let buf = Buffer::<u8>::new(1).unwrap();
        let data = buf.data();
        assert_eq!(data.len(), buf.data_size());
    }

    #[test]
    fn should_place_updated_data_propertly_in_second_slot() {
        let mut buf = Buffer::<u8>::new(2).unwrap();
        *buf.entries[1] = 12;
        assert_eq!(vec![0, 12], buf.data());
    }

    #[test]
    fn should_update_struct_in_data_properly() {
        let mut buf = Buffer::<Thing>::new(2).unwrap();
        buf.entries[0].value2 = 36;
        buf.entries[1].value1 = 12;
        assert_eq!(vec![0,0,0,0,0,0,0,0, 36,0,0,0,0,0,0,0, 12,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0], buf.data());
    }

    #[test]
    fn should_place_updated_data_propertly_in_second_slot_with_padding() {
        let mut buf = Buffer::<u8>::padded(2, 4).unwrap();
        *buf.entries[1] = 12;
        assert_eq!(vec![0,0,0,0, 12,0,0,0], buf.data());
    }

    #[test]
    fn should_update_struct_in_data_properly_with_padding() {
        let mut buf = Buffer::<Thing>::padded(2, 18).unwrap();
        buf.entries[0].value2 = 36;
        buf.entries[1].value1 = 12;
        assert_eq!(vec![0,0,0,0,0,0,0,0, 36,0,0,0,0,0,0,0, 0,0, 12,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0 ,0,0], buf.data());
    }

    // test cache aligned does correct padding
    // test cache aligned with natural align doesn't over pad
}