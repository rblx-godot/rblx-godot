use rblx_godot_derive::lua_enum;

#[lua_enum(default=Immediate)]
pub enum SignalBehavior {
    Default,
    Immediate,
    Deferred,
    AncestryDeferred
}