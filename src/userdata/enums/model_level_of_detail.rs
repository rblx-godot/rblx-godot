use rblx_godot_derive::lua_enum;

#[lua_enum(default=Disabled, default_name=Automatic)]
pub enum ModelLevelOfDetail {
    Automatic,
    StreamingMesh,
    Disabled,
}
