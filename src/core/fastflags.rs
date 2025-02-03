use std::{cell::UnsafeCell, mem::{transmute, variant_count, ManuallyDrop, MaybeUninit}};

use bevy_reflect::{Reflect, TypeInfo};

use super::{LuauState, RobloxVM, RwLock};

#[derive(Reflect, Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u16)]
#[non_exhaustive]
pub enum FastFlag {
    TargetFPS,                // float
    VSync,                    // bool
    TargetPhysicsFPS,         // float
    MaxPhysicsStepsPerFrame,  // int

    GameId,                   // int
    GameName,                 // string
    CreatorId,                // int
    CreatorType,              // int
    PlaceId,                  // int
    JobId,                    // string
    PlaceVersion,             // int
    PrivateServerId,          // string
    PrivateServerOwnerId,     // int

    GlobalsReadonly,          // bool
    IsClient,                 // bool
    IsStudio,                 // bool
    DebugMode,                // bool

    SignalBehavior            // int
}
union FlagInternal {
    bool_value: bool,
    int_value: i64,
    float_value: f64,
    str_value: ManuallyDrop<String>
}

impl FlagInternal {
    #[inline(always)]
    fn get_string(&self, flag: FastFlag) -> String {
        match flag {
            FastFlag::JobId |
            FastFlag::PrivateServerId |
            FastFlag::GameName => unsafe { String::clone(&self.str_value) },
            _ => panic!("Invalid flag")
        }
    }
    #[inline(always)]
    const fn get_int(&self, flag: FastFlag) -> i64 {
        match flag {
            FastFlag::MaxPhysicsStepsPerFrame |
            FastFlag::GameId |
            FastFlag::CreatorId |
            FastFlag::PlaceId |
            FastFlag::PlaceVersion |
            FastFlag::PrivateServerOwnerId |
            FastFlag::SignalBehavior => unsafe { self.int_value },
            _ => panic!("Invalid flag")
        }
    }
    #[inline(always)]
    const fn get_float(&self, flag: FastFlag) -> f64 {
        match flag {
            FastFlag::TargetFPS | 
            FastFlag::TargetPhysicsFPS => unsafe { self.float_value },
            _ => panic!("Invalid flag")
        }
    }
    #[inline(always)]
    const fn get_bool(&self, flag: FastFlag) -> bool {
        match flag {
            FastFlag::VSync |
            FastFlag::GlobalsReadonly |
            FastFlag::IsClient |
            FastFlag::IsStudio |
            FastFlag::DebugMode => unsafe { self.bool_value },
            _ => panic!("Invalid flag")
        }
    }
    #[inline(always)]
    fn set_string(&mut self, flag: FastFlag, v: String) {
        match flag {
            FastFlag::JobId |
            FastFlag::PrivateServerId |
            FastFlag::GameName => unsafe {
                ManuallyDrop::drop(&mut self.str_value);
                self.str_value = ManuallyDrop::new(v);
            }
            _ => panic!("Invalid flag")
        }
    }
    #[inline(always)]
    const fn set_int(&mut self, flag: FastFlag, v: i64) {
        match flag {
            FastFlag::MaxPhysicsStepsPerFrame |
            FastFlag::GameId |
            FastFlag::CreatorId |
            FastFlag::CreatorType |
            FastFlag::PlaceId |
            FastFlag::PlaceVersion |
            FastFlag::PrivateServerOwnerId |
            FastFlag::SignalBehavior => self.int_value = v,
            _ => panic!("Invalid flag")
        }
    }
    #[inline(always)]
    const fn set_float(&mut self, flag: FastFlag, v: f64) {
        match flag {
            FastFlag::TargetFPS | 
            FastFlag::TargetPhysicsFPS => self.float_value = v,
            _ => panic!("Invalid flag")
        }
    }
    #[inline(always)]
    const fn set_bool(&mut self, flag: FastFlag, v: bool) {
        match flag {
            FastFlag::VSync |
            FastFlag::GlobalsReadonly |
            FastFlag::IsClient |
            FastFlag::IsStudio |
            FastFlag::DebugMode => self.bool_value = v,
            _ => panic!("Invalid flag")
        }
    }
    fn get_value(&self, flag: FastFlag) -> FastFlagValue {
        match flag {
            FastFlag::JobId |
            FastFlag::GameName |
            FastFlag::PrivateServerId => unsafe { FastFlagValue::String(String::clone(&self.str_value)) },
            FastFlag::MaxPhysicsStepsPerFrame |
            FastFlag::GameId |
            FastFlag::CreatorId |
            FastFlag::CreatorType |
            FastFlag::PlaceId |
            FastFlag::PlaceVersion |
            FastFlag::PrivateServerOwnerId |
            FastFlag::SignalBehavior => unsafe { FastFlagValue::Int(self.int_value) },
            FastFlag::TargetFPS |
            FastFlag::TargetPhysicsFPS => unsafe { FastFlagValue::Float(self.float_value) },
            FastFlag::VSync |
            FastFlag::GlobalsReadonly |
            FastFlag::IsClient |
            FastFlag::DebugMode |
            FastFlag::IsStudio => unsafe { FastFlagValue::Bool(self.bool_value) }
        }
    }
}

impl FastFlag {
    fn default_value(self) -> FlagInternal {
        match self {
            Self::TargetFPS => FlagInternal { float_value: 60.0 },
            Self::VSync => FlagInternal { bool_value: true },
            Self::TargetPhysicsFPS => FlagInternal { float_value: 60.0 },
            Self::MaxPhysicsStepsPerFrame => FlagInternal { int_value: 8 },
            
            Self::GameId => FlagInternal { int_value: 0 },
            Self::GameName => FlagInternal { str_value: ManuallyDrop::new(String::from("RobloxToGodotProject")) },
            Self::CreatorId => FlagInternal { int_value: 0 },
            Self::CreatorType => FlagInternal { int_value: 0 },
            Self::PlaceId => FlagInternal { int_value: 0 },
            Self::JobId => FlagInternal { str_value: ManuallyDrop::new(String::new()) },
            Self::PlaceVersion => FlagInternal { int_value: 1 },
            Self::PrivateServerId => FlagInternal { str_value: ManuallyDrop::new(String::from("reserved server")) },
            Self::PrivateServerOwnerId => FlagInternal { int_value: 0 },

            Self::GlobalsReadonly => FlagInternal { bool_value: false },
            Self::IsClient => FlagInternal { bool_value: true },
            Self::IsStudio => FlagInternal { bool_value: false },
            Self::DebugMode => FlagInternal { bool_value: true },
            
            Self::SignalBehavior => FlagInternal { int_value: 0 }
        }
    }
    pub fn get_default(self) -> FastFlagValue {
        self.default_value().get_value(self)
    }
    pub fn get_enum_flag_info(self) -> Option<TypeInfo> {
        match self {
            _ => None
        }
    }
}

type FlagsInternal = [FlagInternal; variant_count::<FastFlag>()];

pub enum FastFlagValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String)
}

pub struct FastFlags {
    flags: UnsafeCell<FlagsInternal>,
    vm: *mut RwLock<RobloxVM>
}

unsafe impl Send for FastFlags {}
unsafe impl Sync for FastFlags {}

impl FastFlags {
    pub(in crate::core) fn new(vm: *mut RwLock<RobloxVM>) -> FastFlags {
        let mut flags = MaybeUninit::<FlagsInternal>::uninit();
        unsafe {
            flags.assume_init_mut()
                .iter_mut()
                .enumerate()
                .for_each(|(i, x)| {
                    *x = FastFlag::default_value(transmute(i as u16));
                });
        }
        FastFlags {
            flags: UnsafeCell::new(unsafe { flags.assume_init() }),
            vm
        }
    }
    #[inline]
    pub const fn from_vm(vm: *mut RwLock<RobloxVM>) -> &'static FastFlags {
        unsafe {
            vm.as_ref().unwrap_unchecked().access().as_ref().unwrap_unchecked().flags()
        }
    }
    #[inline(always)]
    pub const fn from_state(state: &LuauState) -> &'static FastFlags {
        state.flags()
    }
    #[inline(always)]
    const fn get_flag_mut_internal(&self, flag: FastFlag) -> &mut FlagInternal {
        unsafe { 
            self.flags.get()
                .as_mut().unwrap_unchecked()
                .as_mut_ptr()
                .add(flag as usize)
                .as_mut().unwrap_unchecked()
        }
    }
    #[inline(always)]
    const fn get_flag_internal(&self, flag: FastFlag) -> &FlagInternal {
        // SAFETY: For every fast flag, there is an element inside the slice.
        // The size of the slice is equal to the number of variants in FastFlag.
        unsafe { 
            self.flags.get()
                .as_ref().unwrap_unchecked()
                .as_ptr()
                .add(flag as usize)
                .as_ref().unwrap_unchecked()
        }
    }
    pub fn get_string(&self, flag: FastFlag) -> String {
        self.get_flag_internal(flag).get_string(flag)
    }
    #[inline]
    pub const fn get_int(&self, flag: FastFlag) -> i64 {
        self.get_flag_internal(flag).get_int(flag)
    }
    #[inline]
    pub const fn get_float(&self, flag: FastFlag) -> f64 {
        self.get_flag_internal(flag).get_float(flag)
    }
    #[inline]
    pub const fn get_bool(&self, flag: FastFlag) -> bool {
        self.get_flag_internal(flag).get_bool(flag)
    }
    pub fn set_string(&self, flag: FastFlag, v: String) {
        unsafe {
            assert!(!self.vm.as_ref().unwrap_unchecked()
                .access().as_ref().unwrap_unchecked()
                .get_global_lock_state());
        }
        self.get_flag_mut_internal(flag).set_string(flag, v)
    }
    pub fn set_int(&self, flag: FastFlag, v: i64) {
        unsafe {
            assert!(!self.vm.as_ref().unwrap_unchecked()
                .access().as_ref().unwrap_unchecked()
                .get_global_lock_state());
        }
        self.get_flag_mut_internal(flag).set_int(flag, v)
    }
    pub fn set_float(&self, flag: FastFlag, v: f64) {
        unsafe {
            assert!(!self.vm.as_ref().unwrap_unchecked()
                .access().as_ref().unwrap_unchecked()
                .get_global_lock_state());
        }
        self.get_flag_mut_internal(flag).set_float(flag, v)
    }
    pub fn set_bool(&self, flag: FastFlag, v: bool) {
        unsafe {
            assert!(!self.vm.as_ref().unwrap_unchecked()
                .access().as_ref().unwrap_unchecked()
                .get_global_lock_state());
        }
        self.get_flag_mut_internal(flag).set_bool(flag, v)
    }
    pub(super) fn initialize_with_table<'a>(&mut self, table: Vec<(FastFlag, FastFlagValue)>) {
        for (flag, value) in table {
            match value {
                FastFlagValue::Bool(v) => self.get_flag_mut_internal(flag).set_bool(flag, v),
                FastFlagValue::Int(v) => self.get_flag_mut_internal(flag).set_int(flag, v),
                FastFlagValue::Float(v) => self.get_flag_mut_internal(flag).set_float(flag, v),
                FastFlagValue::String(v) => self.get_flag_mut_internal(flag).set_string(flag, v)
            }
        }
    }
    pub fn get_flag_value(&self, flag: FastFlag) -> FastFlagValue {
        self.get_flag_internal(flag).get_value(flag)
    }
}
