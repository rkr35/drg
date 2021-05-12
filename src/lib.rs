#![warn(clippy::pedantic)]

use std::ffi::c_void;
use std::ptr;

mod win;

#[no_mangle]
unsafe extern "system" fn DllMain(dll: *mut c_void, reason: u32, _: *mut c_void) -> i32 {
    if reason == win::DLL_PROCESS_ATTACH {
        win::DisableThreadLibraryCalls(dll);
        win::create_thread(on_attach, dll);
    } else if reason == win::DLL_PROCESS_DETACH {
    }

    1
}

unsafe extern "system" fn on_attach(dll: *mut c_void) -> u32 {
    let text = b"Text\0".as_ptr();
    let caption = b"Caption\0".as_ptr();
    win::MessageBoxA(ptr::null_mut(), text, caption, win::MB_OK);
    win::FreeLibraryAndExitThread(dll, 0);
    1
}
