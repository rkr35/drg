#![no_std]
#![warn(clippy::pedantic)]

use core::ffi::c_void;

// mod buffer;

mod game;
#[macro_use]
mod log;
mod util;
mod win;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    extern "Rust" {
        #[link_name = "\n\nDetected possible panic in your code. Remove all panics.\n"]
        fn f() -> !;
    }

    unsafe { f() }
}

#[derive(macros::NoPanicErrorDebug)]
enum Error {
    Module(#[from] win::module::Error),
    Game(#[from] game::Error),
}

#[no_mangle]
unsafe extern "system" fn _DllMainCRTStartup(dll: *mut c_void, reason: u32, _: *mut c_void) -> i32 {
    win::dll_main(dll, reason, on_attach, on_detach)
}

unsafe extern "system" fn on_attach(dll: *mut c_void) -> u32 {
    win::AllocConsole();

    if let Err(e) = run() {
        log!("error: {:?}", e);
        win::idle();
    }

    win::FreeConsole();
    win::FreeLibraryAndExitThread(dll, 0);
    0
}

unsafe fn on_detach() {
    win::msg_box("on_detach()");
}

unsafe fn run() -> Result<(), Error> {
    init_globals()?;
    dump_names()?;
    win::idle();
    Ok(())
}

unsafe fn init_globals() -> Result<(), Error> {
    let module = win::Module::current()?;
    log!(
        "module.start = {}, module.size = {}",
        module.start(),
        module.size()
    );
    game::FNamePool::init(&module)?;
    log!("NamePoolData = {}", game::NamePoolData as usize);
    // log!("CurrentBlock = {}; CurrentByteCursor = {}", (*game::NamePoolData).CurrentBlock, (*game::NamePoolData).CurrentByteCursor);
    Ok(())
}

unsafe fn dump_names() -> Result<(), Error> {
    (*game::NamePoolData).iterate(|name| {});
    Ok(())
}
