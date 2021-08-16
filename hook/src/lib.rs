#![no_std]

// // https://docs.microsoft.com/en-us/cpp/c-runtime-library/crt-library-features?view=msvc-160
// #[link(name = "ucrt")]
// extern {}

#[link(name = "msvcrt")]
extern "C" {}

#[link(name = "vcruntime")]
extern "C" {}

use common::{self, win, List, UFunction, UObject};
use core::ffi::c_void;
use core::mem;
use core::slice;

#[derive(macros::NoPanicErrorDebug)]
enum Error {
    Module(#[from] win::module::Error),
    Common(#[from] common::Error),
    NoCodeCave,
    FindProcessEvent,
}

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

struct Patch<const N: usize> {
    address: *mut u8,
    original_bytes: List<u8, N>,
}

impl<const N: usize> Drop for Patch<N> {
    fn drop(&mut self) {

    }
}

struct ProcessEventHook {

}

impl ProcessEventHook {
    pub unsafe fn new(process_event: *mut u8, code_cave: &mut [u8]) -> ProcessEventHook {
        let first_six_process_event_bytes = slice::from_raw_parts(process_event, 6);

        let code_cave_patch = {
            let mut patch = [
                // push rcx
                0x51,

                // push rdx
                0x52, 
                
                // push r8
                0x41, 0x50,
                
                // mov rax, my_process_event (need to fill in)
                0x48, 0xB8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                
                // call rax
                0xFF, 0xD0,
                
                // pop r8
                0x41, 0x58,
                
                // pop rdx
                0x5A,
                
                // pop rcx
                0x59,
                
                // first six bytes of ProcessEvent (need to fill in)
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                
                // jmp ProcessEvent+6 (need to fill in)
                0xE9, 0x00, 0x00, 0x00, 0x00
            ];

            (&mut patch[6..6+mem::size_of::<usize>()]).copy_from_slice(&(my_process_event as usize).to_le_bytes());

            (&mut patch[20..20+6]).copy_from_slice(first_six_process_event_bytes);

            let patch_len = patch.len();

            (&mut patch[27..27+mem::size_of::<u32>()]).copy_from_slice({
                let destination = process_event as usize + 6;
                let source = code_cave.as_ptr() as usize + patch_len;
                let relative_distance = destination.wrapping_sub(source) as u32;
                &relative_distance.to_le_bytes()
            });

            patch
        };

        for b in code_cave_patch.iter() {
            common::log!("{}", b);
        }

        // 1. Write the code cave.
            // a. At code cave address, enable write access for 31 bytes.
            // b. Fill in address of my_process_event.
            // c. Calculate and fill in relative jump offset to rest of ProcessEvent.
            // d. Write code cave.
            // e. Restore original access protections.
        
        

        // 2. Place the jump.
            // a. At ProcessEvent address, enable write access for 6 bytes.
            // b. Calculate and fill in relative jump offset to code cave.
            // c. Write jump.

        ProcessEventHook {}
    }
}

impl Drop for ProcessEventHook {
    fn drop(&mut self) {
        // 1. Undo the jump.
            // a. At ProcessEvent address, enable write access for 6 bytes.
            // b. Restore original ProcessEvent 6 bytes.
            // d. Restore original access protections.

        // 2. Delete the code cave.
            // a. At code cave address, enable write access for 31 bytes.
            // b. Zero 31 bytes.
            // c. Restore original access protections.
    }
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

    let process_event = module.find_mut(&[Some(0x40), Some(0x55), Some(0x56), Some(0x57), Some(0x41), Some(0x54), Some(0x41), Some(0x55), Some(0x41), Some(0x56), Some(0x41), Some(0x57), Some(0x48), Some(0x81), Some(0xEC), Some(0xF0), Some(0x00), Some(0x00), Some(0x00)])
        .ok_or(Error::FindProcessEvent)?;

    let _process_event_hook = ProcessEventHook::new(process_event, code_cave);

    common::idle();

    Ok(())
}

unsafe fn on_detach() {}

unsafe extern "C" fn my_process_event(object: *mut UObject, function: *mut UFunction, parameters: *mut c_void) {
    common::log!("my_process_event({}, {}, {})", *object, *function, parameters as usize);
}