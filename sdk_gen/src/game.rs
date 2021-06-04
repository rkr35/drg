#![allow(non_snake_case, non_upper_case_globals, non_camel_case_types)]

use core::ffi::c_void;
use core::fmt::{self, Display, Formatter};
use core::mem;
use core::ptr;
use core::str;

pub static mut NamePoolData: *const FNamePool = ptr::null();
pub static mut GUObjectArray: *const FUObjectArray = ptr::null();

#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    FindNamePoolData,
    FindGUObjectArray,
}

const FNameMaxBlockBits: u8 = 13;
const FNameBlockOffsetBits: u8 = 16;
const FNameMaxBlocks: usize = 1 << FNameMaxBlockBits;
const FNameBlockOffsets: usize = 1 << FNameBlockOffsetBits;
const Stride: usize = mem::align_of::<FNameEntry>();
const BlockSizeBytes: usize = Stride * FNameBlockOffsets;
const NumElementsPerChunk: usize = 64 * 1024;

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

    pub unsafe fn iter(&self) -> NameIterator {
        let first_block_size = if self.CurrentBlock > 0 {
            BlockSizeBytes
        } else {
            self.CurrentByteCursor as usize
        };

        NameIterator {
            pool: self,
            block: 0,
            cursor_within_block: self.Blocks[0],
            block_end_pos: self.Blocks[0]
                .add(first_block_size - mem::size_of::<FNameEntryHeader>()),
        }
    }

    unsafe fn get(&self, block: u32, offset: u32) -> *const FNameEntry {
        let block = block as usize;
        crate::assert!(block < self.Blocks.len());
        self.Blocks[block].add(Stride * offset as usize).cast()
    }
}

pub struct NameIterator<'pool> {
    pool: &'pool FNamePool,
    block: u32,
    cursor_within_block: *const u8,
    block_end_pos: *const u8,
}

impl Iterator for NameIterator<'_> {
    type Item = *const FNameEntry;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            // Did we finish iterating this block?
            if self.cursor_within_block >= self.block_end_pos {
                // Let's look at the next block.
                self.block += 1;

                // Get the size of the next block.
                let block_size = if self.block < self.pool.CurrentBlock {
                    // This block is filled.
                    BlockSizeBytes
                } else if self.block == self.pool.CurrentBlock {
                    // This block is the last block. It is partially filled.
                    self.pool.CurrentByteCursor as usize
                } else {
                    // There is no next block. We're done iterating all the blocks.
                    return None;
                };

                // Elide impossible panic branch.
                // We trust Unreal Engine will uphold its own invariant that self.CurrentBlock < FNameMaxBlocks.
                // Since self.block <= self.CurrentBlock, then self.block < FNameMaxBlocks.
                crate::assert!(self.block < self.pool.Blocks.len() as u32);

                // Get a pointer to the next block.
                self.cursor_within_block = self.pool.Blocks[self.block as usize];

                // Calculate where this block ends.
                self.block_end_pos = self
                    .cursor_within_block
                    .add(block_size - mem::size_of::<FNameEntryHeader>());
            }

            let entry: *const FNameEntry = self.cursor_within_block.cast();
            let len = (*entry).len();

            if len > 0 {
                // Advance our block cursor past this entry.
                self.cursor_within_block = self.cursor_within_block.add((*entry).get_size());

                // Yield the entry.
                Some(entry)
            } else {
                // Null-terminator entry found.
                // We're done iterating this block.
                self.cursor_within_block = self.block_end_pos;

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
        align(bytes, Stride)
    }
}

fn align(x: usize, alignment: usize) -> usize {
    (x + alignment - 1) & !(alignment - 1)
}

#[repr(C)]
pub struct FUObjectArray {
    ObjFirstGCIndex: i32,
    ObjLastNonGCIndex: i32,
    MaxObjectsNotConsideredByGC: i32,
    OpenForDisregardForGC: bool,
    pub ObjObjects: TUObjectArray,
}

impl FUObjectArray {
    pub unsafe fn init(module: &crate::win::Module) -> Result<(), Error> {
        // 00007FF773FACC96 | 44:0FB68C24 80000000     | movzx r9d,byte ptr ss:[rsp+80]                          |
        // 00007FF773FACC9F | 48:8D0D 3A7E5503         | lea rcx,qword ptr ds:[7FF777504AE0]                     |
        // 00007FF773FACCA6 | 44:8B8424 90000000       | mov r8d,dword ptr ss:[rsp+90]                           |

        const GU_OBJECT_ARRAY_PATTERN: [Option<u8>; 16] = [
            Some(0x44),
            Some(0x0F),
            Some(0xB6),
            Some(0x8C),
            Some(0x24),
            Some(0x80),
            Some(0x00),
            Some(0x00),
            Some(0x00),
            Some(0x48),
            Some(0x8D),
            Some(0x0D),
            None,
            None,
            None,
            None,
        ];

        // 00007FF773FACC96 | 44:0FB68C24 80000000     | movzx r9d,byte ptr ss:[rsp+80]                          |
        let movzx: *const u8 = module
            .find(&GU_OBJECT_ARRAY_PATTERN)
            .ok_or(Error::FindGUObjectArray)?;

        // 00007FF773FACCA6 | 44:8B8424 90000000       | mov r8d,dword ptr ss:[rsp+90]                           |
        let instruction_after_movsx = movzx.add(GU_OBJECT_ARRAY_PATTERN.len());

        // Silence clippy lint because we do an unaligned read.
        #[allow(clippy::cast_ptr_alignment)]
        let lea_immediate = instruction_after_movsx
            .sub(4)
            .cast::<u32>()
            .read_unaligned();

        GUObjectArray = instruction_after_movsx.add(lea_immediate as usize).cast();

        Ok(())
    }

    pub fn iter(&self) -> ObjectIterator {
        ObjectIterator {
            chunks: self.ObjObjects.Objects,
            num_objects: self.ObjObjects.NumElements as usize,
            index: 0,
        }
    }
}

pub struct ObjectIterator {
    chunks: *const *mut FUObjectItem,
    num_objects: usize,
    index: usize,
}

impl Iterator for ObjectIterator {    
    type Item = *mut UObject;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if self.index < self.num_objects {
                self.index += 1;
                let chunk = *self.chunks.add(self.index / NumElementsPerChunk);
                let object = chunk.add(self.index % NumElementsPerChunk);
                let object = (*object).Object;
                Some(object)
            } else {
                None
            }
        }
    }
}

#[repr(C)]
pub struct TUObjectArray {
    Objects: *const *mut FUObjectItem,
    PreAllocatedObjects: *mut FUObjectItem,
    MaxElements: i32,
    NumElements: i32,
    MaxChunks: i32,
    NumChunks: i32,
}

#[repr(C)]
pub struct FUObjectItem {
    Object: *mut UObject,
    Flags: i32,
    ClusterRootIndex: i32,
    SerialNumber: i32,
}

#[repr(C)]
pub struct UObject {
    vtable: usize,
    ObjectFlags: u32, //EObjectFlags
    pub InternalIndex: i32,
    ClassPrivate: *const UClass,
    NamePrivate: FName,
    OuterPrivate: *const UObject,
}

impl Display for UObject {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        unsafe {
            let name_entry = self.NamePrivate.entry();
            f.write_str((*name_entry).text())?;
        }

        if self.NamePrivate.Number > 0 {
            write!(f, "_{}", self.NamePrivate.Number)?;
        }

        Ok(())
    }
}

#[repr(C)]
pub struct UClass {}

#[repr(C)]
pub struct FName {
    ComparisonIndex: FNameEntryId,
    Number: u32,
}

impl FName {
    unsafe fn entry(&self) -> *const FNameEntry {
        self.ComparisonIndex.entry()
    }
}

#[repr(C)]
pub struct FNameEntryId {
    Value: u32,
}

impl FNameEntryId {
    fn block(&self) -> u32 {
        self.Value >> FNameBlockOffsetBits
    }

    fn offset(&self) -> u32 {
        self.Value & (FNameBlockOffsets - 1) as u32
    }

    unsafe fn entry(&self) -> *const FNameEntry {
        (*NamePoolData).get(self.block(), self.offset())
    }
}
