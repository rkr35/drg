use crate::hooks::{DRAW_TRANSITION, PROCESS_EVENT};
use common::{self, EClassCastFlags, UFunction, UObject};
use core::ffi::c_void;
use core::mem;
use sdk::CoreUObject::{LinearColor, Vector2D};
use sdk::Engine::{Canvas, GameViewportClient};

pub unsafe extern "C" fn my_process_event(
    object: *mut UObject,
    function: *mut UFunction,
    parameters: *mut c_void,
) {
    type ProcessEvent = unsafe extern "C" fn(*mut UObject, *mut UFunction, *mut c_void);
    let original = mem::transmute::<*const c_void, ProcessEvent>(PROCESS_EVENT);
    original(object, function, parameters);
}

pub unsafe extern "C" fn my_draw_transition(
    game_viewport_client: *mut GameViewportClient,
    canvas: *mut Canvas,
) {
    let position = Vector2D {
        X: 100.0,
        Y: 100.0,
    };

    let size = Vector2D {
        X: 200.0,
        Y: 50.0,
    };

    let thickness = 10.0;

    let color = LinearColor {
        R: 0.0,
        G: 1.0,
        B: 1.0,
        A: 1.0,
    };

    Canvas::K2_DrawBox(
        canvas,
        position,
        size,
        thickness,
        color,
    );

    type DrawTransition = unsafe extern "C" fn(*mut GameViewportClient, *mut Canvas);
    let original = mem::transmute::<*const c_void, DrawTransition>(DRAW_TRANSITION);
    original(game_viewport_client, canvas);
}
