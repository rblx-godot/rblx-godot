use crate::userdata::LuaSingleton;
use r2g_mlua::prelude::*;

#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub struct Color3 {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Color3 {
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Color3 { r, g, b }
    }
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Color3 {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
        }
    }
    pub fn from_hsv(h: f32, s: f32, v: f32) -> Self {
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r, g, b) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        Color3 {
            r: r + m,
            g: g + m,
            b: b + m,
        }
    }
    pub fn from_hex(hex: String) -> LuaResult<Self> {
        let hex = hex.trim_start_matches('#');
        match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1], 16).unwrap();
                let g = u8::from_str_radix(&hex[1..2], 16).unwrap();
                let b = u8::from_str_radix(&hex[2..3], 16).unwrap();
                Ok(Color3::from_rgb(r, g, b))
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
                Ok(Color3::from_rgb(r, g, b))
            }
            _ => Err(LuaError::RuntimeError(format!(
                "Invalid hex color format: {}",
                hex
            ))),
        }
    }

    pub fn lerp(self, other: Color3, t: f32) -> Color3 {
        Color3 {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
        }
    }
    pub fn to_hex(&self) -> String {
        format!(
            "#{:02x}{:02x}{:02x}",
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8
        )
    }
    pub fn to_hsv(&self) -> (f32, f32, f32) {
        let max = self.r.max(self.g).max(self.b);
        let min = self.r.min(self.g).min(self.b);
        let delta = max - min;

        let mut h = 0.0;
        if delta > 0.0 {
            if max == self.r {
                h = (self.g - self.b) / delta;
            } else if max == self.g {
                h = 2.0 + (self.b - self.r) / delta;
            } else {
                h = 4.0 + (self.r - self.g) / delta;
            }
            h *= 60.0;
            if h < 0.0 {
                h += 360.0;
            }
        }

        let s = if max == 0.0 { 0.0 } else { delta / max };
        (h, s, max)
    }
}

from_lua_copy_impl!(Color3);

impl LuaUserData for Color3 {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("r", |_, color| Ok(color.r));
        fields.add_field_method_get("g", |_, color| Ok(color.g));
        fields.add_field_method_get("b", |_, color| Ok(color.b));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("Lerp", |_, a, (b, t): (Color3, f32)| Ok(a.lerp(b, t)));
        methods.add_method("ToHex", |_, a, ()| Ok(a.to_hex()));
        methods.add_method("ToHSV", |_, a, ()| Ok(a.to_hsv()));
    }
}

impl LuaSingleton for Color3 {
    fn register_singleton(lua: &r2g_mlua::Lua) -> r2g_mlua::Result<()> {
        let class = lua.create_table()?;
        class.set(
            "new",
            lua.create_function(|_, (r, g, b): (f32, f32, f32)| Ok(Color3::new(r, g, b)))?,
        )?;
        class.set(
            "FromRGB",
            lua.create_function(|_, (r, g, b): (u8, u8, u8)| Ok(Color3::from_rgb(r, g, b)))?,
        )?;
        class.set(
            "FromHSV",
            lua.create_function(|_, (h, s, v): (f32, f32, f32)| Ok(Color3::from_hsv(h, s, v)))?,
        )?;
        class.set(
            "FromHex",
            lua.create_function(|_, hex: String| Color3::from_hex(hex))?,
        )?;
        class.set_readonly(true);
        lua.globals().set("Color3", class)?;
        Ok(())
    }
}
