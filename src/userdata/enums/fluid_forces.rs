use rblx_godot_derive::lua_enum;

#[lua_enum(default=Default)]
pub enum FluidForces {
    Default,
    Experimental
}