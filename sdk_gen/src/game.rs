#![allow(non_snake_case, non_upper_case_globals)]

use core::ffi::c_void;
use core::mem;
use core::ptr;
use core::str;

pub static mut NamePoolData: *const FNamePool = ptr::null();

#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    FindNamePoolData,
}

const FNameMaxBlockBits: u8 = 13;
const FNameBlockOffsetBits: u8 = 16;
const FNameMaxBlocks: usize = 1 << FNameMaxBlockBits;
const FNameBlockOffsets: usize = 1 << FNameBlockOffsetBits;
const Stride: usize = mem::align_of::<FNameEntry>();
const BlockSizeBytes: usize = Stride * FNameBlockOffsets;

#[repr(C)]
pub struct FNamePool {
    Lock: *mut c_void,
    CurrentBlock: u32,
    CurrentByteCursor: u32,
    Blocks: [*const u8; FNameMaxBlocks],
}

impl FNamePool {
    pub unsafe fn init(module: &crate::win::Module) -> Result<(), Error> {
        // 00007FF7F9DC1F96 | 897424 30                | mov dword ptr ss:[rsp+30],esi                           |
        // 00007FF7F9DC1F9A | 894424 34                | mov dword ptr ss:[rsp+34],eax                           |
        // 00007FF7F9DC1F9E | 74 09                    | je fsd-win64-shipping.7FF7F9DC1FA9                      |
        // 00007FF7F9DC1FA0 | 4C:8D05 99A17103         | lea r8,qword ptr ds:[7FF7FD4DC140]                      |
        // 00007FF7F9DC1FA7 | EB 16                    | jmp fsd-win64-shipping.7FF7F9DC1FBF                     |

        const NAME_POOL_DATA_PATTERN: [Option<u8>; 17] = [
            Some(0x89),
            Some(0x74),
            Some(0x24),
            Some(0x30),
            Some(0x89),
            Some(0x44),
            Some(0x24),
            Some(0x34),
            Some(0x74),
            Some(0x09),
            Some(0x4C),
            Some(0x8D),
            Some(0x05),
            None,
            None,
            None,
            None,
        ];

        // 00007FF7F9DC1F96 | 897424 30                | mov dword ptr ss:[rsp+30],esi                           |
        let mov: *const u8 = module
            .find(&NAME_POOL_DATA_PATTERN)
            .ok_or(Error::FindNamePoolData)?;

        // 00007FF7F9DC1FA7 | EB 16                    | jmp fsd-win64-shipping.7FF7F9DC1FBF                     |
        let instruction_after_lea = mov.add(NAME_POOL_DATA_PATTERN.len());

        // 00007FF7F9DC1FA0 | 4C:8D05 99A17103         | lea r8,qword ptr ds:[7FF7FD4DC140]                      |
        // 0x371A199
        // Silence clippy lint because we do an unaligned read.
        #[allow(clippy::cast_ptr_alignment)]
        let lea_immediate = instruction_after_lea.sub(4).cast::<u32>().read_unaligned();

        // 0x7FF7F9DC1FA7 + 0x371A199
        NamePoolData = instruction_after_lea.add(lea_immediate as usize).cast();

        Ok(())
    }

    pub unsafe fn iterate(&self, mut callback: impl FnMut(*const FNameEntry)) {
        unsafe fn iterate_block(
            mut it: *const u8,
            block_size: usize,
            callback: &mut impl FnMut(*const FNameEntry),
        ) {
            let end = it.add(block_size - mem::size_of::<FNameEntryHeader>());

            while it < end {
                let entry: *const FNameEntry = it.cast();
                let len = (*entry).len();

                if len > 0 {
                    callback(entry);
                    it = it.add(FNameEntry::get_size(len));
                } else {
                    // Null-terminator entry found
                    break;
                }
            }
        }

        let current_block = self.CurrentBlock as usize;

        crate::assert!(current_block < self.Blocks.len());

        for block in 0..current_block {
            iterate_block(self.Blocks[block], BlockSizeBytes, &mut callback);
        }

        iterate_block(
            self.Blocks[current_block],
            self.CurrentByteCursor as usize,
            &mut callback,
        );
    }
}

#[repr(C)]
struct FNameEntryHeader {
    bitfield: u16,
}

impl FNameEntryHeader {
    fn is_wide(&self) -> bool {
        self.bitfield & 1 == 1
    }

    fn len(&self) -> u16 {
        self.bitfield >> 6
    }
}

const NAME_SIZE: usize = 1024;

#[repr(C)]
pub struct FNameEntry {
    Header: FNameEntryHeader,
    AnsiName: [u8; NAME_SIZE],
}

impl FNameEntry {
    pub fn len(&self) -> usize {
        usize::from(self.Header.len())
    }

    pub unsafe fn text(&self) -> &str {
        if self.Header.is_wide() {
            "__[UNSUPPORTED WIDE TEXT]__"
        } else {
            str::from_utf8_unchecked(&self.AnsiName[..self.len()])
        }
    }

    fn get_size(length: usize) -> usize {
        align(mem::size_of::<FNameEntryHeader>() + length, Stride)
    }
}

fn align(x: usize, alignment: usize) -> usize {
    (x + alignment - 1) & !(alignment - 1)
}
