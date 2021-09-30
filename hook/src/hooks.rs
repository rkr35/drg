use common::{win, UObject};
use core::ffi::c_void;
use core::ptr;

mod detour;
use detour::Detour;

mod patch;
use patch::Patch;

mod user;

static mut DRAW_TRANSITION: *const c_void = ptr::null();

#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    Detour(#[from] detour::Error),
}

pub struct Hooks {
    _process_event: Detour<6>,
    _draw_transition: Patch<*const c_void>,
}

impl Hooks {
    pub unsafe fn new(module: &win::Module) -> Result<Self, Error> {
        Ok(Self {
            _process_event: Detour::new(module, &mut crate::PROCESS_EVENT, user::my_process_event as *const c_void)?,
            
            _draw_transition: {
                const VTABLE_INDEX: usize = 0x310 / 8;
                let address = (*(*crate::GEngine).GameViewport.cast::<UObject>()).vtable.add(VTABLE_INDEX);
                DRAW_TRANSITION = *address;
                Patch::new(address, user::my_draw_transition as *const c_void)
            },
        })
    }
}
