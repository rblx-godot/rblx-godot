use rblx_godot_derive::lua_enum;

#[lua_enum]
pub enum MessageType {
    MessageOutput,
    MessageInfo,
    MessageWarning,
    MessageError,
}
