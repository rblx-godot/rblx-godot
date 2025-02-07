use rblx_godot_derive::lua_enum;

#[lua_enum]
pub enum RunContext {
    Legacy,
    Server,
    Client,
    Plugin,
}
