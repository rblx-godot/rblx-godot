use r2g_mlua::prelude::*;

use crate::core::FastFlags;
use crate::core::{get_state, inheritance_cast_to, FastFlag, InheritanceBase, InheritanceTable, InheritanceTableBuilder, Irc, 
    ParallelDispatch::Synchronized, RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::core::lua_macros::{lua_getter, lua_invalid_argument};
use crate::userdata::{ManagedRBXScriptSignal, RBXScriptSignal};

use super::{IInstanceComponent, DynInstance, IInstance, IObject, IServiceProvider, InstanceComponent, ManagedInstance, ServiceProviderComponent, WeakManagedInstance};

#[derive(Debug)]
pub struct DataModelComponent {
    bind_close: ManagedRBXScriptSignal,
    workspace: (), //todo!

    pub graphics_quality_change_request: ManagedRBXScriptSignal,
    pub loaded: ManagedRBXScriptSignal,
    is_loaded: bool

}

#[derive(Debug)]
pub struct DataModel {
    instance: RwLock<InstanceComponent>,
    service_provider: RwLock<ServiceProviderComponent>,
    data_model: RwLock<DataModelComponent>
}

pub trait IDataModel {
    fn get_data_model_component(&self) -> RwLockReadGuard<'_,DataModelComponent>;
    fn get_data_model_component_mut(&self) -> RwLockWriteGuard<'_,DataModelComponent>;
}

impl InheritanceBase for DataModel {
    fn inheritance_table(&self) -> InheritanceTable {
        InheritanceTableBuilder::new()
            .insert_type::<DataModel, dyn IObject>(|x| x, |x| x)
            .insert_type::<DataModel, DynInstance>(|x| x, |x| x)
            .insert_type::<DataModel, dyn IServiceProvider>(|x| x, |x| x)
            .insert_type::<DataModel, dyn IDataModel>(|x| x, |x| x)
            .output()
    }
}

impl IObject for DataModel {
    fn lua_get(&self, lua: &Lua, name: String) -> LuaResult<LuaValue> {
        self.data_model.read().unwrap().lua_get(self, lua, &name)
            .or_else(|| self.service_provider.read().unwrap().lua_get(self, lua, &name))
            .unwrap_or_else(|| self.instance.read().unwrap().lua_get(lua, &name))
    }

    fn get_class_name(&self) -> &'static str { "DataModel" }

    fn get_property_changed_signal(&self, property: String) -> ManagedRBXScriptSignal {
        self.get_instance_component().get_property_changed_signal(property).unwrap()
    }

    fn is_a(&self, class_name: &String) -> bool {
        match class_name.as_str() {
            "DataModel" => true,
            "ServiceProvider" => true,
            "Instance" => true,
            "Object" => true,
            _ => false
        }
    }

    fn get_changed_signal(&self) -> ManagedRBXScriptSignal {
        self.get_instance_component().changed.clone()
    }
}

impl IInstance for DataModel {
    fn get_instance_component(&self) -> RwLockReadGuard<InstanceComponent> {
        self.instance.read().unwrap()
    }

    fn get_instance_component_mut(&self) -> RwLockWriteGuard<InstanceComponent> {
        self.instance.write().unwrap()
    }

    fn lua_set(&self, lua: &Lua, name: String, val: LuaValue) -> LuaResult<()> {
        self.data_model.write().unwrap().lua_set(self, lua, &name, &val)
            .or_else(|| self.service_provider.write().unwrap().lua_set(self, lua, &name, &val))
            .unwrap_or_else(|| self.instance.write().unwrap().lua_set(lua, &name, val))
    }

    fn clone_instance(&self, _: &Lua) -> LuaResult<ManagedInstance> {
        Err(LuaError::RuntimeError("DataModel cannot be cloned".into()))
    }
}

impl IServiceProvider for DataModel {
    fn get_service_provider_component(&self) -> RwLockReadGuard<ServiceProviderComponent> {
        self.service_provider.read().unwrap()
    }

    fn get_service_provider_component_mut(&self) -> RwLockWriteGuard<ServiceProviderComponent> {
        self.service_provider.write().unwrap()
    }

    fn get_service(&self, service_name: String) -> LuaResult<ManagedInstance> {
        self.find_service(service_name)
            .and_then(|x|
                x.ok_or_else(|| LuaError::RuntimeError("Service not found".into()))
            )
    }

    fn find_service(&self, service_name: String) -> LuaResult<Option<ManagedInstance>> {
        DynInstance::find_first_child_of_class(self, service_name)
    }
}

impl DataModel {
    pub fn new(flags: &FastFlags) -> ManagedInstance {
        let game: Irc<DynInstance> = Irc::new_cyclic(|x|
            DataModel {
                instance: RwLock::new_with_flag_auto(InstanceComponent::new(x.cast_to_instance(), "DataModel")),
                service_provider: RwLock::new_with_flag_auto(ServiceProviderComponent::new(x.cast_to_instance(), "DataModel")),
                data_model: RwLock::new_with_flag_auto(DataModelComponent::new(x.cast_to_instance(), "DataModel"))
        }).cast_from_sized().unwrap();
        game.set_name(flags.get_string(FastFlag::GameName)).unwrap();
        game.lock_parent();
        game
    }
}

impl IInstanceComponent for DataModelComponent {
    fn lua_get(self: &mut RwLockReadGuard<'_, Self>, _ptr: &DynInstance, lua: &Lua, key: &String) -> Option<LuaResult<LuaValue>> {
        match key.as_str() {
            "CreatorId" => Some(lua_getter!(lua, get_state(lua).flags().get_int(FastFlag::CreatorId))),
            "CreatorType" => todo!(),
            "GameId" => Some(lua_getter!(lua, get_state(lua).flags().get_int(FastFlag::GameId))),
            "JobId" => Some(lua_getter!(lua, get_state(lua).flags().get_string(FastFlag::JobId))),
            "PlaceId" => Some(lua_getter!(lua, get_state(lua).flags().get_int(FastFlag::PlaceId))),
            "PlaceVersion" => Some(lua_getter!(lua, get_state(lua).flags().get_int(FastFlag::PlaceVersion))),
            "PrivateServerId" => Some(lua_getter!(lua, get_state(lua).flags().get_string(FastFlag::PrivateServerId))),
            "PrivateServerOwnerId" => Some(lua_getter!(lua, get_state(lua).flags().get_int(FastFlag::PrivateServerOwnerId))),
            "Workspace" => todo!(),
            "BindToClose" => lua_getter!(function_opt, lua, |lua, (this, func): (ManagedInstance, LuaFunction)| {
                inheritance_cast_to!(&*this, dyn IDataModel)
                    .map_err(|_|
                        lua_invalid_argument!("DataModel::BindToClose",1,self cast Instance to DataModel)
                    )
                    .and_then(|x|
                        x.bind_to_close(lua, func)
                    )
            }),
            "IsLoaded" => Some(Ok(LuaValue::Boolean(true))),
            "GraphicsQualityChangeRequest" => Some(lua_getter!(clone, lua, self.graphics_quality_change_request)),
            "Loaded" => Some(lua_getter!(clone, lua, self.loaded)),
            _ => None
        }
    }

    fn lua_set(self: &mut RwLockWriteGuard<'_, Self>, _ptr: &DynInstance, _lua: &Lua, key: &String, _value: &LuaValue) -> Option<LuaResult<()>> {
        match key.as_str() {
            "CreatorId" |
            "CreatorType" |
            "GameId" |
            "JobId" |
            "PlaceId" |
            "PlaceVersion" |
            "PrivateServerId" |
            "PrivateServerOwnerId" |
            "Workspace "=> Some(Err(LuaError::RuntimeError("Cannot set read only property.".into()))),
            _ => None
        }
    }

    fn clone(self: &RwLockReadGuard<'_, Self>, _: &Lua, _: &WeakManagedInstance) -> LuaResult<Self> {
        Err(LuaError::RuntimeError("Cannot clone DataModelComponent".into()))
    }

    fn new(_ptr: WeakManagedInstance, _class_name: &'static str) -> Self {
        Self {
            bind_close: RBXScriptSignal::new(),
            workspace: (),
            graphics_quality_change_request: RBXScriptSignal::new(),
            loaded: RBXScriptSignal::new(),
            is_loaded: false
        }
    }
}

impl IDataModel for DataModel {
    fn get_data_model_component(&self) -> RwLockReadGuard<'_,DataModelComponent> {
        self.data_model.read().unwrap()
    }

    fn get_data_model_component_mut(&self) -> RwLockWriteGuard<'_,DataModelComponent> {
        self.data_model.write().unwrap()
    }
}
impl dyn IDataModel {
    pub fn bind_to_close(&self, lua: &Lua, func: LuaFunction) -> LuaResult<()> {
        let read = self.get_data_model_component();
        read.bind_close.write().once(lua, func, Synchronized)?;
        Ok(())
    }
    pub fn is_loaded(&self) -> bool { 
        self.get_data_model_component().is_loaded
    }
    pub fn fire_loaded(&self, lua: &Lua) -> LuaResult<()> {
        let mut write = self.get_data_model_component_mut();
        write.is_loaded = true;
        write.loaded.write().fire(lua, ())
    }
}