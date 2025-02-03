use r2g_mlua::prelude::*;

use crate::core::lua_macros::lua_getter;
use crate::core::{get_state, get_state_with_rwlock, FastFlag, InheritanceBase, InheritanceTable, InheritanceTableBuilder, Irc, LuauState, RwLock, RwLockReadGuard, RwLockWriteGuard, Trc};
use crate::userdata::{ManagedRBXScriptSignal, RBXScriptSignal};
use super::{DynInstance, IInstance, ManagedInstance};
use super::{IObject, InstanceComponent, instance::IInstanceComponent};

pub struct RunService {
    instance_component: RwLock<InstanceComponent>,

    render_steps: RwLock<Vec<(String, f64, (Trc<LuauState>, LuaFunction))>>,

    pub heart_beat: ManagedRBXScriptSignal,
    pub post_simulation: ManagedRBXScriptSignal,
    pub pre_animation: ManagedRBXScriptSignal,
    pub pre_render: ManagedRBXScriptSignal,
    pub pre_simulation: ManagedRBXScriptSignal,
    pub render_stepped: ManagedRBXScriptSignal,
    pub stepped: ManagedRBXScriptSignal,
}

impl InheritanceBase for RunService {
    fn inheritance_table(&self) -> InheritanceTable {
        InheritanceTableBuilder::new()
            .insert_type::<RunService, dyn IObject>(|x: &Self| x, |x: &mut Self| x)
            .insert_type::<RunService, DynInstance>(|x: &Self| x, |x: &mut Self| x)
            .output()
    }
}

impl IObject for RunService {
    fn is_a(&self, class_name: &String) -> bool {
        match class_name.as_str() {
            "RunService" |
            "Instance" |
            "Object" => true,
            _ => false
        }
    }
    fn lua_get(&self, lua: &Lua, name: String) -> LuaResult<LuaValue> {
        match name.as_str() {
            "Heartbeat" => lua_getter!(clone, lua, self.heart_beat),
            "PostSimulation" => lua_getter!(clone, lua, self.post_simulation),
            "PreAnimation" => lua_getter!(clone, lua, self.pre_animation),
            "PreRender" => lua_getter!(clone, lua, self.pre_render),
            "PreSimulation" => lua_getter!(clone, lua, self.pre_simulation),
            "RenderStepped" => lua_getter!(clone, lua, self.render_stepped),
            "Stepped" => lua_getter!(clone, lua, self.stepped),
            "IsClient" => lua_getter!(function, lua, 
                |lua, _: ManagedInstance| 
                    Ok(get_state(lua).flags().get_bool(FastFlag::IsClient))
            ),
            "IsServer" => lua_getter!(function, lua, 
                |lua, _: ManagedInstance| 
                    Ok(!get_state(lua).flags().get_bool(FastFlag::IsClient))
            ),
            "IsStudio" => lua_getter!(function, lua, 
                |lua, _: ManagedInstance| 
                    Ok(get_state(lua).flags().get_bool(FastFlag::IsStudio))
            ),
            "IsEdit" => lua_getter!(function, lua, 
                |_, _: ManagedInstance| 
                    Ok(false)
            ),
            "IsRunning" => lua_getter!(function, lua, 
                |_, _: ManagedInstance| 
                    Ok(true)
            ),
            "IsRunMode" => lua_getter!(function, lua, 
                |_, _: ManagedInstance| 
                    Ok(true)
            ),
            "Run" => lua_getter!(function, lua, 
                |_, _: ManagedInstance| 
                    Err::<(), _>(LuaError::RuntimeError("expected security level PluginSecurity, but got None".into()))
            ),
            "Stop" => lua_getter!(function, lua, 
                |_, _: ManagedInstance| 
                    Err::<(), _>(LuaError::RuntimeError("expected security level PluginSecurity, but got None".into()))
            ),
            "Pause" => lua_getter!(function, lua, 
                |_, _: ManagedInstance| 
                    Err::<(), _>(LuaError::RuntimeError("expected security level PluginSecurity, but got None".into()))
            ),
            "BindToRenderStep" => lua_getter!(function, lua, 
                |lua, (this, name, priority, func) : (ManagedInstance, String, f64, LuaFunction)|
                    this.cast_from_unsized::<RunService>()
                        .map_err(|_| LuaError::RuntimeError("expected RunService, got Instance".into()))?
                        .bind_to_render_step(lua, name, priority, func)
            ),
            "UnbindFromRenderStep" => lua_getter!(function, lua, 
                |_, (this, name) : (ManagedInstance, String)|
                    this.cast_from_unsized::<RunService>()
                        .map_err(|_| LuaError::RuntimeError("expected RunService, got Instance".into()))?
                        .unbind_from_render_step(name)
            ),
            _ => self.instance_component.read().unwrap().lua_get(lua, &name)
        }
    }
    fn get_changed_signal(&self) -> ManagedRBXScriptSignal {
        self.instance_component.read().unwrap().changed.clone()
    }
    fn get_property_changed_signal(&self, property: String) -> ManagedRBXScriptSignal {
        self.instance_component.read().unwrap().get_property_changed_signal(property).unwrap()
    }
    fn get_class_name(&self) -> &'static str { "RunService" }
}

impl IInstance for RunService {
    fn get_instance_component(&self) -> RwLockReadGuard<'_, InstanceComponent> {
        self.instance_component.read().unwrap()
    }
    fn get_instance_component_mut(&self) -> RwLockWriteGuard<'_, InstanceComponent> {
        self.instance_component.write().unwrap()
    }
    fn lua_set(&self, lua: &Lua, name: String, val: LuaValue) -> LuaResult<()> {
        self.instance_component.write().unwrap().lua_set(lua, &name, val)
    }
    fn clone_instance(&self, _: &Lua) -> LuaResult<ManagedInstance> {
        Err(LuaError::RuntimeError("Cannot clone RunService.".into()))
    }
}

impl RunService {
    pub fn new() -> Irc<RunService> {
        Irc::new_cyclic(|x| RunService {
            instance_component: RwLock::new_with_flag_auto(InstanceComponent::new(x.cast_to_instance().clone(), "RunService")),
            render_steps: RwLock::new_with_flag_auto(Vec::new()),
            heart_beat: RBXScriptSignal::new(),
            post_simulation: RBXScriptSignal::new(),
            pre_animation: RBXScriptSignal::new(),
            pre_render: RBXScriptSignal::new(),
            pre_simulation: RBXScriptSignal::new(),
            render_stepped: RBXScriptSignal::new(),
            stepped: RBXScriptSignal::new(),
        })
    }
}
impl RunService {
    pub fn bind_to_render_step(&self, lua: &Lua, name: String, priority: f64, func: LuaFunction) -> LuaResult<()> {
        let mut write = self.render_steps.write().unwrap();
        write.push((name, priority, (get_state_with_rwlock(lua).clone(), func)));
        write.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        Ok(())
    }
    pub fn unbind_from_render_step(&self, name: String) -> LuaResult<()> {
        let mut write = self.render_steps.write().unwrap();
        write.retain(|x| x.0 != name);
        Ok(())
    }
}