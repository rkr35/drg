use crate::util;
use crate::win;
use crate::Error;

use core::cmp::Ordering;
use core::ffi::c_void;
use core::fmt::{self, Display, Formatter};
use core::mem;
use core::ptr;
use core::str;

pub static mut NamePoolData: *const FNamePool = ptr::null();

const FNameMaxBlockBits: u8 = 13;
const FNameBlockOffsetBits: u8 = 16;
const FNameMaxBlocks: usize = 1 << FNameMaxBlockBits;
const FNameBlockOffsets: usize = 1 << FNameBlockOffsetBits;
const Stride: usize = mem::align_of::<FNameEntry>();
const BlockSizeBytes: usize = Stride * FNameBlockOffsets;

#[repr(C)]
pub struct FName {
    ComparisonIndex: FNameEntryId,
    Number: u32,
}

impl FName {
    unsafe fn entry(&self) -> *const FNameEntry {
        self.ComparisonIndex.entry()
    }

    pub unsafe fn text(&self) -> &str {
        (*self.entry()).text()
    }

    pub fn number(&self) -> u32 {
        self.Number
    }
}

impl Display for FName {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        unsafe {
            if self.number() == 0 {
                self.text().fmt(f)
            } else {
                write!(f, "{}_{}", self.text(), self.number() - 1)
            }
        }
    }
}

#[repr(C)]
pub struct FNameEntryId {
    Value: u32,
}

impl FNameEntryId {
    fn from(block: u32, offset: u32) -> Self {
        Self {
            Value: (block << FNameBlockOffsetBits) | offset,
        }
    }

    fn block(&self) -> u32 {
        self.Value >> FNameBlockOffsetBits
    }

    fn offset(&self) -> u32 {
        self.Value & (FNameBlockOffsets - 1) as u32
    }

    pub fn value(&self) -> u32 {
        self.Value
    }

    unsafe fn entry(&self) -> *const FNameEntry {
        let block = self.block() as usize;
        let offset = self.offset() as usize;
        (*NamePoolData)
            .Blocks
            .get_unchecked(block)
            .add(Stride * offset)
            .cast()
    }
}

#[repr(C)]
pub struct FNamePool {
    Lock: *mut c_void,
    CurrentBlock: u32,
    CurrentByteCursor: u32,
    Blocks: [*const u8; FNameMaxBlocks],
}

impl FNamePool {
    pub unsafe fn init(module: &win::Module) -> Result<(), Error> {
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

    pub unsafe fn iter(&self) -> NameIterator {
        let first_block_size = if self.CurrentBlock > 0 {
            BlockSizeBytes
        } else {
            self.CurrentByteCursor as usize
        };

        NameIterator {
            pool: self,
            block: 0,
            block_start: self.Blocks[0],
            cursor_within_block: self.Blocks[0],
            block_end: self.Blocks[0].add(first_block_size - mem::size_of::<FNameEntryHeader>()),
        }
    }
}

pub struct NameIterator<'pool> {
    pool: &'pool FNamePool,
    block: u32,
    block_start: *const u8,
    cursor_within_block: *const u8,
    block_end: *const u8,
}

impl Iterator for NameIterator<'_> {
    type Item = (FNameEntryId, *const FNameEntry);

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            // Did we finish iterating this block?
            if self.cursor_within_block >= self.block_end {
                // Let's look at the next block.
                self.block += 1;

                // Get the size of the next block.
                let block_size = match self.block.cmp(&self.pool.CurrentBlock) {
                    // This block is filled.
                    Ordering::Less => BlockSizeBytes,

                    // This block is the last block. It is partially filled.
                    Ordering::Equal => self.pool.CurrentByteCursor as usize,

                    // There is no next block. We're done iterating all the blocks.
                    Ordering::Greater => return None,
                };

                // Get a pointer to the next block.
                // Use .get_unchecked() to elide impossible panic branch. We trust Unreal Engine will uphold its own
                // invariant that self.CurrentBlock < FNameMaxBlocks. Since self.block <= self.CurrentBlock, then
                // self.block < FNameMaxBlocks.
                self.block_start = *self.pool.Blocks.get_unchecked(self.block as usize);
                self.cursor_within_block = self.block_start;

                // Calculate where this block ends.
                self.block_end = self
                    .block_start
                    .add(block_size - mem::size_of::<FNameEntryHeader>());
            }

            let entry: *const FNameEntry = self.cursor_within_block.cast();
            let len = (*entry).len();

            if len > 0 {
                let offset =
                    (self.cursor_within_block as usize - self.block_start as usize) / Stride;

                // Advance our block cursor past this entry.
                self.cursor_within_block = self.cursor_within_block.add((*entry).get_size());

                // Yield the entry.
                Some((FNameEntryId::from(self.block, offset as u32), entry))
            } else {
                // Null-terminator entry found.
                // We're done iterating this block.
                self.cursor_within_block = self.block_end;

                // Try to pull an entry from the next block.
                self.next()
            }
        }
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
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

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

    fn get_size(&self) -> usize {
        let num_text_bytes = if self.Header.is_wide() {
            2 * self.len()
        } else {
            self.len()
        };
        let bytes = mem::size_of::<FNameEntryHeader>() + num_text_bytes;
        util::align(bytes, Stride)
    }
}

impl Display for FNameEntry {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        unsafe { f.write_str(self.text()) }
    }
}
