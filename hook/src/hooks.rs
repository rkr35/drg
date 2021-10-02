use common::{win, UFunction, UObject};
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
    FindStatic(&'static str),
}

pub struct Hooks {
    _draw_transition: Patch<*const c_void>,
    _process_event: Detour<6>,
    _function_invoke: Detour<5>,
    _process_remote_function_for_channel: Detour<7>,
}

impl Hooks {
    pub unsafe fn new(module: &win::Module) -> Result<Self, Error> {
        Self::find_statics()?;

        Ok(Self {
            _draw_transition: {
                const VTABLE_INDEX: usize = 0x310 / 8;
                let address = (*(*crate::GEngine).GameViewport.cast::<UObject>()).vtable.add(VTABLE_INDEX);
                DRAW_TRANSITION = *address;
                Patch::new(address, user::my_draw_transition as *const c_void)
            },

            _process_event: Detour::new(module, &mut crate::PROCESS_EVENT, user::my_process_event as *const c_void)?,
            
            _function_invoke: Detour::new(module, &mut crate::FUNCTION_INVOKE, user::my_function_invoke as *const c_void)?,

            _process_remote_function_for_channel: Detour::new(module, &mut crate::PROCESS_REMOTE_FUNCTION_FOR_CHANNEL, user::my_process_remote_function_for_channel as *const c_void)?,
        })
    }

    unsafe fn find_statics() -> Result<(), Error> {
        Ok(())
    }

    unsafe fn find_function(s: &'static str) -> Result<*mut UFunction, Error> {
        let function = (*common::GUObjectArray).find_function(s);

        if function.is_null() {
            Err(Error::FindStatic(s))
        } else {
            Ok(function)
        }
    }
}
