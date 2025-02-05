use r2g_mlua::prelude::*;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum NormalId {
    Right,
    Top,
    Back,
    Left,
    Bottom,
    Front,
}

from_lua_copy_impl!(NormalId);

impl LuaUserData for NormalId {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method("__tostring", |_, this, ()| {
            Ok(String::from(match *this {
                Self::Right => "NormalId.Right",
                Self::Top => "NormalId.Top",
                Self::Back => "NormalId.Back",
                Self::Left => "NormalId.Left",
                Self::Bottom => "NormalId.Bottom",
                Self::Front => "NormalId.Front",
            }))
        });
    }
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field("__subtype", "EnumItem");
    }
}
