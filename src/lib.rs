#![no_std]
#![warn(clippy::pedantic)]

use core::ffi::c_void;

mod buffer;
use buffer::Buffer;

#[macro_use]
mod log;
mod win;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    extern "Rust" {
        #[link_name = "\n\nDetected possible panic in your code. Remove all panics.\n"]
        fn f() -> !;
    }

    unsafe { f() }
}

#[derive(Debug)]
enum Error {

}

#[no_mangle]
unsafe extern "system" fn DllMain(dll: *mut c_void, reason: u32, _: *mut c_void) -> i32 {
    win::dll_main(dll, reason, on_attach, on_detach)
}

unsafe extern "system" fn on_attach(dll: *mut c_void) -> u32 {
    win::AllocConsole();

    if let Err(e) = run() {
        log!("error: {:?}", e);
        idle();
    }

    win::FreeConsole();
    win::FreeLibraryAndExitThread(dll, 0);
    0
}

unsafe fn on_detach() {
    win::msg_box("on_detach()");
}

unsafe fn run() -> Result<(), Error> {
    idle();
    Ok(())
}

unsafe fn idle() {
    let mut buffer = Buffer::<32>::new();
    let mut num_read = 0;

    win::ReadConsoleA(
        win::GetStdHandle(win::STD_INPUT_HANDLE),
        buffer.as_mut_ptr(),
        buffer.capacity() as u32,
        &mut num_read,
        core::ptr::null_mut()
    );

    buffer.advance(num_read as usize);

    log!("{:?}", buffer);

    win::msg_box("idle()");
}