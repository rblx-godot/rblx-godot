use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::marker::PhantomPinned;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;

use r2g_mlua::prelude::*;

use crate::core::scheduler::GlobalTaskScheduler;
use crate::instance::{DataModel, IDataModel, LogService, RunService, WeakManagedActor, Workspace};

use super::state::LuauState;
use super::{
    FastFlag, FastFlagValue, FastFlags, InstanceReplicationTable, InstanceTagCollectionTable, Irc,
    RwLock, Trc, Watchdog, Weak, GLOBAL_LOCKS_OF_THREAD,
};

pub struct RblxVM {
    main_state: Trc<LuauState>,
    states: Vec<Weak<LuauState>>,
    instances: InstanceReplicationTable,
    instances_tag_collection: InstanceTagCollectionTable,
    flags: MaybeUninit<FastFlags>,
    data_model: MaybeUninit<Irc<DataModel>>,
    global_lock: Arc<AtomicBool>,

    states_locks: HashMap<*mut LuauState, *const Trc<LuauState>>,

    hard_wd: Watchdog,
    soft_wd: Watchdog,

    _pin: PhantomPinned,
}

impl RblxVM {
    pub fn new(flags_table: Option<Vec<(FastFlag, FastFlagValue)>>) -> Box<RwLock<RblxVM>> {
        unsafe {
            let mut vm = Box::new(RwLock::new(RblxVM {
                main_state: Trc::new(LuauState::new_uninit()),
                states: Vec::new(),
                states_locks: HashMap::new(),
                global_lock: Arc::new(AtomicBool::new(true)),
                instances: InstanceReplicationTable::default(),
                instances_tag_collection: InstanceTagCollectionTable::default(),
                data_model: MaybeUninit::uninit(),
                hard_wd: Watchdog::new_timeout(10.0),
                soft_wd: Watchdog::new_timeout(1.0 / 60.0),
                _pin: PhantomPinned::default(),
                flags: MaybeUninit::uninit(),
            }));
            vm.set_global_lock(vm.access().as_ref().unwrap().global_lock.as_ref());
            let vm_ptr = &raw mut *vm;
            let flags = FastFlags::new(vm_ptr);
            vm.get_mut().flags.write(flags);
            if let Some(table) = flags_table {
                vm.get_mut()
                    .flags
                    .assume_init_mut()
                    .initialize_with_table(table);
            }
            vm.access()
                .as_mut()
                .unwrap()
                .data_model
                .write(DataModel::new(
                    vm.access().as_ref().unwrap().flags.assume_init_ref(),
                ));
            let main_state_ptr = vm.get_mut().main_state.access();
            let main_state_lock_ptr = &raw const vm.get_mut().main_state;
            vm.get_mut()
                .states_locks
                .insert(main_state_ptr, main_state_lock_ptr);

            vm.get_mut()
                .main_state
                .access()
                .as_mut()
                .unwrap_unchecked()
                .init(vm_ptr, Box::new(GlobalTaskScheduler::new()));
            vm.access()
                .as_ref()
                .unwrap()
                .data_model
                .assume_init_ref()
                .init_services(vm.access().as_mut().unwrap().get_main_state().get_lua())
                .unwrap();
            vm.get_mut()
                .main_state
                .access()
                .as_mut()
                .unwrap_unchecked()
                .bind_services();
            vm
        }
    }
    pub fn get_main_state(&mut self) -> &mut LuauState {
        unsafe { &mut *self.main_state.access() }
    }
    pub(super) fn get_state_with_rwlock(
        &self,
        ptr: *mut LuauState,
    ) -> Option<*const Trc<LuauState>> {
        self.states_locks.get(&ptr).map(|x| *x)
    }
    unsafe fn watchdog_trip_state(state: *mut LuauState) {
        state
            .as_mut()
            .unwrap_unchecked()
            .get_lua()
            .set_interrupt(|_| {
                Err(LuaError::RuntimeError(
                    "script exhausted maximum execution time".into(),
                ))
            });
    }
    fn watchdog_reset_state(state: &mut LuauState) {
        state.get_lua().remove_interrupt();
    }
    pub fn watchdog_trip(&self) {
        self.hard_wd.trip();
        // SAFETY: Luau permits setting interrupt from other threads.
        unsafe {
            Self::watchdog_trip_state(self.main_state.access());
            for i in self
                .states
                .iter()
                .map(|x| x.upgrade())
                .filter(|x| x.is_some())
                .map(|x| x.unwrap())
            {
                Self::watchdog_trip_state(i.access());
            }
        }
    }
    pub fn watchdog_reset(&mut self) {
        if self.hard_wd.check() {
            Self::watchdog_reset_state(unsafe {
                self.main_state.access().as_mut().unwrap_unchecked()
            });
            for i in self
                .states
                .iter()
                .map(|x| x.upgrade())
                .filter(|x| x.is_some())
                .map(|x| x.unwrap())
            {
                Self::watchdog_reset_state(i.write().borrow_mut());
            }
        }
        self.hard_wd.reset();
        self.soft_wd.reset();
    }
    pub(crate) fn watchdog_check(&self) -> bool {
        if self.hard_wd.check() {
            self.watchdog_trip();
        }
        self.soft_wd.check()
    }
    #[inline(always)]
    pub(crate) fn get_instance_tag_table(&self) -> &InstanceTagCollectionTable {
        &self.instances_tag_collection
    }
    #[inline(always)]
    pub(crate) const fn flags(&self) -> &FastFlags {
        unsafe { self.flags.assume_init_ref() }
    }
    #[inline(always)]
    pub fn get_game_instance(&self) -> Irc<DataModel> {
        unsafe { self.data_model.assume_init_ref().clone() }
    }
    /// SAFETY: Always allowed, even from .access(). Guaranteed thread-safe
    #[inline(always)]
    pub fn get_global_lock_state(&self) -> bool {
        self.global_lock.load(Relaxed)
    }
    /// SAFETY: Modifying the global lock changes the behavior of all RwLocks. Do not modify unless you know what you're doing.
    #[inline(always)]
    pub unsafe fn set_global_lock_state(&mut self, state: bool) {
        self.global_lock.store(state, Relaxed);
    }
    #[inline(always)]
    pub(crate) fn push_global_lock_atomic(&self) {
        GLOBAL_LOCKS_OF_THREAD.with_borrow_mut(|x| x.push(self.global_lock.clone()));
    }
    #[inline(always)]
    pub(crate) fn pop_global_lock_atomic(&self) {
        GLOBAL_LOCKS_OF_THREAD.with_borrow_mut(|x| x.pop().unwrap());
    }
    pub(crate) fn create_sub_state(&mut self, actor: &WeakManagedActor) -> Trc<LuauState> {
        let self_rwlock = unsafe {
            self.main_state
                .access()
                .as_ref()
                .unwrap_unchecked()
                .get_vm_ptr()
        };
        let mut state = LuauState::new(self_rwlock);
        state.set_actor(actor.clone());
        let rc = Trc::new(state);
        self.states.push(rc.downgrade());
        rc
    }
    pub fn get_main_state_rc(&self) -> Trc<LuauState> {
        self.main_state.clone()
    }
    pub fn get_all_states(&self) -> Vec<Trc<LuauState>> {
        self.states
            .iter()
            .map(|x| x.upgrade())
            .filter(|x| x.is_some())
            .map(|x| x.unwrap())
            .chain(std::iter::once(self.main_state.clone()))
            .collect()
    }
    pub fn get_log_service(&self) -> Irc<LogService> {
        <dyn IDataModel>::get_log_service(&*self.get_game_instance())
    }
    pub fn get_run_service(&self) -> Irc<RunService> {
        <dyn IDataModel>::get_run_service(&*self.get_game_instance())
    }
    pub fn get_workspace(&self) -> Irc<Workspace> {
        <dyn IDataModel>::get_workspace(&*self.get_game_instance())
    }
}

impl Drop for RblxVM {
    fn drop(&mut self) {
        self.states.clear();
        unsafe { self.flags.assume_init_drop() };
    }
}
