pub mod alloc;
mod assert_gdext_api;
pub(crate) mod debug;
mod fastflags;
mod inheritance;
mod instance;
mod instance_repl_table;
mod instance_tag_collection;
pub mod lua_macros;
mod object;
mod pointers;
mod rc;
mod rw_lock;
mod scheduler;
mod security;
mod state;
mod vm;
mod watchdog;

pub(crate) use assert_gdext_api::verify_gdext_api_compat;
pub use fastflags::*;
pub use inheritance::*;
pub(crate) use instance::InstanceCreationSignalList;
pub use instance::{
    DynInstance, IInstance, IInstanceComponent, InstanceComponent, InstanceCreationMetadata,
    ManagedInstance, WeakManagedInstance,
};
pub(self) use instance_repl_table::InstanceReplicationTable;
pub(self) use instance_tag_collection::InstanceTagCollectionTable;
pub use object::IObject;
pub(self) use pointers::*;
pub use rc::*;
pub use rw_lock::*;
pub use scheduler::{
    get_task_scheduler_from_lua, GlobalTaskScheduler, ITaskScheduler, ParallelDispatch,
    TaskScheduler,
};
pub use security::*;
pub use state::{
    get_current_identity, get_state, get_state_with_rwlock, get_thread_identity, registry_keys,
    LuauState, ThreadIdentity,
};
pub use vm::RblxVM;
pub use watchdog::Watchdog;

/// Provides a way to ignore borrowck for a specific borrow.
/// **This function has been deprecated:** Under normal circumstances, this should never be done. This is only a temporary solution to a problem that requires more effort to fix properly.
#[deprecated(note = "Temporary solution to a problem that requires more effort to fix properly")]
pub(crate) unsafe fn borrowck_ignore<'a, T: ?Sized>(v: &'a T) -> &'static T {
    &*(&raw const *v)
}
/// Provides a way to ignore borrowck for a specific borrow.
/// **This function has been deprecated:** Under normal circumstances, this should never be done. This is only a temporary solution to a problem that requires more effort to fix properly.
#[deprecated(note = "Temporary solution to a problem that requires more effort to fix properly")]
pub(crate) unsafe fn borrowck_ignore_mut<'a, T: ?Sized>(v: &'a mut T) -> &'static mut T {
    &mut *(&raw mut *v)
}
