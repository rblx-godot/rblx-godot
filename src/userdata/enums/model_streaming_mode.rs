use r2g_mlua::prelude::*;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum ModelStreamingMode {
    Default,
    Atomic,
    Persistent,
    PersistentPerPlayer,
    Nonatomic,
}

from_lua_copy_impl!(ModelStreamingMode);

impl LuaUserData for ModelStreamingMode {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method("__tostring", |_, this, ()| {
            Ok(String::from(match *this {
                Self::Default => "ModelStreamingMode.Default",
                Self::Atomic => "ModelStreamingMode.Atomic",
                Self::Persistent => "ModelStreamingMode.Persistent",
                Self::PersistentPerPlayer => "ModelStreamingMode.PersistentPerPlayer",
                Self::Nonatomic => "ModelStreamingMode.Nonatomic",
            }))
        });
    }
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field("__subtype", "EnumItem");
    }
}
