use std::sync::Arc;

use cgmath::{Deg, Matrix4, Point3, SquareMatrix, Vector2, Vector3, Vector4, Zero};
use ragnarok_packets::EntityId;
use wgpu::BlendFactor;

use super::color::Color;
#[cfg(feature = "debug")]
use super::settings::RenderOptions;
use super::vertices::ModelVertex;
use super::{Buffer, ShadowDetail, ShadowMethod, SlugFont, Texture, TextureSet, TileVertex, WaterVertex};
use crate::graphics::{CornerDiameter, ScreenClip, ScreenPosition, ScreenSize, ShadowPadding};
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;

#[derive(Default)]
pub struct RenderInstruction<'a> {
    pub show_interface: bool,
    /// Clear colour of the forward pass (the map sky, or black).
    pub background_color: Color,
    pub picker_position: ScreenPosition,
    pub uniforms: Uniforms,
    pub indicator: Option<IndicatorInstruction>,
    pub interface: &'a [InterfaceRectangleInstruction],
    /// Between 3D world and effects.
    pub bottom_layer_rectangles: &'a [RectangleInstruction],
    /// Between effects and interface.
    pub middle_layer_rectangles: &'a [RectangleInstruction],
    /// On top of everything else.
    pub top_layer_rectangles: &'a [RectangleInstruction],
    pub directional_light: DirectionalLightInstruction,
    pub directional_light_partitions: &'a [DirectionalLightPartitionInstruction],
    pub point_light: &'a [PointLightInstruction],
    pub point_light_with_shadows: &'a [PointLightWithShadowInstruction],
    pub model_batches: &'a [ModelBatch],
    pub models: &'a mut [ModelInstruction],
    pub entities: &'a mut [EntityInstruction],
    pub directional_shadow_model_batches: &'a [Vec<ModelBatch>],
    pub directional_shadow_models: &'a [ModelInstruction],
    pub directional_shadow_entities: &'a mut [Vec<EntityInstruction>],
    pub point_shadow_models: &'a [ModelInstruction],
    pub point_shadow_entities: &'a mut [EntityInstruction],
    pub effects: &'a [EffectInstruction],
    /// Additive triangle-list effects (the slot/record primitives of the
    /// official client: rings, cylinders, columns, fans). Drawn in the same
    /// pass as `effects`, after every quad.
    pub effect_triangles: &'a [EffectTriangleInstruction],
    pub water: Option<WaterInstruction<'a>>,
    pub map_picker_tile_vertex_buffer: Option<&'a Buffer<TileVertex>>,
    pub map_picker_tile_index_buffer: Option<&'a Buffer<u32>>,
    pub slug_font: Option<&'a SlugFont>,
    #[cfg(feature = "debug")]
    pub render_options: RenderOptions,
    #[cfg(feature = "debug")]
    pub aabb: &'a [DebugAabbInstruction],
    #[cfg(feature = "debug")]
    pub circles: &'a [DebugCircleInstruction],
    #[cfg(feature = "debug")]
    pub rectangles: &'a [DebugRectangleInstruction],
    #[cfg(feature = "debug")]
    pub marker: &'a [MarkerInstruction],
}

#[derive(Clone, Debug)]
pub struct Uniforms {
    pub view_matrix: Matrix4<f32>,
    pub projection_matrix: Matrix4<f32>,
    pub camera_position: Vector4<f32>,
    pub animation_timer_ms: f32,
    pub ambient_light_color: Color,
    /// Quantise the direct/ambient light into flat cel bands.
    pub stylized_lighting: bool,
    /// Scene fog colour; `alpha == 0` disables fog.
    pub fog_color: Color,
    /// Scene fog distance window (`near`, `far`).
    pub fog_range: [f32; 2],
    /// Character/NPC-only night fill (rgb colour, a intensity).
    pub moon_fill: Color,
    /// Procedural weather: x kind, y time, z intensity, w seed.
    pub weather: [f32; 4],
    pub enhanced_lighting: bool,
    pub shadow_method: ShadowMethod,
    pub shadow_detail: ShadowDetail,
    pub use_sdsm: bool,
    pub sdsm_enabled: bool,
}

impl Default for Uniforms {
    fn default() -> Self {
        Self {
            view_matrix: Matrix4::identity(),
            projection_matrix: Matrix4::identity(),
            camera_position: Vector4::zero(),
            animation_timer_ms: 0.0,
            ambient_light_color: Color::default(),
            stylized_lighting: false,
            fog_color: Color::TRANSPARENT,
            fog_range: [0.0, 0.0],
            moon_fill: Color::rgba(0.0, 0.0, 0.0, 0.0),
            weather: [0.0; 4],
            enhanced_lighting: false,
            shadow_method: ShadowMethod::Hard,
            shadow_detail: ShadowDetail::Low,
            use_sdsm: false,
            sdsm_enabled: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct WaterInstruction<'a> {
    pub water_texture: &'a Texture,
    pub water_vertex_buffer: &'a Buffer<WaterVertex>,
    pub water_index_buffer: &'a Buffer<u32>,
    pub texture_repeat: f32,
    pub waveform_phase_shift: f32,
    pub waveform_amplitude: f32,
    pub waveform_frequency: Deg<f32>,
    pub water_opacity: f32,
}

#[derive(Clone, Debug)]
pub struct DirectionalLightInstruction {
    pub view_projection_matrix: Matrix4<f32>,
    pub direction: Vector3<f32>,
    pub color: Color,
}

impl Default for DirectionalLightInstruction {
    fn default() -> Self {
        Self {
            view_projection_matrix: Matrix4::identity(),
            direction: Vector3::zero(),
            color: Color::default(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DirectionalLightPartitionInstruction {
    pub view_projection_matrix: Matrix4<f32>,
    pub projection_matrix: Matrix4<f32>,
    pub view_matrix: Matrix4<f32>,
    pub interval_end: f32,
    pub world_space_texel_size: f32,
    pub near_plane: f32,
    pub far_plane: f32,
}

impl Default for DirectionalLightPartitionInstruction {
    fn default() -> Self {
        Self {
            view_projection_matrix: Matrix4::identity(),
            projection_matrix: Matrix4::identity(),
            view_matrix: Matrix4::identity(),
            interval_end: 0.0,
            world_space_texel_size: 0.0,
            near_plane: 0.0,
            far_plane: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PointLightInstruction {
    pub position: Point3<f32>,
    pub color: Color,
    pub range: f32,
}

/// Right now point light can't cast shadows of models that are not part of
/// the map (like the debug models).
#[derive(Clone, Debug)]
pub struct PointLightWithShadowInstruction {
    pub view_projection_matrices: [Matrix4<f32>; 6],
    pub view_matrices: [Matrix4<f32>; 6],
    pub position: Point3<f32>,
    pub color: Color,
    pub range: f32,
    pub model_texture_set: Arc<TextureSet>,
    pub model_vertex_buffer: Arc<Buffer<ModelVertex>>,
    pub model_index_buffer: Arc<Buffer<u32>>,
    /// Start point inside the point_shadow_entities.
    pub entity_offset: [usize; 6],
    /// Model count inside the point_shadow_entities.
    pub entity_count: [usize; 6],
    /// Start point inside the point_shadow_models.
    pub model_offset: [usize; 6],
    /// Model count inside the point_shadow_models.
    pub model_count: [usize; 6],
}

#[derive(Clone, Debug)]
pub enum RectangleInstruction {
    Solid {
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        color: Color,
    },
    Sprite {
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        color: Color,
        texture_position: Vector2<f32>,
        texture_size: Vector2<f32>,
        linear_filtering: bool,
        texture: Arc<Texture>,
    },
    Sdf {
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        color: Color,
        texture_position: Vector2<f32>,
        texture_size: Vector2<f32>,
        texture: Arc<Texture>,
    },
    Text {
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        color: Color,
        em_position: Vector2<f32>,
        em_size: Vector2<f32>,
        glyph_index: u32,
    },
}

#[derive(Clone, Debug)]
pub enum InterfaceRectangleInstruction {
    Solid {
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        screen_clip: ScreenClip,
        color: Color,
        corner_diameter: CornerDiameter,
        shadow_color: Color,
        shadow_padding: ShadowPadding,
    },
    Sprite {
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        screen_clip: ScreenClip,
        color: Color,
        corner_diameter: CornerDiameter,
        texture: Arc<Texture>,
        smooth: bool,
        /// Normalized position of the section of the texture that should be
        /// drawn.
        texture_position: Vector2<f32>,
        /// Normalized size of the section of the texture that should be drawn.
        texture_size: Vector2<f32>,
    },
    Sdf {
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        screen_clip: ScreenClip,
        color: Color,
        corner_diameter: CornerDiameter,
        texture: Arc<Texture>,
    },
    Text {
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        screen_clip: ScreenClip,
        color: Color,
        em_position: Vector2<f32>,
        em_size: Vector2<f32>,
        glyph_index: u32,
    },
}

#[cfg(feature = "debug")]
#[derive(Clone, Debug)]
pub struct MarkerInstruction {
    pub screen_position: ScreenPosition,
    pub screen_size: ScreenSize,
    pub identifier: MarkerIdentifier,
}

#[derive(Clone, Debug)]
pub struct IndicatorInstruction {
    pub upper_left: Point3<f32>,
    pub upper_right: Point3<f32>,
    pub lower_left: Point3<f32>,
    pub lower_right: Point3<f32>,
    pub color: Color,
}

pub struct ModelBatch {
    pub offset: usize,
    pub count: usize,
    pub texture_set: Arc<TextureSet>,
    pub vertex_buffer: Arc<Buffer<ModelVertex>>,
    pub index_buffer: Arc<Buffer<u32>>,
}

#[derive(Clone, Debug)]
pub struct ModelInstruction {
    pub model_matrix: Matrix4<f32>,
    pub index_offset: u32,
    pub index_count: u32,
    pub base_vertex: i32,
    pub texture_index: i32,
    pub distance: f32,
    pub transparent: bool,
}

#[derive(Clone, Debug)]
pub struct EntityInstruction {
    pub world: Matrix4<f32>,
    pub frame_part_transform: Matrix4<f32>,
    pub texture_position: Vector2<f32>,
    pub texture_size: Vector2<f32>,
    pub frame_size: Vector2<f32>,
    pub extra_depth_offset: f32,
    pub depth_offset: f32,
    pub curvature: f32,
    pub color: Color,
    pub mirror: bool,
    pub entity_id: EntityId,
    pub add_to_picker: bool,
    /// Draw this part with the additive (glow) blend instead of the regular
    /// WBOIT blend when it falls in the transparent pass. Set by status
    /// effects (official `render.flags & 1` / `& 2`).
    pub additive: bool,
    /// Flat silhouette copy (official sprite outline): the fragment shader
    /// ignores lighting and writes the instance colour.
    pub flat: bool,
    pub texture: Arc<Texture>,
    pub distance: f32,
}

#[derive(Clone, Debug)]
pub struct EffectInstruction {
    pub top_left: ScreenPosition,
    pub bottom_left: ScreenPosition,
    pub top_right: ScreenPosition,
    pub bottom_right: ScreenPosition,
    /// NDC depth (reverse-z) of the corners, in the same order as the corner
    /// positions above (top left, bottom left, top right, bottom right).
    /// Used to depth test the effect against the scene.
    pub depth: [f32; 4],
    pub texture_top_left: Vector2<f32>,
    pub texture_bottom_left: Vector2<f32>,
    pub texture_top_right: Vector2<f32>,
    pub texture_bottom_right: Vector2<f32>,
    pub color: Color,
    pub source_blend_factor: BlendFactor,
    pub destination_blend_factor: BlendFactor,
    pub texture: Arc<Texture>,
}

/// One corner of an [`EffectTriangleInstruction`].
///
/// The positions are already projected: `clip_space_position` is a pair of NDC
/// coordinates and `depth` is the NDC depth (reverse-z) of that corner. The
/// official slot/record renderers project every vertex of every triangle on
/// their own, so a triangle can span three different depths; storing the NDC
/// values keeps that exact and needs no camera matrix in the shader.
#[derive(Clone, Copy, Debug)]
pub struct EffectTriangleVertex {
    pub clip_space_position: ScreenPosition,
    pub depth: f32,
    pub texture_coordinate: Vector2<f32>,
}

/// A single triangle of the official client's slot/record primitives.
///
/// Those primitives are submitted as `D3DPT_TRIANGLELIST` triangles with a
/// constant `diffuse` per triangle (`RenderTeiRect`, `RenderRecordRings`,
/// `RenderArcBand`, `RenderPyramid`), so one instruction is one triangle with
/// one colour. The client submits them into the alpha/custom-blend queues; the
/// renderer batches them by blend state exactly like the quad path.
#[derive(Clone, Debug)]
pub struct EffectTriangleInstruction {
    pub vertices: [EffectTriangleVertex; 3],
    pub color: Color,
    pub source_blend_factor: BlendFactor,
    pub destination_blend_factor: BlendFactor,
    pub texture: Arc<Texture>,
}

#[cfg(feature = "debug")]
#[derive(Copy, Clone, Debug)]
pub struct DebugAabbInstruction {
    pub world: Matrix4<f32>,
    pub color: Color,
}

#[cfg(feature = "debug")]
#[derive(Copy, Clone, Debug)]
pub struct DebugCircleInstruction {
    pub position: Point3<f32>,
    pub color: Color,
    pub screen_position: ScreenPosition,
    pub screen_size: ScreenSize,
}

#[cfg(feature = "debug")]
#[derive(Copy, Clone, Debug)]
pub struct DebugRectangleInstruction {
    pub world: Matrix4<f32>,
    pub color: Color,
}
