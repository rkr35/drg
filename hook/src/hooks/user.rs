use common::{self, EClassCastFlags, FFrame, UFunction, UObject};
use core::ffi::c_void;
use core::mem;
use sdk::CoreUObject::{LinearColor, Vector2D};
use sdk::Engine::{Canvas, GameViewportClient};

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
    const WIDTH: f32 = 1024.0;
    const HEIGHT: f32 = 768.0;

    const SIZE: f32 = 100.0;

    let position = Vector2D {
        X: WIDTH / 2.0 - SIZE / 2.0,
        Y: HEIGHT / 2.0 - SIZE / 2.0,
    };

    let size = Vector2D { X: SIZE, Y: SIZE };

    let thickness = 1.0;

    let color = LinearColor {
        R: 0.0,
        G: 1.0,
        B: 1.0,
        A: 1.0,
    };

    Canvas::K2_DrawBox(canvas, position, size, thickness, color);

    type DrawTransition = unsafe extern "C" fn(*mut GameViewportClient, *mut Canvas);
    let original = mem::transmute::<*const c_void, DrawTransition>(crate::hooks::DRAW_TRANSITION);
    original(game_viewport_client, canvas);
}
