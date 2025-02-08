use godot::{global::print_rich, meta::ToGodot};
use r2g_mlua::{ffi::lua_clock, prelude::*};
use std::fmt::Debug;

use crate::{
    core::{
        lua_macros::{lua_getter, lua_invalid_argument},
        DynInstance, IInstance, IInstanceComponent, IObject, InheritanceBase,
        InheritanceTableBuilder, InstanceComponent, InstanceCreationMetadata, Irc, ManagedInstance,
        RwLock, RwLockReadGuard, RwLockWriteGuard,
    },
    userdata::{enums::MessageType, ManagedRBXScriptSignal, RBXScriptSignal},
};

struct LogServiceComponent {
    logs: Vec<(String, MessageType, f64)>,
    hooks: Vec<Box<dyn Fn(Option<(String, MessageType, f64)>)>>,

    pub message_out: ManagedRBXScriptSignal,
}

impl Debug for LogServiceComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogServiceComponent")
            .field("logs", &self.logs)
            .finish()
    }
}

#[derive(Debug)]
pub struct LogService {
    instance: RwLock<InstanceComponent>,
    log_service: RwLock<LogServiceComponent>,
}

impl IInstanceComponent for LogServiceComponent {
    fn lua_get(
        self: &mut RwLockReadGuard<'_, Self>,
        _ptr: &DynInstance,
        lua: &Lua,
        key: &String,
    ) -> Option<LuaResult<LuaValue>> {
        match key.as_str() {
            "ClearOutput" => lua_getter!(function_opt, lua, |_, obj: ManagedInstance| {
                obj.cast_from_unsized::<LogService>()
                    .map_err(|_| lua_invalid_argument!("LogService::ClearOutput", 1, self cast Instance to LogService))
                    .map(|x| x.clear_logs())
            }),
            "GetLogHistory" => lua_getter!(function_opt, lua, |lua, obj: ManagedInstance| {
                obj.cast_from_unsized::<LogService>()
                    .map_err(|_| lua_invalid_argument!("LogService::GetLogHistory", 1, self cast Instance to LogService))
                    .map(|x| {
                        let write = x.log_service.read().unwrap();
                        let tbl = lua.create_table().unwrap();
                        for (msg, msg_type, timestamp) in write.logs.iter() {
                            let entry = lua.create_table().unwrap();
                            entry.push(msg.clone()).unwrap();
                            entry.push(*msg_type).unwrap();
                            entry.push(*timestamp).unwrap();
                            tbl.push(entry).unwrap();
                        }
                        tbl
                    })
            }),
            "MessageOut" => Some(lua_getter!(clone, lua, self.message_out)),
            _ => None,
        }
    }

    fn lua_set(
        self: &mut RwLockWriteGuard<'_, Self>,
        _ptr: &DynInstance,
        _lua: &Lua,
        _key: &String,
        _value: &LuaValue,
    ) -> Option<LuaResult<()>> {
        None
    }

    fn clone(
        self: &RwLockReadGuard<'_, Self>,
        _: &Lua,
        _: &InstanceCreationMetadata,
    ) -> LuaResult<Self> {
        Err(LuaError::RuntimeError(
            "Cannot clone LogServiceComponent".into(),
        ))
    }

    fn new(metadata: &InstanceCreationMetadata) -> Self {
        LogServiceComponent {
            logs: Vec::new(),
            hooks: Vec::new(),
            message_out: RBXScriptSignal::new(metadata),
        }
    }
}

impl InheritanceBase for LogService {
    fn inheritance_table(&self) -> crate::core::InheritanceTable {
        InheritanceTableBuilder::new()
            .insert_type::<LogService, dyn IObject>(|x| x, |x| x)
            .insert_type::<LogService, dyn IInstance>(|x| x, |x| x)
            .insert_type::<LogService, LogService>(|x| x, |x| x)
            .output()
    }
}

impl IObject for LogService {
    fn lua_get(&self, lua: &Lua, name: String) -> LuaResult<LuaValue> {
        self.log_service
            .read()
            .unwrap()
            .lua_get(self, lua, &name)
            .unwrap_or_else(|| self.instance.read().unwrap().lua_get(lua, &name))
    }

    fn get_class_name(&self) -> &'static str {
        "LogService"
    }

    fn get_property_changed_signal(&self, property: String) -> ManagedRBXScriptSignal {
        self.instance
            .read()
            .unwrap()
            .get_property_changed_signal(property)
            .unwrap()
    }

    fn is_a(&self, class_name: &String) -> bool {
        match class_name.as_str() {
            "Object" | "Instance" | "LogService" => true,
            _ => false,
        }
    }

    fn get_changed_signal(&self) -> ManagedRBXScriptSignal {
        self.instance.read().unwrap().changed.clone()
    }
}

impl IInstance for LogService {
    fn get_instance_component(&self) -> RwLockReadGuard<InstanceComponent> {
        self.instance.read().unwrap()
    }

    fn get_instance_component_mut(&self) -> RwLockWriteGuard<InstanceComponent> {
        self.instance.write().unwrap()
    }

    fn lua_set(&self, lua: &Lua, name: String, val: LuaValue) -> LuaResult<()> {
        self.log_service
            .write()
            .unwrap()
            .lua_set(self, lua, &name, &val)
            .unwrap_or_else(|| self.instance.write().unwrap().lua_set(lua, &name, val))
    }

    fn clone_instance(&self, _lua: &Lua) -> LuaResult<ManagedInstance> {
        Err(LuaError::RuntimeError("Cannot clone LogService".into()))
    }
}

impl LogService {
    pub fn new() -> Irc<LogService> {
        let inst = Irc::new_cyclic(|x| {
            let mut metadata = InstanceCreationMetadata::new("LogService", x.cast_to_instance());
            let mut l = LogService {
                instance: RwLock::new_with_flag_auto(InstanceComponent::new(&mut metadata)),
                log_service: RwLock::new_with_flag_auto(LogServiceComponent::new(&mut metadata)),
            };
            DynInstance::submit_metadata(&mut l, metadata);
            l
        });
        inst.add_hook(|x| {
            if let Some((msg, msg_type, _timestamp)) = x {
                match msg_type {
                    MessageType::MessageOutput => print_rich(&[msg.to_variant()]),
                    MessageType::MessageInfo => {
                        print_rich(&[format!("[color=blue]{}[/color]", msg).to_variant()])
                    }
                    MessageType::MessageWarning => {
                        print_rich(&[format!("[color=yellow]{}[/color]", msg).to_variant()])
                    }
                    MessageType::MessageError => {
                        print_rich(&[format!("[color=red]{}[/color]", msg).to_variant()])
                    }
                }
            }
        });
        inst
    }
    pub fn clear_logs(&self) {
        let mut write = self.log_service.write().unwrap();
        write.logs.clear();
        for hook in write.hooks.iter() {
            hook(None);
        }
    }
    pub fn add_hook(&self, hook: impl Fn(Option<(String, MessageType, f64)>) + 'static) {
        self.log_service.write().unwrap().hooks.push(Box::new(hook));
    }
    pub fn log_message(&self, lua: &Lua, message: String) {
        let mut write = self.log_service.write().unwrap();
        let timestamp = unsafe { lua_clock() };
        write
            .logs
            .push((message.clone(), MessageType::MessageOutput, timestamp));
        let message_out_event = write.message_out.clone();
        for hook in write.hooks.iter() {
            hook(Some((
                message.clone(),
                MessageType::MessageOutput,
                timestamp,
            )));
        }
        drop(write);
        message_out_event
            .write()
            .fire(lua, (message, MessageType::MessageOutput))
            .unwrap();
    }
    pub fn log_info(&self, lua: &Lua, message: String) {
        let mut write = self.log_service.write().unwrap();
        let timestamp = unsafe { lua_clock() };
        write
            .logs
            .push((message.clone(), MessageType::MessageInfo, timestamp));
        let message_out_event = write.message_out.clone();
        for hook in write.hooks.iter() {
            hook(Some((message.clone(), MessageType::MessageInfo, timestamp)));
        }
        drop(write);
        message_out_event
            .write()
            .fire(lua, (message, MessageType::MessageInfo))
            .unwrap();
    }
    pub fn log_warn(&self, lua: &Lua, message: String) {
        let mut write = self.log_service.write().unwrap();
        let timestamp = unsafe { lua_clock() };
        write
            .logs
            .push((message.clone(), MessageType::MessageWarning, timestamp));
        let message_out_event = write.message_out.clone();
        for hook in write.hooks.iter() {
            hook(Some((
                message.clone(),
                MessageType::MessageWarning,
                timestamp,
            )));
        }
        drop(write);
        message_out_event
            .write()
            .fire(lua, (message, MessageType::MessageWarning))
            .unwrap();
    }
    pub fn log_err(&self, lua: &Lua, message: String) {
        let mut write = self.log_service.write().unwrap();
        let timestamp = unsafe { lua_clock() };
        write
            .logs
            .push((message.clone(), MessageType::MessageError, timestamp));
        let message_out_event = write.message_out.clone();
        for hook in write.hooks.iter() {
            hook(Some((
                message.clone(),
                MessageType::MessageError,
                timestamp,
            )));
        }
        drop(write);
        message_out_event
            .write()
            .fire(lua, (message, MessageType::MessageError))
            .unwrap();
    }
}

pub(crate) fn escape_bbcode_and_format(msg: String, msg_type: MessageType) -> String {
    let msg = msg.replace("[", "[lb]");
    match msg_type {
        MessageType::MessageOutput => msg,
        MessageType::MessageInfo => format!("[color=blue]{}[/color]", msg),
        MessageType::MessageWarning => format!("[color=orange]{}[/color]", msg),
        MessageType::MessageError => format!("[color=red]{}[/color]", msg),
    }
}
