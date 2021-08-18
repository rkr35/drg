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
use core::mem;
use core::slice;

#[derive(macros::NoPanicErrorDebug)]
enum Error {
    Common(#[from] common::Error),
    Module(#[from] win::module::Error),
    NoCodeCave,
    FindProcessEvent,
}

#[no_mangle]
unsafe extern "system" fn _DllMainCRTStartup(dll: *mut c_void, reason: u32, _: *mut c_void) -> i32 {
    win::dll_main(dll, reason, on_attach, on_detach)
}

unsafe extern "system" fn on_attach(dll: *mut c_void) -> u32 {
    win::AllocConsole();

    if let Err(e) = run(dll) {
        common::log!("error: {:?}", e);
        common::idle();
    }

    win::FreeConsole();
    win::FreeLibraryAndExitThread(dll, 0);
    0
}

struct Patch<const N: usize> {
    dll: *mut c_void,
    address: *mut u8,
    original_bytes: [u8; N],
}

impl<const N: usize> Patch<N> {
    pub unsafe fn new(dll: *mut c_void, address: *mut u8, new_bytes: [u8; N]) -> Patch<N> {
        let mut original_bytes = [0; N];
        (&mut original_bytes).copy_from_slice(slice::from_raw_parts(address, N));

        Self::write(dll, address, new_bytes);

        Patch {
            dll,
            address,
            original_bytes,
        }
    }

    unsafe fn write(dll: *mut c_void, address: *mut u8, bytes: [u8; N]) {
        const PAGE_EXECUTE_READWRITE: u32 = 0x40;
        let mut old_protection = 0;
        win::VirtualProtect(address.cast(), N, PAGE_EXECUTE_READWRITE, &mut old_protection);
        slice::from_raw_parts_mut(address, N).copy_from_slice(&bytes);
        win::FlushInstructionCache(dll, address.cast(), N);
        win::VirtualProtect(address.cast(), N, old_protection, &mut old_protection);
    }
}

impl<const N: usize> Drop for Patch<N> {
    fn drop(&mut self) {
        unsafe {
            Self::write(self.dll, self.address, self.original_bytes);
        }
    }
}

struct ProcessEventHook {
    // Fields in a struct are dropped in their declaration order.
    // We restore a patch by dropping it.
    // We want to restore the jmp patch first (otherwise if we restored the code cave first, then the game would jump to a bunch of zeros).
    // So always ensure `_jmp` is declared before `_code_cave` in this structure.
    _jmp: Patch<6>,
    _code_cave: Patch<31>,
}

impl ProcessEventHook {
    pub unsafe fn new(dll: *mut c_void, process_event: *mut u8, code_cave: &mut [u8]) -> ProcessEventHook {
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
                0xE9, 0x00, 0x00, 0x00, 0x00,
            ];

            // mov rax, my_process_event
            (&mut patch[6..6+mem::size_of::<usize>()]).copy_from_slice(&(my_process_event as usize).to_le_bytes());

            // first six bytes of ProcessEvent
            (&mut patch[20..20+6]).copy_from_slice(first_six_process_event_bytes);

            // jmp ProcessEvent+6
            let patch_len = patch.len();
            (&mut patch[27..27+mem::size_of::<u32>()]).copy_from_slice({
                let destination = process_event as usize + 6;
                let source = code_cave.as_ptr() as usize + patch_len;
                let relative_distance = destination.wrapping_sub(source) as u32;
                &relative_distance.to_le_bytes()
            });

            patch
        };

        let jmp_patch = {
            let mut patch = [
                // jmp code_cave (need to fill in)
                0xE9, 0x00, 0x00, 0x00, 0x00,

                // nop (otherwise we would cut a two byte instruction in half)
                0x90,
            ];

            let destination = code_cave.as_ptr() as usize;
            let source = process_event as usize + 5;
            let relative_distance = destination.wrapping_sub(source) as u32;
            (&mut patch[1..1+mem::size_of::<u32>()]).copy_from_slice(&relative_distance.to_le_bytes());

            patch
        };

        ProcessEventHook {
            _jmp: Patch::new(dll, process_event, jmp_patch),
            _code_cave: Patch::new(dll, code_cave.as_mut_ptr(), code_cave_patch),
        }
    }
}

unsafe fn run(dll: *mut c_void) -> Result<(), Error> {
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

    let _process_event_hook = ProcessEventHook::new(dll, process_event, code_cave);

    common::idle();

    Ok(())
}

unsafe fn on_detach() {}

unsafe extern "C" fn my_process_event(object: *mut UObject, function: *mut UFunction, parameters: *mut c_void) {
    common::log!("my_process_event({}, {}, {})", *object, *function, parameters as usize);
}