use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

use r2g_mlua::ffi::lua_clock;

#[derive(Debug)]
pub struct Watchdog {
    watchdog: f64,
    timeout: f64,
    flag: AtomicBool,
}

impl Watchdog {
    pub const fn new() -> Self {
        Self {
            watchdog: 0f64,
            timeout: 0f64,
            flag: AtomicBool::new(false),
        }
    }
    pub const fn new_timeout(timeout: f64) -> Self {
        Self {
            watchdog: 0f64,
            timeout,
            flag: AtomicBool::new(false),
        }
    }
    #[inline]
    pub fn set_timeout(&mut self, timeout: f64) {
        self.timeout = timeout;
    }
    #[inline(always)]
    fn clock() -> f64 {
        unsafe { lua_clock() }
    }
    #[inline]
    pub fn trip(&self) {
        self.flag.store(true, Relaxed);
    }
    #[inline]
    pub fn check(&self) -> bool {
        if self.flag.load(Relaxed) {
            true
        } else {
            let flag = self.timeout + self.watchdog < Self::clock() && self.timeout != 0f64;
            if flag {
                self.trip()
            }
            flag
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.watchdog = Self::clock();
        self.flag.store(false, Relaxed);
    }
    #[inline]
    pub fn disable(&mut self) {
        self.timeout = 0f64;
        self.flag.store(false, Relaxed);
    }
}

impl Default for Watchdog {
    fn default() -> Self {
        Self::new()
    }
}
