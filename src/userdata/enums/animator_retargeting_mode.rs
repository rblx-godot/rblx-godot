use rblx_godot_derive::lua_enum;

#[lua_enum(default=Disabled)]
pub enum AnimatorRetargetingMode {
    Default,
    Disabled,
    Enabled
}