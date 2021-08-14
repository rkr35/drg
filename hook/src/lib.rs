#![no_std]

// // https://docs.microsoft.com/en-us/cpp/c-runtime-library/crt-library-features?view=msvc-160
// #[link(name = "ucrt")]
// extern {}

#[link(name = "msvcrt")]
extern "C" {}

#[link(name = "vcruntime")]
extern "C" {}

use common::{self, win, UFunction, UObject};
use core::ffi::c_void;

#[derive(macros::NoPanicErrorDebug)]
enum Error {
    Module(#[from] win::module::Error),
    Common(#[from] common::Error),
    NoCodeCave,
}

#[no_mangle]
unsafe extern "system" fn _DllMainCRTStartup(dll: *mut c_void, reason: u32, _: *mut c_void) -> i32 {
    win::dll_main(dll, reason, on_attach, on_detach)
}

unsafe extern "system" fn on_attach(dll: *mut c_void) -> u32 {
    win::AllocConsole();

    if let Err(e) = run() {
        common::log!("error: {:?}", e);
    }

    common::idle();

    win::FreeConsole();
    win::FreeLibraryAndExitThread(dll, 0);
    0
}

unsafe fn run() -> Result<(), Error> {
    let module = win::Module::current()?;
    common::init_globals(&module)?;

    let code_cave = module.find_code_cave().ok_or(Error::NoCodeCave)?;
    let cave_size = code_cave.len();

    common::log!(
        "Module starts at {} and is {} bytes.\n\
        Largest code cave begins at {} and is {} bytes.\n\
        my_process_event is at {}",
        module.start(),
        module.size(),
        code_cave.as_ptr() as usize,
        cave_size,
        my_process_event as usize,
    );

    Ok(())
}

unsafe fn on_detach() {}

unsafe extern "C" fn my_process_event(object: *mut UObject, function: *mut UFunction, parameters: *mut c_void) {
    common::log!("my_process_event({}, {}, {})", *object, *function, parameters as usize);
}