use r2g_mlua::prelude::*;

use crate::{core::{get_state, lua_macros::lua_getter}, instance::{Actor, DynInstance, LocalScript, ManagedInstance, Model, Script}};

use super::LuaSingleton;

impl LuaUserData for ManagedInstance {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method("__index", |lua, this, field: String| {
            this.lua_get(lua, field)
        });
        methods.add_meta_method("__newindex", |lua, this, (field, val): (String, LuaValue)| {
            this.lua_set(lua, field, val)
        });
        methods.add_meta_method("__tostring", |_, this: &ManagedInstance, ()| {
            let instance_read = this.get_instance_component();
            Ok(format!("{} {}: replication 0x{:x}", 
                this.get_class_name(), 
                DynInstance::guard_get_name(&instance_read), 
                DynInstance::guard_get_uniqueid(&instance_read)))
        });
    }
}

impl LuaSingleton for ManagedInstance {
    fn register_singleton(lua: &Lua) -> LuaResult<()> {
        let table = lua.create_table()?;
        table.raw_set("new", lua.create_function(|lua, (class_name,): (String,)| {
            match class_name.as_str() {
                "Model" => lua_getter!(lua, Model::new()),
                "Actor" => lua_getter!(lua, Actor::new(get_state(lua).get_vm_mut())),
                "Script" => lua_getter!(lua, Script::new()),
                "LocalScript" => lua_getter!(lua, LocalScript::new()),
                _ => Err::<LuaValue, LuaError>(LuaError::RuntimeError(format!("invalid class name \"{}\"", class_name)))
            }
        })?)?;
        lua.globals().raw_set("Instance", table)?;
        Ok(())
    }
}

impl FromLua for ManagedInstance {
    fn from_lua(value:LuaValue,_lua: &Lua) -> LuaResult<Self>{
        let ud = value.as_userdata();
        if ud.is_none(){
            Err(LuaError::FromLuaConversionError {
                from:value.type_name(),to:stringify!(ManagedInstance).into(),message:None
            })
        }else {
            let unwrapped = unsafe {
                ud.unwrap_unchecked()
            }.borrow::<ManagedInstance>();
            if unwrapped.is_err(){
                Err(LuaError::FromLuaConversionError {
                    from:"userdata",to:stringify!(ManagedInstance).into(),message:None
                })
            }else {
                unsafe {
                    Ok(unwrapped.unwrap_unchecked().clone())
                }
            }
        }
    }

    }