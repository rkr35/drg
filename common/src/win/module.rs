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
        const PE_HEADER_SIZE: usize = 0x1000; // one page size, kind of overkill for our search.

        let base = super::GetModuleHandleA(core::ptr::null());

        if base.is_null() {
            return Err(Error::GetModuleHandle);
        }

        let pe_header: &[u8] = core::slice::from_raw_parts(base.cast(), PE_HEADER_SIZE);

        let section_header: *const SectionHeader = pe_header
            .windows(SECTION.len())
            .find(|&w| w == SECTION)
            .map(|w| w.as_ptr().cast())
            .ok_or(Error::FindTextSection)?;

        Ok(Self {
            start: base as usize + (*section_header).virtual_address as usize,
            size: (*section_header).size_of_raw_data as usize,
        })
    }

    pub unsafe fn find<T>(&self, pattern: &[Option<u8>]) -> Option<*const T> {
        core::slice::from_raw_parts(self.start as *const u8, self.size)
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
        struct Searcher<'a> {
            space: &'a [u8],
            largest: Option<&'a [u8]>,
        }

        impl<'a> Searcher<'a> {
            fn new(space: &[u8]) -> Searcher {
                Searcher {
                    space,
                    largest: None,
                }
            }

            fn largest(&mut self) -> Option<&'a [u8]> {
                while let Some(cave) = self.find_next_cave() {
                    if self.is_cave_new_largest(cave) {
                        self.set_new_largest(cave);
                    }
                }
                self.largest
            }

            fn find_next_cave(&mut self) -> Option<&'a [u8]> {
                let cave = self.find_next_cave_beginning()?;
                Some(self.end_cave(cave))
            }

            fn find_next_cave_beginning(&mut self) -> Option<&'a [u8]> {
                let beginning = self.find_zero_position()?;
                let cave = unsafe { self.space.get_unchecked(beginning..) };
                self.advance_search_space_to(beginning);
                Some(cave)
            }

            fn find_zero_position(&self) -> Option<usize> {
                self.space.iter().position(|&b| b == 0)
            }

            fn end_cave(&mut self, mut cave: &'a [u8]) -> &'a [u8] {
                if let Some(ending) = Self::find_non_zero_position(cave) {
                    cave = unsafe { cave.get_unchecked(..ending) };
                    self.advance_search_space_to(ending);
                } else {
                    self.end_search_space();
                }
                cave
            }

            fn find_non_zero_position(cave: &'a [u8]) -> Option<usize> {
                cave.iter().position(|&b| b != 0)
            }

            fn advance_search_space_to(&mut self, position: usize) {
                self.space = unsafe { self.space.get_unchecked(position..) };
            }

            fn end_search_space(&mut self) {
                self.space = &[];
            }

            fn is_cave_new_largest(&self, cave: &[u8]) -> bool {
                cave.len() > self.largest.map_or(0, |l| l.len())
            }

            fn set_new_largest(&mut self, cave: &'a [u8]) {
                self.largest = Some(cave);
            }
        }

        let search_space = unsafe { core::slice::from_raw_parts(self.start as *const u8, self.size) };
        Searcher::new(search_space).largest()
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
