use crate::util;

use core::{ptr, slice};

#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    GetModuleHandle,
    FindTextSection,
}

pub struct Module {
    start: usize,
    size: usize,
}

impl Module {
    const CAVE_BYTES: [u8; 3] = [0x00, 0x90, 0xCC];

    pub unsafe fn current() -> Result<Self, Error> {
        const SECTION: [u8; 5] = *b".text";
        const PAGE: usize = 0x1000;
        const PE_HEADER_SIZE: usize = PAGE; // overkill for our search.

        let base = super::GetModuleHandleA(ptr::null());

        if base.is_null() {
            return Err(Error::GetModuleHandle);
        }

        let pe_header: &[u8] = slice::from_raw_parts(base.cast(), PE_HEADER_SIZE);

        let section_header: *const SectionHeader = pe_header
            .windows(SECTION.len())
            .find(|&w| w == SECTION)
            .map(|w| w.as_ptr().cast())
            .ok_or(Error::FindTextSection)?;

        Ok(Self {
            start: base as usize + (*section_header).virtual_address as usize,
            size: util::align((*section_header).size_of_raw_data as usize, PAGE),
        })
    }

    pub unsafe fn find<T>(&self, pattern: &[Option<u8>]) -> Option<*const T> {
        slice::from_raw_parts(self.start as *const u8, self.size)
            .windows(pattern.len())
            .find(|w| {
                w.iter()
                    .zip(pattern)
                    .all(|(&w, p)| p.map_or(true, |p| w == p))
            })
            .map(|w| w.as_ptr().cast())
    }

    pub unsafe fn find_mut<T>(&self, pattern: &[Option<u8>]) -> Option<*mut T> {
        let mut cursor = self.start as *mut u8;
        let end = cursor.add(self.size - pattern.len());

        'outer: while cursor != end {
            for (i, &p) in pattern.iter().enumerate() {
                if let Some(p) = p {
                    if *cursor.add(i) != p {
                        cursor = cursor.add(1);
                        continue 'outer;
                    }
                }
            }

            return Some(cursor.cast());
        }

        None
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub unsafe fn find_code_cave(
        &self,
        start: *mut u8,
        min_required_len: usize,
    ) -> Option<&mut [u8]> {
        let backward = self.backward_cave_search(start, min_required_len);
        let forward = self.forward_cave_search(start, min_required_len);

        match [backward, forward] {
            [Some(b), Some(f)] => {
                if b.as_ptr().offset_from(start).abs() < f.as_ptr().offset_from(start).abs() {
                    Some(b)
                } else {
                    Some(f)
                }
            }

            [b, f] => b.or(f),
        }
    }

    unsafe fn backward_cave_search(
        &self,
        start: *mut u8,
        min_required_len: usize,
    ) -> Option<&'static mut [u8]> {
        let mut cursor = start;
        let module_start = self.start as *mut u8;

        while cursor >= module_start {
            // Advance to the end of the next code cave.
            if !Self::CAVE_BYTES.contains(&*cursor) {
                cursor = cursor.sub(1);
                continue;
            }

            let cave_end = cursor;

            // Advance to the start of this code cave.
            while cursor >= module_start && Self::CAVE_BYTES.contains(&*cursor) {
                cursor = cursor.sub(1);
            }

            let cave_start = cursor.add(1);

            // [cave_start, cave_end] is the cave range.
            let size = (cave_end.offset_from(cave_start) + 1) as usize;

            if size >= min_required_len {
                return Some(slice::from_raw_parts_mut(cave_start, size));
            }
        }

        None
    }

    unsafe fn forward_cave_search(
        &self,
        start: *mut u8,
        min_required_len: usize,
    ) -> Option<&'static mut [u8]> {
        let mut cursor = start;
        let module_end = (self.start + self.size) as *mut u8;

        while cursor < module_end {
            // Advance to the start of the next code cave.
            if !Self::CAVE_BYTES.contains(&*cursor) {
                cursor = cursor.add(1);
                continue;
            }

            let cave_start = cursor;

            // Advance to the end of this code cave.
            while cursor < module_end && Self::CAVE_BYTES.contains(&*cursor) {
                cursor = cursor.add(1);
            }

            // [cave_start, cursor) is all 0's.
            let size = cursor.offset_from(cave_start) as usize;

            if size >= min_required_len {
                return Some(slice::from_raw_parts_mut(cave_start, size));
            }
        }

        None
    }
}

#[repr(C)]
struct SectionHeader {
    name: [u8; 8],
    virtual_size: u32,
    virtual_address: u32,
    size_of_raw_data: u32,
    pointer_to_raw_data: u32,
    pointer_to_relocations: u32,
    pointer_to_line_numbers: u32,
    number_of_relocations: u16,
    number_of_line_numbers: u16,
    characteristics: u32,
}
