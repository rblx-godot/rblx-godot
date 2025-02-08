use rblx_godot_derive::lua_enum;

#[lua_enum(default=Disabled)]
pub enum MeshPartHeadsAndAccessories {
    Default,
    Disabled,
    Enabled
}