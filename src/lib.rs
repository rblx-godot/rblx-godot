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

struct RblxGodotExtension;

#[gdextension]
unsafe impl ExtensionLibrary for RblxGodotExtension {
    fn min_level() -> InitLevel {
        InitLevel::Scene
    }

    fn on_level_init(level: InitLevel) {
        
        match level {
            InitLevel::Scene => {
                verify_gdext_api_compat();

                // Currently, rust panicking leaves Luau in a corrupted state.
                // I am unsure if this is due to mlua or due to task scheduler's exec raw.
                // todo! verify if this is actually needed
                std::panic::always_abort(); 

                godot_print!("rblx-godot v{} (Rust runtime v{}) by {}\n", env!("CARGO_PKG_VERSION"), RUST_VERSION, {
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