use r2g_mlua::prelude::*;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum RunContext {
    Legacy,
    Server,
    Client,
    Plugin,
}

from_lua_copy_impl!(RunContext);

impl LuaUserData for RunContext {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method("__tostring", |_, this, ()| Ok(String::from(match *this {
            Self::Legacy => "RunContext.Legacy",
            Self::Server => "RunContext.Server",
            Self::Client => "RunContext.Client",
            Self::Plugin => "RunContext.Plugin"
        })));
    }
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field("__subtype", "EnumItem");
    }
}