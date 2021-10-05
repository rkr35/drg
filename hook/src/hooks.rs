use common::{win, FNativeFuncPtr, UFunction, UObject};
use core::ffi::c_void;
use core::mem::MaybeUninit;
use core::ptr;

mod detour;
use detour::Detour;

mod patch;
use patch::Patch;

mod user;

static mut DRAW_TRANSITION: *const c_void = ptr::null();
static mut IS_LOCALLY_CONTROLLED: MaybeUninit<FNativeFuncPtr> = MaybeUninit::uninit();

#[derive(macros::NoPanicErrorDebug)]
pub enum Error {
    Detour(#[from] detour::Error),
    FindStatic(&'static str),
}

pub struct Hooks {
    _draw_transition: Patch<*const c_void>,
    
    _function_invoke: Detour<5>,
    _process_remote_function_for_channel: Detour<7>,
    
    _is_locally_controlled: UFunctionHook,
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

            _function_invoke: Detour::new(module, &mut crate::FUNCTION_INVOKE, user::my_function_invoke as *const c_void)?,

            _process_remote_function_for_channel: Detour::new(module, &mut crate::PROCESS_REMOTE_FUNCTION_FOR_CHANNEL, user::my_process_remote_function_for_channel as *const c_void)?,

            _is_locally_controlled: UFunctionHook::new("Function /Script/Engine.Controller.IsLocalController", IS_LOCALLY_CONTROLLED.as_mut_ptr(), user::my_locally_controlled)?,
        })
    }

    unsafe fn find_statics() -> Result<(), Error> {
        Ok(())
    }
}

struct UFunctionHook {
    function: *mut UFunction,
    original: FNativeFuncPtr,
}

impl UFunctionHook {
    pub unsafe fn new(f: &'static str, where_to_place_original: *mut FNativeFuncPtr, hook: FNativeFuncPtr) -> Result<UFunctionHook, Error> {
        let function = find_function(f)?;
        let original = (*function).Func;
        *where_to_place_original = original;
        (*function).Func = hook;
        Ok(UFunctionHook {
            function,
            original,
        })
    }
}

impl Drop for UFunctionHook {
    fn drop(&mut self) {
        unsafe {
            (*self.function).Func = self.original;
        }
    }
}

unsafe fn find_function(s: &'static str) -> Result<*mut UFunction, Error> {
    let function = (*common::GUObjectArray).find_function(s);

    if function.is_null() {
        Err(Error::FindStatic(s))
    } else {
        Ok(function)
    }
}