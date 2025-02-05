use std::sync::Arc;

use r2g_mlua::prelude::*;

use super::{enums::NormalId, LuaSingleton};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct Axes {
    pub x: bool,
    pub y: bool,
    pub z: bool,
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
    pub back: bool,
    pub front: bool,
}

impl LuaUserData for Axes {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("X", |_, this| Ok(this.x));
        fields.add_field_method_get("Y", |_, this| Ok(this.y));
        fields.add_field_method_get("Z", |_, this| Ok(this.z));
        fields.add_field_method_set("X", |_, this, v| {
            Ok({
                this.x = v;
            })
        });
        fields.add_field_method_set("Y", |_, this, v| {
            Ok({
                this.y = v;
            })
        });
        fields.add_field_method_set("Z", |_, this, v| {
            Ok({
                this.z = v;
            })
        });

        fields.add_field_method_get("Top", |_, this| Ok(this.top));
        fields.add_field_method_set("Top", |_, this, v| {
            Ok({
                this.top = v;
            })
        });
        fields.add_field_method_get("Bottom", |_, this| Ok(this.bottom));
        fields.add_field_method_set("Bottom", |_, this, v| {
            Ok({
                this.bottom = v;
            })
        });
        fields.add_field_method_get("Left", |_, this| Ok(this.left));
        fields.add_field_method_set("Left", |_, this, v| {
            Ok({
                this.left = v;
            })
        });
        fields.add_field_method_get("Right", |_, this| Ok(this.right));
        fields.add_field_method_set("Right", |_, this, v| {
            Ok({
                this.right = v;
            })
        });
        fields.add_field_method_get("Back", |_, this| Ok(this.back));
        fields.add_field_method_set("Back", |_, this, v| {
            Ok({
                this.back = v;
            })
        });
        fields.add_field_method_get("Front", |_, this| Ok(this.front));
        fields.add_field_method_set("Front", |_, this, v| {
            Ok({
                this.front = v;
            })
        });
    }
}
impl LuaSingleton for Axes {
    fn register_singleton(lua: &Lua) -> LuaResult<()> {
        let table = lua.create_table()?;
        table.raw_set(
            "new",
            lua.create_function(|_lua, mult: LuaMultiValue| {
                let mut axes = Axes {
                    x: true,
                    y: true,
                    z: true,
                    top: true,
                    bottom: true,
                    left: true,
                    right: true,
                    back: true,
                    front: true,
                };
                for (i, value) in mult.into_iter().enumerate() {
                    if let Some(userdata) = value.as_userdata() {
                        let result = userdata.borrow::<NormalId>();
                        if result.is_ok() {
                            match unsafe { *result.unwrap_unchecked() } {
                                NormalId::Right => {
                                    axes.x = true;
                                    axes.right = true;
                                }
                                NormalId::Top => {
                                    axes.y = true;
                                    axes.top = true;
                                }
                                NormalId::Back => {
                                    axes.z = true;
                                    axes.back = true;
                                }
                                NormalId::Left => {
                                    axes.x = true;
                                    axes.left = true;
                                }
                                NormalId::Bottom => {
                                    axes.y = true;
                                    axes.bottom = true;
                                }
                                NormalId::Front => {
                                    axes.z = true;
                                    axes.front = true;
                                }
                            }
                        }
                    } else {
                        return Err(LuaError::BadArgument {
                            to: Some("Axes::new".into()),
                            pos: i + 1,
                            name: None,
                            cause: Arc::new(LuaError::FromLuaConversionError {
                                from: value.type_name(),
                                to: "Enum.NormalId".into(),
                                message: None,
                            }),
                        });
                    }
                }
                Ok(axes)
            })?,
        )?;
        lua.globals().set("Axes", table)?;
        Ok(())
    }
}
