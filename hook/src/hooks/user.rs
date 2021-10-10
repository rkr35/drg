use common::{self, FFrame, List, UFunction, UObject};
use core::ffi::c_void;
use core::mem;
use sdk::Engine::{Canvas, GameViewportClient};

mod weapon;

pub unsafe extern "C" fn my_process_remote_function_for_channel(net_driver: *mut c_void, actor_channel: *mut c_void, class_cache: *mut c_void, field_cache: *mut c_void, object: *mut UObject, net_connection: *mut c_void, function: *mut UFunction, parms: *mut c_void, out_params: *mut c_void, stack: *mut FFrame, is_server: bool, send_policy: i32) {
    type ProcessRemoteFunctionForChannel = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void, *mut UObject, *mut c_void, *mut UFunction, *mut c_void, *mut c_void, *mut FFrame, bool, i32);
    let original = mem::transmute::<*const c_void, ProcessRemoteFunctionForChannel>(crate::PROCESS_REMOTE_FUNCTION_FOR_CHANNEL);

    if weapon::is_server_register_hit(function) {
        weapon::no_spread(object.cast());

        // for _ in 0..4 {
        //     original(net_driver, actor_channel, class_cache, field_cache, object, net_connection, function, parms, out_params, stack, is_server, send_policy);
        // }
    } else if function == super::SERVER_DAMAGE_TARGET {
        for _ in 0..4 {
            original(net_driver, actor_channel, class_cache, field_cache, object, net_connection, function, parms, out_params, stack, is_server, send_policy);
        }
    }

    original(net_driver, actor_channel, class_cache, field_cache, object, net_connection, function, parms, out_params, stack, is_server, send_policy);
}

pub unsafe extern "C" fn my_function_invoke(function: *mut UFunction, object: *mut UObject, stack: *mut FFrame, result: *mut c_void) {
    type FunctionInvoke = unsafe extern "C" fn(*mut UFunction, *mut UObject, *mut FFrame, *mut c_void);
    let original = mem::transmute::<*const c_void, FunctionInvoke>(crate::FUNCTION_INVOKE);
    original(function, object, stack, result);
}

pub unsafe extern "C" fn my_draw_transition(game_viewport_client: *mut GameViewportClient, canvas: *mut Canvas) {
    type DrawTransition = unsafe extern "C" fn(*mut GameViewportClient, *mut Canvas);
    let original = mem::transmute::<*const c_void, DrawTransition>(super::DRAW_TRANSITION);
    original(game_viewport_client, canvas);
}

pub unsafe extern "C" fn my_on_item_amount_changed(context: *mut UObject, stack: *mut FFrame, result: *mut c_void) {
    weapon::on_item_amount_changed(context.cast());
    (*super::ON_ITEM_AMOUNT_CHANGED.as_ptr())(context, stack, result);
}

pub unsafe extern "C" fn my_get_item_name(context: *mut UObject, stack: *mut FFrame, result: *mut c_void) {
    weapon::on_item_equipped(context.cast());
    (*super::GET_ITEM_NAME.as_ptr())(context, stack, result);
}

pub static mut SEEN_FUNCTIONS: List<*mut UFunction, 4096> = List::new();

#[allow(dead_code)]
unsafe fn print_if_unseen(object: *mut UObject, function: *mut UFunction) {
    if (*function).seen_count == 0 {
        if SEEN_FUNCTIONS.push(function).is_ok() {
            (*function).seen_count = 1;
            common::log!("{} {}", *object, *function);
        } else {
            common::log!("SEEN_FUNCTIONS is full. Increase its capacity.");
        }
    }
}