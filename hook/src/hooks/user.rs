use crate::hooks::{PROCESS_EVENT, DRAW_TRANSITION};
use common::{self, EClassCastFlags, UFunction, UObject};
use core::ffi::c_void;
use core::mem;
use sdk::Engine::{Canvas, GameViewportClient};

pub unsafe extern "C" fn my_process_event(
    object: *mut UObject,
    function: *mut UFunction,
    parameters: *mut c_void,
) {
    type ProcessEvent = unsafe extern "C" fn (*mut UObject, *mut UFunction, *mut c_void);

    // BP_EngineerCharacter_C /Game/Maps/SpaceRig/LVL_SpaceRig.LVL_SpaceRig.PersistentLevel.BP_EngineerCharacter_C_2147480000 Function /Game/Character/BP_PlayerCharacter.BP_PlayerCharacter_C.InpAxisKeyEvt_MouseX_K2Node_InputAxisKeyEvent_0
    if (*object).fast_is(EClassCastFlags::CASTCLASS_APawn) {
        common::log!("{} {}", *object, *function);
    }

    let original = mem::transmute::<*const c_void, ProcessEvent>(PROCESS_EVENT);
    original(object, function, parameters);
}

pub unsafe extern "C" fn my_draw_transition(
    game_viewport_client: *mut GameViewportClient,
    canvas: *mut Canvas,
) {
    type DrawTransition = unsafe extern "C" fn(*mut GameViewportClient, *mut Canvas);
    let original = mem::transmute::<*const c_void, DrawTransition>(DRAW_TRANSITION);
    original(game_viewport_client, canvas);
}
