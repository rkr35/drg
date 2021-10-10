use common::UFunction;
use sdk::FSD::{AmmoCountWidget, AmmoDrivenWeapon, HitscanBaseComponent, Item, RandRange};

pub unsafe fn on_item_amount_changed(widget: *mut AmmoCountWidget) {
    use crate::hooks::*;

    let character = (*widget).Character;
    let inventory = (*character).InventoryComponent;
    (*inventory).Flares = 4;
    
    let item = (*widget).Item;

    if (*item.cast::<UObject>()).is(AMMO_DRIVEN_WEAPON) {
        replenish_ammo(item.cast());
    }
}

pub unsafe fn on_item_equipped(item: *mut Item) {
    use crate::hooks::*;

    if (*item.cast::<UObject>()).is(AMMO_DRIVEN_WEAPON) {
        no_recoil(item.cast());
    }
}

pub unsafe fn no_spread(hitscan: *mut HitscanBaseComponent) {
    (*hitscan).SpreadPerShot = 0.0;
    (*hitscan).MinSpread = 0.0;
    (*hitscan).MaxSpread = 0.0;
    (*hitscan).MinSpreadWhenMoving = 0.0;
    (*hitscan).MinSpreadWhenSprinting = 0.0;
    (*hitscan).VerticalSpreadMultiplier = 0.0;
    (*hitscan).HorizontalSpredMultiplier = 0.0;
    (*hitscan).MaxVerticalSpread = 0.0;
    (*hitscan).MaxHorizontalSpread = 0.0;
}

pub unsafe fn no_recoil(weapon: *mut AmmoDrivenWeapon) {
    const ZERO: RandRange = RandRange { Min: 0.0, Max: 0.0 };
    (*weapon).RecoilSettings.RecoilRoll = ZERO;
    (*weapon).RecoilSettings.RecoilPitch = ZERO;
    (*weapon).RecoilSettings.RecoilYaw = ZERO;
}

pub unsafe fn is_server_register_hit(function: *mut UFunction) -> bool {
    use crate::hooks::*;
    function == SERVER_REGISTER_HIT || 
    function == SERVER_REGISTER_HIT_MULTI ||
    function == SERVER_REGISTER_HIT_TERRAIN ||
    function == SERVER_REGISTER_HIT_DESTRUCTABLE ||
    function == SERVER_REGISTER_RICOCHET_HIT ||
    function == SERVER_REGISTER_RICOCHET_HIT_TERRAIN ||
    function == SERVER_REGISTER_RICOCHET_HIT_DESTRUCTABLE
}

pub unsafe fn replenish_ammo(weapon: *mut AmmoDrivenWeapon) {
    (*weapon).ClipCount = (*weapon).ClipSize;
    (*weapon).AmmoCount = 2 * (*weapon).ClipSize;
}