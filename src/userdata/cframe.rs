use std::mem::take;
use std::ops::{Add, Mul, Sub};
use std::sync::Arc;

use r2g_mlua::prelude::*;

use super::{enums::RotationOrder, LuaSingleton, Vector3};

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub struct CFrame {
    pub rot_matrix: [[f64; 3]; 3],
    pub pos: [f64; 3],
}

impl Default for CFrame {
    fn default() -> Self {
        CFrame::IDENTITY
    }
}

from_lua_copy_impl!(CFrame);

impl CFrame {
    pub const IDENTITY: CFrame = CFrame {
        rot_matrix: [[1f64, 0f64, 0f64], [0f64, 1f64, 0f64], [0f64, 0f64, 1f64]],
        pos: [0f64, 0f64, 0f64],
    };
    pub const fn new() -> Self {
        CFrame::IDENTITY
    }
    pub const fn new_with_position(pos: Vector3) -> Self {
        CFrame {
            rot_matrix: CFrame::IDENTITY.rot_matrix,
            pos: [pos.x, pos.y, pos.z],
        }
    }
    pub fn new_looking_at(pos: Vector3, look_at: Vector3) -> Self {
        Self::look_at(pos, look_at, None)
    }
    pub const fn new_quaternion(pos: Vector3, q_x: f64, q_y: f64, q_z: f64, q_w: f64) -> Self {
        let d = q_x * q_x + q_y * q_y + q_z * q_z + q_w * q_w;
        let s = 2.0 / d;
        let (xs, ys, zs) = (q_x * s, q_y * s, q_z * s);
        let (wx, wy, wz) = (q_w * xs, q_w * ys, q_w * zs);
        let (xx, xy, xz) = (q_x * xs, q_x * ys, q_x * zs);
        let (yy, yz, zz) = (q_y * ys, q_y * zs, q_z * zs);
        Self {
            rot_matrix: [
                [1.0 - (yy + zz), xy - wz, xz + wy],
                [xy + wz, 1.0 - (xx + zz), yz - wx],
                [xz - wy, yz + wx, 1.0 - (xx + yy)],
            ],
            pos: [pos.x, pos.y, pos.z],
        }
    }
    pub const fn new_rot_matrix(
        pos: (f64, f64, f64),
        matrix: ((f64, f64, f64), (f64, f64, f64), (f64, f64, f64)),
    ) -> Self {
        CFrame {
            rot_matrix: [
                [matrix.0 .0, matrix.0 .1, matrix.0 .2],
                [matrix.1 .0, matrix.1 .1, matrix.1 .2],
                [matrix.2 .0, matrix.2 .1, matrix.2 .2],
            ],
            pos: [pos.0, pos.1, pos.2],
        }
    }
    pub const fn from_matrix(pos: Vector3, r0: Vector3, r1: Vector3, r2: Vector3) -> Self {
        CFrame {
            rot_matrix: [[r0.x, r0.y, r0.z], [r1.x, r1.y, r1.z], [r2.x, r2.y, r2.z]],
            pos: [pos.x, pos.y, pos.z],
        }
    }
    pub fn look_at(pos: Vector3, look_at: Vector3, up: Option<Vector3>) -> Self {
        Self::look_along(pos, (look_at - pos).get_unit(), up)
    }
    pub fn look_along(pos: Vector3, direction: Vector3, up: Option<Vector3>) -> Self {
        let up = up.unwrap_or(Vector3::Y_AXIS);
        let v_z = direction.get_unit();
        let v_x = up.cross(v_z).get_unit();
        let v_y = v_z.cross(v_x);
        Self::from_matrix(pos, v_x, v_y, v_z)
    }
    pub fn from_rotation_between_vectors(from: Vector3, to: Vector3) -> Self {
        let dot = from.dot(to);
        if dot > 0.99999 {
            return CFrame::IDENTITY;
        } else if dot < -0.99999 {
            let mut axis = from.cross(Vector3::X_AXIS);
            if axis.get_magnitude_squared() < 0.00001 {
                axis = from.cross(Vector3::Y_AXIS);
            }
            axis = axis.get_unit();
            return CFrame::from_axis_angle(axis, std::f64::consts::PI);
        }
        let axis = from.cross(to).get_unit();
        let angle = from.get_angle(to, None);
        CFrame::from_axis_angle(axis, angle)
    }
    pub fn from_euler_angles(rx: f64, ry: f64, rz: f64, order: Option<RotationOrder>) -> Self {
        let (sx, cx) = (rx.sin(), rx.cos());
        let (sy, cy) = (ry.sin(), ry.cos());
        let (sz, cz) = (rz.sin(), rz.cos());

        let rot_matrix = match order.unwrap_or(RotationOrder::XYZ) {
            RotationOrder::XYZ => [
                [cy * cz, -cy * sz, sy],
                [cx * sz + cz * sx * sy, cx * cz - sx * sy * sz, -cy * sx],
                [sx * sz - cx * cz * sy, cz * sx + cx * sy * sz, cx * cy],
            ],
            RotationOrder::XZY => [
                [cy * cz, -sz, cz * sy],
                [sx * sy + cx * cy * sz, cx * cz, cx * sy * sz - cy * sx],
                [cy * sx * sz - cx * sy, cz * sx, cx * cy + sx * sy * sz],
            ],
            RotationOrder::YXZ => [
                [cy * cz + sx * sy * sz, cz * sx * sy - cy * sz, cx * sy],
                [cx * sz, cx * cz, -sx],
                [cy * sx * sz - cz * sy, sy * sz + cy * cz * sx, cx * cy],
            ],
            RotationOrder::YZX => [
                [cy * cz, sx * sy - cx * cy * sz, cx * sy + cy * sx * sz],
                [sz, cx * cz, -cz * sx],
                [-cz * sy, cy * sx + cx * sy * sz, cx * cy - sx * sy * sz],
            ],
            RotationOrder::ZXY => [
                [cy * cz - sx * sy * sz, -cx * sz, cz * sy + cy * sx * sz],
                [cz * sx * sy + cy * sz, cx * cz, sy * sz - cy * cz * sx],
                [-cx * sy, sx, cx * cy],
            ],
            RotationOrder::ZYX => [
                [cy * cz, -cx * cy * sz + sx * sy, sx * sz + cx * sy * cz],
                [sz, cx * cz, -cz * sx],
                [-sy * cz, cx * sy * sz + sx * cy, cx * cy - sx * sy * sz],
            ],
        };

        Self {
            rot_matrix,
            pos: [0.0, 0.0, 0.0],
        }
    }
    #[inline]
    pub fn from_euler_angles_xyz(rx: f64, ry: f64, rz: f64) -> Self {
        Self::from_euler_angles(rx, ry, rz, Some(RotationOrder::XYZ))
    }
    #[inline]
    pub fn from_euler_angles_yxz(rx: f64, ry: f64, rz: f64) -> Self {
        Self::from_euler_angles(rx, ry, rz, Some(RotationOrder::YXZ))
    }
    #[inline]
    pub fn from_angles(rx: f64, ry: f64, rz: f64) -> Self {
        Self::from_euler_angles_xyz(rx, ry, rz)
    }
    #[inline]
    pub fn from_orientation(rx: f64, ry: f64, rz: f64) -> Self {
        Self::from_euler_angles_yxz(rx, ry, rz)
    }
    pub fn from_axis_angle(axis: Vector3, angle: f64) -> Self {
        let axis = axis.get_unit();
        let cos_angle = angle.cos();
        let sin_angle = angle.sin();
        let one_minus_cos = 1.0 - cos_angle;

        let (x, y, z) = (axis.x, axis.y, axis.z);
        let (xx, yy, zz) = (x * x, y * y, z * z);
        let (xy, yz, xz) = (x * y, y * z, x * z);

        Self {
            rot_matrix: [
                [
                    xx * one_minus_cos + cos_angle,
                    xy * one_minus_cos - z * sin_angle,
                    xz * one_minus_cos + y * sin_angle,
                ],
                [
                    xy * one_minus_cos + z * sin_angle,
                    yy * one_minus_cos + cos_angle,
                    yz * one_minus_cos - x * sin_angle,
                ],
                [
                    xz * one_minus_cos - y * sin_angle,
                    yz * one_minus_cos + x * sin_angle,
                    zz * one_minus_cos + cos_angle,
                ],
            ],
            pos: [0.0, 0.0, 0.0],
        }
    }
    pub const fn rotation_only(&self) -> Self {
        Self {
            rot_matrix: self.rot_matrix,
            pos: [0.0, 0.0, 0.0],
        }
    }
    pub const fn look_vector(&self) -> Vector3 {
        let (x, y, z) = (
            self.rot_matrix[2][0],
            self.rot_matrix[2][1],
            self.rot_matrix[2][2],
        );
        Vector3::new(-x, -y, -z)
    }
    pub const fn right_vector(&self) -> Vector3 {
        Vector3::new(
            self.rot_matrix[0][0],
            self.rot_matrix[0][1],
            self.rot_matrix[0][2],
        )
    }
    pub const fn up_vector(&self) -> Vector3 {
        Vector3::new(
            self.rot_matrix[1][0],
            self.rot_matrix[1][1],
            self.rot_matrix[1][2],
        )
    }
    pub const fn x_vector(&self) -> Vector3 {
        Vector3::new(
            self.rot_matrix[0][0],
            self.rot_matrix[0][1],
            self.rot_matrix[0][2],
        )
    }
    pub const fn y_vector(&self) -> Vector3 {
        Vector3::new(
            self.rot_matrix[1][0],
            self.rot_matrix[1][1],
            self.rot_matrix[1][2],
        )
    }
    pub const fn z_vector(&self) -> Vector3 {
        Vector3::new(
            self.rot_matrix[2][0],
            self.rot_matrix[2][1],
            self.rot_matrix[2][2],
        )
    }
    pub const fn inverse(&self) -> Self {
        let mut new_rot = [[0.0; 3]; 3];
        // Transpose the rotation matrix to get its inverse
        new_rot[0][0] = self.rot_matrix[0][0];
        new_rot[0][1] = self.rot_matrix[1][0];
        new_rot[0][2] = self.rot_matrix[2][0];
        new_rot[1][0] = self.rot_matrix[0][1];
        new_rot[1][1] = self.rot_matrix[1][1];
        new_rot[1][2] = self.rot_matrix[2][1];
        new_rot[2][0] = self.rot_matrix[0][2];
        new_rot[2][1] = self.rot_matrix[1][2];
        new_rot[2][2] = self.rot_matrix[2][2];
        // Calculate the inverse position
        let new_pos = [
            -(self.pos[0] * new_rot[0][0]
                + self.pos[1] * new_rot[0][1]
                + self.pos[2] * new_rot[0][2]),
            -(self.pos[0] * new_rot[1][0]
                + self.pos[1] * new_rot[1][1]
                + self.pos[2] * new_rot[1][2]),
            -(self.pos[0] * new_rot[2][0]
                + self.pos[1] * new_rot[2][1]
                + self.pos[2] * new_rot[2][2]),
        ];
        Self {
            rot_matrix: new_rot,
            pos: new_pos,
        }
    }
    pub const fn lerp(&self, goal: Self, alpha: f64) -> Self {
        CFrame {
            rot_matrix: [
                [
                    self.rot_matrix[0][0] * (1.0 - alpha) + goal.rot_matrix[0][0] * alpha,
                    self.rot_matrix[0][1] * (1.0 - alpha) + goal.rot_matrix[0][1] * alpha,
                    self.rot_matrix[0][2] * (1.0 - alpha) + goal.rot_matrix[0][2] * alpha,
                ],
                [
                    self.rot_matrix[1][0] * (1.0 - alpha) + goal.rot_matrix[1][0] * alpha,
                    self.rot_matrix[1][1] * (1.0 - alpha) + goal.rot_matrix[1][1] * alpha,
                    self.rot_matrix[1][2] * (1.0 - alpha) + goal.rot_matrix[1][2] * alpha,
                ],
                [
                    self.rot_matrix[2][0] * (1.0 - alpha) + goal.rot_matrix[2][0] * alpha,
                    self.rot_matrix[2][1] * (1.0 - alpha) + goal.rot_matrix[2][1] * alpha,
                    self.rot_matrix[2][2] * (1.0 - alpha) + goal.rot_matrix[2][2] * alpha,
                ],
            ],
            pos: [
                self.pos[0] * (1.0 - alpha) + goal.pos[0] * alpha,
                self.pos[1] * (1.0 - alpha) + goal.pos[1] * alpha,
                self.pos[2] * (1.0 - alpha) + goal.pos[2] * alpha,
            ],
        }
    }
    pub fn orthonormalize(&self) -> Self {
        let x = self.x_vector().get_unit();
        let z = x.cross(self.y_vector()).get_unit();
        let y = z.cross(x).get_unit();

        Self::from_matrix(Vector3::new(self.pos[0], self.pos[1], self.pos[2]), x, y, z)
    }
    pub fn to_world_space(&self, cframe: Self) -> Self {
        let mut result = self.clone();
        result.pos[0] += cframe.pos[0];
        result.pos[1] += cframe.pos[1];
        result.pos[2] += cframe.pos[2];
        let mut new_rot = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                new_rot[i][j] = 0.0;
                for k in 0..3 {
                    new_rot[i][j] += self.rot_matrix[i][k] * cframe.rot_matrix[k][j];
                }
            }
        }
        result.rot_matrix = new_rot;
        result
    }
    pub fn to_object_space(&self, cframe: Self) -> Self {
        self.to_world_space(cframe.inverse())
    }
    pub fn to_world_space_multiple(&self, cframes: &[Self]) -> Vec<Self> {
        let mut result = Vec::with_capacity(cframes.len());
        for cframe in cframes {
            result.push(self.to_world_space(*cframe));
        }
        result
    }
    pub fn to_object_space_multiple(&self, cframes: &[Self]) -> Vec<Self> {
        let mut result = Vec::with_capacity(cframes.len());
        for cframe in cframes {
            result.push(self.to_object_space(*cframe));
        }
        result
    }
    pub const fn point_to_world_space(&self, point: Vector3) -> Vector3 {
        let x = self.x_vector();
        let y = self.y_vector();
        let z = self.z_vector();
        Vector3::new(
            self.pos[0] + x.x * point.x + y.x * point.y + z.x * point.z,
            self.pos[1] + x.y * point.x + y.y * point.y + z.y * point.z,
            self.pos[2] + x.z * point.x + y.z * point.y + z.z * point.z,
        )
    }
    pub const fn point_to_object_space(&self, point: Vector3) -> Vector3 {
        let x = self.x_vector();
        let y = self.y_vector();
        let z = self.z_vector();
        Vector3::new(
            x.x * (point.x - self.pos[0])
                + x.y * (point.y - self.pos[1])
                + x.z * (point.z - self.pos[2]),
            y.x * (point.x - self.pos[0])
                + y.y * (point.y - self.pos[1])
                + y.z * (point.z - self.pos[2]),
            z.x * (point.x - self.pos[0])
                + z.y * (point.y - self.pos[1])
                + z.z * (point.z - self.pos[2]),
        )
    }
    pub fn points_to_world_space(&self, points: &[Vector3]) -> Vec<Vector3> {
        let mut result = Vec::with_capacity(points.len());
        for point in points {
            result.push(self.point_to_world_space(*point));
        }
        result
    }
    pub fn points_to_object_space(&self, points: &[Vector3]) -> Vec<Vector3> {
        let mut result = Vec::with_capacity(points.len());
        for point in points {
            result.push(self.point_to_object_space(*point));
        }
        result
    }
    pub const fn vector_to_world_space(&self, vector: Vector3) -> Vector3 {
        let x = self.x_vector();
        let y = self.y_vector();
        let z = self.z_vector();
        Vector3::new(
            x.x * vector.x + y.x * vector.y + z.x * vector.z,
            x.y * vector.x + y.y * vector.y + z.y * vector.z,
            x.z * vector.x + y.z * vector.y + z.z * vector.z,
        )
    }
    pub const fn vector_to_object_space(&self, vector: Vector3) -> Vector3 {
        let x = self.x_vector();
        let y = self.y_vector();
        let z = self.z_vector();
        Vector3::new(
            x.x * vector.x + x.y * vector.y + x.z * vector.z,
            y.x * vector.x + y.y * vector.y + y.z * vector.z,
            z.x * vector.x + z.y * vector.y + z.z * vector.z,
        )
    }
    pub fn vectors_to_world_space(&self, vectors: &[Vector3]) -> Vec<Vector3> {
        let mut result = Vec::with_capacity(vectors.len());
        for vector in vectors {
            result.push(self.vector_to_world_space(*vector));
        }
        result
    }
    pub fn vectors_to_object_space(&self, vectors: &[Vector3]) -> Vec<Vector3> {
        let mut result = Vec::with_capacity(vectors.len());
        for vector in vectors {
            result.push(self.vector_to_object_space(*vector));
        }
        result
    }
    pub fn to_euler_angles(&self, order: Option<RotationOrder>) -> [f64; 3] {
        let rot_matrix = self.rot_matrix;
        let sy = (rot_matrix[0][0] * rot_matrix[0][0] + rot_matrix[1][0] * rot_matrix[1][0]).sqrt();
        let singular = sy < 1e-6;
        let x;
        let y;
        let z;
        if !singular {
            x = rot_matrix[2][1].atan2(rot_matrix[2][2]);
            y = -rot_matrix[2][0].atan2(sy);
            z = rot_matrix[1][0].atan2(rot_matrix[0][0]);
        } else {
            x = rot_matrix[1][2].atan2(rot_matrix[1][1]);
            y = -rot_matrix[2][0].atan2(sy);
            z = 0.0;
        }
        if let Some(order) = order {
            match order {
                RotationOrder::XYZ => [x, y, z],
                RotationOrder::XZY => [x, z, y],
                RotationOrder::YXZ => [y, x, z],
                RotationOrder::YZX => [y, z, x],
                RotationOrder::ZXY => [z, x, y],
                RotationOrder::ZYX => [z, y, x],
            }
        } else {
            [x, y, z]
        }
    }
    #[inline]
    pub fn to_euler_angles_xyz(&self) -> [f64; 3] {
        self.to_euler_angles(Some(RotationOrder::XYZ))
    }
    #[inline]
    pub fn to_euler_angles_yxz(&self) -> [f64; 3] {
        self.to_euler_angles(Some(RotationOrder::YXZ))
    }
    #[inline]
    pub fn to_orientation(&self) -> [f64; 3] {
        self.to_euler_angles_yxz()
    }
    pub fn to_axis_angle(&self) -> (Vector3, f64) {
        let trace = self.rot_matrix[0][0] + self.rot_matrix[1][1] + self.rot_matrix[2][2];
        let cos_angle = (trace - 1.0) / 2.0;
        let angle = cos_angle.acos();

        if angle.abs() < 1e-6 {
            (Vector3::new(1.0, 0.0, 0.0), 0.0)
        } else if (cos_angle + 1.0).abs() < 1e-6 {
            // Angle is PI, need to find axis differently
            let mut axis = Vector3::new(
                (self.rot_matrix[0][0] + 1.0).sqrt(),
                (self.rot_matrix[1][1] + 1.0).sqrt(),
                (self.rot_matrix[2][2] + 1.0).sqrt(),
            );
            if axis.x > axis.y && axis.x > axis.z {
                axis.y = self.rot_matrix[0][1] / axis.x;
                axis.z = self.rot_matrix[0][2] / axis.x;
            } else if axis.y > axis.z {
                axis.x = self.rot_matrix[0][1] / axis.y;
                axis.z = self.rot_matrix[1][2] / axis.y;
            } else {
                axis.x = self.rot_matrix[0][2] / axis.z;
                axis.y = self.rot_matrix[1][2] / axis.z;
            }
            (axis.get_unit(), std::f64::consts::PI)
        } else {
            let axis = Vector3::new(
                self.rot_matrix[2][1] - self.rot_matrix[1][2],
                self.rot_matrix[0][2] - self.rot_matrix[2][0],
                self.rot_matrix[1][0] - self.rot_matrix[0][1],
            );
            (axis.get_unit(), angle)
        }
    }
    pub fn fuzzy_eq(&self, other: Self, epsilon: f64) -> bool {
        let mut equal = true;
        for i in 0..3 {
            for j in 0..3 {
                equal &= (self.rot_matrix[i][j] - other.rot_matrix[i][j]).abs() < epsilon;
            }
        }
        for i in 0..3 {
            equal &= (self.pos[i] - other.pos[i]).abs() < epsilon;
        }
        equal
    }
}

impl Mul for CFrame {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        let mut new_rot = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                new_rot[i][j] = 0.0;
                for k in 0..3 {
                    new_rot[i][j] += self.rot_matrix[i][k] * rhs.rot_matrix[k][j];
                }
            }
        }
        let new_pos = [
            self.pos[0]
                + self.rot_matrix[0][0] * rhs.pos[0]
                + self.rot_matrix[0][1] * rhs.pos[1]
                + self.rot_matrix[0][2] * rhs.pos[2],
            self.pos[1]
                + self.rot_matrix[1][0] * rhs.pos[0]
                + self.rot_matrix[1][1] * rhs.pos[1]
                + self.rot_matrix[1][2] * rhs.pos[2],
            self.pos[2]
                + self.rot_matrix[2][0] * rhs.pos[0]
                + self.rot_matrix[2][1] * rhs.pos[1]
                + self.rot_matrix[2][2] * rhs.pos[2],
        ];
        Self {
            rot_matrix: new_rot,
            pos: new_pos,
        }
    }
}
impl Mul<Vector3> for CFrame {
    type Output = Vector3;

    fn mul(self, rhs: Vector3) -> Vector3 {
        Vector3::new(
            self.pos[0]
                + self.rot_matrix[0][0] * rhs.x
                + self.rot_matrix[0][1] * rhs.y
                + self.rot_matrix[0][2] * rhs.z,
            self.pos[1]
                + self.rot_matrix[1][0] * rhs.x
                + self.rot_matrix[1][1] * rhs.y
                + self.rot_matrix[1][2] * rhs.z,
            self.pos[2]
                + self.rot_matrix[2][0] * rhs.x
                + self.rot_matrix[2][1] * rhs.y
                + self.rot_matrix[2][2] * rhs.z,
        )
    }
}
impl Add<Vector3> for CFrame {
    type Output = Self;

    fn add(self, rhs: Vector3) -> Self {
        Self {
            rot_matrix: self.rot_matrix,
            pos: [
                self.pos[0] + rhs.x,
                self.pos[1] + rhs.y,
                self.pos[2] + rhs.z,
            ],
        }
    }
}
impl Sub<Vector3> for CFrame {
    type Output = Self;

    fn sub(self, rhs: Vector3) -> Self {
        Self {
            rot_matrix: self.rot_matrix,
            pos: [
                self.pos[0] - rhs.x,
                self.pos[1] - rhs.y,
                self.pos[2] - rhs.z,
            ],
        }
    }
}

impl LuaUserData for CFrame {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("Position", |_, this| Ok(Vector3::from(this.pos)));
        fields.add_field_method_get("Rotation", |_, this| Ok(this.rotation_only()));
        fields.add_field_method_get("X", |_, this| Ok(this.pos[0]));
        fields.add_field_method_get("Y", |_, this| Ok(this.pos[1]));
        fields.add_field_method_get("Z", |_, this| Ok(this.pos[2]));
        fields.add_field_method_get("LookVector", |_, this| Ok(this.look_vector()));
        fields.add_field_method_get("RightVector", |_, this| Ok(this.right_vector()));
        fields.add_field_method_get("UpVector", |_, this| Ok(this.up_vector()));
        fields.add_field_method_get("XVector", |_, this| Ok(this.x_vector()));
        fields.add_field_method_get("YVector", |_, this| Ok(this.y_vector()));
        fields.add_field_method_get("ZVector", |_, this| Ok(this.z_vector()));
    }
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("Inverse", |_, this, ()| Ok(this.inverse()));
        methods.add_method("Lerp", |_, this, (goal, alpha): (CFrame, f64)| {
            Ok(this.lerp(goal, alpha))
        });
        methods.add_method("Orthonormalize", |_, this, ()| Ok(this.orthonormalize()));
        methods.add_method("ToWorldSpace", |_, this, cframe: CFrame| {
            Ok(this.to_world_space(cframe))
        });
        methods.add_method("ToObjectSpace", |_, this, cframe: CFrame| {
            Ok(this.to_object_space(cframe))
        });
        methods.add_method("ToWorldSpace", |_, this, cframes: Vec<CFrame>| {
            Ok(this.to_world_space_multiple(&cframes))
        });
        methods.add_method("ToObjectSpace", |_, this, cframes: Vec<CFrame>| {
            Ok(this.to_object_space_multiple(&cframes))
        });
        methods.add_method("PointToWorldSpace", |_, this, points: Vec<Vector3>| {
            Ok(this.points_to_world_space(&points))
        });
        methods.add_method("PointToObjectSpace", |_, this, points: Vec<Vector3>| {
            Ok(this.points_to_object_space(&points))
        });
        methods.add_method("VectorToWorldSpace", |_, this, vectors: Vec<Vector3>| {
            Ok(this.vectors_to_world_space(&vectors))
        });
        methods.add_method("VectorToObjectSpace", |_, this, vectors: Vec<Vector3>| {
            Ok(this.vectors_to_object_space(&vectors))
        });
        methods.add_method("components", |_, this, ()| {
            Ok((
                this.pos[0],
                this.pos[1],
                this.pos[2],
                this.rot_matrix[0][0],
                this.rot_matrix[0][1],
                this.rot_matrix[0][2],
                this.rot_matrix[1][0],
                this.rot_matrix[1][1],
                this.rot_matrix[1][2],
                this.rot_matrix[2][0],
                this.rot_matrix[2][1],
                this.rot_matrix[2][2],
            ))
        });
        methods.add_method("ToEulerAngles", |_, this, order: Option<RotationOrder>| {
            let angles = this.to_euler_angles(order);
            Ok((angles[0], angles[1], angles[2]))
        });
        methods.add_method("ToEulerAnglesXYZ", |_, this, ()| {
            let angles = this.to_euler_angles_xyz();
            Ok((angles[0], angles[1], angles[2]))
        });
        methods.add_method("ToEulerAnglesYXZ", |_, this, ()| {
            let angles = this.to_euler_angles_yxz();
            Ok((angles[0], angles[1], angles[2]))
        });
        methods.add_method("ToOrientation", |_, this, ()| {
            let angles = this.to_orientation();
            Ok((angles[0], angles[1], angles[2]))
        });
        methods.add_method("ToAxisAngle", |_, this, ()| Ok(this.to_axis_angle()));
        methods.add_method("FuzzyEq", |_, this, (other, epsilon): (CFrame, f64)| {
            Ok(this.fuzzy_eq(other, epsilon))
        });
        methods.add_meta_method("__eq", |_, this, other: CFrame| Ok(*this == other));
        methods.add_meta_method("__mul", |lua, this, rhs: LuaValue| match rhs {
            LuaValue::UserData(ud) => {
                if let Ok(vector) = ud.borrow::<Vector3>() {
                    (*this * *vector).into_lua(lua)
                } else if let Ok(cframe) = ud.borrow::<CFrame>() {
                    (*this * *cframe).into_lua(lua)
                } else {
                    Err(LuaError::BadArgument {
                        to: Some("CFrame::__mul".into()),
                        pos: 2,
                        name: Some("other".into()),
                        cause: Arc::new(LuaError::FromLuaConversionError {
                            from: "userdata",
                            to: "Vector3 or CFrame".into(),
                            message: None,
                        }),
                    })
                }
            }
            _ => Err(LuaError::BadArgument {
                to: Some("CFrame::__mul".into()),
                pos: 2,
                name: Some("other".into()),
                cause: Arc::new(LuaError::FromLuaConversionError {
                    from: rhs.type_name(),
                    to: "Vector3 or CFrame".into(),
                    message: None,
                }),
            }),
        });
        methods.add_meta_method("__add", |_, this, rhs: Vector3| Ok(*this + rhs));
        methods.add_meta_method("__sub", |_, this, rhs: Vector3| Ok(*this - rhs));
        methods.add_meta_method("__tostring", |_, this, ()| {
            Ok(format!(
                "({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {})",
                this.rot_matrix[0][0],
                this.rot_matrix[0][1],
                this.rot_matrix[0][2],
                this.rot_matrix[1][0],
                this.rot_matrix[1][1],
                this.rot_matrix[1][2],
                this.rot_matrix[2][0],
                this.rot_matrix[2][1],
                this.rot_matrix[2][2],
                this.pos[0],
                this.pos[1],
                this.pos[2]
            ))
        });
    }
}

impl LuaSingleton for CFrame {
    fn register_singleton(lua: &Lua) -> LuaResult<()> {
        let cframe = lua.create_table()?;
        cframe.raw_set(
            "new",
            lua.create_function(|lua, mut mv: LuaMultiValue| match mv.len() {
                0 => Ok(CFrame::new()),
                1 => Ok(CFrame::new_with_position(Vector3::from_lua(
                    take(&mut mv[0]),
                    lua,
                )?)),
                2 => Ok(CFrame::new_looking_at(
                    Vector3::from_lua(take(&mut mv[0]), lua)?,
                    Vector3::from_lua(take(&mut mv[1]), lua)?,
                )),
                3 => Ok(CFrame::new_with_position(Vector3::new(
                    f64::from_lua(take(&mut mv[0]), lua)?,
                    f64::from_lua(take(&mut mv[1]), lua)?,
                    f64::from_lua(take(&mut mv[2]), lua)?,
                ))),
                7 => Ok(CFrame::new_quaternion(
                    Vector3::new(
                        f64::from_lua(take(&mut mv[0]), lua)?,
                        f64::from_lua(take(&mut mv[1]), lua)?,
                        f64::from_lua(take(&mut mv[2]), lua)?,
                    ),
                    f64::from_lua(take(&mut mv[3]), lua)?,
                    f64::from_lua(take(&mut mv[4]), lua)?,
                    f64::from_lua(take(&mut mv[5]), lua)?,
                    f64::from_lua(take(&mut mv[6]), lua)?,
                )),
                12 => Ok(CFrame::new_rot_matrix(
                    (
                        f64::from_lua(take(&mut mv[0]), lua)?,
                        f64::from_lua(take(&mut mv[1]), lua)?,
                        f64::from_lua(take(&mut mv[2]), lua)?,
                    ),
                    (
                        (
                            f64::from_lua(take(&mut mv[3]), lua)?,
                            f64::from_lua(take(&mut mv[4]), lua)?,
                            f64::from_lua(take(&mut mv[5]), lua)?,
                        ),
                        (
                            f64::from_lua(take(&mut mv[6]), lua)?,
                            f64::from_lua(take(&mut mv[7]), lua)?,
                            f64::from_lua(take(&mut mv[8]), lua)?,
                        ),
                        (
                            f64::from_lua(take(&mut mv[9]), lua)?,
                            f64::from_lua(take(&mut mv[10]), lua)?,
                            f64::from_lua(take(&mut mv[11]), lua)?,
                        ),
                    ),
                )),
                _ => Err(LuaError::RuntimeError(format!(
                    "expected 0, 1, 2, 3, 7, or 12 arguments, got {}",
                    mv.len()
                ))),
            })?,
        )?;
        cframe.raw_set(
            "lookAt",
            lua.create_function(
                |_, (at, look_at, up): (Vector3, Vector3, Option<Vector3>)| {
                    Ok(CFrame::look_at(at, look_at, up))
                },
            )?,
        )?;
        cframe.raw_set(
            "lookAlong",
            lua.create_function(
                |_, (at, direction, up): (Vector3, Vector3, Option<Vector3>)| {
                    Ok(CFrame::look_along(at, direction, up))
                },
            )?,
        )?;
        cframe.raw_set(
            "fromRotationBetweenVectors",
            lua.create_function(|_, (from, to): (Vector3, Vector3)| {
                Ok(CFrame::from_rotation_between_vectors(from, to))
            })?,
        )?;
        cframe.raw_set(
            "fromEulerAngles",
            lua.create_function(
                |_, (rx, ry, rz, order): (f64, f64, f64, Option<RotationOrder>)| {
                    Ok(CFrame::from_euler_angles(rx, ry, rz, order))
                },
            )?,
        )?;
        cframe.raw_set(
            "fromEulerAnglesXYZ",
            lua.create_function(|_, (rx, ry, rz): (f64, f64, f64)| {
                Ok(CFrame::from_euler_angles_xyz(rx, ry, rz))
            })?,
        )?;
        cframe.raw_set(
            "fromEulerAnglesYXZ",
            lua.create_function(|_, (rx, ry, rz): (f64, f64, f64)| {
                Ok(CFrame::from_euler_angles_yxz(rx, ry, rz))
            })?,
        )?;
        cframe.raw_set(
            "Angles",
            lua.create_function(|_, (rx, ry, rz): (f64, f64, f64)| {
                Ok(CFrame::from_angles(rx, ry, rz))
            })?,
        )?;
        cframe.raw_set(
            "fromOrientation",
            lua.create_function(|_, (rx, ry, rz): (f64, f64, f64)| {
                Ok(CFrame::from_orientation(rx, ry, rz))
            })?,
        )?;
        cframe.raw_set(
            "fromAxisAngle",
            lua.create_function(|_, (axis, angle): (Vector3, f64)| {
                Ok(CFrame::from_axis_angle(axis, angle))
            })?,
        )?;
        cframe.raw_set(
            "fromMatrix",
            lua.create_function(
                |_, (pos, r0, r1, r2): (Vector3, Vector3, Vector3, Vector3)| {
                    Ok(CFrame::from_matrix(pos, r0, r1, r2))
                },
            )?,
        )?;
        cframe.raw_set("identity", CFrame::IDENTITY)?;

        lua.globals().set("CFrame", cframe)?;
        Ok(())
    }
}
