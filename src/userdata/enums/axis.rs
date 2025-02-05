use r2g_mlua::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Axis {
    X,
    Y,
    Z,
}

from_lua_copy_impl!(Axis);

impl LuaUserData for Axis {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method("__tostring", |_, this, ()| {
            Ok(String::from(match *this {
                Self::X => "Axis.X",
                Self::Y => "Axis.Y",
                Self::Z => "Axis.Z",
            }))
        });
    }
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field("__subtype", "EnumItem");
    }
}
