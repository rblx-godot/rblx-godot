use r2g_mlua::prelude::*;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum ModelLevelOfDetail {
    Automatic,
    StreamingMesh,
    Disabled,
}

from_lua_copy_impl!(ModelLevelOfDetail);

impl LuaUserData for ModelLevelOfDetail {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method("__tostring", |_, this, ()| {
            Ok(String::from(match *this {
                Self::Automatic => "ModelLevelOfDetail.Automatic",
                Self::StreamingMesh => "ModelLevelOfDetail.StreamingMesh",
                Self::Disabled => "ModelLevelOfDetail.Disabled",
            }))
        });
    }
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field("__subtype", "EnumItem");
    }
}
