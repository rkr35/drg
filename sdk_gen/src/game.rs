#![allow(non_snake_case, non_upper_case_globals, non_camel_case_types)]

use crate::list::List;
use crate::split::ReverseSplitIterator;

use core::cmp::Ordering;
use core::convert::TryFrom;
use core::ffi::c_void;
use core::fmt::{self, Display, Formatter};
use core::mem;
use core::ptr;
use core::slice;
use core::str;

mod full_name;
use full_name::FullName;

pub static mut NamePoolData: *const FNamePool = ptr::null();
pub static mut GUObjectArray: *const FUObjectArray = ptr::null();

#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    FindNamePoolData,
    FindGUObjectArray,
    FullName(#[from] full_name::Error),
    UnableToFind(&'static str),
}

const FNameMaxBlockBits: u8 = 13;
const FNameBlockOffsetBits: u8 = 16;
const FNameMaxBlocks: usize = 1 << FNameMaxBlockBits;
const FNameBlockOffsets: usize = 1 << FNameBlockOffsetBits;
const Stride: usize = mem::align_of::<FNameEntry>();
const BlockSizeBytes: usize = Stride * FNameBlockOffsets;
const NumElementsPerChunk: usize = 64 * 1024;

// The maximum number of outers we can store in an array.
// Set to a large enough number to cover the outers length of all objects.
// Used when constructing an object's name, as well as for name comparisons.
const MAX_OUTERS: usize = 32;

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
            block_start: self.Blocks[0],
            cursor_within_block: self.Blocks[0],
            block_end: self.Blocks[0]
                .add(first_block_size - mem::size_of::<FNameEntryHeader>()),
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
                let offset = (self.cursor_within_block as usize - self.block_start as usize) / Stride;

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

impl Display for FNameEntry {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        unsafe { f.write_str(self.text()) }
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

    pub unsafe fn find(&self, name: &'static str) -> Result<*mut UObject, Error> {
        // Do a short-circuiting name comparison.

        // Compare the class from `name` against the class in `self`.
        // Then compare the outers in `name` against the outers in `self`.

        // This way, we don't have to construct the full name of `self` if we
        // can rule out non-matching classes and outers sooner.

        let target = FullName::<MAX_OUTERS>::try_from(name)?;

        'outer: for object in self.iter() {
            if object.is_null() {
                // We're not looking for a null object.
                continue;
            }

            let my_name = (*object).name().as_bytes();

            if my_name != target.name {
                // Object names don't match.
                // No need to check the class. Let's bail.
                continue;
            }

            let my_class = (*(*object).ClassPrivate).name().as_bytes();

            if my_class != target.class {
                // Classes don't match.
                // No need to check the outers. Let's bail.
                continue;
            }

            let mut my_outer = (*object).OuterPrivate;

            for target_outer in target.outers.iter() {
                if my_outer.is_null() {
                    // We have no more outers left to check for this object, but
                    // we still have target outers. So this object can't be what
                    // we're looking for. Let's check out the next object.
                    continue 'outer;
                }

                let my_outer_name = (*my_outer).name().as_bytes();

                if my_outer_name != *target_outer {
                    // This outer doesn't match the target outer we're looking for.
                    // No need to check the remaining outers. Let's bail.
                    continue 'outer;
                }

                // Advance up to the next outer.
                my_outer = (*my_outer).OuterPrivate;
            }

            // We got here because the name, class, and outers all match the
            // input name. So our search is over.
            return Ok(object);
        }

        // No object matched our search.
        Err(Error::UnableToFind(name))
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
                let chunk = *self.chunks.add(self.index / NumElementsPerChunk);
                let object = chunk.add(self.index % NumElementsPerChunk);
                let object = (*object).Object;
                self.index += 1;
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

macro_rules! impl_deref {
    ($Derived:ty as $Base:ty) => {
        impl core::ops::Deref for $Derived {
            type Target = $Base;

            fn deref(&self) -> &Self::Target {
                &self.base
            }
        }

        impl core::ops::DerefMut for $Derived {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.base
            }
        }

        impl core::fmt::Display for $Derived {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
                let object: &UObject = self;
                object.fmt(f)
            }
        }
    }
}

#[repr(C)]
pub struct UObject {
    vtable: usize,
    ObjectFlags: u32, //EObjectFlags
    pub InternalIndex: i32,
    ClassPrivate: *const UClass,
    NamePrivate: FName,
    OuterPrivate: *mut UObject,
}

impl UObject {
    pub unsafe fn package(&mut self) -> *mut UPackage {
        let mut top = self as *mut UObject;

        while !(*top).OuterPrivate.is_null() {
            top = (*top).OuterPrivate;
        }

        top.cast()
    }

    pub unsafe fn is(&self, class: *const UClass) -> bool {
        (*self.ClassPrivate)
            .struct_base_chain
            .is(&(*class).struct_base_chain)
    }

    pub unsafe fn name(&self) -> &str {
        self.NamePrivate.text()
    }
}

impl Display for UObject {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        unsafe {
            write!(f, "{} ", (*self.ClassPrivate).name())?;

            let mut outers = List::<&str, MAX_OUTERS>::new();
            let mut outer = self.OuterPrivate;

            while !outer.is_null() {
                if outers.push((*outer).name()).is_err() {
                    crate::log!("warning: reached outers capacity of {} for {}. outer name will be truncated.", outers.capacity(), self as *const _ as usize);
                    break;
                }

                outer = (*outer).OuterPrivate;
            }

            for outer in outers.iter().rev() {
                write!(f, "{}.", outer)?;
            }

            write!(f, "{}", self.name())?;

            if self.NamePrivate.Number > 0 {
                write!(f, "_{}", self.NamePrivate.Number - 1)?;
            }
        }

        Ok(())
    }
}

#[repr(C)]
pub struct FField {
    vtable: usize,
    pad: [u8; 48],
}

#[repr(C)]
pub struct UField {
    base: UObject,
    Next: *const UField,
}

impl_deref! { UField as UObject }

#[repr(C)]
pub struct FStructBaseChain {
    StructBaseChainArray: *const *const FStructBaseChain,
    NumStructBasesInChainMinusOne: i32,
}

impl FStructBaseChain {
    unsafe fn is(&self, other: &Self) -> bool {
        let other_index = other.NumStructBasesInChainMinusOne;
        let our_index = self.NumStructBasesInChainMinusOne;
        other_index <= our_index && *self.StructBaseChainArray.add(other_index as usize) == other
    }
}

#[repr(C)]
pub struct UStruct {
    base: UField,
    struct_base_chain: FStructBaseChain,
    pub SuperStruct: *mut UStruct,
    pub Children: *const UField,
    pub ChildProperties: *const FField,
    pub PropertiesSize: i32,
    pad1: [u8; 84],
}

impl_deref! { UStruct as UField }

#[repr(C)]
pub struct UClass {
    base: UStruct,
    pad: [u8; 384],
}

impl_deref! { UClass as UStruct }

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
        unsafe { (*self.entry()).fmt(f) }
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
pub struct UEnum {
    base: UField,
    CppType: FString,
    pub Names: TArray<TPair<FName, i64>>,
    CppForm: i32,
    EnumDisplayNameFn: usize,
}

impl_deref! { UEnum as UField }

#[repr(C)]
pub struct TArray<T> {
    data: *const T,
    len: i32,
    capacity: i32,
}

impl<T> TArray<T> {
    pub fn as_slice(&self) -> &[T] {
        unsafe {
            if self.data.is_null() || self.len == 0 {
                slice::from_raw_parts(ptr::NonNull::dangling().as_ptr(), 0)
            } else {
                slice::from_raw_parts(self.data, self.len as usize)
            }
        }
    }
}

pub type FString = TArray<u16>;

#[repr(C)]
pub struct TPair<K, V> {
    pub Key: K,
    pub Value: V,
}

#[repr(C)]
pub struct UPackage {
    base: UObject,
    unneeded_0: [u8; 56],
    pub PIEInstanceID: i32,
    unneeded_1: [u8; 60],
}

impl UPackage {
    pub fn short_name(&self) -> &str {
        let name = unsafe { self.base.name() }.as_bytes();
        let name = ReverseSplitIterator::new(name, b'/')
            .next()
            .unwrap_or(b"UPackage::short_name(): empty object name");

        // SAFETY: We started with an ASCII string (`self.base.name()`) and
        // split on an ASCII delimiter (`/`). Therefore, we still have a valid
        // ASCII string after the split. Since ASCII is a subset of UTF-8, the
        // bytes in `name` are valid UTF-8.
        unsafe { core::str::from_utf8_unchecked(name) }
    }
}
