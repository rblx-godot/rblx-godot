use r2g_mlua::prelude::*;

use super::instance::IInstanceComponent;
use super::pvinstance::IPVInstance;
use super::{DynInstance, IInstance, IObject, InstanceComponent, ManagedInstance, PVInstanceComponent, WeakManagedInstance};

use crate::core::{InheritanceBase, InheritanceTable, InheritanceTableBuilder, Irc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::userdata::{CFrame, ManagedRBXScriptSignal};
use crate::userdata::enums::{ModelLevelOfDetail, ModelStreamingMode};

#[derive(Debug)]
pub struct ModelComponent {
    level_of_detail: ModelLevelOfDetail,
    model_streaming_mode: ModelStreamingMode,
    primary_part: Option<ManagedInstance>, // todo!()
    world_pivot: CFrame
}
#[derive(Debug)]
pub struct Model {
    instance: RwLock<InstanceComponent>,
    pvinstance: RwLock<PVInstanceComponent>,
    model: RwLock<ModelComponent>
}
pub trait IModel: IPVInstance {
    fn get_model_component(&self) -> RwLockReadGuard<'_,ModelComponent>;
    fn get_model_component_mut(&self) -> RwLockWriteGuard<'_,ModelComponent>;
}

impl InheritanceBase for Model {
    fn inheritance_table(&self) -> InheritanceTable {
        InheritanceTableBuilder::new()
            .insert_type::<Model,dyn IObject>(|x: &Self| x as &dyn IObject, |x: &mut Self| x as &mut dyn IObject)
            .insert_type::<Model,dyn IInstance>(|x: &Self| x as &dyn IInstance, |x: &mut Self| x as &mut dyn IInstance)
            .insert_type::<Model,dyn IPVInstance>(|x: &Self| x as &dyn IPVInstance, |x: &mut Self| x as &mut dyn IPVInstance)
            .insert_type::<Model,dyn IModel>(|x: &Self| x as &dyn IModel, |x: &mut Self| x as &mut dyn IModel)
            .output()
    }
}
impl IObject for Model {
    fn is_a(&self, class_name: &String) -> bool {
        match class_name.as_str() {
            "Model" |
            "PVInstance" |
            "Instance" |
            "Object" => true,
            _ => false
        }
    }
    fn lua_get(&self, lua: &Lua, name: String) -> LuaResult<LuaValue> {
        self.get_model_component().lua_get(self, lua, &name)
            .or_else(|| self.get_pv_instance_component().lua_get(self, lua, &name))
            .unwrap_or_else(|| self.get_instance_component().lua_get(lua, &name))
    }
    fn get_changed_signal(&self) -> ManagedRBXScriptSignal {
        self.get_instance_component().changed.clone()
    }
    fn get_property_changed_signal(&self, property: String) -> ManagedRBXScriptSignal {
        self.get_instance_component().get_property_changed_signal(property).unwrap()
    }
    fn get_class_name(&self) -> &'static str { "Model" }
}
impl IInstance for Model {
    fn get_instance_component(&self) -> RwLockReadGuard<'_, InstanceComponent> {
        self.instance.read().unwrap()
    }
    fn get_instance_component_mut(&self) -> RwLockWriteGuard<'_, InstanceComponent> {
        self.instance.write().unwrap()
    }
    fn lua_set(&self, lua: &Lua, name: String, val: LuaValue) -> LuaResult<()> {
        self.get_model_component_mut().lua_set(self, lua, &name, &val)
            .or_else(|| self.get_pv_instance_component_mut().lua_set(self, lua, &name, &val))
            .unwrap_or_else(|| self.get_instance_component_mut().lua_set(lua, &name, val))
    }
    fn clone_instance(&self, lua: &Lua) -> LuaResult<ManagedInstance> {
        Ok(Irc::new_cyclic_fallable::<_, LuaError>(|x| {
            let i = x.cast_to_instance();
            Ok(Model {
                instance: RwLock::new_with_flag_auto(self.get_instance_component().clone(lua, &i)?),
                pvinstance: RwLock::new_with_flag_auto(self.get_pv_instance_component().clone(lua, &i)?),
                model: RwLock::new_with_flag_auto(self.get_model_component().clone(lua, &i)?)
            })
        })?.cast_from_sized().unwrap())
    }
}
impl IPVInstance for Model {
    fn get_pv_instance_component(&self) -> RwLockReadGuard<'_, PVInstanceComponent> {
        self.pvinstance.read().unwrap()
    }

    fn get_pv_instance_component_mut(&self) -> RwLockWriteGuard<'_, PVInstanceComponent> {
        self.pvinstance.write().unwrap()
    }
}
impl IModel for Model {
    fn get_model_component(&self) -> RwLockReadGuard<'_, ModelComponent> {
        self.model.read().unwrap()
    }
    fn get_model_component_mut(&self) -> RwLockWriteGuard<'_, ModelComponent> {
        self.model.write().unwrap()
    }
}

impl IInstanceComponent for ModelComponent {
    fn lua_get(self: &mut RwLockReadGuard<'_, ModelComponent>, _: &DynInstance, _lua: &Lua, key: &String) -> Option<LuaResult<LuaValue>> {
        match key.as_str() {
            "LevelOfDetail" => todo!(),
            "ModelStreamingMode" => todo!(),
            "PrimaryPart" => todo!(),
            "WorldPivot" => todo!(),
            _ => None
        }
    }

    fn lua_set(self: &mut RwLockWriteGuard<'_, ModelComponent>, _: &DynInstance, _lua: &Lua, key: &String, _value: &LuaValue) -> Option<LuaResult<()>> {
        match key.as_str() {
            "LevelOfDetail" => todo!(),
            "ModelStreamingMode" => todo!(),
            "PrimaryPart" => todo!(),
            "WorldPivot" => todo!(),
            _ => None
        }
    }

    fn clone(self: &RwLockReadGuard<'_, ModelComponent>, _: &Lua, _: &WeakManagedInstance) -> LuaResult<Self> {
        Ok(ModelComponent {
            level_of_detail: self.level_of_detail,
            model_streaming_mode: self.model_streaming_mode,
            primary_part: None,
            world_pivot: self.world_pivot
        })
    }

    fn new(_: super::WeakManagedInstance, _: &'static str) -> Self {
        ModelComponent {
            level_of_detail: ModelLevelOfDetail::Automatic,
            model_streaming_mode: ModelStreamingMode::Default,
            primary_part: None,
            world_pivot: CFrame::IDENTITY
        }
    }
}

impl Model {
    pub fn new() -> ManagedInstance {
        Irc::new_cyclic(|x| {
            Model {
                instance: RwLock::new_with_flag_auto(InstanceComponent::new(x.cast_to_instance(), "Model")),
                pvinstance: RwLock::new_with_flag_auto(PVInstanceComponent::new(x.cast_to_instance(), "Model")),
                model: RwLock::new_with_flag_auto(ModelComponent::new(x.cast_to_instance(), "Model"))
            }
        }).cast_from_sized().unwrap()
    }
}