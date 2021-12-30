// https://docs.microsoft.com/en-us/windows/win32/winprog/windows-data-types

use core::ffi::c_void;
use core::ptr;

pub mod file;
pub use file::File;

pub mod module;
pub use module::Module;

pub mod random;

pub const DLL_PROCESS_DETACH: u32 = 0;
pub const DLL_PROCESS_ATTACH: u32 = 1;
pub const STD_OUTPUT_HANDLE: u32 = 0xFFFF_FFF5;
pub const STD_INPUT_HANDLE: u32 = 0xFFFF_FFF6;

type ThreadProc = unsafe extern "system" fn(parameter: *mut c_void) -> u32;

#[link(name = "Kernel32")]
extern "system" {
    pub fn AllocConsole() -> i32;
    fn CloseHandle(object: *mut c_void) -> i32;
    fn CreateFileA(
        file_name: *const u8,
        desired_access: u32,
        share_mode: u32,
        security_attributes: *mut c_void,
        creation_disposition: u32,
        flags_and_attributes: u32,
        template_file: *mut c_void,
    ) -> *mut c_void;
    fn CreateThread(
        attributes: *mut c_void,
        stack_size: usize,
        start_address: ThreadProc,
        parameter: *mut c_void,
        creation_flags: u32,
        thread_id: *mut u32,
    ) -> *mut c_void;
    fn DisableThreadLibraryCalls(dll: *mut c_void) -> i32;
    fn FlushFileBuffers(file: *mut c_void) -> i32;
    pub fn FlushInstructionCache(
        hProcess: *mut c_void,
        lpBaseAddress: *const c_void,
        dwSize: usize,
    ) -> i32;
    pub fn FreeConsole() -> i32;
    pub fn FreeLibraryAndExitThread(dll: *mut c_void, exit_code: u32);
    pub fn GetCurrentProcess() -> *mut c_void;
    pub fn GetModuleHandleA(module_name: *const u8) -> *mut c_void;
    pub fn GetStdHandle(std_handle: u32) -> *mut c_void;
    pub fn ReadConsoleA(
        console_input: *mut c_void,
        buffer: *mut u8,
        len: u32,
        num_read: *mut u32,
        input_control: *mut c_void,
    ) -> i32;
    pub fn Sleep(dwMilliseconds: u32);
    pub fn QueryPerformanceCounter(lpPerformanceCount: *mut i64) -> i32;
    pub fn QueryPerformanceFrequency(lpFrequency: *mut i64) -> i32;
    pub fn VirtualProtect(
        lpAddress: *mut c_void,
        dwSize: usize,
        flNewProtect: u32,
        lpflOldProtect: *mut u32,
    ) -> i32;
    pub fn WriteConsoleA(
        console: *mut c_void,
        buffer: *const u8,
        len: u32,
        num_written: *mut u32,
        reserved: *mut c_void,
    ) -> i32;
    fn WriteFile(
        file: *mut c_void,
        buffer: *const u8,
        number_of_bytes_to_write: u32,
        number_of_bytes_written: *mut u32,
        overlapped: *mut c_void,
    ) -> i32;
}

#[link(name = "Bcrypt")]
extern "system" {
    fn BCryptGenRandom(hAlgorithm: *mut c_void, pbBuffer: *mut u8, cbBuffer: u32, dwFlags: u32) -> i32;
}

pub unsafe fn dll_main(
    dll: *mut c_void,
    reason: u32,
    on_attach: ThreadProc,
    on_detach: unsafe fn(),
) -> i32 {
    if reason == DLL_PROCESS_ATTACH {
        DisableThreadLibraryCalls(dll);
        CloseHandle(CreateThread(
            ptr::null_mut(),
            0,
            on_attach,
            dll,
            0,
            ptr::null_mut(),
        ));
    } else if reason == DLL_PROCESS_DETACH {
        on_detach();
    }

    1
}

pub unsafe fn idle() {
    let mut buffer = [0_u8; 2];
    let mut num_read = 0;

    // Our buffer is small (2 bytes). We're not truncating going from
    // usize to u32.
    #[allow(clippy::cast_possible_truncation)]
    ReadConsoleA(
        GetStdHandle(STD_INPUT_HANDLE),
        buffer.as_mut_ptr(),
        buffer.len() as u32,
        &mut num_read,
        ptr::null_mut(),
    );
}
