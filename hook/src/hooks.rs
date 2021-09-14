use common::{self, win, UObject};
use core::ffi::c_void;
use core::mem::{self, ManuallyDrop};
use core::ptr;
use core::slice;

mod patch;
use patch::Patch;

mod user;

static mut PROCESS_EVENT: *const c_void = ptr::null();
static mut DRAW_TRANSITION: *const c_void = ptr::null();

#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    FindProcessEvent,
    NoCodeCave,
    CaveIsTooSmall(usize, usize),
}

pub struct Hooks {
    _process_event_hook: ProcessEventHook,
    _draw_transition_hook: DrawTransitionHook,
}

impl Hooks {
    pub unsafe fn new(module: &win::Module) -> Result<Self, Error> {
        Ok(Self {
            _process_event_hook: ProcessEventHook::new(module)?,
            _draw_transition_hook: DrawTransitionHook::new(),
        })
    }
}

struct ProcessEventHook {
    jmp: ManuallyDrop<Patch<[u8; 6]>>,
    code_cave: ManuallyDrop<Patch<[u8; 23]>>,
}

impl Drop for ProcessEventHook {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.jmp);
            // Before we destroy the code cave, give the CPU time to exit the cave.
            win::Sleep(100);
            ManuallyDrop::drop(&mut self.code_cave);
        }
    }
}

impl ProcessEventHook {
    pub unsafe fn new(module: &win::Module) -> Result<ProcessEventHook, Error> {
        let process_event: *mut u8 = module
            .find_mut(&[
                Some(0x40),
                Some(0x55),
                Some(0x56),
                Some(0x57),
                Some(0x41),
                Some(0x54),
                Some(0x41),
                Some(0x55),
                Some(0x41),
                Some(0x56),
                Some(0x41),
                Some(0x57),
                Some(0x48),
                Some(0x81),
                Some(0xEC),
                Some(0xF0),
                Some(0x00),
                Some(0x00),
                Some(0x00),
            ])
            .ok_or(Error::FindProcessEvent)?;

        let code_cave = module.find_code_cave().ok_or(Error::NoCodeCave)?;
        let cave_size = code_cave.len();

        common::log!(
            "Module starts at {} and is {} bytes.\n\
            Largest code cave begins at {} and is {} bytes.\n\
            process_event is at {}\n\
            my_process_event is at {}",
            module.start(),
            module.size(),
            code_cave.as_ptr() as usize,
            cave_size,
            process_event as usize,
            user::my_process_event as usize,
        );

        let code_cave_patch = ManuallyDrop::new(Patch::new(
            code_cave.as_mut_ptr().cast(),
            Self::create_code_cave_patch(code_cave, process_event)?,
        ));

        let jmp = ManuallyDrop::new(Patch::new(
            process_event.cast(),
            Self::create_jmp_patch(code_cave, process_event),
        ));

        Ok(ProcessEventHook {
            jmp,
            code_cave: code_cave_patch,
        })
    }

    unsafe fn create_jmp_patch(code_cave: &[u8], process_event: *const u8) -> [u8; 6] {
        let mut patch = [
            // jmp code_cave (need to fill in)
            0xE9, 0x00, 0x00, 0x00, 0x00,
            // nop (otherwise we would cut a two byte instruction in half)
            0x90,
        ];

        let destination = code_cave.as_ptr() as usize;
        let source = process_event as usize + 5;
        let relative_distance = destination.wrapping_sub(source) as u32;
        (&mut patch[1..=mem::size_of::<u32>()]).copy_from_slice(&relative_distance.to_le_bytes());

        patch
    }

    unsafe fn create_code_cave_patch(
        code_cave: &[u8],
        process_event: *const u8,
    ) -> Result<[u8; 23], Error> {
        #[rustfmt::skip]
        let mut patch = [
            // mov rax, user::my_process_event (need to fill in)
            0x48, 0xB8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,

            // jmp rax
            0xFF, 0xE0,

            // first six bytes of ProcessEvent that we overwrote with the jmp to codecave (need to fill in)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,

            // jmp ProcessEvent+6 (need to fill in)
            0xE9, 0x00, 0x00, 0x00, 0x00, 
        ];

        if code_cave.len() < patch.len() {
            return Err(Error::CaveIsTooSmall(code_cave.len(), patch.len()));
        }

        // mov rax, my_process_event
        (&mut patch[2..2 + mem::size_of::<usize>()])
            .copy_from_slice(&(user::my_process_event as usize).to_le_bytes());

        // first six bytes of ProcessEvent
        let first_six_process_event_bytes = slice::from_raw_parts(process_event, 6);
        (&mut patch[12..12 + first_six_process_event_bytes.len()])
            .copy_from_slice(first_six_process_event_bytes);

        PROCESS_EVENT = code_cave.as_ptr().add(12).cast();

        // jmp ProcessEvent+6
        let patch_len = patch.len();
        (&mut patch[19..19 + mem::size_of::<u32>()]).copy_from_slice({
            let destination = process_event as usize + first_six_process_event_bytes.len();
            let source = code_cave.as_ptr() as usize + patch_len;
            let relative_distance = destination.wrapping_sub(source) as u32;
            &relative_distance.to_le_bytes()
        });

        Ok(patch)
    }
}

struct DrawTransitionHook {
    _patch: Patch<*const c_void>,
}

impl DrawTransitionHook {
    pub unsafe fn new() -> Self {
        const VTABLE_INDEX: usize = 0x310 / 8;
        let address = (*(*crate::GEngine).GameViewport.cast::<UObject>())
            .vtable
            .add(VTABLE_INDEX);
        DRAW_TRANSITION = *address;
        Self {
            _patch: Patch::new(address, user::my_draw_transition as *const c_void),
        }
    }
}
