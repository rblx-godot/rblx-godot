use r2g_mlua::prelude::*;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum RotationOrder {
    XYZ,
    XZY,
    YZX,
    YXZ,
    ZXY,
    ZYX,
}

from_lua_copy_impl!(RotationOrder);

impl LuaUserData for RotationOrder {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method("__tostring", |_, this, ()| {
            Ok(String::from(match *this {
                Self::XYZ => "RotationOrder.XYZ",
                Self::XZY => "RotationOrder.XZY",
                Self::YZX => "RotationOrder.YZX",
                Self::YXZ => "RotationOrder.YXZ",
                Self::ZXY => "RotationOrder.ZXY",
                Self::ZYX => "RotationOrder.ZYX",
            }))
        });
    }
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field("__subtype", "EnumItem");
    }
}
