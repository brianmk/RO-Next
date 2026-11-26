//! Exact port of `Source/Utility/Math/ClientAngles.cpp`.
//!
//! The client does not call `sinf` at use sites. It builds a 722-entry table
//! from `index / 180.0f * 3.141592f` (the literal, not `PI`) and reads it with
//! integer degrees, using the second (negated) half for negative angles at
//! index `361 + degrees` - which is one degree off the mathematical value.
//! Effects place quads and particles with these tables, so the port has to
//! reproduce them including that offset.

/// `g_radTable` / `g_sinTable` / `g_cosTable`.
pub struct Angles {
    sin_table: [f32; 722],
    cos_table: [f32; 722],
    rad_table: [f32; 722],
}

impl Angles {
    /// `CreateTrigonometricTable()` - `ClientAngles.cpp:24-34`.
    pub fn new() -> Self {
        let mut sin_table = [0.0f32; 722];
        let mut cos_table = [0.0f32; 722];
        let mut rad_table = [0.0f32; 722];

        for index in 0..=360usize {
            let angle = index as f32 / 180.0f32 * 3.141592f32;
            rad_table[index] = angle;
            sin_table[index] = client_sin(angle);
            cos_table[index] = client_cos(angle);
            rad_table[index + 361] = -rad_table[index];
            sin_table[index + 361] = -sin_table[index];
            cos_table[index + 361] = -cos_table[index];
        }

        Self {
            sin_table,
            cos_table,
            rad_table,
        }
    }

    /// `GetRadian(int)` - `ClientAngles.cpp:37-45`.
    pub fn radian(&self, mut degrees: i32) -> f32 {
        wrap(&mut degrees);
        if degrees < 0 {
            self.rad_table[(361 + degrees) as usize]
        } else {
            self.rad_table[degrees as usize]
        }
    }

    /// `ClientSine(int)` - `ClientAngles.cpp:48-55`.
    pub fn sine(&self, mut degrees: i32) -> f32 {
        wrap(&mut degrees);
        if degrees < 0 {
            self.sin_table[(361 + degrees) as usize]
        } else {
            self.sin_table[degrees as usize]
        }
    }

    /// `ClientCosine(int)` - `ClientAngles.cpp:58-65`.
    pub fn cosine(&self, mut degrees: i32) -> f32 {
        wrap(&mut degrees);
        if degrees < 0 {
            self.cos_table[(361 + degrees) as usize]
        } else {
            self.cos_table[degrees as usize]
        }
    }

    /// Raw `g_cosTable[index]`, for code that indexes the table with a
    /// truncated float and no wrap (`Render2DCircle`, `Render2DPrimitive`).
    pub fn raw_cosine(&self, index: usize) -> f32 {
        self.cos_table[index]
    }

    /// Raw `g_sinTable[index]`; see [`Self::raw_cosine`].
    pub fn raw_sine(&self, index: usize) -> f32 {
        self.sin_table[index]
    }
}

impl Angles {
    /// Process-wide table, like the client globals (`g_sinTable`, ...).
    pub fn shared() -> &'static Angles {
        static ANGLES: std::sync::OnceLock<Angles> = std::sync::OnceLock::new();
        ANGLES.get_or_init(Angles::new)
    }
}

impl Default for Angles {
    fn default() -> Self {
        Self::new()
    }
}

/// `Wrap(int&)` - `ClientAngles.cpp:16-22`.
fn wrap(degrees: &mut i32) {
    if *degrees > 360 || *degrees < -360 {
        *degrees %= 360;
    }
}

/// `ClientSin(volatile float)`.
fn client_sin(angle: f32) -> f32 {
    (angle as f64).sin() as f32
}

/// `ClientCos(volatile float)`.
fn client_cos(angle: f32) -> f32 {
    (angle as f64).cos() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_uses_the_original_constant() {
        let angles = Angles::new();

        assert!(angles.sine(0).abs() < 1e-6);
        assert!((angles.cosine(0) - 1.0).abs() < 1e-6);
        assert!((angles.sine(90) - 1.0).abs() < 1e-5);
        // Not exactly zero: the table is built with 3.141592f.
        assert!(angles.cosine(90).abs() < 2e-6);
    }

    #[test]
    fn negative_angles_use_the_361_offset() {
        let angles = Angles::new();

        // ClientSine(-90) reads the mirrored half at 271 == sin(-89 deg).
        assert_eq!(angles.sine(-90), angles.raw_sine(271));
        assert!((angles.sine(-90) + 1.0).abs() < 1e-3);
    }

    #[test]
    fn wrap_only_beyond_a_full_turn() {
        let angles = Angles::new();

        assert_eq!(angles.sine(361), angles.sine(1));
        assert_eq!(angles.cosine(721), angles.cosine(1));
    }
}
