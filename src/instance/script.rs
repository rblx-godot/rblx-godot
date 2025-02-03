use std::mem::take;

use r2g_mlua::prelude::*;

use crate::core::lua_macros::{lua_getter, lua_setter};
use crate::core::ParallelDispatch::Synchronized;
use crate::instance::{IObject, IInstance, DynInstance, ManagedInstance, InstanceComponent, IInstanceComponent, WeakManagedInstance};
use crate::userdata::enums::RunContext;
use crate::userdata::{ManagedRBXScriptSignal, RBXScriptConnection};
use crate::core::{borrowck_ignore, borrowck_ignore_mut, get_current_identity, get_state, get_task_scheduler_from_lua, inheritance_cast_to, FastFlag, InheritanceBase, InheritanceTableBuilder, Irc, LuauState, RwLock, RwLockReadGuard, RwLockWriteGuard, SecurityContext, Trc};

use super::ManagedActor;
#[derive(Debug)]
enum ActorLuauState {
    Main(Trc<LuauState>),
    Actor(ManagedActor),
    None
}

#[derive(Debug)]
pub struct BaseScriptComponent {
    disabled: bool,
    change_scheduled: Option<LuaThread>,
    run_context: RunContext,
    actor: ActorLuauState,
    self_instance: WeakManagedInstance,
    source: String,
    has_set_up_destroying: bool,
    pub(crate) connections: Vec<RBXScriptConnection>
}

impl BaseScriptComponent {
    fn schedule_start<'a>(self: &mut RwLockWriteGuard<'_, Self>, lua: &'a Lua) -> LuaResult<()> {
        if self.change_scheduled.is_some() {
            return Ok(());
        }
        let instance = self.self_instance.upgrade().unwrap();
        self.actor = instance.get_actor()?
            .map(|x| ActorLuauState::Actor(x.cast_from_unsized().unwrap()))
            .unwrap_or_else(|| ActorLuauState::Main(get_state(lua).get_vm().get_main_state_rc()));
        let state_lock = self.get_state();
        let mut state = unsafe {borrowck_ignore(&state_lock)}.write();
        let debug = state.flags().get_bool(FastFlag::DebugMode);

        let env = state.create_env_from_global()?;
        env.set("script", instance.clone())?;
        let func: LuaFunction;
        {
            let f: LuaResult<LuaFunction>;
            if debug {
                f = state.compile_debug(
                    format!("<script at {}>", instance.get_full_name()?).as_str(), self.source.as_str(), env
                );
            } else if self.source.lines().any(|x| x == "--!native") {
                f = state.compile_jit(
                    format!("<script at {}>", instance.get_full_name()?).as_str(), self.source.as_str(), env
                );
            } else {
                f = state.compile_release(
                    format!("<script at {}>", instance.get_full_name()?).as_str(), self.source.as_str(), env
                );
            }
            func = f.inspect_err(|x|
                match x {
                    LuaError::RuntimeError(err) =>
                    state.get_vm().log_err(IntoLuaMulti::into_lua_multi(
                        format!("Error occured while setting up script {}: {}", instance.get_full_name().unwrap(), err),
                        lua
                    ).unwrap()),
                    LuaError::SyntaxError { message, .. } =>
                        state.get_vm().log_err(IntoLuaMulti::into_lua_multi(
                            format!("Error occured while compiling script {}: {}", instance.get_full_name().unwrap(), message),
                            lua
                        ).unwrap()),
                    x => state.get_vm().log_err(IntoLuaMulti::into_lua_multi(
                        format!("Unknown error while compiling script {}: {:?}", instance.get_full_name().unwrap(), x),
                        lua
                    ).unwrap()),
                }
            )?;
        }
        self.change_scheduled = Some(unsafe {
            let thread = borrowck_ignore_mut(borrowck_ignore_mut(&mut *state).get_task_scheduler_mut())
                .defer_func(borrowck_ignore(lua), func, (), Synchronized)?;
            
            thread
        });
        Ok(())
    }
    pub fn terminate(self: &mut RwLockWriteGuard<'_, Self>, lua: &Lua) -> LuaResult<()> {
        for i in take(&mut self.connections) {
            i.disconnect();
        }
        unsafe {
            let lstate = self.get_state();
            let mut state = lstate.write();
            let lua = borrowck_ignore_mut(&mut *state).get_lua();
            
            borrowck_ignore_mut(&mut *state).get_task_scheduler_mut().cancel_script(lua, &self.self_instance)?;
        }
        if let Some(iden) = get_current_identity(lua) {
            if iden.script.as_ref().map(|x| self.self_instance == *x).unwrap_or(false) {
                get_task_scheduler_from_lua(lua).cancel(lua, &lua.current_thread())?;
            }
        }
        Ok(())
    }
    pub fn set_source(self: &mut RwLockWriteGuard<'_, Self>, source: String) -> LuaResult<()> {
        self.source = source;
        Ok(())
    }
    pub fn set_disabled(self: &mut RwLockWriteGuard<'_, Self>, lua: &Lua, disabled: bool, implicit_run_context: RunContext) -> LuaResult<()> {
        if self.disabled == disabled {
            return Ok(());
        }
        self.disabled = disabled;
        if disabled {
            self.terminate(lua)
        } else {
            if !self.has_set_up_destroying {
                self.has_set_up_destroying = true;
                let instance = self.self_instance.upgrade().unwrap();
                let instance_ref = instance.clone();
                instance.get_instance_component().destroying.write().once(lua, lua.create_function(move |lua, ()| {
                    instance_ref.lua_set(lua, "Disabled".into(), LuaValue::Boolean(true))
                }).unwrap(), Synchronized).unwrap();
            }
            let current_run_context = {
                let flags = get_state(lua).flags();
                if flags.get_bool(FastFlag::IsStudio) {
                    RunContext::Plugin
                } else if flags.get_bool(FastFlag::IsClient) {
                    RunContext::Client
                } else {
                    RunContext::Server
                }
            };
            let required_run_context = match self.run_context {
                RunContext::Legacy => implicit_run_context,
                x => x
            };
            if current_run_context == required_run_context {
                self.schedule_start(lua)
            } else {
                Ok(())
            }
        }
    }
    fn schedule_start_if_not_started(self: &mut RwLockWriteGuard<'_, Self>, lua: &Lua, implicit_run_context: RunContext) -> LuaResult<()> {
        if self.change_scheduled.is_none() && !self.disabled {
            self.disabled = true;
            self.set_disabled(lua, false, implicit_run_context)
        } else {
            Ok(())
        }
    }
    fn get_state(self: &RwLockWriteGuard<'_, Self>) -> Trc<LuauState> {
        match &self.actor {
            ActorLuauState::Main(x) => x.clone(),
            ActorLuauState::Actor(x) => x.get_state().clone(),
            ActorLuauState::None => panic!("Actor state not set")
        }
    }
}
pub trait IBaseScript: IInstance {
    fn get_base_script_component(&self) -> RwLockReadGuard<'_, BaseScriptComponent>;
    fn get_base_script_component_mut(&self) -> RwLockWriteGuard<'_, BaseScriptComponent>;
    fn implicit_run_context(&self) -> RunContext;
}

impl IInstanceComponent for BaseScriptComponent {
    fn lua_get(self: &mut RwLockReadGuard<'_, Self>, _: &DynInstance, lua: &Lua, key: &String) -> Option<LuaResult<LuaValue>> {
        match key.as_str() {
            "Disabled" => Some(Ok(LuaValue::Boolean(self.disabled))),
            "Enabled" => Some(Ok(LuaValue::Boolean(!self.disabled))),
            "RunContext" => Some(lua_getter!(lua, self.run_context)),
            "Source" => Some(lua_getter!(clone, lua, self.source)),
            _ => None
        }
    }

    fn lua_set(self: &mut RwLockWriteGuard<'_, Self>, ptr: &DynInstance, lua: &Lua, key: &String, value: &LuaValue) -> Option<LuaResult<()>> {
        match key.as_str() {
            "Disabled" => {
                let disabled: bool = lua_setter!(opt_clone, lua, value);
                if disabled == self.disabled {
                    return Some(Ok(()));
                }
                let d = self.set_disabled(
                    lua,
                    disabled,
                    inheritance_cast_to!(ptr, dyn IBaseScript).unwrap().implicit_run_context()
                );
                if let Err(err) = d {
                    return Some(Err(err));
                }
                let d = InstanceComponent::emit_property_changed(
                    &ptr.get_instance_component(),
                    lua,
                    "Disabled",
                    &LuaValue::Boolean(self.disabled)
                );
                if let Err(err) = d {
                    return Some(Err(err));
                }
                let d = InstanceComponent::emit_property_changed(
                    &ptr.get_instance_component(),
                    lua,
                    "Enabled",
                    &LuaValue::Boolean(!self.disabled)
                );
                if let Err(err) = d {
                    return Some(Err(err));
                }
                Some(Ok(()))
            },
            "Enabled" => {
                let disabled: bool = lua_setter!(opt_clone, lua, value);
                if !disabled == self.disabled {
                    return Some(Ok(()));
                }
                let d = self.set_disabled(
                    lua,
                    !disabled,
                    inheritance_cast_to!(ptr, dyn IBaseScript).unwrap().implicit_run_context()
                );
                if let Err(err) = d {
                    return Some(Err(err));
                }
                let d = InstanceComponent::emit_property_changed(
                    &ptr.get_instance_component(),
                    lua,
                    "Disabled",
                    &LuaValue::Boolean(self.disabled)
                );
                if let Err(err) = d {
                    return Some(Err(err));
                }
                let d = InstanceComponent::emit_property_changed(
                    &ptr.get_instance_component(),
                    lua,
                    "Enabled",
                    &LuaValue::Boolean(!self.disabled)
                );
                if let Err(err) = d {
                    return Some(Err(err));
                }
                Some(Ok(()))
            },
            "RunContext" => {
                let run_context = lua_setter!(opt_clone, lua, value);
                if run_context == self.run_context {
                    return Some(Ok(()));
                }
                self.run_context = run_context;
                let v = lua_getter!(lua, run_context);
                if let Err(err) = v {
                    return Some(Err(err));
                }
                let d = InstanceComponent::emit_property_changed(
                    &ptr.get_instance_component(),
                    lua,
                    "RunContext",
                    &v.unwrap()
                );
                Some(d)
            },
            "Source" => {
                if let Some(iden) = get_current_identity(lua) {
                    if !iden.security_identity.get_security_contexts().has(SecurityContext::PLUGIN) {
                        return Some(Err(LuaError::RuntimeError("Source property is protected (PluginSecurity or higher)".into())));
                    }
                } else {
                    return Some(Err(LuaError::RuntimeError("Source property is protected (PluginSecurity or higher)".into())));
                }
                if let Err(err) = self.set_source(lua_setter!(opt_clone, lua, value)) {
                    return Some(Err(err));
                }
                Some(Ok(()))
            }
            _ => None
        }
    }

    // NOTE: The programmer of the instance needs to modify cloning here.
    fn clone(self: &RwLockReadGuard<'_, Self>, lua: &Lua, new_ptr: &WeakManagedInstance) -> LuaResult<Self> {
        let c = BaseScriptComponent {
            disabled: self.disabled,
            change_scheduled: None,
            run_context: self.run_context.clone(),
            actor: ActorLuauState::None,
            self_instance: new_ptr.clone(),
            source: self.source.clone(),
            connections: Vec::new(),
            has_set_up_destroying: false
        };
        get_task_scheduler_from_lua(lua)
            .defer_native(lua, new_ptr.upgrade().unwrap(), Synchronized, move |lua, script: ManagedInstance| {
                let script = script.cast_from_unsized::<dyn IBaseScript>().unwrap();
                let res = script.get_base_script_component_mut()
                    .schedule_start_if_not_started(lua, script.implicit_run_context());
                res
            }
        )?;
        Ok(c)
    }

    fn new(ptr: WeakManagedInstance, _class_name: &'static str) -> Self {
        BaseScriptComponent {
            disabled: true,
            change_scheduled: None,
            run_context: RunContext::Legacy,
            actor: ActorLuauState::None,
            self_instance: ptr,
            source: String::new(),
            connections: Vec::new(),
            has_set_up_destroying: false
        }
    }
}

impl dyn IBaseScript {
    fn get_source(&self) -> String {
        self.get_base_script_component().source.clone()
    }
    fn set_source(&self, lua: &Lua, source: String) -> LuaResult<()> {
        self.get_base_script_component_mut().set_source(source.clone())?;
        InstanceComponent::emit_property_changed(
            &self.get_instance_component(), lua, "Source", &lua_getter!(lua, source)?)
    }
    fn get_disabled(&self) -> bool {
        self.get_base_script_component().disabled
    }
    fn set_disabled(&self, lua: &Lua, disabled: bool) -> LuaResult<()> {
        self.get_base_script_component_mut().set_disabled(lua, disabled, self.implicit_run_context())?;
        InstanceComponent::emit_property_changed(
            &self.get_instance_component(), lua, "Disabled", &lua_getter!(lua, disabled)?)?;
        InstanceComponent::emit_property_changed(
            &self.get_instance_component(), lua, "Enabled", &lua_getter!(lua, !disabled)?)
    }
    fn get_run_context(&self) -> RunContext {
        self.get_base_script_component().run_context
    }
    fn set_run_context(&self, lua: &Lua, run_context: RunContext) -> LuaResult<()> {
        self.get_base_script_component_mut().run_context = run_context;
        InstanceComponent::emit_property_changed(
            &self.get_instance_component(), lua, "RunContext", &lua_getter!(lua, run_context)?)
    }
}

//pub struct ScriptComponent {}
#[derive(Debug)]
pub struct Script {
    instance: RwLock<InstanceComponent>,
    base_script: RwLock<BaseScriptComponent>
}

impl InheritanceBase for Script {
    fn inheritance_table(&self) -> crate::core::InheritanceTable {
        InheritanceTableBuilder::new()
            .insert_type::<Script, dyn IObject>(|x| x, |x| x)
            .insert_type::<Script, dyn IInstance>(|x| x, |x| x)
            .insert_type::<Script, dyn IBaseScript>(|x| x, |x| x)
            .output()
    }
}
impl IObject for Script {
    fn lua_get(&self, lua: &Lua, name: String) -> LuaResult<LuaValue> {
        self.base_script.read().unwrap().lua_get(self, lua, &name)
            .unwrap_or_else(|| self.instance.read().unwrap().lua_get(lua, &name))
    }

    fn get_class_name(&self) -> &'static str { "Script" }

    fn get_property_changed_signal(&self, property: String) -> ManagedRBXScriptSignal {
        self.instance.read().unwrap().get_property_changed_signal(property).unwrap()
    }

    fn is_a(&self, class_name: &String) -> bool {
        match class_name.as_str() {
            "Object" |
            "Instance" |
            "LuaSourceContainer" |
            "BaseScript" |
            "Script" => true,
            _ => false
        }
    }

    fn get_changed_signal(&self) -> ManagedRBXScriptSignal {
        self.instance.read().unwrap().changed.clone()
    }
}

impl IInstance for Script {
    fn get_instance_component(&self) -> RwLockReadGuard<InstanceComponent> {
        self.instance.read().unwrap()
    }

    fn get_instance_component_mut(&self) -> RwLockWriteGuard<InstanceComponent> {
        self.instance.write().unwrap()
    }

    fn lua_set(&self, lua: &Lua, name: String, val: LuaValue) -> LuaResult<()> {
        self.base_script.write().unwrap().lua_set(self, lua, &name, &val)
            .unwrap_or_else(|| self.instance.write().unwrap().lua_set(lua, &name, val))
    }

    fn clone_instance(&self, lua: &Lua) -> LuaResult<ManagedInstance> {
        Ok(Irc::new_cyclic_fallable::<_, LuaError>(|x| {
            let i = x.cast_to_instance();
            let script = Script {
                instance: RwLock::new_with_flag_auto(self.get_instance_component().clone(lua, &i)?),
                base_script: RwLock::new_with_flag_auto(self.get_base_script_component().clone(lua, &i)?),
            };
            Ok(script)
        })?.cast_from_sized().unwrap())
    }

    fn get_actor(&self) -> LuaResult<Option<ManagedInstance>> {
        match &self.get_base_script_component().actor {
            ActorLuauState::Actor(x) => Ok(Some(x.clone().cast_from_sized().unwrap())),
            _ => Ok(None)
        }
    }
}

impl IBaseScript for Script {
    fn get_base_script_component(&self) -> RwLockReadGuard<'_, BaseScriptComponent> {
        self.base_script.read().unwrap()
    }

    fn get_base_script_component_mut(&self) -> RwLockWriteGuard<'_, BaseScriptComponent> {
        self.base_script.write().unwrap()
    }

    fn implicit_run_context(&self) -> RunContext { RunContext::Server }
}

impl Script {
    pub fn new() -> ManagedInstance {
        Irc::new_cyclic(|x|
            Script {
                instance: RwLock::new_with_flag_auto(InstanceComponent::new(x.cast_to_instance(), "Script")),
                base_script: RwLock::new_with_flag_auto(BaseScriptComponent::new(x.cast_to_instance(), "Script"))
            }
        ).cast_from_sized().unwrap()
    }
}
#[derive(Debug)]
pub struct LocalScript {
    instance: RwLock<InstanceComponent>,
    base_script: RwLock<BaseScriptComponent>
}

impl InheritanceBase for LocalScript {
    fn inheritance_table(&self) -> crate::core::InheritanceTable {
        InheritanceTableBuilder::new()
            .insert_type::<LocalScript, dyn IObject>(|x| x, |x| x)
            .insert_type::<LocalScript, dyn IInstance>(|x| x, |x| x)
            .insert_type::<LocalScript, dyn IBaseScript>(|x| x, |x| x)
            .output()
    }
}
impl IObject for LocalScript {
    fn lua_get(&self, lua: &Lua, name: String) -> LuaResult<LuaValue> {
        self.base_script.read().unwrap().lua_get(self, lua, &name)
            .unwrap_or_else(|| self.instance.read().unwrap().lua_get(lua, &name))
    }

    fn get_class_name(&self) -> &'static str { "LocalScript" }

    fn get_property_changed_signal(&self, property: String) -> ManagedRBXScriptSignal {
        self.instance.read().unwrap().get_property_changed_signal(property).unwrap()
    }

    fn is_a(&self, class_name: &String) -> bool {
        match class_name.as_str() {
            "Object" |
            "Instance" |
            "LuaSourceContainer" |
            "BaseScript" |
            "Script" |
            "LocalScript" => true,
            _ => false
        }
    }

    fn get_changed_signal(&self) -> ManagedRBXScriptSignal {
        self.instance.read().unwrap().changed.clone()
    }
}

impl IInstance for LocalScript {
    fn get_instance_component(&self) -> RwLockReadGuard<InstanceComponent> {
        self.instance.read().unwrap()
    }

    fn get_instance_component_mut(&self) -> RwLockWriteGuard<InstanceComponent> {
        self.instance.write().unwrap()
    }

    fn lua_set(&self, lua: &Lua, name: String, val: LuaValue) -> LuaResult<()> {
        self.base_script.write().unwrap().lua_set(self, lua, &name, &val)
            .unwrap_or_else(|| self.instance.write().unwrap().lua_set(lua, &name, val))
    }

    fn clone_instance(&self, lua: &Lua) -> LuaResult<ManagedInstance> {
        Ok(Irc::new_cyclic_fallable::<_, LuaError>(|x| {
            let i = x.cast_to_instance();
            let script = LocalScript {
                instance: RwLock::new_with_flag_auto(self.get_instance_component().clone(lua, &i)?),
                base_script: RwLock::new_with_flag_auto(self.get_base_script_component().clone(lua, &i)?),
            };
            Ok(script)
        })?.cast_from_sized().unwrap())
    }

    fn get_actor(&self) -> LuaResult<Option<ManagedInstance>> {
        match &self.get_base_script_component().actor {
            ActorLuauState::Actor(x) => Ok(Some(x.clone().cast_from_sized().unwrap())),
            _ => Ok(None)
        }
    }
}

impl IBaseScript for LocalScript {
    fn get_base_script_component(&self) -> RwLockReadGuard<'_, BaseScriptComponent> {
        self.base_script.read().unwrap()
    }

    fn get_base_script_component_mut(&self) -> RwLockWriteGuard<'_, BaseScriptComponent> {
        self.base_script.write().unwrap()
    }

    fn implicit_run_context(&self) -> RunContext { RunContext::Client }
}

impl LocalScript {
    pub fn new() -> ManagedInstance {
        Irc::new_cyclic(|x|
            LocalScript {
                instance: RwLock::new_with_flag_auto(InstanceComponent::new(x.cast_to_instance(), "LocalScript")),
                base_script: RwLock::new_with_flag_auto(BaseScriptComponent::new(x.cast_to_instance(), "LocalScript"))
            }
        ).cast_from_sized().unwrap()
    }
}
