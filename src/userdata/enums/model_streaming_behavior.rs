use rblx_godot_derive::lua_enum;

#[lua_enum(default=Legacy)]
pub enum ModelStreamingBehavior {
    Default,
    Legacy,
    Improved
}