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

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn find_code_cave(&self) -> Option<&[u8]> {
        fn find_next_cave<'a>(space: &mut &'a [u8]) -> Option<&'a [u8]> {
            // A cave begins at the next zero and extends to the first non-zero.

            // Find the next zero.
            let beginning = space.iter().position(|&b| b == 0)?;

            let mut cave = unsafe {
                // SAFETY: Per above `position` call, `beginning` is within bounds of `space`.
                space.get_unchecked(beginning..)
            };

            // Find the first non-zero after the zero.
            if let Some(length) = cave.iter().position(|&b| b != 0) {
                cave = unsafe {
                    // SAFETY: Per above `position` call, `length` is within bounds of `cave`.
                    cave.get_unchecked(..length)
                };
                *space = unsafe {
                    // SAFETY: `cave` is a subset of `space`, so we can skip past this subset while still being within
                    // `space`'s bounds.
                    space.get_unchecked(beginning + length..)
                };
            } else {
                // This cave extends to the end of the .text section. Our search space is now empty.
                *space = &[];
            }

            Some(cave)
        }

        let mut search_space = unsafe { slice::from_raw_parts(self.start as *const u8, self.size) };
        let mut largest: Option<&[u8]> = None;

        while let Some(cave) = find_next_cave(&mut search_space) {
            if cave.len() > largest.map_or(0, |l| l.len()) {
                largest = Some(cave);
            }
        }

        largest
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
