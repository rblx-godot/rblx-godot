use std::ops::{Add, Div, Mul, Neg, Sub};
use std::sync::Arc;

use godot::builtin::math::FloatExt;
use r2g_mlua::prelude::*;

use super::enums::{Axis, NormalId};
use super::LuaSingleton;

/// The [`Vector2`](https://create.roblox.com/docs/en-us/reference/engine/datatypes/Vector2) data type represents a 2D value with direction and magnitude. Some applications include GUI elements and 2D mouse positions.
#[derive(Clone, Copy, Default, PartialEq, PartialOrd, Debug, Hash)]
pub struct Vector2<T = f64> {
    pub x: T,
    pub y: T
}
/// The [`Vector2int16`](https://create.roblox.com/docs/en-us/reference/engine/datatypes/Vector2int16) data type represents a vector in 2D space with a signed 16-bit integer for its components. It is similar to [`Vector2`](https://docs.rs/roblox-to-godot-project/latest/roblox_to_godot_project/userdata/type.Vector2.html) in that it allows for the same arithmetic operations, but it lacks commonly used vector functions.
/// 
/// `Vector2int16` should not be confused with:
/// - [`Vector2`](https://docs.rs/roblox-to-godot-project/latest/roblox_to_godot_project/userdata/type.Vector2.html), a more precise and complete implementation for 2D vectors.
/// - [`Vector3int16`](https://docs.rs/roblox-to-godot-project/latest/roblox_to_godot_project/userdata/type.Vector3int16.html), a similar implementation for 3D vectors.
/// 
/// For each component:
/// - The lower bound is -2^15^, or -32,768.
/// - The upper bound is 2^15^ − 1, or 32,767.
pub type Vector2int16 = Vector2<i16>;

/// The [`Vector3`](https://create.roblox.com/docs/en-us/reference/engine/datatypes/Vector3) data type represents a vector in 3D space, typically usually used as a point in 3D space or the dimensions of a rectangular prism. `Vector3` supports basic component-based arithmetic operations (sum, difference, product, and quotient) and these operations can be applied on the left or right hand side to either another `Vector3` or a number. It also features methods for common vector operations, such as Cross() and Dot().
#[derive(Clone, Copy, Default, PartialEq, PartialOrd, Debug, Hash)]
pub struct Vector3<T = f64> {
    pub x: T,
    pub y: T,
    pub z: T
}
/// The [`Vector3int16`](https://create.roblox.com/docs/en-us/reference/engine/datatypes/Vector3int16) data type represents a vector in 3D space with a signed 16-bit integer for its components. It is similar to [`Vector3`](https://docs.rs/roblox-to-godot-project/latest/roblox_to_godot_project/userdata/type.Vector3.html) in that it allows for the same arithmetic operations, but it lacks commonly used vector functions.
/// 
/// `Vector3int16` should not be confused with:
/// - [`Vector3`](https://docs.rs/roblox-to-godot-project/latest/roblox_to_godot_project/userdata/type.Vector3.html), a more precise and complete implementation for 3D vectors.
/// - [`Vector2int16`](https://docs.rs/roblox-to-godot-project/latest/roblox_to_godot_project/userdata/type.Vector2int16.html), a similar implementation for 2D vectors.
/// 
/// For each component:
/// - The lower bound is -2^15^, or -32,768.
/// - The upper bound is 2^15^ − 1, or 32,767.
pub type Vector3int16 = Vector3<i16>;

impl Vector2 {
    /// A `Vector2` with magnitude of zero.
    pub const ZERO: Vector2 = Vector2 {x: 0f64, y: 0f64};
    pub const ONE: Vector2 = Vector2 {x: 1f64, y: 1f64};
    pub const X_AXIS: Vector2 = Vector2 {x: 1f64, y: 0f64};
    pub const Y_AXIS: Vector2 = Vector2 {x: 0f64, y: 1f64};
    /// Creates a new `Vector2` with the specified x and y components.
    pub const fn new(x: f64, y: f64) -> Vector2 {
        Vector2 {
            x, y
        }
    }
    /// Returns the magnitude of the vector.
    pub fn get_magnitude(&self) -> f64 {
        (self.x*self.x + self.y*self.y).sqrt()
    }
    /// Sets the magnitude of the vector by getting the unit vector and multiplying it by the specified value.
    pub fn set_magnitude(&mut self, value: f64) {
        *self = self.get_unit()*value;
    }
    /// Returns a vector of same direction with magnitude of one.
    pub fn get_unit(&self) -> Vector2 {
        let magnitude = self.get_magnitude();
        Vector2 {
            x: self.x / magnitude,
            y: self.y / magnitude
        }
    }
    /// Returns the cross product of the vector with another vector.
    pub fn cross(&self, other: Vector2) -> f64 {
        self.x*other.y - self.y*other.x
    }
    /// Returns the absolute value of the vector.
    pub fn abs(&self) -> Vector2 {
        Vector2 {
            x: self.x.abs(),
            y: self.y.abs()
        }
    }
    /// Returns the smallest integer greater than or equal to the vector.
    pub fn ceil(&self) -> Vector2 {
        Vector2 {
            x: self.x.ceil(),
            y: self.y.ceil()
        }
    }
    /// Returns the largest integer less than or equal to the vector.
    pub fn floor(&self) -> Vector2 {
        Vector2 {
            x: self.x.floor(),
            y: self.y.floor()
        }
    }
    /// Returns a vector with the sign of each component.
    pub fn sign(&self) -> Vector2 {
        Vector2 {
            x: self.x.sign(),
            y: self.y.sign()
        }
    }
    /// Returns the dot product of the vector with another vector.
    pub fn dot(&self, other: Vector2) -> f64 {
        self.x*other.x+self.y*other.y
    }
    /// Returns the angle between the vector and another vector.
    pub fn get_angle(&self, other: Vector2, is_signed: bool) -> f64 {
        if is_signed {
            todo!();
        }
        (self.dot(other)/(self.get_magnitude()*other.get_magnitude())).acos()
    }
    /// Returns a vector linearly interpolated between the vector and another vector.
    pub fn lerp(&self, other: Vector2, alpha: f64) -> Vector2 {
        *self+(other-*self)*alpha
    }
    /// Returns a vector with each component set to the maximum of the vector and another vector.
    pub fn max(&self, other: Vector2) -> Vector2 {
        Vector2 {
            x: self.x.max(other.x),
            y: self.y.max(other.y)
        }
    }
    /// Returns a vector with each component set to the minimum of the vector and another vector.
    pub fn min(&self, other: Vector2) -> Vector2 {
        Vector2 {
            x: self.x.min(other.x),
            y: self.y.min(other.y)
        }
    }
    /// Returns whether the vector is approximately equal to another vector with a specified epsilon.
    pub fn fuzzy_eq(&self, other: Vector2, epsilon: f64) -> bool {
        (self.x-other.x).abs() < epsilon && (self.y-other.y).abs() < epsilon
    }
}

impl Vector2int16 {
    /// Creates a new `Vector2int16` with the specified x and y components.
    pub const fn new(x: i16, y: i16) -> Vector2int16 {
        Vector2int16 {
            x, y
        }
    }
}

impl<T> Add for Vector2<T>
where
    T: Add<T, Output = T> + Copy
{
    type Output = Vector2<T>;
    
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x+rhs.x,
            y: self.y+rhs.y
        }
    }
}
impl<T> Sub for Vector2<T>
where
    T: Sub<T, Output = T> + Copy
{
    type Output = Vector2<T>;
    
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x-rhs.x,
            y: self.y-rhs.y
        }
    }
}
impl<T> Mul for Vector2<T>
where
    T: Mul<T, Output = T> + Copy
{
    type Output = Vector2<T>;
    
    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x*rhs.x,
            y: self.y*rhs.y
        }
    }
}
impl<T> Div for Vector2<T>
where
    T: Div<T, Output = T> + Copy
{
    type Output = Vector2<T>;
    
    fn div(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x/rhs.x,
            y: self.y/rhs.y
        }
    }
}
impl Mul<f64> for Vector2 {
    type Output = Vector2;
    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            x: self.x*rhs,
            y: self.y*rhs
        }
    }
}
impl Div<f64> for Vector2 {
    type Output = Vector2;
    fn div(self, rhs: f64) -> Self::Output {
        Self {
            x: self.x/rhs,
            y: self.y/rhs
        }
    }
}

impl Neg for Vector2 {
    type Output = Vector2;
    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y
        }
    }
}

impl Mul<f64> for Vector2int16 {
    type Output = Vector2int16;
    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            x: self.x*rhs as i16,
            y: self.y*rhs as i16
        }
    }
}
impl Div<f64> for Vector2int16 {
    type Output = Vector2int16;
    fn div(self, rhs: f64) -> Self::Output {
        Self {
            x: self.x/rhs as i16,
            y: self.y/rhs as i16
        }
    }
}
impl Mul<i64> for Vector2int16 {
    type Output = Vector2int16;
    fn mul(self, rhs: i64) -> Self::Output {
        Self {
            x: self.x*rhs as i16,
            y: self.y*rhs as i16
        }
    }
}
impl Div<i64> for Vector2int16 {
    type Output = Vector2int16;
    fn div(self, rhs: i64) -> Self::Output {
        Self {
            x: self.x/rhs as i16,
            y: self.y/rhs as i16
        }
    }
}
impl Neg for Vector2int16 {
    type Output = Vector2int16;
    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y
        }
    }
}

from_lua_copy_impl!(Vector2);
from_lua_copy_impl!(Vector2int16);

impl LuaUserData for Vector2 {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("X", |_, this| Ok(this.x));
        fields.add_field_method_set("X", |_, this, v| Ok({this.x = v;}));
        
        fields.add_field_method_get("Y", |_, this| Ok(this.y));
        fields.add_field_method_set("Y", |_, this, v| Ok({this.y = v;}));

        fields.add_field_method_get("Unit", |_, this| Ok(this.get_unit()));

        fields.add_field_method_get("Magnitude", |_, this| Ok(this.get_magnitude()));
        fields.add_field_method_set("Magnitude", |_, this, val| Ok(this.set_magnitude(val)));
    }
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("Cross",|_, this, other| Ok(this.cross(other)));
        methods.add_method("Abs", |_, this, ()| Ok(this.abs()));
        methods.add_method("Ceil", |_, this, ()| Ok(this.ceil()));
        methods.add_method("Floor", |_, this, ()| Ok(this.floor()));
        methods.add_method("Sign", |_, this, ()| Ok(this.sign()));
        methods.add_method("Angle", 
            |_, this, (other, is_signed): (Vector2, Option<bool>)| 
                Ok(this.get_angle(other, is_signed.unwrap_or(false)))
        );
        methods.add_method("Dot",|_, this, other| Ok(this.dot(other)));
        methods.add_method("Lerp",|_, this, (other, alpha)| Ok(this.lerp(other, alpha)));
        methods.add_method("Max",|_, this, other| Ok(this.max(other)));
        methods.add_method("Min",|_, this, other| Ok(this.min(other)));
        methods.add_method("FuzzyEq",|_, this, (other, epsilon): (Vector2, f64)|
            Ok(this.fuzzy_eq(other, epsilon))
        );
        methods.add_meta_method("__add", |_, this, other| Ok(*this+other));
        methods.add_meta_method("__sub", |_, this, other| Ok(*this-other));
        methods.add_meta_method("__mul", |lua, this, val: LuaValue| {
            if val.is_number() {
                Ok(*this * unsafe { val.as_number().unwrap_unchecked() })
            } else if val.is_userdata() {
                let result = Vector2::from_lua(val, lua);
                if result.is_ok() {
                    Ok(*this * unsafe { result.unwrap_unchecked() })
                } else {
                    Err(LuaError::BadArgument {
                        to: Some("Vector2::__mul".into()),
                        pos: 2,
                        name: Some("other".into()),
                        cause: Arc::new(result.unwrap_err())
                    })
                }
            } else {
                Err(LuaError::BadArgument {
                    to: Some("Vector2::__mul".into()),
                    pos: 2,
                    name: Some("other".into()),
                    cause: Arc::new(LuaError::FromLuaConversionError {
                        from: val.type_name(),
                        to: "Vector2".into(),
                        message: None
                    })})
            }
        });
        methods.add_meta_method("__div", |lua, this, val: LuaValue| {
            if val.is_number() {
                Ok(*this / unsafe { val.as_number().unwrap_unchecked() })
            } else if val.is_userdata() {
                let result = Vector2::from_lua(val, lua);
                if result.is_ok() {
                    Ok(*this / unsafe { result.unwrap_unchecked() })
                } else {
                    Err(LuaError::BadArgument {
                        to: Some("Vector2::__div".into()),
                        pos: 2,
                        name: Some("other".into()),
                        cause: Arc::new(result.unwrap_err())
                    })
                }
            } else {
                Err(LuaError::BadArgument {
                    to: Some("Vector2::__div".into()),
                    pos: 2,
                    name: Some("other".into()),
                    cause: Arc::new(LuaError::FromLuaConversionError {
                        from: val.type_name(),
                        to: "Vector2".into(),
                        message: None
                    })})
            }
        });
        methods.add_meta_method("__tostring", |_, this, ()| Ok(format!("({}, {})",this.x,this.y)));
        methods.add_meta_method("__eq", |_, this, other| Ok(*this == other));
        methods.add_meta_method("__unm", |_, this, ()| Ok(-*this));
    }
}

impl LuaSingleton for Vector2 {
    fn register_singleton(lua: &Lua) -> LuaResult<()> {
        let table = lua.create_table()?;
        table.raw_set(
            "new",
            lua.create_function(|_, (x, y): (f64, f64)| {
                Ok(Vector2 {
                    x, y
                })
            })?
        )?;
        table.raw_set("zero", Vector2::ZERO)?;
        table.raw_set("one", Vector2::ONE)?;
        table.raw_set("xAxis", Vector2::X_AXIS)?;
        table.raw_set("yAxis", Vector2::Y_AXIS)?;
        lua.globals().raw_set("Vector2", table)?;
        Ok(())
    }
}

impl LuaUserData for Vector2int16 {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("X", |_, this| Ok(this.x));
        fields.add_field_method_set("X", |_, this, v| Ok({this.x = v;}));
        
        fields.add_field_method_get("Y", |_, this| Ok(this.y));
        fields.add_field_method_set("Y", |_, this, v| Ok({this.y = v;}));
    }
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method("__add", |_, this, other| Ok(*this+other));
        methods.add_meta_method("__sub", |_, this, other| Ok(*this-other));
        methods.add_meta_method("__mul", |lua, this, val: LuaValue| {
            if val.is_number() {
                Ok(*this * unsafe { val.as_number().unwrap_unchecked() })
            } else if val.is_userdata() {
                let result = Vector2int16::from_lua(val, lua);
                if result.is_ok() {
                    Ok(*this * unsafe { result.unwrap_unchecked() })
                } else {
                    Err(LuaError::BadArgument {
                        to: Some("Vector2int16::__mul".into()),
                        pos: 2,
                        name: Some("other".into()),
                        cause: Arc::new(result.unwrap_err())
                    })
                }
            } else {
                Err(LuaError::BadArgument {
                    to: Some("Vector2int16::__mul".into()),
                    pos: 2,
                    name: Some("other".into()),
                    cause: Arc::new(LuaError::FromLuaConversionError {
                        from: val.type_name(),
                        to: "Vector2int16".into(),
                        message: None
                    })})
            }
        });
        methods.add_meta_method("__div", |lua, this, val: LuaValue| {
            if val.is_number() {
                Ok(*this / unsafe { val.as_number().unwrap_unchecked() })
            } else if val.is_userdata() {
                let result = Vector2int16::from_lua(val, lua);
                if result.is_ok() {
                    Ok(*this / unsafe { result.unwrap_unchecked() })
                } else {
                    Err(LuaError::BadArgument {
                        to: Some("Vector2int16::__div".into()),
                        pos: 2,
                        name: Some("other".into()),
                        cause: Arc::new(result.unwrap_err())
                    })
                }
            } else {
                Err(LuaError::BadArgument {
                    to: Some("Vector2int16::__div".into()),
                    pos: 2,
                    name: Some("other".into()),
                    cause: Arc::new(LuaError::FromLuaConversionError {
                        from: val.type_name(),
                        to: "Vector2int16".into(),
                        message: None
                    })})
            }
        });
        methods.add_meta_method("__tostring", |_, this, ()| Ok(format!("({}, {})",this.x,this.y)));
        methods.add_meta_method("__eq", |_, this, other| Ok(*this == other));
        methods.add_meta_method("__unm", |_, this, ()| Ok(-*this));
    }
}

impl LuaSingleton for Vector2int16 {
    fn register_singleton(lua: &Lua) -> LuaResult<()> {
        let table = lua.create_table()?;
        table.raw_set(
            "new",
            lua.create_function(|_, (x, y): (i64, i64)| {
                Ok(Vector2int16 {
                    x: x as i16,
                    y: y as i16
                })
            })?
        )?;
        lua.globals().raw_set("Vector2int16", table)?;
        Ok(())
    }
}

impl From<[f64; 3]> for Vector3 {
    fn from(value: [f64; 3]) -> Self {
        Self {
            x: value[0],
            y: value[1],
            z: value[2]
        }
    }
}
impl From<Vector3> for [f64; 3] {
    fn from(value: Vector3) -> Self {
        [value.x, value.y, value.z]
    }
}

impl Vector3 {
    pub const ZERO: Vector3 = Vector3 { x: 0f64, y: 0f64, z: 0f64 };
    pub const ONE: Vector3 = Vector3 { x: 1f64, y: 1f64, z: 1f64 };
    pub const X_AXIS: Vector3 = Vector3 { x: 1f64, y: 0f64, z: 0f64 };
    pub const Y_AXIS: Vector3 = Vector3 { x: 0f64, y: 1f64, z: 0f64 };
    pub const Z_AXIS: Vector3 = Vector3 { x: 0f64, y: 0f64, z: 1f64 };

    pub const fn new(x: f64, y: f64, z: f64) -> Vector3 {
        Vector3 {
            x, y, z
        }
    }
    pub fn get_magnitude(&self) -> f64 {
        self.get_magnitude_squared().sqrt()
    }
    pub fn get_magnitude_squared(&self) -> f64 {
        self.x*self.x + self.y * self.y + self.z * self.z
    }
    pub fn get_unit(&self) -> Vector3 {
        let magnitude = self.get_magnitude();
        Vector3 {
            x: self.x / magnitude,
            y: self.y / magnitude,
            z: self.z / magnitude
        }
    }
    pub fn set_magnitude(&mut self, magnitude: f64) {
        *self = self.get_unit()*magnitude;
    }
    pub fn abs(&self) -> Vector3 {
        Vector3 {
            x: self.x.abs(),
            y: self.y.abs(),
            z: self.z.abs()
        }
    }
    pub fn ceil(&self) -> Vector3 {
        Vector3 {
            x: self.x.ceil(),
            y: self.y.ceil(),
            z: self.z.ceil()
        }
    }
    pub fn floor(&self) -> Vector3 {
        Vector3 {
            x: self.x.floor(),
            y: self.y.floor(),
            z: self.z.floor()
        }
    }
    pub fn sign(&self) -> Vector3 {
        Vector3 {
            x: self.x.sign(),
            y: self.y.sign(),
            z: self.z.sign()
        }
    }
    pub fn cross(&self, other: Vector3) -> Vector3 {
        Vector3 {
            x: self.y*other.z - self.z*other.y,
            y: self.z*other.x - self.x*other.z,
            z: self.x*other.y - self.y*other.x
        }
    }
    pub fn get_angle(&self, other: Vector3, axis: Option<Vector3>) -> f64 {
        if axis.is_some() {
            todo!();
        }
        (self.dot(other)/(self.get_magnitude()*other.get_magnitude())).acos()
    }
    pub fn dot(&self, other: Vector3) -> f64 {
        self.x*other.x+self.y*other.y+self.z*other.z
    }
    pub fn fuzzy_eq(&self, other: Vector3, epsilon: f64) -> bool {
        (self.x-other.x).abs() < epsilon && (self.y-other.y).abs() < epsilon && (self.z-other.z).abs() < epsilon
    }
    pub fn lerp(&self, other: Vector3, alpha: f64) -> Vector3 {
        *self+(other-*self)*alpha
    }
    pub fn max(&self, other: Vector3) -> Vector3 {
        Vector3 {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
            z: self.z.max(other.z)
        }
    }
    pub fn min(&self, other: Vector3) -> Vector3 {
        Vector3 {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
            z: self.z.min(other.z)
        }
    }
}

from_lua_copy_impl!(Vector3);
from_lua_copy_impl!(Vector3int16);

impl<T> Add for Vector3<T>
where
    T: Add<T, Output = T> + Copy
{
    type Output = Vector3<T>;
    
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x+rhs.x,
            y: self.y+rhs.y,
            z: self.z+rhs.z
        }
    }
}
impl<T> Sub for Vector3<T>
where
    T: Sub<T, Output = T> + Copy
{
    type Output = Vector3<T>;
    
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x-rhs.x,
            y: self.y-rhs.y,
            z: self.z-rhs.z
        }
    }
}
impl<T> Mul for Vector3<T>
where
    T: Mul<T, Output = T> + Copy
{
    type Output = Vector3<T>;
    
    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x*rhs.x,
            y: self.y*rhs.y,
            z: self.z*rhs.z
        }
    }
}
impl<T> Div for Vector3<T>
where
    T: Div<T, Output = T> + Copy
{
    type Output = Vector3<T>;
    
    fn div(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x/rhs.x,
            y: self.y/rhs.y,
            z: self.z/rhs.z
        }
    }
}
impl Mul<f64> for Vector3 {
    type Output = Vector3;
    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            x: self.x*rhs,
            y: self.y*rhs,
            z: self.z*rhs
        }
    }
}
impl Div<f64> for Vector3 {
    type Output = Vector3;
    fn div(self, rhs: f64) -> Self::Output {
        Self {
            x: self.x/rhs,
            y: self.y/rhs,
            z: self.z/rhs
        }
    }
}
impl Neg for Vector3 {
    type Output = Vector3;
    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z
        }
    }
}
impl Mul<f64> for Vector3int16 {
    type Output = Vector3int16;
    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            x: self.x*rhs as i16,
            y: self.y*rhs as i16,
            z: self.z*rhs as i16
        }
    }
}
impl Div<f64> for Vector3int16 {
    type Output = Vector3int16;
    fn div(self, rhs: f64) -> Self::Output {
        Self {
            x: self.x/rhs as i16,
            y: self.y/rhs as i16,
            z: self.z/rhs as i16
        }
    }
}
impl Mul<i64> for Vector3int16 {
    type Output = Vector3int16;
    fn mul(self, rhs: i64) -> Self::Output {
        Self {
            x: self.x*rhs as i16,
            y: self.y*rhs as i16,
            z: self.z*rhs as i16
        }
    }
}
impl Div<i64> for Vector3int16 {
    type Output = Vector3int16;
    fn div(self, rhs: i64) -> Self::Output {
        Self {
            x: self.x/rhs as i16,
            y: self.y/rhs as i16,
            z: self.z/rhs as i16
        }
    }
}
impl Neg for Vector3int16 {
    type Output = Vector3int16;
    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z
        }
    }
}

impl From<NormalId> for Vector3 {
    fn from(value: NormalId) -> Self {
        match value {
            NormalId::Back => Self::new(0.0, 0.0, -1.0),
            NormalId::Bottom => Self::new(0.0, -1.0, 0.0),
            NormalId::Front => Self::new(0.0, 0.0, 1.0),
            NormalId::Left => Self::new(-1.0, 0.0, 0.0),
            NormalId::Right => Self::new(1.0, 0.0, 0.0),
            NormalId::Top => Self::new(0.0, 1.0, 0.0)
        }
    }
}
impl From<Axis> for Vector3 {
    fn from(value: Axis) -> Self {
        match value {
            Axis::X => Self::X_AXIS,
            Axis::Y => Self::Y_AXIS,
            Axis::Z => Self::Z_AXIS
        }
    }
}

impl LuaUserData for Vector3 {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("X", |_, this| Ok(this.x));
        fields.add_field_method_set("X", |_, this, v| Ok({this.x = v;}));
        
        fields.add_field_method_get("Y", |_, this| Ok(this.y));
        fields.add_field_method_set("Y", |_, this, v| Ok({this.y = v;}));

        fields.add_field_method_get("Z", |_, this| Ok(this.z));
        fields.add_field_method_set("Z", |_, this, v| Ok({this.z = v;}));

        fields.add_field_method_get("Unit", |_, this| Ok(this.get_unit()));

        fields.add_field_method_get("Magnitude", |_, this| Ok(this.get_magnitude()));
        fields.add_field_method_set("Magnitude", |_, this, val| Ok(this.set_magnitude(val)));
    }
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("Cross",|_, this, other| Ok(this.cross(other)));
        methods.add_method("Abs", |_, this, ()| Ok(this.abs()));
        methods.add_method("Ceil", |_, this, ()| Ok(this.ceil()));
        methods.add_method("Floor", |_, this, ()| Ok(this.floor()));
        methods.add_method("Sign", |_, this, ()| Ok(this.sign()));
        methods.add_method("Angle", 
            |_, this, (other, axis): (Vector3, Option<Vector3>)| 
                Ok(this.get_angle(other, axis))
        );
        methods.add_method("Dot",|_, this, other| Ok(this.dot(other)));
        methods.add_method("Lerp",|_, this, (other, alpha)| Ok(this.lerp(other, alpha)));
        methods.add_method("Max",|_, this, other| Ok(this.max(other)));
        methods.add_method("Min",|_, this, other| Ok(this.min(other)));
        methods.add_method("FuzzyEq",|_, this, (other, epsilon): (Vector3, f64)| 
            Ok(this.fuzzy_eq(other, epsilon))
        );
        methods.add_meta_method("__add", |_, this, other| Ok(*this+other));
        methods.add_meta_method("__sub", |_, this, other| Ok(*this-other));
        methods.add_meta_method("__mul", |lua, this, val: LuaValue| {
            if val.is_number() {
                Ok(*this * unsafe { val.as_number().unwrap_unchecked() })
            } else if val.is_userdata() {
                let result = Vector3::from_lua(val, lua);
                if result.is_ok() {
                    Ok(*this * unsafe { result.unwrap_unchecked() })
                } else {
                    Err(LuaError::BadArgument {
                        to: Some("Vector3::__mul".into()),
                        pos: 2,
                        name: Some("other".into()),
                        cause: Arc::new(result.unwrap_err())
                    })
                }
            } else {
                Err(LuaError::BadArgument {
                    to: Some("Vector3::__mul".into()),
                    pos: 2,
                    name: Some("other".into()),
                    cause: Arc::new(LuaError::FromLuaConversionError {
                        from: val.type_name(),
                        to: "Vector3".into(),
                        message: None
                    })})
            }
        });
        methods.add_meta_method("__div", |lua, this, val: LuaValue| {
            if val.is_number() {
                Ok(*this / unsafe { val.as_number().unwrap_unchecked() })
            } else if val.is_userdata() {
                let result = Vector3::from_lua(val, lua);
                if result.is_ok() {
                    Ok(*this / unsafe { result.unwrap_unchecked() })
                } else {
                    Err(LuaError::BadArgument {
                        to: Some("Vector3::__div".into()),
                        pos: 2,
                        name: Some("other".into()),
                        cause: Arc::new(result.unwrap_err())
                    })
                }
            } else {
                Err(LuaError::BadArgument {
                    to: Some("Vector3::__div".into()),
                    pos: 2,
                    name: Some("other".into()),
                    cause: Arc::new(LuaError::FromLuaConversionError {
                        from: val.type_name(),
                        to: "Vector3".into(),
                        message: None
                    })})
            }
        });
        methods.add_meta_method("__tostring", |_, this, ()| Ok(format!("({}, {}, {})",this.x,this.y,this.z)));
        methods.add_meta_method("__eq", |_, this, other| Ok(*this == other));
        methods.add_meta_method("__unm", |_, this, ()| Ok(-*this));
    }
}

impl LuaSingleton for Vector3 {
    fn register_singleton(lua: &Lua) -> LuaResult<()> {
        let table = lua.create_table()?;
        table.raw_set(
            "new",
            lua.create_function(|_, (x, y, z): (f64, f64, f64)| {
                Ok(Vector3 {
                    x, y, z
                })
            })?
        )?;
        table.raw_set(
            "FromNormalId",
            lua.create_function(|_, v: NormalId| {
                Ok(Vector3::from(v))
            })?
        )?;
        table.raw_set(
            "FromAxis",
            lua.create_function(|_, v: Axis| {
                Ok(Vector3::from(v))
            })?
        )?;
        table.raw_set("zero", Vector3::ZERO)?;
        table.raw_set("one", Vector3::ONE)?;
        table.raw_set("xAxis", Vector3::X_AXIS)?;
        table.raw_set("yAxis", Vector3::Y_AXIS)?;
        lua.globals().raw_set("Vector3", table)?;
        Ok(())
    }
}

impl LuaUserData for Vector3int16 {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("X", |_, this| Ok(this.x));
        fields.add_field_method_set("X", |_, this, v| Ok({this.x = v;}));
        
        fields.add_field_method_get("Y", |_, this| Ok(this.y));
        fields.add_field_method_set("Y", |_, this, v| Ok({this.y = v;}));
    }
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method("__add", |_, this, other| Ok(*this+other));
        methods.add_meta_method("__sub", |_, this, other| Ok(*this-other));
        methods.add_meta_method("__mul", |lua, this, val: LuaValue| {
            if val.is_number() {
                Ok(*this * unsafe { val.as_number().unwrap_unchecked() })
            } else if val.is_userdata() {
                let result = Vector3int16::from_lua(val, lua);
                if result.is_ok() {
                    Ok(*this * unsafe { result.unwrap_unchecked() })
                } else {
                    Err(LuaError::BadArgument {
                        to: Some("Vector3int16::__mul".into()),
                        pos: 2,
                        name: Some("other".into()),
                        cause: Arc::new(result.unwrap_err())
                    })
                }
            } else {
                Err(LuaError::BadArgument {
                    to: Some("Vector3int16::__mul".into()),
                    pos: 2,
                    name: Some("other".into()),
                    cause: Arc::new(LuaError::FromLuaConversionError {
                        from: val.type_name(),
                        to: "Vector3int16".into(),
                        message: None
                    })})
            }
        });
        methods.add_meta_method("__div", |lua, this, val: LuaValue| {
            if val.is_number() {
                Ok(*this / unsafe { val.as_number().unwrap_unchecked() })
            } else if val.is_userdata() {
                let result = Vector3int16::from_lua(val, lua);
                if result.is_ok() {
                    Ok(*this / unsafe { result.unwrap_unchecked() })
                } else {
                    Err(LuaError::BadArgument {
                        to: Some("Vector3int16::__div".into()),
                        pos: 2,
                        name: Some("other".into()),
                        cause: Arc::new(result.unwrap_err())
                    })
                }
            } else {
                Err(LuaError::BadArgument {
                    to: Some("Vector3int16::__div".into()),
                    pos: 2,
                    name: Some("other".into()),
                    cause: Arc::new(LuaError::FromLuaConversionError {
                        from: val.type_name(),
                        to: "Vector3int16".into(),
                        message: None
                    })})
            }
        });
        methods.add_meta_method("__tostring", |_, this, ()| Ok(format!("({}, {}, {})",this.x,this.y,this.z)));
        methods.add_meta_method("__eq", |_, this, other| Ok(*this == other));
        methods.add_meta_method("__unm", |_, this, ()| Ok(-*this));
    }
}

impl LuaSingleton for Vector3int16 {
    fn register_singleton(lua: &Lua) -> LuaResult<()> {
        let table = lua.create_table()?;
        table.raw_set(
            "new",
            lua.create_function(|_, (x, y, z): (i64, i64, i64)| {
                Ok(Vector3int16 {
                    x: x as i16,
                    y: y as i16,
                    z: z as i16
                })
            })?
        )?;
        lua.globals().raw_set("Vector3int16", table)?;
        Ok(())
    }
}