use common::{win, FNativeFuncPtr, UClass, UFunction, UObject};
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
static mut ON_ITEM_AMOUNT_CHANGED: MaybeUninit<FNativeFuncPtr> = MaybeUninit::uninit();

static mut AMMO_DRIVEN_WEAPON: *const UClass = ptr::null();

static mut SERVER_REGISTER_HIT: *mut UFunction = ptr::null_mut();
static mut SERVER_REGISTER_HIT_MULTI: *mut UFunction = ptr::null_mut();
static mut SERVER_DAMAGE_TARGET: *mut UFunction = ptr::null_mut();
static mut SERVER_REGISTER_HIT_TERRAIN: *mut UFunction = ptr::null_mut();
static mut SERVER_REGISTER_HIT_DESTRUCTABLE: *mut UFunction = ptr::null_mut();
static mut SERVER_REGISTER_RICOCHET_HIT: *mut UFunction = ptr::null_mut();
static mut SERVER_REGISTER_RICOCHET_HIT_TERRAIN: *mut UFunction = ptr::null_mut();
static mut SERVER_REGISTER_RICOCHET_HIT_DESTRUCTABLE: *mut UFunction = ptr::null_mut();

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
    _on_item_amount_changed: UFunctionHook,
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
            _on_item_amount_changed: UFunctionHook::new("Function /Script/FSD.AmmoCountWidget.OnItemAmountChanged", ON_ITEM_AMOUNT_CHANGED.as_mut_ptr(), user::my_on_item_amount_changed)?,
        })
    }

    unsafe fn find_statics() -> Result<(), Error> {
        AMMO_DRIVEN_WEAPON = find("Class /Script/FSD.AmmoDrivenWeapon")?.cast();

        SERVER_REGISTER_HIT = find("Function /Script/FSD.HitscanComponent.Server_RegisterHit")?.cast();
        SERVER_REGISTER_HIT_MULTI = find("Function /Script/FSD.MultiHitscanComponent.Server_RegisterHit")?.cast();
        SERVER_REGISTER_HIT_TERRAIN = find("Function /Script/FSD.HitscanComponent.Server_RegisterHit_Terrain")?.cast();
        SERVER_REGISTER_HIT_DESTRUCTABLE = find("Function /Script/FSD.HitscanComponent.Server_RegisterHit_Destructable")?.cast();
        SERVER_REGISTER_RICOCHET_HIT = find("Function /Script/FSD.HitscanComponent.Server_RegisterRicochetHit")?.cast();
        SERVER_REGISTER_RICOCHET_HIT_TERRAIN = find("Function /Script/FSD.HitscanComponent.Server_RegisterRicochetHit_Terrain")?.cast();
        SERVER_REGISTER_RICOCHET_HIT_DESTRUCTABLE = find("Function /Script/FSD.HitscanComponent.Server_RegisterRicochetHit_Destructable")?.cast();
        SERVER_DAMAGE_TARGET = find("Function /Script/FSD.PickaxeItem.Server_DamageTarget")?.cast();
        Ok(())
    }
}

impl Drop for Hooks {
    fn drop(&mut self) {
        unsafe { 
            for &function in user::SEEN_FUNCTIONS.iter() {
                (*function).seen_count = 0;
            }
        }
    }
}

struct UFunctionHook {
    function: *mut UFunction,
    original: FNativeFuncPtr,
}

impl UFunctionHook {
    pub unsafe fn new(f: &'static str, where_to_place_original: *mut FNativeFuncPtr, hook: FNativeFuncPtr) -> Result<UFunctionHook, Error> {
        let function = find(f)?.cast::<UFunction>();
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

unsafe fn find(s: &'static str) -> Result<*mut UObject, Error> {
    (*common::GUObjectArray).find(s).map_err(|_| Error::FindStatic(s))
}