use rblx_godot_derive::lua_enum;

#[lua_enum(default=Enabled)]
pub enum ReplicateInstanceDestroySetting {
    Default,
    Disabled,
    Enabled,
}