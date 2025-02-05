use std::intrinsics::abort;

use godot::classes::Engine;
use godot::global::godot_error;

pub(crate) fn force_unload() -> ! {
    abort();
}

pub(crate) fn verify_gdext_api_compat() {
    if !{
        let v = (*Engine::singleton())
            .get_copyright_info()
            .at(0)
            .get("name")
            .unwrap();
        let s = String::from(v.stringify());
        s.starts_with("Godot")
    } {
        godot_error!(
            "FATAL: incompatible gdextension api. Please update the gdextension api header."
        );
        force_unload();
    }
}
