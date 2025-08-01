use std::cell::UnsafeCell;
use std::fmt::Debug;
use std::mem::transmute;
use std::ops::{BitAnd, BitOr};
use std::ptr::NonNull;

use r2g_mlua::prelude::*;

use crate::core::InheritanceBase;
use crate::userdata::{ManagedRBXScriptSignal, RBXScriptSignal};

use super::alloc::Allocator;
use super::{IWeak, IrcHead, RwLockReadGuard, RwLockWriteGuard, SecurityContext, Weak};

pub trait IObject: InheritanceBase + Send + Sync {
    fn lua_get(&self, lua: &Lua, name: String) -> LuaResult<LuaValue>;
    fn get_class_name(&self) -> &'static str;
    fn is_a(&self, class_name: &String) -> bool;

    // Modification of the object

    fn lua_set(&self, lua: &Lua, name: String, val: LuaValue) -> LuaResult<()>;
    fn get_property_changed_signal(&self, property: String) -> ManagedRBXScriptSignal;
    fn get_changed_signal(&self) -> ManagedRBXScriptSignal;
}

impl PartialEq for dyn IObject {
    fn eq(&self, other: &Self) -> bool {
        (&raw const *self) == (&raw const *other)
    }
}
impl Eq for dyn IObject {}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// Implementation details about this Object class.
pub enum ObjectMask {
    None = 0,

    // Virtual makes it call the init on the base class.
    // The second allows overriding `_init`
    // Both enabled, calls `_init` and then init on the base class.
    // RequiresInitVirtual = 1, NOW ENABLED BY DEFALT
    RequiresInit = 1,

    // Whether signals are handled at the base of the hierarchy, just before Object or by the class itself.
    // SignalsBottom = 2, BEHAVIOR CHANGE: If signals is not enabled, overrides to call it on the base class.
    Signals = 2,

    // Whether there is a function named _component_init
    ComponentInit = 4,

    NotConstructible = 8,
}

impl ObjectMask {
    pub const fn create_raw(v: u8) -> Self {
        unsafe { transmute(v) }
    }
}

impl BitOr for ObjectMask {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        unsafe { std::mem::transmute(self as u8 | rhs as u8) }
    }
}
impl BitAnd for ObjectMask {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        unsafe { std::mem::transmute(self as u8 & rhs as u8) }
    }
}

#[derive(Debug)]
pub struct LuaPropertyInfo {
    pub name: &'static str,
    pub readonly: bool,
    pub security_context: SecurityContext,
    pub not_replicated: bool,
    pub get: Option<&'static str>,
    pub set: Option<&'static str>,
}

#[derive(Debug)]
pub enum ObjectPropertyInfo {
    Rust {
        name: &'static str,
        ty: &'static str,
        default: Option<&'static str>,
    },
    Transparent(LuaPropertyInfo),
    Lua {
        name: &'static str,
        ty: &'static str,
        default: Option<&'static str>,
        info: LuaPropertyInfo,
    },
}

#[derive(Debug)]
pub struct LuaFunctionSignature {
    pub name: &'static str,
    pub args: &'static [(&'static str, &'static str)],
    pub ret: &'static str,
}

#[derive(Debug)]
pub struct LuaFunctionInfo {
    pub lua_name: &'static str,
    pub security_context: SecurityContext,
    pub sig: LuaFunctionSignature,
}

#[derive(Debug)]
pub struct VirtualLuaFunctionInfo {
    pub func_info: LuaFunctionInfo,
    pub is_overriding: bool,
    pub base: &'static str,
}

#[derive(Debug)]
pub struct PartialObjectClassInfo {
    pub class_name: &'static str,
    pub hierarchy: &'static [&'static str],
    pub properties: &'static [ObjectPropertyInfo],
    pub virtual_funcs: &'static [VirtualLuaFunctionInfo],
    pub signals: &'static [&'static str],
    pub mask: ObjectMask,
}

#[derive(Debug)]
pub struct ObjectClassInfo {
    pub partial_info: PartialObjectClassInfo,
    pub method_list: &'static [LuaFunctionInfo],
}

pub type DynObject = dyn IObject;

pub trait IObjectComponent: Sized {
    fn lua_get(
        self: &mut RwLockReadGuard<'_, Self>,
        ptr: &DynObject,
        lua: &Lua,
        key: &String,
    ) -> Option<LuaResult<LuaValue>>;
    fn lua_set(
        self: &mut RwLockWriteGuard<'_, Self>,
        ptr: &DynObject,
        lua: &Lua,
        key: &String,
        value: &LuaValue,
    ) -> Option<LuaResult<()>>;
    fn new(metadata: &ObjectCreationMetadata) -> LuaResult<Self>;
}

pub type WeakManagedObject = IWeak<DynObject>;
pub(crate) type ObjectCreationSignalList = Vec<Weak<RBXScriptSignal>>;

pub struct ObjectCreationMetadata {
    pub(super) class_name: &'static str,
    pub(super) ptr: WeakManagedObject,
    pub(crate) signal_list: UnsafeCell<ObjectCreationSignalList>,
}

impl ObjectCreationMetadata {
    pub fn new(class_name: &'static str, ptr: WeakManagedObject) -> Self {
        Self {
            class_name,
            ptr,
            signal_list: UnsafeCell::new(ObjectCreationSignalList::new()),
        }
    }
    pub fn get_ptr(&self) -> WeakManagedObject {
        self.ptr.clone()
    }
    pub fn get_ptr_ref(&self) -> &WeakManagedObject {
        &self.ptr
    }
    #[doc(hidden)]
    pub(crate) unsafe fn consume(
        self,
    ) -> (&'static str, WeakManagedObject, ObjectCreationSignalList) {
        let signal_list = std::ptr::read(self.signal_list.get());
        let ptr = std::ptr::read(&self.ptr);
        let class_name = self.class_name;
        std::mem::forget(self);
        (class_name, ptr, signal_list)
    }
}

impl Drop for ObjectCreationMetadata {
    fn drop(&mut self) {
        panic!("ObjectCreationMetadata dropped without being submitted to Object.");
    }
}

pub trait PartialObjectClass: Sized {
    const PARTIAL_OBJECT_CLASS_INFO: PartialObjectClassInfo;
}

pub trait ObjectClass: Sized {
    const OBJECT_CLASS_INFO: ObjectClassInfo;
}

impl<T: IObject, A: Allocator + Clone + Send + Sync> IWeak<T, A> {
    pub fn cast_to_object(&self) -> IWeak<DynObject, A> {
        let (header_raw, p_raw, alloc) = self.clone().into_inner_with_allocator();
        let p = p_raw as *mut DynObject;
        let header =
            unsafe { NonNull::new_unchecked(header_raw.as_ptr() as *mut IrcHead<DynObject>) };
        IWeak::from_inner_with_allocator((header, p, alloc))
    }
}

impl Debug for DynObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{:x}", &raw const *self as *const u8 as usize))
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InstanceMask {
    None = 0,
    NonConstructible = 1,
    NoClone = 2,
    ParentChangeLocked = 4,
    NoDestruction = 8,
}

impl BitOr for InstanceMask {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        unsafe { std::mem::transmute(self as u8 | rhs as u8) }
    }
}

impl BitAnd for InstanceMask {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        unsafe { std::mem::transmute(self as u8 & rhs as u8) }
    }
}

impl InstanceMask {
    pub const fn from_raw(v: u8) -> Self {
        unsafe { transmute(v) }
    }
}

#[derive(Debug)]
pub struct PartialInstanceClassInfo {
    pub class_name: &'static str,
    pub hierarchy: &'static [&'static str],
    pub properties: &'static [ObjectPropertyInfo],
    pub virtual_funcs: &'static [VirtualLuaFunctionInfo],
    pub signals: &'static [&'static str],
    pub mask: ObjectMask,
    pub instance_mask: InstanceMask,
}

#[derive(Debug)]
pub struct InstanceClassInfo {
    pub partial_info: PartialInstanceClassInfo,
    pub method_list: &'static [LuaFunctionInfo],
}

pub trait PartialInstanceClass {
    const PARTIAL_INSTANCE_CLASS_INFO: PartialInstanceClassInfo;
}
pub trait InstanceClass {
    const INSTANCE_CLASS_INFO: InstanceClassInfo;
}
