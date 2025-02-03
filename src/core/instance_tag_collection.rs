use std::collections::{HashMap, HashSet};

use crate::instance::WeakManagedInstance;

use super::RwLock;

#[derive(Default, Debug)]
pub(crate) struct InstanceTagCollectionTable {
    main: RwLock<HashMap<String, RwLock<HashSet<WeakManagedInstance>>>>,
}

impl InstanceTagCollectionTable {
    pub(crate) fn add_tag(&self, tag: String, instance: WeakManagedInstance) {
        let read_guard = self.main.read().unwrap();
        let table = read_guard.get(&tag);
        if table.is_some() {
            unsafe { table.unwrap_unchecked() }.write().unwrap().insert(instance);
        } else {
            drop(read_guard);
            let mut write_guard = self.main.write().unwrap();
            let mut set = HashSet::new();
            set.insert(instance);
            write_guard.insert(tag, RwLock::new_with_flag_auto(set));
        }
    }
    pub(crate) fn remove_tag(&self, tag: String, instance: &WeakManagedInstance) {
        let read_guard = self.main.read().unwrap();
        let table = read_guard.get(&tag);
        if table.is_some() {
            unsafe { table.unwrap_unchecked() }.write().unwrap().remove(instance);
        }
    }
    pub fn garbage_collect(&self) {
        for (_, tbl) in self.main.read().unwrap().iter() {
            tbl.write().unwrap()
                .retain(|x| !x.dead());
        }
    }
}