#![no_std]
#![allow(non_snake_case, non_upper_case_globals, non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::c_void;
use core::fmt;
use core::marker::PhantomData;
use core::ptr;
use core::slice;

pub mod win;

mod name;
pub use name::*;

mod object;
pub use object::*;

pub mod list;
pub use list::*;

mod split;
pub use split::*;

mod util;

#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    FindNamePoolData,
    FindGUObjectArray,
    Fmt(#[from] fmt::Error),
}

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

#[repr(C)]
pub struct TWeakObjectPtr<T> {
    base: FWeakObjectPtr,
    _marker: PhantomData<*const T>,
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

// #[repr(C)]
// pub struct TFieldPath<T> {
//     base: FFieldPath,
//     _marker: PhantomData<*const T>,
// }