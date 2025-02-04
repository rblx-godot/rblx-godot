use r2g_mlua::prelude::*;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum MessageType {
    MessageOutput,
    MessageInfo,
    MessageWarning,
    MessageError
}

from_lua_copy_impl!(MessageType);

impl LuaUserData for MessageType {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method("__tostring", |_, this, ()| Ok(String::from(match *this {
            Self::MessageOutput => "MessageType.MessageOutput",
            Self::MessageInfo => "MessageType.MessageInfo",
            Self::MessageWarning => "MessageType.MessageWarning",
            Self::MessageError => "MessageType.MessageError"
        })));
    }
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field("__subtype", "EnumItem");
    }
}