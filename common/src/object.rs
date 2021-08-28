use crate::split::ReverseSplitIterator;
use crate::win;
use crate::Error;
use crate::FName;
use crate::List;

use core::ffi::c_void;
use core::fmt::{self, Display, Formatter};
use core::mem;
use core::ops::BitOr;
use core::ptr;
use core::str;

pub static mut GUObjectArray: *const FUObjectArray = ptr::null();

const NumElementsPerChunk: usize = 64 * 1024;

// The maximum number of outers we can store in an array.
// Set to a large enough number to cover the outers length of all objects.
// Used when constructing an object's name, as well as for name comparisons.
const MAX_OUTERS: usize = 32;

#[repr(C)]
pub struct FUObjectArray {
    ObjFirstGCIndex: i32,
    ObjLastNonGCIndex: i32,
    MaxObjectsNotConsideredByGC: i32,
    OpenForDisregardForGC: bool,
    pub ObjObjects: TUObjectArray,
}

impl FUObjectArray {
    pub unsafe fn init(module: &win::Module) -> Result<(), Error> {
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

#[macro_export]
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
    };
}

#[repr(C)]
pub struct UObject {
    pub vtable: *mut *const c_void,
    ObjectFlags: u32, //EObjectFlags
    pub InternalIndex: i32,
    ClassPrivate: *const UClass,
    NamePrivate: FName,
    OuterPrivate: *mut UObject,
}

impl UObject {
    pub unsafe fn package(&self) -> *const UPackage {
        let mut top = self as *const UObject;

        while !(*top).OuterPrivate.is_null() {
            top = (*top).OuterPrivate;
        }

        top.cast()
    }

    pub unsafe fn package_mut(&mut self) -> *mut UPackage {
        let mut top = self as *mut UObject;

        while !(*top).OuterPrivate.is_null() {
            top = (*top).OuterPrivate;
        }

        top.cast()
    }

    pub unsafe fn fast_is(&self, class: EClassCastFlags) -> bool {
        (*self.ClassPrivate).ClassCastFlags.any(class)
    }

    pub unsafe fn name(&self) -> &str {
        self.NamePrivate.text()
    }

    pub unsafe fn process_event(
        this: *mut UObject,
        function: *mut UFunction,
        parameters: *mut c_void,
    ) {
        const PROCESS_EVENT_VTABLE_INDEX: usize = 66;
        type ProcessEvent = unsafe extern "C" fn(*mut UObject, *mut UFunction, *mut c_void);
        let process_event = mem::transmute::<*const c_void, ProcessEvent>(
            *(*this).vtable.add(PROCESS_EVENT_VTABLE_INDEX),
        );
        process_event(this, function, parameters);
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

            if self.NamePrivate.number() > 0 {
                write!(f, "_{}", self.NamePrivate.number() - 1)?;
            }
        }

        Ok(())
    }
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

#[repr(C)]
pub struct UStruct {
    base: UField,
    struct_base_chain: FStructBaseChain,
    pub SuperStruct: *mut UStruct,
    pub Children: *const UField,
    pub ChildProperties: *const FField,
    pub PropertiesSize: i32,
    pub MinAlignment: i32,
    pad1: [u8; 80],
}

impl_deref! { UStruct as UField }

#[repr(C)]
pub struct UClass {
    base: UStruct,
    pad0: [u8; 28],
    pub ClassFlags: EClassFlags,
    pub ClassCastFlags: EClassCastFlags,
    pad1: [u8; 344],
}

impl_deref! { UClass as UStruct }

impl UClass {
    pub fn is_blueprint_generated(&self) -> bool {
        self.ClassFlags
            .any(EClassFlags::CLASS_CompiledFromBlueprint)
    }
}

// #define RESULT_DECL void*const RESULT_PARAM
// typedef void (*FNativeFuncPtr)(UObject* Context, FFrame& TheStack, RESULT_DECL);

#[repr(C)]
struct FFrame {
    // TODO: fill in from UnrealEngine\Engine\Source\Runtime\CoreUObject\Public\UObject\Stack.h
    // struct FFrame : public FOutputDevice
    replace_me: *const c_void,
}

type FNativeFuncPtr =
    unsafe extern "C" fn(Context: *mut UObject, TheStack: *mut FFrame, Result: *const c_void);

// 	// Scope required for scoped script stats.
// 	{
// 		uint8* Frame = NULL;
// #if USE_UBER_GRAPH_PERSISTENT_FRAME
// 		if (Function->HasAnyFunctionFlags(FUNC_UbergraphFunction))
// 		{
// 			Frame = Function->GetOuterUClassUnchecked()->GetPersistentUberGraphFrame(this, Function);
// 		}
// #endif
// 		const bool bUsePersistentFrame = (NULL != Frame);
// 		if (!bUsePersistentFrame)
// 		{
// 			Frame = (uint8*)FMemory_Alloca(Function->PropertiesSize);
// 			// zero the local property memory
// 			FMemory::Memzero(Frame + Function->ParmsSize, Function->PropertiesSize - Function->ParmsSize);
// 		}

// 		// initialize the parameter properties
// 		FMemory::Memcpy(Frame, Parms, Function->ParmsSize);

// 		// Create a new local execution stack.
// 		FFrame NewStack(this, Function, Frame, NULL, Function->ChildProperties);

// 		checkSlow(NewStack.Locals || Function->ParmsSize == 0);

// inline FFrame::FFrame( UObject* InObject, UFunction* InNode, void* InLocals, FFrame* InPreviousFrame, FField* InPropertyChainForCompiledIn )
// 	: Node(InNode)
// 	, Object(InObject)
// 	, Code(InNode->Script.GetData())
// 	, Locals((uint8*)InLocals)
// 	, MostRecentProperty(NULL)
// 	, MostRecentPropertyAddress(NULL)
// 	, PreviousFrame(InPreviousFrame)
// 	, OutParms(NULL)
// 	, PropertyChainForCompiledIn(InPropertyChainForCompiledIn)
// 	, CurrentNativeFunction(NULL)
// 	, bArrayContextFailed(false)
// {
// #if DO_BLUEPRINT_GUARD
// 	FBlueprintExceptionTracker::Get().ScriptStack.Push(this);
// #endif
// }

#[repr(C)]
pub struct UFunction {
    base: UStruct,
    FunctionFlags: u32,
    NumParms: u8,
    ParmsSize: u16,
    ReturnValueOffset: u16,
    RPCId: u16,
    RPCResponseId: u16,
    FirstPropertyToInit: *const c_void,
    EventGraphFunction: *const UFunction,
    EventGraphCallOffset: i32,
    pub seen_count: u32,
    Func: FNativeFuncPtr,
}

impl_deref! { UFunction as UStruct }

#[repr(C)]
pub struct FFieldClass {
    pad0: [u8; 8],
    pub Id: EClassCastFlags,
    pub CastFlags: EClassCastFlags,
    pad1: [u8; 40],
}

#[repr(C)]
pub struct FField {
    vtable: usize,
    pub ClassPrivate: *const FFieldClass,
    pad0: [u8; 16],
    pub Next: *const FField,
    pub NamePrivate: FName,
    pub FlagsPrivate: u32,
    pad1: [u8; 4],
}

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct EClassCastFlags(pub u64);

impl EClassCastFlags {
    pub const CASTCLASS_UField: Self = Self(0x1);
    pub const CASTCLASS_FInt8Property: Self = Self(0x2);
    pub const CASTCLASS_UEnum: Self = Self(0x4);
    pub const CASTCLASS_UStruct: Self = Self(0x8);
    pub const CASTCLASS_UScriptStruct: Self = Self(0x10);
    pub const CASTCLASS_UClass: Self = Self(0x20);
    pub const CASTCLASS_FByteProperty: Self = Self(0x40);
    pub const CASTCLASS_FIntProperty: Self = Self(0x80);
    pub const CASTCLASS_FFloatProperty: Self = Self(0x100);
    pub const CASTCLASS_FUInt64Property: Self = Self(0x200);
    pub const CASTCLASS_FClassProperty: Self = Self(0x400);
    pub const CASTCLASS_FUInt32Property: Self = Self(0x800);
    pub const CASTCLASS_FInterfaceProperty: Self = Self(0x1000);
    pub const CASTCLASS_FNameProperty: Self = Self(0x2000);
    pub const CASTCLASS_FStrProperty: Self = Self(0x4000);
    pub const CASTCLASS_FProperty: Self = Self(0x8000);
    pub const CASTCLASS_FObjectProperty: Self = Self(0x10000);
    pub const CASTCLASS_FBoolProperty: Self = Self(0x20000);
    pub const CASTCLASS_FUInt16Property: Self = Self(0x40000);
    pub const CASTCLASS_UFunction: Self = Self(0x80000);
    pub const CASTCLASS_FStructProperty: Self = Self(0x100000);
    pub const CASTCLASS_FArrayProperty: Self = Self(0x200000);
    pub const CASTCLASS_FInt64Property: Self = Self(0x400000);
    pub const CASTCLASS_FDelegateProperty: Self = Self(0x800000);
    pub const CASTCLASS_FNumericProperty: Self = Self(0x1000000);
    pub const CASTCLASS_FMulticastDelegateProperty: Self = Self(0x2000000);
    pub const CASTCLASS_FObjectPropertyBase: Self = Self(0x4000000);
    pub const CASTCLASS_FWeakObjectProperty: Self = Self(0x8000000);
    pub const CASTCLASS_FLazyObjectProperty: Self = Self(0x10000000);
    pub const CASTCLASS_FSoftObjectProperty: Self = Self(0x20000000);
    pub const CASTCLASS_FTextProperty: Self = Self(0x40000000);
    pub const CASTCLASS_FInt16Property: Self = Self(0x80000000);
    pub const CASTCLASS_FDoubleProperty: Self = Self(0x100000000);
    pub const CASTCLASS_FSoftClassProperty: Self = Self(0x200000000);
    pub const CASTCLASS_UPackage: Self = Self(0x400000000);
    pub const CASTCLASS_ULevel: Self = Self(0x800000000);
    pub const CASTCLASS_AActor: Self = Self(0x1000000000);
    pub const CASTCLASS_APlayerController: Self = Self(0x2000000000);
    pub const CASTCLASS_APawn: Self = Self(0x4000000000);
    pub const CASTCLASS_USceneComponent: Self = Self(0x8000000000);
    pub const CASTCLASS_UPrimitiveComponent: Self = Self(0x10000000000);
    pub const CASTCLASS_USkinnedMeshComponent: Self = Self(0x20000000000);
    pub const CASTCLASS_USkeletalMeshComponent: Self = Self(0x40000000000);
    pub const CASTCLASS_UBlueprint: Self = Self(0x80000000000);
    pub const CASTCLASS_UDelegateFunction: Self = Self(0x100000000000);
    pub const CASTCLASS_UStaticMeshComponent: Self = Self(0x200000000000);
    pub const CASTCLASS_FMapProperty: Self = Self(0x400000000000);
    pub const CASTCLASS_FSetProperty: Self = Self(0x800000000000);
    pub const CASTCLASS_FEnumProperty: Self = Self(0x1000000000000);
    pub const CASTCLASS_USparseDelegateFunction: Self = Self(0x2000000000000);
    pub const CASTCLASS_FMulticastInlineDelegateProperty: Self = Self(0x4000000000000);
    pub const CASTCLASS_FMulticastSparseDelegateProperty: Self = Self(0x8000000000000);
    pub const CASTCLASS_FFieldPathProperty: Self = Self(0x10000000000000);

    pub fn any(&self, Self(flags): Self) -> bool {
        self.0 & flags != 0
    }
}

impl BitOr for EClassCastFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct EClassFlags(u32);

impl EClassFlags {
    pub const CLASS_CompiledFromBlueprint: Self = Self(0x40000);

    pub fn any(&self, Self(flags): Self) -> bool {
        self.0 & flags != 0
    }
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
        unsafe { str::from_utf8_unchecked(name) }
    }
}
