use std::collections::HashMap;
use std::mem::take;
use std::sync::{RwLock, TryLockError};

use crate::core::{DynInstance, ManagedInstance, WeakManagedInstance};

#[derive(Default)]
pub(super) struct InstanceReplicationTable {
    main: RwLock<HashMap<usize, WeakManagedInstance>>,
    secondary: RwLock<HashMap<usize, WeakManagedInstance>>,
}

impl InstanceReplicationTable {
    pub fn get_instance(&self, id: usize) -> Option<WeakManagedInstance> {
        self.main
            .read()
            .unwrap()
            .get(&id)
            .map(|x| x.clone())
            .or_else(|| self.secondary.read().unwrap().get(&id).map(|x| x.clone()))
    }
    pub fn add_instance(&self, instance: ManagedInstance) {
        let mut instance_write = instance.get_instance_component_mut();
        if DynInstance::guard_get_uniqueid(&instance_write) == 0 {
            DynInstance::guard_init_uniqueid(&mut instance_write).unwrap();
        }
        self.main
            .try_write()
            .and_then(|mut guard| {
                guard.insert(
                    DynInstance::guard_get_uniqueid(&instance_write),
                    instance.downgrade(),
                );
                Ok(())
            })
            .or_else(|error| {
                if let TryLockError::WouldBlock = error {
                    self.secondary.write().unwrap().insert(
                        DynInstance::guard_get_uniqueid(&instance_write),
                        instance.downgrade(),
                    );
                    Ok(())
                } else {
                    Err(())
                }
            })
            .expect("Failed to access both main and secondary, is the object poisoned?");
    }
    pub fn garbage_collect(&self) {
        let mut dead_instances = Vec::new();
        {
            let guard = self.main.read().expect("object is poisoned.");
            for (id, instance) in guard.iter() {
                if instance.dead() {
                    dead_instances.push(*id);
                }
            }
        }
        {
            let mut guard = self.main.write().expect("object is poisoned");
            for id in dead_instances {
                guard.remove(&id);
            }
            let mut secondary_guard = self.secondary.write().expect("object is poisoned");
            for (k, v) in take(&mut *secondary_guard).into_iter() {
                guard.insert(k, v);
            }
        }
    }
    pub fn get_stats(&self) -> (usize, usize, usize) {
        // len() main, len() secondary, total capacity
        let mut p = (0, 0, 0);
        {
            let guard = self.main.read().expect("object is poisoned.");
            p.0 = guard.len();
            p.2 = guard.capacity();
        }
        {
            let guard = self.secondary.read().expect("object is poisoned.");
            p.1 = guard.len();
            p.2 += guard.capacity();
        }
        p
    }
}
