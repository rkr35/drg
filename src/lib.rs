#![no_std]
#![warn(clippy::pedantic)]

use core::ffi::c_void;

mod win;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

// Pick up _DllMainCRTStartup
#[link(name = "msvcrt")]
extern {}

#[no_mangle]
unsafe extern "system" fn DllMain(dll: *mut c_void, reason: u32, _: *mut c_void) -> i32 {
    win::dll_main(dll, reason, on_attach, on_detach)
}

unsafe extern "system" fn on_attach(dll: *mut c_void) -> u32 {
    win::msg_box("attach");
    win::FreeLibraryAndExitThread(dll, 0);
    1
}

unsafe fn on_detach() {
    win::msg_box("detach");
}