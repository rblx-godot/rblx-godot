use rblx_godot_derive::lua_enum;

#[lua_enum]
pub enum RotationOrder {
    XYZ,
    XZY,
    YZX,
    YXZ,
    ZXY,
    ZYX,
}
