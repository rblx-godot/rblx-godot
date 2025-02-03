use std::collections::HashMap;

use super::{pvinstance::IPVInstance, DynInstance, IInstance, IInstanceComponent, IModel, IObject, InstanceComponent, ManagedInstance, ModelComponent, PVInstanceComponent};
use crate::{core::{get_state_with_rwlock, lua_macros::{lua_getter, lua_invalid_argument}, IWeak, InheritanceBase, InheritanceTableBuilder, Irc, LuauState, ParallelDispatch::{Desynchronized, Synchronized}, RobloxVM, RwLock, RwLockReadGuard, RwLockWriteGuard, Trc}, userdata::{ManagedRBXScriptSignal, RBXScriptConnection, RBXScriptSignal}};
use r2g_mlua::prelude::*;

pub type ManagedActor = Irc<Actor>;
pub type WeakManagedActor = IWeak<Actor>;

#[derive(Debug)]
pub struct Actor {
    instance: RwLock<InstanceComponent>,
    pvinstance: RwLock<PVInstanceComponent>,
    model: RwLock<ModelComponent>,
    state: Trc<LuauState>,
    messages_bound: RwLock<HashMap<String, ManagedRBXScriptSignal>>
}

impl InheritanceBase for Actor {
    fn inheritance_table(&self) -> crate::core::InheritanceTable {
        InheritanceTableBuilder::new()
            .insert_type::<Actor, dyn IObject>(|x| x, |x| x)
            .insert_type::<Actor, dyn IInstance>(|x| x, |x| x)
            .insert_type::<Actor, dyn IPVInstance>(|x| x, |x| x)
            .insert_type::<Actor, dyn IModel>(|x| x, |x| x)
            .insert_type::<Actor, Actor>(|x| x, |x| x)
            .output()
    }
}

impl IObject for Actor {
    fn lua_get(&self, lua: &Lua, name: String) -> LuaResult<LuaValue> {
        match name.as_str() {
            "BindToMessage" => lua_getter!(function, lua, |lua, (this, topic, function): (ManagedInstance, String, LuaFunction)|
                this.cast_from_unsized::<Actor>()
                    .map_err(|_| lua_invalid_argument!("Actor::BindToMessage", 1, self cast Instance to Actor))
                    .map(|this| this.bind_to_message(lua, topic, function))
            ),
            "BindToMessageParallel" => lua_getter!(function, lua, |lua, (this, topic, function): (ManagedInstance, String, LuaFunction)|
                this.cast_from_unsized::<Actor>()
                    .map_err(|_| lua_invalid_argument!("Actor::BindToMessageParallel", 1, self cast Instance to Actor))
                    .map(|this| this.bind_to_message_parallel(lua, topic, function))
            ),
            "SendMessage" => lua_getter!(function, lua, |lua, (this, topic, args): (ManagedInstance, String, LuaMultiValue)|
                this.cast_from_unsized::<Actor>()
                    .map_err(|_| lua_invalid_argument!("Actor::SendMessage", 1, self cast Instance to Actor))
                    .map(|this| this.send_message(lua, topic, args))
            ),
            _ => self.get_model_component().lua_get(self, lua, &name)
                .or_else(|| self.get_pv_instance_component().lua_get(self, lua, &name))
                .unwrap_or_else(|| self.get_instance_component().lua_get(lua, &name))
        }
    }

    fn get_class_name(&self) -> &'static str { "Actor" }

    fn get_property_changed_signal(&self, property: String) -> ManagedRBXScriptSignal {
        self.get_instance_component().get_property_changed_signal(property).unwrap()
    }

    fn is_a(&self, class_name: &String) -> bool {
        match class_name.as_str() {
            "Object" |
            "Instance" |
            "PVInstance" |
            "Model" |
            "Actor" => true,
            _ => false
        }
    }

    fn get_changed_signal(&self) -> ManagedRBXScriptSignal {
        self.get_instance_component().changed.clone()
    }
}

impl IInstance for Actor {
    fn get_instance_component(&self) -> RwLockReadGuard<InstanceComponent> {
        self.instance.read().unwrap()
    }

    fn get_instance_component_mut(&self) -> RwLockWriteGuard<InstanceComponent> {
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
            let state = self.state.read().get_vm_mut().create_sub_state();
            let a = Actor {
                instance: RwLock::new_with_flag_auto(self.get_instance_component().clone(lua, &i)?),
                pvinstance: RwLock::new_with_flag_auto(self.get_pv_instance_component().clone(lua, &i)?),
                model: RwLock::new_with_flag_auto(self.get_model_component().clone(lua, &i)?),
                state,
                messages_bound: RwLock::new(HashMap::new())
            };
            Ok(a)
        })?.cast_from_sized().unwrap())
    }

    fn get_actor(&self) -> LuaResult<Option<ManagedInstance>> {
        Ok(Some(self.get_instance_component().get_instance_pointer()))
    }
}

impl IPVInstance for Actor {
    fn get_pv_instance_component(&self) -> RwLockReadGuard<'_, PVInstanceComponent> {
        self.pvinstance.read().unwrap()
    }

    fn get_pv_instance_component_mut(&self) -> RwLockWriteGuard<'_, PVInstanceComponent> {
        self.pvinstance.write().unwrap()
    }
}

impl IModel for Actor {
    fn get_model_component(&self) -> RwLockReadGuard<'_, ModelComponent> {
        self.model.read().unwrap()
    }

    fn get_model_component_mut(&self) -> RwLockWriteGuard<'_, ModelComponent> {
        self.model.write().unwrap()
    }
}

impl Actor {
    pub fn new(mut vm: RwLockWriteGuard<'_, RobloxVM>) -> ManagedInstance {
        let actor: Irc<DynInstance> = Irc::new_cyclic(|x|
            Actor {
                instance: RwLock::new_with_flag_auto(InstanceComponent::new(x.cast_to_instance(), "Actor")),
                pvinstance: RwLock::new_with_flag_auto(PVInstanceComponent::new(x.cast_to_instance(), "Actor")),
                model: RwLock::new_with_flag_auto(ModelComponent::new(x.cast_to_instance(), "Actor")),
                state: vm.create_sub_state(),
                messages_bound: RwLock::new(HashMap::new())
        }).cast_from_sized().unwrap();
        actor
    }

    pub fn bind_to_message(&self, lua: &Lua, topic: String, function: LuaFunction) -> LuaResult<RBXScriptConnection> {
        if *get_state_with_rwlock(lua) != self.state {
            return Err(LuaError::RuntimeError("Cannot bind from outside of the Actor.".into()));
        }
        let mut messages_bound = self.messages_bound.write().unwrap();
        if let Some(sig) = messages_bound.get(&topic) {
            sig.write().connect(lua, function, Synchronized)
        } else {
            let sig = RBXScriptSignal::new();
            let conn = sig.write().connect(lua, function, Synchronized);
            messages_bound.insert(topic, sig.clone());
            conn
        }
    }
    pub fn bind_to_message_parallel(&self, lua: &Lua, topic: String, function: LuaFunction) -> LuaResult<RBXScriptConnection> {
        if *get_state_with_rwlock(lua) != self.state {
            return Err(LuaError::RuntimeError("Cannot bind from outside of the Actor.".into()));
        }
        let mut messages_bound = self.messages_bound.write().unwrap();
        if let Some(sig) = messages_bound.get(&topic) {
            sig.write().connect(lua, function, Desynchronized)
        } else {
            let sig = RBXScriptSignal::new();
            let conn = sig.write().connect(lua, function, Desynchronized);
            messages_bound.insert(topic, sig.clone());
            conn
        }
    }
    pub fn send_message(&self, lua: &Lua, topic: String, message: LuaMultiValue) -> LuaResult<()> {
        let messages_bound = self.messages_bound.read().unwrap();
        if let Some(sig) = messages_bound.get(&topic) {
            let sig_copy = sig.clone();
            drop(messages_bound);
            sig_copy.write().fire(lua, message)
        } else {
            Ok(())
        }
    }
    pub fn get_state(&self) -> &Trc<LuauState> {
        &self.state
    }
}