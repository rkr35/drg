use common::{self, win, EClassCastFlags, List, UFunction, UObject};
use core::ffi::c_void;
use core::mem::{self, ManuallyDrop};
use core::ptr;
use core::slice;
use sdk::Engine::{Actor, Canvas, GameViewportClient};

mod patch;
use patch::Patch;

#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    FindProcessEvent,
    NoCodeCave,
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
    code_cave: ManuallyDrop<Patch<[u8; 31]>>,
}

impl Drop for ProcessEventHook {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.jmp);
            // Before we destroy the code cave, give the CPU time to exit the cave.
            win::Sleep(100);
            ManuallyDrop::drop(&mut self.code_cave);

            for &function in RESET_THESE_SEEN_COUNTS.iter() {
                (*function).seen_count = 0;
            }
        }
    }
}

impl ProcessEventHook {
    pub unsafe fn new(module: &win::Module) -> Result<ProcessEventHook, Error> {
        let process_event = module
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
            my_process_event is at {}",
            module.start(),
            module.size(),
            code_cave.as_ptr() as usize,
            cave_size,
            my_process_event as usize,
        );

        let code_cave_patch = {
            let mut patch = [
                0x51, // push rcx
                0x52, // push rdx
                0x41, 0x50, // push r8
                0x48, 0xB8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // mov rax, my_process_event (need to fill in)
                0xFF, 0xD0, // call rax
                0x41, 0x58, // pop r8
                0x5A, // pop rdx
                0x59, // pop rcx
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // first six bytes of ProcessEvent (need to fill in)
                0xE9, 0x00, 0x00, 0x00, 0x00, // jmp ProcessEvent+6 (need to fill in)
            ];

            // mov rax, my_process_event
            (&mut patch[6..6 + mem::size_of::<usize>()])
                .copy_from_slice(&(my_process_event as usize).to_le_bytes());

            // first six bytes of ProcessEvent
            let first_six_process_event_bytes = slice::from_raw_parts(process_event, 6);
            (&mut patch[20..20 + first_six_process_event_bytes.len()])
                .copy_from_slice(first_six_process_event_bytes);

            // jmp ProcessEvent+6
            let patch_len = patch.len();
            (&mut patch[27..27 + mem::size_of::<u32>()]).copy_from_slice({
                let destination = process_event as usize + first_six_process_event_bytes.len();
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
            (&mut patch[1..1 + mem::size_of::<u32>()])
                .copy_from_slice(&relative_distance.to_le_bytes());

            patch
        };

        Ok(ProcessEventHook {
            jmp: ManuallyDrop::new(Patch::new(process_event.cast(), jmp_patch)),
            code_cave: ManuallyDrop::new(Patch::new(
                code_cave.as_mut_ptr().cast(),
                code_cave_patch,
            )),
        })
    }
}

static mut RESET_THESE_SEEN_COUNTS: List<*mut UFunction, 4096> = List::new();

unsafe extern "C" fn my_process_event(
    object: *mut UObject,
    function: *mut UFunction,
    _parameters: *mut c_void,
) {
    const MAX_PRINTS: u32 = 1;

    let seen_count = (*function).seen_count;

    if seen_count == 0 && RESET_THESE_SEEN_COUNTS.push(function).is_err() {
        common::log!("Warning: RESET_THESE_SEEN_COUNTS reached its max capacity of {}. We won't print any more unseen UFunctions.", RESET_THESE_SEEN_COUNTS.capacity());
        return;
    }

    if seen_count < MAX_PRINTS {
        (*function).seen_count += 1;

        let is_actor = (*object).fast_is(EClassCastFlags::CASTCLASS_AActor);

        common::log!(
            "{}{}\n\t{}",
            if is_actor { "\n" } else { "" },
            (*object).name(),
            *function
        );

        if is_actor {
            let mut owner = (*object.cast::<Actor>()).Owner;

            while !owner.is_null() {
                common::log!("owned by\n\t{}", (*owner.cast::<UObject>()).name());
                owner = (*owner).Owner;
            }

            common::log!();
        }
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
        ORIGINAL_DRAW_TRANSITION = *address;
        Self {
            _patch: Patch::new(address, my_draw_transition as *const c_void),
        }
    }
}

static mut ORIGINAL_DRAW_TRANSITION: *const c_void = ptr::null();

unsafe extern "C" fn my_draw_transition(
    game_viewport_client: *mut GameViewportClient,
    canvas: *mut Canvas,
) {
    type DrawTransition = unsafe extern "C" fn(*mut GameViewportClient, *mut Canvas);
    let original = mem::transmute::<*const c_void, DrawTransition>(ORIGINAL_DRAW_TRANSITION);
    original(game_viewport_client, canvas);
}
