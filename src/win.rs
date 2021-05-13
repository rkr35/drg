// https://docs.microsoft.com/en-us/windows/win32/winprog/windows-data-types

use core::ffi::c_void;
use core::ptr;

pub const DLL_PROCESS_DETACH: u32 = 0;
pub const DLL_PROCESS_ATTACH: u32 = 1;
pub const MB_OK: u32 = 0;

type ThreadProc = unsafe extern "system" fn(parameter: *mut c_void) -> u32;

#[link(name = "Kernel32")]
extern "system" {
    pub fn AllocConsole() -> i32;
    fn CreateThread(
        attributes: *mut c_void,
        stack_size: usize,
        start_address: ThreadProc,
        parameter: *mut c_void,
        creation_flags: u32,
        thread_id: *mut u32,
    ) -> *mut c_void;
    fn DisableThreadLibraryCalls(dll: *mut c_void) -> i32;
    pub fn FreeConsole() -> i32;
    pub fn FreeLibraryAndExitThread(dll: *mut c_void, exit_code: u32);
}

#[link(name = "User32")]
extern "system" {
    pub fn MessageBoxA(window: *mut c_void, text: *const u8, caption: *const u8, typ: u32) -> i32;
}

pub unsafe fn dll_main(dll: *mut c_void, reason: u32, on_attach: ThreadProc, on_detach: unsafe fn()) -> i32 {
    if reason == DLL_PROCESS_ATTACH {
        DisableThreadLibraryCalls(dll);
        CreateThread(ptr::null_mut(), 0, on_attach, dll, 0, ptr::null_mut());
    } else if reason == DLL_PROCESS_DETACH {
        on_detach();
    }

    1
}

pub unsafe fn msg_box<T: AsRef<[u8]>>(text: T) {
    let buffer = {
        let mut b = [0; 256];
        let text = text.as_ref();
        let copy_n = text.len().min(b.len() - 1);
        b[..copy_n].copy_from_slice(&text[..copy_n]);
        b
    };
    
    MessageBoxA(ptr::null_mut(), buffer.as_ptr(), ptr::null_mut(), MB_OK);
}
