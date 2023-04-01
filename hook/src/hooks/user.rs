use common::win::random;
use common::{self, EClassCastFlags, FFrame, List, UFunction, UObject};
use core::ffi::c_void;
use core::mem;
use sdk::Engine::{Actor, LocalPlayer};
use sdk::FSD::{FSDCheatManager, FSDPlayerController, PlayerCharacter};

mod pawn;
mod weapon;

mod render;

pub static mut SEEN_FUNCTIONS: List<*mut UFunction, 4096> = List::new();

pub struct OneTimeModifications;

impl OneTimeModifications {
    pub unsafe fn new() -> Self {
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

#[allow(dead_code)]
unsafe fn set_custom_name(controller: *mut FSDPlayerController) {
    const NAME: [u16; 5] = [0x6e, 0x6f, 0x6f, 0x62, 0];
    (*controller).ServerChangeName(NAME.as_slice().into());
}

pub unsafe extern "C" fn my_process_remote_function_for_channel(
    net_driver: *mut c_void,
    actor_channel: *mut c_void,
    class_cache: *mut c_void,
    field_cache: *mut c_void,
    object: *mut UObject,
    net_connection: *mut c_void,
    function: *mut UFunction,
    parms: *mut c_void,
    out_params: *mut c_void,
    stack: *mut FFrame,
    is_server: bool,
    send_policy: i32,
) {
    type ProcessRemoteFunctionForChannel = unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        *mut c_void,
        *mut c_void,
        *mut UObject,
        *mut c_void,
        *mut UFunction,
        *mut c_void,
        *mut c_void,
        *mut FFrame,
        bool,
        i32,
    );
    let original = mem::transmute::<*const c_void, ProcessRemoteFunctionForChannel>(
        crate::PROCESS_REMOTE_FUNCTION_FOR_CHANNEL,
    );

    if weapon::is_server_register_hit(function) {
        for _ in 0..2 {
            original(
                net_driver,
                actor_channel,
                class_cache,
                field_cache,
                object,
                net_connection,
                function,
                parms,
                out_params,
                stack,
                is_server,
                send_policy,
            );
        }
    }

    original(
        net_driver,
        actor_channel,
        class_cache,
        field_cache,
        object,
        net_connection,
        function,
        parms,
        out_params,
        stack,
        is_server,
        send_policy,
    );
}

// pub unsafe extern "C" fn my_function_invoke(
//     function: *mut UFunction,
//     object: *mut UObject,
//     stack: *mut FFrame,
//     result: *mut c_void,
// ) {
//     type FunctionInvoke =
//         unsafe extern "C" fn(*mut UFunction, *mut UObject, *mut FFrame, *mut c_void);
//     print_if_unseen(object, function);
//     let original = mem::transmute::<*const c_void, FunctionInvoke>(crate::FUNCTION_INVOKE);
//     original(function, object, stack, result);
// }

pub unsafe extern "C" fn my_add_cheats(controller: *mut FSDPlayerController, _: bool) {
    type AddCheats = unsafe extern "C" fn(*mut FSDPlayerController, bool);
    let original = mem::transmute::<*const c_void, AddCheats>(crate::ADD_CHEATS);
    original(controller, true);
}

pub unsafe extern "C" fn my_on_item_amount_changed(
    context: *mut UObject,
    stack: *mut FFrame,
    result: *mut c_void,
) {
    weapon::on_item_amount_changed(context.cast());
    (*super::ON_ITEM_AMOUNT_CHANGED.as_ptr())(context, stack, result);
}

pub unsafe extern "C" fn my_get_item_name(
    context: *mut UObject,
    stack: *mut FFrame,
    result: *mut c_void,
) {
    weapon::on_item_equipped(context.cast());
    (*super::GET_ITEM_NAME.as_ptr())(context, stack, result);
}

// pub unsafe extern "C" fn my_on_flare(
//     context: *mut UObject,
//     stack: *mut FFrame,
//     result: *mut c_void,
// ) {
//     let widget = context.cast::<FSDUserWidget>();
//     let character = (*widget).Character;
//     let inv = (*character).InventoryComponent;
//     (*inv).FlareProductionTime = 0.0;
//     (*super::ON_FLARE.as_ptr())(context, stack, result);
// }

pub unsafe extern "C" fn my_on_keypress_insert(
    context: *mut UObject,
    stack: *mut FFrame,
    result: *mut c_void,
) {
    let character = context.cast::<PlayerCharacter>();
    let health = (*character).HealthComponent;
    (*health).ToggleCanTakeDamage();
    (*super::ON_KEYPRESS_INSERT.as_ptr())(context, stack, result);
}

#[allow(dead_code)]
unsafe fn get_game_data() -> *mut sdk::FSD::GameData {
    let asset_manager = (*crate::GEngine)
        .AssetManager
        .cast::<sdk::FSD::FSDAssetManager>();

    if asset_manager.is_null() {
        core::ptr::null_mut()
    } else {
        (*asset_manager).GameData
    }
}

pub unsafe extern "C" fn my_on_keypress_delete(
    context: *mut UObject,
    stack: *mut FFrame,
    result: *mut c_void,
) {
    render::toggle_lighting();
    (*super::ON_KEYPRESS_DELETE.as_ptr())(context, stack, result);
}

#[allow(dead_code)]
pub unsafe extern "C" fn my_post_actor_construction(actor: *mut Actor) {
    type PostActorConstruction = unsafe extern "C" fn(*mut Actor);
    let original =
        mem::transmute::<*const c_void, PostActorConstruction>(crate::POST_ACTOR_CONSTRUCTION);
    original(actor);
    let obj = actor.cast::<UObject>();

    if (*obj).fast_is(EClassCastFlags::CASTCLASS_APawn) {
        pawn::set_outline(obj.cast())
    }
}

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

#[repr(C)]
pub struct Id {
    vtable: usize,
    this: *mut Id,
    magic: usize,
    value: u64,
}

#[repr(C)]
pub struct IdWrapper {
    vtable: usize,
    id: *mut Id,
    magic: usize,
    pad: [u8; 16],
}

#[allow(dead_code)]
pub unsafe extern "C" fn my_get_preferred_unique_net_id(
    local_player: *mut LocalPlayer,
    out_id: *mut IdWrapper,
) -> *mut IdWrapper {
    type GetPreferredUniqueNetId =
        unsafe extern "C" fn(*mut LocalPlayer, *mut IdWrapper) -> *mut IdWrapper;
    let original = mem::transmute::<*const c_void, GetPreferredUniqueNetId>(
        crate::GET_PREFERRED_UNIQUE_NET_ID,
    );
    original(local_player, out_id);

    let old_id = (*(*out_id).id).value;

    let new_id = {
        const ID_WITHOUT_ACCOUNT: u64 = 76561197960265728;
        let random_account = random::u32();
        ID_WITHOUT_ACCOUNT | u64::from(random_account)
    };

    (*(*out_id).id).value = new_id;

    common::log!("ID: {} -> {}", old_id, new_id);

    out_id
}
