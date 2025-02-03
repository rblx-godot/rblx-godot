use std::{ffi::c_int, mem::take, ptr::slice_from_raw_parts};

use r2g_mlua::{ffi::{self, luaL_checknumber, lua_State, lua_gettop, lua_pushnumber, lua_resume, lua_settop, lua_tothread, lua_type, lua_typename, lua_xmove, lua_yield, LUA_ERRRUN}, prelude::*};
use crate::instance::WeakManagedInstance;

use super::{borrowck_ignore_mut, get_state, get_thread_identity, registry_keys, RobloxVM, RwLockWriteGuard};

#[derive(Debug)]
pub struct TaskScheduler {
    defer_threads: [Vec<(LuaThread, u32)>; 2],
    delay_threads: [Vec<(LuaThread, u32, f64)>; 2],
    // Wait threads need to be resumed wih how much time it actually elapsed since they got pushed.
    // The first one is for when it needs to resume, the second one when it got pushed.
    wait_threads: [Vec<(LuaThread, u32, f64, f64)>; 2], 
    parallel_dispatch: bool
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum ParallelDispatch {
    Desynchronized,
    Default,
    Synchronized
}

impl Default for ParallelDispatch {
    fn default() -> Self {
        Self::Default
    }
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self::new()
    }
}

pub trait ITaskScheduler {
    fn get_task_scheduler(&self) -> &TaskScheduler;
    fn get_task_scheduler_mut(&mut self) -> &mut TaskScheduler;
}
impl TaskScheduler {
    pub fn as_dyn(&self) -> &dyn ITaskScheduler {
        unsafe { & *((&raw const *self) as *const dyn ITaskScheduler) }
    }
    pub fn as_dyn_mut(&mut self) -> &mut dyn ITaskScheduler {
        unsafe { &mut *((&raw mut *self) as *mut dyn ITaskScheduler) }
    }
    pub(crate) const fn new() -> TaskScheduler {
        TaskScheduler {
            defer_threads: [Vec::new(),Vec::new()],
            delay_threads: [Vec::new(),Vec::new()],
            wait_threads: [Vec::new(),Vec::new()],
            parallel_dispatch: false
        }
    }
}
impl ITaskScheduler for TaskScheduler {
    fn get_task_scheduler(&self) -> &TaskScheduler { self }
    fn get_task_scheduler_mut(&mut self) -> &mut TaskScheduler { self }
}
impl dyn ITaskScheduler {
    #[inline(always)]
    fn clock() -> f64 {
        unsafe { ffi::lua_clock() }
    }
    #[inline(always)]
    fn dispatch_to_table(&self, parallel: ParallelDispatch) -> usize {
        (match parallel {
            ParallelDispatch::Synchronized => false,
            ParallelDispatch::Default => self.get_task_scheduler().parallel_dispatch,
            ParallelDispatch::Desynchronized => true,
        }) as usize
    }
    pub fn spawn_func(&self, lua: &Lua, func: LuaFunction, args: impl IntoLuaMulti) -> LuaResult<LuaThread> {
        let thread = lua.create_thread(func)?;
        let _: LuaResult<()> = thread.resume(args);
        Ok(thread)
    }
    pub fn spawn_thread(&self, thr: LuaThread) -> LuaResult<LuaThread> {
        let _ = thr.resume::<()>(());
        Ok(thr)
    }
    pub fn defer_high_priority<F, A, R>(&mut self, lua: &Lua, args: impl IntoLuaMulti, parallel: ParallelDispatch, f: F) -> LuaResult<LuaThread> 
    where
        F: FnMut(&Lua, A) -> LuaResult<R> + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti
    {
        let parallel = self.dispatch_to_table(parallel);
        let func = lua.create_function_mut(f)?;
        let thread = lua.create_thread(func)?;
        let mut args_count: i32 = 0;
        unsafe { 
            lua.exec_raw::<()>((thread.clone(), args),|lua_raw: *mut lua_State| {
                args_count = ffi::lua_gettop(lua_raw) - 1;
                let thread = ffi::lua_tothread(lua_raw, 1);
                
                ffi::lua_xmove(lua_raw, thread, args_count);
                ffi::lua_settop(lua_raw, 0); // clear stack
            })?;
        }
        self.get_task_scheduler_mut().defer_threads[parallel].insert(0, (thread.clone(), args_count as u32));
        Ok(thread)
    }
    pub fn defer_native<F, A, R>(&mut self, lua: &Lua, args: impl IntoLuaMulti, parallel: ParallelDispatch, f: F) -> LuaResult<LuaThread> 
    where
        F: FnMut(&Lua, A) -> LuaResult<R> + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti
    {
        let parallel = self.dispatch_to_table(parallel);
        let func = lua.create_function_mut(f)?;
        let thread = lua.create_thread(func)?;
        let mut args_count: i32 = 0;
        unsafe { 
            lua.exec_raw::<()>((thread.clone(), args),|lua_raw: *mut lua_State| {
                args_count = ffi::lua_gettop(lua_raw) - 1;
                let thread = ffi::lua_tothread(lua_raw, 1);
                
                ffi::lua_xmove(lua_raw, thread, args_count);
                ffi::lua_settop(lua_raw, 0); // clear stack
            })?;
        }
        self.get_task_scheduler_mut().defer_threads[parallel].push((thread.clone(), args_count as u32));
        Ok(thread)
    }
    pub fn defer_func<'a, 'b>(&'a mut self, lua: &'b Lua, func: LuaFunction, args: impl IntoLuaMulti, parallel: ParallelDispatch) -> LuaResult<LuaThread> {
        let parallel = self.dispatch_to_table(parallel);
        let thread = lua.create_thread(func)?;
        let mut args_count: i32 = 0;
        unsafe { 
            lua.exec_raw::<()>((thread.clone(), args),|lua_raw: *mut lua_State| {
                args_count = ffi::lua_gettop(lua_raw) - 1;
                let thread = ffi::lua_tothread(lua_raw, 1);
                
                ffi::lua_xmove(lua_raw, thread, args_count);
                ffi::lua_settop(lua_raw, 0); // clear stack
            })?;
        }
        self.get_task_scheduler_mut().defer_threads[parallel].push((thread.clone(), args_count as u32));
        Ok(thread)
    }
    pub fn defer_thread(&mut self, thread: LuaThread, parallel: ParallelDispatch) -> LuaResult<LuaThread> {
        let parallel = self.dispatch_to_table(parallel);
        self.get_task_scheduler_mut().defer_threads[parallel].push((thread.clone(), 0));
        Ok(thread)
    }
    pub fn delay_func(&mut self, lua: &Lua, func: LuaFunction, args: impl IntoLuaMulti, parallel: ParallelDispatch, seconds: f64) -> LuaResult<LuaThread> {
        let parallel = self.dispatch_to_table(parallel);
        let thread = lua.create_thread(func)?;
        let mut args_count: i32 = 0;
        unsafe { 
            lua.exec_raw::<()>((thread.clone(), args),|lua_raw: *mut lua_State| {
                args_count = ffi::lua_gettop(lua_raw) - 1;
                let thread = ffi::lua_tothread(lua_raw, 1);
                
                ffi::lua_xmove(lua_raw, thread, args_count);
                ffi::lua_settop(lua_raw, 0); // clear stack
            })?;
        }
        self.get_task_scheduler_mut().delay_threads[parallel].push((thread.clone(), args_count as u32, Self::clock() + seconds));
        Ok(thread)
    }
    pub fn delay_thread(&mut self, thread: LuaThread, parallel: ParallelDispatch, seconds: f64) -> LuaResult<LuaThread> {
        let parallel = self.dispatch_to_table(parallel);
        self.get_task_scheduler_mut().delay_threads[parallel].push((thread.clone(), 0, Self::clock() + seconds));
        Ok(thread)
    }
    pub fn defer_single_cycle<'a>(&mut self, lua: &'a Lua, parallel: bool) -> LuaResult<bool> {
        self.get_task_scheduler_mut().parallel_dispatch = parallel;
        let threads = take(&mut self.get_task_scheduler_mut().defer_threads[parallel as usize]);
        for (thread, args) in threads { unsafe {
            if thread.status() == LuaThreadStatus::Resumable {
                lua.exec_raw::<()>((thread,), |lua_raw: *mut lua_State| {
                    let thread = lua_tothread(lua_raw, 1);
                    let mut _nres = 0;
                    if lua_resume(thread, lua_raw, args as i32, &raw mut _nres) == LUA_ERRRUN {
                        lua_xmove(thread, lua_raw, 1);
                        if lua_gettop(lua_raw) == 0 {
                            ffi::lua_pushstring(lua_raw, "unknown error".as_ptr() as *const i8);
                        }
                        dbg!(lua_gettop(lua_raw));
                        let typename = std::ffi::CStr::from_ptr(lua_typename(lua_raw, lua_type(lua_raw, -1)));
                        dbg!(typename);
                        let mut len = 0;
                        let string_slice= 
                            slice_from_raw_parts(ffi::lua_tolstring(lua_raw, -1, &raw mut len).cast(), len).as_ref().unwrap();
                        let string = std::str::from_utf8(string_slice).unwrap();
                        get_state(lua).get_vm().log_err(
                            IntoLuaMulti::into_lua_multi(format!("Error in thread 0x{:x} during defer: {}", thread as usize, string), lua).unwrap()
                        );
                    }
                    lua_settop(lua_raw, 0);
                })?;
            }
        }}
        Ok(!self.get_task_scheduler_mut().defer_threads[parallel as usize].is_empty())
    }
    pub fn delay_single_cycle<'a>(&mut self, lua: &'a Lua, parallel: bool) -> LuaResult<bool> {
        self.get_task_scheduler_mut().parallel_dispatch = parallel;
        let mut min = f64::MAX;
        // Delay
        {
            let threads = take(&mut self.get_task_scheduler_mut().delay_threads[parallel as usize]);
            let new_threads  = &mut self.get_task_scheduler_mut().delay_threads[parallel as usize];
            new_threads.reserve(threads.len());
            for (thread, args, time ) in threads { unsafe {
                if Self::clock() >= time {
                    if thread.status() == LuaThreadStatus::Resumable {
                        lua.exec_raw::<()>((thread,), |lua_raw: *mut lua_State| {
                            let thread = lua_tothread(lua_raw, 1);
                            let mut _nres = 0;
                            lua_resume(thread, lua_raw, args as i32, &raw mut _nres);
                            lua_settop(lua_raw, 0);
                        })?;
                    }
                } else if thread.status() == LuaThreadStatus::Resumable{
                    min = min.min(time);
                    new_threads.push((thread, args, time));
                }
            }}
        }
        // Wait
        {
            let threads = take(&mut self.get_task_scheduler_mut().wait_threads[parallel as usize]);
            let new_threads  = &mut self.get_task_scheduler_mut().wait_threads[parallel as usize];
            new_threads.reserve(threads.len());
            for (thread, args, time , started) in threads { unsafe {
                debug_assert!(thread.status() != LuaThreadStatus::Running);
                if Self::clock() >= time {
                    if thread.status() == LuaThreadStatus::Resumable {
                        lua.exec_raw::<()>((thread,), |lua_raw: *mut lua_State| {
                            let thread = lua_tothread(lua_raw, 1);
                            let mut _nres = 0;
                            lua_pushnumber(thread, Self::clock()-started);
                            lua_resume(thread, lua_raw, 1, &raw mut _nres);
                            lua_settop(lua_raw, 0);
                        })?;
                    }
                } else if thread.status() == LuaThreadStatus::Resumable{
                    min = min.min(time);
                    new_threads.push((thread, args, time, started));
                }
            }}
        }

        Ok(min > Self::clock())
    }
    #[inline]
    fn watchdog_check(&self, lua: &Lua) -> bool {
        unsafe { get_state(lua).watchdog_check() }
    }
    pub fn defer_cycle<'a>(&mut self, lua: &'a Lua, parallel: bool) -> LuaResult<()> {
        while self.defer_single_cycle(lua, parallel)? && !self.watchdog_check(lua) {}
        Ok(())
    }
    pub fn delay_cycle(&mut self, lua: &Lua, parallel: bool) -> LuaResult<()> {
        while self.delay_single_cycle(lua, parallel)? && !self.watchdog_check(lua) {}
        Ok(())
    }
    pub fn is_desynchronized(&self) -> bool {
        self.get_task_scheduler().parallel_dispatch
    }
    pub fn cancel(&mut self, lua: &Lua, thread: &LuaThread) -> LuaResult<()> {
        match thread.status() {
            LuaThreadStatus::Resumable => thread.close(),
            LuaThreadStatus::Running => {
                let clone = thread.clone();
                self.defer_high_priority(lua, (), ParallelDispatch::Default, move |_, ()|
                    clone.close()
                )?;
                unsafe {
                    let _: () = lua.exec_raw((), |lua: *mut lua_State| {
                        ffi::lua_yield(lua, 0);
                        0;
                    })?;
                }
                Err(LuaError::CallbackDestructed)
            }
            _ => Ok(())
        }
    }
    pub fn cancel_script(&mut self, lua: &Lua, script: &WeakManagedInstance) -> LuaResult<()> {
        for parallel in 0..1 {
            let mut v: Vec<LuaThread> = self.get_task_scheduler().defer_threads[parallel].iter()
                .map(|(thread, _)| (thread, get_thread_identity(lua, thread)))
                .filter(|x| x.1.is_some())
                .map(|x| (x.0, unsafe { x.1.unwrap_unchecked() }))
                .filter(|x| x.1.script.as_ref().map(|x| *script == *x).unwrap_or(false))
                .map(|(thread, _)| thread.clone())
                .collect();
            v.append(&mut self.get_task_scheduler().delay_threads[parallel].iter()
                .map(|(thread, _, _)| (thread, get_thread_identity(lua, thread)))
                .filter(|x| x.1.is_some())
                .map(|x| (x.0, unsafe { x.1.unwrap_unchecked() }))
                .filter(|x| x.1.script.as_ref().map(|x| *script == *x).unwrap_or(false))
                .map(|(thread, _)| thread.clone())
                .collect()
            );
            for thread in v {
                self.cancel(lua, &thread)?;
            }
        }
        Ok(())
    }
    
    pub(super) fn push_wait(lua: &Lua, time: f64) -> LuaResult<()> {
        let task = get_task_scheduler_from_lua(lua);
        let dispatch = task.dispatch_to_table(ParallelDispatch::Default);
        task.get_task_scheduler_mut().wait_threads[dispatch].push((
            lua.current_thread(), 0, time+Self::clock(), Self::clock()
        ));
        Ok(())
    }
    pub(super) fn push_sync_desync(lua: &Lua, parallel: bool) -> LuaResult<bool> {
        let task = get_task_scheduler_from_lua(lua);
        let dispatch = task.dispatch_to_table(ParallelDispatch::Default);
        if dispatch == parallel as usize {
            Ok(false)
        } else {
            task.get_task_scheduler_mut().defer_threads[dispatch].push((lua.current_thread(), 0));
            Ok(true)
        }
    }
    pub(super) unsafe extern "C-unwind" fn synchronize(state: *mut lua_State) -> c_int {
        ffi::lua_rawgetfield(state, ffi::LUA_REGISTRYINDEX, c"__task_push_sync_desync__".as_ptr());
        ffi::lua_pushboolean(state, false as c_int);
        ffi::lua_call(state, 1, 1);
        let b = ffi::lua_toboolean(state, -1) != 0;
        if b {
            lua_yield(state, 0)
        } else {
            0
        }
    }
    pub(super) unsafe extern "C-unwind" fn desynchronize(state: *mut lua_State) -> c_int {
        ffi::lua_rawgetfield(state, ffi::LUA_REGISTRYINDEX, c"__task_push_sync_desync__".as_ptr());
        ffi::lua_pushboolean(state, true as c_int);
        ffi::lua_call(state, 1, 1);
        let b = ffi::lua_toboolean(state, -1) != 0;
        if b {
            lua_yield(state, 0)
        } else {
            0
        }
    }
    pub(super) unsafe extern "C-unwind" fn wait(state: *mut lua_State) -> c_int {
        let time = luaL_checknumber(state, 1);
        ffi::lua_rawgetfield(state, ffi::LUA_REGISTRYINDEX, c"__task_push_wait__".as_ptr());
        ffi::lua_insert(state, 1);
        ffi::lua_call(state, 1, 0);
        lua_yield(state, 0)
    }
}

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct GlobalTaskScheduler {
    task: TaskScheduler
}
impl GlobalTaskScheduler {
    pub fn as_dyn(&self) -> &dyn ITaskScheduler {
        unsafe { & *((&raw const self.task) as *const dyn ITaskScheduler) }
    }
    pub fn as_dyn_mut(&mut self) -> &mut dyn ITaskScheduler {
        unsafe { &mut *((&raw mut self.task) as *mut dyn ITaskScheduler) }
    }
    pub fn frame_step(mut vm: RwLockWriteGuard<RobloxVM>, _: f64) -> LuaResult<()> {
        
        // SAFETY: This function avoids the borrow checker since the main state outlives global task scheduler.
        let main_state = unsafe { borrowck_ignore_mut(vm.get_main_state()) };

        let lua = main_state.get_lua().clone();
        let task = main_state.get_task_scheduler_mut();
        unsafe { vm.set_global_lock_state(false) };
        vm.push_global_lock_atomic();
        vm.watchdog_reset();
        task.defer_cycle(&lua, false)?;
        task.delay_cycle(&lua, false)?;
        // todo! run service events

        vm.get_all_states().iter().for_each(|state| {
            let write = state.write();
            write.gc();
        });

        vm.pop_global_lock_atomic();
        unsafe { vm.set_global_lock_state(true) };
        drop(vm);
        Ok(())
    }
    pub fn new() -> GlobalTaskScheduler {
        GlobalTaskScheduler {
            task: TaskScheduler::new()
        }
    }
}
impl ITaskScheduler for GlobalTaskScheduler {
    fn get_task_scheduler(&self) -> &TaskScheduler {
        &self.task
    }

    fn get_task_scheduler_mut(&mut self) -> &mut TaskScheduler {
        &mut self.task
    }
}

pub fn get_task_scheduler_from_lua<'a, 'b>(lua: &'a Lua) -> &'b mut dyn ITaskScheduler {
    // SAFETY: Lua and the task scheduler must have same lifetime.
    get_state(unsafe {(lua as *const Lua).as_ref().unwrap_unchecked()}).get_task_scheduler_mut()
}