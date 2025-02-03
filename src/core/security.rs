use std::ops::BitOr;
#[derive(Eq, PartialEq, Clone, Copy, PartialOrd, Ord)]
pub struct SecurityContext(u8);
impl SecurityContext {
    pub const NONE: SecurityContext = SecurityContext(0);
    pub const PLUGIN: SecurityContext = SecurityContext(0x1);
    pub const ROBLOX_PLACE: SecurityContext = SecurityContext(0x2);
    pub const WRITE_PLAYER: SecurityContext = SecurityContext(0x4);
    pub const LOCAL_USER: SecurityContext = SecurityContext(0x10);
    pub const ROBLOX_SCRIPT: SecurityContext = SecurityContext(0x1D);
    pub const ROBLOX: SecurityContext = SecurityContext(0x1F);
    pub const TEST_LOCAL_USER: SecurityContext = SecurityContext(0x10);
}

impl BitOr for SecurityContext {
    type Output = SecurityContext;
    fn bitor(self, rhs: Self) -> Self::Output {
        SecurityContext(self.0 | rhs.0)
    }
}

impl SecurityContext {
    pub const fn has(self, other: SecurityContext) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl Into<u8> for SecurityContext {
    fn into(self) -> u8 {
        self.0
    }
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThreadIdentityType {
    Anon,
    /// UserInit is a thread that was created by a user, such as a plugin or a command bar script.
    UserInit,
    /// Script is a thread that was created by a script.
    Script,
    /// Script in a place made by Roblox.
    ScriptInRobloxPlace,
    /// Core script
    ScriptByRoblox,
    /// Command bar script
    StudioCommandBar,
    /// Studio plugin
    StudioPlugin,
    /// Web server
    WebServer,
    /// Replication from server to client
    Replication
}

impl Default for ThreadIdentityType {
    fn default() -> Self {
        ThreadIdentityType::Anon
    }
}
impl ThreadIdentityType {
    #[inline]
    pub const fn get_security_contexts(&self) -> SecurityContext {
        match self {
            ThreadIdentityType::Anon => SecurityContext::NONE,
            ThreadIdentityType::UserInit => 
                SecurityContext(SecurityContext::PLUGIN.0 | SecurityContext::ROBLOX_PLACE.0 | SecurityContext::LOCAL_USER.0),
            ThreadIdentityType::Script => SecurityContext::NONE,
            ThreadIdentityType::ScriptInRobloxPlace => SecurityContext::ROBLOX_PLACE,
            ThreadIdentityType::ScriptByRoblox => 
                SecurityContext(SecurityContext::PLUGIN.0 | SecurityContext::ROBLOX_PLACE.0 | SecurityContext::LOCAL_USER.0 | SecurityContext::ROBLOX_SCRIPT.0),
            ThreadIdentityType::StudioCommandBar => 
                SecurityContext(SecurityContext::PLUGIN.0 | SecurityContext::ROBLOX_PLACE.0 | SecurityContext::LOCAL_USER.0),
            ThreadIdentityType::StudioPlugin => SecurityContext::PLUGIN,
            ThreadIdentityType::WebServer => SecurityContext::ROBLOX,
            ThreadIdentityType::Replication => 
                SecurityContext(SecurityContext::WRITE_PLAYER.0 | SecurityContext::ROBLOX_PLACE.0 | SecurityContext::ROBLOX_SCRIPT.0)
        }
    }
}