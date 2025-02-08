use rblx_godot_derive::lua_enum;

#[lua_enum(default=Fixed)]
pub enum PhysicsSteppingMethod {
    Default,
    Fixed,
    Adaptive,
}