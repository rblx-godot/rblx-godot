pub mod alloc;
mod rc;
mod security;
pub(crate) mod debug;
mod state;
mod scheduler;
mod vm;
mod inheritance;
mod pointers;
mod instance_repl_table;
mod instance_tag_collection;
mod rw_lock;
mod watchdog;
mod fastflags;
pub mod lua_macros;
mod assert_gdext_api;

pub(self) use instance_tag_collection::InstanceTagCollectionTable;
pub(self) use instance_repl_table::InstanceReplicationTable;
pub(crate) use assert_gdext_api::verify_gdext_api_compat;
pub use inheritance::*;
pub use rc::*;
pub use vm::RobloxVM;
pub use rw_lock::*;
pub use state::{LuauState, registry_keys, get_current_identity, get_state, get_state_with_rwlock, get_thread_identity, ThreadIdentity};
pub use scheduler::{ITaskScheduler, TaskScheduler, get_task_scheduler_from_lua, ParallelDispatch, GlobalTaskScheduler};
pub use security::*;
pub use fastflags::*;
pub(self) use pointers::*;
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