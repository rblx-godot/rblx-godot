use r2g_mlua::prelude::*;

use super::instance::IInstanceComponent;
use super::{DynInstance, IInstance, ManagedInstance, WeakManagedInstance};

use crate::core::lua_macros::{lua_getter, lua_invalid_argument};
use crate::core::{inheritance_cast_to, RwLockReadGuard, RwLockWriteGuard};
use crate::userdata::{ManagedRBXScriptSignal, RBXScriptSignal};
#[derive(Debug)]
pub struct ServiceProviderComponent {
    pub close: ManagedRBXScriptSignal,
    pub service_added: ManagedRBXScriptSignal,
    pub service_removing: ManagedRBXScriptSignal,
}
pub trait IServiceProvider: IInstance {
    fn get_service_provider_component(&self) -> RwLockReadGuard<'_, ServiceProviderComponent>;
    fn get_service_provider_component_mut(&self) -> RwLockWriteGuard<'_, ServiceProviderComponent>;
    fn get_service(&self, service_name: String) -> LuaResult<ManagedInstance>;
    fn find_service(&self, service_name: String) -> LuaResult<Option<ManagedInstance>>;
}

impl IInstanceComponent for ServiceProviderComponent {
    fn lua_get(self: &mut RwLockReadGuard<'_, Self>, _ptr: &DynInstance, lua: &Lua, key: &String) -> Option<LuaResult<LuaValue>> {
        match key.as_str() {
            "Close" => Some(lua_getter!(clone, lua, self.close)),
            "ServiceAdded" => Some(lua_getter!(clone, lua, self.service_added)),
            "ServiceRemoving" => Some(lua_getter!(clone, lua, self.service_removing)),
            "GetService" => lua_getter!(function_opt, lua, |_, (this, name): (ManagedInstance, String)| {
                let i = inheritance_cast_to!(&*this, dyn IServiceProvider);
                i.map_err(|_|
                    lua_invalid_argument!("ServiceProvider::GetService",1,self cast Instance to ServiceProvider)
                )?;
                unsafe {
                    i.unwrap_unchecked().get_service(name)
                }
            }),
            "FindService" => lua_getter!(function_opt, lua, |_, (this, name): (ManagedInstance, String)| {
                let i = inheritance_cast_to!(&*this, dyn IServiceProvider);
                i.map_err(|_|
                    lua_invalid_argument!("ServiceProvider::FindService",1,self cast Instance to ServiceProvider)
                )?;
                unsafe {
                    i.unwrap_unchecked().find_service(name)
                }
            }),
            _ => None
        }
    }

    fn lua_set(self: &mut RwLockWriteGuard<'_, Self>, _ptr: &DynInstance, _lua: &Lua, _key: &String, _value: &LuaValue) -> Option<LuaResult<()>> {
        None // unimplemented
    }

    fn clone(self: &RwLockReadGuard<'_, Self>, _: &Lua, new_ptr: &WeakManagedInstance) -> LuaResult<Self> {
        Ok(Self::new(new_ptr.clone(), ""))
    }

    fn new(_ptr: WeakManagedInstance, _class_name: &'static str) -> Self {
        Self {
            close: RBXScriptSignal::new(),
            service_added: RBXScriptSignal::new(),
            service_removing: RBXScriptSignal::new(),
        }
    }
}