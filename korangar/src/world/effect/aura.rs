//! Classic level-99 aura rendered below player entities.
//!
//! The original client composes the level-99 aura from three official effects
//! (see roBrowser `EffectTable.js`, ids 200/201/202):
//!
//! * `SwirlingAura` — three tilted bands sweeping around the character
//!   (`effect/ring_blue.tga`, 21 divisions, 315° arc, three bands with
//!   different heights/radii/rise angles and spin speeds),
//! * `GroundAura` — two flat, pulsing ground quads (`effect/pikapika2.bmp`,
//!   size 100, distance 15),
//! * `Level99Bubble` — camera facing sparkle quads (`effect/whitelight.tga`, 4
//!   emitter columns with 4 anchors each, falling anchor points, additive
//!   blend).
//!
//! The original client textures of the classic aura (`aurafloat.tga`,
//! `auraring.bmp`) are not part of the CondaRO data set, so the aura keeps the
//! procedural ground rings/glow/sparkles that were built from the effect
//! textures that do exist, and adds the three official components on top:
//!
//! * `effect/ring_liner.tga` — rotating outer ring,
//! * `effect/ring_blue.tga` / `effect/ring_red.tga` — inner ring and the
//!   swirling bands,
//! * `effect/glow01..glow04.bmp` — soft ground glow,
//! * `effect/pikapika2.bmp` — floating sparkles and the official ground aura,
//! * `effect/whitelight.tga` — official bubble sparkles.
//!
//! Every texture is optional: if one is missing the aura uses the remaining
//! ones (and the missing asset name is logged, so a trimmed data set can never
//! cause a panic or a black quad). The aura is drawn with the regular effect
//! renderer (additive blend, depth tested against the scene), so it is occluded
//! by map geometry like every other effect.
//!
//! The aura color is chosen from the configured [`AuraStyle`]:
//!
//! * `Auto` (default) — classic tiers: cyan for level 99, violet for
//!   transcendent/rebirth level 99, blue for 100+, gold for 175+,
//! * `Off` — no aura,
//! * `Cyan`, `Violet`, `Blue`, `Gold`, `Green` — explicit style for all auras.
//!   `Green` is the official EF_LEVEL99 size-type 7 variant.
//!
//! The effect is attached to the entity (`EffectCenter::Entity`), follows it
//! while walking and removes itself when the entity despawns, dies or drops
//! below level 99.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use cgmath::{InnerSpace, Point3, Vector2, Vector3};
use wgpu::BlendFactor;

use crate::graphics::{Color, ScreenSize, Texture};
use crate::loaders::{ImageType, TextureLoader};
use crate::renderer::EffectRenderer;
use crate::settings::AuraStyle;
use crate::world::Camera;
use crate::world::effect::{EffectBase, EffectCenter, render_world_quad, render_world_quad_blended};

/// Minimum base level for the classic aura.
pub const AURA_MINIMUM_LEVEL: usize = 99;
/// Base radius of the ground rings in world units.
const AURA_RADIUS: f32 = 13.0;
/// Height of the ground rings above the floor (avoids z-fighting).
const RING_HEIGHT: f32 = 0.25;
/// Number of floating sparkles.
const SPARKLE_COUNT: usize = 6;
/// Radians per second at which the outer ring rotates.
const RING_ROTATION_SPEED: f32 = 0.7;
/// Radians per second at which the inner ring rotates (opposite direction).
const INNER_RING_ROTATION_SPEED: f32 = -1.1;

/// The original client (and roBrowser) tick rate: 25 ms per animation frame.
const OFFICIAL_TICK_SECONDS: f32 = 0.025;
/// The original client runs effects at 40 frames per second.
const OFFICIAL_FRAMES_PER_SECOND: f32 = 1.0 / OFFICIAL_TICK_SECONDS;

/// Official client units to this client's world units (1 map cell = 10 units).
const OFFICIAL_TO_WORLD: f32 = 0.22 * 10.0;

/// Number of divisions of a swirling band (0..=20).
const SWIRL_DIVISIONS: usize = 21;
/// The swirling band covers 315 degrees instead of a full circle.
const SWIRL_ARC_DEGREES: f32 = 315.0;
/// Alpha of the official swirling bands (120/255).
const SWIRL_ALPHA: f32 = 120.0 / 255.0;
/// Global opacity boost for the blended aura components. The official values are
/// tuned for the original renderer; in bright outdoor maps they read as barely
/// visible, so the blended parts are drawn noticeably stronger.
const AURA_OPACITY_SCALE: f32 = 1.7;
/// The official bands are drawn slightly inside the ground aura.
const SWIRL_INNER_CIRCLE_SCALE: f32 = 0.6;

/// `GroundAura` size argument from the official effect table.
const GROUND_AURA_SIZE: f32 = 100.0;
/// `GroundAura` distance argument from the official effect table.
const GROUND_AURA_DISTANCE: f32 = 15.0;
/// SpriteRenderer conversion used by the official `GroundAura` shader:
/// `size / 35.0` world units per texture unit (cell).
const GROUND_AURA_WORLD_SCALE: f32 = 10.0 / 35.0;

/// Official `Level99Bubble` emitter columns.
const BUBBLE_COLUMNS: usize = 4;
/// Official `Level99Bubble` anchors per emitter column.
const BUBBLE_ANCHORS: usize = 4;
/// Billboard radius of the bubble sparkles in official units.
const BUBBLE_RADIUS: f32 = 2.4;
/// Fall speed of the bubble anchors in official units per frame.
const BUBBLE_FALL_SPEED: f32 = 0.15;
/// Horizontal jitter amplitude of the bubble anchors.
const BUBBLE_DRIFT: f32 = 0.15;
/// Highest spawn position of a bubble anchor (official units).
const BUBBLE_SEED_MAX: f32 = 99.0;
/// A bubble anchor is reset when it falls below this value.
const BUBBLE_RESET_Y: f32 = -30.0;
/// Alpha formula constants: `clamp(250 + gain * (y + offset), 0, 250)`.
const BUBBLE_ALPHA_OFFSET: f32 = 20.0;
const BUBBLE_ALPHA_GAIN: f32 = 30.0;

/// Texture names verified to exist in the CondaRO data set.
const RING_LINER_TEXTURE: &str = "effect\\ring_liner.tga";
const RING_BLUE_TEXTURE: &str = "effect\\ring_blue.tga";
const RING_RED_TEXTURE: &str = "effect\\ring_red.tga";
const BUBBLE_TEXTURE: &str = "effect\\whitelight.tga";
const GLOW_TEXTURES: [&str; 4] = [
    "effect\\glow01.bmp",
    "effect\\glow02.bmp",
    "effect\\glow03.bmp",
    "effect\\glow04.bmp",
];
const SPARKLE_TEXTURE: &str = "effect\\pikapika2.bmp";

/// One of the three official swirling aura bands.
#[derive(Clone, Copy)]
struct SwirlBand {
    /// Starting rotation of the band in degrees (0, 90, 180).
    initial_rot_start: f32,
    /// Additional rotation in degrees per 25 ms frame (3, 4, 5).
    spin_speed: f32,
    /// Maximum height of the band profile in official units (15, 13, 11).
    max_height: f32,
    /// Radius of the band in official units (3.9, 4.1, 4.3).
    distance: f32,
    /// Rise angle of the band in radians (55°, 50°, 45°).
    rise_angle: f32,
}

/// The three official `SwirlingAura` bands with their original parameters.
fn swirl_bands() -> [SwirlBand; 3] {
    [
        SwirlBand {
            initial_rot_start: 0.0,
            spin_speed: 3.0,
            max_height: 15.0,
            distance: 3.9,
            // 55 degrees
            rise_angle: 0.959_931,
        },
        SwirlBand {
            initial_rot_start: 90.0,
            spin_speed: 4.0,
            max_height: 13.0,
            distance: 4.1,
            // 50 degrees
            rise_angle: 0.872_665,
        },
        SwirlBand {
            initial_rot_start: 180.0,
            spin_speed: 5.0,
            max_height: 11.0,
            distance: 4.3,
            // 45 degrees
            rise_angle: std::f32::consts::FRAC_PI_4,
        },
    ]
}

/// One of the two flat official `GroundAura` quads.
struct GroundAuraQuad {
    /// Current size in official `GroundAura` units.
    size: Vector2<f32>,
    /// Size the quad oscillates around.
    initial_size: Vector2<f32>,
    /// Rotation phase in degrees, advanced by 3 per tick.
    rise_angle: f32,
    /// Current growth direction (1 or -1).
    direction: f32,
    /// Fixed rotation of the quad around the vertical axis.
    angle_offset: f32,
}

/// A falling anchor of one official `Level99Bubble` emitter column.
struct BubbleAnchor {
    x: f32,
    y: f32,
    z: f32,
}

/// One emitter column of the official `Level99Bubble` effect.
struct BubbleColumn {
    phases: [f32; 16],
    phase_targets: [f32; 16],
    anchors: [BubbleAnchor; BUBBLE_ANCHORS],
}

/// Small deterministic PRNG so the bubble effect does not need the global
/// random generator and stays independent for every aura.
struct AuraRandom(u32);

impl AuraRandom {
    fn new(seed: u32) -> Self {
        Self(if seed == 0 { 0x9E37_79B9 } else { seed })
    }

    fn next_u32(&mut self) -> u32 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 17;
        value ^= value << 5;
        self.0 = value;
        value
    }

    fn unit(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }

    fn range(&mut self, minimum: f32, maximum: f32) -> f32 {
        minimum + self.unit() * (maximum - minimum)
    }
}

/// Classic tier detection for a base level (used by [`AuraStyle::Auto`]).
pub fn auto_aura_style(level: usize, transcendent: bool) -> AuraStyle {
    if level >= 175 {
        AuraStyle::Gold
    } else if level >= 100 {
        AuraStyle::Blue
    } else if transcendent {
        AuraStyle::Violet
    } else {
        AuraStyle::Cyan
    }
}

/// Global size multiplier applied on top of the tier scale. The official sizes
/// read too large on our camera, so the whole aura is scaled down by 30%.
const AURA_SIZE_SCALE: f32 = 0.7;

/// Overall size multiplier of the aura for a base level (higher levels get the
/// larger classic aura).
pub fn aura_tier_scale(level: usize) -> f32 {
    if level >= 175 {
        1.45
    } else if level >= 150 {
        1.3
    } else if level >= 100 {
        1.15
    } else {
        1.0
    }
}

/// Color of the aura for the given (explicit) style.
pub fn aura_color(style: AuraStyle) -> Color {
    match style {
        AuraStyle::Cyan | AuraStyle::Auto | AuraStyle::Off => Color::rgb_u8(40, 200, 255),
        AuraStyle::Violet => Color::rgb_u8(165, 70, 255),
        AuraStyle::Blue => Color::rgb_u8(45, 100, 255),
        AuraStyle::Gold => Color::rgb_u8(255, 185, 40),
        // Official EF_LEVEL99 size-type 7 green tint (100, 255, 100).
        AuraStyle::Green => Color::rgb_u8(100, 255, 100),
    }
}

/// The level-99 aura effect of one entity.
pub struct Aura {
    center: EffectCenter,
    style_cell: Arc<AtomicU8>,
    level: usize,
    transcendent: bool,
    ring_liner: Option<Arc<Texture>>,
    ring_blue: Option<Arc<Texture>>,
    ring_red: Option<Arc<Texture>>,
    glows: Vec<Arc<Texture>>,
    sparkle: Option<Arc<Texture>>,
    bubble: Option<Arc<Texture>>,
    bands: [SwirlBand; 3],
    ground_quads: [GroundAuraQuad; 2],
    bubble_columns: Vec<BubbleColumn>,
    random: AuraRandom,
    time: f32,
    gets_deleted: bool,
}

impl Aura {
    /// Create the aura for an entity with the given level.
    ///
    /// Returns `None` if none of the required effect textures exist in the game
    /// data (graceful fallback for trimmed data sets).
    pub fn new(style_cell: Arc<AtomicU8>, level: usize, transcendent: bool, texture_loader: &TextureLoader) -> Option<Self> {
        let load = |name: &str| {
            let texture = texture_loader.get_or_load(name, ImageType::Color).ok();

            if texture.is_none() {
                crate::logging::log_warn!(
                    "[aura] effect texture '{}' is missing from the game data, skipping that component",
                    name
                );
            }

            texture
        };

        let ring_liner = load(RING_LINER_TEXTURE);
        let ring_blue = load(RING_BLUE_TEXTURE);
        let ring_red = load(RING_RED_TEXTURE);
        let glows = GLOW_TEXTURES.iter().filter_map(|name| load(name)).collect::<Vec<_>>();
        let sparkle = load(SPARKLE_TEXTURE);
        let bubble = load(BUBBLE_TEXTURE);

        if ring_liner.is_none() && ring_blue.is_none() && ring_red.is_none() && glows.is_empty() && sparkle.is_none() && bubble.is_none() {
            return None;
        }

        let mut random = AuraRandom::new(level as u32 ^ (transcendent as u32) << 16 ^ 0xA17A_0001);
        let mut bubble_columns = Vec::with_capacity(BUBBLE_COLUMNS);

        for _ in 0..BUBBLE_COLUMNS {
            let mut column = BubbleColumn {
                phases: [0.0; 16],
                phase_targets: [0.0; 16],
                anchors: [
                    BubbleAnchor { x: 0.0, y: 0.0, z: 0.0 },
                    BubbleAnchor { x: 0.0, y: 0.0, z: 0.0 },
                    BubbleAnchor { x: 0.0, y: 0.0, z: 0.0 },
                    BubbleAnchor { x: 0.0, y: 0.0, z: 0.0 },
                ],
            };

            for phase in column.phases.iter_mut() {
                *phase = random.range(0.0, 360.0);
            }

            for target in column.phase_targets.iter_mut() {
                *target = random.range(0.0, 360.0);
            }

            for anchor in column.anchors.iter_mut() {
                anchor.y = random.range(0.0, BUBBLE_SEED_MAX);
            }

            bubble_columns.push(column);
        }

        let ground_quads = [
            GroundAuraQuad {
                size: Vector2::new(GROUND_AURA_SIZE + GROUND_AURA_DISTANCE, GROUND_AURA_SIZE + GROUND_AURA_DISTANCE),
                initial_size: Vector2::new(GROUND_AURA_SIZE + GROUND_AURA_DISTANCE, GROUND_AURA_SIZE + GROUND_AURA_DISTANCE),
                rise_angle: 0.0,
                direction: 1.0,
                // 0 degrees * 23
                angle_offset: 0.0,
            },
            GroundAuraQuad {
                size: Vector2::new(
                    GROUND_AURA_SIZE + GROUND_AURA_DISTANCE * 2.0,
                    GROUND_AURA_SIZE + GROUND_AURA_DISTANCE * 2.0,
                ),
                initial_size: Vector2::new(
                    GROUND_AURA_SIZE + GROUND_AURA_DISTANCE * 2.0,
                    GROUND_AURA_SIZE + GROUND_AURA_DISTANCE * 2.0,
                ),
                rise_angle: 0.0,
                direction: -1.0,
                // 1 * 23 degrees
                angle_offset: 23.0_f32.to_radians(),
            },
        ];

        Some(Self {
            // The actual position is filled on the first update.
            center: EffectCenter::Position(Point3::new(0.0, 0.0, 0.0)),
            style_cell,
            level,
            transcendent,
            ring_liner,
            ring_blue,
            ring_red,
            glows,
            sparkle,
            bubble,
            bands: swirl_bands(),
            ground_quads,
            bubble_columns,
            random,
            time: 0.0,
            gets_deleted: false,
        })
    }

    /// Attach the aura to an entity (the aura follows it from now on).
    pub fn attach_to_entity(&mut self, entity_id: ragnarok_packets::EntityId, position: Point3<f32>) {
        self.center = EffectCenter::Entity(entity_id, position);
    }

    fn configured_style(&self) -> AuraStyle {
        AuraStyle::decode(self.style_cell.load(Ordering::Relaxed))
    }

    /// The style actually used for rendering (Auto resolves to the tier).
    pub fn resolved_style(&self) -> AuraStyle {
        match self.configured_style() {
            AuraStyle::Auto => auto_aura_style(self.level, self.transcendent),
            style => style,
        }
    }

    /// The inner ring texture for the current style, with a fallback.
    fn inner_ring_texture(&self, style: AuraStyle) -> Option<Arc<Texture>> {
        let preferred = match style {
            AuraStyle::Blue | AuraStyle::Green => self.ring_blue.as_ref(),
            AuraStyle::Gold => self.ring_red.as_ref(),
            _ => None,
        };

        preferred.or(self.ring_liner.as_ref()).or(self.ring_blue.as_ref()).cloned()
    }

    /// The texture of the official swirling bands: `ring_blue.tga`, like the
    /// original client (with the remaining rings as a fallback).
    fn swirl_texture(&self, style: AuraStyle) -> Option<Arc<Texture>> {
        match style {
            AuraStyle::Gold => self.ring_red.as_ref(),
            _ => self.ring_blue.as_ref(),
        }
        .or(self.ring_liner.as_ref())
        .or(self.ring_blue.as_ref())
        .cloned()
    }

    fn glow_texture(&self, style: AuraStyle) -> Option<Arc<Texture>> {
        if self.glows.is_empty() {
            return None;
        }

        let index = match style {
            AuraStyle::Cyan | AuraStyle::Auto | AuraStyle::Off => 0,
            AuraStyle::Violet => 3,
            AuraStyle::Blue => 2,
            AuraStyle::Gold => 1,
            AuraStyle::Green => 0,
        };

        self.glows.get(index).or_else(|| self.glows.first()).cloned()
    }

    /// Corners of a horizontal quad at the given height above the anchor.
    fn ground_quad(anchor: Point3<f32>, radius: f32, angle: f32, height: f32) -> [Point3<f32>; 4] {
        let directions = [
            Vector3::new(-1.0, 0.0, -1.0),
            Vector3::new(1.0, 0.0, -1.0),
            Vector3::new(-1.0, 0.0, 1.0),
            Vector3::new(1.0, 0.0, 1.0),
        ];

        directions.map(|direction| {
            let x = direction.x * angle.cos() - direction.z * angle.sin();
            let z = direction.x * angle.sin() + direction.z * angle.cos();

            anchor + Vector3::new(x * radius, height, z * radius)
        })
    }

    /// Corners of an arbitrarily sized and rotated horizontal rectangle.
    fn ground_rectangle(anchor: Point3<f32>, size: Vector2<f32>, angle: f32, height: f32) -> [Point3<f32>; 4] {
        let half = size * 0.5;
        let directions = [
            Vector2::new(-half.x, -half.y),
            Vector2::new(half.x, -half.y),
            Vector2::new(-half.x, half.y),
            Vector2::new(half.x, half.y),
        ];

        directions.map(|direction| {
            let x = direction.x * angle.cos() - direction.y * angle.sin();
            let z = direction.x * angle.sin() + direction.y * angle.cos();

            anchor + Vector3::new(x, height, z)
        })
    }

    fn quad_texture_coordinates() -> [Vector2<f32>; 4] {
        [
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(0.0, 1.0),
            Vector2::new(1.0, 1.0),
        ]
    }

    /// One official 25 ms tick: advances the ground aura quads and bubble
    /// anchors exactly like the original client.
    fn step_official_tick(&mut self) {
        let ticks = 1.0;

        // GroundAura: the two flat quads slowly grow/shrink while rotating
        // their growth phase by 3 degrees per tick.
        for quad in self.ground_quads.iter_mut() {
            quad.rise_angle += 3.0 * ticks;

            if quad.rise_angle >= 360.0 {
                quad.rise_angle -= 360.0;
            }

            if quad.rise_angle > 0.0 && (quad.rise_angle.round() as i32) % 180 == 0 {
                quad.direction = -quad.direction;

                if (quad.direction < 0.0 && quad.size.x < quad.initial_size.x)
                    || (quad.direction > 0.0 && quad.size.x > quad.initial_size.x)
                {
                    quad.size = quad.initial_size;
                }
            }

            let (size_x, size_y) = ground_aura_size_modifier(quad.rise_angle, GROUND_AURA_DISTANCE);
            let half_size = GROUND_AURA_SIZE / 2.0;
            quad.size.x += size_x * quad.direction / half_size;
            quad.size.y += size_y * quad.direction / half_size;
        }

        // Level99Bubble: advance the jitter phases and the falling anchors.
        for column in self.bubble_columns.iter_mut() {
            for index in 0..column.phases.len() {
                let (angle, target) = advance_phase(column.phases[index], column.phase_targets[index], ticks, &mut self.random);
                column.phases[index] = angle;
                column.phase_targets[index] = target;
            }

            for anchor_index in 0..BUBBLE_ANCHORS {
                let signs = BUBBLE_ANCHOR_SIGNS[anchor_index];
                let phases = BUBBLE_ANCHOR_PHASES[anchor_index];
                let anchor = &mut column.anchors[anchor_index];

                if anchor.y < 0.0 {
                    let phase_a = column.phases[phases.0].to_radians();
                    let phase_b = column.phases[phases.1].to_radians();
                    anchor.x += signs.0 * BUBBLE_DRIFT * phase_a.sin() * ticks;
                    anchor.z += signs.1 * BUBBLE_DRIFT * phase_b.sin() * ticks;
                }

                anchor.y -= BUBBLE_FALL_SPEED * ticks;

                if anchor.y < BUBBLE_RESET_Y {
                    anchor.x = 0.0;
                    anchor.z = 0.0;
                    anchor.y = self.random.range(0.0, BUBBLE_SEED_MAX);

                    column.phases[phases.0] = self.random.range(0.0, 360.0);
                    column.phase_targets[phases.0] = self.random.range(0.0, 360.0);
                    column.phases[phases.1] = self.random.range(0.0, 360.0);
                    column.phase_targets[phases.1] = self.random.range(0.0, 360.0);
                }
            }
        }
    }

    /// The `SwirlingAura` bands: three tilted ribbons that rise around the
    /// character and spin at different speeds (official parameter set).
    fn render_swirl_bands(
        &self,
        renderer: &mut EffectRenderer,
        camera: &dyn Camera,
        screen_size: ScreenSize,
        anchor: Point3<f32>,
        color: Color,
        scale: f32,
    ) {
        let Some(texture) = self.swirl_texture(self.resolved_style()) else {
            return;
        };

        let process = self.time * OFFICIAL_FRAMES_PER_SECOND;
        if process <= 0.0 {
            return;
        }

        let basic_angle = (SWIRL_ARC_DEGREES / (SWIRL_DIVISIONS - 1) as f32).to_radians();
        let wave = if process <= 90.0 { process.to_radians().sin() } else { 1.0 };
        let tint = Color::rgba(
            color.red,
            color.green,
            color.blue,
            (SWIRL_ALPHA * 1.25 * AURA_OPACITY_SCALE).min(1.0),
        );

        for band in self.bands {
            let rotation_start = (band.initial_rot_start + process * band.spin_speed).to_radians();
            let distance = band.distance * OFFICIAL_TO_WORLD * SWIRL_INNER_CIRCLE_SCALE * scale;
            let max_height = band.max_height * OFFICIAL_TO_WORLD * scale;
            let cos_rise = band.rise_angle.cos();
            let sin_rise = band.rise_angle.sin();

            let band_point = |index: usize| {
                let angle = rotation_start + index as f32 * basic_angle;
                let sin_limit = (90.0 + (index as f32 - 10.0) * 9.0).to_radians().sin();
                let height = (max_height * sin_limit * wave).max(0.0);
                let radial = cos_rise * height;
                let base = anchor + Vector3::new(distance * angle.cos(), 0.0, distance * angle.sin());
                let top = base + Vector3::new(radial * angle.cos(), sin_rise * height, radial * angle.sin());

                (base, top)
            };

            for division in 0..SWIRL_DIVISIONS - 1 {
                let (base_start, top_start) = band_point(division);
                let (base_end, top_end) = band_point(division + 1);
                let u_start = division as f32 / (SWIRL_DIVISIONS - 1) as f32;
                let u_end = (division + 1) as f32 / (SWIRL_DIVISIONS - 1) as f32;

                let corners = [base_start, top_start, base_end, top_end];
                let texture_coordinates = [
                    Vector2::new(u_start, 1.0),
                    Vector2::new(u_start, 0.0),
                    Vector2::new(u_end, 1.0),
                    Vector2::new(u_end, 0.0),
                ];

                render_world_quad(
                    renderer,
                    camera,
                    anchor,
                    screen_size,
                    texture.clone(),
                    tint,
                    corners,
                    texture_coordinates,
                );
            }
        }
    }

    /// The official `GroundAura`: two flat, pulsing textured quads on the
    /// ground below the character.
    fn render_ground_aura(
        &self,
        renderer: &mut EffectRenderer,
        camera: &dyn Camera,
        screen_size: ScreenSize,
        anchor: Point3<f32>,
        scale: f32,
    ) {
        let Some(texture) = self.sparkle.clone() else {
            return;
        };

        let texture_coordinates = Self::quad_texture_coordinates();

        for quad in &self.ground_quads {
            let size = quad.size * (GROUND_AURA_WORLD_SCALE * scale);
            // The official effect draws the ground aura slightly above the
            // floor to avoid z-fighting with the map.
            let height = RING_HEIGHT - 0.12;
            let corners = Self::ground_rectangle(anchor, Vector2::new(size.x, size.y), quad.angle_offset, height);

            render_world_quad_blended(
                renderer,
                camera,
                anchor,
                screen_size,
                texture.clone(),
                Color::rgba(0.85, 0.92, 1.0, (0.16 * AURA_OPACITY_SCALE).min(0.9)),
                corners,
                texture_coordinates,
                BlendFactor::SrcAlpha,
                BlendFactor::One,
            );
        }
    }

    /// The official `Level99Bubble` sparkles: camera facing `whitelight.tga`
    /// quads around the character (4 emitter columns, 4 anchors each).
    fn render_bubbles(
        &self,
        renderer: &mut EffectRenderer,
        camera: &dyn Camera,
        screen_size: ScreenSize,
        anchor: Point3<f32>,
        color: Color,
        scale: f32,
    ) {
        let Some(texture) = self.bubble.clone() else {
            return;
        };

        let view_direction = camera.view_direction();
        let look_up = camera.look_up_vector();
        let right = look_up.cross(view_direction).normalize();
        let up = view_direction.cross(right).normalize();
        let radius = BUBBLE_RADIUS * OFFICIAL_TO_WORLD * SWIRL_INNER_CIRCLE_SCALE * scale;
        let texture_coordinates = Self::quad_texture_coordinates();

        for column in &self.bubble_columns {
            for anchor_data in &column.anchors {
                // Only anchors that already fell below the ground are visible.
                if anchor_data.y > 0.0 {
                    continue;
                }

                let alpha = bubble_alpha(anchor_data.y);
                if alpha <= 0.0 {
                    continue;
                }

                let center = anchor
                    + Vector3::new(
                        anchor_data.x * OFFICIAL_TO_WORLD,
                        -anchor_data.y * OFFICIAL_TO_WORLD,
                        anchor_data.z * OFFICIAL_TO_WORLD,
                    );
                let corners = [
                    center - right * radius - up * radius,
                    center + right * radius - up * radius,
                    center - right * radius + up * radius,
                    center + right * radius + up * radius,
                ];
                let tint = Color::rgba(color.red * alpha, color.green * alpha, color.blue * alpha, 1.0);

                render_world_quad(
                    renderer,
                    camera,
                    anchor,
                    screen_size,
                    texture.clone(),
                    tint,
                    corners,
                    texture_coordinates,
                );
            }
        }
    }
}

/// Offsets of the four anchors inside an emitter column:
/// B_pre (+kx,+kz), B_now (-kx,-kz), T_pre (+kx,-kz), T_now (-kx,+kz).
const BUBBLE_ANCHOR_SIGNS: [(f32, f32); BUBBLE_ANCHORS] = [(1.0, 1.0), (-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0)];
/// Phase pairs of the four anchors inside an emitter column.
const BUBBLE_ANCHOR_PHASES: [(usize, usize); BUBBLE_ANCHORS] = [(0, 2), (4, 6), (8, 10), (12, 14)];

/// Advances one bubble jitter phase toward its random target, returning the new
/// phase and target (the target is re-seeded when it is reached).
fn advance_phase(current: f32, target: f32, step_scale: f32, random: &mut AuraRandom) -> (f32, f32) {
    let mut difference = target - current;
    difference = ((difference + 540.0) % 360.0) - 180.0;
    let step = (2.0 + random.unit()) * step_scale;

    if difference.abs() <= step {
        (target, random.range(0.0, 360.0))
    } else {
        let mut angle = current + difference.signum() * step;
        angle %= 360.0;
        if angle < 0.0 {
            angle += 360.0;
        }

        (angle, target)
    }
}

/// The official `GroundAura` size modifier for one 25 ms tick: the quads pulse
/// based on the current rise angle.
fn ground_aura_size_modifier(rise_angle_degrees: f32, distance: f32) -> (f32, f32) {
    let rise_factor = distance * 0.1 * (rise_angle_degrees.to_radians().sin() + 1.0);
    let angle = rise_angle_degrees.to_radians();
    let radius = distance * 0.8 + rise_factor;
    let start_x = angle.cos() * radius;
    let end_x = (angle + 90.0_f32.to_radians()).cos() * radius;
    let start_y = (angle + 180.0_f32.to_radians()).sin() * radius;
    let end_y = (angle + 270.0_f32.to_radians()).sin() * radius;

    ((end_x - start_x).abs(), (end_y - start_y).abs())
}

/// Alpha of a bubble anchor based on its official height value.
fn bubble_alpha(y: f32) -> f32 {
    (250.0 + BUBBLE_ALPHA_GAIN * (y + BUBBLE_ALPHA_OFFSET)).clamp(0.0, 250.0) / 255.0
}

impl EffectBase for Aura {
    fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) -> bool {
        if self.gets_deleted {
            return false;
        }

        // Off: the aura removes itself, the client will not spawn a new one
        // while the setting stays Off.
        if self.configured_style() == AuraStyle::Off {
            return false;
        }

        let Some(entity_id) = (match &self.center {
            EffectCenter::Entity(entity_id, _) => Some(*entity_id),
            EffectCenter::Position(_) => None,
        }) else {
            return false;
        };

        let Some(entity) = entities.iter().find(|entity| entity.get_entity_id() == entity_id) else {
            // The entity despawned (map change, distance, death cleanup).
            return false;
        };

        if entity.is_dead() {
            return false;
        }

        let Some(level) = entity.get_base_level() else {
            // Level unknown for this entity (e.g. population shell without a
            // level packet) — fall back gracefully and do not render an aura.
            return false;
        };

        if level < AURA_MINIMUM_LEVEL {
            // The level dropped below the aura threshold (or the entity was
            // re-created with a lower level).
            return false;
        }

        self.level = level;
        self.transcendent = entity.is_transcendent_job();
        self.center.follow_entity(entities);
        self.time += delta_time;

        // The official components are advanced at the original client's 25 ms
        // tick rate instead of the frame rate, so their look does not depend on
        // the client frame rate. Cap the amount of catch-up ticks after a long
        // stall (the original client caps this at 250 ms as well).
        let ticks = ((delta_time * OFFICIAL_FRAMES_PER_SECOND).clamp(0.0, 10.0)).floor() as usize;
        for _ in 0..ticks {
            self.step_official_tick();
        }

        true
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, _: &mut crate::world::PointLightManager, _: &dyn Camera) {}

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        if self.gets_deleted {
            return;
        }

        let style = self.resolved_style();

        if style == AuraStyle::Off {
            return;
        }

        let anchor = self.center.to_position();
        let screen_size = ScreenSize {
            width: renderer.window_size().width,
            height: renderer.window_size().height,
        };

        let color = aura_color(style);
        let scale = aura_tier_scale(self.level) * AURA_SIZE_SCALE;
        let pulse = 0.8 + 0.2 * (self.time * 2.3).sin();
        let radius = AURA_RADIUS * scale * (1.0 + 0.035 * (self.time * 1.8).sin());
        let texture_coordinates = Self::quad_texture_coordinates();

        // Official GroundAura: two pulsing flat quads on the ground.
        self.render_ground_aura(renderer, camera, screen_size, anchor, scale);

        // Outer rotating ring.
        if let Some(texture) = self.ring_liner.clone() {
            render_world_quad(
                renderer,
                camera,
                anchor,
                screen_size,
                texture,
                Color::rgba(
                    color.red * 0.6 * pulse,
                    color.green * 0.6 * pulse,
                    color.blue * 0.6 * pulse,
                    1.0,
                ),
                Self::ground_quad(anchor, radius, self.time * RING_ROTATION_SPEED, RING_HEIGHT),
                texture_coordinates,
            );
        }

        // Inner counter-rotating ring, slightly smaller and brighter.
        if let Some(texture) = self.inner_ring_texture(style) {
            render_world_quad(
                renderer,
                camera,
                anchor,
                screen_size,
                texture,
                Color::rgba(color.red * 0.95 * pulse, color.green * 0.95 * pulse, color.blue * pulse, 1.0),
                Self::ground_quad(anchor, radius * 0.62, self.time * INNER_RING_ROTATION_SPEED, RING_HEIGHT + 0.05),
                texture_coordinates,
            );
        }

        // Soft ground glow that pulses with the aura.
        if let Some(texture) = self.glow_texture(style) {
            let glow = 0.10 + 0.10 * (self.time * 1.6).sin().max(0.0);
            render_world_quad(
                renderer,
                camera,
                anchor,
                screen_size,
                texture,
                Color::rgba(
                    (color.red * glow).min(1.0),
                    (color.green * glow).min(1.0),
                    (color.blue * glow).min(1.0),
                    1.0,
                ),
                Self::ground_quad(anchor, radius * 1.15, self.time * 0.2, RING_HEIGHT - 0.05),
                texture_coordinates,
            );
        }

        // Floating sparkles that rise, sink and slowly orbit around the feet.
        if let Some(texture) = self.sparkle.clone() {
            for index in 0..SPARKLE_COUNT {
                let phase = index as f32 * std::f32::consts::TAU / SPARKLE_COUNT as f32;
                let angle = phase + self.time * 0.35;
                let sparkle_radius = radius * (0.5 + 0.18 * (self.time * 1.7 + phase).sin().abs());
                let height = 2.0 + 5.0 * (0.5 + 0.5 * (self.time * 1.3 + phase * 1.7).sin());

                let sparkle_center = anchor + Vector3::new(angle.cos() * sparkle_radius, height, angle.sin() * sparkle_radius);
                let sparkle_size = (0.9 + 0.5 * (self.time * 2.1 + phase).sin().abs()) * scale;
                let sparkle_alpha = 0.55 + 0.45 * (self.time * 3.0 + phase * 2.3).sin().abs();

                render_world_quad(
                    renderer,
                    camera,
                    anchor,
                    screen_size,
                    texture.clone(),
                    Color::rgba(
                        (color.red * 0.7 + 0.3) * sparkle_alpha,
                        (color.green * 0.7 + 0.3) * sparkle_alpha,
                        (color.blue * 0.7 + 0.3) * sparkle_alpha,
                        1.0,
                    ),
                    Self::ground_quad(sparkle_center, sparkle_size, angle, 0.0),
                    texture_coordinates,
                );
            }
        }

        // Official SwirlingAura: three tilted, spinning bands rising around the
        // character.
        self.render_swirl_bands(renderer, camera, screen_size, anchor, color, scale);

        // Official Level99Bubble: camera facing whitelight sparkles.
        self.render_bubbles(renderer, camera, screen_size, anchor, color, scale);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_tiers_follow_the_classic_rules() {
        assert_eq!(auto_aura_style(99, false), AuraStyle::Cyan);
        assert_eq!(auto_aura_style(99, true), AuraStyle::Violet);
        assert_eq!(auto_aura_style(100, false), AuraStyle::Blue);
        assert_eq!(auto_aura_style(174, false), AuraStyle::Blue);
        assert_eq!(auto_aura_style(175, false), AuraStyle::Gold);
        assert_eq!(auto_aura_style(250, false), AuraStyle::Gold);
    }

    #[test]
    fn aura_styles_have_distinct_colors() {
        let colors = [
            AuraStyle::Cyan,
            AuraStyle::Violet,
            AuraStyle::Blue,
            AuraStyle::Gold,
            AuraStyle::Green,
        ]
        .map(aura_color);

        for (index, color) in colors.iter().enumerate() {
            for other in &colors[index + 1..] {
                assert_ne!(color, other);
            }
        }
    }

    #[test]
    fn green_style_uses_the_official_green_tint() {
        let green = aura_color(AuraStyle::Green);
        let blue = aura_color(AuraStyle::Blue);

        assert!(green.green >= green.red && green.green >= green.blue);
        assert!(blue.blue > blue.red && blue.blue > blue.green);
    }

    #[test]
    fn official_swirl_bands_match_the_reference_parameters() {
        let bands = swirl_bands();

        assert_eq!(bands.len(), 3);
        assert_eq!(bands[0].initial_rot_start, 0.0);
        assert_eq!(bands[1].initial_rot_start, 90.0);
        assert_eq!(bands[2].initial_rot_start, 180.0);
        assert_eq!(bands[0].spin_speed, 3.0);
        assert_eq!(bands[1].spin_speed, 4.0);
        assert_eq!(bands[2].spin_speed, 5.0);
        assert_eq!(bands[0].max_height, 15.0);
        assert_eq!(bands[1].max_height, 13.0);
        assert_eq!(bands[2].max_height, 11.0);
        // The bands get slightly wider and flatter from the inside out.
        assert!(bands[1].distance > bands[0].distance);
        assert!(bands[2].distance > bands[1].distance);
        assert!(bands[1].rise_angle < bands[0].rise_angle);
        assert!(bands[2].rise_angle < bands[1].rise_angle);
    }

    #[test]
    fn official_swirl_arc_and_divisions_match_the_reference() {
        assert_eq!(SWIRL_DIVISIONS, 21);
        assert_eq!(SWIRL_ARC_DEGREES, 315.0);
        // The last division ends exactly at the end of the 315 degree arc.
        let basic_angle = SWIRL_ARC_DEGREES / (SWIRL_DIVISIONS - 1) as f32;
        assert!((basic_angle * (SWIRL_DIVISIONS - 1) as f32 - SWIRL_ARC_DEGREES).abs() < f32::EPSILON);
    }

    #[test]
    fn official_ground_aura_sizes_match_the_reference() {
        let minimum = GROUND_AURA_SIZE + GROUND_AURA_DISTANCE;
        let maximum = GROUND_AURA_SIZE + GROUND_AURA_DISTANCE * 2.0;

        assert_eq!(GROUND_AURA_SIZE, 100.0);
        assert_eq!(GROUND_AURA_DISTANCE, 15.0);
        assert_eq!(minimum, 115.0);
        assert_eq!(maximum, 130.0);
    }

    #[test]
    fn official_bubble_alpha_fades_out_below_the_reset_threshold() {
        // Full alpha at and above the ground, fading out while falling below.
        assert_eq!(bubble_alpha(0.0), 250.0 / 255.0);
        assert_eq!(bubble_alpha(-20.0), 250.0 / 255.0);
        assert!(bubble_alpha(-25.0) < bubble_alpha(-20.0));
        assert!(bubble_alpha(-28.0) > 0.0);
        assert_eq!(bubble_alpha(-28.4), 0.0);
        assert_eq!(bubble_alpha(BUBBLE_RESET_Y), 0.0);
    }

    #[test]
    fn higher_levels_get_larger_auras() {
        assert!(aura_tier_scale(100) > aura_tier_scale(99));
        assert!(aura_tier_scale(150) > aura_tier_scale(100));
        assert!(aura_tier_scale(175) > aura_tier_scale(150));
    }

    #[test]
    fn aura_style_round_trips_through_the_atomic_cell() {
        for style in [
            AuraStyle::Auto,
            AuraStyle::Off,
            AuraStyle::Cyan,
            AuraStyle::Violet,
            AuraStyle::Blue,
            AuraStyle::Gold,
            AuraStyle::Green,
        ] {
            assert_eq!(AuraStyle::decode(style.encode()), style);
        }

        // Invalid values fall back to Auto.
        assert_eq!(AuraStyle::decode(255), AuraStyle::Auto);
    }
}
