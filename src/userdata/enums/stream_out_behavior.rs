use rblx_godot_derive::lua_enum;

#[lua_enum(default=LowMemory)]
pub enum StreamOutBehavior {
    Default,
    LowMemory,
    Opportunistic
}