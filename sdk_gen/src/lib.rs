#![no_std]

// // https://docs.microsoft.com/en-us/cpp/c-runtime-library/crt-library-features?view=msvc-160
// #[link(name = "ucrt")]
// extern {}

// #[link(name = "msvcrt")]
// extern {}

#[link(name = "vcruntime")]
extern "C" {}

use core::ffi::c_void;
use core::fmt::{self, Write};
use core::str;

mod game;
use game::{TPair, UEnum, UObject};
mod list;
use list::List;
mod timer;
use timer::Timer;
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

    timer::initialize_ticks_per_second();

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
    // dump_objects()?;
    generate_sdk()?;
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
    let timer = Timer::new("dump global names");

    let mut file = win::File::new(sdk_file!("global_names.txt"))?;

    for name in (*game::NamePoolData).iter() {
        let text = (*name).text();
        writeln!(&mut file, "{}", text)?;
    }

    timer.stop();
    Ok(())
}

unsafe fn dump_objects() -> Result<(), Error> {
    let timer = Timer::new("dump global objects");

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

    timer.stop();
    Ok(())
}

unsafe fn generate_sdk() -> Result<(), Error> {
    let timer = Timer::new("generate sdk");

    let enum_class = (*game::GUObjectArray)
        .find("Class /Script/CoreUObject.Enum")?
        .cast();

    for object in (*game::GUObjectArray).iter().filter(|o| !o.is_null()) {
        if (*object).is(enum_class) {
            let mut out = List::<u8, 2048>::new();
            generate_enum(&mut out, object.cast())?;
            if let Ok(s) = out.as_str() {
                log!("{}", s);
            }
        }
    }

    timer.stop();
    Ok(())
}

unsafe fn generate_enum(mut out: impl Write, enumeration: *const UEnum) -> Result<(), Error> {
    let object = enumeration.cast::<UObject>();

    writeln!(
        out,
        "// {}\n#[repr(u8)]\npub enum {} {{",
        *object,
        (*object).name()
    )?;

    for TPair { Key: name, .. } in (*enumeration).Names.as_slice().iter() {
        let name = name.text();

        if let Some(namespace_colon) = name.as_bytes().iter().rposition(|&c| c == b':') {
            // SAFETY:
            // Per rposition():       0 <= namespace_colon   <  name.len()
            // Slice we're accessing: 1 <= namespace_colon+1 <= name.len()
            // Therefore, the slice is always within bounds and valid UTF8 (we started from an ASCII string).
            writeln!(out, "    {},", name.get_unchecked(namespace_colon + 1..))?;
        } else {
            writeln!(out, "    {},", name)?;
        }
    }

    writeln!(out, "}}")?;

    Ok(())
}

unsafe fn idle() {
    log!("Idling. Press enter to continue.");
    win::idle();
}
