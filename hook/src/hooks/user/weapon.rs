use common::UFunction;
use sdk::FSD::{AmmoCountWidget, AmmoDrivenWeapon, DoubleDrillItem, HitscanBaseComponent, Item, RandRange, ThrownGrenadeItem};

pub unsafe fn on_item_amount_changed(widget: *mut AmmoCountWidget) {
    use crate::hooks::*;

    let item = (*widget).Item.cast::<UObject>();

    if (*item).is(AMMO_DRIVEN_WEAPON) {
        let weapon = item.cast::<AmmoDrivenWeapon>();

        if (*weapon).AmmoCount < (*weapon).ClipSize {
            (*weapon).AmmoCount = (*weapon).ClipSize;
        }
    } else if (*item).is(DOUBLE_DRILL_ITEM) {
        let drill = item.cast::<DoubleDrillItem>();

        if (*drill).Fuel < 2.0 {
            (*drill).Fuel = 2.0;
        }
    }
}

pub unsafe fn on_item_equipped(item: *mut Item) {
    use crate::hooks::*;

    if item.is_null() {
        return;
    }
    
    let item = item.cast::<UObject>();

    if (*item).is(AMMO_DRIVEN_WEAPON) {
        let weapon = item.cast();

        no_recoil(weapon);

        let fire = (*weapon).WeaponFire.cast::<UObject>();
        
        if !fire.is_null() && (*fire).is(HITSCAN_BASE_COMPONENT) {
            no_spread(fire.cast());
        }
    } else if (*item).is(THROWN_GRENADE_ITEM) {
        let item = item.cast::<ThrownGrenadeItem>();
        (*item).Server_Resupply(1.0);
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
    function == SERVER_REGISTER_RICOCHET_HIT_DESTRUCTABLE ||
    function == SERVER_DAMAGE_TARGET
}
