#![no_std]
#![warn(clippy::pedantic)]

use core::ffi::c_void;

mod logger;

use log::{error, info, warn};

mod win;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
unsafe extern "system" fn DllMain(dll: *mut c_void, reason: u32, _: *mut c_void) -> i32 {
    win::dll_main(dll, reason, on_attach, on_detach)
}

unsafe extern "system" fn on_attach(dll: *mut c_void) -> u32 {
    win::AllocConsole();

    win::msg_box("attach1");

    if logger::init().is_err() {
        win::msg_box("Failed to initialize logger.");
    } else {
        info!("testing");
        warn!("warning");
        error!("erroring");
    }

    win::msg_box("attach2");

    win::FreeConsole();
    win::FreeLibraryAndExitThread(dll, 0);
    1
}

unsafe fn on_detach() {
    win::msg_box("detach");
}
