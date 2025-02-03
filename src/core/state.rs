use std::mem::MaybeUninit;
use std::{collections::HashMap, ffi::c_void, mem::transmute, ptr::addr_of_mut};
use std::ptr::null_mut;

use godot::global::godot_print;
use r2g_mlua::{prelude::*, ChunkMode, Compiler};
use super::scheduler::ITaskScheduler;
use super::ParallelDispatch::{Default, Synchronized};
use super::{borrowck_ignore, FastFlag, FastFlags, RwLock, RwLockReadGuard, RwLockWriteGuard, TaskScheduler, Trc};
use super::{security::ThreadIdentityType, vm::RobloxVM};
use crate::instance::WeakManagedInstance;
use crate::userdata::register_userdata_singletons;

pub mod registry_keys {
    pub const VM_REGISTRYKEY: &'static str = "__vm__";
    pub const STATE_REGISTRYKEY: &'static str = "__state__";
    pub(super) const TASK_PUSH_WAIT: &'static str = "__task_push_wait__";
    pub(super) const TASK_PUSH_SYNC_DESYNC: &'static str = "__task_push_sync_desync__";
}
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ThreadIdentity {
    pub security_identity: ThreadIdentityType,
    pub script: Option<WeakManagedInstance>,
}
#[derive(Debug)]
pub struct LuauState {
    vm: *mut RwLock<RobloxVM>,
    lua: Lua,
    threads: HashMap<*const c_void, ThreadIdentity>,
    task: MaybeUninit<Box<dyn ITaskScheduler>>
}
impl LuauState {
    fn get_vm_from_lua(lua: &Lua) -> &RwLock<RobloxVM> {
        unsafe {
            let ptr: *mut RwLock<RobloxVM> = transmute(
                lua.named_registry_value::<LuaLightUserData>(registry_keys::VM_REGISTRYKEY)
                .unwrap_unchecked().0
            );
            debug_assert!(!ptr.is_null());
            ptr.as_mut().unwrap_unchecked()
        }
    }
    fn thread_event_callback(lua: &Lua, event: LuaThreadEventInfo) -> Result<(), LuaError> {
        let is_running = lua.gc_is_running();
        if is_running {
            lua.gc_stop();
        }
        let state;
        unsafe {
            let reg: LuaLightUserData = lua.named_registry_value(registry_keys::STATE_REGISTRYKEY)?;
            state = reg.0.cast::<LuauState>().as_mut().unwrap();
        }
        match event {
            LuaThreadEventInfo::Created(parent) => {
                godot_print!("[thread_events] new thread created: thread: 0x{:x} by thread: 0x{:x}",lua.current_thread().to_pointer() as isize,parent.to_pointer() as isize);
                let iden = state.threads.get(&parent.to_pointer());
                if iden.is_some() {
                    state.threads.insert(lua.current_thread().to_pointer(), iden.unwrap().clone());
                }
            }
            LuaThreadEventInfo::Destroyed(thread_ptr) => {
                godot_print!("[thread_events] thread destroyed: thread: 0x{:x}",thread_ptr as isize);
                state.threads.remove(&thread_ptr);
            }
        }
        if is_running {
            lua.gc_restart();
        }
        Ok(())
    }
    pub fn get_thread_identity_pointer(&self, thread: *const c_void) -> Option<&ThreadIdentity> {
        self.threads.get(&thread)
    }
    pub fn get_thread_identity_pointer_pointer_mut(&mut self, thread: *const c_void) -> Option<&mut ThreadIdentity> {
        self.threads.get_mut(&thread)
    }
    pub fn get_thread_identity(&self, thread: LuaThread) -> Option<&ThreadIdentity> {
        self.get_thread_identity_pointer(thread.to_pointer().cast())
    }
    pub fn get_thread_identity_mut(&mut self, thread: LuaThread) -> Option<&mut ThreadIdentity> {
        self.get_thread_identity_pointer_pointer_mut(thread.to_pointer().cast())
    }
    pub fn set_thread_identity(&mut self, thread: LuaThread, identity: ThreadIdentity) {
        self.threads.insert(thread.to_pointer().cast(), identity);
    }
    unsafe fn register_globals(&mut self) {
        self.lua.globals().raw_set("print", self.lua.create_function(|lua, args: LuaMultiValue| {
            let vm = Self::get_vm_from_lua(lua);
            vm.read().unwrap().log_message(args);
            Ok(())
        }).unwrap()).unwrap();
        self.lua.globals().raw_set("warn", self.lua.create_function(|lua, args: LuaMultiValue| {
            let vm = Self::get_vm_from_lua(lua);
            vm.read().unwrap().log_warn(args);
            Ok(())
        }).unwrap()).unwrap();
        self.lua.globals().raw_set("game", self.vm.as_ref().unwrap_unchecked().read().unwrap().get_game_instance()).unwrap();
        // Task scheduler registration
        {
            type DynTaskScheduler = dyn ITaskScheduler;
            let task = self.lua.create_table().unwrap();
            task.raw_set("spawn", self.lua.create_function(|lua, (thread_or_func,mv): (LuaValue, LuaMultiValue)| {
                match thread_or_func {
                    LuaValue::Thread(thread) => get_state(borrowck_ignore(lua)).get_task_scheduler().spawn_thread(thread),
                    LuaValue::Function(func) => get_state(borrowck_ignore(lua)).get_task_scheduler().spawn_func(lua, func, mv),
                    _ => Err(LuaError::RuntimeError("invalid argument #1 to 'spawn' (thread or function expected)".into()))
                }
            }).unwrap()).unwrap();
            task.raw_set("defer", self.lua.create_function(|lua, (thread_or_func,mv): (LuaValue, LuaMultiValue)| {
                match thread_or_func {
                    LuaValue::Thread(thread) => get_state(borrowck_ignore(lua)).get_task_scheduler_mut().defer_thread(thread, Default),
                    LuaValue::Function(func) => get_state(borrowck_ignore(lua)).get_task_scheduler_mut().defer_func(lua, func, mv, Synchronized),
                    _ => Err(LuaError::RuntimeError("invalid argument #1 to 'defer' (thread or function expected)".into()))
                }
            }).unwrap()).unwrap();
            task.raw_set("delay", self.lua.create_function(|lua, (time,thread_or_func,mv): (f64,LuaValue,LuaMultiValue)| {
                match thread_or_func {
                    LuaValue::Thread(thread) => get_state(borrowck_ignore(lua)).get_task_scheduler_mut().delay_thread(thread, Default, time),
                    LuaValue::Function(func) => get_state(borrowck_ignore(lua)).get_task_scheduler_mut().delay_func(lua, func, mv, Synchronized, time),
                    _ => Err(LuaError::RuntimeError("invalid argument #2 to 'delay' (thread or function expected)".into()))
                }
            }).unwrap()).unwrap();
            task.raw_set("cancel", self.lua.create_function(|lua, thread: LuaThread| {
                get_state(borrowck_ignore(lua)).get_task_scheduler_mut().cancel(lua, &thread)
            }).unwrap()).unwrap();
            task.raw_set("synchronize", self.lua.create_c_function(DynTaskScheduler::synchronize).unwrap()).unwrap();
            task.raw_set("desynchronize", self.lua.create_c_function(DynTaskScheduler::desynchronize).unwrap()).unwrap();
            task.raw_set("wait", self.lua.create_c_function(DynTaskScheduler::wait).unwrap()).unwrap();
            task.raw_set("synchronize", self.lua.create_c_function(DynTaskScheduler::synchronize).unwrap()).unwrap();
            task.raw_set("desynchronize", self.lua.create_c_function(DynTaskScheduler::desynchronize).unwrap()).unwrap();
            task.set_readonly(true);

            self.lua.globals().raw_set("task", task).unwrap();
            self.lua.set_named_registry_value("task", self.lua.globals().raw_get::<LuaValue>("task").unwrap()).unwrap();

            self.lua.set_named_registry_value(
                registry_keys::TASK_PUSH_WAIT,
                self.lua.create_function(DynTaskScheduler::push_wait).unwrap()
            ).unwrap();
            self.lua.set_named_registry_value(
                registry_keys::TASK_PUSH_SYNC_DESYNC,
                self.lua.create_function(DynTaskScheduler::push_sync_desync).unwrap()
            ).unwrap()
        }

    }
    unsafe fn _init(&mut self) {
        self.lua.set_named_registry_value(registry_keys::VM_REGISTRYKEY, LuaLightUserData(self.vm.cast())).unwrap();
        let state_userdata = LuaLightUserData(addr_of_mut!(*self).cast());
        self.lua.set_named_registry_value(registry_keys::STATE_REGISTRYKEY, state_userdata).unwrap();
        
        self.register_globals();
        
        self.lua.sandbox(true).unwrap();
        self.lua.enable_jit(false);
        register_userdata_singletons(&mut self.lua).unwrap();
        self.lua.globals().set_readonly(self.flags().get_bool(FastFlag::GlobalsReadonly));
        self.lua.set_thread_event_callback(Self::thread_event_callback);
        self.lua.gc_stop();
    }
    pub(super) unsafe fn init(&mut self, ptr: *mut RwLock<RobloxVM>, task: Box<dyn ITaskScheduler>) {
        self.vm = ptr;
        self.task = MaybeUninit::new(task);
        self._init();
    }
    #[doc(hidden)]
    pub(super) fn new(ptr: *mut RwLock<RobloxVM>) -> LuauState {
        let mut state = LuauState {
            vm: ptr,
            lua: Lua::new(),
            threads: HashMap::default(),
            task: MaybeUninit::new(Box::new(TaskScheduler::new()))
        };
        unsafe {state._init();}
        state
    }
    pub(super) unsafe fn new_uninit() -> LuauState {
        LuauState {
            vm: null_mut(),
            lua: Lua::new(),
            threads: HashMap::default(),
            task: MaybeUninit::uninit()
        }
    }
    pub fn get_vm(&self) -> RwLockReadGuard<RobloxVM> {
        unsafe {
            self.vm.as_ref().unwrap_unchecked().read().unwrap()
        }
    }
    pub fn get_vm_mut(&self) -> RwLockWriteGuard<RobloxVM> {
        unsafe {
            self.vm.as_ref().unwrap_unchecked().write().unwrap()
        }
    }

    pub(super) unsafe fn watchdog_check(&self) -> bool {
        self.vm.as_ref().unwrap_unchecked().access().as_ref().unwrap_unchecked().watchdog_check()
    }

    // Mutable borrow is forced here to prevent modifying the lua state with a read-only borrow.
    pub fn get_lua(&mut self) -> &Lua {
        &self.lua
    }
    pub fn get_userdata_types() -> &'static [&'static str] {
        /*&[
            "Axes","BrickColor","CatalogSearchParams","CFrame","ColorSequence","ColorSequenceKeypoint","Content","DateTime",
            "DockWidgetPluginGuiInfo","Enum","EnumItem","Enums","Faces","FloatCurveKey","Font","Instance","NumberRange",
            "NumberSequence","NumberSequenceKeypoint","OverlapParams","Path2DControlPoint","PathWaypoint","PhysicalProperties",
            "Random","Ray","RaycastParams","RaycastResult","RBXScriptConnection","RBXScriptSignal","Rect","Region3","Region3int16",
            "RotationCurveKey","Secret","SharedTable","TweenInfo","UDim","UDim2","Vector2","Vector2int16","Vector3","Vector3int16"
        ]*/ // sadly doesnt work
        &["Instance"] // good enough
    }
    fn get_debug_compiler() -> Compiler {
        Compiler::new()
            .set_debug_level(2)
            .set_optimization_level(1)
            .set_userdata_types(Self::get_userdata_types().into_iter().map(|x| String::from(*x)).collect())
    }
    fn get_release_compiler() -> Compiler {
        Compiler::new()
            .set_debug_level(1)
            .set_optimization_level(2)
            .set_userdata_types(Self::get_userdata_types().into_iter().map(|x| String::from(*x)).collect())
            .set_type_info_level(1)
    }
    pub fn compile_jit(&mut self, chunk_name: &str, chunk: &str, env: LuaTable) -> LuaResult<LuaFunction> {
        let v = Self::get_release_compiler()
            .compile(chunk)?;
        self.lua.enable_jit(true);
        let f = self.lua
            .load(v)
            .set_name(chunk_name)
            .set_mode(ChunkMode::Binary)
            .set_environment(env)
            .into_function();
        self.lua.enable_jit(false);
        f
    }
    pub fn compile_release(&mut self, chunk_name: &str, chunk: &str, env: LuaTable) -> LuaResult<LuaFunction> {
        let v = Self::get_release_compiler()
            .compile(chunk)?;
        self.lua.load(v)
            .set_name(chunk_name)
            .set_mode(ChunkMode::Binary)
            .set_environment(env)
            .into_function()
    }
    pub fn compile_debug(&mut self, chunk_name: &str, chunk: &str, env: LuaTable) -> LuaResult<LuaFunction> {
        let v = Self::get_debug_compiler()
            .compile(chunk)?;
        self.lua.load(v)
            .set_name(chunk_name)
            .set_mode(ChunkMode::Binary)
            .set_environment(env)
            .into_function()
    }
    pub fn create_env_from_global(&mut self) -> LuaResult<LuaTable> {
        let lua = self.get_lua();
        let metatable = lua.create_table()?;
        metatable.raw_set("__metatable", "env")?;
        metatable.raw_set("__index", lua.globals())?;

        let table = lua.create_table()?;
        table.set_metatable(Some(metatable));

        Ok(table)
    }
    #[inline]
    pub fn get_task_scheduler_mut(&mut self) -> &mut dyn ITaskScheduler {
        // SAFETY: The state must be initialized
        unsafe { &mut **self.task.assume_init_mut() }
    }
    #[inline]
    pub fn get_task_scheduler(&self) -> &dyn ITaskScheduler {
        // SAFETY: The state must be initialized
        unsafe { &**self.task.assume_init_ref() }
    }
    #[inline(always)]
    pub const fn flags(&self) -> &'static FastFlags {
        FastFlags::from_vm(self.vm)
    }
    pub fn gc(&self) {
        self.lua.gc_restart();
        self.lua.gc_collect().unwrap();
        self.lua.gc_stop();
    }
    pub fn gc_step(&self, kb: i32) {
        self.lua.gc_restart();
        self.lua.gc_step_kbytes(kb).unwrap();
        self.lua.gc_stop();
    }
    /// Gets the RobloxVM pointer. This is thread-safe even with `.access()`.
    #[inline(always)]
    pub(super) const fn get_vm_ptr(&self) -> *mut RwLock<RobloxVM> {
        self.vm
    }
}

impl Drop for LuauState {
    fn drop(&mut self) {
        unsafe { self.task.assume_init_drop() };
    }
}

pub fn get_current_identity(l: &Lua) -> Option<&mut ThreadIdentity> {
    let state;
    unsafe {
        let reg: Option<LuaLightUserData> = l.named_registry_value(registry_keys::STATE_REGISTRYKEY).ok();
        if reg.is_none() {
            return None;
        }
        state = reg.unwrap_unchecked().0.cast::<LuauState>().as_mut().unwrap();
    }
    state.threads.get_mut(&l.current_thread().to_pointer().cast())
}
pub fn get_thread_identity<'a, 'b>(l: &'a Lua, thread: &'b LuaThread) -> Option<&'b ThreadIdentity> {
    let state;
    unsafe {
        let reg: Option<LuaLightUserData> = l.named_registry_value(registry_keys::STATE_REGISTRYKEY).ok();
        if reg.is_none() {
            return None;
        }
        state = reg.unwrap_unchecked().0.cast::<LuauState>().as_mut().unwrap();
    }
    state.threads.get(&thread.to_pointer().cast())
}
// SAFETY: This function is safe because lua instances can be obtained only if its inside of it or if it already holds a lock.
pub fn get_state(l: &Lua) -> &mut LuauState {
    let state;
    unsafe {
        let reg = l.named_registry_value::<LuaLightUserData>(registry_keys::STATE_REGISTRYKEY);
        state = reg.unwrap().0.cast::<LuauState>().as_mut().unwrap();
    }
    state
}
pub fn get_state_with_rwlock(l: &Lua) -> &Trc<LuauState> {
    let state = get_state(l);
    let ptr = &raw mut *state;
    let vm = state.get_vm();
    vm.get_state_with_rwlock(ptr).map(|x| unsafe {&*x}).unwrap()
}