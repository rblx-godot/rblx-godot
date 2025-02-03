#![allow(dead_code)]

#![feature(trait_upcasting)]
#![feature(ptr_metadata)]
#![feature(arbitrary_self_types)]
#![feature(negative_impls)]
#![feature(variant_count)]
#![feature(panic_always_abort)]

#![allow(internal_features)]
#![feature(core_intrinsics)]

#[rustversion::not(nightly)]
compile_error!("This crate can only be built with nightly rust due to the use of unstable features.");

pub mod core;
pub mod instance;
pub mod userdata;
mod godot_vm_bindings;

use core::verify_gdext_api_compat;

pub use godot_vm_bindings::RobloxVMNode;

use godot::prelude::*;
use roblox_to_godot_project_derive::methods;
use rustversion_detect::RUST_VERSION;

#[cfg(debug_assertions)]
macro_rules! godot_debug {
    ($fmt:literal $(, $args:expr)* $(,)?) => {
        godot::prelude::godot_print_rich!("[color=cyan]{}[/color]\n[color=gray]stack traceback:\n{}[/color]", 
            format!($fmt, $(, $args)*), 
            std::backtrace::Backtrace::force_capture()
        );
    };
    ($thing:expr) => {
        godot::prelude::godot_print_rich!("[color=cyan]{}[/color]\n[color=gray]stack traceback:\n{}[/color]", 
            format!("{} = {:?}", stringify!($thing), $thing), 
            std::backtrace::Backtrace::force_capture()
        );
    };
    (backtrace $thing:expr) => {
        godot::prelude::godot_print_rich!("[color=gray]stack traceback:\n{}[/color]", $thing);
    };
}
#[cfg(not(debug_assertions))]
macro_rules! godot_debug {
    ($fmt:literal $(, $args:expr)* $(,)?) => {};
    ($thing:expr) => {};
}
pub(crate) use godot_debug;

struct RobloxToGodotProjectExtension;

#[gdextension]
unsafe impl ExtensionLibrary for RobloxToGodotProjectExtension {
    fn min_level() -> InitLevel {
        InitLevel::Scene
    }

    fn on_level_init(level: InitLevel) {
        
        match level {
            InitLevel::Scene => {
                verify_gdext_api_compat();

                // Currently, rust panicking leaves Luau in a corrupted state.
                // I am unsure if this is due to mlua or due to task scheduler's exec raw.
                std::panic::always_abort();

                godot_print!("Roblox To Godot Project v{} (Rust runtime v{}) by {}\n", env!("CARGO_PKG_VERSION"), RUST_VERSION, {
                    let authors: &'static str = env!("CARGO_PKG_AUTHORS");
                    authors.replace(":", ", ")
                });
                /*
                let mut roblox_vm = RobloxVM::new(None);
                let env = roblox_vm.get_mut().get_main_state().create_env_from_global().unwrap();
                roblox_vm.get_mut()
                    .get_main_state()
                    .compile_jit("test.lua", include_str!("test.lua"), env).unwrap()
                    .call::<()>(()).unwrap();
                */
                
            }
            _ => ()
        }
    }
}

#[roblox_to_godot_project_derive::instance(hierarchy=[ServiceProvider], no_clone, parent_locked)]
#[method(func = fn some_func(), name = "SomeFunc", security_context = PluginSecurity)]
#[method(func = fn another_func(_: String) -> usize, name = "AnotherFunc", security_context = PluginSecurity)]
struct TestInstance {
    #[property(name = "meow", readonly)]
    meow: u32,
    a_field: String,
    #[property(name = "owo", readonly)]
    another_field: String,
}

#[methods]
impl TestInstance {
    fn some_func() {}
    fn another_func(s: String) -> usize { s.len() }
}

// impl crate::instance::IInstanceComponent for TestInstanceComponent {
//     fn lua_get(self: &mut core::RwLockReadGuard<'_, Self>, ptr: &instance::DynInstance, lua: &r2g_mlua::Lua, key: &String) -> Option<r2g_mlua::Result<r2g_mlua::Value>> {
//         todo!()
//     }

//     fn lua_set(self: &mut core::RwLockWriteGuard<'_, Self>, ptr: &instance::DynInstance, lua: &r2g_mlua::Lua, key: &String, value: &r2g_mlua::Value) -> Option<r2g_mlua::Result<()>> {
//         todo!()
//     }

//     fn clone(self: &core::RwLockReadGuard<'_, Self>, new_ptr: &instance::WeakManagedInstance) -> r2g_mlua::Result<Self> {
//         todo!()
//     }

//     fn new(ptr: instance::WeakManagedInstance, class_name: &'static str) -> Self {
//         todo!()
//     }
// }