#![no_std]
#![allow(non_snake_case, non_upper_case_globals, non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

#[cfg(not(debug_assertions))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    extern "Rust" {
        #[link_name = "\n\nDetected possible panic in your code. Remove all panics.\n"]
        fn f() -> !;
    }

    unsafe { f() }
}

#[cfg(debug_assertions)]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

use core::ffi::c_void;
use core::marker::PhantomData;
use core::ptr;
use core::slice;

mod name;
pub use name::*;

mod object;
pub use object::*;

pub mod list;
pub use list::*;

mod split;
pub use split::*;

pub mod timer;
pub use timer::Timer;

mod util;

pub mod win;

#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    FindNamePoolData,
    Object(#[from] object::Error),
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct TArray<T> {
    data: *const T,
    pub len: i32,
    pub capacity: i32,
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
struct TSharedRef<T> {
    Object: *const T,
    SharedReferenceCount: *const c_void,
}

#[repr(C)]
struct ITextData {
    vtable: *const *const usize,
}

#[repr(C)]
pub struct FText {
    TextData: TSharedRef<ITextData>,
    Flags: u32,
}

#[repr(C)]
pub struct FWeakObjectPtr {
    ObjectIndex: i32,
    ObjectSerialNumber: i32,
}

impl FWeakObjectPtr {
    pub unsafe fn get(&self) -> *mut UObject {
        if self.ObjectSerialNumber == 0 || self.ObjectIndex < 0 {
            ptr::null_mut()
        } else {
            let object_item = (*GUObjectArray).index_to_object(self.ObjectIndex);
    
            if object_item.is_null() || (*object_item).SerialNumber != self.ObjectSerialNumber || !(*object_item).is_valid() {
                ptr::null_mut()
            } else {
                (*object_item).Object
            }
        }
    } 
}

#[repr(C)]
pub struct TWeakObjectPtr<T> {
    base: FWeakObjectPtr,
    _marker: PhantomData<*mut T>,
}

impl<T> TWeakObjectPtr<T> {
    pub unsafe fn get(&self) -> *mut T {
        self.base.get().cast()
    }
}

#[repr(C)]
pub struct FScriptDelegate {
    Object: FWeakObjectPtr,
    FunctionName: FName,
}

#[repr(C)]
pub struct TScriptInterface<T> {
    ObjectPointer: *const UObject,
    InterfacePointer: *const T,
}

#[repr(C)]
pub struct FMulticastScriptDelegate {
    InvocationList: TArray<FScriptDelegate>,
}

#[repr(C)]
pub struct FSparseDelegate {
    bIsBound: bool,
}

#[repr(C)]
pub struct FSoftObjectPath {
    AssetPathName: FName,
    SubPathString: FString,
}

#[repr(C)]
pub struct TPersistentObjectPtr<TObjectID> {
    WeakPtr: FWeakObjectPtr,
    TagAtLastTest: i32,
    ObjectID: TObjectID,
}

#[repr(C)]
pub struct FSoftObjectPtr {
    base: TPersistentObjectPtr<FSoftObjectPath>,
}

#[repr(C)]
pub struct TSoftObjectPtr<T> {
    SoftObjectPtr: FSoftObjectPtr,
    _marker: PhantomData<*const T>,
}

#[repr(C)]
pub struct TSoftClassPtr<T> {
    SoftObjectPtr: FSoftObjectPtr,
    _marker: PhantomData<*const T>,
}

#[repr(C)]
pub struct FFieldPath {
    ResolvedField: *const FField,
    ResolvedOwner: TWeakObjectPtr<UStruct>,
    Path: TArray<FName>,
}

#[repr(C)]
pub struct FGuid {
    A: u32,
    B: u32,
    C: u32,
    D: u32,
}

#[repr(C)]
pub struct FUniqueObjectGuid {
    Guid: FGuid,
}

#[repr(C)]
pub struct FLazyObjectPtr {
    base: TPersistentObjectPtr<FUniqueObjectGuid>,
}

#[repr(C)]
pub struct TLazyObjectPtr<T> {
    base: FLazyObjectPtr,
    _marker: PhantomData<*const T>,
}

// #[repr(C)]
// pub struct TFieldPath<T> {
//     base: FFieldPath,
//     _marker: PhantomData<*const T>,
// }

pub unsafe fn idle() {
    log!("Idling. Press enter to continue.");
    win::idle();
}

pub unsafe fn init_globals(module: &win::Module) -> Result<(), Error> {
    FNamePool::init(module)?;
    FUObjectArray::init(module)?;
    Ok(())
}
