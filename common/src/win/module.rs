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

            return Some(cursor.cast())
        }

        None
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub unsafe fn find_code_cave(&self) -> Option<&mut [u8]> {
        let mut cursor = self.start as *mut u8;
        let end = cursor.add(self.size);
        let mut largest_cave: Option<(*mut u8, isize)> = None;

        'outer: while cursor != end {
            // Advance to the beginning of the next code cave.
            while *cursor != 0 {
                cursor = cursor.add(1);

                if cursor == end {
                    // No more code caves since we reached the end of the .text section.
                    break 'outer;
                }
            }

            let cave_begin = cursor;
            
            // Advance to the end of this code cave.
            while cursor != end && *cursor == 0 {
                cursor = cursor.add(1);
            }

            let size = cursor.offset_from(cave_begin);

            if size > largest_cave.map_or(0, |(_, size)| size) {
                largest_cave = Some((cave_begin, size));
            }
        }

        largest_cave.map(|(begin, size)| slice::from_raw_parts_mut(begin, size as usize))
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
