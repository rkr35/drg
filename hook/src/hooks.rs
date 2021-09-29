use common::{self, win, UObject};
use core::ffi::c_void;
use core::mem::{self, ManuallyDrop};
use core::ptr;
use core::slice;

mod patch;
use patch::Patch;

mod user;

static mut PROCESS_EVENT: *mut c_void = ptr::null_mut();
static mut DRAW_TRANSITION: *const c_void = ptr::null();

#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    FindProcessEvent,
    NoCodeCave,
    CaveIsTooSmall(usize, usize),
}

pub struct CodeCave<const JMP_LEN: usize> {
    _jmp_to_hook: Patch<[u8; 12]>,
    _original_bytes: Patch<[u8; JMP_LEN]>,
    _jmp_to_original: Patch<[u8; 5]>,
}

impl<const JMP_LEN: usize> CodeCave<JMP_LEN> {
    pub unsafe fn new(code_cave: &mut [u8], original: *const u8, hook: *const c_void) -> Result<CodeCave<JMP_LEN>, Error> {
        let mut jmp_to_hook = [0x48, 0xB8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xE0];
        (&mut jmp_to_hook[2..10]).copy_from_slice(&(hook as usize).to_le_bytes());

        let mut original_bytes = [0; JMP_LEN];
        original_bytes.copy_from_slice(slice::from_raw_parts(original, JMP_LEN));

        let mut jmp_to_original = [0xE9, 0x00, 0x00, 0x00, 0x00];

        let total_patch_len = jmp_to_hook.len() + original_bytes.len() + jmp_to_original.len();

        if code_cave.len() < total_patch_len {
            return Err(Error::CaveIsTooSmall(code_cave.len(), total_patch_len));
        }

        (&mut jmp_to_original[1..]).copy_from_slice({
            let destination = original as usize + JMP_LEN;
            let source = code_cave.as_ptr() as usize + total_patch_len;
            let relative_distance = destination.wrapping_sub(source) as u32;
            &relative_distance.to_le_bytes()
        });

        let code_cave = code_cave.as_mut_ptr();

        Ok(CodeCave {
            _jmp_to_hook: Patch::new(code_cave.cast(), jmp_to_hook),
            _original_bytes: Patch::new(code_cave.add(jmp_to_hook.len()).cast(), original_bytes),
            _jmp_to_original: Patch::new(code_cave.add(jmp_to_hook.len() + original_bytes.len()).cast(), jmp_to_original),
        })
    }
}

pub struct Detour<const JMP_LEN: usize> {
    jmp: ManuallyDrop<Patch<[u8; JMP_LEN]>>,
    code_cave: ManuallyDrop<CodeCave<JMP_LEN>>,
}

impl<const JMP_LEN: usize> Detour<JMP_LEN> {
    pub unsafe fn new(module: &win::Module, original: *mut *mut c_void, hook: *const c_void) -> Result<Detour<JMP_LEN>, Error> {
        let code_cave = module.find_code_cave().ok_or(Error::NoCodeCave)?;

        let code_cave_patch = ManuallyDrop::new(CodeCave::new(
            code_cave, *original.cast(), hook
        )?);

        // There's something to be desired about this variable name...
        let original_original = *original;

        // TODO(unhook): Restore to original address.
        *original = code_cave.as_mut_ptr().add(12).cast();

        let jmp = ManuallyDrop::new(Patch::new(
            original_original.cast(),
            Self::create_jmp_patch(code_cave, original_original),
        ));

        Ok(Detour {
            jmp,
            code_cave: code_cave_patch,
        })
    }

    unsafe fn create_jmp_patch(code_cave: &[u8], original: *const c_void) -> [u8; JMP_LEN] {
        let mut patch = [0; JMP_LEN];

        // jmp code_cave
        patch[0] = 0xE9;

        (&mut patch[1..5]).copy_from_slice({
            let destination = code_cave.as_ptr() as usize;
            let source = original as usize + 5;
            let relative_distance = destination.wrapping_sub(source) as u32;
            &relative_distance.to_le_bytes()
        });

        // no-ops to patch cleaved instructions
        (&mut patch[5..]).fill(0x90);

        patch
    }
}

impl<const JMP_LEN: usize> Drop for Detour<JMP_LEN> {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.jmp);
            // Before we destroy the code cave, give the CPU time to exit the cave.
            win::Sleep(100);
            ManuallyDrop::drop(&mut self.code_cave);
        }
    }
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
    _detour: Detour<6>,
}

impl ProcessEventHook {
    pub unsafe fn new(module: &win::Module) -> Result<ProcessEventHook, Error> {
        const PATTERN: [Option<u8>; 19] = [Some(0x40), Some(0x55), Some(0x56), Some(0x57), Some(0x41), Some(0x54), Some(0x41), Some(0x55), Some(0x41), Some(0x56), Some(0x41), Some(0x57), Some(0x48), Some(0x81), Some(0xEC), Some(0xF0), Some(0x00), Some(0x00), Some(0x00)];

        PROCESS_EVENT = module.find_mut(&PATTERN).ok_or(Error::FindProcessEvent)?;

        Ok(ProcessEventHook {
            _detour: Detour::new(module, &mut PROCESS_EVENT, user::my_process_event as *const c_void)?,
        })
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
