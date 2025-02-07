use enums::LuaEnums;
use r2g_mlua::prelude::*;

pub trait LuaSingleton {
    #[allow(unused_variables)]
    fn register_singleton(lua: &Lua) -> LuaResult<()> {
        Ok(())
    }
}

macro_rules! from_lua_copy_impl {
    ($name: ident) => {
        impl FromLua for $name {
            fn from_lua(value: LuaValue, _lua: &Lua) -> LuaResult<Self> {
                let ud = value.as_userdata();
                if ud.is_none() {
                    Err(LuaError::FromLuaConversionError {
                        from: value.type_name(),
                        to: stringify!($name).into(),
                        message: None,
                    })
                } else {
                    let unwrapped = unsafe { ud.unwrap_unchecked() }.borrow::<$name>();
                    if unwrapped.is_err() {
                        Err(LuaError::FromLuaConversionError {
                            from: "userdata",
                            to: stringify!($name).into(),
                            message: None,
                        })
                    } else {
                        unsafe { Ok(*unwrapped.unwrap_unchecked()) }
                    }
                }
            }
        }
    };
}
macro_rules! from_lua_clone_impl {
    ($name: ident) => {
        impl FromLua for $name {
            fn from_lua(value: LuaValue, _lua: &Lua) -> LuaResult<Self> {
                let ud = value.as_userdata();
                if ud.is_none() {
                    Err(LuaError::FromLuaConversionError {
                        from: value.type_name(),
                        to: stringify!($name).into(),
                        message: None,
                    })
                } else {
                    let unwrapped = unsafe { ud.unwrap_unchecked() }.borrow::<$name>();
                    if unwrapped.is_err() {
                        Err(LuaError::FromLuaConversionError {
                            from: "userdata",
                            to: stringify!($name).into(),
                            message: None,
                        })
                    } else {
                        unsafe { Ok(unwrapped.unwrap_unchecked().clone()) }
                    }
                }
            }
        }
    };
}
pub(self) use from_lua_clone_impl;

mod axes;
mod cframe;
pub mod enums;
mod events;
mod instance;
mod vectors;

pub use axes::Axes;
pub use vectors::{Vector2int16, Vector3int16};
pub type Vector2 = vectors::Vector2<f64>;
pub type Vector3 = vectors::Vector3<f64>;
pub use cframe::CFrame;
pub use events::{ManagedRBXScriptSignal, RBXScriptConnection, RBXScriptSignal};

use crate::instance::ManagedInstance;

pub fn register_userdata_singletons(lua: &mut Lua) -> LuaResult<()> {
    Axes::register_singleton(lua)?;
    CFrame::register_singleton(lua)?;

    Vector2::register_singleton(lua)?;
    Vector2int16::register_singleton(lua)?;
    Vector3::register_singleton(lua)?;
    Vector3int16::register_singleton(lua)?;

    ManagedInstance::register_singleton(lua)?;

    LuaEnums::register_singleton(lua)?;

    Ok(())
}
