#![no_std]

// // https://docs.microsoft.com/en-us/cpp/c-runtime-library/crt-library-features?view=msvc-160
// #[link(name = "ucrt")]
// extern {}

#[link(name = "msvcrt")]
extern "C" {}

#[link(name = "vcruntime")]
extern "C" {}

use common::win;
use core::ffi::c_void;
use core::fmt::{self, Write};
use core::str;

mod buf_writer;
use buf_writer::BufWriter;
mod game;
mod generator;
use generator::Generator;
mod timer;
use timer::Timer;
mod util;

#[derive(macros::NoPanicErrorDebug)]
enum Error {
    Module(#[from] win::module::Error),
    Game(#[from] game::Error),
    File(#[from] win::file::Error),
    Fmt(#[from] fmt::Error),
    List(#[from] common::list::Error),
    Generator(#[from] generator::Error),
    Common(#[from] common::Error),
}

#[no_mangle]
unsafe extern "system" fn _DllMainCRTStartup(dll: *mut c_void, reason: u32, _: *mut c_void) -> i32 {
    win::dll_main(dll, reason, on_attach, on_detach)
}

unsafe extern "system" fn on_attach(dll: *mut c_void) -> u32 {
    win::AllocConsole();

    timer::initialize_ticks_per_second();

    if let Err(e) = run() {
        common::log!("error: {:?}", e);
        common::idle();
    }

    win::FreeConsole();
    win::FreeLibraryAndExitThread(dll, 0);
    0
}

unsafe fn on_detach() {}

unsafe fn run() -> Result<(), Error> {
    init_globals()?;
    dump_globals()?;
    generate_sdk()?;
    common::idle();
    Ok(())
}

unsafe fn init_globals() -> Result<(), Error> {
    let timer = Timer::new("init globals");
    let module = win::Module::current()?;
    common::FNamePool::init(&module)?;
    common::FUObjectArray::init(&module)?;
    timer.stop();

    common::log!(
        "module.start = {}, module.size = {}",
        module.start(),
        module.size()
    );
    common::log!("NamePoolData = {}", common::NamePoolData as usize);
    common::log!("GUObjectArray = {}", common::GUObjectArray as usize);
    Ok(())
}

unsafe fn dump_globals() -> Result<(), Error> {
    let timer = Timer::new("dump global names and objects");
    dump_names()?;
    dump_objects()?;
    timer.stop();
    Ok(())
}

unsafe fn dump_names() -> Result<(), Error> {
    let mut file = BufWriter::new(win::File::new(sdk_file!("global_names.txt"))?);

    for (index, name) in (*common::NamePoolData).iter() {
        let text = (*name).text();
        writeln!(&mut file, "[{}] {}", index.value(), text)?;
    }

    Ok(())
}

unsafe fn dump_objects() -> Result<(), Error> {
    let mut file = BufWriter::new(win::File::new(sdk_file!("global_objects.txt"))?);

    for object in (*common::GUObjectArray).iter() {
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

    Ok(())
}

unsafe fn generate_sdk() -> Result<(), Error> {
    let timer = Timer::new("generate sdk");
    Generator::new()?.generate_sdk()?;
    timer.stop();
    Ok(())
}
