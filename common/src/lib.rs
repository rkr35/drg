#![no_std]
#![allow(non_snake_case, non_upper_case_globals, non_camel_case_types)]
use core::ffi::c_void;
use core::fmt;
use core::ptr;
use core::slice;

pub mod win;
mod name;
pub use name::*;


#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    FindNamePoolData,
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
pub struct FScriptDelegate {
    Object: FWeakObjectPtr,
    FunctionName: FName,
}