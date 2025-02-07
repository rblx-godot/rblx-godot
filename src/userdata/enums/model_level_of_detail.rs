use rblx_godot_derive::lua_enum;

#[lua_enum]
pub enum ModelLevelOfDetail {
    Automatic,
    StreamingMesh,
    Disabled,
}
