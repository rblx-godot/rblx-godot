use rblx_godot_derive::lua_enum;

#[lua_enum]
pub enum Technology {
    Voxel,
    ShadowMap,
    Future,
    Future2,
    Future3
}