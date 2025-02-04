use std::{collections::HashMap, mem::transmute};

use bevy_reflect::Typed;
use godot::{classes::{node::InternalMode, Engine, FileAccess, RichTextLabel}, global::Error, prelude::*};

use crate::{core::{borrowck_ignore_mut, get_state, FastFlag, FastFlagValue, GlobalTaskScheduler, ParallelDispatch::Synchronized, RblxVM, RwLock, ThreadIdentity, ThreadIdentityType}, instance::escape_bbcode_and_format};

/// The RblxVM node, holding either a client or a server state, depending on the startup flags.
/// 
/// [b]Note:[/b] This object is not thread-safe. It should from only a single thread.
#[derive(GodotClass)]
#[class(base=Node,rename=RblxVM)]
pub struct RblxVMNode {
    vm: Option<Box<RwLock<RblxVM>>>,
    /// The fast flags loaded on startup.
    /// [b]Note:[/b] This is only loaded on startup! At runtime, you have to use the [method set_fast_flag_async] and [method get_fast_flag] methods.
    #[export]
    startup_flags: Dictionary,

    base: Base<Node>,
}

#[godot_api]
impl INode for RblxVMNode {
    fn init(owner: Base<Node>) -> Self {
        let mut dict = Dictionary::default();
        for (flag, name) in FastFlag::type_info().as_enum().unwrap().variant_names().iter().enumerate() {
            let _ = dict.insert(*name, match FastFlag::get_default(unsafe {transmute(flag as u16)}) {
                FastFlagValue::Bool(v) => v.to_variant(),
                FastFlagValue::Int(v) => v.to_variant(),
                FastFlagValue::Float(v) => v.to_variant(),
                FastFlagValue::String(v) => v.to_variant(),
            });
        }
        RblxVMNode {
            vm: None,
            startup_flags: dict,
            base: owner,
        }
    }

    fn ready(&mut self) {
        if Engine::singleton().is_editor_hint() {
            return;
        }
        let variants: HashMap<&str, usize> = FastFlag::type_info().as_enum().unwrap().variant_names()
            .iter().enumerate()
            .map(|(id, name)| (*name, id))
            .fold(HashMap::new(), |mut v,  y| {
                v.insert(y.0, y.1);
                v
            });
        let mut flags_table: Vec<(FastFlag, FastFlagValue)> = Vec::new();
        for (k,v) in self.startup_flags.iter_shared() {
            if let Err(e) = (|| {
                let k = k.try_to::<GString>().map_err(|_| "key in dictionary is not string")?;
                let fastflag_id = *variants.get(k.to_string().as_str()).ok_or("key is not FastFlag name")? as u16;
                match v.get_type() {
                    VariantType::BOOL =>
                        flags_table.push((unsafe{ transmute(fastflag_id) }, FastFlagValue::Bool(v.to::<bool>()))),
                    VariantType::INT =>
                        flags_table.push((unsafe{ transmute(fastflag_id) }, FastFlagValue::Int(v.to::<i64>()))),
                    VariantType::FLOAT =>
                        flags_table.push((unsafe{ transmute(fastflag_id) }, FastFlagValue::Float(v.to::<f64>()))),
                    VariantType::STRING =>
                        flags_table.push((unsafe{ transmute(fastflag_id) }, FastFlagValue::String(v.to::<GString>().to_string()))),
                    _ => return Err("expected value to be bool, int, uint, float or string")
                }
                Ok(())
            })() {
                godot_error!("RblxVMNode: {}", e);
            }
        }
        
        self.vm = Some(RblxVM::new(Some(flags_table)));
        self.post_init();
    }

    fn process(&mut self, delta: f64) {
        if let Some(vm) = self.vm.as_mut() {
            let write = vm.write()
                .inspect_err(|_| godot_error!("RblxVMNode: failed to acquire write lock on RblxVM"))
                .unwrap();
            GlobalTaskScheduler::frame_step(write, delta).unwrap();
        }
    }
}

impl RblxVMNode {
    fn post_init(&mut self) {
        let scene = load::<PackedScene>("res://addons/rblx-godot/ConsoleInterface.tscn");
        let loaded_scene = scene.instantiate().unwrap();

        self.base_mut().add_child_ex(&loaded_scene)
            .internal(InternalMode::FRONT)
            .done();

        let mut log_window = self.base_mut().get_node_as::<RichTextLabel>("ConsoleInterface/ConsoleInterface/PanelContainer/RichTextLabel");
        
        let vm = self.vm.as_ref().unwrap();
        let read = vm.read()
            .inspect_err(|_| godot_error!("RblxVMNode: failed to acquire write lock on RblxVM"))
            .map_err(|_| Error::ERR_CANT_ACQUIRE_RESOURCE)
            .unwrap();

        let add_callable = log_window.callable("append_text");

        let clear_callable = log_window.callable("clear");
        read.get_log_service().add_hook(move |str| {
            if let Some((msg, msg_type, _)) = str {
                add_callable.call_deferred(&[escape_bbcode_and_format(msg, msg_type).to_variant()]);
                add_callable.call_deferred(&["\n".to_variant()]);
            } else {
                clear_callable.call_deferred(&[]);
            }
        });

        // Read current log file from godot.
        let log_content = FileAccess::get_file_as_string("user://logs/godot.log");

        log_window.append_text(&log_content);
        log_window.append_text(include_str!("startup_message.rtf"));
    }
}

#[godot_api]
impl RblxVMNode {
    /// Sets a fast flag to a new value asynchronously, returns an error if the flag name is invalid.
    /// [b]Note:[/b] If provided an invalid type, it will panic inside the task scheduler.
    #[func]
    fn set_fast_flag_async(&self, flag: GString, value: Variant) -> Error {
        let variants: HashMap<&str, usize> = FastFlag::type_info().as_enum().unwrap()
            .variant_names().iter()
            .enumerate()
            .map(|(id, name)| (*name, id))
            .fold(HashMap::new(), |mut v,  y| {
                v.insert(y.0, y.1);
                v
            });
        (|| {
            if let Some(vm) = self.vm.as_ref() {
                let mut write = vm.write()
                    .inspect_err(|_| godot_error!("RblxVMNode: failed to acquire write lock on RblxVM"))
                    .map_err(|_| Error::ERR_CANT_ACQUIRE_RESOURCE)?;
                let fastflag_id = *variants.get(flag.to_string().as_str()).ok_or(Error::ERR_INVALID_PARAMETER)
                    .inspect_err(|_| godot_error!("RblxVMNode: invalid flag name provided"))? as u16;
                let fastflag_id = unsafe {transmute(fastflag_id)};
                let fastflag_value: FastFlagValue = match value.get_type() {
                    VariantType::BOOL => FastFlagValue::Bool(value.to::<bool>()),
                    VariantType::INT => FastFlagValue::Int(value.to::<i64>()),
                    VariantType::FLOAT => FastFlagValue::Float(value.to::<f64>()),
                    VariantType::STRING => FastFlagValue::String(value.to::<GString>().to_string()),
                    _ => return Err(Error::ERR_INVALID_PARAMETER)
                };
                let lua = write.get_main_state().get_lua();
                let func = lua.create_function_mut(move |lua, ()| {
                    match &fastflag_value {
                        FastFlagValue::Bool(v) => get_state(lua).flags().set_bool(fastflag_id, *v),
                        FastFlagValue::Int(v) => get_state(lua).flags().set_int(fastflag_id, *v),
                        FastFlagValue::Float(v) => get_state(lua).flags().set_float(fastflag_id, *v),
                        FastFlagValue::String(v) => get_state(lua).flags().set_string(fastflag_id, v.clone()),
                    }
                    Ok(())
                }).unwrap();
                unsafe { vm.access().as_mut().unwrap_unchecked() }.get_main_state()
                    .get_task_scheduler_mut()
                    .defer_func(lua, func, (), Synchronized)
                    .inspect_err(|_| godot_error!("RblxVMNode: failed to defer on task scheduler"))
                    .map_err(|_| Error::FAILED)?;
            }
            Ok(Error::OK)
        })().unwrap_or_else(|e| e)
    }
    /// Fetches a fast flag, if it doesn't exist it returns null.
    #[func]
    fn get_fast_flag(&self, flag: GString) -> Variant {
        let variants: HashMap<&str, usize> = FastFlag::type_info().as_enum().unwrap()
            .variant_names().iter()
            .enumerate()
            .map(|(id, name)| (*name, id))
            .fold(HashMap::new(), |mut v,  y| {
                v.insert(y.0, y.1);
                v
            });
        (|| {
            if let Some(vm) = self.vm.as_ref() {
                let read = vm.read()
                    .inspect_err(|_| godot_error!("RblxVMNode: failed to acquire read lock on RblxVM"))
                    .map_err(|_| Variant::nil())?;
                let fastflag_id = *variants.get(flag.to_string().as_str()).ok_or_else(|| Variant::nil())
                    .inspect_err(|_| godot_error!("RblxVMNode: invalid flag name provided"))? as u16;
                let fastflag_id = unsafe {transmute(fastflag_id)};
                Ok(match read.flags().get_flag_value(fastflag_id) {
                    FastFlagValue::Bool(v) => v.to_variant(),
                    FastFlagValue::Int(v) => v.to_variant(),
                    FastFlagValue::Float(v) => v.to_variant(),
                    FastFlagValue::String(v) => v.to_variant(),
                })
            } else {
                godot_error!("RblxVMNode: RblxVM not initialized");
                Err(Variant::nil())
            }
        })().unwrap_or_else(|e| e)
    }
    /// Pushes Lua code to the task scheduler and runs it on the next deferred cycle.
    #[func]
    fn push_code(&mut self, chunk: GString) -> Error {
        self.vm.as_mut().map(|vm| {
            let mut write = vm.write()
                .inspect_err(|_| godot_error!("RblxVMNode: failed to acquire write lock on RblxVM"))
                .map_err(|_| Error::ERR_CANT_ACQUIRE_RESOURCE)
                .unwrap();
            let state = write.get_main_state();
            let env = state.create_env_from_global().unwrap();
            let func = state.compile_jit("<godot>", chunk.to_string().as_str(), env).unwrap();
            let lua = unsafe {(&raw const *state.get_lua()).as_ref().unwrap_unchecked()};
            let thr = unsafe { borrowck_ignore_mut(state) }.get_task_scheduler_mut()
                .defer_func(lua, func, (), Synchronized)
                .inspect_err(|_| godot_error!("RblxVMNode: failed to defer on task scheduler"))
                .unwrap();
            state.set_thread_identity(thr, ThreadIdentity {
                security_identity: ThreadIdentityType::UserInit,
                script: None
            });
        });
        Error::OK
    }
}