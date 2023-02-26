use crate::hooks::Patch;
use common::win;
use core::ffi::c_void;
use core::mem::ManuallyDrop;
use core::slice;

#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    NoCodeCave,
    JmpLenIsSmallerThanFiveBytes,
    CaveIsTooSmall(usize, usize),
}

pub const JMP_TO_HOOK_LEN: usize = 12;
pub const JMP_TO_ORIG_LEN: usize = 5;

pub struct Detour<const JMP_LEN: usize> {
    jmp: ManuallyDrop<Patch<[u8; JMP_LEN]>>,
    code_cave: ManuallyDrop<CodeCave<JMP_LEN>>,
}

impl<const JMP_LEN: usize> Detour<JMP_LEN> {
    pub unsafe fn new(
        module: &win::Module,
        original: *mut *mut c_void,
        hook: *const c_void,
    ) -> Result<Detour<JMP_LEN>, Error> {
        if JMP_LEN < 5 {
            return Err(Error::JmpLenIsSmallerThanFiveBytes);
        }

        let code_cave = module
            .find_code_cave(
                *original.cast(),
                JMP_LEN + JMP_TO_HOOK_LEN + JMP_TO_ORIG_LEN,
            )
            .ok_or(Error::NoCodeCave)?;

        let code_cave_patch = ManuallyDrop::new(CodeCave::new(code_cave, *original.cast(), hook)?);

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
        let mut patch = [0x90; JMP_LEN];

        // jmp code_cave
        patch[0] = 0xE9;

        patch[1..5].copy_from_slice({
            let destination = code_cave.as_ptr() as usize;
            let source = original as usize + 5;
            let relative_distance = destination.wrapping_sub(source) as u32;
            &relative_distance.to_le_bytes()
        });

        patch
    }
}

impl<const JMP_LEN: usize> Drop for Detour<JMP_LEN> {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.jmp);
            // Before we destroy the code cave, give the CPU time to exit the cave.
            win::Sleep(10);
            ManuallyDrop::drop(&mut self.code_cave);
        }
    }
}

pub struct CodeCave<const JMP_LEN: usize> {
    _jmp_to_hook: Patch<[u8; JMP_TO_HOOK_LEN]>,
    _original_bytes: Patch<[u8; JMP_LEN]>,
    _jmp_to_original: Patch<[u8; JMP_TO_ORIG_LEN]>,
}

impl<const JMP_LEN: usize> CodeCave<JMP_LEN> {
    pub unsafe fn new(
        code_cave: &mut [u8],
        original: *const u8,
        hook: *const c_void,
    ) -> Result<CodeCave<JMP_LEN>, Error> {
        let mut jmp_to_hook = [
            0x48, 0xB8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xE0,
        ];
        jmp_to_hook[2..10].copy_from_slice(&(hook as usize).to_le_bytes());

        let mut original_bytes = [0; JMP_LEN];
        original_bytes.copy_from_slice(slice::from_raw_parts(original, JMP_LEN));

        let mut jmp_to_original = [0xE9, 0x00, 0x00, 0x00, 0x00];

        let total_patch_len = jmp_to_hook.len() + original_bytes.len() + jmp_to_original.len();

        if code_cave.len() < total_patch_len {
            return Err(Error::CaveIsTooSmall(code_cave.len(), total_patch_len));
        }

        jmp_to_original[1..].copy_from_slice({
            let destination = original as usize + JMP_LEN;
            let source = code_cave.as_ptr() as usize + total_patch_len;
            let relative_distance = destination.wrapping_sub(source) as u32;
            &relative_distance.to_le_bytes()
        });

        let code_cave = code_cave.as_mut_ptr();

        Ok(CodeCave {
            _jmp_to_hook: Patch::new(code_cave.cast(), jmp_to_hook),
            _original_bytes: Patch::new(code_cave.add(jmp_to_hook.len()).cast(), original_bytes),
            _jmp_to_original: Patch::new(
                code_cave
                    .add(jmp_to_hook.len() + original_bytes.len())
                    .cast(),
                jmp_to_original,
            ),
        })
    }
}
