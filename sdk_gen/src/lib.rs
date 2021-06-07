#![no_std]
#![warn(clippy::pedantic)]

// // https://docs.microsoft.com/en-us/cpp/c-runtime-library/crt-library-features?view=msvc-160
// #[link(name = "ucrt")]
// extern {}

// #[link(name = "msvcrt")]
// extern {}

use core::ffi::c_void;
use core::fmt::{self, Write};

mod game;
#[macro_use]
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
    File(#[from] win::file::Error),
    Fmt(#[from] fmt::Error),
}

#[no_mangle]
unsafe extern "system" fn _DllMainCRTStartup(dll: *mut c_void, reason: u32, _: *mut c_void) -> i32 {
    // unsafe extern "system" fn DllMain(dll: *mut c_void, reason: u32, _: *mut c_void) -> i32 {
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

unsafe fn on_detach() {}

unsafe fn run() -> Result<(), Error> {
    init_globals()?;
    dump_names()?;
    dump_objects()?;
    idle();
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
    game::FUObjectArray::init(&module)?;
    log!("NamePoolData = {}", game::NamePoolData as usize);
    log!("GUObjectArray = {}", game::GUObjectArray as usize);
    Ok(())
}

unsafe fn dump_names() -> Result<(), Error> {
    log!("dumping global names");

    let mut file = win::File::new(sdk_file!("global_names.txt"))?;

    for name in (*game::NamePoolData).iter() {
        let text = (*name).text();
        writeln!(&mut file, "{}", text)?;
    }

    log!("done dumping global names");
    Ok(())
}

unsafe fn dump_objects() -> Result<(), Error> {
    log!("dumping global objects");

    let mut file = win::File::new(sdk_file!("global_objects.txt"))?;

    for object in (*game::GUObjectArray).iter() {
        if object.is_null() {
            writeln!(&mut file, "skipped null object")?;
        } else {
            writeln!(
                &mut file,
                "[{}] {} {}",
                (*object).InternalIndex,
                *object,
                object as usize
            )?;
        }
    }

    log!("done dumping global objects");
    Ok(())
}

unsafe fn idle() {
    log!("Idling. Press enter to continue.");
    win::idle();
}
