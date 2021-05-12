// https://docs.microsoft.com/en-us/windows/win32/winprog/windows-data-types

use std::ffi::c_void;
use std::ptr;

pub const DLL_PROCESS_DETACH: u32 = 0;
pub const DLL_PROCESS_ATTACH: u32 = 1;
pub const MB_OK: u32 = 0;

type ThreadProc = unsafe extern "system" fn(parameter: *mut c_void) -> u32;

#[link(name = "Kernel32")]
extern "system" {
    fn CreateThread(
        attributes: *mut c_void,
        stack_size: usize,
        start_address: ThreadProc,
        parameter: *mut c_void,
        creation_flags: u32,
        thread_id: *mut u32,
    ) -> *mut c_void;
    pub fn DisableThreadLibraryCalls(dll: *mut c_void) -> i32;
    pub fn FreeLibraryAndExitThread(dll: *mut c_void, exit_code: u32);
}

#[link(name = "User32")]
extern "system" {
    pub fn MessageBoxA(window: *mut c_void, text: *const u8, caption: *const u8, typ: u32) -> i32;
}

pub unsafe fn create_thread(on_attach: ThreadProc, parameter: *mut c_void) -> *mut c_void {
    CreateThread(ptr::null_mut(), 0, on_attach, parameter, 0, ptr::null_mut())
}
