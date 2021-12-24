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
    FindFunctionInvoke,
    FindProcessRemoteFunctionForChannel,
    FindAddCheats,
    FindPostActorConstruction,
    FindDestroyActor,
}

#[allow(non_upper_case_globals)]
static mut GEngine: *const Engine = ptr::null();

static mut FUNCTION_INVOKE: *mut c_void = ptr::null_mut();
static mut PROCESS_REMOTE_FUNCTION_FOR_CHANNEL: *mut c_void = ptr::null_mut();
static mut ADD_CHEATS: *mut c_void = ptr::null_mut();
static mut POST_ACTOR_CONSTRUCTION: *mut c_void = ptr::null_mut();
static mut DESTROY_ACTOR: *mut c_void = ptr::null_mut();

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
    find_function_invoke(module)?;
    find_process_remote_function_for_channel(module)?;
    find_add_cheats(module)?;
    find_post_actor_construction(module)?;
    find_destroy_actor(module)?;
    Ok(())
}

unsafe fn find_global_engine(module: &win::Module) -> Result<(), Error> {
    // 00007FF63919DE6E   48:8B0D 137D3204   mov rcx,qword ptr ds:[7FF63D4C5B88]
    // 00007FF63919DE75   49:8BD7            mov rdx,r15
    // 00007FF63919DE78   48:8B01            mov rax,qword ptr ds:[rcx]
    // 00007FF63919DE7B   FF90 80020000      call qword ptr ds:[rax+280]
    const PATTERN: [Option<u8>; 19] = [Some(0x48), Some(0x8B), Some(0x0D), None, None, None, None, Some(0x49), Some(0x8B), Some(0xD7), Some(0x48), Some(0x8B), Some(0x01), Some(0xFF), Some(0x90), Some(0x80), Some(0x02), Some(0x00), Some(0x00)];
    let mov_rcx_global_engine: *const u8 = module.find(&PATTERN).ok_or(Error::FindGlobalEngine)?;
    let relative_offset = mov_rcx_global_engine.add(3).cast::<u32>().read_unaligned();
    GEngine = *mov_rcx_global_engine.add(7 + relative_offset as usize).cast::<*const Engine>();
    Ok(())
}

unsafe fn find_function_invoke(module: &win::Module) -> Result<(), Error> {
    const PATTERN: [Option<u8>; 30] = [Some(0x48), Some(0x89), Some(0x5C), Some(0x24), Some(0x08), Some(0x48), Some(0x89), Some(0x6C), Some(0x24), Some(0x10), Some(0x48), Some(0x89), Some(0x74), Some(0x24), Some(0x18), Some(0x48), Some(0x89), Some(0x7C), Some(0x24), Some(0x20), Some(0x41), Some(0x56), Some(0x48), Some(0x83), Some(0xEC), Some(0x20), Some(0x48), Some(0x8B), Some(0x59), Some(0x20)];
    FUNCTION_INVOKE = module.find_mut(&PATTERN).ok_or(Error::FindFunctionInvoke)?;
    Ok(())
}

unsafe fn find_process_remote_function_for_channel(module: &win::Module) -> Result<(), Error> {
    const PATTERN: [Option<u8>; 43] = [Some(0x48), Some(0x8B), Some(0xC4), Some(0x4C), Some(0x89), Some(0x48), Some(0x20), Some(0x4C), Some(0x89), Some(0x40), Some(0x18), Some(0x48), Some(0x89), Some(0x50), Some(0x10), Some(0x48), Some(0x89), Some(0x48), Some(0x08), Some(0x55), Some(0x57), Some(0x41), Some(0x56), Some(0x41), Some(0x57), Some(0x48), Some(0x8D), Some(0xA8), None, None, None, None, Some(0x48), Some(0x81), Some(0xEC), None, None, None, None, Some(0xF6), Some(0x42), Some(0x30), Some(0x02)];
    PROCESS_REMOTE_FUNCTION_FOR_CHANNEL = module.find_mut(&PATTERN).ok_or(Error::FindProcessRemoteFunctionForChannel)?;
    Ok(())
}

unsafe fn find_add_cheats(module: &win::Module) -> Result<(), Error> {
    const PATTERN: [Option<u8>; 18] = [Some(0x48), Some(0x89), Some(0x5C), Some(0x24), Some(0x18), Some(0x48), Some(0x89), Some(0x74), Some(0x24), Some(0x20), Some(0x57), Some(0x48), Some(0x83), Some(0xEC), Some(0x50), Some(0x48), Some(0x8B), Some(0x01)];
    ADD_CHEATS = module.find_mut(&PATTERN).ok_or(Error::FindAddCheats)?;
    Ok(())
}

unsafe fn find_post_actor_construction(module: &win::Module) -> Result<(), Error> {
    // 00007FF66B98E688 | 48:8BCF                  | mov rcx,rdi                             |
    // 00007FF66B98E68B | E8 20D40000              | call fsd-win64-shipping.7FF66B99BAB0    | void AActor::PostActorConstruction()
    // 00007FF66B98E690 | 48:8B4D C0               | mov rcx,qword ptr ss:[rbp-40]           |
    // 00007FF66B98E694 | 48:33CC                  | xor rcx,rsp                             |
    // 00007FF66B98E697 | E8 C4510401              | call fsd-win64-shipping.7FF66C9D3860    |
    const PATTERN: [Option<u8>; 20] = [Some(0x48), Some(0x8B), Some(0xCF), Some(0xE8), None, None, None, None, Some(0x48), Some(0x8B), Some(0x4D), Some(0xC0), Some(0x48), Some(0x33), Some(0xCC), Some(0xE8), None, None, None, None];
    let mov_rcx_rdi: *mut u8 = module.find_mut(&PATTERN).ok_or(Error::FindPostActorConstruction)?;
    let call_immediate = mov_rcx_rdi.add(4).cast::<u32>().read_unaligned();
    POST_ACTOR_CONSTRUCTION = mov_rcx_rdi.add(8 + call_immediate as usize).cast();
    Ok(())
}

unsafe fn find_destroy_actor(module: &win::Module) -> Result<(), Error> {
    const PATTERN: [Option<u8>; 30] = [Some(0x40), Some(0x55), Some(0x53), Some(0x56), Some(0x57), Some(0x41), Some(0x54), Some(0x41), Some(0x56), Some(0x41), Some(0x57), Some(0x48), Some(0x8D), Some(0x6C), Some(0x24), Some(0x90), Some(0x48), Some(0x81), Some(0xEC), Some(0x70), Some(0x01), Some(0x00), Some(0x00), Some(0x48), Some(0x8B), Some(0x05), Some(0xAA), Some(0x36), Some(0x30), Some(0x02)];
    DESTROY_ACTOR = module.find_mut(&PATTERN).ok_or(Error::FindDestroyActor)?;
    Ok(())
}