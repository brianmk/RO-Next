//! Exact port of `Source/Utility/Math/AffineMatrix.cpp` (the subset the
//! primitives use).
//!
//! The client's `matrix` is 12 floats, row-major 3x4: rows `0..3`, `3..6`,
//! `6..9` are the rotation/scale part and `9..12` the translation. Rotations are
//! built with `ClientSin`/`ClientCos` (float `sinf`/`cosf`) and appended with
//! `Rotate`, whose row-major multiply order is preserved here.

use super::angles::{client_cos, client_sin};

/// `matrix` - 12 floats, row-major 3x4.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix {
    pub values: [f32; 12],
}

/// `g_identityMatrix`.
pub const IDENTITY: [f32; 12] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0];

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

    /// `Rotate(float* values, const float* rotation)` - `AffineMatrix.cpp:16-36`.
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

/// `Forward(vector3d&, float, const float*)` - `EffectPrimitiveMotion.cpp:76-81`.
pub fn forward(distance: f32, matrix: &[f32; 12]) -> [f32; 3] {
    let point = [0.0f32, 0.0f32, distance];

    [
        ((matrix[3] * point[1] + matrix[0] * point[0]) + point[2] * matrix[6]) + matrix[9],
        ((matrix[4] * point[1] + matrix[1] * point[0]) + point[2] * matrix[7]) + matrix[10],
        ((matrix[5] * point[1] + matrix[2] * point[0]) + point[2] * matrix[8]) + matrix[11],
    ]
}

/// `Upward(vector3d&, float, const float*)` - `EffectPrimitiveMotion.cpp:84-89`.
pub fn upward(distance: f32, matrix: &[f32; 12]) -> [f32; 3] {
    let point = [0.0f32, -distance, 0.0f32];

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
}
