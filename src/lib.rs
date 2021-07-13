extern crate core;
extern crate log;
extern crate memsec;

// TODO: more robust cache line detection
// #[cfg(target_pointer_width = "32")]
// const CACHE_LINE_SIZE: usize = 56;
// #[cfg(target_pointer_width = "64")]
const CACHE_LINE_SIZE: usize = 64;

use std::cmp::{ max };
use std::alloc::{ alloc_zeroed, dealloc, Layout, LayoutError };
use std::mem;

#[derive(Debug)]
pub enum Error {
    AllocCapacityOverflow,
    BufferSizeOverflow,
    InsufficientMemory,
    LayoutError(LayoutError),
    ZeroBufferNotSupported,
}

impl From<LayoutError> for Error {
    fn from(error: LayoutError) -> Self {
        Error::LayoutError(error)
    }
}

#[derive(Debug)]
pub struct Buffer<'a, T: 'a> {
    layout: Layout,
    ptr: *mut u8,
    cap: usize,
    size: usize,
    padded_size: usize,
    pub entries: Vec<&'a mut T>,
}

fn buffer_from<'a, T>(cap: usize, size: usize, padded_size: usize, alloc_size: usize) -> Result<Buffer<'a, T>, Error> {
    let align = mem::align_of::<T>();
    let layout = Layout::from_size_align(alloc_size, align)?;
    // Heap allocation can yield undefined behavior if not checked to ensure non null pointer result
    // https://specs.amethyst.rs/docs/api/nom/lib/std/alloc/trait.globalalloc#tymethod.alloc
    let ptr = unsafe {
        let raw_ptr = alloc_zeroed(layout); // Heap allocation
        // See assertion example of non zero pointer:
        // https://edp.fortanix.com/docs/api/std/alloc/fn.alloc_zeroed.html
        if *(raw_ptr as *mut u16) != 0 {
            return Err(Error::InsufficientMemory);
        }
        raw_ptr as *mut u8
    };
    let mut entries: Vec<&mut T> = Vec::with_capacity(cap);
    for i in 0..cap {
        entries.push(
            unsafe {
                mem::transmute(ptr.add(i * padded_size))
            }
        );
    }
    Ok(Buffer {
        layout,
        ptr,
        cap,
        size,
        padded_size,
        entries,
    })
}

enum Padding {
    None,
    Padded (usize),
    CacheAligned,
}

fn new<'a, T>(cap: usize, padding: Padding) -> Result<Buffer<'a, T>, Error> {
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
    let alloc_size = cap.checked_mul(padded_size)
        .ok_or(Error::BufferSizeOverflow)
        .and_then(alloc_guard)?;
    if alloc_size == 0 {
        return Err (Error::ZeroBufferNotSupported)
    }
    buffer_from::<T>(cap, size, padded_size, alloc_size)
}

impl <'a, T: 'a> Buffer<'a, T> {
    pub fn new(cap: usize) -> Result<Self, Error> {
        new(cap, Padding::None)
    }

    pub fn dealloc(self) {
        unsafe {
            dealloc(self.ptr, self.layout)
        }
    }

    pub fn padded(cap: usize, padded_size: usize) -> Result<Self, Error> {
        new(cap, Padding::Padded(padded_size))
    }

    pub fn cache_aligned(cap: usize) -> Result<Self, Error> {
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
fn alloc_guard(alloc_size: usize) -> Result<usize, Error> {
    if mem::size_of::<usize>() < 8 && alloc_size > ::core::isize::MAX as usize {
        Err(Error::AllocCapacityOverflow)
    } else {
        Ok(alloc_size)
    }
}

#[cfg(test)]
mod tests {
    extern crate env_logger;
    use super::*;

    #[derive(Debug)]
    struct Thing {
        value1: u64,
        value2: u64,
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