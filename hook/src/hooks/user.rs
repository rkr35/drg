use common::{self, EClassCastFlags, FFrame, List, UFunction, UObject};
use common::win::random;
use core::ffi::c_void;
use core::mem;
use sdk::Engine::{Actor, LocalPlayer, World};
use sdk::FSD::{FSDCheatManager, FSDPlayerController, FSDUserWidget, PlayerCharacter};

mod weapon;
mod pawn;
use pawn::Pawns;

mod render;

pub static mut SEEN_FUNCTIONS: List<*mut UFunction, 4096> = List::new();
pub static mut PAWNS: Pawns = Pawns::new();

pub struct OneTimeModifications;

impl OneTimeModifications {
    pub unsafe fn new() -> Self {
        render::remove_lighting();
        Self
    }
}

impl Drop for OneTimeModifications {
    fn drop(&mut self) {
        unsafe {
            render::restore_lighting();
        }
    }
}
unsafe fn set_blank_name(controller: *mut FSDPlayerController) {
    const ZERO_WIDTH_SPACE: [u16; 2] = [0x200b, 0];
    (*controller).ServerChangeName(ZERO_WIDTH_SPACE.as_slice().into());
}

pub unsafe extern "C" fn my_process_remote_function_for_channel(net_driver: *mut c_void, actor_channel: *mut c_void, class_cache: *mut c_void, field_cache: *mut c_void, object: *mut UObject, net_connection: *mut c_void, function: *mut UFunction, parms: *mut c_void, out_params: *mut c_void, stack: *mut FFrame, is_server: bool, send_policy: i32) {
    type ProcessRemoteFunctionForChannel = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void, *mut UObject, *mut c_void, *mut UFunction, *mut c_void, *mut c_void, *mut FFrame, bool, i32);
    let original = mem::transmute::<*const c_void, ProcessRemoteFunctionForChannel>(crate::PROCESS_REMOTE_FUNCTION_FOR_CHANNEL);

    if weapon::is_server_register_hit(function) {
        for _ in 0..4 {
            original(net_driver, actor_channel, class_cache, field_cache, object, net_connection, function, parms, out_params, stack, is_server, send_policy);
        }
    } else if function == super::SERVER_SET_FALL_VELOCITY {
        #[allow(non_snake_case)]
        #[repr(C)]
        struct Parameters {
            Velocity: f32, 
        }

        let p = parms.cast::<Parameters>();
        (*p).Velocity = 0.0;
    } else if function == super::SERVER_SET_CONTROLLER_READY {
        set_blank_name(object.cast());
    } 

    original(net_driver, actor_channel, class_cache, field_cache, object, net_connection, function, parms, out_params, stack, is_server, send_policy);
}

pub unsafe extern "C" fn my_function_invoke(function: *mut UFunction, object: *mut UObject, stack: *mut FFrame, result: *mut c_void) {
    type FunctionInvoke = unsafe extern "C" fn(*mut UFunction, *mut UObject, *mut FFrame, *mut c_void);
    let original = mem::transmute::<*const c_void, FunctionInvoke>(crate::FUNCTION_INVOKE);
    original(function, object, stack, result);
}

pub unsafe extern "C" fn my_add_cheats(controller: *mut FSDPlayerController, _: bool) {
    type AddCheats = unsafe extern "C" fn(*mut FSDPlayerController, bool);
    let original = mem::transmute::<*const c_void, AddCheats>(crate::ADD_CHEATS);
    original(controller, true);
}

pub unsafe extern "C" fn my_on_item_amount_changed(context: *mut UObject, stack: *mut FFrame, result: *mut c_void) {
    weapon::on_item_amount_changed(context.cast());
    (*super::ON_ITEM_AMOUNT_CHANGED.as_ptr())(context, stack, result);
}

pub unsafe extern "C" fn my_get_item_name(context: *mut UObject, stack: *mut FFrame, result: *mut c_void) {
    weapon::on_item_equipped(context.cast());
    (*super::GET_ITEM_NAME.as_ptr())(context, stack, result);
}

pub unsafe extern "C" fn my_on_flare(context: *mut UObject, stack: *mut FFrame, result: *mut c_void) {
    let widget = context.cast::<FSDUserWidget>();
    let character = (*widget).Character;
    let inv = (*character).InventoryComponent;
    (*inv).FlareProductionTime = 0.0;
    (*super::ON_FLARE.as_ptr())(context, stack, result);
}

pub unsafe extern "C" fn my_on_keypress_insert(context: *mut UObject, stack: *mut FFrame, result: *mut c_void) {    
    let character = context.cast::<PlayerCharacter>();
    (*character).Server_EscapeFromGrabber();
    let health = (*character).HealthComponent;
    (*health).ToggleCanTakeDamage();
    set_blank_name((*character).Controller.cast());
    (*super::ON_KEYPRESS_INSERT.as_ptr())(context, stack, result);
}

pub unsafe extern "C" fn my_post_actor_construction(actor: *mut Actor) {
    type PostActorConstruction = unsafe extern "C" fn(*mut Actor);
    let original = mem::transmute::<*const c_void, PostActorConstruction>(crate::POST_ACTOR_CONSTRUCTION);
    original(actor);

    let obj = actor.cast::<UObject>();

    if (*obj).fast_is(EClassCastFlags::CASTCLASS_APawn) {
        if let Err(e) = PAWNS.add(obj.cast()) {
            common::log!("failed to add pawn {}: {:?}", *obj, e);
        }
    }
}

pub unsafe extern "C" fn my_destroy_actor(world: *mut World, actor: *mut Actor, net_force: bool, should_modify_level: bool) -> bool {
    let obj = actor.cast::<UObject>();

    if (*obj).fast_is(EClassCastFlags::CASTCLASS_APawn) {
        if let Err(e) = PAWNS.remove(obj.cast()) {
            common::log!("failed to remove pawn {}: {:?}", *obj, e)
        }
    }

    type DestroyActor = unsafe extern "C" fn (*mut World, *mut Actor, bool, bool) -> bool;
    let original = mem::transmute::<*const c_void, DestroyActor>(crate::DESTROY_ACTOR);
    original(world, actor, net_force, should_modify_level)
}

pub unsafe extern "C" fn my_route_end_play(actor: *mut Actor, end_play_reason: u32) {
    let obj = actor.cast::<UObject>();

    if (*obj).fast_is(EClassCastFlags::CASTCLASS_APawn) {
        if let Err(e) = PAWNS.remove(obj.cast()) {
            common::log!("failed to remove pawn {}: {:?}", *obj, e)
        }
    }

    type RouteEndPlay = unsafe extern "C" fn (*mut Actor, u32);
    let original = mem::transmute::<*const c_void, RouteEndPlay>(crate::ROUTE_END_PLAY);
    original(actor, end_play_reason);
}

// #[repr(C)]
// pub struct Id {
//     vtable: usize,
//     this: *mut Id,
//     magic: usize,
//     value: u64,
// }

// #[repr(C)]
// pub struct IdWrapper {
//     vtable: usize,
//     id: *mut Id,
//     magic: usize,
//     pad: [u8; 16],
// }

// pub unsafe extern "C" fn my_get_preferred_unique_net_id(local_player: *mut LocalPlayer, out_id: *mut IdWrapper) -> *mut IdWrapper {
//     type GetPreferredUniqueNetId = unsafe extern "C" fn (*mut LocalPlayer, *mut IdWrapper) -> *mut IdWrapper;
//     let original = mem::transmute::<*const c_void, GetPreferredUniqueNetId>(crate::GET_PREFERRED_UNIQUE_NET_ID);
//     original(local_player, out_id);

//     let old_id = (*(*out_id).id).value;

//     let new_id = {
//         const ID_WITHOUT_ACCOUNT: u64 = 76561197960265728;
//         let random_account = random::u32();
//         ID_WITHOUT_ACCOUNT | u64::from(random_account)
//     };

//     (*(*out_id).id).value = new_id;

//     common::log!("ID: {} -> {}", old_id, new_id);

//     out_id
// }

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

#[allow(dead_code)]
unsafe fn run_cheat_manager(character: *mut PlayerCharacter) {
    let controller = (*character).Controller.cast::<FSDPlayerController>();
    (*controller).EnableCheats();

    #[allow(unused_variables)]
    let cheat_manager = (*controller).CheatManager.cast::<FSDCheatManager>();

}