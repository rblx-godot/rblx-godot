
macro_rules! lua_getter {
    ($lua: ident, $prop: expr) => {
        IntoLua::into_lua($prop, $lua)
    };
    (string, $lua: ident, $prop: expr) => {
        IntoLua::into_lua($prop.as_str(), $lua)
    };
    (clone, $lua: ident, $prop: expr) => {
        IntoLua::into_lua($prop.clone(), $lua)
    };
    (opt_clone, $lua: ident, $prop: expr) => {
        IntoLua::into_lua($prop.as_ref(), $lua)
    };
    (weak_clone, $lua: ident, $prop: expr) => {
        IntoLua::into_lua($prop.upgrade(), $lua)
    };
    (opt_weak_clone, $lua: ident, $prop: expr) => {
        IntoLua::into_lua($prop.as_ref().map(|x| x.upgrade()).flatten(), $lua)
    };
    (function, $lua: ident, $func: expr) => {
        Ok(r2g_mlua::Value::Function($lua.create_function($func)?))
    };
    (function_async, $lua: ident, $func: expr) => {
        Ok(r2g_mlua::Value::Function($lua.create_async_function($func)?))
    };
    (function_opt, $lua: ident, $func: expr) => {
        Some({
            let f = $lua.create_function($func);
            if let Ok(f) = f {
                Ok(r2g_mlua::Value::Function(f))
            } else {
                return Some(Err(f.err().unwrap()));
            }
        })
    };
    (function_async_opt, $lua: ident, $func: expr) => {
        Some({
            let f = $lua.create_async_function($func);
            if let Ok(f) = f {
                Ok(r2g_mlua::Value::Function(f))
            } else {
                return Some(Err(f.err().unwrap()));
            }
        })
    };
}
macro_rules! lua_setter {
    ($lua: ident, $prop: expr) => {
        FromLua::from_lua($prop, $lua)
    };
    (clone, $lua: ident, $prop: expr) => {
        FromLua::from_lua($prop.clone(), $lua)
    };
    (opt_clone, $lua: ident, $prop: expr) => {
        match FromLua::from_lua($prop.clone(), $lua) {
            Ok(res) => res,
            Err(err) => return Some(Err(err))
        }
    };
    
}
macro_rules! lua_invalid_argument {
    ($func_name: literal, $pos: expr, $arg_name: ident, $err: expr) => {
        LuaError::BadArgument { 
            to: Some($func_name.into()),
            pos: $pos,
            name: Some(stringify!($arg_name).into()),
            cause: std::sync::Arc::new($err)
        }
    };
    ($func_name: literal, $pos: expr, $arg_name: ident, $err: expr) => {
        LuaError::BadArgument { 
            to: Some($func_name.into()),
            pos: $pos,
            name: Some(stringify!($arg_name).into()),
            cause: std::sync::Arc::new($err)
        }
    };
    ($func_name: literal, $pos: expr, $arg_name: ident cast to $to: ident) => {
        LuaError::BadArgument { 
            to: Some($func_name.into()),
            pos: $pos,
            name: Some(stringify!($arg_name).into()),
            cause: std::sync::Arc::new(LuaError::FromLuaConversionError {
                from: $arg_name.type_name(),
                to: stringify!($to).into(),
                message: None
            })
        }
    };
    ($func_name: literal, $pos: expr, $arg_name: ident cast unknown to $to: ident) => {
        LuaError::BadArgument { 
            to: Some($func_name.into()),
            pos: $pos,
            name: Some(stringify!($arg_name).into()),
            cause: std::sync::Arc::new(LuaError::FromLuaConversionError {
                from: "",
                to: stringify!($to).into(),
                message: None
            })
        }
    };
    ($func_name: literal, $pos: expr, $arg_name: ident cast $from: ident to $to: ident) => {
        LuaError::BadArgument { 
            to: Some($func_name.into()),
            pos: $pos,
            name: Some(stringify!($arg_name).into()),
            cause: std::sync::Arc::new(LuaError::FromLuaConversionError {
                from: stringify!($from),
                to: stringify!($to).into(),
                message: None
            })
        }
    }
}

pub(crate) use lua_getter;
pub(crate) use lua_setter;
pub(crate) use lua_invalid_argument;