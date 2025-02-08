use std::intrinsics::abort;

use godot::classes::Engine;
use godot::global::push_error;
use godot::meta::ToGodot;

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
        push_error(&[
            "FATAL: incompatible gdextension api. Please update the gdextension api header."
                .to_variant(),
        ]);
        force_unload();
    }
}
