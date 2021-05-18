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
