use common::{self, EClassCastFlags, FFrame, UFunction, UObject};
use core::ffi::c_void;
use core::mem;
use sdk::CoreUObject::{LinearColor, Vector2D};
use sdk::Engine::{Canvas, GameViewportClient};

pub unsafe extern "C" fn my_process_remote_function_for_channel(net_driver: *mut c_void, actor_channel: *mut c_void, class_cache: *mut c_void, field_cache: *mut c_void, object: *mut UObject, net_connection: *mut c_void, function: *mut UFunction, parms: *mut c_void, out_params: *mut c_void, stack: *mut FFrame, is_server: bool, send_policy: i32) {
    const IGNORE: [&str; 9] = ["ServerMove", "ServerMoveOld", "ServerUpdateCamera", "ServerMoveDual", "ServerMoveNoBase", "Server_SetFallVelocity", "Server_UpdateTarget", "ClientAckGoodMove", "ServerSetSpectatorLocation"];
    
    let function_name = (*function).name();
    let should_ignore = IGNORE.iter().any(|&s| function_name == s);

    if !should_ignore {
        common::log!("{} {}", *object, *function);
    }

    type ProcessRemoteFunctionForChannel = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void, *mut UObject, *mut c_void, *mut UFunction, *mut c_void, *mut c_void, *mut FFrame, bool, i32);
    let original = mem::transmute::<*const c_void, ProcessRemoteFunctionForChannel>(crate::PROCESS_REMOTE_FUNCTION_FOR_CHANNEL);
    original(net_driver, actor_channel, class_cache, field_cache, object, net_connection, function, parms, out_params, stack, is_server, send_policy);
}

pub unsafe extern "C" fn my_function_invoke(
    function: *mut UFunction,
    object: *mut UObject,
    stack: *mut FFrame,
    result: *mut c_void,
) {
    type FunctionInvoke = unsafe extern "C" fn(*mut UFunction, *mut UObject, *mut FFrame, *mut c_void);
    let original = mem::transmute::<*const c_void, FunctionInvoke>(crate::FUNCTION_INVOKE);
    original(function, object, stack, result);
}

pub unsafe extern "C" fn my_process_event(
    object: *mut UObject,
    function: *mut UFunction,
    parameters: *mut c_void,
) {
    type ProcessEvent = unsafe extern "C" fn(*mut UObject, *mut UFunction, *mut c_void);
    let original = mem::transmute::<*const c_void, ProcessEvent>(crate::PROCESS_EVENT);
    original(object, function, parameters);
}

pub unsafe extern "C" fn my_draw_transition(
    game_viewport_client: *mut GameViewportClient,
    canvas: *mut Canvas,
) {
    type DrawTransition = unsafe extern "C" fn(*mut GameViewportClient, *mut Canvas);
    let original = mem::transmute::<*const c_void, DrawTransition>(crate::hooks::DRAW_TRANSITION);
    original(game_viewport_client, canvas);
}
