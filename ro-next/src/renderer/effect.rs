use std::sync::Arc;

use cgmath::{Matrix2, Point3, Rad, Vector2, Vector4};
use ro_next_interface::application::Position;
use wgpu::BlendFactor;

use crate::graphics::{Color, EffectInstruction, EffectTriangleInstruction, EffectTriangleVertex, ScreenPosition, ScreenSize, Texture};
use crate::world::Camera;

pub struct EffectRenderer {
    instructions: Vec<EffectInstruction>,
    /// Additive triangle-list primitives of the official slot/record renderers
    /// (rings, cylinders, columns, fans). They cannot be expressed as quads
    /// because every vertex has its own depth and the strips are not planar, so
    /// they go through their own GPU path in the forward effect pass.
    triangle_instructions: Vec<EffectTriangleInstruction>,
    window_size: ScreenSize,
}

impl EffectRenderer {
    pub fn new(window_size: ScreenSize) -> Self {
        Self {
            instructions: Vec::default(),
            triangle_instructions: Vec::default(),
            window_size,
        }
    }

    pub fn clear(&mut self) {
        self.instructions.clear();
        self.triangle_instructions.clear();
    }

    pub fn get_instructions(&self) -> &[EffectInstruction] {
        self.instructions.as_ref()
    }

    /// The triangles submitted for the current frame.
    pub fn get_triangle_instructions(&self) -> &[EffectTriangleInstruction] {
        self.triangle_instructions.as_ref()
    }

    pub fn update_window_size(&mut self, window_size: ScreenSize) {
        self.window_size = window_size;
    }

    pub fn window_size(&self) -> ScreenSize {
        self.window_size
    }

    /// Submits one triangle of a slot/record primitive.
    ///
    /// The three corners are world positions; each is projected (and depth
    /// tested) on its own, exactly like the official `Project` + `Visible`
    /// pair: a triangle with any corner at or behind the camera plane is
    /// dropped (`RenderTriangle::Visible` rejects `rhw <= 0`), and every corner
    /// keeps its own NDC depth (reverse-z) so the triangle is depth tested
    /// against the scene per vertex.
    ///
    /// The colour is per triangle, which is what every official emitter does
    /// (`RenderTeiRect`, `RenderRecordRings`, `RenderArcBand`, `RenderPyramid`
    /// write the same diffuse to all three vertices).
    #[allow(clippy::too_many_arguments)]
    pub fn render_triangle(
        &mut self,
        camera: &dyn Camera,
        corners: [Point3<f32>; 3],
        texture_coordinates: [Vector2<f32>; 3],
        color: Color,
        source_blend_factor: BlendFactor,
        destination_blend_factor: BlendFactor,
        texture: Arc<Texture>,
    ) {
        let view_projection = camera.view_projection_matrix();
        let mut vertices = [EffectTriangleVertex {
            clip_space_position: ScreenPosition::new(0.0, 0.0),
            depth: 0.0,
            texture_coordinate: Vector2::new(0.0, 0.0),
        }; 3];

        for (index, corner) in corners.iter().enumerate() {
            let clip_space_position = view_projection * corner.to_homogeneous();

            // `RenderTriangle::Visible` requires every corner to have a
            // positive reciprocal w.
            if clip_space_position.w <= 0.0 {
                return;
            }

            vertices[index] = EffectTriangleVertex {
                clip_space_position: ScreenPosition::new(
                    clip_space_position.x / clip_space_position.w,
                    clip_space_position.y / clip_space_position.w,
                ),
                depth: clip_space_position.z / clip_space_position.w,
                texture_coordinate: texture_coordinates[index],
            };
        }

        self.triangle_instructions.push(EffectTriangleInstruction {
            vertices,
            color,
            source_blend_factor,
            destination_blend_factor,
            texture,
        });
    }

    /// Submits one triangle with an explicit NDC depth for every corner.
    ///
    /// Used by the official `RenderTeiRect` variants whose `extra` flag forces
    /// the triangle to the near plane (`extra == 1` in the decompilation writes
    /// z = 1e-09 into a standard depth buffer, i.e. "always on top"); the
    /// port's reverse-z buffer expresses the same thing as depth 1.0.
    #[allow(clippy::too_many_arguments)]
    pub fn render_triangle_with_depth(
        &mut self,
        camera: &dyn Camera,
        corners: [Point3<f32>; 3],
        texture_coordinates: [Vector2<f32>; 3],
        depths: [f32; 3],
        color: Color,
        source_blend_factor: BlendFactor,
        destination_blend_factor: BlendFactor,
        texture: Arc<Texture>,
    ) {
        let view_projection = camera.view_projection_matrix();
        let mut vertices = [EffectTriangleVertex {
            clip_space_position: ScreenPosition::new(0.0, 0.0),
            depth: 0.0,
            texture_coordinate: Vector2::new(0.0, 0.0),
        }; 3];

        for (index, corner) in corners.iter().enumerate() {
            let clip_space_position = view_projection * corner.to_homogeneous();

            if clip_space_position.w <= 0.0 {
                return;
            }

            vertices[index] = EffectTriangleVertex {
                clip_space_position: ScreenPosition::new(
                    clip_space_position.x / clip_space_position.w,
                    clip_space_position.y / clip_space_position.w,
                ),
                depth: depths[index],
                texture_coordinate: texture_coordinates[index],
            };
        }

        self.triangle_instructions.push(EffectTriangleInstruction {
            vertices,
            color,
            source_blend_factor,
            destination_blend_factor,
            texture,
        });
    }

    /// Renders a screen space effect quad.
    ///
    /// All corners of the effect are assumed to lie on a plane through
    /// `position` that is parallel to the camera plane, which is how the
    /// `.str` effect files of the original client work. The quad is depth
    /// tested against the scene at that depth.
    #[allow(clippy::too_many_arguments)]
    pub fn render_effect(
        &mut self,
        camera: &dyn Camera,
        position: Point3<f32>,
        texture: Arc<Texture>,
        corner_screen_position: [Vector2<f32>; 4],
        texture_coordinates: [Vector2<f32>; 4],
        offset: Vector2<f32>,
        angle: Rad<f32>,
        color: Color,
        source_blend_factor: BlendFactor,
        destination_blend_factor: BlendFactor,
    ) {
        let clip_space_position = camera.view_projection_matrix() * position.to_homogeneous();
        let depth = clip_space_depth(clip_space_position);

        self.render_effect_with_depth(
            camera,
            position,
            texture,
            corner_screen_position,
            [depth; 4],
            texture_coordinates,
            offset,
            angle,
            color,
            source_blend_factor,
            destination_blend_factor,
        );
    }

    /// Renders a screen space effect quad with an explicit NDC depth
    /// (reverse-z) for every corner, in the same order as the corner
    /// positions. Used by effects whose corners are actual world positions
    /// (and therefore don't share a single depth).
    #[allow(clippy::too_many_arguments)]
    pub fn render_effect_with_depth(
        &mut self,
        camera: &dyn Camera,
        position: Point3<f32>,
        texture: Arc<Texture>,
        corner_screen_position: [Vector2<f32>; 4],
        corner_depth: [f32; 4],
        texture_coordinates: [Vector2<f32>; 4],
        offset: Vector2<f32>,
        angle: Rad<f32>,
        color: Color,
        source_blend_factor: BlendFactor,
        destination_blend_factor: BlendFactor,
    ) {
        const EFFECT_ORIGIN: Vector2<f32> = Vector2::new(319.0, 291.0);

        let clip_space_position = camera.view_projection_matrix() * position.to_homogeneous();
        let screen_space_position = camera.clip_to_screen_space(clip_space_position);

        let half_screen = Vector2::new(self.window_size.width / 2.0, self.window_size.height / 2.0);
        let rotation_matrix = Matrix2::from_angle(angle);

        let corner_screen_position =
            corner_screen_position.map(|position| (rotation_matrix * position) + offset - EFFECT_ORIGIN - half_screen);

        let clip_space_positions = corner_screen_position.map(|position| {
            let normalized_screen_position = Vector2::new(
                (position.x / half_screen.x) * 0.5 + 0.5 + screen_space_position.x,
                (position.y / half_screen.y) * 0.5 + 0.5 + screen_space_position.y,
            );
            let clip_space_position = camera.screen_to_clip_space(normalized_screen_position);
            ScreenPosition::new(clip_space_position.x, clip_space_position.y)
        });

        self.instructions.push(EffectInstruction {
            top_left: clip_space_positions[0],
            bottom_left: clip_space_positions[2],
            top_right: clip_space_positions[1],
            bottom_right: clip_space_positions[3],
            // The instruction stores the corners in the order top left, bottom left, top
            // right, bottom right.
            depth: instruction_corner_depths(corner_depth),
            texture_top_left: texture_coordinates[2],
            texture_bottom_left: texture_coordinates[3],
            texture_top_right: texture_coordinates[1],
            texture_bottom_right: texture_coordinates[0],
            color,
            source_blend_factor,
            destination_blend_factor,
            texture,
        });
    }
}

/// Computes the NDC depth (reverse-z) of a clip space position.
///
/// The scene uses reverse-z (the projection maps a view depth of `z` to `near /
/// z`, the depth buffer is cleared to 0 and compares with `Greater`), so the
/// value returned here can be used as the z component of the clip space
/// position of an effect vertex and gets depth tested against the scene in
/// exactly the same way as regular geometry.
fn clip_space_depth(clip_space_position: Vector4<f32>) -> f32 {
    if clip_space_position.w > f32::EPSILON {
        clip_space_position.z / clip_space_position.w
    } else {
        // Behind the camera: the effect is not visible, use the far plane. Using 0 here
        // is safe because the depth test (`Greater`) will fail against any
        // cleared background as well as against any geometry.
        0.0
    }
}

/// Re-orders per-corner depths from the order used by `render_effect` (top
/// left, top right, bottom left, bottom right) into the order used by
/// [`EffectInstruction`] (top left, bottom left, top right, bottom right).
fn instruction_corner_depths(corner_depth: [f32; 4]) -> [f32; 4] {
    [corner_depth[0], corner_depth[2], corner_depth[1], corner_depth[3]]
}

#[cfg(test)]
mod tests {
    use cgmath::Vector4;

    use super::{clip_space_depth, instruction_corner_depths};

    #[test]
    fn clip_depth_is_ndc_z() {
        // Reverse-z projection: a point at twice the view depth has half the NDC depth.
        // The values are the ones the forward shaders write, so the effect
        // depth test compares like with like.
        assert!((clip_space_depth(Vector4::new(0.0, 0.0, 0.5, 1.0)) - 0.5).abs() < f32::EPSILON);
        assert!((clip_space_depth(Vector4::new(1.0, 2.0, 0.25, 4.0)) - 0.0625).abs() < f32::EPSILON);
        // With the reverse-z projection the clip space z is the near plane and w is the
        // view depth, so the NDC depth strictly decreases with increasing view depth.
        assert!(clip_space_depth(Vector4::new(0.0, 0.0, 0.1, 1.0)) > clip_space_depth(Vector4::new(0.0, 0.0, 0.1, 2.0)));
    }

    #[test]
    fn clip_depth_behind_camera_is_zero() {
        assert_eq!(clip_space_depth(Vector4::new(0.0, 0.0, 1.0, 0.0)), 0.0);
        assert_eq!(clip_space_depth(Vector4::new(0.0, 0.0, 1.0, -1.0)), 0.0);
    }

    #[test]
    fn corner_depths_follow_instruction_order() {
        // render_effect takes its corners as (top left, top right, bottom left, bottom
        // right) while the effect instruction/shaders use (top left, bottom
        // left, top right, bottom right). The depths have to be permuted the
        // same way as the corners.
        assert_eq!(instruction_corner_depths([1.0, 2.0, 3.0, 4.0]), [1.0, 3.0, 2.0, 4.0]);
    }
}
