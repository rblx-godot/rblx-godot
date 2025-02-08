use r2g_mlua::prelude::*;

use crate::core::InheritanceBase;
use crate::userdata::ManagedRBXScriptSignal;

pub trait IObject: InheritanceBase + Send + Sync {
    fn lua_get(&self, lua: &Lua, name: String) -> LuaResult<LuaValue>;
    fn get_class_name(&self) -> &'static str;
    fn get_property_changed_signal(&self, property: String) -> ManagedRBXScriptSignal;
    fn is_a(&self, class_name: &String) -> bool;
    fn get_changed_signal(&self) -> ManagedRBXScriptSignal;
}

impl PartialEq for dyn IObject {
    fn eq(&self, other: &Self) -> bool {
        (&raw const *self) == (&raw const *other)
    }
}
impl Eq for dyn IObject {}
