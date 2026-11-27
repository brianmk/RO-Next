//! `CParticle` - the billboard object the alternate `CThrowItemEffect` path
//! drives (`Contents/Effects/Particles/Particle.h:49-121`,
//! `Particle.cpp:741-928`).
//!
//! `CParticle` is **not** a `CEffectPrim`: it has no `slots[4]` / `records[50]`
//! state and no primitive id. Each object is one camera-pitched, textured quad
//! whose world matrix the effect body rebuilds in its own `OnProcess`
//! (`ThrowEffects.cpp:500-579`). This module carries only the fields the three
//! ported throw effects (`EF_THROW_HAPPOKUNAI` 981, `EF_THROW_MULTIPLE_COIN`
//! 982, `EF_THROW_BAKURETSU` 983) actually read or write.
//!
//! Field offsets are the official `CParticle` layout
//! (`Particles/Particle.h:65-120`); `Default` is `ResetParticle`
//! (`Particle.cpp:741-777`), not a zeroed struct.

use cgmath::{Point3, Rad, Vector2};
use wgpu::BlendFactor;

use super::super::project_point;
use super::math::{Matrix, matrix_mult};
use super::render3d::renderer_quad_arrays;
use crate::graphics::Color;
use crate::loaders::{ImageType, TextureLoader};
use crate::renderer::EffectRenderer;
use crate::world::Camera;

/// The four local corners `CParticle::InitParticleQuad` writes
/// (`Particle.cpp:847-860`), in the official order: top left, top right,
/// bottom left, bottom right.
const LOCAL_CORNERS: [[f32; 3]; 4] = [[-1.0, 1.0, 1.0], [1.0, 1.0, 1.0], [-1.0, -1.0, 1.0], [1.0, -1.0, 1.0]];

/// `SetQuad(vertices, 0.0f, 1.0f, 0.0f, 1.0f)` (`Particle.cpp:828-843`).
const QUAD_UVS: [Vector2<f32>; 4] = [
    Vector2::new(0.0, 0.0),
    Vector2::new(1.0, 0.0),
    Vector2::new(0.0, 1.0),
    Vector2::new(1.0, 1.0),
];

/// One `CParticle` created by `CreateParticle(&position, 5)`
/// (`EffectMemoryMgr.cpp:246-252`).
#[derive(Debug, Clone, PartialEq)]
pub struct CastedParticle {
    /// `/* 0004 */ effectId`
    pub effect_id: i32,
    /// `/* 0008 */ groupIndex`
    pub group_index: i32,
    /// `/* 000C */ columnIndex`
    pub column_index: i32,
    /// `/* 0010 */ rowIndex`
    pub row_index: i32,
    /// `/* 0018 */ texture`
    pub texture: Option<&'static str>,
    /// `/* 001C */ flags`
    pub flags: i32,
    /// `/* 0020 */ scale`
    pub scale: f32,
    /// `/* 0024 */ position`
    pub position: [f32; 3],
    /// `/* 003C */ red`
    pub red: i16,
    /// `/* 003E */ green`
    pub green: i16,
    /// `/* 0040 */ blue`
    pub blue: i16,
    /// `/* 0044 */ alpha`
    pub alpha: i32,
    /// `/* 004C */ phaseParameter`
    pub phase_parameter: i32,
    /// `/* 0054 */ animationParameter`
    pub animation_parameter: f32,
    /// `/* 00C0 */ speedParameter`
    pub speed_parameter: f32,
    /// `/* 00C4 */ angle`
    pub angle: f32,
    /// `/* 00D0 */ offsetX`
    pub offset_x: f32,
    /// `/* 00D4 */ offsetY`
    pub offset_y: f32,
    /// `/* 00D8 */ offsetZ`
    pub offset_z: f32,
    /// `/* 00E8 */ elapsedFrames`
    pub elapsed_frames: i32,
    /// `/* 01C8 */ transform`
    pub transform: Matrix,
    /// `/* 02A8 */ sourceBlend`
    pub source_blend: i32,
    /// `/* 02AC */ destinationBlend`
    pub destination_blend: i32,
}

impl Default for CastedParticle {
    /// `CParticle::ResetParticle` - `Particle.cpp:741-777`.
    fn default() -> Self {
        Self {
            effect_id: 0,
            group_index: 0,
            column_index: 0,
            row_index: 0,
            texture: None,
            flags: -1,
            scale: 1.0,
            position: [0.0; 3],
            red: 255,
            green: 255,
            blue: 255,
            alpha: 255,
            phase_parameter: 0,
            animation_parameter: 0.0,
            speed_parameter: 0.0,
            angle: 0.0,
            offset_x: 0.0,
            offset_y: 0.0,
            offset_z: 0.0,
            elapsed_frames: 0,
            transform: Matrix::identity(),
            source_blend: 5,
            destination_blend: 2,
        }
    }
}

impl CastedParticle {
    /// `CreateParticle(&position, flags)` - `EffectMemoryMgr.cpp:246-252`.
    ///
    /// The pool hands back a reset `CParticle` and overwrites the position and
    /// flags; the port's owned `Vec` does the same.
    pub fn new(position: [f32; 3], flags: i32) -> Self {
        let mut particle = Self::default();
        particle.position = position;
        particle.flags = flags;
        particle
    }

    /// `CParticle::SetTexture(const char*)` - `Particle.cpp:956-958`.
    ///
    /// The official call resolves the texture handle immediately; the port
    /// stores the path and lets the holder's loader resolve it at render time,
    /// like every other ported texture.
    pub fn set_texture(&mut self, name: &'static str) {
        self.texture = Some(name);
    }

    /// `CParticle::SetColor(short, short, short)` - `Particle.cpp:942-952`.
    ///
    /// `-1` leaves that channel untouched.
    pub fn set_color(&mut self, red: i16, green: i16, blue: i16) {
        if red != -1 {
            self.red = red;
        }

        if green != -1 {
            self.green = green;
        }

        if blue != -1 {
            self.blue = blue;
        }
    }

    /// The four world-space corners after `transform` (`Particle.cpp:883-910`).
    pub fn corners(&self) -> [[f32; 3]; 4] {
        LOCAL_CORNERS.map(|corner| matrix_mult(corner, &self.transform.values))
    }

    /// `SetQuad`'s packed `argb` (`Particle.cpp:828-833`), decoded the same
    /// way as every other ported effect colour.
    pub fn color(&self) -> Color {
        let argb = ((self.alpha << 8 | self.red as i32) << 8 | self.green as i32) << 8 | self.blue as i32;
        let argb = argb as u32;

        Color::rgba_u8(
            ((argb >> 16) & 0xFF) as u8,
            ((argb >> 8) & 0xFF) as u8,
            (argb & 0xFF) as u8,
            ((argb >> 24) & 0xFF) as u8,
        )
    }

    /// `RenderPrimitive::sourceBlend` / `.destinationBlend`
    /// (`Particle.cpp:924-927`). The official effect path only uses
    /// `SRCALPHA` (5), `ONE` (2) and `INVSRCALPHA` (6).
    pub fn blend_factors(&self) -> (BlendFactor, BlendFactor) {
        (blend_factor(self.source_blend), blend_factor(self.destination_blend))
    }
}

fn blend_factor(value: i32) -> BlendFactor {
    match value {
        // D3DBLEND_SRCALPHA
        5 => BlendFactor::SrcAlpha,
        // D3DBLEND_INVSRCALPHA
        6 => BlendFactor::OneMinusSrcAlpha,
        // D3DBLEND_ONE; also the fallback for the values this path never
        // writes (the official enum has 0..=10).
        _ => BlendFactor::One,
    }
}

/// `CParticle::Render(true, 0.0f, 1.0f, 0.0f, 1.0f, false, true)` -
/// `Particle.cpp:870-928`.
///
/// The official projection composes `transform * view` and projects every
/// corner. The port uses the same camera matrix through [`project_point`] and
/// the shared quad ordering of [`renderer_quad_arrays`], so the geometry,
/// per-corner depth and UV pairing match `PP_3DTEXTURE`'s path.
pub(crate) fn render_particles(renderer: &mut EffectRenderer, camera: &dyn Camera, textures: &TextureLoader, particles: &[CastedParticle]) {
    let screen_size = renderer.window_size();

    for particle in particles {
        let Some(path) = particle.texture else {
            continue;
        };

        let Ok(texture) = textures.get_or_load(path, ImageType::Color) else {
            continue;
        };

        // `transform` already contains the `MakeTranslate` from `OnProcess`;
        // `CParticle::Render(true)` ignores `position`.
        let anchor = Point3::new(
            particle.transform.values[9],
            particle.transform.values[10],
            particle.transform.values[11],
        );

        // `Project` rejects the whole quad when any corner is behind the
        // camera (`Particle.cpp:910-921`), and the port's `project_point` also
        // rejects `w <= 0`.
        let Some((anchor_screen, _)) = project_point(camera, anchor, screen_size) else {
            continue;
        };

        let mut screen_corners = [Vector2::new(0.0, 0.0); 4];
        let mut corner_depths = [0.0; 4];
        let mut visible = true;

        for (index, corner) in particle.corners().iter().enumerate() {
            let Some((projected, depth)) = project_point(camera, Point3::new(corner[0], corner[1], corner[2]), screen_size) else {
                visible = false;
                break;
            };

            screen_corners[index] = projected - anchor_screen;
            corner_depths[index] = depth;
        }

        if !visible {
            continue;
        }

        let (corners, texture_coordinates, corner_depths) = renderer_quad_arrays(screen_corners, QUAD_UVS, corner_depths);
        let (source, destination) = particle.blend_factors();

        renderer.render_effect_with_depth(
            camera,
            anchor,
            texture,
            corners,
            corner_depths,
            texture_coordinates,
            Vector2::new(0.0, 0.0),
            Rad(0.0),
            particle.color(),
            source,
            destination,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::particle::CastedParticle;

    #[test]
    fn defaults_match_reset_particle() {
        let particle = CastedParticle::default();

        assert_eq!(particle.red, 255);
        assert_eq!(particle.green, 255);
        assert_eq!(particle.blue, 255);
        assert_eq!(particle.alpha, 255);
        assert_eq!(particle.scale, 1.0);
        assert_eq!(particle.speed_parameter, 0.0);
        assert_eq!(particle.source_blend, 5);
        assert_eq!(particle.destination_blend, 2);
        assert_eq!(particle.transform, Matrix::identity());
        assert!(particle.texture.is_none());
    }

    #[test]
    fn set_color_skips_minus_one_channels() {
        let mut particle = CastedParticle::default();
        particle.set_color(255, -1, 135);

        assert_eq!(particle.red, 255);
        assert_eq!(particle.green, 255);
        assert_eq!(particle.blue, 135);
    }

    #[test]
    fn identity_transform_keeps_the_official_unit_quad() {
        let particle = CastedParticle::default();

        assert_eq!(particle.corners(), LOCAL_CORNERS);
    }

    #[test]
    fn the_transform_scales_and_translates_the_quad() {
        let mut particle = CastedParticle::default();
        particle.transform.set_scale(2.0, 3.0, 1.0);
        let mut translation = Matrix::identity();
        translation.make_translate(5.0, 6.0, 7.0);
        let mut composed = Matrix::identity();
        composed.compose(&particle.transform, &translation);
        particle.transform = composed;

        let corners = particle.corners();

        assert_eq!(corners[0], [3.0, 9.0, 8.0]);
        assert_eq!(corners[3], [7.0, 3.0, 8.0]);
    }

    #[test]
    fn color_packs_alpha_red_green_blue_in_the_official_order() {
        let mut particle = CastedParticle::default();
        particle.alpha = 128;
        particle.red = 10;
        particle.green = 20;
        particle.blue = 30;

        let color = particle.color();

        assert!((color.red - 10.0 / 255.0).abs() < 1e-6);
        assert!((color.green - 20.0 / 255.0).abs() < 1e-6);
        assert!((color.blue - 30.0 / 255.0).abs() < 1e-6);
        assert!((color.alpha - 128.0 / 255.0).abs() < 1e-6);
    }

    #[test]
    fn blend_factors_follow_the_d3d_values() {
        let mut particle = CastedParticle::default();
        particle.source_blend = 5;
        particle.destination_blend = 6;

        assert_eq!(particle.blend_factors(), (BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha));

        particle.destination_blend = 2;
        assert_eq!(particle.blend_factors(), (BlendFactor::SrcAlpha, BlendFactor::One));
    }
}
