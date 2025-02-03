use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::mem::swap;
use std::ops::Deref;
use std::ptr::NonNull;

use r2g_mlua::prelude::*;

use crate::core::alloc::Allocator;
use crate::core::lua_macros::lua_getter;
use crate::core::{get_state, get_task_scheduler_from_lua, IWeak, Irc, IrcHead, ParallelDispatch, RwLockReadGuard, RwLockWriteGuard};
use crate::userdata::{ManagedRBXScriptSignal, RBXScriptSignal};

use super::IObject;

pub type DynInstance = dyn IInstance;
pub type ManagedInstance = Irc<DynInstance>;
pub type WeakManagedInstance = IWeak<DynInstance>;
pub type EventsTable = HashMap<String, ManagedRBXScriptSignal>;

pub trait IInstanceComponent: Sized {
    unsafe fn weak_to_strong_instance(ptr: WeakManagedInstance) -> ManagedInstance {
        ptr.upgrade().unwrap_unchecked()
    }
    fn lua_get(self: &mut RwLockReadGuard<'_, Self>, ptr: &DynInstance, lua: &Lua, key: &String) -> Option<LuaResult<LuaValue>>;
    fn lua_set(self: &mut RwLockWriteGuard<'_, Self>, ptr: &DynInstance, lua: &Lua, key: &String, value: &LuaValue) -> Option<LuaResult<()>>;
    fn clone(self: &RwLockReadGuard<'_, Self>, lua: &Lua, new_ptr: &WeakManagedInstance) -> LuaResult<Self>;
    fn new(ptr: WeakManagedInstance, class_name: &'static str) -> Self;
}

type ReadInstanceComponent<'a> = RwLockReadGuard<'a, InstanceComponent>;
type WriteInstanceComponent<'a> = RwLockWriteGuard<'a, InstanceComponent>;

pub trait IReadInstanceComponent: Deref<Target = InstanceComponent> + Borrow<InstanceComponent> + Sized {}

impl<'a> IReadInstanceComponent for ReadInstanceComponent<'a> {}
impl<'a> IReadInstanceComponent for WriteInstanceComponent<'a> {}

pub trait IInstance: IObject {
    fn get_instance_component(&self) -> RwLockReadGuard<InstanceComponent>;
    fn get_instance_component_mut(&self) -> RwLockWriteGuard<InstanceComponent>;
    
    fn lua_set(&self, lua: &Lua, name: String, val: LuaValue) -> LuaResult<()>;

    fn clone_instance(&self, lua: &Lua) -> LuaResult<ManagedInstance>;

    fn get_actor(&self) -> LuaResult<Option<ManagedInstance>> {
        DynInstance::guard_find_first_ancestor_of_class(&self.get_instance_component(),"Actor".into())
    }
}

impl DynInstance {
    #[inline]
    pub fn get_parent(&self) -> Option<ManagedInstance> {
        DynInstance::guard_get_parent(&self.get_instance_component())
    }
    #[inline]
    pub fn set_parent(&self, lua: &Lua, parent: Option<ManagedInstance>) -> LuaResult<()> {
        DynInstance::guard_set_parent(&mut self.get_instance_component_mut(), lua, parent)
    }
    #[inline]
    pub fn get_name(&self) -> String {
        DynInstance::guard_get_name(&self.get_instance_component())
    }
    #[inline]
    pub fn set_name(&self, val: String) -> LuaResult<()> {
        DynInstance::guard_set_name(&mut self.get_instance_component_mut(), val)
    }
    #[inline]
    pub fn get_archivable(&self) -> bool {
        DynInstance::guard_get_archivable(&self.get_instance_component())
    }
    #[inline]
    pub fn set_archivable(&self, val: bool) -> LuaResult<()> {
        DynInstance::guard_set_archivable(&mut self.get_instance_component_mut(), val)
    }
    #[inline]
    pub fn get_uniqueid(&self) -> usize {
        Self::guard_get_uniqueid(&self.get_instance_component())
    }
    #[inline]
    pub fn init_uniqueid(&self) -> LuaResult<()> {
        DynInstance::guard_init_uniqueid(&mut self.get_instance_component_mut())
    }
    #[inline]
    pub fn set_uniqueid(&self, value: usize) -> LuaResult<()> {
        DynInstance::guard_set_uniqueid(&mut self.get_instance_component_mut(), value)
    }
    
    #[inline]
    pub fn get_ancestors(&self) -> Vec<ManagedInstance> {
        DynInstance::guard_get_ancestors(&self.get_instance_component())
    }
    #[inline]
    pub fn get_children(&self) -> LuaResult<Vec<ManagedInstance>> {
        DynInstance::guard_get_children(&self.get_instance_component())
    }
    #[inline]
    pub fn get_descendants(&self) -> LuaResult<Vec<ManagedInstance>> {
        DynInstance::guard_get_descendants(&self.get_instance_component())
    }
    #[inline]
    pub fn is_ancestor_of(&self, descendant: ManagedInstance) -> LuaResult<bool> {
        DynInstance::is_ancestor_of_guard_guard(&self.get_instance_component(), &descendant.get_instance_component())
    }
    #[inline]
    pub fn is_descendant_of(&self, ancestor: ManagedInstance) -> LuaResult<bool> {
        DynInstance::is_ancestor_of_guard_guard(&ancestor.get_instance_component(), &self.get_instance_component())
    }
    #[inline]
    pub fn lock_parent(&self) {
        DynInstance::guard_lock_parent(&mut self.get_instance_component_mut());
    }
    #[inline]
    pub fn get_parent_protected(&self) -> bool {
        DynInstance::guard_get_parent_protected(&self.get_instance_component())
    }
    #[inline]
    pub fn clear_all_children(&self, lua: &Lua) -> LuaResult<()> {
        DynInstance::guard_clear_all_children(&mut self.get_instance_component_mut(), lua)
    }
    #[inline]
    pub fn destroy(&self, lua: &Lua) -> LuaResult<()> {
        DynInstance::guard_destroy(&mut self.get_instance_component_mut(), lua)
    }

    
    pub fn guard_get_parent(this: &impl IReadInstanceComponent) -> Option<ManagedInstance> {
        let parent = this.parent.as_ref();
        if parent.is_some() { unsafe {
            let parent = parent.unwrap_unchecked().upgrade();
            if parent.is_some() {
                return Some(parent.unwrap_unchecked())
            }
            return None
        }}
        None
    }
    #[inline]
    pub fn guard_set_parent(this: &mut WriteInstanceComponent, lua: &Lua, parent: Option<ManagedInstance>) -> LuaResult<()> {
        if this.parent_locked {
            Err(LuaError::RuntimeError("Parent property is locked.".into()))
        } else {
            DynInstance::set_parent_forced(this, lua, parent)
        }
    }
    fn set_parent_forced(this: &mut WriteInstanceComponent, lua: &Lua, parent: Option<ManagedInstance>) -> LuaResult<()> {
        // Havent tested roblox's internal order
        // This internal order is: DescendantRemoving -> ChildRemoved -> AncestryChanged -> ChildAdded -> DescendantAdded
        if parent.is_some() {
            let _parent_instance = parent.as_ref().unwrap();
            let p = _parent_instance.get_instance_component();
            let this_ptr = this._ptr.as_ref().unwrap().upgrade().unwrap();
            let _guard_release = this.guard_release();
            if DynInstance::guard_is_descendant_of(&p, this_ptr)? {
                return Err(LuaError::RuntimeError("Invalid hierarchy while setting up instance tree.".into()))
            }
        }
        if this.parent.is_some() {
            // Descendant removing for all ancestors
            let ancestors = DynInstance::guard_get_ancestors(&*this);
            let _ptr_this = this._ptr.as_ref().unwrap().upgrade().unwrap();
            let _guard_release = this.guard_release();
            for ancestor in ancestors {
                ancestor.get_instance_component()
                    .descendant_removing.write().fire(lua, (_ptr_this.clone(),))?;
            }
        }

        let new_parent = parent.clone();
        let mut old_parent = parent.map(|x| x.downgrade());
        swap(&mut this.parent, &mut old_parent);

        if old_parent.is_some() {
            let _ptr_this = this._ptr.as_ref().unwrap().upgrade().unwrap();
            let _guard_release = this.guard_release();
            let old_parent = old_parent.unwrap().upgrade().unwrap();
            old_parent.get_instance_component_mut().children.retain(|x| *x != _ptr_this);
            old_parent.get_instance_component().child_removed.write().fire_ancestry(lua, (_ptr_this,))?;
        }

        let descendants = DynInstance::guard_get_descendants(this)?;

        let _ptr_this = this._ptr.as_ref().unwrap().upgrade().unwrap();
        let _guard_release = this.guard_release();
        for i in descendants {
            i.get_instance_component()
                .ancestry_changed.write().fire(lua, (_ptr_this.clone(), new_parent.clone()))?;
        }
        drop(_guard_release);

        if new_parent.is_some() {
            let ancestors = DynInstance::guard_get_ancestors(this);
            let _guard_release = this.guard_release();
            let new_parent = new_parent.unwrap();
            new_parent.get_instance_component_mut().children.push(_ptr_this.clone());
            new_parent.get_instance_component().child_added.write().fire_ancestry(lua, (_ptr_this.clone(),))?;
            for ancestor in ancestors {
                ancestor.get_instance_component().descendant_added.write().fire_ancestry(lua, (_ptr_this.clone(),))?;
            }
        }
        Ok(())
    }
    pub fn guard_get_name(this: &impl IReadInstanceComponent) -> String {
        this.name.clone()
    }
    pub fn guard_set_name(this: &mut WriteInstanceComponent, val: String) -> LuaResult<()> {
        this.name = val;
        Ok(())
    }
    pub fn guard_get_archivable(this: &impl IReadInstanceComponent) -> bool {
        this.archivable
    }
    pub fn guard_set_archivable(this: &mut WriteInstanceComponent, val: bool) -> LuaResult<()> {
        this.archivable = val;
        Ok(())
    }
    pub fn guard_get_uniqueid(this: &impl IReadInstanceComponent) -> usize {
        this.unique_id
    }
    pub fn guard_init_uniqueid(this: &mut WriteInstanceComponent) -> LuaResult<()> {
        if this.unique_id != 0 {
            return Err(LuaError::RuntimeError("Instance::UniqueId was previously initialized.".into()));
        }
        let mut hasher = DefaultHasher::new();
        (&raw const *this).hash(&mut hasher);
        this.unique_id = hasher.finish() as usize;
        Ok(())
    }
    pub fn guard_set_uniqueid(this: &mut WriteInstanceComponent, value: usize) -> LuaResult<()> {
        if this.unique_id != 0 {
            return Err(LuaError::RuntimeError("Instance::UniqueId was previously initialized.".into()));
        }
        this.unique_id = value;
        Ok(())
    }
    
    pub fn guard_get_ancestors(this: &impl IReadInstanceComponent) -> Vec<ManagedInstance> {
        let mut i = this.parent.as_ref().map(|x| x.upgrade()).flatten();
        let mut vec = Vec::new();
        while let Some(instance) = i {
            i = instance.get_parent();
            vec.push(instance);
        }
        vec
    }
    pub fn guard_get_children(this: &impl IReadInstanceComponent) -> LuaResult<Vec<ManagedInstance>> {
        Ok(this.children.clone())
    }
    pub fn guard_get_descendants(this: &impl IReadInstanceComponent) -> LuaResult<Vec<ManagedInstance>> {
        let mut current = DynInstance::guard_get_children(this)?;
        let mut descendants: Vec<ManagedInstance> = vec![];
        descendants.append(&mut current);
        while current.len() != 0 {
            let mut new_current: Vec<ManagedInstance> = vec![];
            for i in current {
                new_current.append(&mut i.get_descendants()?);
            }
            descendants.append(&mut new_current);
            current = new_current;
        };
        Ok(descendants)
    }
    pub fn guard_lock_parent(this: &mut WriteInstanceComponent) {
        this.parent_locked = true;
    }
    pub fn guard_get_parent_protected(this: &impl IReadInstanceComponent) -> bool {
        this.parent_locked
    }
    pub fn guard_clear_all_children(this: &mut WriteInstanceComponent, lua: &Lua) -> LuaResult<()> {
        let children = this.children.clone().into_iter();
        let _guard = this.guard_release();
        for i in children {
            let mut instance_write = i.get_instance_component_mut();
            if !instance_write.parent_locked {
                DynInstance::guard_destroy(&mut instance_write, lua)?;
            }
        }
        Ok(())
    }
    pub fn guard_destroy(this: &mut WriteInstanceComponent, lua: &Lua) -> LuaResult<()> {
        if this.parent_locked {
            return Err(LuaError::RuntimeError("Parent property locked.".into()));
        }
        {
            let signal = this.destroying.clone();
            let destroying = signal.write();
            
            let _guard_release = this.guard_release();
         
            destroying.fire(lua,())?;
        }
        this.parent_locked = true;
        DynInstance::set_parent_forced(this, lua, None)?;
        DynInstance::guard_clear_all_children(this, lua)?;
        Ok(())
    }
    #[inline]
    pub fn guard_is_ancestor_of(this: &impl IReadInstanceComponent, descendant: ManagedInstance) -> LuaResult<bool> {
        DynInstance::is_ancestor_of_guard_guard(this, &descendant.get_instance_component())
    }
    #[inline]
    pub fn guard_is_descendant_of(this: &impl IReadInstanceComponent, ancestor: ManagedInstance) -> LuaResult<bool> {
        DynInstance::is_ancestor_of_guard_guard(&ancestor.get_instance_component(), this)
    }
    pub fn guard_find_first_ancestor_of_class(this: &impl IReadInstanceComponent, class: String) -> LuaResult<Option<ManagedInstance>> {
        for i in DynInstance::guard_get_ancestors(this) {
            if i.get_class_name() == class {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }
    
    pub fn get_debug_id(&self, _scope_length: LuaNumber) -> LuaResult<String> {
        Ok(format!("0x{:x}",(&raw const *self.get_instance_component()) as usize))
    }
    pub fn get_full_name(&self) -> LuaResult<String> {
        let mut hierarchy = Vec::new();
        let mut parent = self.get_parent();
        while parent.is_some() {
            let parent_unwrapped = unsafe { parent.unwrap_unchecked() };
            let parent_read = parent_unwrapped.get_instance_component();
            hierarchy.insert(0, DynInstance::guard_get_name(&parent_read));

            parent = DynInstance::guard_get_parent(&parent_read);
        }
        hierarchy.push(self.get_name());
        Ok(hierarchy.join("."))
    }
    pub fn add_tag(&self, lua: &Lua, tag: String) -> LuaResult<()> {
        let mut write = self.get_instance_component_mut();
        let ptr = write._ptr.clone().unwrap();
        write.tags.insert(tag.clone());
        drop(write);
        get_state(lua).get_vm().get_instance_tag_table().add_tag(tag, ptr);
        Ok(())
    }
    
    pub async fn wait_for_child(&self, lua: &Lua, name: String, timeout: Option<LuaNumber>) -> LuaResult<Option<ManagedInstance>> {
        let inst = self.find_first_child(name.clone(), Some(false)).unwrap();
        if inst.is_some() {
            Ok(Some(unsafe { inst.unwrap_unchecked() }))
        } else {
            let thread = lua.current_thread();
            if timeout.is_some() {
                get_task_scheduler_from_lua(lua).delay_thread(thread.clone(), ParallelDispatch::Default, timeout.unwrap())?;
            }
            let i = self.get_instance_component().child_added.clone();
            let mut instance: ManagedInstance;
            loop {
                let mv = i.read().wait(lua).await?;
                if mv.is_empty() {
                    return Ok(None); // timed out
                }
                instance = ManagedInstance::from_lua_multi(mv, lua)?;
                if instance.get_name() == name {
                    break;
                }
            }
            Ok(Some(instance))
        }
    }
    pub fn find_first_ancestor(&self, name: String) -> LuaResult<Option<ManagedInstance>> {
        for i in self.get_ancestors() {
            if i.get_name() == name {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }
    pub fn find_first_ancestor_of_class(&self, class: String) -> LuaResult<Option<ManagedInstance>> {
        for i in self.get_ancestors() {
            if i.get_class_name() == class {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }
    pub fn find_first_ancestor_which_is_a(&self, class: String) -> LuaResult<Option<ManagedInstance>> {
        for i in self.get_ancestors() {
            if i.is_a(&class) {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }
    pub fn find_first_child(&self, name: String, recursive: Option<bool>) -> LuaResult<Option<ManagedInstance>> {
        if recursive.unwrap_or(false) {
            for i in self.get_children()? {
                if i.get_name() == name {
                    return Ok(Some(i));
                }
            }
            Ok(None)
        } else {
            self.find_first_descendant(name)
        }
    }
    pub fn find_first_child_of_class(&self, class: String) -> LuaResult<Option<ManagedInstance>> {
        for i in self.get_children()? {
            if i.get_class_name() == class {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }
    pub fn find_first_child_which_is_a(&self, class: String) -> LuaResult<Option<ManagedInstance>> {
        for i in self.get_children()? {
            if i.is_a(&class) {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }
    pub fn find_first_descendant(&self, name: String) -> LuaResult<Option<ManagedInstance>> {
        for i in self.get_descendants()? {
            if i.get_name() == name {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }
    pub fn get_attribute(&self, attribute: String) -> LuaResult<LuaValue> {
        Ok(self.get_instance_component().attributes.get(&attribute).unwrap_or(&LuaNil).clone())
    }
    pub fn get_attribute_changed_signal(&self, attribute: String) -> LuaResult<ManagedRBXScriptSignal> {
        let read = self.get_instance_component();
        if let Some(event) = read.attribute_changed_table.get(&attribute) {
            Ok(event.clone())
        } else {
            drop(read);
            let mut write = self.get_instance_component_mut();
            let event = RBXScriptSignal::new();
            write.attribute_changed_table.insert(attribute, event.clone());
            Ok(event)
        }
    }
    pub fn get_attributes(&self, lua: &Lua) -> LuaResult<LuaValue> {
        self.get_instance_component().attributes.clone().into_lua(lua)
    }
    pub fn get_tags(&self) -> LuaResult<Vec<String>> {
        Ok(self.get_instance_component().tags.iter().map(|x| x.clone()).collect())
    }
    pub fn has_tag(&self, tag: String) -> LuaResult<bool> {
        Ok(self.get_instance_component().tags.get(&tag).is_some())
    }
    fn is_ancestor_of_guard_guard(this: &impl IReadInstanceComponent, descendant: &impl IReadInstanceComponent) -> LuaResult<bool> {
        let mut i = DynInstance::guard_get_parent(descendant);
        while let Some(ref parent) = i {
            let read = parent.get_instance_component();
            if unsafe {read._ptr.as_ref().unwrap_unchecked() == this._ptr.as_ref().unwrap_unchecked()} {
                return Ok(true);
            }
            let new_i = read.parent.as_ref().map(|x| x.upgrade()).flatten();
            drop(read);
            i = new_i;
        }
        Ok(false)
    }
    pub fn remove_tag(&self, lua: &Lua, tag: String) -> LuaResult<()> {
        let mut write = self.get_instance_component_mut();
        let ptr = write._ptr.clone().unwrap();
        write.tags.remove(&tag);
        drop(write);
        get_state(lua).get_vm().get_instance_tag_table().remove_tag(tag, &ptr);
        Ok(())
    }
    pub fn set_attribute(&self, lua: &Lua, attribute: String, value: LuaValue) -> LuaResult<()> {
        let mut write = self.get_instance_component_mut();
        write.attributes.insert(attribute.clone(), value.clone());
        let attribute_changed = write.attribute_changed.clone();
        let attribute_changed_signal = write.attribute_changed_table.get(&attribute).cloned();
        attribute_changed.write().fire(lua, (attribute,))?;
        attribute_changed_signal.map(move |x| x.write().fire(lua, (value,))).unwrap_or(Ok(()))
    }
    
}

impl Debug for DynInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{:x}", &raw const *self as *const u8 as usize))
    }
}

#[derive(Debug)]
pub struct InstanceComponent {
    archivable: bool,
    name: String,
    parent: Option<WeakManagedInstance>,
    _ptr: Option<WeakManagedInstance>,
    unique_id: usize,
    children: Vec<ManagedInstance>,
    children_cache: HashMap<String, WeakManagedInstance>,
    children_cache_dirty: bool,
    parent_locked: bool,

    pub ancestry_changed: ManagedRBXScriptSignal,
    pub attribute_changed: ManagedRBXScriptSignal,
    pub changed: ManagedRBXScriptSignal,
    pub child_added: ManagedRBXScriptSignal,
    pub child_removed: ManagedRBXScriptSignal,
    pub descendant_added: ManagedRBXScriptSignal,
    pub descendant_removing: ManagedRBXScriptSignal,
    pub destroying: ManagedRBXScriptSignal,

    pub attribute_changed_table: EventsTable,
    pub property_changed_table: EventsTable,

    attributes: HashMap<String, LuaValue>,
    tags: HashSet<String>
}

impl PartialEq for DynInstance {
    fn eq(&self, other: &Self) -> bool {
        (&raw const *self) == (&raw const *other)
    }
}
impl Eq for DynInstance {}

impl IInstanceComponent for InstanceComponent {
    fn lua_get(self: &mut RwLockReadGuard<'_, Self>, _: &DynInstance, lua: &Lua, key: &String) -> Option<LuaResult<LuaValue>> {
        Some(InstanceComponent::lua_get(self, lua, key))
    }

    fn lua_set(self: &mut RwLockWriteGuard<'_, Self>, _: &DynInstance, lua: &Lua, key: &String, value: &LuaValue) -> Option<LuaResult<()>> {
        Some(InstanceComponent::lua_set(self, lua, key, value.clone()))
    }

    fn new(ptr: WeakManagedInstance, class_name: &'static str) -> Self {
        let inst = InstanceComponent {
            parent: None,
            name: String::from(class_name),
            archivable: true,
            unique_id: usize::default(), //uninitialized,
            _ptr: Some(ptr),
            parent_locked: false,
            children: Vec::new(),
            children_cache: HashMap::new(),
            children_cache_dirty: false,

            ancestry_changed: RBXScriptSignal::new(),
            attribute_changed: RBXScriptSignal::new(),
            changed: RBXScriptSignal::new(),
            child_added: RBXScriptSignal::new(),
            child_removed: RBXScriptSignal::new(),
            descendant_added: RBXScriptSignal::new(),
            descendant_removing: RBXScriptSignal::new(),
            destroying: RBXScriptSignal::new(),
            
            attribute_changed_table: EventsTable::default(),
            property_changed_table: EventsTable::default(),

            attributes: HashMap::default(),
            tags: HashSet::default()
        };
        inst
    }
    fn clone(self: &RwLockReadGuard<'_, Self>, lua: &Lua, ptr: &WeakManagedInstance) -> LuaResult<Self> {
        let mut new_children = Vec::new();
        for i in self.children.iter() {
            let inst = i.clone_instance(lua);
            if inst.is_ok() {
                new_children.push(unsafe { inst.unwrap_unchecked() });
            }
        }
        Ok(InstanceComponent {
            archivable: self.archivable,
            name: self.name.clone(),
            parent: None,
            unique_id: 0,
            children: new_children,
            children_cache: HashMap::default(),
            children_cache_dirty: true,
            _ptr: Some(ptr.clone()),
            parent_locked: false,

            ancestry_changed: RBXScriptSignal::new(),
            attribute_changed: RBXScriptSignal::new(),
            changed: RBXScriptSignal::new(),
            child_added: RBXScriptSignal::new(),
            child_removed: RBXScriptSignal::new(),
            descendant_added: RBXScriptSignal::new(),
            descendant_removing: RBXScriptSignal::new(),
            destroying: RBXScriptSignal::new(),
            
            attribute_changed_table: EventsTable::default(),
            property_changed_table: EventsTable::default(),

            attributes: self.attributes.clone(),
            tags: HashSet::default()
        })
    }

}

impl InstanceComponent {
    pub fn get_instance_pointer(self: &RwLockReadGuard<'_, Self>) -> ManagedInstance {
        unsafe {
            self._ptr.as_ref().unwrap_unchecked().upgrade().unwrap_unchecked()
        }
    }
    pub fn get_weak_instance_pointer(self: &RwLockReadGuard<'_, Self>) -> WeakManagedInstance {
        unsafe { self._ptr.as_ref().unwrap_unchecked().clone() }
    }
    pub fn lua_get(self: &mut RwLockReadGuard<'_, Self>, lua: &Lua, key: &String) -> LuaResult<LuaValue> {
        match key.as_str() {
            "Archivable" => lua_getter!(lua, self.archivable),
            "ClassName" => IntoLua::into_lua(unsafe { 
                self._ptr.as_ref().unwrap_unchecked()
                    .upgrade().unwrap_unchecked().get_class_name()
                }, lua),
            "Name" => lua_getter!(string, lua, self.name),
            "Parent" => lua_getter!(opt_weak_clone, lua, self.parent),

            "AddTag" => lua_getter!(function, lua, 
                |lua, (this, tag): (ManagedInstance, String)| 
                    this.add_tag(lua, tag)
            ),
            "ClearAllChildren" => lua_getter!(function, lua,
                |lua, (this, ): (ManagedInstance, )| {
                    this.clear_all_children(lua)
                }
            ),
            "Clone" => lua_getter!(function, lua,
                |lua, (this, ): (ManagedInstance, )| 
                    this.clone_instance(lua)
            ),
            "Destroy" => lua_getter!(function, lua,
                |lua, (this, ): (ManagedInstance, )| {
                    this.destroy(lua)
                }
            ),
            "FindFirstAncestor" => lua_getter!(function, lua,
                |_, (this, name): (ManagedInstance, String)| 
                    this.find_first_ancestor(name)
            ),
            "FindFirstAncestorOfClass" => lua_getter!(function, lua,
                |_, (this, class): (ManagedInstance, String)| 
                    this.find_first_ancestor_of_class(class)
            ),
            "FindFirstAncestorWhichIsA" => lua_getter!(function, lua,
                |_, (this, class): (ManagedInstance, String)| 
                    this.find_first_ancestor_which_is_a(class)
            ),
            "FindFirstChild" => lua_getter!(function, lua,
                |_, (this, name, recursive): (ManagedInstance, String, Option<bool>)| 
                    this.find_first_child(name, recursive)
            ),
            "FindFirstChildOfClass" => lua_getter!(function, lua,
                |_, (this, class): (ManagedInstance, String)| 
                    this.find_first_child_of_class(class)
            ),
            "FindFirstChildWhichIsA" => lua_getter!(function, lua,
                |_, (this, class): (ManagedInstance, String)| 
                    this.find_first_ancestor_which_is_a(class)
            ),
            "GetActor" => lua_getter!(function, lua,
                |_, (this,): (ManagedInstance,)| 
                    this.get_actor()
            ),
            "GetAttribute" => lua_getter!(function, lua,
                |_, (this, name): (ManagedInstance, String)| 
                    this.get_attribute(name)
            ),
            "GetAttributeChangedSignal" => lua_getter!(function, lua,
                |_, (this, attribute): (ManagedInstance, String)| {
                    Ok(this.get_attribute_changed_signal(attribute))
            }),
            "GetAttributes" => lua_getter!(function, lua,
                |lua, (this,): (ManagedInstance,)|
                    this.get_attributes(lua)
            ),
            "GetChildren" => lua_getter!(function, lua,
                |_, (this,): (ManagedInstance,)|
                    this.get_children()
            ),
            "GetDebugId" => lua_getter!(function, lua,
                |_, (this,scope_len): (ManagedInstance,LuaNumber)|
                    this.get_debug_id(scope_len)
            ),
            "GetDescendants" => lua_getter!(function, lua,
                |_, (this,): (ManagedInstance,)|
                    this.get_descendants()
            ),
            "GetFullName" => lua_getter!(function, lua,
                |_, (this,): (ManagedInstance,)|
                    this.get_full_name()
            ),
            "GetStyled" => lua_getter!(function, lua,
                |_, (_this,): (ManagedInstance,)| {
                    Err::<(), LuaError>(LuaError::RuntimeError("todo!(): function not yet implemented".into()))
                }
            ),
            "GetTags" => lua_getter!(function, lua,
                |_, (this,): (ManagedInstance,)|
                    this.get_tags()
            ),
            "HasTag" => lua_getter!(function, lua,
                |_, (this, tag): (ManagedInstance, String)|
                    this.has_tag(tag)
            ),
            "IsAncestorOf" => lua_getter!(function, lua,
                |_, (this, inst): (ManagedInstance, ManagedInstance)|
                    this.is_ancestor_of(inst)
            ),
            "IsDescendantOf" => lua_getter!(function, lua,
                |_, (this, inst): (ManagedInstance, ManagedInstance)|
                    this.is_descendant_of(inst)
            ),
            "RemoveTag" => lua_getter!(function, lua,
                |lua, (this, tag): (ManagedInstance, String)|
                    this.remove_tag(lua, tag)
            ),
            "SetAttribute" => lua_getter!(function, lua,
                |lua, (this, attribute, value): (ManagedInstance, String, LuaValue)|
                    this.set_attribute(lua, attribute, value)
            ),
            "WaitForChild" => lua_getter!(function_async, lua,
                async |lua, (this, child, timeout): (ManagedInstance, String, Option<LuaNumber>)| {
                    this.wait_for_child(&lua, child, timeout).await
                }
            ),
            
            "AncestryChanged" => lua_getter!(clone, lua, self.ancestry_changed),
            "AttributeChanged" => lua_getter!(clone, lua, self.attribute_changed),
            "ChildAdded" => lua_getter!(clone, lua, self.child_added),
            "ChildRemoved" => lua_getter!(clone, lua, self.child_removed),
            "DescendantAdded" => lua_getter!(clone, lua, self.descendant_added),
            "DescendantRemoving" => lua_getter!(clone, lua, self.descendant_removing),
            "Destroying" => lua_getter!(clone, lua, self.destroying),

            _ => lua_getter!(lua, self.find_first_child(key))
        }
    }

    fn remake_cache(self: &mut RwLockReadGuard<'_, Self>) {
        let inst = self._ptr.as_ref().map(|x| x.upgrade()).flatten().unwrap();
        let _release = self.guard_release();
        let mut write = inst.get_instance_component_mut();
        
        let iter: Vec<(String, WeakManagedInstance)> = write.children.iter()
            .map(|x| (x.get_name(), x.downgrade()))
            .collect();
        write.children_cache.clear();
        for (name, i) in iter {
            if write.children_cache.get(&name).is_none() {
                write.children_cache.insert(name, i);
            }
        }
    }
    fn find_first_child(self: &mut RwLockReadGuard<'_, Self>, key: &String) -> Option<ManagedInstance> {
        if self.children_cache_dirty {
            self.remake_cache();
            self.children_cache.get(key).map(|x| x.upgrade()).flatten()
        } else {
            self.children_cache.get(key).map(|x| x.upgrade()).flatten()
        }
    }
    pub fn emit_property_changed(this: &impl IReadInstanceComponent, lua: &Lua, property: &'static str, value: &LuaValue) -> LuaResult<()> {
        this.changed.write().fire(lua, (property,))?;
        this.property_changed_table.get(property)
            .map(|x| x.write().fire(lua, (value,)))
            .unwrap_or(Ok(()))
    }
    pub fn get_property_changed_signal(self: &mut RwLockReadGuard<'_, Self>, property: String) -> LuaResult<ManagedRBXScriptSignal> {
        let read = self.property_changed_table.get(&property);
        if read.is_some() {
            Ok(read.unwrap().clone())
        } else {
            let inst = self._ptr.as_ref().map(|x| x.upgrade()).flatten().unwrap();
            let _release = self.guard_release();
            
            let mut write = inst.get_instance_component_mut();
            let signal = RBXScriptSignal::new();
            write.property_changed_table.insert(property, signal.clone());
            Ok(signal)
        }
    }

    pub fn lua_set(self: &mut RwLockWriteGuard<'_, Self>, lua: &Lua, key: &String, value: LuaValue) -> LuaResult<()> {
        match key.as_str() {
            "Archivable" => {
                self.archivable = value.as_boolean().ok_or(LuaError::RuntimeError("bad argument to setting Archivable".into()))?;
                Self::emit_property_changed(self, lua, "Archivable", &value)
            },
            "Name" => {
                self.name = value.as_string_lossy().ok_or(LuaError::RuntimeError("bad argument to setting Name".into()))?;
                Self::emit_property_changed(self, lua, "Name", &value)
            },
            "Parent" => {
                DynInstance::guard_set_parent(self, lua, FromLua::from_lua(value, lua)?)
            },
            _ => Err(LuaError::RuntimeError(format!("can't set property {} on object of type Instance",key.as_str())))
        }
    }
}

impl<T: IInstance, A: Allocator + Clone + Send + Sync> IWeak<T, A> {
    pub fn cast_to_instance(&self) -> IWeak<DynInstance, A> {
        let (header_raw, p_raw, alloc) = self.clone().into_inner_with_allocator();
        let p = p_raw as *mut DynInstance;
        let header = unsafe { NonNull::new_unchecked(header_raw.as_ptr() as *mut IrcHead<DynInstance>) };
        IWeak::from_inner_with_allocator((header, p, alloc))
    }
}