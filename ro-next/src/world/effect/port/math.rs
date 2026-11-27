//! Exact port of `Source/Utility/Math/AffineMatrix.cpp` (the subset the
//! primitives use).
//!
//! The client's `matrix` is 12 floats, row-major 3x4: rows `0..3`, `3..6`,
//! `6..9` are the rotation/scale part and `9..12` the translation. Rotations
//! are built with `ClientSin`/`ClientCos` (float `sinf`/`cosf`) and appended
//! with `Rotate`, whose row-major multiply order is preserved here.

use super::angles::{client_cos, client_sin};

/// `matrix` - 12 floats, row-major 3x4.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix {
    pub values: [f32; 12],
}

/// `g_identityMatrix`.
pub const IDENTITY: [f32; 12] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0];

/// `g_depthAverageHalf` - `MapSkyMgr.cpp:27`, the quaternion half-angle factor
/// `matrix::MakeTransform` uses.
const DEPTH_AVERAGE_HALF: f32 = 0.5;

/// `g_projectionDepthNumerator` - `RenderProjection.cpp:9`. `MakeTransform`
/// writes it on the quaternion matrix diagonal (the original uses it instead
/// of `1.0` when the projection depth changes).
const PROJECTION_DEPTH_NUMERATOR: f32 = 1.0;

impl Default for Matrix {
    fn default() -> Self {
        Self { values: IDENTITY }
    }
}

impl Matrix {
    /// `matrix::Identity()`.
    pub fn identity() -> Self {
        Self::default()
    }

    /// `matrix::MakeXRotation(float)` - `AffineMatrix.cpp:40-49`.
    pub fn make_x_rotation(&mut self, radians: f32) {
        self.values = IDENTITY;

        let sine = client_sin(radians);
        let cosine = client_cos(radians);
        self.values[4] = cosine;
        self.values[8] = cosine;
        self.values[5] = sine;
        self.values[7] = -sine;
    }

    /// `matrix::MakeYRotation(float)` - `AffineMatrix.cpp:53-61`.
    pub fn make_y_rotation(&mut self, radians: f32) {
        self.values = IDENTITY;

        let sine = client_sin(radians);
        let cosine = client_cos(radians);
        self.values[0] = cosine;
        self.values[8] = cosine;
        self.values[2] = -sine;
        self.values[6] = sine;
    }

    /// `matrix::MakeTranslate(float, float, float)` -
    /// `AffineMatrix.cpp:103-108`.
    pub fn make_translate(&mut self, x: f32, y: f32, z: f32) {
        self.values = IDENTITY;
        self.values[9] = x;
        self.values[10] = y;
        self.values[11] = z;
    }

    /// `matrix::SetScale(float, float, float)` - `AffineMatrix.cpp:112-117`.
    pub fn set_scale(&mut self, x: f32, y: f32, z: f32) {
        self.values = IDENTITY;
        self.values[0] = x;
        self.values[4] = y;
        self.values[8] = z;
    }

    /// `matrix::Compose(const matrix&, const matrix&)` -
    /// `AffineMatrix.cpp:137-150`.
    pub fn compose(&mut self, first: &Matrix, second: &Matrix) {
        self.values[0] = first.values[0] * second.values[0] + first.values[1] * second.values[3] + first.values[2] * second.values[6];
        self.values[3] = first.values[3] * second.values[0] + first.values[4] * second.values[3] + first.values[5] * second.values[6];
        self.values[6] = first.values[6] * second.values[0] + first.values[7] * second.values[3] + first.values[8] * second.values[6];
        self.values[9] = first.values[9] * second.values[0]
            + first.values[10] * second.values[3]
            + first.values[11] * second.values[6]
            + second.values[9];
        self.values[1] = first.values[0] * second.values[1] + first.values[1] * second.values[4] + first.values[2] * second.values[7];
        self.values[4] = first.values[3] * second.values[1] + first.values[4] * second.values[4] + first.values[5] * second.values[7];
        self.values[7] = first.values[6] * second.values[1] + first.values[7] * second.values[4] + first.values[8] * second.values[7];
        self.values[10] = first.values[9] * second.values[1]
            + first.values[10] * second.values[4]
            + first.values[11] * second.values[7]
            + second.values[10];
        self.values[2] = first.values[0] * second.values[2] + first.values[1] * second.values[5] + first.values[2] * second.values[8];
        self.values[5] = first.values[3] * second.values[2] + first.values[4] * second.values[5] + first.values[5] * second.values[8];
        self.values[8] = first.values[6] * second.values[2] + first.values[7] * second.values[5] + first.values[8] * second.values[8];
        self.values[11] = first.values[9] * second.values[2]
            + first.values[10] * second.values[5]
            + first.values[11] * second.values[8]
            + second.values[11];
    }

    /// `matrix::MakeZRotation(float)` - `AffineMatrix.cpp:64-72`.
    pub fn make_z_rotation(&mut self, radians: f32) {
        self.values = IDENTITY;

        let sine = client_sin(radians);
        let cosine = client_cos(radians);
        self.values[0] = cosine;
        self.values[4] = cosine;
        self.values[3] = -sine;
        self.values[1] = sine;
    }

    /// `matrix::AppendZRotation(float)` - `AffineMatrix.cpp:95-99`.
    pub fn append_z_rotation(&mut self, radians: f32) {
        let mut rotation = Matrix::identity();
        rotation.make_z_rotation(radians);
        self.rotate(&rotation.values);
    }

    /// `matrix::AppendYRotation(float)` - `AffineMatrix.cpp:87-91`.
    pub fn append_y_rotation(&mut self, radians: f32) {
        let mut rotation = Matrix::identity();
        rotation.make_y_rotation(radians);
        self.rotate(&rotation.values);
    }

    /// `matrix::AppendXRotation(float)`.
    pub fn append_x_rotation(&mut self, radians: f32) {
        let mut rotation = Matrix::identity();
        rotation.make_x_rotation(radians);
        self.rotate(&rotation.values);
    }

    /// `matrix::MakeTransform(const float* rotaxis, float rotangle)` -
    /// `AffineMatrix.cpp:237-264`.
    ///
    /// The axis-angle overload used by `PP_3DPARTICLESPLINE`: the rotation is
    /// built from the quaternion `(axis * sin(angle / 2), cos(angle / 2))`,
    /// with `g_projectionDepthNumerator` on the diagonal and the translation
    /// part zeroed (the original has a second overload that takes a position;
    /// this one does not).
    pub fn make_transform(&mut self, rot_axis: &[f32; 3], radians: f32) {
        let omega = radians * DEPTH_AVERAGE_HALF;
        // The original overload calls `sinf`/`cosf`, not the `ClientSin`
        // double-promoting helpers the rotation builders use.
        let sine = omega.sin();
        let cosine = omega.cos();
        let qx = sine * rot_axis[0];
        let qy = sine * rot_axis[1];
        let qz = sine * rot_axis[2];
        let qw = cosine;

        let x2 = qx + qx;
        let y2 = qy + qy;
        let z2 = qz + qz;
        let wx = qw * x2;
        let wy = qw * y2;
        let wz = qw * z2;
        let xx = qx * x2;
        let xy = qx * y2;
        let xz = qx * z2;
        let yy = qy * y2;
        let yz = qy * z2;
        let zz = qz * z2;

        let numerator = PROJECTION_DEPTH_NUMERATOR;
        self.values[0] = numerator - (yy + zz);
        self.values[1] = xy + wz;
        self.values[3] = xy - wz;
        self.values[2] = xz - wy;
        self.values[6] = xz + wy;
        self.values[4] = numerator - (xx + zz);
        self.values[5] = yz + wx;
        self.values[7] = yz - wx;
        self.values[8] = numerator - (xx + yy);
        self.values[9] = 0.0;
        self.values[10] = 0.0;
        self.values[11] = 0.0;
    }

    /// `Rotate(float* values, const float* rotation)` -
    /// `AffineMatrix.cpp:16-36`.
    fn rotate(&mut self, rotation: &[f32; 12]) {
        let mut x = [0.0f32; 4];
        let mut y = [0.0f32; 4];
        let mut z = [0.0f32; 4];

        for row in 0..4 {
            x[row] = self.values[row * 3];
            y[row] = self.values[row * 3 + 1];
            z[row] = self.values[row * 3 + 2];
        }

        for row in 0..4 {
            self.values[row * 3] = (x[row] * rotation[0] + y[row] * rotation[3]) + z[row] * rotation[6];
            self.values[row * 3 + 1] = (x[row] * rotation[1] + y[row] * rotation[4]) + z[row] * rotation[7];
            self.values[row * 3 + 2] = (x[row] * rotation[2] + y[row] * rotation[5]) + z[row] * rotation[8];
        }

        self.values[9] += rotation[9];
        self.values[10] += rotation[10];
        self.values[11] += rotation[11];
    }
}

/// `Forward(vector3d&, float, const float*)` -
/// `EffectPrimitiveMotion.cpp:76-81`.
pub fn forward(distance: f32, matrix: &[f32; 12]) -> [f32; 3] {
    let point = [0.0f32, 0.0f32, distance];

    [
        ((matrix[3] * point[1] + matrix[0] * point[0]) + point[2] * matrix[6]) + matrix[9],
        ((matrix[4] * point[1] + matrix[1] * point[0]) + point[2] * matrix[7]) + matrix[10],
        ((matrix[5] * point[1] + matrix[2] * point[0]) + point[2] * matrix[8]) + matrix[11],
    ]
}

/// `Upward(vector3d&, float, const float*)` -
/// `EffectPrimitiveMotion.cpp:84-89`.
pub fn upward(distance: f32, matrix: &[f32; 12]) -> [f32; 3] {
    let point = [0.0f32, -distance, 0.0f32];

    [
        ((matrix[3] * point[1] + matrix[0] * point[0]) + point[2] * matrix[6]) + matrix[9],
        ((matrix[4] * point[1] + matrix[1] * point[0]) + point[2] * matrix[7]) + matrix[10],
        ((matrix[5] * point[1] + matrix[2] * point[0]) + point[2] * matrix[8]) + matrix[11],
    ]
}

/// `Lift(vector3d&, float, const float*)` - `EffectPrimitiveMotion.cpp:92-97`.
pub fn lift(distance: f32, matrix: &[f32; 12]) -> [f32; 3] {
    let point = [0.0f32, distance, 0.0f32];

    [
        ((matrix[3] * point[1] + matrix[0] * point[0]) + point[2] * matrix[6]) + matrix[9],
        ((matrix[4] * point[1] + matrix[1] * point[0]) + point[2] * matrix[7]) + matrix[10],
        ((matrix[5] * point[1] + matrix[2] * point[0]) + point[2] * matrix[8]) + matrix[11],
    ]
}

/// `MatrixMult(vector3d&, const vector3d&, const float*)` -
/// `RagEffect.cpp:145-149`.
pub fn matrix_mult(v: [f32; 3], m: &[f32; 12]) -> [f32; 3] {
    [
        v[0] * m[0] + v[1] * m[3] + v[2] * m[6] + m[9],
        v[0] * m[1] + v[1] * m[4] + v[2] * m[7] + m[10],
        v[0] * m[2] + v[1] * m[5] + v[2] * m[8] + m[11],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;

    #[test]
    fn identity_multiplies_vectors_unchanged() {
        let matrix = Matrix::identity();
        let result = matrix_mult([3.0, 4.0, 5.0], &matrix.values);

        assert!((result[0] - 3.0).abs() < EPSILON);
        assert!((result[1] - 4.0).abs() < EPSILON);
        assert!((result[2] - 5.0).abs() < EPSILON);
    }

    #[test]
    fn x_rotation_maps_forward_to_negative_y() {
        let mut matrix = Matrix::identity();
        matrix.make_x_rotation(std::f32::consts::FRAC_PI_2);

        // Forward(1) = (0, -sin, cos) for a 90 degree X rotation.
        let result = forward(1.0, &matrix.values);

        assert!(result[0].abs() < EPSILON);
        assert!((result[1] + 1.0).abs() < EPSILON);
        assert!(result[2].abs() < EPSILON);
    }

    #[test]
    fn y_rotation_turns_forward_around_y() {
        let mut matrix = Matrix::identity();
        matrix.make_y_rotation(std::f32::consts::FRAC_PI_2);

        let result = forward(1.0, &matrix.values);

        // MakeYRotation maps the +z axis onto +x.
        assert!((result[0] - 1.0).abs() < EPSILON);
        assert!(result[1].abs() < EPSILON);
        assert!(result[2].abs() < EPSILON);
    }

    #[test]
    fn append_y_rotation_composes_in_order() {
        let mut matrix = Matrix::identity();
        matrix.make_x_rotation(std::f32::consts::FRAC_PI_2);
        matrix.append_y_rotation(0.0);

        let result = forward(1.0, &matrix.values);

        assert!(result[0].abs() < EPSILON);
        assert!((result[1] + 1.0).abs() < EPSILON);
        assert!(result[2].abs() < EPSILON);
    }

    #[test]
    fn make_transform_builds_an_axis_angle_rotation() {
        let mut matrix = Matrix::identity();
        matrix.make_transform(&[0.0, 0.0, 1.0], std::f32::consts::FRAC_PI_2);

        // A +90 degree Z rotation maps +x onto +y; the translation column is
        // zero, unlike the position-taking overload.
        assert!(matrix.values[0].abs() < EPSILON);
        assert!((matrix.values[1] - 1.0).abs() < EPSILON);
        assert!((matrix.values[3] + 1.0).abs() < EPSILON);
        assert!((matrix.values[4]).abs() < EPSILON);
        assert_eq!(matrix.values[9..], [0.0, 0.0, 0.0]);
    }

    #[test]
    fn translate_and_scale_only_touch_their_official_cells() {
        let mut translate = Matrix::identity();
        translate.make_translate(3.0, 4.0, 5.0);
        assert_eq!(translate.values[9..], [3.0, 4.0, 5.0]);
        assert!(matrix_mult([0.0, 0.0, 0.0], &translate.values) == [3.0, 4.0, 5.0]);

        let mut scale = Matrix::identity();
        scale.set_scale(2.0, 3.0, 4.0);
        assert_eq!(scale.values, [2.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 4.0, 0.0, 0.0, 0.0]);
        assert!(matrix_mult([1.0, 1.0, 1.0], &scale.values) == [2.0, 3.0, 4.0]);
    }

    #[test]
    fn compose_applies_the_second_matrix_to_the_first() {
        // `Compose(scale, translate)` scales, then translates.
        let mut scale = Matrix::identity();
        scale.set_scale(2.0, 2.0, 1.0);
        let mut translate = Matrix::identity();
        translate.make_translate(5.0, 0.0, 0.0);

        let mut composed = Matrix::identity();
        composed.compose(&scale, &translate);

        assert!(matrix_mult([1.0, 0.0, 0.0], &composed.values) == [7.0, 0.0, 0.0]);
        assert_eq!(composed.values[9..], [5.0, 0.0, 0.0]);
    }

    #[test]
    fn lift_rotates_the_up_vector_by_the_matrix() {
        let mut matrix = Matrix::identity();
        matrix.make_transform(&[0.0, 0.0, 1.0], std::f32::consts::FRAC_PI_2);

        // Up (0, distance, 0) rotated +90 around Z points along -x.
        let result = lift(2.0, &matrix.values);

        assert!((result[0] + 2.0).abs() < EPSILON);
        assert!(result[1].abs() < EPSILON);
        assert!(result[2].abs() < EPSILON);
    }
}
