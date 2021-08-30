#![no_std]

// // https://docs.microsoft.com/en-us/cpp/c-runtime-library/crt-library-features?view=msvc-160
// #[link(name = "ucrt")]
// extern {}

#[link(name = "msvcrt")]
extern "C" {}

#[link(name = "vcruntime")]
extern "C" {}

use common::{self, win};
use core::ffi::c_void;
use core::ptr;
use sdk::Engine::Engine;

mod hooks;
use hooks::Hooks;

#[derive(macros::NoPanicErrorDebug)]
enum Error {
    Common(#[from] common::Error),
    Module(#[from] win::module::Error),
    Hooks(#[from] hooks::Error),
    FindGlobalEngine,
}

#[allow(non_upper_case_globals)]
static mut GEngine: *const Engine = ptr::null();

#[no_mangle]
unsafe extern "system" fn _DllMainCRTStartup(dll: *mut c_void, reason: u32, _: *mut c_void) -> i32 {
    win::dll_main(dll, reason, on_attach, on_detach)
}

unsafe extern "system" fn on_attach(dll: *mut c_void) -> u32 {
    win::AllocConsole();

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
    let module = win::Module::current()?;

    init_globals(&module)?;

    {
        let _hooks = Hooks::new(&module)?;
        common::idle();
    }

    Ok(())
}

unsafe fn init_globals(module: &win::Module) -> Result<(), Error> {
    common::init_globals(module)?;
    find_global_engine(module)?;
    Ok(())
}

unsafe fn find_global_engine(module: &win::Module) -> Result<(), Error> {
    // 00007FF63919DE6E   48:8B0D 137D3204   mov rcx,qword ptr ds:[7FF63D4C5B88]
    // 00007FF63919DE75   49:8BD7            mov rdx,r15
    // 00007FF63919DE78   48:8B01            mov rax,qword ptr ds:[rcx]
    // 00007FF63919DE7B   FF90 80020000      call qword ptr ds:[rax+280]
    const PATTERN: [Option<u8>; 19] = [
        Some(0x48),
        Some(0x8B),
        Some(0x0D),
        None,
        None,
        None,
        None,
        Some(0x49),
        Some(0x8B),
        Some(0xD7),
        Some(0x48),
        Some(0x8B),
        Some(0x01),
        Some(0xFF),
        Some(0x90),
        Some(0x80),
        Some(0x02),
        Some(0x00),
        Some(0x00),
    ];
    let mov_rcx_global_engine: *const u8 = module.find(&PATTERN).ok_or(Error::FindGlobalEngine)?;
    let relative_offset = mov_rcx_global_engine.add(3).cast::<u32>().read_unaligned();
    GEngine = *mov_rcx_global_engine
        .add(7 + relative_offset as usize)
        .cast::<*const Engine>();
    common::log!("GEngine = {}", GEngine as usize);
    Ok(())
}