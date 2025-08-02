use r2g_mlua::{FromLua, IntoLua};

use crate::{
    core::{lua_macros::from_lua, IObjectComponent, Trc},
    render::RenderBackend,
    userdata::{enums::Technology, Color3},
};

pub struct LightingComponent {
    ambient: Color3,
    brightness: f32,
    clock_time: f32,
    color_shift_bottom: Color3,
    color_shift_top: Color3,
    environment_diffuse_scale: f32,
    environment_specular_scale: f32,
    exposure_compensation: f32,
    fog_color: Color3,
    fog_end: f32,
    fog_start: f32,
    geographic_latitude: f32,
    global_shadows: bool,
    outdoor_ambient: Color3,
    shadow_softness: f32,
    technology: Technology,
    time_of_day: f32,

    render_backend: Trc<RenderBackend>,
}

impl IObjectComponent for LightingComponent {
    fn lua_get(
        self: &mut crate::core::RwLockReadGuard<'_, Self>,
        _ptr: &crate::core::DynObject,
        lua: &r2g_mlua::Lua,
        key: &String,
    ) -> Option<r2g_mlua::Result<r2g_mlua::Value>> {
        match key.as_str() {
            "Ambient" => Some(self.ambient.into_lua(lua)),
            "Brightness" => Some(self.brightness.into_lua(lua)),
            "ClockTime" => Some(self.clock_time.into_lua(lua)),
            "ColorShiftBottom" => Some(self.color_shift_bottom.into_lua(lua)),
            "ColorShiftTop" => Some(self.color_shift_top.into_lua(lua)),
            "EnvironmentDiffuseScale" => Some(self.environment_diffuse_scale.into_lua(lua)),
            "EnvironmentSpecularScale" => Some(self.environment_specular_scale.into_lua(lua)),
            "ExposureCompensation" => Some(self.exposure_compensation.into_lua(lua)),
            "FogColor" => Some(self.fog_color.into_lua(lua)),
            "FogEnd" => Some(self.fog_end.into_lua(lua)),
            "FogStart" => Some(self.fog_start.into_lua(lua)),
            "GeographicLatitude" => Some(self.geographic_latitude.into_lua(lua)),
            "GlobalShadows" => Some(self.global_shadows.into_lua(lua)),
            "OutdoorAmbient" => Some(self.outdoor_ambient.into_lua(lua)),
            "ShadowSoftness" => Some(self.shadow_softness.into_lua(lua)),
            "Technology" => Some(self.technology.into_lua(lua)),
            "TimeOfDay" => Some(self.time_of_day.into_lua(lua)),
            _ => None,
        }
    }

    fn lua_set(
        self: &mut crate::core::RwLockWriteGuard<'_, Self>,
        _ptr: &crate::core::DynObject,
        lua: &r2g_mlua::Lua,
        key: &String,
        value: &r2g_mlua::Value,
    ) -> Option<r2g_mlua::Result<()>> {
        match key.as_str() {
            "Ambient" => {
                self.ambient = from_lua!(lua? => value);
                self.render_backend.write().update_ambient(
                    self.ambient,
                    self.brightness,
                    self.environment_diffuse_scale,
                );
                Some(Ok(()))
            }
            "Brightness" => {
                self.brightness = from_lua!(lua? => value);
                self.render_backend.write().update_ambient(
                    self.ambient,
                    self.brightness,
                    self.environment_diffuse_scale,
                );
                Some(Ok(()))
            }
            "ClockTime" => todo!(),
            "ColorShiftBottom" => todo!(),
            "ColorShiftTop" => todo!(),
            "EnvironmentDiffuseScale" => {
                self.environment_diffuse_scale = from_lua!(lua? => value);
                self.render_backend.write().update_ambient(
                    self.ambient,
                    self.brightness,
                    self.environment_diffuse_scale,
                );
                Some(Ok(()))
            }
            "EnvironmentSpecularScale" => todo!(),
            _ => None,
        }
    }

    fn new(metadata: &crate::core::ObjectCreationMetadata) -> r2g_mlua::Result<Self> {
        todo!()
    }
}
