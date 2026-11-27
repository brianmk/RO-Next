//! Renderers for the official client's **slot/record** effect primitives.
//!
//! These are the primitives whose geometry is built from the four
//! `EffectPrimitiveSlot`s (`slots[0..4]`, 21 height samples each) or from the
//! segment `records[]`, and which the official client submits as
//! `D3DPT_TRIANGLELIST` triangles through `CRenderer::SubmitTriangle`. They are
//! the reason the port needs a triangle path: a ring is a strip of independent
//! vertices with a height per angle, so no quad can represent it.
//!
//! Every function here is a line-by-line port of one `CEffectPrim::Render*`
//! member (`Source/Contents/Effects/Primitives/EffectPrimitiveRendering.cpp`
//! and `EffectPrimitiveSweeps.cpp`/`EffectPrimitiveArcs.cpp`/
//! `EffectPrimitiveGlowSquares.cpp`), dispatched by `CEffectPrim::Render`'s
//! `switch (state.count)`:
//!
//! | id | official renderer | used by |
//! | --- | --- | --- |
//! | 60, 63 | `Render3DCasting` | heal pillar/ring, portals, magic ring, ripple rings |
//! | 24 | `Render3DCasting` | `QuadRingStack` - `EF_DEFENDER`, `EF_REFLECTSHIELD` |
//! | 30, 31 | `Render3DCasting` | `TexturedQuadRingSized` rings (`EF_AURABLADE`, `EF_BEGINASURA`), `EF_GUMGANG2` |
//! | 35 | `RenderOrbitIcons` | `SuperSlashFan` - `EF_GUMGANG`, `EF_STEELBODY` |
//! | 68 | `RenderHaloLayers` | `AsuraStrike`/`TexturedSkillIcon` - `EF_BEGINASURA`, 1..7, 11 |
//! | 99 | `RenderDoubleGumGang` | `SuperSlashSpread` - `EF_DOUBLEGUMGANG`, 2, 3 |
//! | 147 | `RenderHaloLayers` (stored angles) | `TexturedSlam` - `EF_SECRA`/2, `EF_ROLLING1..10`, `EF_HEARTASURA` |
//! | 48 | `RenderColorPlane` | `TexturedMagicRing(name, kind)` - the bottom-song family |
//! | 75 | `RenderSlash1` | `JobLvUp50_2` (`EF_KAIZEL`), `RunStop` (`EF_STOPEFFECT`) |
//! | 12 | `RenderRecordRings` | magnum/cart/bowling rings, spray pond, water ball |
//! | 10, 11 | `RenderRecordQuads` | `PP_3DSPLINE`/`PP_3DSPIRAL` helix ribbons |
//! | 13 | `RenderArcBand` | cart revolution, magnum, bowling bash, frost diver |
//! | 42, 53, 59, 116 | `RenderFieldPanel` | shield throws, thrown items, heal sparks, sprinkle sand, snow, rising orbs |
//! | 9 | `RenderPyramid` | earth spike, heavens drive, storm gust, grim tooth |
//! | 137 | `RenderArcWalls` | height ramp / arc walls |
//! | 130 | `RenderFacingQuads` | rising fan |
//! | 131 | `RenderRipple` | rising fan 2 |
//! | 39 | `RenderSizedSquare` | halo |
//! | 62 | `RenderRiseArc` | wind |
//! | 143 | `RenderGroundSquare` | texture spin |
//! | 155 | `RenderTwinGlowSquares` | wandering flames / fork bolt |
//! | 43 | `RenderOrbitSquare` | intimidate / white spray |
//! | 64 | `RenderRectUp` | textured pulse kind |
//! | 66 | `RenderPentagon` | forest light |
//! | 89 | `RenderFanBeam` | textured quad spin |
//! | 82 | `RenderTeiFacingSlots` | `TexturedAura` - glows, status auras, map spheres, tarot cards |
//! | 103 | `RenderStackedRings` | `TexturedMark` - the Storm Kick rings |
//! | 96 | `RenderPeongUp` | `WhiteLightTrail` - the Storm Kick 4/5 light trails |
//! | 34 | `Render3DCasting2` | Grand Cross rings (`PP_GI_3`, `EF_GRANDCROSS`) |
//! | 37 | `RenderGrandCross2` | Grand Cross cross quads (`PP_GRANDCROSS2`, `EF_GRANDCROSS`) |
//! | 44 | `RenderRotatedPlate` | `TexturedScatter`/`TexturedScatterLow` - `EF_TEIHIT1X`, `EF_TEIHIT3` |
//! | 45 | `RenderFieldPanel` | `TEIHIT2` - `EF_TEIHIT2`, `EF_BACKSTAP` |
//! | 61 | `RenderSphereCage` | `BlackMaximize` - `EF_GUARD2` |
//! | 73 | `RenderMagnum2` | `Magnum2` - `EF_MAGNUM2` |
//! | 86, 92 | `RenderFloorSquare` | `SonicWave` / `PurpleSlash` - `EF_SOULBREAKER`, `EF_SOULBREAKER2` |
//! | 87 | `RenderFieldPanel` (camera angles) | `TexturedRain` - `EF_PRESSURE` |
//! | 88 | `RenderSizedSquare` | `TexturedLevelRing` - `EF_PRESSURE` |
//! | 91 | `RenderSphereWind` | `FeatherBurst` - `EF_BABY` |
//!
//! ## Projection and depth
//!
//! The official renderers fill `RenderVertex` with `g_renderer->Project(...)`
//! (screen space + `rhw`), then call `RenderTriangle::Visible()` (`rhw > 0` for
//! all three vertices) before `SubmitTriangle`. The port keeps the world-space
//! corners here and lets
//! [`EffectRenderer::render_triangle`](crate::renderer::EffectRenderer::render_triangle)
//! do the same per-vertex projection, so one triangle can still span three
//! different depths. That is also how the already-ported `PP_3DTEXTURE` path
//! works.
//!
//! ## Blend states
//!
//! `SubmitTriangle(triangle, flags)` puts the triangle into a queue chosen by
//! `RSF_*` flags (`RenderQueues.cpp:20-60`):
//!
//! * `RSF_ADDITIVE` (`0x2`) - the additive queue, drawn `ONE`/`ONE`,
//! * `RSF_CUSTOM_BLEND` (`0x4`) - `queue_01EC`, drawn with the triangle's own
//!   `sourceBlend`/`destinationBlend` (DX9 enum values: `5` = `SRCALPHA`, `6` =
//!   `INVSRCALPHA`, `2` = `ONE`),
//! * otherwise - `queue_0198`, drawn `SRCALPHA`/`INVSRCALPHA`.
//!
//! The client's effect pipeline is premultiplied, so the forward effect pass
//! translates a `SrcAlpha` source factor to `One`; both paths are expressed
//! with the same factors the quad path uses.
//!
//! ## Deliberate deviations (all render-side, simulation untouched)
//!
//! * `RenderFieldPanel`'s `ecl_fild01.rsw` alpha clamp reads the map name; the
//!   port has no map context in the renderer and skips it (it only dims that
//!   one map's field panels above alpha 100).
//! * `RenderPyramid`'s `flag1[0] == 3` branch rewrites `state.argb` with
//!   `std::rand()` *inside the render call*. Rendering may not mutate the
//!   ported simulation, so the same flicker colour is derived deterministically
//!   from `state.state_cnt` instead (documented at [`pyramid_color`]).
//! * `RenderTeiRect`'s `extra == 1` writes `z = 1e-09`, `rhw = 1` (the near
//!   plane of the original's standard depth buffer). The port's scene uses
//!   reverse-z, so the same "always on top" is submitted as depth `1.0`; the
//!   triangle still has to be in front of the camera to be projected at all.
//! * `RenderRectUp` leaves its colour and `pw` uninitialised for a `height[3]`
//!   other than 0/1/2/4. Undefined values cannot be reproduced; no ported body
//!   writes such a value (`TexturedPulseKind` only writes 0 and 4), and the
//!   port falls back to the `height[3] == 0` white/glow values.
//! * The 510-triangle cap of `RenderTeiRect`/the `triangles[512]` array is
//!   enforced on *emitted* triangles; the original counts only the visible
//!   ones. No ported body comes close (the densest, a 4-slot casting ring, is
//!   160).
//! * Angles are read from the client's trig tables with the index wrapped into
//!   `[0, 360]`; the original would read out of bounds where a body leaves a
//!   `height[...]` phase outside that range.
//! * `RenderTeiRect`'s `extra == 2` (only `RenderOrbitIcons`, prim 35, and
//!   `RenderDoubleGumGang`, prim 99) bumps every projected vertex's reciprocal
//!   depth by `+0.000045` and recomputes `z = RhwToDepth(rhw)`
//!   (`DepthAlpha.cpp:33`), a constant depth nudge of `10.067114 * 0.000045 =
//!   0.000453` in the original's depth units. The port projects in the holder
//!   pass and has no per-vertex depth channel on [`SlotTriangle`], so the nudge
//!   is not reproduced (the geometry, colours and blend are). It only biases
//!   the icons against the caster in the depth buffer.
//! * `RenderTeiFacingSlots` reads `slots[0].alphaT`/`slots[1].alphaT`, which
//!   `PrimAuraGlow` fills with `FaceCamera` from the client camera. The port's
//!   effect simulation has no camera, so the holder computes the same two
//!   angles at render time ([`face_camera_angles`]) and passes them in
//!   [`SlotRenderContext`]. `RenderTeiFacingSlots`'s `extra == 3` branch also
//!   reads `g_renderer->viewport` and `this->render.distanceScale`; both are
//!   render context and are carried by the same struct.
//!
//! ## `extra == 3` (the face-camera clamp)
//!
//! `RenderTeiRect`'s `extra == 3` (only `RenderTeiFacingSlots` kind 16, i.e.
//! `TexturedAura` kind 22) projects the quad, then rewrites each vertex's
//! window-space `x` around the projected `render.position` and shifts `y` by
//! `(-30 * viewport.height / 480) * render.distanceScale` (integer division
//! before the float cast). It forces `z = 1e-09`, `rhw = 1`, i.e. a
//! pre-transformed near-plane triangle. The port projects the same corners to
//! window pixels, applies the same rewrite, converts the result to NDC and
//! submits it through [`SlotTriangle::projected`]. The triangles behind the
//! camera still project (like the original's `Project`, which has no `w`
//! guard); non-finite results are dropped instead of being submitted as `NaN`.
//!
//! `this->render.distanceScale` is `1.0` for every `CEffectPrim`:
//! `CRenderObject::CRenderObject` -> `Reset` sets it
//! (`RenderObjectState.cpp:44`) and no effect body ever assigns it again (the
//! actor render paths that recompute `render.distanceScale` are
//! `CGameActor`/`CPc`-only). The port carries it as
//! [`CEFFECT_PRIM_DISTANCE_SCALE`] so the term stays explicit.

use cgmath::{Point3, Vector2};
use ragnarok_packets::EffectId;
use wgpu::BlendFactor;

use super::angles::Angles;
use super::math::matrix_mult;
use super::primitive::{EffectPrimitiveSlot, EffectPrimitiveState, PrimitiveId};
use super::runtime::EffectRuntime;
use crate::graphics::ScreenSize;
use crate::world::Camera;

/// `RenderTeiRect`'s triangle array bound (`if (*rpCnt >= 510) return;`).
pub(crate) const MAX_TRIANGLES_PER_PRIMITIVE: usize = 510;

/// The DX9 blend pair the official triangle was submitted with, already
/// resolved through the `RSF_*` queue selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotBlend {
    /// `SRCALPHA` / `INVSRCALPHA` - the alpha queue.
    Alpha,
    /// `SRCALPHA` / `ONE` - `RSF_CUSTOM_BLEND` with destination blend `ONE`,
    /// the glow blend of the casting/sized-square primitives.
    AlphaOne,
    /// `ONE` / `ONE` - the `RSF_ADDITIVE` queue.
    Additive,
}

impl SlotBlend {
    /// The factors handed to
    /// [`EffectRenderer`](crate::renderer::EffectRenderer).
    ///
    /// `Alpha` and `AlphaOne` keep `SrcAlpha` as the source factor: the effect
    /// pass translates it to `One` for the client's premultiplied textures
    /// (see `graphics/passes/forward/effect.rs`), which reproduces the straight
    /// alpha of the original exactly.
    pub fn factors(self) -> (BlendFactor, BlendFactor) {
        match self {
            Self::Alpha => (BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha),
            Self::AlphaOne => (BlendFactor::SrcAlpha, BlendFactor::One),
            Self::Additive => (BlendFactor::One, BlendFactor::One),
        }
    }
}

/// `CRenderObject::Reset`'s `render.distanceScale = 1`
/// (`RenderObjectState.cpp:44`).
///
/// `CEffectPrim` inherits `CRenderObject` and no effect body ever assigns the
/// field again, so this is the exact `this->render.distanceScale` that
/// `RenderTeiRect`'s `extra == 3` term multiplies. The holder threads it into
/// [`SlotRenderContext`] instead of hard-coding the factor in the geometry, so
/// the value is explicit and testable.
pub const CEFFECT_PRIM_DISTANCE_SCALE: f32 = 1.0;

/// One triangle built by a slot/record renderer, in world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlotTriangle {
    pub corners: [[f32; 3]; 3],
    pub texture_coordinates: [[f32; 2]; 3],
    /// Packed `argb` exactly as the official emitter builds it.
    pub argb: u32,
    /// `state.textures[index]` of the triangle.
    pub texture: Option<&'static str>,
    pub blend: SlotBlend,
    /// `RenderTeiRect`'s `extra == 1`: the original overwrites the projected z
    /// with `1e-09` and `rhw` with `1.0`, i.e. forces the triangle to the near
    /// plane. The port's reverse-z buffer expresses that as depth `1.0`.
    pub forced_depth: Option<f32>,
    /// `RenderTeiRect`'s `extra == 3` (only `RenderTeiFacingSlots` kind 16):
    /// the original has already rewritten the projected vertices and forced
    /// `rhw = 1`, so there are no world-space corners left to project. When
    /// `Some`, the triangle is submitted pre-projected through
    /// [`EffectRenderer::render_triangle_projected`](crate::renderer::EffectRenderer::render_triangle_projected)
    /// and `corners` is unused.
    pub projected: Option<ProjectedTriangle>,
}

/// A triangle the official renderer already projected and rewrote in window
/// space (`RenderTeiRect`'s `extra == 3`), stored as final NDC.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectedTriangle {
    /// NDC `x`/`y` of the three corners, in submission order.
    pub corners: [[f32; 2]; 3],
    /// Reverse-z depth. The original writes `z = 1e-09` into a standard depth
    /// buffer, i.e. the near plane, which the port expresses as `1.0`.
    pub depth: f32,
}

impl SlotTriangle {
    fn new(corners: [[f32; 3]; 3], texture_coordinates: [[f32; 2]; 3], argb: u32, texture: Option<&'static str>, blend: SlotBlend) -> Self {
        Self {
            corners,
            texture_coordinates,
            argb,
            texture,
            blend,
            forced_depth: None,
            projected: None,
        }
    }
}

/// The render-context values `RenderTeiFacingSlots` reads outside the primitive
/// state: the camera (for the `extra == 3` projection and `FaceCamera`), the
/// viewport, and `this->render.distanceScale`.
///
/// The other slot/record renderers are pure functions of the primitive state;
/// this context is only consulted by [`render_tei_facing_slots`]. It is passed
/// through [`triangles_with_context`] so [`triangles`] stays callable without a
/// camera.
#[derive(Clone, Copy)]
pub struct SlotRenderContext<'a> {
    /// The camera that rendered the frame. Used to project the kind-16 quad
    /// and is not stored anywhere.
    pub camera: &'a dyn Camera,
    /// `g_renderer->viewport` in physical pixels
    /// (`EffectRenderer::window_size`).
    pub viewport: ScreenSize,
    /// `this->render.distanceScale`; see [`CEFFECT_PRIM_DISTANCE_SCALE`].
    pub distance_scale: f32,
    /// `slots[0].alphaT` / `slots[1].alphaT` as `FaceCamera` computes them.
    pub facing: [i16; 2],
}

/// `g_fullCircleAngle` - `EffectPrimitiveRendering.cpp:44`.
const FULL_CIRCLE_ANGLE: f32 = 360.0;
/// `g_projectionDepthNumerator` - `RenderProjection.cpp:9`. It doubles as the
/// `textureV` of `RenderTeiRect` and the `uInc` wrap of the record renderers.
const PROJECTION_DEPTH_NUMERATOR: f32 = 1.0;

/// The `Render*` function of a primitive id, if it is ported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotRenderer {
    /// `Render3DCasting` - ids 60/63 (and every other id in that case list,
    /// including prim 24).
    Casting,
    /// `RenderColorPlane` - id 48 (`PP_BOTTOM2`).
    ColorPlane,
    /// `RenderSlash1` - id 75 (`PP_SLASH1`).
    Slash1,
    /// `RenderRecordRings` - id 12.
    RecordRings,
    /// `RenderRecordQuads` - ids 10/11 (`PP_3DSPLINE`/`PP_3DSPIRAL`).
    RecordQuads,
    /// `RenderArcBand` - id 13.
    ArcBand,
    /// `RenderFieldPanel` - ids 42/53/59/116 (and the other ids in that case
    /// list, `EffectPrimitiveRendering.cpp:573-592`).
    FieldPanel,
    /// `RenderPyramid` - ids 9/117.
    Pyramid,
    /// `RenderArcWalls` - id 137.
    ArcWalls,
    /// `RenderFacingQuads` - id 130.
    FacingQuads,
    /// `RenderRipple` - id 131.
    Ripple,
    /// `RenderSizedSquare` - id 39 (and 26/28/67/88/161).
    SizedSquare,
    /// `RenderRiseArc` - id 62.
    RiseArc,
    /// `RenderGroundSquare` - id 143.
    GroundSquare,
    /// `RenderTwinGlowSquares` - id 155.
    TwinGlowSquares,
    /// `RenderOrbitSquare` - id 43.
    OrbitSquare,
    /// `RenderRectUp` - id 64.
    RectUp,
    /// `RenderPentagon` - id 66.
    Pentagon,
    /// `RenderFanBeam` - id 89.
    FanBeam,
    /// `RenderTeiFacingSlots` - id 82 (and the other ids in that case list).
    AuraGlow,
    /// `RenderHalfSphere` - id 104.
    HalfSphere,
    /// `RenderBladeFins` - id 154 (`PrimFlashFade` / `TexturedBeam`).
    BladeFins,
    /// `RenderStackedRings` - prim 103 (`TexturedMark`).
    StackedRings,
    /// `RenderPeongUp` - prim 96 (`WhiteLightTrail`).
    PeongUp,
    /// `RenderHaloLayers` - prim 68 (`PP_ASURACASTING`).
    HaloLayers,
    /// `RenderOrbitIcons` - prim 35 (`SuperSlashFan`).
    OrbitIcons,
    /// `RenderDoubleGumGang` - prim 99 (`PP_DOUBLEGUMGANG`).
    DoubleGumGang,
    /// `RenderPentaRing` - prim 69 (`EF_TRIPLEATTACK`/2/3's cloud,
    /// trail and stream). No `EffectPrimId.h` symbol.
    PentaRing,
    /// `RenderHitLine3` - `PP_HITLINE3` (100), the white02 spark strands of
    /// `EF_HITLINE4/5/6`.
    HitLine3,
    /// `Render3DCasting2` - prim 34 (`PP_GI_3`), the Grand Cross rings.
    CastingGi3,
    /// `RenderGrandCross2` - prim 37 (`PP_GRANDCROSS2`), the cross quads.
    GrandCross2,
    /// `RenderRotatedPlate` - prim 44 (the `TexturedScatter` pulse).
    RotatedPlate,
    /// `RenderTube` - `PP_LINELINK` (38), the soul-link tube.
    Tube,
    /// `RenderChemical` - `PP_CHEMICALPROTECTION` (54), the shockwave wedges.
    Chemical,
    /// `RenderSphereCage` - `PP_GUARD` (61), the black-maximize shell
    /// (`EF_GUARD2`).
    SphereCage,
    /// `RenderMagnum2` - `PP_MAGNUM2` (73), the yellow shockwave ring.
    Magnum2,
    /// `RenderSphereWind` - `PP_SPHEREWIND2` (91), the fire-vine sphere; prim
    /// 77 shares it.
    SphereWind,
    /// `RenderFloorSquare` - prim 86 (`SonicWave`) and prim 92
    /// (`PurpleSlash`), the flat slash diamonds.
    FloorSquare,
    /// `RenderGroundBar` - `PP_CHEMICAL3` (56), the chemical line streaks.
    GroundBar,
    /// `RenderFloorSquare2` - prim 119/122/125, the slash fan/level/wave.
    FloorSquare2,
    /// `RenderSMA2` - `PP_SMA2` (121), the stacked wave rings of `EF_SMA2` /
    /// `EF_SMA_BLACK2`.
    Sma2,
    /// `RenderGroundArcGlow` - prim 149, the four ground arcs of
    /// `BeginSpellSwirl` (`EF_BEGINSPELL_N*`).
    GroundArcGlow,
    /// `RenderRotatedRing` - prim 126 (`TexturedSlashCross`).
    RotatedRing,
    /// `RenderSoulLink` - `PP_SOULLINK` (109).
    SoulLink,
    /// `RenderSoulParticle` - prim 115 (`TexturedTumble`).
    SoulParticle,
}

/// `CEffectPrim::Render`'s `switch (state.count)` for the ported ids
/// (`EffectPrimitiveRendering.cpp:222-650`).
pub fn renderer(state: &EffectPrimitiveState) -> Option<SlotRenderer> {
    Some(match state.count {
        PrimitiveId::PulsingColumns | PrimitiveId::RippleRings => SlotRenderer::Casting,
        PrimitiveId::Casting2 => SlotRenderer::Casting,
        PrimitiveId::Bottom2 => SlotRenderer::ColorPlane,
        // `EffectPrimitiveRendering.cpp:575-576`: `PP_BOTTOM` and
        // `PP_BOTTOMOUT` share `RenderFieldPanel` with the shield/item family.
        PrimitiveId::Bottom | PrimitiveId::BottomOut => SlotRenderer::FieldPanel,
        PrimitiveId::Slash1 => SlotRenderer::Slash1,
        PrimitiveId::Circle3d => SlotRenderer::RecordRings,
        // `EffectPrimitiveRendering.cpp:747-750`: both ids draw the shared
        // `RenderRecordQuads`.
        PrimitiveId::Spline3d | PrimitiveId::Spiral3d => SlotRenderer::RecordQuads,
        PrimitiveId::Cylinder3d => SlotRenderer::ArcBand,
        PrimitiveId::ParticleUp => SlotRenderer::FieldPanel,
        PrimitiveId::ShieldBoomerang | PrimitiveId::ThrowItem | PrimitiveId::ShieldBoomerang3 => SlotRenderer::FieldPanel,
        PrimitiveId::QuadHorn3d => SlotRenderer::Pyramid,
        PrimitiveId::HeightRamp => SlotRenderer::ArcWalls,
        PrimitiveId::RisingFan => SlotRenderer::FacingQuads,
        PrimitiveId::RisingFan2 => SlotRenderer::Ripple,
        PrimitiveId::Halo => SlotRenderer::SizedSquare,
        PrimitiveId::Wind => SlotRenderer::RiseArc,
        PrimitiveId::TextureSpin => SlotRenderer::GroundSquare,
        PrimitiveId::WanderingFlames => SlotRenderer::TwinGlowSquares,
        PrimitiveId::Intimidate => SlotRenderer::OrbitSquare,
        PrimitiveId::PulseKind => SlotRenderer::RectUp,
        PrimitiveId::ForestLight => SlotRenderer::Pentagon,
        PrimitiveId::SpinQuad => SlotRenderer::FanBeam,
        PrimitiveId::AuraGlow => SlotRenderer::AuraGlow,
        PrimitiveId::HalfSphere => SlotRenderer::HalfSphere,
        PrimitiveId::FlashFade => SlotRenderer::BladeFins,
        PrimitiveId::StackedRings => SlotRenderer::StackedRings,
        PrimitiveId::PeongUp => SlotRenderer::PeongUp,
        PrimitiveId::Casting3d | PrimitiveId::WaveSpin => SlotRenderer::Casting,
        PrimitiveId::AsuraCasting => SlotRenderer::HaloLayers,
        // Prim 147 (`TexturedSlam`) reuses `RenderHaloLayers`.
        PrimitiveId::TexturedSlam => SlotRenderer::HaloLayers,
        PrimitiveId::FloatingBubbles => SlotRenderer::OrbitIcons,
        PrimitiveId::DoubleGumGang => SlotRenderer::DoubleGumGang,
        // `EffectPrimitiveRendering.cpp:693-695`: bare `case 69` draws
        // `RenderPentaRing`.
        PrimitiveId::PentaRing => SlotRenderer::PentaRing,
        // `EffectPrimitiveRendering.cpp:440-442`: `PP_HITLINE3` draws
        // `RenderHitLine3` (distinct from `PP_HITLINE`'s `RenderHitLine`).
        PrimitiveId::HitLine3 => SlotRenderer::HitLine3,
        // `EffectPrimitiveRendering.cpp:521-522`: `PP_GI_3`.
        PrimitiveId::Gi3 => SlotRenderer::CastingGi3,
        // `EffectPrimitiveRendering.cpp:524-525`: `PP_GRANDCROSS2`.
        PrimitiveId::GrandCross2 => SlotRenderer::GrandCross2,
        // `EffectPrimitiveRendering.cpp:542-543`: bare `case 44`.
        PrimitiveId::ExpandPulse => SlotRenderer::RotatedPlate,
        // `EffectPrimitiveRendering.cpp:574`: `PP_TEIHIT2` shares
        // `RenderFieldPanel` with the shield/item family.
        PrimitiveId::TeiHit2 => SlotRenderer::FieldPanel,
        // `EffectPrimitiveRendering.cpp:504`: bare `case 50` draws
        // `Render3DCasting` with the casting family.
        PrimitiveId::TexturedGlow => SlotRenderer::Casting,
        // `EffectPrimitiveRendering.cpp:504`: bare `case 41`.
        PrimitiveId::TexturedCross => SlotRenderer::Casting,
        // `EffectPrimitiveRendering.cpp:690-691`: `PP_LINELINK`.
        PrimitiveId::LineLink => SlotRenderer::Tube,
        // `EffectPrimitiveRendering.cpp:611-612`: `PP_CHEMICALPROTECTION`.
        PrimitiveId::ChemicalProtection => SlotRenderer::Chemical,
        // `EffectPrimitiveRendering.cpp:578`: `PP_FIRSTAID` shares
        // `RenderFieldPanel`.
        PrimitiveId::FirstAid => SlotRenderer::FieldPanel,
        // `EffectPrimitiveRendering.cpp:580`: prim 87 (`TexturedRain`'s cross
        // rain) is in the `RenderFieldPanel` case list.
        PrimitiveId::BubbleSwirl => SlotRenderer::FieldPanel,
        // `EffectPrimitiveRendering.cpp:417-418`: prim 88 (`TexturedLevelRing`)
        // is in the `RenderSizedSquare` case list.
        PrimitiveId::GrowFollow => SlotRenderer::SizedSquare,
        // `EffectPrimitiveRendering.cpp:391-392`: `PP_GUARD`.
        PrimitiveId::Guard => SlotRenderer::SphereCage,
        // `EffectPrimitiveRendering.cpp:388-390`: `PP_MAGNUM2`.
        PrimitiveId::Magnum2 => SlotRenderer::Magnum2,
        // `EffectPrimitiveRendering.cpp:545-547`: bare `case 86`/`case 92`.
        PrimitiveId::SonicWave | PrimitiveId::PurpleSlash => SlotRenderer::FloorSquare,
        // `EffectPrimitiveRendering.cpp:404-405`: `PP_SPHEREWIND2` (and 77).
        PrimitiveId::SphereWind | PrimitiveId::SphereWind2 => SlotRenderer::SphereWind,
        // `EffectPrimitiveRendering.cpp:638-639`: `PP_CHEMICAL3`.
        PrimitiveId::Chemical3 => SlotRenderer::GroundBar,
        // `EffectPrimitiveRendering.cpp:549-551`.
        PrimitiveId::SlashFan | PrimitiveId::Stin3 | PrimitiveId::Stin4 => SlotRenderer::FloorSquare2,
        // `EffectPrimitiveRendering.cpp:385-386`: `PP_SMA2` draws
        // `RenderSMA2`.
        PrimitiveId::Sma2 => SlotRenderer::Sma2,
        // `EffectPrimitiveRendering.cpp:373-375`: bare `case 149:`.
        PrimitiveId::WaveHeights => SlotRenderer::GroundArcGlow,
        // `EffectPrimitiveRendering.cpp:554`.
        PrimitiveId::SlashCross => SlotRenderer::RotatedRing,
        // `EffectPrimitiveRendering.cpp:479-480`.
        PrimitiveId::SoulLink => SlotRenderer::SoulLink,
        // `EffectPrimitiveRendering.cpp:469-470`.
        PrimitiveId::ParticleSpread => SlotRenderer::SoulParticle,
        // `PP_3DPARTICLESPLINE` (8) is drawn by the shared
        // `RenderSegmentSprites2` (`EffectPrimitiveRendering.cpp:269`), which
        // is the sprite pass (`sprite_pass`), not a slot/record renderer.
        // Prim 46 (`ChimtoSprite`) shares that pass
        // (`EffectPrimitiveRendering.cpp:272-281`), so it also returns `None`.
        // Prim 120 (`Sma`) is `RenderSMA3`, which submits no geometry at all;
        // it launches child effects, so the holder calls
        // [`render_sma3`] directly instead of going through this table.
        PrimitiveId::ParticleSpline | PrimitiveId::ChimtoSprite | PrimitiveId::Sma => return None,
        _ => return None,
    })
}

/// Every triangle the primitive draws this frame, in submission order.
///
/// The order matters: the original submits into per-queue lists and the client
/// blends them in insertion order, so the port keeps the same sequence.
///
/// This is the camera-less entry point. It is everything the slot/record
/// primitives need except `RenderTeiFacingSlots`'s `extra == 3` sub-path (which
/// has no geometry without a projection); the live render path uses
/// [`triangles_with_context`].
pub fn triangles(state: &EffectPrimitiveState) -> Vec<SlotTriangle> {
    triangles_with_context(state, None)
}

/// [`triangles`] plus the render-context values
/// [`RenderTeiFacingSlots`](render_tei_facing_slots) needs.
pub fn triangles_with_context(state: &EffectPrimitiveState, context: Option<&SlotRenderContext<'_>>) -> Vec<SlotTriangle> {
    let mut out = Vec::new();

    match renderer(state) {
        Some(SlotRenderer::Casting) => render_3d_casting(state, &mut out),
        Some(SlotRenderer::ColorPlane) => render_color_plane(state, &mut out),
        Some(SlotRenderer::Slash1) => render_slash_1(state, &mut out),
        Some(SlotRenderer::RecordRings) => render_record_rings(state, &mut out),
        Some(SlotRenderer::RecordQuads) => render_record_quads(state, &mut out),
        Some(SlotRenderer::ArcBand) => render_arc_band(state, &mut out),
        Some(SlotRenderer::FieldPanel) => {
            // `PrimBubbleSwirl` (prim 87) runs `FaceCamera` on every frame; the
            // hermetic simulation has no camera, so the live path uses the
            // angles recovered from it. Every other field-panel primitive
            // keeps the values its motion wrote.
            let facing = if matches!(state.count, PrimitiveId::BubbleSwirl) {
                facing_or_stored(state, context)
            } else {
                [state.slots[0].alpha_t, state.slots[1].alpha_t]
            };

            render_field_panel_with_facing(state, facing, &mut out);
        }
        Some(SlotRenderer::Pyramid) => render_pyramid(state, &mut out),
        Some(SlotRenderer::ArcWalls) => render_arc_walls(state, &mut out),
        Some(SlotRenderer::FacingQuads) => render_facing_quads(state, &mut out),
        Some(SlotRenderer::Ripple) => render_ripple(state, &mut out),
        Some(SlotRenderer::SizedSquare) => render_sized_square(state, &mut out),
        Some(SlotRenderer::RiseArc) => render_rise_arc(state, &mut out),
        Some(SlotRenderer::GroundSquare) => render_ground_square(state, &mut out),
        Some(SlotRenderer::TwinGlowSquares) => render_twin_glow_squares(state, &mut out),
        Some(SlotRenderer::OrbitSquare) => render_orbit_square(state, &mut out),
        Some(SlotRenderer::RectUp) => render_rect_up(state, &mut out),
        Some(SlotRenderer::Pentagon) => render_pentagon(state, &mut out),
        Some(SlotRenderer::FanBeam) => render_fan_beam(state, &mut out),
        Some(SlotRenderer::AuraGlow) => render_tei_facing_slots(state, context, &mut out),
        Some(SlotRenderer::HalfSphere) => render_half_sphere(state, &mut out),
        Some(SlotRenderer::BladeFins) => render_blade_fins(state, &mut out),
        Some(SlotRenderer::StackedRings) => render_stacked_rings(state, &mut out),
        Some(SlotRenderer::PeongUp) => {
            // `PrimPeongUp` runs the file-local `FaceCamera` inline each frame
            // and stores the two angles in `slots[0].alphaT`/`slots[1].alphaT`.
            // The hermetic simulation has no camera, so the live render path
            // computes them from the camera at draw time (exactly like
            // `RenderTeiFacingSlots`); the fallback keeps the simulation value
            // for the camera-less `triangles` tests.
            let facing = context
                .map(|context| context.facing)
                .unwrap_or([state.slots[0].alpha_t, state.slots[1].alpha_t]);

            render_peong_up(state, facing, &mut out);
        }
        Some(SlotRenderer::HaloLayers) => {
            // `PrimAsuraCasting` (prim 68) runs `FaceCamera(state.slots)` each
            // frame; the hermetic simulation has no camera, so the live render
            // path uses the angles recovered from it (like
            // `RenderTeiFacingSlots`). Prim 147's `PrimRisingFan` does *not*
            // call `FaceCamera`, so `TexturedSlam`'s layers keep the
            // constructor's zeroed `alphaT` angles.
            let facing = if matches!(state.count, PrimitiveId::AsuraCasting) {
                facing_or_stored(state, context)
            } else {
                [state.slots[0].alpha_t, state.slots[1].alpha_t]
            };

            render_halo_layers(state, facing, &mut out);
        }
        Some(SlotRenderer::OrbitIcons) => {
            let facing = facing_or_stored(state, context);

            render_orbit_icons(state, facing, 1.0, 100, 100, 255, &mut out);
        }
        Some(SlotRenderer::DoubleGumGang) => {
            // `PrimFloatingBubbles` runs `FaceCamera` each frame (prim 99 shares
            // it), so the live path uses the camera angles like prim 35.
            let facing = facing_or_stored(state, context);

            render_double_gum_gang(state, facing, &mut out);
        }
        Some(SlotRenderer::PentaRing) => render_penta_ring(state, &mut out),
        Some(SlotRenderer::HitLine3) => render_hitline3(state, &mut out),
        Some(SlotRenderer::CastingGi3) => render_3d_casting2(state, &mut out),
        Some(SlotRenderer::GrandCross2) => render_grand_cross2(state, &mut out),
        Some(SlotRenderer::RotatedPlate) => render_rotated_plate(state, &mut out),
        Some(SlotRenderer::Tube) => render_tube(state, &mut out),
        Some(SlotRenderer::Chemical) => render_chemical(state, &mut out),
        Some(SlotRenderer::SphereCage) => render_sphere_cage(state, &mut out),
        Some(SlotRenderer::Magnum2) => render_magnum_2(state, &mut out),
        Some(SlotRenderer::SphereWind) => render_sphere_wind(state, &mut out),
        Some(SlotRenderer::FloorSquare) => render_floor_square(state, &mut out),
        Some(SlotRenderer::GroundBar) => render_ground_bar(state, &mut out),
        Some(SlotRenderer::FloorSquare2) => render_floor_square2(state, &mut out),
        Some(SlotRenderer::Sma2) => render_sma2(state, &mut out),
        Some(SlotRenderer::GroundArcGlow) => render_ground_arc_glow(state, &mut out),
        Some(SlotRenderer::RotatedRing) => render_rotated_ring(state, &mut out),
        Some(SlotRenderer::SoulLink) => render_soul_link(state, &mut out),
        Some(SlotRenderer::SoulParticle) => render_soul_particle(state, &mut out),
        None => {}
    }

    out
}

/// `[slots[0].alphaT, slots[1].alphaT]` after `FaceCamera`.
///
/// Prim 68 (`PrimAsuraCasting`), prim 35 (`PrimFloatingBubbles`) and prim 96
/// (`PrimPeongUp`) all call `FaceCamera`, which the hermetic simulation cannot
/// run. The live render path therefore takes the two angles from the camera
/// ([`face_camera_angles`]); the camera-less [`triangles`] entry point falls
/// back to the values stored in the slots.
fn facing_or_stored(state: &EffectPrimitiveState, context: Option<&SlotRenderContext<'_>>) -> [i16; 2] {
    context
        .map(|context| context.facing)
        .unwrap_or([state.slots[0].alpha_t, state.slots[1].alpha_t])
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn angles() -> &'static Angles {
    Angles::shared()
}

/// `g_cosTable[index]` with the index the renderers compute.
///
/// The client tables have 722 entries: `0..=360` and the negated half at
/// `361 + degrees` used by `ClientCosine`/`ClientSine`. The renderers index
/// them with plain integers, so the same indexing is kept here; a value outside
/// the table (which the decompilation would read out of bounds) is wrapped into
/// `[0, 360]` instead.
fn table_index(degrees: i32) -> usize {
    if degrees < 0 {
        let mirrored = 361 + degrees % 361;

        mirrored.clamp(0, 361) as usize
    } else if degrees > 360 {
        (degrees % 360) as usize
    } else {
        degrees as usize
    }
}

fn table_cos(degrees: i32) -> f32 {
    angles().raw_cosine(table_index(degrees))
}

fn table_sin(degrees: i32) -> f32 {
    angles().raw_sine(table_index(degrees))
}

/// The packed `argb` of a triangle: `(alphaB, red, green, blue)` with the alpha
/// in the high byte, exactly like
/// `(((alphaB << 8) + r) << 8) + g) << 8) + b`.
fn pack_argb(alpha: i16, red: u8, green: u8, blue: u8) -> u32 {
    let alpha = alpha.clamp(0, 255) as u32;

    (alpha << 24) | ((red as u32) << 16) | ((green as u32) << 8) | blue as u32
}

/// `this->state.textures[index]`.
///
/// The port stores the texture array as either the animated `texture_frames`
/// sequence (indexed by the frame, like `textures[curMotion]`) or as the three
/// fixed slots the bodies assign (`textures[0..2]`).
pub(crate) fn texture_slot(state: &EffectPrimitiveState, index: usize) -> Option<&'static str> {
    if !state.texture_frames.is_empty() {
        return state
            .texture_frames
            .get(index)
            .copied()
            .or_else(|| state.texture_frames.last().copied());
    }

    match index {
        0 => state.texture,
        1 => state.texture_secondary,
        2 => state.texture_tertiary,
        _ => None,
    }
}

/// `state.textures[curMotion]` - the frame the animated primitives advance.
fn current_texture(state: &EffectPrimitiveState) -> Option<&'static str> {
    texture_slot(state, state.cur_motion as usize)
}

/// The `RSF_*` queue selection of `CRenderer::Submit` (`RenderQueues.cpp:20`).
///
/// `RSF_ADDITIVE` wins over `RSF_CUSTOM_BLEND`, which wins over the alpha
/// queue, so the per-triangle blend of `RenderTeiRect` only applies when the
/// body asked for the custom-blend queue.
fn resolve_blend(render_flag: u32, triangle_blend: SlotBlend) -> SlotBlend {
    if (render_flag & 0x2) != 0 {
        SlotBlend::Additive
    } else if (render_flag & 0x4) != 0 {
        triangle_blend
    } else {
        SlotBlend::Alpha
    }
}

/// `Tei_TowardScreen` - `EffectPrimitiveMotion.cpp:4730`.
fn tei_toward_screen(corners: &mut [[f32; 3]; 4], rot0: i16, rot1: i16) {
    for corner in corners.iter_mut() {
        rotate_pair_x(corner, rot1 as i32);
    }

    for corner in corners.iter_mut() {
        rotate_pair_y(corner, rot0 as i32);
    }
}

/// `RotatePairX` - `EffectPrimitiveMotion.cpp:4712`.
fn rotate_pair_x(point: &mut [f32; 3], angle: i32) {
    let z = point[2];
    let y = point[1];

    point[2] = z * table_cos(angle) - y * table_sin(angle);
    point[1] = y * table_cos(angle) + z * table_sin(angle);
}

/// `RotatePairY` - `EffectPrimitiveMotion.cpp:4721`.
fn rotate_pair_y(point: &mut [f32; 3], angle: i32) {
    let x = point[0];
    let z = point[2];

    point[0] = x * table_cos(angle) - z * table_sin(angle);
    point[2] = z * table_cos(angle) + x * table_sin(angle);
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// The four screen-space corners of one `RenderTeiRect`/`RenderRecordRings`
/// emitter, before `render.position` is added.
type Quad = [[f32; 3]; 4];

/// Emits the two triangles and texture coordinates of the 10-argument
/// `RenderTeiRect` (`EffectPrimitiveRendering.cpp:978`).
///
/// * `tex_part` selects the horizontal texture window (`texPart / 21`), so a
///   column strip samples one 1/21 slice of the texture per height segment,
/// * the first triangle is `(vec1, vec2, vec3)`, the second `(vec4, vec3,
///   vec1)`, with `v = textureV` on the outer edge and `0` on the inner edge,
/// * the colour comes from the `state.size` table (`size_color`), including its
///   `alphaB = 80` override for size 3.
fn emit_tei_rect_10(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>, tex_part: i32, alpha_b: i16, quad: Quad, pw: i32) {
    emit_tei_rect_10_size(state, out, tex_part, alpha_b, quad, pw, state.size);
}

/// [`emit_tei_rect_10`] with an explicit `state.size`.
///
/// `RenderGroundArcGlow` writes `state.size` *inside* the renderer (0 for its
/// base band, then `17`/`18`/.../`32` for the glow band). Rendering may not
/// mutate the ported simulation, so the value is passed in instead; every
/// other caller keeps reading `state.size` through [`emit_tei_rect_10`].
fn emit_tei_rect_10_size(
    state: &EffectPrimitiveState,
    out: &mut Vec<SlotTriangle>,
    tex_part: i32,
    alpha_b: i16,
    quad: Quad,
    pw: i32,
    size: f32,
) {
    if out.len() >= MAX_TRIANGLES_PER_PRIMITIVE {
        return;
    }

    let (red, green, blue, texture_v, glow) = size_color(size);

    let alpha = if size as i32 == 3 { 80 } else { alpha_b };

    // `left = texPart / 21` (`0` for texPart 0), `right = (texPart + 1) / 21`.
    let left = if tex_part == 0 { 0.0 } else { tex_part as f32 / 21.0 };
    let right = (tex_part + 1) as f32 / 21.0;

    let position = state.position;
    let corners = quad.map(|corner| add(corner, position));
    let argb = pack_argb(alpha, red, green, blue);
    let blend = resolve_blend(state.render_flag, if glow { SlotBlend::AlphaOne } else { SlotBlend::Alpha });
    let texture = texture_slot(state, pw.max(0) as usize);

    push_quad(
        out,
        [corners[0], corners[1], corners[2], corners[3]],
        [(left, texture_v), (right, texture_v), (right, 0.0), (left, 0.0)],
        argb,
        texture,
        blend,
    );
}

/// Emits the two triangles of the 18-argument `RenderTeiRect`
/// (`EffectPrimitiveRendering.cpp:825`), used by the "field panel" style
/// primitives.
///
/// `add_position` is the `f1` argument: when it is zero the four corners are
/// offset by `render.position` first (the quad renderers pass corners relative
/// to the anchor), when it is one the corners are already absolute world
/// positions.
#[allow(clippy::too_many_arguments)]
fn emit_tei_rect_18(
    state: &EffectPrimitiveState,
    out: &mut Vec<SlotTriangle>,
    alpha: i16,
    quad: Quad,
    texture_index: i32,
    add_position: bool,
    (red, green, blue): (u8, u8, u8),
    (ty1, ty2, tx, tx2): (f32, f32, f32, f32),
    blend: SlotBlend,
) {
    emit_tei_rect_18_impl(
        state,
        out,
        alpha,
        quad,
        texture_index,
        add_position,
        (red, green, blue),
        (ty1, ty2, tx, tx2),
        blend,
        None,
    );
}

/// The 18-argument `RenderTeiRect` call with `extra == 1`: every emitted
/// triangle is forced to the original's near-plane depth (`z = 1e-09`,
/// `rhw = 1`, expressed as reverse-z `1.0`).
#[allow(clippy::too_many_arguments)]
fn emit_tei_rect_18_near(
    state: &EffectPrimitiveState,
    out: &mut Vec<SlotTriangle>,
    alpha: i16,
    quad: Quad,
    texture_index: i32,
    add_position: bool,
    (red, green, blue): (u8, u8, u8),
    (ty1, ty2, tx, tx2): (f32, f32, f32, f32),
    blend: SlotBlend,
) {
    emit_tei_rect_18_impl(
        state,
        out,
        alpha,
        quad,
        texture_index,
        add_position,
        (red, green, blue),
        (ty1, ty2, tx, tx2),
        blend,
        Some(1.0),
    );
}

#[allow(clippy::too_many_arguments)]
fn emit_tei_rect_18_impl(
    state: &EffectPrimitiveState,
    out: &mut Vec<SlotTriangle>,
    alpha: i16,
    quad: Quad,
    texture_index: i32,
    add_position: bool,
    (red, green, blue): (u8, u8, u8),
    (ty1, ty2, tx, tx2): (f32, f32, f32, f32),
    blend: SlotBlend,
    forced_depth: Option<f32>,
) {
    if out.len() >= MAX_TRIANGLES_PER_PRIMITIVE {
        return;
    }

    let position = state.position;
    let corners = if add_position {
        quad.map(|corner| add(corner, position))
    } else {
        quad
    };

    let argb = pack_argb(alpha, red, green, blue);
    let blend = resolve_blend(state.render_flag, blend);
    let texture = texture_slot(state, texture_index.max(0) as usize);
    let start = out.len();

    push_quad(
        out,
        [corners[0], corners[1], corners[2], corners[3]],
        [(tx, ty2), (tx2, ty2), (tx2, ty1), (tx, ty1)],
        argb,
        texture,
        blend,
    );

    if forced_depth.is_some() {
        for triangle in &mut out[start..] {
            triangle.forced_depth = forced_depth;
        }
    }
}

/// Splits a quad into the two triangles both `RenderTeiRect` overloads emit:
/// `(c0, c1, c2)` then `(c3, c2, c0)`, with the matching UVs.
#[allow(clippy::too_many_arguments)]
fn push_quad(out: &mut Vec<SlotTriangle>, corners: Quad, uvs: [(f32, f32); 4], argb: u32, texture: Option<&'static str>, blend: SlotBlend) {
    let triangle = |indices: [usize; 3]| {
        SlotTriangle::new(
            indices.map(|index| corners[index]),
            indices.map(|index| [uvs[index].0, uvs[index].1]),
            argb,
            texture,
            blend,
        )
    };

    if out.len() < MAX_TRIANGLES_PER_PRIMITIVE {
        out.push(triangle([0, 1, 2]));
    }

    if out.len() < MAX_TRIANGLES_PER_PRIMITIVE {
        out.push(triangle([3, 2, 0]));
    }
}

/// The colour table of the 10-argument `RenderTeiRect`
/// (`EffectPrimitiveRendering.cpp:985-1120`).
///
/// Returns `(red, green, blue, textureV, glow)`. `glow` is the destination
/// blend `ONE` (`blend = 2`, the initial value) versus `INVSRCALPHA`
/// (`blend = 6`, set by the listed cases).
pub fn size_color(size: f32) -> (u8, u8, u8, f32, bool) {
    let mut texture_v = PROJECTION_DEPTH_NUMERATOR;
    let mut glow = true;
    let (red, green, blue) = match size as i32 {
        0 => {
            glow = false;
            (255, 255, 255)
        }
        1 => (255, 175, 175),
        2 => {
            glow = false;
            (195, 195, 255)
        }
        3 => {
            glow = false;
            (255, 255, 255)
        }
        4 => (100, 100, 255),
        6 => (255, 100, 100),
        7 => (100, 255, 100),
        8 => (255, 255, 170),
        9 => (140, 210, 140),
        10 => (255, 89, 182),
        11 => (170, 170, 255),
        12 => {
            glow = false;
            (50, 50, 50)
        }
        13 => {
            glow = false;
            (255, 50, 255)
        }
        14 => {
            glow = false;
            (120, 120, 255)
        }
        15 => (10, 255, 10),
        16 => (20, 255, 20),
        17 => (0, 0, 255),
        18 => (255, 0, 0),
        19 => (0, 255, 0),
        20 => (255, 255, 0),
        21 => (255, 0, 255),
        22 => (0, 255, 255),
        23 => {
            glow = false;
            (41, 94, 188)
        }
        24 => (10, 66, 114),
        25 => (35, 146, 185),
        26 => (164, 205, 254),
        27 => (84, 169, 205),
        28 => (108, 26, 255),
        29 => (177, 255, 58),
        30 => (85, 30, 185),
        31 => (244, 187, 124),
        32 => {
            glow = false;
            (0, 0, 0)
        }
        40 => (70, 70, 255),
        41 => (30, 30, 255),
        42 => {
            glow = false;
            (43, 40, 46)
        }
        43 => {
            glow = false;
            (125, 182, 160)
        }
        44 => {
            glow = false;
            (180, 55, 30)
        }
        45 => (65, 141, 231),
        46 => (164, 47, 29),
        47 => (172, 154, 56),
        48 => (80, 20, 20),
        49 => (200, 130, 84),
        50 => (255, 198, 91),
        100 => {
            texture_v = 0.9;
            glow = false;
            (250, 250, 255)
        }
        _ => (255, 255, 255),
    };

    (red, green, blue, texture_v, glow)
}

// ---------------------------------------------------------------------------
// Render3DCasting (60, 63)
// ---------------------------------------------------------------------------

/// `CEffectPrim::Render3DCasting` - `EffectPrimitiveRendering.cpp:6596-6675`.
///
/// Every live slot sweeps its `fullDisplayAngle` in 21 steps
/// (`fullDisplayAngle / 20` apart, integer division) around `rotStart`, with a
/// ring radius of `distance` and a height of `height[order]` raised along
/// `riseAngle`. Consecutive sweep positions form one quad per step; the quad's
/// texture window is the segment index (`texPart = order - 1`), so the column
/// texture repeats 1/21 per height step.
///
/// Two `maxHeight` values are recognised by the original and shift the whole
/// strip down by 2 or 1 units; the port keeps that (`6.001`/`24.001` are the
/// markers the ring-wave bodies set).
pub fn render_3d_casting(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        if state.slots[ec].life != 1 {
            continue;
        }

        let slot = &state.slots[ec];
        let basic_angle = slot.full_display_angle as i32 / 20;

        // The original would loop forever here (`count += 0`); every body that
        // draws casting geometry sets at least 20 degrees. Bail out instead of
        // hanging.
        if basic_angle <= 0 {
            continue;
        }

        let base_y = if slot.max_height == 6.001 {
            -2.0
        } else if slot.max_height == 24.001 {
            -1.0
        } else {
            0.0
        };

        let mut order = 0usize;
        let mut previous_bottom = [0.0f32; 3];
        let mut previous_top = [0.0f32; 3];
        let mut count = 0i32;

        while count <= slot.full_display_angle as i32 {
            let mut angle = (count + slot.rot_start as i32) % 360;

            if slot.full_display_angle == 360 && order == 20 {
                angle = slot.rot_start as i32;
            }

            let cosine = table_cos(angle);
            let sine = table_sin(angle);

            let bottom = [cosine * slot.distance, base_y, sine * slot.distance];

            let rx = table_cos(slot.rise_angle as i32) * slot.height[order];
            let ry = table_sin(slot.rise_angle as i32) * slot.height[order];
            let rz = sine * rx;
            let rx = cosine * rx;
            let top = [bottom[0] + rx, base_y - ry, bottom[2] + rz];

            if order > 0 {
                emit_tei_rect_10(
                    state,
                    out,
                    order as i32 - 1,
                    slot.alpha_b,
                    [previous_bottom, bottom, top, previous_top],
                    0,
                );
            }

            order += 1;
            if order >= 21 {
                order = 20;
            }

            previous_bottom = bottom;
            previous_top = top;
            count += basic_angle;
        }
    }
}

// ---------------------------------------------------------------------------
// RenderColorPlane (48)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderColorPlane` - `EffectPrimitiveRendering.cpp:2710-2746`.
///
/// One upright colour plane per live slot with a positive `alphaB`: the quad
/// spans `vecBNow` to `vecBPre` horizontally and from `y = 0` down to
/// `y = -maxHeight`. The colour comes from `slot.height[0]` (the
/// `TexturedMagicRing` `kind`), and `kind` 1/2 use the straight alpha blend
/// (`pw = 1`) while 3/4/everything else use the glow blend (`pw = 0`).
///
/// The corners are absolute slot vectors, but the call passes `f1 = 0`, so the
/// original still adds `render.position` to all four before projecting; the
/// port keeps that (`add_position`).
pub fn render_color_plane(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 || slot.alpha_b <= 0 {
            continue;
        }

        let height = -slot.max_height;
        let first = [slot.vec_b_now[0], 0.0, slot.vec_b_now[2]];
        let second = [slot.vec_b_pre[0], 0.0, slot.vec_b_pre[2]];
        let third = [slot.vec_b_pre[0], height, slot.vec_b_pre[2]];
        let fourth = [slot.vec_b_now[0], height, slot.vec_b_now[2]];

        let kind = slot.height[0];

        let (colour, blend) = if kind == 1.0 {
            ((255, 155, 155), SlotBlend::Alpha)
        } else if kind == 2.0 {
            ((155, 255, 155), SlotBlend::Alpha)
        } else if kind == 3.0 {
            ((255, 255, 100), SlotBlend::AlphaOne)
        } else if kind == 4.0 {
            ((255, 150, 150), SlotBlend::AlphaOne)
        } else {
            ((255, 255, 255), SlotBlend::AlphaOne)
        };

        emit_tei_rect_18(
            state,
            out,
            slot.alpha_b,
            [first, second, third, fourth],
            0,
            true,
            colour,
            (0.0, 1.0, 0.0, 1.0),
            blend,
        );
    }
}

// ---------------------------------------------------------------------------
// RenderSlash1 (75)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderSlash1` - `EffectPrimitiveRendering.cpp:5402-5468`.
///
/// Three quads per live slot with `alphaB > 0`, swept from
/// `rotStart - 1` over `rotStart + 1` in one-degree steps. The inner edge is at
/// radius `height[0]`, the outer at `height[0] + distance`, and the drop below
/// `y = -0.0` depends on the step index: the middle quad (index 1) drops
/// `maxHeight` on the outer edge and `maxHeight * 0.2` on the inner one, the
/// two outer quads drop `* 0.4` / `* 0.1`. Only the middle quad keeps the
/// slot's full alpha; the others use `alphaB / 3` (integer division).
///
/// The call passes `pw = 0`, i.e. the glow (destination `ONE`) blend, and the
/// quad arguments are `(vec1, vec3, vec4, vec2)` in the original's local
/// naming.
pub fn render_slash_1(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 || slot.alpha_b <= 0 {
            continue;
        }

        for index in 0..=2i32 {
            let mut angle = slot.rot_start as i32 - 1 + index;

            if angle < 0 {
                angle += 360;
            } else if angle >= 360 {
                angle -= 360;
            }

            let cosine = table_cos(angle);
            let sine = table_sin(angle);

            let mut radius = slot.height[0];

            let first = [cosine * radius, -0.0, sine * radius];
            let mut second = first;
            second[1] = if index == 1 {
                first[1] - slot.max_height * 0.2
            } else {
                first[1] - slot.max_height * 0.1
            };

            radius = slot.height[0] + slot.distance;

            let third = [cosine * radius, -0.0, sine * radius];
            let mut fourth = third;
            fourth[1] = if index == 1 {
                third[1] - slot.max_height
            } else {
                third[1] - slot.max_height * 0.4
            };

            // `static_cast<short>(alpha)`; `alphaB / 3` truncates toward zero.
            let alpha = if index == 1 { slot.alpha_b } else { slot.alpha_b / 3 };

            emit_tei_rect_18(
                state,
                out,
                alpha,
                [first, third, fourth, second],
                0,
                true,
                (255, 255, 255),
                (0.0, 1.0, 0.0, 1.0),
                SlotBlend::AlphaOne,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// RenderRecordRings (12)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderRecordRings` - `EffectPrimitiveRendering.cpp:5846-5942`.
///
/// One ring per segment record: the inner radius is `records[j].radius -
/// state.size`, the outer is `records[j].radius`, and the strip is swept in
/// `arcAngle` steps over `360 - arcAngle` degrees. Each step emits two
/// triangles whose `u` advances by a quarter of the texture per step
/// (`uInc`), wrapping at 1, and whose `v` is 1 on the inner edge and 0 on the
/// outer one. The per-vertex colour is `records[j].color`, the alpha ramp that
/// `ShiftSegment` builds.
pub fn render_record_rings(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    let temp = FULL_CIRCLE_ANGLE - state.arc_angle;
    let arc_angle = state.arc_angle;
    let mut u_inc = 0.0f32;

    for record in state.records.iter().take(state.num_segment.max(0) as usize) {
        let inner_size = record.radius - state.size;

        let mut i = 0.0f32;

        while i <= temp {
            let angle1 = i as i32;
            let angle2 = (i + arc_angle) as i32;

            let cs1 = table_cos(angle1);
            let sn1 = table_sin(angle1);
            let cs2 = table_cos(angle2);
            let sn2 = table_sin(angle2);

            let position = record.position;
            let matrix = &record.matrix.values;

            let vec1 = add(matrix_mult([inner_size * cs1, inner_size * sn1, 0.0], matrix), position);
            let vec2 = add(matrix_mult([inner_size * cs2, inner_size * sn2, 0.0], matrix), position);
            let vec3 = add(matrix_mult([record.radius * cs1, record.radius * sn1, 0.0], matrix), position);
            let vec4 = add(matrix_mult([record.radius * cs2, record.radius * sn2, 0.0], matrix), position);

            let blend = resolve_blend(state.render_flag, SlotBlend::Alpha);
            let texture = texture_slot(state, 0);
            let u_left = u_inc;
            let u_right = u_inc + 0.25;

            if out.len() < MAX_TRIANGLES_PER_PRIMITIVE {
                out.push(SlotTriangle::new(
                    [vec1, vec2, vec3],
                    [[u_left, 1.0], [u_right, 1.0], [u_left, 0.0]],
                    record.color,
                    texture,
                    blend,
                ));
            }

            if out.len() < MAX_TRIANGLES_PER_PRIMITIVE {
                out.push(SlotTriangle::new(
                    [vec4, vec3, vec2],
                    [[u_right, 0.0], [u_left, 0.0], [u_right, 1.0]],
                    record.color,
                    texture,
                    blend,
                ));
            }

            u_inc += 0.25;
            if u_inc >= PROJECTION_DEPTH_NUMERATOR {
                u_inc = 0.0;
            }

            i += arc_angle;
        }
    }
}

// ---------------------------------------------------------------------------
// RenderRecordQuads (10, 11)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderRecordQuads` - `EffectPrimitiveRendering.cpp:5790-5842`,
/// dispatch `:747-750` for `PP_3DSPLINE`/`PP_3DSPIRAL`.
///
/// Every segment record contributes a quad split into the two triangles
/// `(corner0, corner1, corner2)` and `(corner3, corner2, corner1)`:
///
/// ```text
/// triangle A: (0,1,2)  uv (0,1) (1,1) (0,0)
/// triangle B: (3,2,1)  uv (1,0) (0,0) (1,1)
/// ```
///
/// The `v` axis runs from the top edge (`corners[0..2]`, v = 1) to the bottom
/// edge (`corners[2..4]`, v = 0), and the colour is the record's own packed
/// `color` (alpha + RGB swept by `ShiftSegment`) rather than `state.argb`.
///
/// Quirk kept: the official call is `SubmitTriangle(triangle, 1)`, a literal
/// `RSF_ALPHA`, so `state.renderFlag` is ignored here (unlike
/// `RenderRecordRings`, which passes it through). The port therefore always
/// uses the alpha queue.
///
/// `Visible()` (all three projected `rhw > 0`) is enforced by the renderer
/// when it projects the world-space corners, exactly like the other slot
/// renderers; the original's `rpCnt` scratch slot reuse has no observable
/// effect on the submitted order.
pub fn render_record_quads(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    let texture = texture_slot(state, 0);
    let blend = SlotBlend::Alpha;

    for index in 0..state.num_segment.max(0) as usize {
        // The official `records[50]` is always allocated and zeroed, so a
        // `numSegment` larger than the ported `Vec` renders zero corners
        // (the original's behaviour) instead of shortening the strip.
        let record = state.records.get(index).copied().unwrap_or_default();

        out.push(SlotTriangle::new(
            [record.corners[0], record.corners[1], record.corners[2]],
            [[0.0, 1.0], [1.0, 1.0], [0.0, 0.0]],
            record.color,
            texture,
            blend,
        ));

        out.push(SlotTriangle::new(
            [record.corners[3], record.corners[2], record.corners[1]],
            [[1.0, 0.0], [0.0, 0.0], [1.0, 1.0]],
            record.color,
            texture,
            blend,
        ));
    }
}

// ---------------------------------------------------------------------------
// RenderArcBand (13)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderArcBand` - `EffectPrimitiveRendering.cpp:6008-6100`.
///
/// The cylinder band: `innerSize` is the radius at the anchor height and
/// `outerSize` the radius at `-heightSize` (the band flares downwards), swept
/// in `arcAngle` steps. Every step emits the two triangles
/// `(inner1, outer1, inner2)` and `(outer2, outer1, inner2)` with the texture
/// `u` advancing a quarter per step and `v` = 1 on the inner rim, 0 on the
/// outer one. The colour is the primitive's `argb` (`ProcessAlpha`).
pub fn render_arc_band(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    let temp = FULL_CIRCLE_ANGLE - state.arc_angle;
    let arc_angle = state.arc_angle;
    let matrix = &state.transform.values;
    let mut u_inc = 0.0f32;

    let mut i = 0.0f32;

    while i <= temp {
        let angle1 = i as i32;
        let angle2 = (i + arc_angle) as i32;

        let cs1 = table_cos(angle1);
        let sn1 = table_sin(angle1);
        let cs2 = table_cos(angle2);
        let sn2 = table_sin(angle2);

        let vec1 = add(
            matrix_mult([state.inner_size * cs1, 0.0, state.inner_size * sn1], matrix),
            state.position,
        );
        let vec2 = add(
            matrix_mult([state.inner_size * cs2, 0.0, state.inner_size * sn2], matrix),
            state.position,
        );
        let vec3 = add(
            matrix_mult([state.outer_size * cs1, -state.height_size, state.outer_size * sn1], matrix),
            state.position,
        );
        let vec4 = add(
            matrix_mult([state.outer_size * cs2, -state.height_size, state.outer_size * sn2], matrix),
            state.position,
        );

        let blend = resolve_blend(1, SlotBlend::Alpha);
        let texture = current_texture(state);
        let u_left = u_inc;
        let u_right = u_inc + 0.25;

        if out.len() < MAX_TRIANGLES_PER_PRIMITIVE {
            out.push(SlotTriangle::new(
                [vec1, vec3, vec2],
                [[u_left, 1.0], [u_right, 1.0], [u_left, 0.0]],
                state.argb,
                texture,
                blend,
            ));
        }

        if out.len() < MAX_TRIANGLES_PER_PRIMITIVE {
            out.push(SlotTriangle::new(
                [vec4, vec3, vec2],
                [[u_right, 0.0], [u_left, 0.0], [u_right, 1.0]],
                state.argb,
                texture,
                blend,
            ));
        }

        u_inc += 0.25;
        if u_inc >= PROJECTION_DEPTH_NUMERATOR {
            u_inc = 0.0;
        }

        i += arc_angle;
    }
}

// ---------------------------------------------------------------------------
// RenderFieldPanel (59)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderFieldPanel` - `EffectPrimitiveRendering.cpp:3312-3505`.
///
/// The billboard-ish "field panel" family: four ground-plane corners at
/// `distance` rotated by `fullDisplayAngle`, then rotated towards the screen by
/// `slots[0].alphaT`/`slots[1].alphaT` (`Tei_TowardScreen`), offset by the
/// slot's own `vecBNow`. `slots[0].flag1[2]` selects the colour and corner
/// order; `slots[ec].flag1[0]` is the texture index. `height[8]` wobbles the
/// radius, `height[9] == 10` shifts the panel 4 units in x, and `flag1[2] ==
/// 25` drops it 110 units.
///
/// The `ecl_fild01.rsw` alpha clamp of the original reads the map name; that
/// context does not exist in the renderer (see the module deviations).
pub fn render_field_panel(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    let facing = [state.slots[0].alpha_t, state.slots[1].alpha_t];

    render_field_panel_with_facing(state, facing, out);
}

/// `RenderFieldPanel` with the two `FaceCamera` angles supplied by the caller.
///
/// Every spawner of the renderer except prim 87 (`PrimBubbleSwirl`) keeps
/// `slots[0..1].alphaT` as written by its motion, so
/// [`render_field_panel`] passes the stored values. `PrimBubbleSwirl` calls
/// `FaceCamera` (`EffectPrimitiveMotion.cpp:6963`) and the hermetic simulation
/// has no camera, so its live render path passes the angles recovered from the
/// camera (see [`face_camera_angles`]), exactly like the prim 35/68/82/96
/// renderers.
pub fn render_field_panel_with_facing(state: &EffectPrimitiveState, facing: [i16; 2], out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 || slot.alpha_b <= 0 {
            continue;
        }

        // `if (height[8] >= 360 || height[8] < 0) height[8] = 0;`
        let height8 = if slot.height[8] >= 360.0 || slot.height[8] < 0.0 {
            0.0
        } else {
            slot.height[8]
        };

        let rx = slot.distance * table_sin(height8 as i32) * 0.05 + slot.distance;

        let angle1 = slot.full_display_angle as i32;
        let angle2 = angle1 + if angle1 < 270 { 90 } else { -270 };
        let angle3 = angle2 + if angle2 + 90 >= 360 { -270 } else { 90 };
        let angle4 = angle3 + if angle3 + 90 >= 360 { -270 } else { 90 };

        let mut corners: Quad = [
            [rx * table_cos(angle1), 0.0, rx * table_sin(angle1)],
            [rx * table_cos(angle2), 0.0, rx * table_sin(angle2)],
            [rx * table_cos(angle3), 0.0, rx * table_sin(angle3)],
            [rx * table_cos(angle4), 0.0, rx * table_sin(angle4)],
        ];

        if slot.height[9] == 10.0 {
            for corner in corners.iter_mut() {
                corner[0] += 4.0;
            }
        }

        tei_toward_screen(&mut corners, facing[0], facing[1]);

        let base = slot.vec_b_now;
        let corners = corners.map(|corner| add(corner, base));

        let texture_index = slot.flag1[0] as i32;
        // `switch (this->state.slots[0].flag1[2])`
        //
        // Every call passes `pw`; the 18-argument `RenderTeiRect` turns
        // `pw != 0` into `INVSRCALPHA` (plain alpha) and `pw == 0` into `ONE`
        // (the glow blend), hence the `glow` booleans below.
        let flipped = [corners[1], corners[0], corners[3], corners[2]];

        match state.slots[0].flag1[2] {
            0 => emit_field_panel(state, out, slot.alpha_b, corners, texture_index, false, (255, 255, 255)),
            1 => emit_field_panel(state, out, slot.alpha_b, corners, texture_index, true, (150, 150, 250)),
            2 => emit_field_panel(state, out, slot.alpha_b, flipped, texture_index, true, (130, 130, 250)),
            3 => emit_field_panel(state, out, slot.alpha_b, flipped, texture_index, true, (200, 200, 100)),
            4 => emit_field_panel(state, out, slot.alpha_b, flipped, texture_index, false, (255, 255, 255)),
            7 | 23 => emit_field_panel(state, out, slot.alpha_b, flipped, texture_index, true, (255, 255, 255)),
            8 => emit_field_panel(state, out, slot.alpha_b, flipped, texture_index, false, (135, 255, 135)),
            10 => emit_field_panel(state, out, slot.alpha_b, flipped, texture_index, true, (250, 250, 150)),
            11 => emit_field_panel(state, out, slot.alpha_b, flipped, texture_index, true, (220, 250, 220)),
            12 => emit_field_panel(state, out, slot.alpha_b, flipped, texture_index, false, (252, 171, 143)),
            13 => emit_field_panel(state, out, slot.alpha_b, flipped, texture_index, true, (250, 100, 100)),
            14 => emit_field_panel(state, out, slot.alpha_b, flipped, texture_index, false, (94, 0, 0)),
            15 => emit_field_panel(state, out, slot.alpha_b, corners, texture_index, false, (255, 200, 255)),
            16 => emit_field_panel(state, out, slot.alpha_b, flipped, texture_index, false, (0, 0, 0)),
            17 => emit_field_panel(state, out, slot.alpha_b, flipped, texture_index, false, (255, 180, 180)),
            18 => emit_field_panel(state, out, slot.alpha_b / 2, flipped, texture_index, true, (255, 255, 255)),
            24 => emit_field_panel(state, out, slot.alpha_b, flipped, texture_index, true, (63, 72, 204)),
            9 => {
                // `switch (this->state.slots[0].flag1[3])`
                let colours = match state.slots[0].flag1[3] {
                    // pw = 1 for 1..=5, pw = 0 for 6.
                    1 => Some(((255, 135, 135), false)),
                    2 => Some(((255, 205, 135), false)),
                    3 => Some(((255, 255, 135), false)),
                    4 => Some(((255, 255, 255), false)),
                    5 => Some(((135, 135, 255), false)),
                    6 => Some(((255, 255, 255), true)),
                    _ => None,
                };

                if let Some((colour, glow)) = colours {
                    emit_field_panel(state, out, slot.alpha_b, flipped, texture_index, glow, colour);
                }
            }
            25 => {
                let dropped = corners.map(|corner| [corner[0], corner[1] - 110.0, corner[2]]);

                emit_field_panel(
                    state,
                    out,
                    slot.alpha_b,
                    [dropped[1], dropped[0], dropped[3], dropped[2]],
                    texture_index,
                    true,
                    (255, 255, 255),
                );
            }
            _ => {}
        }
    }
}

/// The 18-argument `RenderTeiRect` call shape of `RenderFieldPanel`.
#[allow(clippy::too_many_arguments)]
fn emit_field_panel(
    state: &EffectPrimitiveState,
    out: &mut Vec<SlotTriangle>,
    alpha: i16,
    corners: Quad,
    texture_index: i32,
    glow: bool,
    (red, green, blue): (u8, u8, u8),
) {
    emit_tei_rect_18(
        state,
        out,
        alpha,
        corners,
        texture_index,
        false,
        (red, green, blue),
        (0.0, 1.0, 0.0, 1.0),
        if glow { SlotBlend::AlphaOne } else { SlotBlend::Alpha },
    );
}

// ---------------------------------------------------------------------------
// RenderTube (38)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderTube` - `EffectPrimitiveRendering.cpp:3214-3310`.
///
/// The `PP_LINELINK` tube: a 10-step elliptical cross-section swept from
/// `vecTNow` to `vecTPre`, submitted twice. Pass 0 uses radius `maxHeight`,
/// pass 1 doubles it. The end point is a sine-eased interpolation towards
/// `vecTPre` for the first 15 frames and `vecTPre` itself afterwards. The
/// side colours key off `slots[0].height[4]` (2 = red glow, 3 = pink glow,
/// 4 = translucent grey double layer, default = yellow/blue glow).
///
/// The call passes `f1 = 1`, so the corners are already absolute world
/// positions (`add_position = false`). `alpha` is a `short` local in the
/// original and is intentionally **not** reset between the two passes: the
/// kind-4 branch reuses `alphaB / 2` from pass 0 in pass 1.
pub fn render_tube(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    let slot = state.slots[0];

    if slot.life != 1 || slot.alpha_b <= 0 {
        return;
    }

    // `const float length = SquareRoot(deltaX * deltaX + deltaZ * deltaZ);`
    let delta_z = slot.vec_t_now[2] - slot.vec_t_pre[2];
    let delta_x = slot.vec_t_now[0] - slot.vec_t_pre[0];
    let length = (delta_x * delta_x + delta_z * delta_z).sqrt();
    let dir_z = delta_z / length;
    let dir_x = delta_x / length;

    let mut heading: i32 = 0;
    while heading <= 180 {
        if dir_x >= table_cos(heading) {
            break;
        }

        heading += 1;
    }

    let heading = if heading <= 180 {
        if dir_z >= 0.0 { heading as i16 } else { (360 - heading) as i16 }
    } else {
        180
    };

    let mut side = heading + 90;
    if side >= 360 {
        side -= 360;
    }

    let end = if slot.process < 15 {
        let t = table_sin(6 * slot.process);
        [
            (slot.vec_t_pre[0] - slot.vec_t_now[0]) * t + slot.vec_t_now[0],
            (slot.vec_t_pre[1] - slot.vec_t_now[1]) * t + slot.vec_t_now[1],
            (slot.vec_t_pre[2] - slot.vec_t_now[2]) * t + slot.vec_t_now[2],
        ]
    } else {
        slot.vec_t_pre
    };

    let mut previous_start = [0.0f32; 3];
    let mut previous_end = [0.0f32; 3];
    let mut alpha: i16 = 0;

    for pass in 0..2 {
        for step in 0..=10 {
            let radius = if pass == 0 {
                slot.max_height
            } else {
                slot.max_height + slot.max_height
            };

            let angle = step * 36;
            let across = radius * table_cos(angle);
            let up = radius * table_sin(angle);
            let offset_x = across * table_cos(side as i32);
            let offset_z = across * table_sin(side as i32);

            let start = [offset_x + slot.vec_t_now[0], up + slot.vec_t_now[1], offset_z + slot.vec_t_now[2]];
            let finish = [end[0] + offset_x, end[1] + up, end[2] + offset_z];

            if step > 0 {
                let quad = [previous_start, start, finish, previous_end];

                if pass == 0 {
                    if slot.height[4] == 2.0 {
                        alpha = slot.alpha_b;
                        emit_tei_rect_18(
                            state,
                            out,
                            alpha,
                            quad,
                            0,
                            false,
                            (0, 100, 150),
                            (0.0, 1.0, 0.0, 1.0),
                            SlotBlend::Alpha,
                        );
                    } else if slot.height[4] == 3.0 {
                        alpha = slot.alpha_b;
                        emit_tei_rect_18(
                            state,
                            out,
                            alpha,
                            quad,
                            0,
                            false,
                            (255, 89, 182),
                            (0.0, 1.0, 0.0, 1.0),
                            SlotBlend::Alpha,
                        );
                    } else if slot.height[4] == 4.0 {
                        alpha = slot.alpha_b / 2;
                        emit_tei_rect_18(
                            state,
                            out,
                            alpha,
                            quad,
                            0,
                            false,
                            (50, 50, 50),
                            (0.0, 1.0, 0.0, 1.0),
                            SlotBlend::AlphaOne,
                        );
                    } else {
                        alpha = slot.alpha_b;
                        emit_tei_rect_18(
                            state,
                            out,
                            alpha,
                            quad,
                            0,
                            false,
                            (255, 255, 0),
                            (0.0, 1.0, 0.0, 1.0),
                            SlotBlend::Alpha,
                        );
                    }
                } else if slot.height[4] == 2.0 {
                    alpha = (2 * slot.alpha_b as i32) as i16;
                    emit_tei_rect_18(
                        state,
                        out,
                        alpha,
                        quad,
                        0,
                        false,
                        (255, 50, 0),
                        (0.0, 1.0, 0.0, 1.0),
                        SlotBlend::Alpha,
                    );
                } else if slot.height[4] == 3.0 {
                    alpha = (2 * slot.alpha_b as i32) as i16;
                    emit_tei_rect_18(
                        state,
                        out,
                        alpha,
                        quad,
                        0,
                        false,
                        (255, 89, 182),
                        (0.0, 1.0, 0.0, 1.0),
                        SlotBlend::Alpha,
                    );
                } else if slot.height[4] == 4.0 {
                    // QUIRK: `alpha` keeps the `alphaB / 2` of pass 0.
                    emit_tei_rect_18(
                        state,
                        out,
                        alpha,
                        quad,
                        0,
                        false,
                        (50, 50, 50),
                        (0.0, 1.0, 0.0, 1.0),
                        SlotBlend::AlphaOne,
                    );
                } else {
                    alpha = (2 * slot.alpha_b as i32) as i16;
                    emit_tei_rect_18(
                        state,
                        out,
                        alpha,
                        quad,
                        0,
                        false,
                        (0, 0, 255),
                        (0.0, 1.0, 0.0, 1.0),
                        SlotBlend::Alpha,
                    );
                }
            }

            previous_start = start;
            previous_end = finish;
        }
    }
}

// ---------------------------------------------------------------------------
// RenderChemical (54)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderChemical` - `EffectPrimitiveRendering.cpp:6906-6975`.
///
/// The two ground wedges per live slot of `PP_CHEMICALPROTECTION` (the
/// `ShockwaveRing` family). Each wedge is a triangle fan around the primitive
/// origin between `riseAngle - 3` and `riseAngle + 3` (white) and
/// `riseAngle - 5` / `riseAngle + 5` (blue), at `distance` from the anchor.
/// The corners are rotated by `Tei_TowardScreen` with the two `FaceCamera`
/// angles the motion stores in `slots[0].alphaT`/`slots[1].alphaT`.
///
/// `g_depthAverageHalf = 0.5` (`MapSkyMgr.cpp:27`) scales the first wedge's
/// alpha: `static_cast<int>(alphaB * 0.5f)`.
pub fn render_chemical(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = state.slots[ec];

        if slot.life != 1 || slot.alpha_b <= 0 {
            continue;
        }

        // The original submits the +/- 3 degree wedge first, then the +/- 5
        // degree one; both start from `vec1 = vec4 = (0, 0, 0)`.
        for delta in [3i32, 5] {
            let mut angle = slot.rise_angle as i32 - delta;
            if angle < 0 {
                angle += 360;
            }
            let second = [table_cos(angle) * slot.distance, 0.0, table_sin(angle) * slot.distance];

            angle = slot.rise_angle as i32 + delta;
            if angle >= 360 {
                angle -= 360;
            }
            let third = [table_cos(angle) * slot.distance, 0.0, table_sin(angle) * slot.distance];

            let mut corners: Quad = [[0.0; 3], second, third, [0.0; 3]];
            tei_toward_screen(&mut corners, state.slots[0].alpha_t, state.slots[1].alpha_t);

            // `static_cast<int>(alphaB * g_depthAverageHalf)` with
            // `g_depthAverageHalf = 0.5f`.
            let (alpha, (red, green, blue)) = if delta == 3 {
                ((slot.alpha_b as f32 * 0.5) as i32 as i16, (255, 255, 255))
            } else {
                (slot.alpha_b, (130, 130, 255))
            };

            emit_tei_rect_18(
                state,
                out,
                alpha,
                corners,
                0,
                true,
                (red, green, blue),
                (0.0, 1.0, 0.0, 1.0),
                SlotBlend::Alpha,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// RenderPyramid (9)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderPyramid` - `EffectPrimitiveRendering.cpp:6100-6172`.
///
/// `PP_3DQUADHORN`'s four-sided pyramid: a square base of `state.size` and an
/// apex `heightSize` above it in +z, rotated by `rotTransform`. Each of the
/// four side triangles samples a fifth of the texture (`uVal` advances 0.2).
/// The base corner order is the official one, so the visible sides are
/// `(-,-) -> apex -> (+,-)`, then counter-clockwise.
pub fn render_pyramid(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    let size = state.size;
    let height = state.height_size;

    let corners: [[[f32; 3]; 3]; 4] = [
        [[-size, -size, 0.0], [0.0, 0.0, height], [size, -size, 0.0]],
        [[-size, -size, 0.0], [0.0, 0.0, height], [-size, size, 0.0]],
        [[-size, size, 0.0], [0.0, 0.0, height], [size, size, 0.0]],
        [[size, size, 0.0], [0.0, 0.0, height], [size, -size, 0.0]],
    ];

    let argb = if state.slots[0].flag1[0] == 3 {
        pyramid_color(state)
    } else {
        state.argb
    };

    let matrix = &state.rot_transform.values;
    let mut u_value = 0.0f32;

    for triangle in corners.iter() {
        let projected = triangle.map(|corner| add(matrix_mult(corner, matrix), state.position));

        let (blend, argb) = if state.slots[0].flag1[0] == 3 {
            // `SubmitTriangle(..., 5)` with `sourceBlend = 5`,
            // `destinationBlend = 2` by the caller.
            (resolve_blend(5, SlotBlend::AlphaOne), argb)
        } else {
            (resolve_blend(1, SlotBlend::Alpha), state.argb)
        };

        if out.len() < MAX_TRIANGLES_PER_PRIMITIVE {
            out.push(SlotTriangle::new(
                projected,
                [[u_value, 1.0], [u_value, 0.0], [u_value + 0.2, 1.0]],
                argb,
                texture_slot(state, 0),
                blend,
            ));
        }

        u_value += 0.2;
    }
}

/// The `flag1[0] == 3` colour override of `RenderPyramid`.
///
/// The original does `state.argb = ((((rand() % 11) << 8) + rand() % 11) << 8)
/// - 0xD7D701` *inside the render call*, i.e. it mutates the simulation state
/// and flickers with the process-wide `std::rand` sequence. Rendering in the
/// port is `&self` (it must not touch the simulation), so the same shape is
/// built from a deterministic hash of the frame and slot phase: the alpha byte
/// stays zero and the colour bytes stay in `0..11`, so the additive spark
/// flicker is preserved without a side effect.
fn pyramid_color(state: &EffectPrimitiveState) -> u32 {
    let mut seed = (state.state_cnt as u32).wrapping_mul(0x9E37_79B9) ^ (state.slots[0].process as u32);
    seed ^= seed >> 15;
    seed = seed.wrapping_mul(0x85EB_CA6B);
    seed ^= seed >> 13;

    let red = seed % 11;
    let green = (seed >> 8) % 11;

    0u32.wrapping_sub(((green << 8) + red) << 8).wrapping_sub(0xD7D701)
}

// ---------------------------------------------------------------------------
// RenderArcWalls (137)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderArcWalls` - `EffectPrimitiveSweeps.cpp:421-521`.
///
/// Two wall bands per slot: an inner one at `distance` with the red
/// `255,10,10` field-panel texture and an outer one at `distance * 1.5`
/// (offset 45 degrees) with the `state.size` colour table. The sweep is
/// `fullDisplayAngle / 4` apart, five height samples per slot. Both bands add
/// the slot's `vecBNow + vecTPre` to every corner, which is the "wall" offset
/// the motion accumulates.
pub fn render_arc_walls(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 {
            continue;
        }

        let step = slot.full_display_angle as i32 / 4;

        // `offset += 0` would loop forever in the original.
        if step <= 0 {
            continue;
        }

        let base = add(slot.vec_b_now, slot.vec_t_pre);

        // Inner wall band (`distance`), red field-panel colour.
        let mut previous_bottom = [0.0f32; 3];
        let mut previous_top = [0.0f32; 3];
        let mut i = 0i32;
        let mut offset = 0i32;

        while offset <= slot.full_display_angle as i32 {
            let mut angle = (slot.rot_start as i32 + offset) % 360;

            if slot.full_display_angle == 360 && i == 4 {
                angle = slot.rot_start as i32;
            }

            let cosine = table_cos(angle);
            let sine = table_sin(angle);
            let rise_x = table_cos(slot.rise_angle as i32) * slot.height[i as usize];
            let rise_y = table_sin(slot.rise_angle as i32) * slot.height[i as usize];

            let current_bottom = add([cosine * slot.distance, 0.0, sine * slot.distance], base);
            let current_top = add(
                [
                    cosine * slot.distance + cosine * rise_x,
                    -rise_y,
                    sine * slot.distance + sine * rise_x,
                ],
                base,
            );

            if i > 0 {
                emit_tei_rect_18(
                    state,
                    out,
                    slot.alpha_b / 5,
                    [previous_bottom, current_bottom, previous_top, current_top],
                    0,
                    true,
                    (255, 10, 10),
                    (0.0, 1.0, 0.0, 1.0),
                    SlotBlend::Alpha,
                );
            }

            i += 1;
            if i > 4 {
                i = 4;
            }

            previous_bottom = current_bottom;
            previous_top = current_top;
            offset += step;
        }

        // Outer wall band (`distance * 1.5`, rotated 45 degrees).
        let mut i = 0i32;
        let mut offset = 0i32;

        while offset <= slot.full_display_angle as i32 {
            let mut angle = (slot.rot_start as i32 + 45 + offset) % 360;

            if slot.full_display_angle == 360 && i == 4 {
                angle = slot.rot_start as i32;
            }

            let cosine = table_cos(angle);
            let sine = table_sin(angle);
            let rise_x = table_cos(slot.rise_angle as i32) * slot.height[i as usize];
            let rise_y = table_sin(slot.rise_angle as i32) * slot.height[i as usize];

            let current_bottom = add([cosine * slot.distance * 1.5, 0.0, sine * slot.distance * 1.5], base);
            let current_top = add(
                [
                    cosine * slot.distance * 1.5 + cosine * rise_x,
                    -rise_y,
                    sine * slot.distance * 1.5 + sine * rise_x,
                ],
                base,
            );

            // The original clears the rise on the bottom of this band before
            // adding the slot offset: `curB.y = 0`.
            let current_bottom = [current_bottom[0], base[1], current_bottom[2]];

            if i > 0 {
                emit_tei_rect_10(
                    state,
                    out,
                    i - 1,
                    slot.alpha_b,
                    [previous_bottom, current_bottom, current_top, previous_top],
                    0,
                );
            }

            i += 1;
            if i > 4 {
                i = 4;
            }

            previous_bottom = current_bottom;
            previous_top = current_top;
            offset += step;
        }
    }
}

// ---------------------------------------------------------------------------
// RenderFacingQuads (130)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderFacingQuads` - `EffectPrimitiveRendering.cpp:1667-1700`.
///
/// A `distance`-sized square turned towards the screen (`Tei_TowardScreen`),
/// drawn four times at the slot's four stored positions (`vecBPre`, `vecBNow`,
/// `vecTPre`, `vecTNow`). The alpha of each copy is `height[11..14] * 2`
/// (`flag1[11..14]`), and `slot.flag1[5]` selects the colour.
pub fn render_facing_quads(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 {
            continue;
        }

        let size = slot.distance;
        let mut corners: Quad = [
            [size * table_cos(45), 0.0, size * table_sin(45)],
            [size * table_cos(135), 0.0, size * table_sin(135)],
            [size * table_cos(225), 0.0, size * table_sin(225)],
            [size * table_cos(315), 0.0, size * table_sin(315)],
        ];

        tei_toward_screen(&mut corners, state.slots[0].alpha_t, state.slots[1].alpha_t);

        let (red, green, blue) = match slot.flag1[5] {
            7 => (250, 70, 100),
            8 => (250, 250, 10),
            10 => (250, 250, 250),
            _ => (70, 120, 100),
        };

        let positions = [slot.vec_b_pre, slot.vec_b_now, slot.vec_t_pre, slot.vec_t_now];
        let alphas = [slot.flag1[11], slot.flag1[12], slot.flag1[13], slot.flag1[14]];

        for (position, alpha) in positions.into_iter().zip(alphas) {
            let doubled = alpha as i32 * 2;
            let alpha = if doubled >= 0 { doubled as i16 } else { 0 };
            let corners = corners.map(|corner| add(corner, position));

            emit_tei_rect_18(
                state,
                out,
                alpha,
                corners,
                0,
                true,
                (red, green, blue),
                (0.0, 1.0, 0.0, 1.0),
                SlotBlend::AlphaOne,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// RenderRipple (131)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderRipple` - `EffectPrimitiveRendering.cpp:2173-2230`.
///
/// Ten concentric squares per slot, from `ring = 10` down to `1`, each
/// `distance + spread` wide (`spread` grows by 3 per ring, or 1 for kind 2),
/// rotated towards the screen, lowered 30 units (40 for kind 2), and drawn with
/// the alpha stored in `height[ring]`. The texture index is `flag1[0]`.
pub fn render_ripple(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 {
            continue;
        }

        for ring in (1..=10usize).rev() {
            let kind = slot.flag1[4];
            let mut spread = 10i32 - ring as i32;

            if kind != 2 {
                spread *= 3;
            }

            let rx = spread as f32 + slot.distance;

            let mut corners: Quad = [
                [rx * table_cos(45), 0.0, rx * table_sin(45)],
                [rx * table_cos(135), 0.0, rx * table_sin(135)],
                [rx * table_cos(225), 0.0, rx * table_sin(225)],
                [rx * table_cos(315), 0.0, rx * table_sin(315)],
            ];

            let base = slot.height[0];
            for corner in corners.iter_mut() {
                corner[0] += base;
            }

            tei_toward_screen(&mut corners, state.slots[0].alpha_t, state.slots[1].alpha_t);

            let drop = if kind == 2 { 40.0 } else { 30.0 };
            for corner in corners.iter_mut() {
                corner[1] -= drop;
            }

            let alpha = slot.height[ring] as i32;
            if alpha <= 0 {
                continue;
            }

            let (red, green, blue) = if ring == 10 { (55, 55, 255) } else { (5, 5, 255) };

            emit_tei_rect_18(
                state,
                out,
                alpha as i16,
                [corners[1], corners[0], corners[3], corners[2]],
                slot.flag1[0] as i32,
                true,
                (red, green, blue),
                (0.0, 1.0, 0.0, 1.0),
                SlotBlend::AlphaOne,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// RenderSizedSquare (39)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderSizedSquare` - `EffectPrimitiveRendering.cpp:5942-6008`.
///
/// A ground square of radius
/// `(sin(riseAngle) + 1) * (distance * 0.1) + distance * 0.8`, rotated by the
/// slot's `rotStart` (or `ec * 23` for the sizes the `RenderSizedSquare` case
/// list does not name), drawn with `state.size` selecting the colour. This is
/// the halo ring.
pub fn render_sized_square(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 {
            continue;
        }

        let size = state.size;
        let mut angle = ec as i32 * 23;

        if size == 100.0 || size >= 110.0 || (size == 4.0 && slot.height[4] == 4.0) || size == 10.0 || size == 0.0 {
            angle = slot.rot_start as i32;
        }

        // `const double radius = (g_sinTable[riseAngle] + 1.0f) * (distance * 0.1f) +
        // distance * 0.8;` The sum is evaluated in float and then widened; the
        // trigonometric product below is the part that runs in double.
        let radius = (table_sin(slot.rise_angle as i32) + 1.0) * (slot.distance * 0.1) + slot.distance * 0.8;

        let angle2 = if angle + 90 >= 360 { angle - 270 } else { angle + 90 };
        let angle3 = if angle2 + 90 >= 360 { angle2 - 270 } else { angle2 + 90 };
        let angle4 = if angle3 + 90 >= 360 { angle3 - 270 } else { angle3 + 90 };

        let y = slot.height[0];
        let point = |degrees: i32| {
            [
                ((table_cos(degrees) as f64) * radius as f64) as f32,
                y,
                ((table_sin(degrees) as f64) * radius as f64) as f32,
            ]
        };

        let quad: Quad = [point(angle), point(angle2), point(angle3), point(angle4)];

        // The mode table of the original: the duplicate calls (101, 100) are
        // kept.
        match size as i32 {
            101 => {
                for _ in 0..2 {
                    emit_sized(state, out, slot, quad, (255, 0, 0), SlotBlend::AlphaOne);
                }
            }
            102 => emit_sized(state, out, slot, quad, (255, 255, 0), SlotBlend::AlphaOne),
            111 => emit_sized(state, out, slot, quad, (55, 55, 255), SlotBlend::AlphaOne),
            112 => emit_sized(state, out, slot, quad, (105, 225, 105), SlotBlend::AlphaOne),
            100 => {
                for _ in 0..2 {
                    emit_sized(state, out, slot, quad, (255, 255, 255), SlotBlend::AlphaOne);
                }
            }
            110 => emit_sized(state, out, slot, quad, (255, 255, 255), SlotBlend::Alpha),
            113 => emit_sized(state, out, slot, quad, (5, 5, 255), SlotBlend::AlphaOne),
            114 => emit_sized(state, out, slot, quad, (155, 5, 5), SlotBlend::AlphaOne),
            15 => emit_sized(state, out, slot, quad, (35, 255, 35), SlotBlend::AlphaOne),
            17 => emit_sized(state, out, slot, quad, (105, 105, 255), SlotBlend::AlphaOne),
            18 => emit_sized(state, out, slot, quad, (35, 35, 255), SlotBlend::AlphaOne),
            0 => emit_sized(state, out, slot, quad, (255, 255, 255), SlotBlend::Alpha),
            _ => emit_sized(state, out, slot, quad, (255, 255, 255), SlotBlend::AlphaOne),
        }
    }
}

fn emit_sized(
    state: &EffectPrimitiveState,
    out: &mut Vec<SlotTriangle>,
    slot: &EffectPrimitiveSlot,
    quad: Quad,
    colour: (u8, u8, u8),
    blend: SlotBlend,
) {
    emit_tei_rect_18(state, out, slot.alpha_b, quad, 0, false, colour, (0.0, 1.0, 0.0, 1.0), blend);
}

// ---------------------------------------------------------------------------
// RenderRiseArc (62)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderRiseArc` - `EffectPrimitiveRendering.cpp:1599-1642`.
///
/// The winding arc of `PP_WIND`: 21 height samples swept over
/// `fullDisplayAngle / 21` degrees, each step one `RenderTeiRect` strip at
/// `distance` with the slot's `maxHeight` as the base offset.
pub fn render_rise_arc(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 {
            continue;
        }

        let step = slot.full_display_angle as i32 / 21;

        if step <= 0 {
            continue;
        }

        let mut i = 0usize;
        let mut previous_bottom = [0.0f32; 3];
        let mut previous_top = [0.0f32; 3];
        let mut offset = 0i32;

        while offset < slot.full_display_angle as i32 {
            let mut angle = (slot.rot_start as i32 + offset) % 360;

            if slot.full_display_angle == 360 && i == 20 {
                angle = slot.rot_start as i32;
            }

            let cosine = table_cos(angle);
            let sine = table_sin(angle);

            let bottom = [cosine * slot.distance, -slot.max_height, sine * slot.distance];

            let rise_x = table_cos(slot.rise_angle as i32) * slot.height[i];
            let rise_y = table_sin(slot.rise_angle as i32) * slot.height[i];

            let top = [bottom[0] + cosine * rise_x, -(rise_y + slot.max_height), bottom[2] + sine * rise_x];

            if i > 0 {
                emit_tei_rect_10(
                    state,
                    out,
                    i as i32 - 1,
                    slot.alpha_b,
                    [previous_bottom, bottom, top, previous_top],
                    0,
                );
            }

            i += 1;
            if i >= 21 {
                i = 20;
            }

            previous_bottom = bottom;
            previous_top = top;
            offset += step;
        }
    }
}

// ---------------------------------------------------------------------------
// RenderGroundSquare (143)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderGroundSquare` - `EffectPrimitiveArcs.cpp:437-481`.
///
/// `PP_TEXTURETWEEN`'s ground square: `radius = (sin(riseAngle) + 1) *
/// (distance * 0.1) + distance * 0.8` at `height[0]`, rotated by `rotStart` in
/// 90 degree steps. `height[2] == 1` selects the glow blend, `height[1]` is the
/// texture index (a `char` cast in the original).
pub fn render_ground_square(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 {
            continue;
        }

        let mut angle = slot.rot_start as i32;
        let radius = (table_sin(slot.rise_angle as i32) + 1.0) * (slot.distance * 0.1) + slot.distance * 0.8;
        let y = slot.height[0];

        let point = |degrees: i32| {
            [
                ((table_cos(degrees) as f64) * radius as f64) as f32,
                y,
                ((table_sin(degrees) as f64) * radius as f64) as f32,
            ]
        };

        let first = point(angle);

        angle += if angle < 270 { 90 } else { -270 };
        let second = point(angle);

        angle += if angle + 90 < 360 { 90 } else { -270 };
        let third = point(angle);

        angle += if angle + 90 < 360 { 90 } else { -270 };
        let fourth = point(angle);

        let glow = slot.height[2] != 1.0;

        emit_tei_rect_18(
            state,
            out,
            slot.alpha_b,
            [first, second, third, fourth],
            slot.height[1] as i8 as i32,
            true,
            (255, 255, 255),
            (0.0, 1.0, 0.0, 1.0),
            if glow { SlotBlend::AlphaOne } else { SlotBlend::Alpha },
        );
    }
}

// ---------------------------------------------------------------------------
// RenderTwinGlowSquares (155)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderTwinGlowSquares` -
/// `EffectPrimitiveGlowSquares.cpp:7-109`.
///
/// Two `distance`-sized squares turned towards the screen and placed at
/// `vecBPre` and `vecBNow`; the pair is the fork bolt's twin glow. The alphas
/// live in `height[12]`/`height[13]`, `flag1[0]` selects the colour pair, and
/// both copies are submitted with `extra = 1` (forced to the near plane, i.e.
/// depth `1.0` in the port's reverse-z buffer).
pub fn render_twin_glow_squares(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 {
            continue;
        }

        let mut corners: Quad = [
            [slot.distance * table_cos(45), 0.0, slot.distance * table_sin(45)],
            [slot.distance * table_cos(135), 0.0, slot.distance * table_sin(135)],
            [slot.distance * table_cos(225), 0.0, slot.distance * table_sin(225)],
            [slot.distance * table_cos(315), 0.0, slot.distance * table_sin(315)],
        ];

        tei_toward_screen(&mut corners, state.slots[0].alpha_t, state.slots[1].alpha_t);

        for (position, height_index) in [(slot.vec_b_pre, 12usize), (slot.vec_b_now, 13usize)] {
            let alpha = slot.height[height_index] as i32;
            let alpha = alpha.max(0);

            let corners = corners.map(|corner| add(corner, position));
            let first_triangle = out.len();

            match slot.flag1[0] {
                1 => {
                    // pw = 1 (alpha) then pw = 0 (glow).
                    emit_tei_rect_18(
                        state,
                        out,
                        (alpha / 3) as i16,
                        corners,
                        0,
                        true,
                        (255, 255, 255),
                        (0.0, 1.0, 0.0, 1.0),
                        SlotBlend::Alpha,
                    );
                    emit_tei_rect_18(
                        state,
                        out,
                        alpha as i16,
                        corners,
                        0,
                        true,
                        (255, 50, 50),
                        (0.0, 1.0, 0.0, 1.0),
                        SlotBlend::AlphaOne,
                    );
                }
                2 => emit_tei_rect_18(
                    state,
                    out,
                    alpha as i16,
                    [corners[1], corners[0], corners[3], corners[2]],
                    0,
                    true,
                    (255, 255, 255),
                    (0.0, 1.0, 0.0, 1.0),
                    SlotBlend::AlphaOne,
                ),
                3 => emit_tei_rect_18(
                    state,
                    out,
                    alpha as i16,
                    [corners[1], corners[0], corners[3], corners[2]],
                    0,
                    true,
                    (255, 255, 255),
                    (0.0, 1.0, 0.0, 1.0),
                    SlotBlend::AlphaOne,
                ),
                _ => {
                    emit_tei_rect_18(
                        state,
                        out,
                        (alpha / 3) as i16,
                        corners,
                        0,
                        true,
                        (255, 255, 255),
                        (0.0, 1.0, 0.0, 1.0),
                        SlotBlend::Alpha,
                    );
                    emit_tei_rect_18(
                        state,
                        out,
                        alpha as i16,
                        corners,
                        0,
                        true,
                        (50, 50, 255),
                        (0.0, 1.0, 0.0, 1.0),
                        SlotBlend::AlphaOne,
                    );
                }
            }

            // `extra == 1`: the original overwrites z with 1e-09, i.e. draws
            // the glow on top regardless of the scene depth.
            for triangle in &mut out[first_triangle..] {
                triangle.forced_depth = Some(1.0);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// RenderOrbitSquare (43)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderOrbitSquare` - `EffectPrimitiveRendering.cpp:3838-3925`.
///
/// The intimidate orbit square: a `height[1]`-radius square at `rotStart`,
/// tilted by `height[2]`, offset by `distance` along `fullDisplayAngle` and
/// `riseAngle`, and placed at the slot's `vecBNow`. `flag1[0]` selects the
/// colour (2/8/9 also turn the square towards the screen); 6 and 7 are forced
/// onto the near plane (`extra = 1`).
pub fn render_orbit_square(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 || slot.alpha_b <= 0 {
            continue;
        }

        let distance = slot.distance;
        let full_angle = slot.full_display_angle as i32;
        let base_x = table_cos(full_angle) * distance - table_sin(full_angle) * 0.0;
        let offset_y = table_sin(full_angle) * distance + table_cos(full_angle) * 0.0;
        let offset_x = base_x * table_cos(slot.rise_angle as i32) - table_sin(slot.rise_angle as i32) * 0.0;
        let offset_z = base_x * table_sin(slot.rise_angle as i32) + table_cos(slot.rise_angle as i32) * 0.0;

        let radius = slot.height[1];
        let angle1 = slot.rot_start as i32;
        let tilt = slot.height[2] as i32;
        let sn = table_sin(tilt);
        let cs = table_cos(tilt);

        let angle2 = if angle1 + 90 < 360 { angle1 + 90 } else { angle1 - 270 };
        let angle3 = angle2 + if angle2 + 90 < 360 { 90 } else { -270 };
        let angle4 = angle3 + if angle3 + 90 < 360 { 90 } else { -270 };

        let tilted = |degrees: i32| {
            [
                cs * (radius * table_cos(degrees)) - sn * 0.0,
                radius * table_sin(degrees),
                sn * (radius * table_cos(degrees)) + cs * 0.0,
            ]
        };

        let mut corners: Quad = [tilted(angle1), tilted(angle2), tilted(angle3), tilted(angle4)];
        let kind = slot.flag1[0];

        if kind == 2 {
            let face1 = if angle1 + 90 < 360 { angle1 + 90 } else { angle1 - 270 };
            let face2 = face1 + if face1 + 90 < 360 { 90 } else { -270 };
            let face3 = face2 + if face2 + 90 < 360 { 90 } else { -270 };

            let flat = |degrees: i32| [radius * table_cos(degrees), 0.0, radius * table_sin(degrees)];

            corners = [flat(angle1), flat(face1), flat(face2), flat(face3)];
            tei_toward_screen(&mut corners, state.slots[0].alpha_t, state.slots[1].alpha_t);
        } else if kind == 8 || kind == 9 {
            let face1 = angle1 + if angle1 < 270 { 90 } else { -270 };
            let face2 = face1 + if face1 + 90 < 360 { 90 } else { -270 };
            let face3 = face2 + if face2 + 90 < 360 { 90 } else { -270 };

            let flat = |degrees: i32| [radius * table_cos(degrees), 0.0, radius * table_sin(degrees)];

            corners = [flat(angle1), flat(face1), flat(face2), flat(face3)];
            tei_toward_screen(&mut corners, state.slots[0].alpha_t, state.slots[1].alpha_t);
        }

        let position = slot.vec_b_now;
        let offset = [offset_x, offset_y, offset_z];
        let corners = corners.map(|corner| add(add(corner, offset), position));

        let colour = match kind {
            0 => (100, 150, 255),
            1 => (250, 250, 155),
            2 => (250, 100, 100),
            3 => (50, 50, 255),
            4 => (255, 50, 50),
            5 => (50, 255, 50),
            6 => (250, 100, 100),
            7 => (5, 5, 255),
            8 => (50, 50, 255),
            9 => (70, 70, 70),
            _ => continue,
        };

        let start = out.len();
        emit_tei_rect_18(
            state,
            out,
            slot.alpha_b,
            corners,
            0,
            false,
            colour,
            (0.0, 1.0, 0.0, 1.0),
            SlotBlend::AlphaOne,
        );

        // Kinds 6/7 pass `extra = 1` (near plane).
        if kind == 6 || kind == 7 {
            for triangle in &mut out[start..] {
                triangle.forced_depth = Some(1.0);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// RenderRectUp (64)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderRectUp` - `EffectPrimitiveRendering.cpp:5149-5227`.
///
/// `TexturedPulseKind`'s vertical box: the slot's `height[0]`/`height[1]`
/// corners rotated by `rotStart` at `y = -1`, `height[2]` lowering the top
/// edge, and four side quads (plus two crossing quads for `height[3] == 2`).
/// `height[3]` selects the colour and the blend.
pub fn render_rect_up(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 || slot.alpha_b <= 0 {
            continue;
        }

        let cs1 = table_cos(slot.rot_start as i32);
        let sn1 = table_sin(slot.rot_start as i32);

        let mut rx = slot.height[0];
        let mut rz = slot.height[1];
        let corner = |rx: f32, rz: f32| [cs1 * rx - sn1 * rz, -1.0, sn1 * rx + cs1 * rz];

        let vec_b_pre = corner(rx, rz);
        rz = -slot.height[1];
        let vec_b_now = corner(rx, rz);
        rx = -slot.height[0];
        let vec_t_now = corner(rx, rz);
        rz = slot.height[1];
        let vec_t_pre = corner(rx, rz);

        let y = -1.0 - slot.height[2];
        let vec1 = [vec_b_pre[0], y, vec_b_pre[2]];
        let vec2 = [vec_b_now[0], y, vec_b_now[2]];
        let vec3 = [vec_t_pre[0], y, vec_t_pre[2]];
        let vec4 = [vec_t_now[0], y, vec_t_now[2]];

        // `height[3]` selects the colour and `pw`. The original leaves the
        // values uninitialised for any other height; no ported body sets one
        // (`TexturedPulseKind` only writes 0 and 4), so the port keeps the
        // `height[3] == 0` white/glow values there and says so.
        let (colour, glow) = match slot.height[3] {
            0.0 => ((255, 255, 255), true),
            1.0 => ((235, 255, 235), false),
            2.0 => ((80, 80, 80), false),
            4.0 => {
                let blue = (slot.height[4] + 0.0 + slot.height[5]) as i32 as u8;
                ((255, 255, blue), false)
            }
            _ => ((255, 255, 255), true),
        };

        let emit = |out: &mut Vec<SlotTriangle>, alpha: i16, quad: Quad| {
            emit_tei_rect_18(
                state,
                out,
                alpha,
                quad,
                0,
                true,
                colour,
                (0.0, 1.0, 0.0, 1.0),
                if glow { SlotBlend::AlphaOne } else { SlotBlend::Alpha },
            )
        };

        emit(out, slot.alpha_b, [vec_b_pre, vec_b_now, vec2, vec1]);
        emit(out, slot.alpha_b, [vec_b_now, vec_t_now, vec4, vec2]);
        emit(out, slot.alpha_b, [vec_t_now, vec_t_pre, vec3, vec4]);
        emit(out, slot.alpha_b, [vec_t_pre, vec_b_pre, vec1, vec3]);

        if slot.height[3] == 2.0 {
            let alpha = (slot.alpha_b as i32 * 2).min(255) as i16;
            emit(out, alpha, [vec_b_pre, vec_t_now, vec4, vec1]);
            emit(out, alpha, [vec_b_now, vec_t_pre, vec3, vec2]);
        }
    }
}

// ---------------------------------------------------------------------------
// RenderHalfSphere (104)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderHalfSphere` - `EffectPrimitiveRendering.cpp:1744-1820`.
///
/// `TexturedPulse`'s shell: for each live slot, ten latitude rows
/// (`angle2 = 0..162` step 18) sweeping `fullDisplayAngle` of longitude in
/// `fullDisplayAngle / 10` degree steps. The row radius is `distance`, the
/// shell hangs `maxHeight` below the slot's `vecTNow` anchor, and every quad
/// is the 18-argument `RenderTeiRect` call
/// `(alphaB, v1, v2, v4, v3, wtm, 1, 0, 0, 250, 250, 255, 0, 1, 0, 1, 0)`.
///
/// The original declares `vec1`/`vec3` once, outside the slot loop, and chains
/// them across rows; the port keeps that sharing (every row's first
/// `count == 0` pass overwrites them before the `count > 0` emit).
///
/// QUIRK: `basicAngle` is `fullDisplayAngle / 10`; a `fullDisplayAngle` in
/// `0..=9` makes it 0 and the original's inner `for` never advances (an
/// infinite loop). The port keeps the same step, so such a slot would never
/// terminate either; no ported body configures one (`TexturedPulse` uses 180).
pub fn render_half_sphere(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    /// `const int basicAngle2 = 18;`
    const BASIC_ANGLE2: i32 = 18;

    let mut vec1 = [0.0f32; 3];
    let mut vec3 = [0.0f32; 3];

    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 {
            continue;
        }

        let basic_angle = slot.full_display_angle as i32 / 10;

        let mut angle2 = 0i32;
        while angle2 < 180 {
            let mut count = 0i32;
            while count <= slot.full_display_angle as i32 {
                let mut angle = count + slot.rot_start as i32;

                if angle >= 360 {
                    angle -= 360;
                }

                if slot.full_display_angle == 360 && count == slot.full_display_angle as i32 {
                    angle = slot.rot_start as i32;
                }

                let cs1 = table_cos(angle);
                let sn1 = table_sin(angle);

                let cs2 = table_cos(angle2);
                let sn2 = table_sin(angle2);

                let dist = slot.distance;

                let mut vec2 = [0.0f32; 3];
                vec2[1] = cs2 * dist;
                let dist = sn2 * dist;

                vec2[0] = cs1 * dist;
                vec2[2] = sn1 * dist;
                vec2[1] -= slot.max_height;

                vec2[0] += slot.vec_t_now[0];
                vec2[2] += slot.vec_t_now[2];

                let cs2 = table_cos(angle2 + BASIC_ANGLE2);
                let sn2 = table_sin(angle2 + BASIC_ANGLE2);

                let dist = slot.distance;

                let mut vec4 = [0.0f32; 3];
                vec4[1] = cs2 * dist;
                let dist = sn2 * dist;

                vec4[0] = cs1 * dist;
                vec4[2] = sn1 * dist;
                vec4[1] -= slot.max_height;

                vec4[0] += slot.vec_t_now[0];
                vec4[2] += slot.vec_t_now[2];

                if count > 0 {
                    // The call passes `vec1, vec2, vec4, vec3`, i.e. the
                    // caller's fourth corner becomes `RenderTeiRect`'s
                    // `vec3` argument.
                    emit_tei_rect_18(
                        state,
                        out,
                        slot.alpha_b,
                        [vec1, vec2, vec4, vec3],
                        0,
                        true,
                        (250, 250, 255),
                        (0.0, 1.0, 0.0, 1.0),
                        SlotBlend::Alpha,
                    );
                }

                vec1 = vec2;
                vec3 = vec4;

                count += basic_angle;
            }

            angle2 += BASIC_ANGLE2;
        }
    }
}

// ---------------------------------------------------------------------------
// RenderBladeFins (154)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderBladeFins` - `EffectPrimitiveArcs.cpp:1394-1461`.
///
/// `TexturedBeam`'s fins: each live slot with a non-zero alpha submits two
/// mirrored pairs. The inner corner of a pair sits at `height[0]` along
/// `riseAngle -/+ 25` (wrapped once), the outer corner `maxHeight` further
/// along that angle minus 90 degrees; the pair itself is anchored six units
/// below the effect (`base`/`base2.y = -6`). Every quad is the 18-argument
/// `RenderTeiRect` call `(alphaB/4 ...)` + `(alphaB ...)` with `pw = 1`
/// (`SRCALPHA`/`INVSRCALPHA`), `f1 = 0` (corners relative to
/// `render.position`) and `extra = 1` (near-plane depth).
///
/// `flag1[0]` selects the colour pair: `1` is pink then red, `2` is white
/// twice, everything else is white then blue.
pub fn render_blade_fins(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 || slot.alpha_b <= 0 {
            continue;
        }

        // `base`/`base2` never change; `outer.y`/`inner.y` stay at -6.
        let base = [0.0f32, -6.0, 0.0];
        let base2 = [0.0f32, -6.0, 0.0];
        let mut outer = [0.0f32, -6.0, 0.0];
        let mut inner = [0.0f32, -6.0, 0.0];

        let mut angle = slot.rise_angle as i32 - 25;
        if angle < 0 {
            angle += 360;
        }
        inner[0] = slot.height[0] * table_cos(angle);
        inner[2] = slot.height[0] * table_sin(angle);
        angle -= 90;
        if angle < 0 {
            angle += 360;
        }
        outer[0] = slot.max_height * table_cos(angle) + inner[0];
        outer[2] = slot.max_height * table_sin(angle) + inner[2];

        emit_blade_fins_pair(state, out, slot.alpha_b, [base, outer, inner, base2], slot.flag1[0]);

        angle = slot.rise_angle as i32 + 25;
        if angle >= 360 {
            angle -= 360;
        }
        inner[0] = slot.height[0] * table_cos(angle);
        inner[2] = slot.height[0] * table_sin(angle);
        angle -= 90;
        if angle < 0 {
            angle += 360;
        }
        outer[0] = slot.max_height * table_cos(angle) + inner[0];
        outer[2] = slot.max_height * table_sin(angle) + inner[2];

        emit_blade_fins_pair(state, out, slot.alpha_b, [base, outer, inner, base2], slot.flag1[0]);
    }
}

/// The two 18-argument `RenderTeiRect` calls of one [`render_blade_fins`] pair.
fn emit_blade_fins_pair(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>, alpha_b: i16, quad: Quad, flag: i8) {
    let (inner_colour, outer_colour) = match flag {
        1 => ((255, 89, 182), (255, 25, 25)),
        2 => ((255, 255, 255), (255, 255, 255)),
        _ => ((255, 255, 255), (25, 25, 255)),
    };

    emit_tei_rect_18_near(
        state,
        out,
        alpha_b / 4,
        quad,
        0,
        true,
        inner_colour,
        (0.0, 1.0, 0.0, 1.0),
        SlotBlend::Alpha,
    );
    emit_tei_rect_18_near(
        state,
        out,
        alpha_b,
        quad,
        0,
        true,
        outer_colour,
        (0.0, 1.0, 0.0, 1.0),
        SlotBlend::Alpha,
    );
}

// ---------------------------------------------------------------------------
// RenderStackedRings (103)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderStackedRings` -
/// `EffectPrimitiveRendering.cpp:3645-3729`.
///
/// `TexturedMark`'s stacked rings: for each live slot, `flag1[0]` rings (10 by
/// default, 14 for the tall kind 10) spaced 45 degrees apart, descending by
/// `height[1]` and shrinking by `(2.2 - i * 0.2) * distance` per ring. Ring 0
/// sits at `-height[0]`; each ring is a quad whose top edge (`vec3`/`vec4`) is
/// one unit above its bottom edge (`vec1`/`vec2`), submitted through the
/// 18-argument `RenderTeiRect` with `f1 = 0` (the corners are relative to the
/// primitive's `render.position`).
///
/// `flag1[4]` (the `TexturedMark` kind) selects the colour and blend: `pw = 0`
/// is the `ONE` destination blend ([`SlotBlend::AlphaOne`]), `pw = 1` the
/// straight alpha blend ([`SlotBlend::Alpha`]). Kind 11 submits two overlaid
/// quads (white at `alphaB / 3`, red at `alphaB`); kind 8 divides the alpha by
/// 10. Kinds 9 and every other value emit nothing, exactly like the original's
/// empty `default`.
pub fn render_stacked_rings(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 || slot.alpha_b <= 0 {
            continue;
        }

        let mut y = -slot.height[0];
        let mut radius = slot.max_height;

        for i in 0..slot.flag1[0].max(0) as i32 {
            if i > 0 {
                y += slot.height[1];
                radius -= (2.2 - i as f32 * 0.2) * slot.distance;
            }

            // `short angle1 = rotStart + 45 * i;`
            // `if (angle1 >= 360) angle1 = (unsigned short)angle1 % 360;`
            let mut angle1 = slot.rot_start as i32 + 45 * i;
            if angle1 >= 360 {
                angle1 = (angle1 as u16 % 360) as i32;
            }

            let vec1 = [radius * table_cos(angle1), y - 1.0, radius * table_sin(angle1)];

            // The remaining corners only ever subtract one full turn; the
            // original applies the same single-wrap to each.
            let mut angle2 = angle1 + 90;
            if angle2 >= 360 {
                angle2 -= 360;
            }

            let vec2 = [radius * table_cos(angle2), y - 1.0, radius * table_sin(angle2)];

            let mut angle3 = angle2 + 90;
            if angle3 >= 360 {
                angle3 -= 360;
            }

            let vec3 = [radius * table_cos(angle3), y, radius * table_sin(angle3)];

            let mut angle4 = angle3 + 90;
            if angle4 >= 360 {
                angle4 -= 360;
            }

            let vec4 = [radius * table_cos(angle4), y, radius * table_sin(angle4)];

            let quad = [vec1, vec2, vec3, vec4];
            let alpha = slot.alpha_b;

            // `switch (slot.flag1[4])`
            match slot.flag1[4] {
                // `0, 0, 0, 50, 50, 255, 0, 1, 0, 1, 0`
                0 => stacked_ring(out, state, alpha, quad, (50, 50, 255), SlotBlend::AlphaOne),
                // `1, 0, 0, 164, 136, 55, ...`
                1 => stacked_ring(out, state, alpha, quad, (164, 136, 55), SlotBlend::Alpha),
                2 => stacked_ring(out, state, alpha, quad, (155, 255, 155), SlotBlend::Alpha),
                3 => stacked_ring(out, state, alpha, quad, (130, 130, 255), SlotBlend::Alpha),
                4 => stacked_ring(out, state, alpha, quad, (255, 50, 50), SlotBlend::Alpha),
                5 => stacked_ring(out, state, alpha, quad, (5, 5, 5), SlotBlend::Alpha),
                6 => stacked_ring(out, state, alpha, quad, (255, 255, 255), SlotBlend::Alpha),
                7 => stacked_ring(out, state, alpha, quad, (255, 125, 255), SlotBlend::Alpha),
                // `static_cast<short>(alphaB / 10)`
                8 => stacked_ring(out, state, alpha / 10, quad, (225, 225, 255), SlotBlend::Alpha),
                // `0, 0, 0, 125, 125, 255, ...`
                10 => stacked_ring(out, state, alpha, quad, (125, 125, 255), SlotBlend::AlphaOne),
                11 => {
                    // `static_cast<short>(alphaB / 3)` white, then `alphaB` red.
                    stacked_ring(out, state, alpha / 3, quad, (255, 255, 255), SlotBlend::Alpha);
                    stacked_ring(out, state, alpha, quad, (255, 55, 55), SlotBlend::AlphaOne);
                }
                _ => {}
            }
        }
    }
}

/// One 18-argument `RenderTeiRect` call of [`render_stacked_rings`].
///
/// Every call passes `f1 = 0` (add `render.position` to the corners),
/// `tn = 0` (the primitive's first texture), and the full `0, 1` texture
/// window.
fn stacked_ring(out: &mut Vec<SlotTriangle>, state: &EffectPrimitiveState, alpha: i16, quad: Quad, colour: (u8, u8, u8), blend: SlotBlend) {
    emit_tei_rect_18(state, out, alpha, quad, 0, true, colour, (0.0, 1.0, 0.0, 1.0), blend);
}

// ---------------------------------------------------------------------------
// RenderPeongUp (96)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderPeongUp` - `EffectPrimitiveRendering.cpp:2227-2263`.
///
/// One `distance`-sized square per live slot at the slot's 45/135/225/315
/// degree diagonals, turned towards the screen by `slots[0..1].alphaT`
/// (`Tei_TowardScreen`), offset by `slot.vecBPre` and then by
/// `render.position` (`RenderTeiRect`'s `f1 = 0`). The quad is submitted only
/// while its first corner is still at or below `y = 0`.
///
/// `slot.flag1[0]` selects the colour: `2` -> `105,105,225`, `3` ->
/// `55,55,55`, anything else -> white. Every call uses `pw = 0` (the
/// `SRCALPHA`/`ONE` glow blend), `tn = 0` and the full `0, 1` texture window.
///
/// `facing` is `[slots[0].alphaT, slots[1].alphaT]` after `FaceCamera`. The
/// live render path passes the values recovered from the camera (the
/// simulation has no camera); `triangles` passes the values `PrimPeongUp`
/// stored.
pub fn render_peong_up(state: &EffectPrimitiveState, facing: [i16; 2], out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 {
            continue;
        }

        let radius = slot.distance;
        let mut corners = [
            [table_cos(45) * radius, 0.0, table_sin(45) * radius],
            [table_cos(135) * radius, 0.0, table_sin(135) * radius],
            [table_cos(225) * radius, 0.0, table_sin(225) * radius],
            [table_cos(315) * radius, 0.0, table_sin(315) * radius],
        ];

        tei_toward_screen(&mut corners, facing[0], facing[1]);

        let base = slot.vec_b_pre;
        let quad = corners.map(|corner| add(corner, base));

        if quad[0][1] <= 0.0 {
            let colour = match slot.flag1[0] {
                2 => (105, 105, 225),
                3 => (55, 55, 55),
                _ => (255, 255, 255),
            };

            emit_tei_rect_18(
                state,
                out,
                slot.alpha_b,
                quad,
                0,
                true,
                colour,
                (0.0, 1.0, 0.0, 1.0),
                SlotBlend::AlphaOne,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// RenderPentagon (66)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderPentagon` - `EffectPrimitiveRendering.cpp:2342-2422`.
///
/// `ForestLight`'s pentagon: six sweep positions (`rotStart + point * 72`,
/// wrapped once) between the slot's `vecTNow` (bottom ring) and `vecTPre` (top
/// ring), each `maxHeight` wide. `flag1[0] == 13` selects the blue colour.
pub fn render_pentagon(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 || slot.alpha_b <= 0 {
            continue;
        }

        let top = slot.vec_t_pre;

        let mut previous_bottom = [0.0f32; 3];
        let mut previous_top = [0.0f32; 3];

        for point in 0..=5i32 {
            let mut angle = slot.rot_start as i32 + point * 72;

            if angle >= 360 {
                angle -= 360;
            }

            let dx = slot.max_height * table_cos(angle);
            let dz = slot.max_height * table_sin(angle);

            let bottom = [dx + slot.vec_t_now[0], slot.vec_t_now[1], dz + slot.vec_t_now[2]];
            let top_point = [top[0] + dx, top[1], top[2] + dz];

            if point != 0 {
                let colour = if slot.flag1[0] == 13 { (80, 80, 255) } else { (230, 255, 230) };

                emit_tei_rect_18(
                    state,
                    out,
                    slot.alpha_b,
                    [previous_bottom, bottom, top_point, previous_top],
                    0,
                    false,
                    colour,
                    (0.0, 1.0, 0.0, 1.0),
                    SlotBlend::AlphaOne,
                );
            }

            previous_bottom = bottom;
            previous_top = top_point;
        }
    }
}

// ---------------------------------------------------------------------------
// RenderPentaRing (69)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderPentaRing` - `EffectPrimitiveRendering.cpp:1399-1477`.
///
/// Prim 69's five-sided ring pair: for each live slot and each of two passes
/// (`maxHeight`, then `maxHeight * 2` for kind 3 else `maxHeight + 0.25`), six
/// points at `72` degrees walk a ring around `vecBNow` (pass 0) up to
/// `vecTNow` (pass 1). The `direction = (rotStart + 90) % 360` rotates the
/// ring, and `height[4]` selects the colour. Every call uses the 18-argument
/// `RenderTeiRect` with `pw = 1` (the `INVSRCALPHA` queue), `f1 = 1` (the
/// corners are already absolute) and `extra = 1` for kinds 4/5 (near-plane
/// depth).
pub fn render_penta_ring(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    let mut previous_bottom = [0.0f32; 3];
    let mut previous_top = [0.0f32; 3];

    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 || slot.alpha_b <= 0 {
            continue;
        }

        let direction = (slot.rot_start as i32 + 90) % 360;

        for pass in 0..2 {
            let mut i = 0;
            let mut angle = 0;

            while angle <= 360 {
                let radius = if pass == 0 {
                    slot.max_height
                } else if slot.height[4] == 3.0 {
                    slot.max_height + slot.max_height
                } else {
                    slot.max_height + 0.25
                };

                let across = radius * table_cos(angle);
                let offset_x = across * table_cos(direction);
                let offset_z = across * table_sin(direction);
                let offset_y = radius * table_sin(angle) - slot.height[0];

                let bottom = [
                    offset_x + slot.vec_b_now[0],
                    offset_y + slot.vec_b_now[1],
                    offset_z + slot.vec_b_now[2],
                ];
                let top = [
                    offset_x + slot.vec_t_now[0],
                    offset_y + slot.vec_t_now[1],
                    offset_z + slot.vec_t_now[2],
                ];

                if i > 0 {
                    let (colour, near) = if pass == 0 {
                        if slot.height[4] == 2.0 {
                            ((255, 255, 255), false)
                        } else if slot.height[4] == 3.0 {
                            ((255, 89, 182), false)
                        } else if slot.height[4] == 4.0 {
                            ((55, 55, 255), true)
                        } else if slot.height[4] == 5.0 {
                            ((55, 55, 55), true)
                        } else {
                            ((255, 255, 55), false)
                        }
                    } else if slot.height[4] == 2.0 {
                        ((255, 255, 255), false)
                    } else if slot.height[4] == 3.0 {
                        ((255, 89, 182), false)
                    } else if slot.height[4] == 4.0 {
                        ((55, 55, 255), true)
                    } else if slot.height[4] == 5.0 {
                        ((55, 55, 55), true)
                    } else {
                        ((255, 255, 200), false)
                    };

                    let quad = [previous_bottom, bottom, top, previous_top];

                    if near {
                        emit_tei_rect_18_near(
                            state,
                            out,
                            slot.alpha_b,
                            quad,
                            0,
                            false,
                            colour,
                            (0.0, 1.0, 0.0, 1.0),
                            SlotBlend::Alpha,
                        );
                    } else {
                        emit_tei_rect_18(
                            state,
                            out,
                            slot.alpha_b,
                            quad,
                            0,
                            false,
                            colour,
                            (0.0, 1.0, 0.0, 1.0),
                            SlotBlend::Alpha,
                        );
                    }
                }

                previous_bottom = bottom;
                previous_top = top;
                i += 1;
                angle += 72;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// RenderHitLine3 (100)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderHitLine3` - `EffectPrimitiveRendering.cpp:1872-1962`.
///
/// `PP_HITLINE3`'s strands: for each live slot with a started `process`, the
/// motion's `height[0..vecTNow.y]` samples are converted into two vertical
/// curves (`vecA` from `height[j]`, `vecB` from `height[j + 1]`), a heading is
/// taken across the segment, and four cross-section points at `120` degrees
/// turn the pair into three quads. `alphaT` (the body's `kind`) selects the
/// colour ramp; kinds `>= 3` add the wider blue halo pass. Every call uses the
/// 18-argument `RenderTeiRect` with `pw = 0` (the `ONE` glow queue), `f1 = 1`
/// and `extra = 0`.
pub fn render_hitline3(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    let mut vec1 = [0.0f32; 3];
    let mut vec3 = [0.0f32; 3];

    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 || slot.alpha_b <= 0 || slot.process <= 0 {
            continue;
        }

        let mut j = 0i32;

        while (j as f32) < slot.vec_t_now[1] {
            let sample = j as usize;

            let mut speed = 0.0f32;

            if slot.height[sample] > 0.0 {
                speed += slot.height[sample] / 90.0 * slot.distance;
            }

            let a = [
                speed * table_cos(slot.rise_angle as i32),
                -slot.distance * table_sin(slot.height[sample] as i16 as i32) * slot.vec_t_now[0],
                speed * table_sin(slot.rise_angle as i32),
            ];

            speed = 0.0;

            if slot.height[sample + 1] > 0.0 {
                speed += slot.height[sample + 1] / 90.0 * slot.distance;
            }

            let b = [
                speed * table_cos(slot.rise_angle as i32),
                -slot.distance * table_sin(slot.height[sample + 1] as i16 as i32) * slot.vec_t_now[0],
                speed * table_sin(slot.rise_angle as i32),
            ];

            let mut angle = super::primitive::get_angle(a[0] - b[0], a[2] - b[2]) as i32;

            let a = add(a, slot.vec_b_now);
            let b = add(b, slot.vec_b_now);

            angle += 90;

            if angle >= 360 {
                angle -= 360;
            }

            for i in 0..=3i32 {
                let radius = slot.max_height;
                let tx = radius * table_cos(i * 120);
                let ry = radius * table_sin(i * 120) - 5.0;
                let rx = tx * table_cos(angle);
                let rz = tx * table_sin(angle);
                let vec2 = [a[0] + rx, a[1] + ry, a[2] + rz];
                let vec4 = [b[0] + rx, b[1] + ry, b[2] + rz];

                if i > 0 {
                    let alpha = i32::from(slot.alpha_b) - j * 10;

                    if alpha > 0 {
                        let colour = if slot.alpha_t == 0 {
                            Some((255, 200, (150 - (5 - j) * 30) as u8))
                        } else if slot.alpha_t == 1 {
                            let value = 120 - (5 - j) * 22;
                            Some((value as u8, value as u8, 255))
                        } else if slot.alpha_t == 2 {
                            Some((255, 200, (150 - (3 - j) * 50) as u8))
                        } else if slot.alpha_t >= 3 {
                            let grey = 255 - (5 - j) * 30;
                            Some((grey as u8, grey as u8, grey as u8))
                        } else {
                            None
                        };

                        if let Some(colour) = colour {
                            emit_tei_rect_18(
                                state,
                                out,
                                alpha as i16,
                                [vec2, vec4, vec3, vec1],
                                0,
                                false,
                                colour,
                                (0.0, 1.0, 0.0, 1.0),
                                SlotBlend::AlphaOne,
                            );
                        }
                    }
                }

                vec1 = vec2;
                vec3 = vec4;
            }

            if slot.alpha_t >= 3 {
                for i in 0..=3i32 {
                    let radius = slot.max_height * 4.0;
                    let tx = radius * table_cos(i * 120);
                    let ry = radius * table_sin(i * 120) - 5.0;
                    let rx = tx * table_cos(angle);
                    let rz = tx * table_sin(angle);
                    let vec2 = [a[0] + rx, a[1] + ry, a[2] + rz];
                    let vec4 = [b[0] + rx, b[1] + ry, b[2] + rz];

                    if i > 0 {
                        let alpha = ((i32::from(slot.alpha_b) - j * 10) as f32 * 0.5) as i32;

                        if alpha > 0 && slot.alpha_t == 3 {
                            emit_tei_rect_18(
                                state,
                                out,
                                alpha as i16,
                                [vec2, vec4, vec3, vec1],
                                0,
                                false,
                                (0, 0, 255),
                                (0.0, 1.0, 0.0, 1.0),
                                SlotBlend::AlphaOne,
                            );
                        }
                    }

                    vec1 = vec2;
                    vec3 = vec4;
                }
            }

            j += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// RenderFanBeam (89)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderFanBeam` - `EffectPrimitiveRendering.cpp:3563-3645`.
///
/// `TexturedQuadSpin`'s two fans at `rotStart`: a narrow beam (`±2`, `±1` for
/// mode 2) and a wide one (`±5`, `±2`), both from the slot's `vecBNow`-relative
/// origin at `height[0]`. `height[4]` selects the colour and the blend; modes 6
/// are forced onto the near plane (`extra = 1`).
pub fn render_fan_beam(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 || slot.alpha_b <= 0 {
            continue;
        }

        let mode = slot.height[4];
        let length = slot.distance;
        let lift = slot.height[0];
        let rot_cos = table_cos(slot.rot_start as i32);
        let rot_sin = table_sin(slot.rot_start as i32);

        let emit = |out: &mut Vec<SlotTriangle>, half: i32, glow: bool, colour: (u8, u8, u8), extra: bool| {
            let mut left = slot.rise_angle as i32 - half;
            if left < 0 {
                left += 360;
            }
            let mut right = slot.rise_angle as i32 + half;
            if right >= 360 {
                right -= 360;
            }

            let corner1 = [
                rot_cos * (length * table_cos(left)),
                rot_sin * (length * table_cos(left)) + lift,
                length * table_sin(left),
            ];
            let corner2 = [
                length * table_cos(right) * rot_cos,
                length * table_cos(right) * rot_sin + lift,
                length * table_sin(right),
            ];
            let apex = [0.0, lift, 0.0];

            let start = out.len();
            emit_tei_rect_18(
                state,
                out,
                slot.alpha_b,
                [corner1, corner2, apex, apex],
                0,
                true,
                colour,
                (0.0, 1.0, 0.0, 1.0),
                if glow { SlotBlend::AlphaOne } else { SlotBlend::Alpha },
            );

            if extra {
                for triangle in &mut out[start..] {
                    triangle.forced_depth = Some(1.0);
                }
            }
        };

        let narrow = if mode == 2.0 { 1 } else { 2 };
        let (narrow_colour, narrow_glow, narrow_extra) = if mode == 2.0 {
            ((0, 0, 250), true, false)
        } else if mode == 3.0 {
            ((250, 250, 250), true, false)
        } else if mode == 4.0 {
            ((0, 0, 250), true, false)
        } else if mode == 5.0 {
            ((50, 50, 50), true, false)
        } else if mode == 6.0 {
            ((0, 0, 255), true, true)
        } else {
            ((0, 250, 250), true, false)
        };
        emit(out, narrow, narrow_glow, narrow_colour, narrow_extra);

        let wide = if mode == 2.0 { 2 } else { 5 };
        let (wide_colour, wide_glow, wide_extra) = if mode == 2.0 {
            ((250, 250, 0), true, false)
        } else if mode == 3.0 {
            ((250, 250, 250), false, false)
        } else if mode == 4.0 {
            ((250, 250, 0), false, false)
        } else if mode == 5.0 {
            ((250, 250, 250), false, false)
        } else if mode == 6.0 {
            ((50, 50, 255), true, true)
        } else {
            ((250, 0, 0), true, false)
        };
        emit(out, wide, wide_glow, wide_colour, wide_extra);
    }
}

// ---------------------------------------------------------------------------
// RenderHaloLayers (68)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderHaloLayers` - `EffectPrimitiveRendering.cpp:4140-4265`.
///
/// `PP_ASURACASTING`'s stacked fan layers. Every live slot draws ten quads,
/// layer `i = 10` (outermost) down to `1`: the four corners sit on the
/// 45/135/225/315 degree diagonals at radius `step + distance`, where `step`
/// is `10 - i` (times three unless `flag1[4]` is 2/4/5), all lifted by
/// `height[0]` and turned towards the screen by `FaceCamera`
/// (`Tei_TowardScreen`). The quad is then dropped by 30 units (40 for kinds
/// 2/4/6/7).
///
/// `slot.flag1[4]` selects the colour and the blend:
///
/// * `0` - black, `pw = 1`,
/// * `3` - dark grey; layer 10 draws at half alpha and again at full alpha,
/// * `5` - white, `pw = 1`, plus a second white pass at 1.1x radius,
/// * `6`/`7` - white, `pw = 0` (glow), plus a second pass at 1.1x radius (kind
///   7 red-tinted, otherwise blue-tinted),
/// * anything else - white, `pw = 1`.
///
/// `slot.flag1[0]` picks the texture (`state.textures[flag1[0]]`).
///
/// The original writes the four rotated corners and the 1.1x scaling back into
/// `slots[ec].vecBPre/vecBNow/vecTNow/vecTPre`. Nothing reads those vectors
/// afterwards (the motion body does not), so the port keeps them local.
pub fn render_halo_layers(state: &EffectPrimitiveState, facing: [i16; 2], out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 {
            continue;
        }

        for i in (1..=10usize).rev() {
            let mode = slot.flag1[4];

            let (angle1, angle2) = if mode == 6 || mode == 7 { (35, 145) } else { (45, 135) };

            let mut step = 10 - i as i32;

            if mode != 2 && mode != 4 && mode != 5 {
                step *= 3;
            }

            let radius = step as f32 + slot.distance;

            let b_pre = [radius * table_cos(angle1), 0.0, table_sin(angle1) * radius];
            let b_now = [radius * table_cos(angle2), 0.0, table_sin(angle2) * radius];

            // `unsigned angle3 = angle2 + (kinds 6/7 ? 70 : 90); if (>= 360) -= 360;`
            let mut angle3 = angle2 + if mode == 6 || mode == 7 { 70 } else { 90 };

            if angle3 >= 360 {
                angle3 -= 360;
            }

            let t_now = [radius * table_cos(angle3), 0.0, table_sin(angle3) * radius];

            // `int angle4 = angle3 + (kinds 6/7 ? 110 : 90); if (>= 360) -= 360;`
            let mut angle4 = angle3 + if mode == 6 || mode == 7 { 110 } else { 90 };

            if angle4 >= 360 {
                angle4 -= 360;
            }

            let t_pre = [radius * table_cos(angle4), 0.0, table_sin(angle4) * radius];

            let base = slot.height[0];
            let mut corners = [
                [base + b_pre[0], 0.0, b_pre[2]],
                [base + b_now[0], 0.0, b_now[2]],
                [base + t_now[0], 0.0, t_now[2]],
                [base + t_pre[0], 0.0, t_pre[2]],
            ];

            tei_toward_screen(&mut corners, facing[0], facing[1]);

            let lift = if mode == 2 || mode == 4 || mode == 6 || mode == 7 {
                40.0
            } else {
                30.0
            };

            for corner in &mut corners {
                corner[1] -= lift;
            }

            // `RenderTeiRect(..., vec2, vec1, vec4, vec3, ...)`.
            let quad = [corners[1], corners[0], corners[3], corners[2]];
            let texture_index = slot.flag1[0] as u8 as usize;
            let alpha = slot.height[i] as i32;

            if alpha > 0 {
                match mode {
                    0 => halo_layer_quad(out, state, quad, alpha as i16, texture_index, (0, 0, 0), SlotBlend::Alpha),
                    3 => {
                        if i == 10 {
                            halo_layer_quad(
                                out,
                                state,
                                quad,
                                (alpha / 2) as i16,
                                texture_index,
                                (10, 10, 10),
                                SlotBlend::Alpha,
                            );
                        }

                        halo_layer_quad(out, state, quad, alpha as i16, texture_index, (10, 10, 10), SlotBlend::Alpha);
                    }
                    5 => halo_layer_quad(out, state, quad, alpha as i16, texture_index, (255, 255, 255), SlotBlend::Alpha),
                    6 | 7 => halo_layer_quad(
                        out,
                        state,
                        quad,
                        alpha as i16,
                        texture_index,
                        (255, 255, 255),
                        SlotBlend::AlphaOne,
                    ),
                    _ => halo_layer_quad(out, state, quad, alpha as i16, texture_index, (255, 255, 255), SlotBlend::Alpha),
                }
            }

            if mode == 5 || mode == 6 || mode == 7 {
                let scaled = |corner: [f32; 3]| [corner[0] * 1.1, corner[1] * 1.1, corner[2] * 1.1];

                let mut outer = [
                    add_scaled_height(base, scaled(b_pre)),
                    add_scaled_height(base, scaled(b_now)),
                    add_scaled_height(base, scaled(t_now)),
                    add_scaled_height(base, scaled(t_pre)),
                ];

                tei_toward_screen(&mut outer, facing[0], facing[1]);

                let outer_lift = if mode == 5 { 30.0 } else { 40.0 };

                for corner in &mut outer {
                    corner[1] -= outer_lift;
                }

                let quad = [outer[1], outer[0], outer[3], outer[2]];
                let outer_alpha = slot.height[i] as i32;

                if outer_alpha > 0 {
                    let colour = if mode == 7 { (255, 5, 5) } else { (5, 5, 255) };

                    halo_layer_quad(out, state, quad, outer_alpha as i16, texture_index, colour, SlotBlend::AlphaOne);
                }
            }
        }
    }
}

/// `{corner.x + height[0], corner.y, corner.z}` for the outer pass of
/// [`render_halo_layers`]: the original adds the lift to `x` only
/// (`vec1.Set(vecBPre.x + height[0], vecBPre.y, vecBPre.z)`).
fn add_scaled_height(height: f32, corner: [f32; 3]) -> [f32; 3] {
    [corner[0] + height, corner[1], corner[2]]
}

/// One 18-argument `RenderTeiRect` call of [`render_halo_layers`].
///
/// `f1 = 0` (the corners are relative to `render.position`), full `0, 1`
/// texture window, and the caller's `pw`/colour/`tn`.
#[allow(clippy::too_many_arguments)]
fn halo_layer_quad(
    out: &mut Vec<SlotTriangle>,
    state: &EffectPrimitiveState,
    quad: Quad,
    alpha: i16,
    texture_index: usize,
    colour: (u8, u8, u8),
    blend: SlotBlend,
) {
    emit_tei_rect_18(
        state,
        out,
        alpha,
        quad,
        texture_index as i32,
        true,
        colour,
        (0.0, 1.0, 0.0, 1.0),
        blend,
    );
}

// ---------------------------------------------------------------------------
// RenderOrbitIcons (35)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderOrbitIcons` - `EffectPrimitiveRendering.cpp:4764-4897`.
///
/// `SuperSlashFan`'s four orbiting icon quads. Each live slot keeps four
/// independent icon records, one per slot vector, in `height[4k..4k+3]`:
/// `height[4k]` is the icon height, `height[4k+1]` the alpha, `height[4k+2]`
/// the texture index and `height[4k+3]` the orbit angle. Every record is a
/// square at the slot's `distance` radius, turned towards the screen by
/// `slots[0..1].alphaT` (`FaceCamera`), offset by the slot vector
/// (`vecBPre`/`vecBNow`/`vecTPre`/`vecTNow`, whose `y` holds the body's
/// random vertical offset) and by `-height[20]` (the caster's name offset).
///
/// The back-facing test is
/// `(height[4k+2] == 0 || height[4k+2] == g_effectIconOuterRadius) ==
/// ((angle - roll) in (-80, 70) || |angle - roll| > 270)`: when it holds the
/// quad keeps the 45/135/225/315 order, otherwise corners 2/1 and 4/3 are
/// swapped so the icon stays readable from the other side.
///
/// When `flag1[4] == 1` the colour switches to red for every *later* slot too
/// (the original overwrites its `red`/`green`/`blue` parameters).
///
/// `extra == 2`'s reciprocal-depth nudge is not reproduced; see the module
/// header's deviations.
#[allow(clippy::too_many_arguments)]
pub fn render_orbit_icons(
    state: &EffectPrimitiveState,
    facing: [i16; 2],
    scale: f32,
    mut red: u8,
    mut green: u8,
    mut blue: u8,
    out: &mut Vec<SlotTriangle>,
) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 {
            continue;
        }

        if slot.flag1[4] == 1 {
            red = 255;
            green = 20;
            blue = 20;
        }

        let rx = scale * slot.distance;

        let mut corners = [
            [rx * table_cos(45), 0.0, rx * table_sin(45)],
            [rx * table_cos(135), 0.0, rx * table_sin(135)],
            [rx * table_cos(225), 0.0, rx * table_sin(225)],
            [rx * table_cos(315), 0.0, rx * table_sin(315)],
        ];

        let rot0 = facing[0];

        tei_toward_screen(&mut corners, rot0, facing[1]);

        let base = corners;

        // `(angle index, texture index, alpha index, owner vector)`, in the
        // original's four blocks: `vecBPre`, `vecBNow`, `vecTPre`, `vecTNow`.
        let icons: [(usize, usize, usize, [f32; 3]); 4] = [
            (3, 2, 1, slot.vec_b_pre),
            (7, 6, 5, slot.vec_b_now),
            (11, 10, 9, slot.vec_t_pre),
            (15, 14, 13, slot.vec_t_now),
        ];

        for (angle_index, texture_index, alpha_index, owner) in icons {
            let angle = slot.height[angle_index] as i32;

            // `slot.vecBPre.x = cos(angle0) * g_effectIconRadius;
            //  slot.vecBPre.z = sin(angle0) * g_effectIconRadius;`
            // Only `x`/`z` are rewritten; `y` keeps the body's value.
            let offset = [
                table_cos(angle) * EFFECT_ICON_RADIUS,
                owner[1],
                table_sin(angle) * EFFECT_ICON_RADIUS,
            ];

            let difference = (slot.height[angle_index] - rot0 as f32) as i32;
            let icon_faces_us = (difference < 70 && difference > -80) || !(-270..=270).contains(&difference);

            let texture = slot.height[texture_index];
            let keep_order = (texture == 0.0 || texture == EFFECT_ICON_OUTER_RADIUS) == icon_faces_us;

            let mut quad = if keep_order {
                [
                    add(base[0], offset),
                    add(base[1], offset),
                    add(base[2], offset),
                    add(base[3], offset),
                ]
            } else {
                [
                    add(base[1], offset),
                    add(base[0], offset),
                    add(base[3], offset),
                    add(base[2], offset),
                ]
            };

            let name_offset = slot.height[20];

            for corner in &mut quad {
                corner[1] -= name_offset;
            }

            let alpha = slot.height[alpha_index] as i32 as i16;
            let texture_slot_index = (slot.height[texture_index] as i8).max(0) as usize;

            emit_tei_rect_18(
                state,
                out,
                alpha,
                quad,
                texture_slot_index as i32,
                true,
                (red, green, blue),
                (0.0, 1.0, 0.0, 1.0),
                SlotBlend::AlphaOne,
            );
        }
    }
}

/// `g_effectIconRadius` - `EffectPrimitiveRendering.cpp:43`.
const EFFECT_ICON_RADIUS: f32 = 4.0;
/// `g_effectIconOuterRadius` - `EffectPrimitiveRendering.cpp:40`.
const EFFECT_ICON_OUTER_RADIUS: f32 = 2.0;

// ---------------------------------------------------------------------------
// RenderDoubleGumGang (99)
// ---------------------------------------------------------------------------

/// `const float length = 4.0f;` in `RenderDoubleGumGang`
/// (`EffectPrimitiveRendering.cpp:7161`). Equal to `g_effectIconRadius`, but
/// the original spells it as a local literal.
const DOUBLE_GUMGANG_ICON_LENGTH: f32 = 4.0;

/// `CEffectPrim::RenderDoubleGumGang` -
/// `EffectPrimitiveRendering.cpp:7106-7363`.
///
/// `PP_DOUBLEGUMGANG`'s three nested `superN.bmp` sheets. Each live slot draws
/// twelve quads: for `i = 0..2` (scale `1.0`, `1.2`, `2.0`) it emits one quad
/// per slot vector (`vecBPre`, `vecBNow`, `vecTPre`, `vecTNow`), so four live
/// slots make 48 quads / 96 triangles.
///
/// The base corners sit on the 45/135/225/315 degree diagonals at radius
/// `distance * rdist` and are turned towards the screen by `Tei_TowardScreen`
/// with `slots[0..1].alphaT` (prim 99 shares `PrimFloatingBubbles`, which runs
/// `FaceCamera`). The per-vector offsets are rebuilt at a fixed `length = 4.0`
/// from `height[3]`/`[7]`/`[11]`/`[15]`; their `y` keeps the body's random
/// vertical offset (`SuperSlashSpread`). Every corner is then dropped by
/// `height[20]` (the caster name offset).
///
/// `slot.flag1[4]` selects the colours: `0` blue, `1` grey, anything else red
/// (`EF_DOUBLEGUMGANG` passes 2, `...2` 1, `...3` 0).
///
/// The alpha is `min(height[1|5|9|13], slot.process)`: `SuperSlashSpread` seeds
/// 40/80/120/160, so the rings fade in one unit per frame. The texture is
/// `height[2|6|10|14]`, and the corner order flips when
/// `(texture == 0 || texture == 2) != visible-range`, exactly like
/// `RenderOrbitIcons`.
///
/// `RenderTeiRect`'s `extra == 2` reciprocal-depth nudge is not reproduced (see
/// the module header's deviations). Nothing here writes back into the slot's
/// `vec*` fields; the motion body never reads them.
pub fn render_double_gum_gang(state: &EffectPrimitiveState, facing: [i16; 2], out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 {
            continue;
        }

        // The original rebuilds the four vectors on every `i`; their angle
        // indices do not depend on `i`, so the port hoists them. Only `x`/`z`
        // are rewritten - `y` keeps the body's `-ClampedRandomAtZero(10)`
        // offset.
        let offsets: [[f32; 3]; 4] = [
            [
                table_cos(slot.height[3] as i32) * DOUBLE_GUMGANG_ICON_LENGTH,
                slot.vec_b_pre[1],
                table_sin(slot.height[3] as i32) * DOUBLE_GUMGANG_ICON_LENGTH,
            ],
            [
                table_cos(slot.height[7] as i32) * DOUBLE_GUMGANG_ICON_LENGTH,
                slot.vec_b_now[1],
                table_sin(slot.height[7] as i32) * DOUBLE_GUMGANG_ICON_LENGTH,
            ],
            [
                table_cos(slot.height[11] as i32) * DOUBLE_GUMGANG_ICON_LENGTH,
                slot.vec_t_pre[1],
                table_sin(slot.height[11] as i32) * DOUBLE_GUMGANG_ICON_LENGTH,
            ],
            [
                table_cos(slot.height[15] as i32) * DOUBLE_GUMGANG_ICON_LENGTH,
                slot.vec_t_now[1],
                table_sin(slot.height[15] as i32) * DOUBLE_GUMGANG_ICON_LENGTH,
            ],
        ];

        for i in 0..3usize {
            let (red, green, blue, rdist) = double_gum_gang_color(slot.flag1[4], i);
            let rx = slot.distance * rdist;

            let mut corners = [
                [table_cos(45) * rx, 0.0, table_sin(45) * rx],
                [table_cos(135) * rx, 0.0, table_sin(135) * rx],
                [table_cos(225) * rx, 0.0, table_sin(225) * rx],
                [table_cos(315) * rx, 0.0, table_sin(315) * rx],
            ];

            tei_toward_screen(&mut corners, facing[0], facing[1]);

            let base = corners;

            // `(angle index, texture index, alpha index, owner vector)` in the
            // original's block order: `vecBPre`, `vecBNow`, `vecTPre`,
            // `vecTNow`.
            let icons: [(usize, usize, usize, [f32; 3]); 4] = [
                (3, 2, 1, offsets[0]),
                (7, 6, 5, offsets[1]),
                (11, 10, 9, offsets[2]),
                (15, 14, 13, offsets[3]),
            ];

            for (angle_index, texture_index, alpha_index, offset) in icons {
                let difference = (slot.height[angle_index] - facing[0] as f32) as i32;
                let icon_faces_us = (difference < 70 && difference > -80) || !(-270..=270).contains(&difference);

                let texture = slot.height[texture_index];
                let keep_order = (texture == 0.0 || texture == EFFECT_ICON_OUTER_RADIUS) == icon_faces_us;

                let mut quad = if keep_order {
                    [
                        add(base[0], offset),
                        add(base[1], offset),
                        add(base[2], offset),
                        add(base[3], offset),
                    ]
                } else {
                    [
                        add(base[1], offset),
                        add(base[0], offset),
                        add(base[3], offset),
                        add(base[2], offset),
                    ]
                };

                let name_offset = slot.height[20];

                for corner in &mut quad {
                    corner[1] -= name_offset;
                }

                // `count = (int)height[...]; if (count > process) count = process;`
                let alpha = (slot.height[alpha_index] as i32).min(slot.process) as i16;
                let texture_slot_index = (slot.height[texture_index] as i8).max(0) as usize;

                emit_tei_rect_18(
                    state,
                    out,
                    alpha,
                    quad,
                    texture_slot_index as i32,
                    true,
                    (red, green, blue),
                    (0.0, 1.0, 0.0, 1.0),
                    SlotBlend::AlphaOne,
                );
            }
        }
    }
}

/// The colour/scale table of [`render_double_gum_gang`]
/// (`EffectPrimitiveRendering.cpp:7126-7177`).
fn double_gum_gang_color(kind: i8, layer: usize) -> (u8, u8, u8, f32) {
    const BLUE: [(u8, u8, u8, f32); 3] = [(100, 100, 255, 1.0), (0, 0, 120, 1.2), (0, 0, 120, 2.0)];
    const GREY: [(u8, u8, u8, f32); 3] = [(90, 90, 80, 1.0), (60, 60, 50, 1.2), (30, 30, 20, 2.0)];
    const RED: [(u8, u8, u8, f32); 3] = [(255, 100, 100, 1.0), (120, 0, 0, 1.2), (120, 0, 0, 2.0)];

    match kind {
        0 => BLUE[layer],
        1 => GREY[layer],
        _ => RED[layer],
    }
}

// ---------------------------------------------------------------------------
// RenderTeiFacingSlots (82)
// ---------------------------------------------------------------------------

/// `FaceCamera` - `EffectPrimitiveMotion.cpp:100-114`.
///
/// The original reads the game camera's `camera.yaw`/`camera.pitch` floats,
/// truncates each into a `short` and derives the two angles
/// `Tei_TowardScreen` rotates by:
///
/// ```text
/// slots[0].alphaT = (short)camera.yaw;   slots[0].alphaT = 360 - it;
/// slots[1].alphaT = (short)camera.pitch; slots[1].alphaT = 360 - it + 90;
///                                        if (>= 360) -= 360;
/// ```
///
/// The port's [`Camera`] trait exposes the view direction rather than those two
/// floats, so the orbit angles are recovered from it. A camera orbited by
/// `(yaw, pitch)` looks along `(cos p sin y, -sin p, cos p cos y)`, hence
/// `yaw = atan2(direction.x, direction.z)` and `pitch = asin(-direction.y)`.
/// The yaw is wrapped into `[0, 360)` first, matching the client's
/// `CameraSmoothing`, which keeps `camera.yaw` in that range.
///
/// This is exactly the rotation `FaceCamera` is for: it turns the `+Y` normal
/// of the x/z-plane quad towards the camera, so the aura billboards.
pub fn face_camera_angles(camera: &dyn Camera) -> [i16; 2] {
    let direction = camera.view_direction();

    let yaw = direction.x.atan2(direction.z).to_degrees();
    let yaw = if yaw < 0.0 { yaw + 360.0 } else { yaw };
    let pitch = (-direction.y).clamp(-1.0, 1.0).asin().to_degrees();

    let scroll = 360 - i32::from(quantize_camera_angle(yaw));
    let mut tilt = 360 - i32::from(quantize_camera_angle(pitch)) + 90;

    if tilt >= 360 {
        tilt -= 360;
    }

    [scroll as i16, tilt as i16]
}

/// Applies the client's `(short)` truncation to an angle recovered from the
/// camera's view direction.
///
/// The `atan2`/`asin` round-trip has about `1e-5` degrees of error, which is
/// enough to push an exactly integral camera angle (the port's fixed
/// `view_angle = 180`, `CAMERA_PITCH = -55` defaults) just below the next
/// integer and truncate it one degree low. Snapping to `0.001` degrees first
/// removes only that numerical noise and leaves the client's truncation intact
/// for every real (non-boundary) camera angle.
fn quantize_camera_angle(degrees: f32) -> i16 {
    ((degrees * 1000.0).round() / 1000.0) as i16
}

/// `static_cast<float>(-30 * g_renderer->viewport.height / 480) *
/// this->render.distanceScale` from `RenderTeiRect`'s `extra == 3` branch.
///
/// The `-30 * height / 480` part is **integer** arithmetic before the cast
/// (`EffectPrimitiveRendering.cpp:900`), so e.g. 1080 px gives `-67`, not
/// `-67.5`.
pub fn face_camera_vertical_shift(viewport_height: i32, distance_scale: f32) -> f32 {
    ((-30 * viewport_height) / 480) as f32 * distance_scale
}

/// One 18-argument `RenderTeiRect` call of the `RenderTeiFacingSlots` switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FacingRect {
    /// `(red, green, blue)` of the call.
    colour: (u8, u8, u8),
    /// `pw`: `1` is `INVSRCALPHA` (plain alpha), `0` is `ONE` (the glow blend).
    pw_one: bool,
    /// `extra == 3`, the screen-space face-camera clamp.
    extra_three: bool,
}

const AURA_WHITE: (u8, u8, u8) = (255, 255, 255);

/// Shorthand for a [`FacingRect`] in the const tables below.
const fn aura_rect(colour: (u8, u8, u8), pw_one: bool, extra_three: bool) -> FacingRect {
    FacingRect {
        colour,
        pw_one,
        extra_three,
    }
}

/// `case 0:` / `case 3:`.
const AURA_RECTS_0_3: [FacingRect; 1] = [aura_rect(AURA_WHITE, true, false)];
/// `case 2:` falls through 4, 5, 10, 12 and 15.
const AURA_RECTS_2: [FacingRect; 6] = [
    aura_rect((130, 130, 250), false, false),
    aura_rect(AURA_WHITE, false, false),
    aura_rect((125, 255, 125), false, false),
    aura_rect((150, 150, 150), false, false),
    aura_rect((50, 50, 50), false, false),
    aura_rect((51, 153, 153), false, false),
];
/// `case 4:` falls through 5, 10, 12 and 15.
const AURA_RECTS_4: [FacingRect; 5] = [
    aura_rect(AURA_WHITE, false, false),
    aura_rect((125, 255, 125), false, false),
    aura_rect((150, 150, 150), false, false),
    aura_rect((50, 50, 50), false, false),
    aura_rect((51, 153, 153), false, false),
];
/// `case 5:` falls through 10, 12 and 15.
const AURA_RECTS_5: [FacingRect; 4] = [
    aura_rect((125, 255, 125), false, false),
    aura_rect((150, 150, 150), false, false),
    aura_rect((50, 50, 50), false, false),
    aura_rect((51, 153, 153), false, false),
];
/// `case 10:` falls through 12 and 15.
const AURA_RECTS_10: [FacingRect; 3] = [
    aura_rect((150, 150, 150), false, false),
    aura_rect((50, 50, 50), false, false),
    aura_rect((51, 153, 153), false, false),
];
/// `case 12:` falls through 15 (there is no `break;` after case 12).
const AURA_RECTS_12: [FacingRect; 2] = [aura_rect((50, 50, 50), false, false), aura_rect((51, 153, 153), false, false)];
const AURA_RECTS_15: [FacingRect; 1] = [aura_rect((51, 153, 153), false, false)];
const AURA_RECTS_16: [FacingRect; 1] = [aura_rect(AURA_WHITE, true, true)];
const AURA_RECTS_17: [FacingRect; 1] = [aura_rect(AURA_WHITE, false, false)];
const AURA_RECTS_18: [FacingRect; 2] = [aura_rect((5, 5, 255), false, false), aura_rect((155, 155, 255), false, false)];
const AURA_RECTS_19: [FacingRect; 2] = [aura_rect((5, 255, 5), false, false), aura_rect((155, 255, 155), false, false)];
const AURA_RECTS_20: [FacingRect; 2] = [aura_rect((255, 5, 5), false, false), aura_rect((255, 155, 155), false, false)];
const AURA_RECTS_32: [FacingRect; 1] = [aura_rect((255, 5, 5), false, false)];

/// The `switch (slots[ec].flag1[2])` of `RenderTeiFacingSlots`
/// (`EffectPrimitiveRendering.cpp:7878-7932`), encoded as the exact sequence of
/// `RenderTeiRect` calls each entry point emits.
///
/// The C++ switch has a real fall-through:
///
/// ```text
/// case 2:  ... // 130,130,250
/// case 4:  ... // 255,255,255
/// case 5:  ... // 125,255,125
/// case 10: ... // 150,150,150
/// case 12: ... //  50, 50, 50
/// case 15: ... //  51,153,153
///          break;
/// ```
///
/// so entering at 2 emits six rects, at 4 five, at 5 four, at 10 three, at 12
/// two and at 15 one. Note that **case 12 also falls into 15**: there is no
/// `break;` after the case 12 call. Cases 0/3, 16, 17, 18, 19, 20 and 32 have
/// their own `break`s; every other value emits nothing.
///
/// `kind = 16` is the only `extra == 3` call, and 0/3 and 16 are the only
/// `pw = 1` calls.
fn facing_rects(kind: i8) -> &'static [FacingRect] {
    match kind {
        0 | 3 => &AURA_RECTS_0_3,
        2 => &AURA_RECTS_2,
        4 => &AURA_RECTS_4,
        5 => &AURA_RECTS_5,
        10 => &AURA_RECTS_10,
        12 => &AURA_RECTS_12,
        15 => &AURA_RECTS_15,
        16 => &AURA_RECTS_16,
        17 => &AURA_RECTS_17,
        18 => &AURA_RECTS_18,
        19 => &AURA_RECTS_19,
        20 => &AURA_RECTS_20,
        32 => &AURA_RECTS_32,
        _ => &[],
    }
}

/// `Tei_TowardScreen`'s per-corner angle step of `RenderTeiFacingSlots`,
/// including the one-shot `if (angle < 0) angle += 360;` of the original.
fn advance_facing_angle(angle: i32, delta: i32) -> i32 {
    if delta == 0 {
        return angle;
    }

    let next = angle + delta;

    if next < 0 { next + 360 } else { next }
}

/// `CEffectPrim::RenderTeiFacingSlots` -
/// `EffectPrimitiveRendering.cpp:7779-7934`.
///
/// One quad per live slot: four points at `distance` (wobbled by `height[8]`
/// through the client sine table) around `fullDisplayAngle`. `height[10]` is
/// the shape (0, 1 or 2 select the angular spacing; any other value leaves all
/// four points coincident). The quad is rotated towards the screen with
/// `slots[0].alphaT`/`slots[1].alphaT`, offset by the slot's own `vecBNow`, and
/// emitted by the 18-argument `RenderTeiRect` with full-texture UVs and texture
/// index `slots[ec].flag1[0]`. The colour/blend/`extra` sequence comes from
/// [`facing_rects`]'s `switch (slots[ec].flag1[2])`, including its
/// fall-through.
///
/// `f1 = 1` in every call, so `RenderTeiRect` does *not* add
/// `render.position`: the base is the slot's `vecBNow`. `state.size` is not
/// read by this primitive - that table belongs to the 10-argument
/// `RenderTeiRect` overload.
///
/// **Known input gap (simulation, not this renderer).** The port's
/// `rag::TexturedAura` does not reproduce two writes of `RagEffect.cpp:17964`
/// and `:17965`: `flag1[4] = kind` (with `4 -> 1`) and
/// `height[15] = render.position.y - vecBPre.y`. Consequently the ported
/// motion runs its `flag1[4] == 0` alpha branch for every aura, and
/// `vecBNow.y` is the primitive position instead of the kinds' `-10`/`-30`/
/// head lift. The renderer consumes `vecBNow` and `flag1[2]` exactly; fixing
/// the body would place the aura correctly without any change here.
pub fn render_tei_facing_slots(state: &EffectPrimitiveState, context: Option<&SlotRenderContext<'_>>, out: &mut Vec<SlotTriangle>) {
    let facing = context.map_or([state.slots[0].alpha_t, state.slots[1].alpha_t], |context| context.facing);

    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 || slot.alpha_b <= 0 {
            continue;
        }

        let dist = slot.distance * table_sin(slot.height[8] as i32) * 0.05 + slot.distance;
        let shape = slot.height[10];

        let deltas = if shape == 0.0 {
            [-90, -90, -90]
        } else if shape == 1.0 {
            [-60, -120, -60]
        } else if shape == 2.0 {
            [-130, -50, -130]
        } else {
            [0, 0, 0]
        };

        let mut angles = [0i32; 4];
        angles[0] = i32::from(slot.full_display_angle);

        for index in 0..3 {
            angles[index + 1] = advance_facing_angle(angles[index], deltas[index]);
        }

        let mut corners: Quad = angles.map(|angle| [dist * table_cos(angle), 0.0, dist * table_sin(angle)]);

        let kind = slot.flag1[2];

        if kind == 3 {
            for corner in corners.iter_mut() {
                corner[0] += 3.1;
                corner[2] += 6.0;
            }
        } else if (17..=31).contains(&kind) {
            for corner in corners.iter_mut() {
                corner[2] += 6.0;
            }
        }

        tei_toward_screen(&mut corners, facing[0], facing[1]);

        let base = slot.vec_b_now;
        let corners = corners.map(|corner| add(corner, base));

        // `case 0` and `case 3` pass `(vec1, vec2, vec3, vec4)`; every other
        // case passes `(vec2, vec1, vec4, vec3)`.
        let quad = if kind == 0 || kind == 3 {
            corners
        } else {
            [corners[1], corners[0], corners[3], corners[2]]
        };

        let texture_index = i32::from(slot.flag1[0]);
        let alpha = slot.alpha_b;

        for rect in facing_rects(kind) {
            let blend = if rect.pw_one { SlotBlend::Alpha } else { SlotBlend::AlphaOne };

            if rect.extra_three {
                emit_tei_rect_extra_three(state, out, alpha, quad, texture_index, rect.colour, blend, context);
            } else {
                emit_tei_rect_18(
                    state,
                    out,
                    alpha,
                    quad,
                    texture_index,
                    false,
                    rect.colour,
                    (0.0, 1.0, 0.0, 1.0),
                    blend,
                );
            }
        }
    }
}

/// `RenderTeiRect`'s `extra == 3` branch
/// (`EffectPrimitiveRendering.cpp:891-908`, identical for the second triangle).
///
/// Projects the four corners to window pixels, rewrites the two triangles'
/// `x` symmetrically around the projected `render.position` (keeping each
/// triangle's original width), shifts `y` by
/// `(-30 * viewport.height / 480) * distanceScale` and forces
/// `z = 1e-09`/`rhw = 1`. The result is stored as NDC on the triangle.
///
/// Without a [`SlotRenderContext`] the projection cannot be reproduced; the
/// call is skipped (only reachable through the camera-less [`triangles`]
/// helper - the live render path always provides the context).
#[allow(clippy::too_many_arguments)]
fn emit_tei_rect_extra_three(
    state: &EffectPrimitiveState,
    out: &mut Vec<SlotTriangle>,
    alpha: i16,
    quad: Quad,
    texture_index: i32,
    (red, green, blue): (u8, u8, u8),
    blend: SlotBlend,
    context: Option<&SlotRenderContext<'_>>,
) {
    let Some(context) = context else {
        return;
    };

    let viewport = context.viewport;

    if viewport.width <= 0.0 || viewport.height <= 0.0 {
        return;
    }

    let Some(anchor) = project_window_pixel(context.camera, state.position, viewport) else {
        return;
    };

    let mut projected = [[0.0f32; 2]; 4];

    for (index, corner) in quad.iter().enumerate() {
        let Some(pixel) = project_window_pixel(context.camera, *corner, viewport) else {
            return;
        };

        projected[index] = pixel;
    }

    // `static_cast<float>(-30 * viewport.height / 480)`: the division is
    // integer arithmetic before the cast, then multiplied by distanceScale.
    let shift = face_camera_vertical_shift(viewport.height as i32, context.distance_scale);

    let first_width = (projected[0][0] - projected[1][0]).abs() * 0.5;
    let first = [
        [anchor[0] + first_width, projected[0][1] + shift],
        [anchor[0] - first_width, projected[1][1] + shift],
        [anchor[0] - first_width, projected[2][1] + shift],
    ];

    let second_width = (projected[3][0] - projected[2][0]).abs() * 0.5;
    let second = [
        [anchor[0] + second_width, projected[3][1] + shift],
        [anchor[0] - second_width, projected[2][1] + shift],
        [anchor[0] + second_width, projected[0][1] + shift],
    ];

    let to_ndc = |pixel: [f32; 2]| [pixel[0] / viewport.width * 2.0 - 1.0, 1.0 - pixel[1] / viewport.height * 2.0];

    let argb = pack_argb(alpha, red, green, blue);
    let texture = texture_slot(state, texture_index.max(0) as usize);
    let blend = resolve_blend(state.render_flag, blend);
    let uvs = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

    push_projected(out, first.map(to_ndc), [uvs[0], uvs[1], uvs[2]], argb, texture, blend);
    push_projected(out, second.map(to_ndc), [uvs[3], uvs[2], uvs[0]], argb, texture, blend);
}

/// Pushes one already-projected triangle (`RenderTeiRect`'s `extra == 3`).
fn push_projected(
    out: &mut Vec<SlotTriangle>,
    corners: [[f32; 2]; 3],
    texture_coordinates: [[f32; 2]; 3],
    argb: u32,
    texture: Option<&'static str>,
    blend: SlotBlend,
) {
    if out.len() >= MAX_TRIANGLES_PER_PRIMITIVE {
        return;
    }

    let mut triangle = SlotTriangle::new([[0.0; 3]; 3], texture_coordinates, argb, texture, blend);
    triangle.projected = Some(ProjectedTriangle { corners, depth: 1.0 });

    out.push(triangle);
}

/// `CRenderer::Project`'s window-space mapping without its implicit visibility
/// test (`Project` has no `w` guard; `RenderTeiRect`'s `extra == 3` relies on
/// that and forces `rhw = 1` afterwards).
///
/// Returns window pixels with a top-left origin, exactly like
/// [`project_point`](super::super::project_point), or `None` only when the
/// homogeneous divide is not finite.
fn project_window_pixel(camera: &dyn Camera, position: [f32; 3], viewport: ScreenSize) -> Option<[f32; 2]> {
    let clip_space_position = camera.view_projection_matrix() * Point3::new(position[0], position[1], position[2]).to_homogeneous();

    if clip_space_position.w.abs() <= f32::EPSILON {
        return None;
    }

    let inverse_w = 1.0 / clip_space_position.w;
    let normalized_x = clip_space_position.x * inverse_w;
    let normalized_y = clip_space_position.y * inverse_w;

    let pixel = [
        (normalized_x * 0.5 + 0.5) * viewport.width,
        (0.5 - normalized_y * 0.5) * viewport.height,
    ];

    (pixel[0].is_finite() && pixel[1].is_finite()).then_some(pixel)
}

/// The world-space corners of one [`SlotTriangle`] as [`Point3`]s (the shape
/// the effect renderer takes).
pub fn triangle_points(triangle: &SlotTriangle) -> [Point3<f32>; 3] {
    triangle.corners.map(|corner| Point3::new(corner[0], corner[1], corner[2]))
}

/// The texture coordinates of one [`SlotTriangle`] as [`Vector2`]s.
pub fn triangle_uvs(triangle: &SlotTriangle) -> [Vector2<f32>; 3] {
    triangle.texture_coordinates.map(|uv| Vector2::new(uv[0], uv[1]))
}

// ---------------------------------------------------------------------------
// Render3DCasting2 (34)
// ---------------------------------------------------------------------------

/// `CEffectPrim::Render3DCasting2` - `EffectPrimitiveRendering.cpp:6519-6592`,
/// dispatch `:521-522` (`PP_GI_3`, prim 34).
///
/// The Grand Cross ring sweep. Unlike `Render3DCasting` (which divides
/// `fullDisplayAngle` by 20), this renderer divides by **9**, so a 90-degree
/// ring is only nine segments and `order` never reaches the `order == 20`
/// clamp; the `basicAngle == 0` case would loop forever in the original and is
/// skipped here.
///
/// The original stores the previous iteration's `vecBNow`/`vecTNow` in the
/// slot's `vecBPre`/`vecTPre`, then overwrites them at the end of each
/// iteration. The first iteration (`order == 0`) never emits, so the previous
/// values are always the ones this same loop wrote one step earlier: the port
/// keeps them in locals and produces the same quads without mutating the
/// primitive state.
pub fn render_3d_casting2(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 {
            continue;
        }

        // `const int basicAngle = slot.fullDisplayAngle / 9;`
        let basic_angle = slot.full_display_angle as i32 / 9;

        // The original would spin forever when the body leaves the sweep
        // below nine degrees (`count += 0`).
        if basic_angle <= 0 {
            continue;
        }

        let mut order = 0usize;
        let mut previous_bottom = [0.0f32; 3];
        let mut previous_top = [0.0f32; 3];
        let mut count = 0i32;

        while count < slot.full_display_angle as i32 {
            let mut angle = (slot.rot_start as i32 + count) % 360;

            if slot.full_display_angle == 360 && order == 20 {
                angle = slot.rot_start as i32;
            }

            let cosine = table_cos(angle);
            let sine = table_sin(angle);

            // `vecBNow`
            let bottom = [cosine * slot.distance, 0.0, sine * slot.distance];

            let rx = table_cos(slot.rise_angle as i32) * slot.height[order];
            let ry = table_sin(slot.rise_angle as i32) * slot.height[order];

            // The per-slot 90/270/450/630-degree remap, applied to the *ring*
            // angle only. A negative result reads the mirrored trig table in
            // the original (out of bounds); the shared `table_cos`/`table_sin`
            // wrap it, a documented deviation.
            let mut remapped = match ec {
                2 => 90 - angle,
                1 => 270 - angle,
                0 => 450 - angle,
                _ => 630 - angle,
            };

            if remapped >= 360 {
                remapped -= 360;
            }

            let sin_remapped = table_sin(remapped);
            let cos_remapped = table_cos(remapped);
            let rz = sin_remapped * rx;
            let rx = cos_remapped * rx;

            // `vecTNow`
            let top = [bottom[0] + rx, -ry, bottom[2] + rz];

            let offset_x = slot.flag1[0] as f32;
            let offset_z = slot.flag1[1] as f32;
            let vec1 = [top[0] + offset_x, top[1], top[2] + offset_z];
            let vec2 = [bottom[0] + offset_x, bottom[1], bottom[2] + offset_z];
            let vec3 = [previous_top[0] + offset_x, previous_top[1], previous_top[2] + offset_z];
            let vec4 = [previous_bottom[0] + offset_x, previous_bottom[1], previous_bottom[2] + offset_z];

            if order > 0 {
                // `RenderTeiRect(&rpCnt, order - 1, alphaB, alphaB, vec4,
                // vec2, vec1, vec3, wtm, 0)`
                emit_tei_rect_10(state, out, order as i32 - 1, slot.alpha_b, [vec4, vec2, vec1, vec3], 0);
            }

            order += 1;
            if order >= 21 {
                order = 20;
            }

            previous_top = top;
            previous_bottom = bottom;
            count += basic_angle;
        }
    }
}

// ---------------------------------------------------------------------------
// RenderGrandCross2 (37)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderGrandCross2` - `EffectPrimitiveRendering.cpp:7365-7421`,
/// dispatch `:524-525` (`PP_GRANDCROSS2`, prim 37).
///
/// Per live slot with a positive `alphaB` this builds the four rotated base
/// corners from `height[0]`/`height[1]`, draws the four skirt quads with
/// `y = -1 - height[2]` (`pw = 1`, straight alpha), then the flat quad twice
/// (`pw = 0`, the glow queue) at `alphaB / 2`. All six calls pass `f1 = 0`, so
/// `render.position` is added by `RenderTeiRect`.
pub fn render_grand_cross2(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 || slot.alpha_b <= 0 {
            continue;
        }

        let cosine = table_cos(slot.rot_start as i32);
        let sine = table_sin(slot.rot_start as i32);
        let height = slot.height[0];
        let side = slot.height[1];
        let drop = slot.height[2];

        let corner = |rx: f32, rz: f32| [cosine * rx - sine * rz, -1.0, sine * rx + cosine * rz];
        let b_pre = corner(height, side);
        let b_now = corner(height, -side);
        let t_now = corner(-height, -side);
        let t_pre = corner(-height, side);

        let lowered = |point: [f32; 3]| [point[0], point[1] - drop, point[2]];
        let vec1 = lowered(b_pre);
        let vec2 = lowered(b_now);
        let vec3 = lowered(t_pre);
        let vec4 = lowered(t_now);

        let white = (255, 255, 255);
        let uvs = (0.0, 1.0, 0.0, 1.0);

        emit_tei_rect_18(
            state,
            out,
            slot.alpha_b,
            [b_pre, b_now, vec2, vec1],
            0,
            true,
            white,
            uvs,
            SlotBlend::Alpha,
        );
        emit_tei_rect_18(
            state,
            out,
            slot.alpha_b,
            [b_now, t_now, vec4, vec2],
            0,
            true,
            white,
            uvs,
            SlotBlend::Alpha,
        );
        emit_tei_rect_18(
            state,
            out,
            slot.alpha_b,
            [t_now, t_pre, vec3, vec4],
            0,
            true,
            white,
            uvs,
            SlotBlend::Alpha,
        );
        emit_tei_rect_18(
            state,
            out,
            slot.alpha_b,
            [t_pre, b_pre, vec1, vec3],
            0,
            true,
            white,
            uvs,
            SlotBlend::Alpha,
        );

        // `const short alpha = static_cast<short>(alphaB / 2);`
        let alpha = slot.alpha_b / 2;

        emit_tei_rect_18(
            state,
            out,
            alpha,
            [b_pre, b_now, t_now, t_pre],
            0,
            true,
            white,
            uvs,
            SlotBlend::AlphaOne,
        );
        emit_tei_rect_18(
            state,
            out,
            alpha,
            [t_pre, t_now, b_now, b_pre],
            0,
            true,
            white,
            uvs,
            SlotBlend::AlphaOne,
        );
    }
}

// ---------------------------------------------------------------------------
// RenderRotatedPlate (44)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderRotatedPlate` -
/// `EffectPrimitiveRendering.cpp:2747-2784`, dispatch `:542-543` (bare `case
/// 44`).
///
/// The scatter pulse: a quad whose inner and outer edges are pushed apart by
/// `flag1[1] * height[1]`, rotated around `fullDisplayAngle` and then tilted by
/// `riseAngle`. `flag1[0]` selects the colour and blend: 0/1 glow white-yellow,
/// 2 blue forced onto the near plane (`extra == 1`), anything else a dim
/// alpha-blended white.
pub fn render_rotated_plate(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 || slot.alpha_b <= 0 {
            continue;
        }

        let shift = slot.flag1[1] as f32 * slot.height[1];
        let outer = shift + slot.distance;
        let inner = slot.distance - shift;
        let back = slot.height[1] * -1.0;

        let display = slot.full_display_angle as i32;
        let rise = slot.rise_angle as i32;
        let cos_display = table_cos(display);
        let sin_display = table_sin(display);
        let cos_rise = table_cos(rise);
        let sin_rise = table_sin(rise);

        // The original's `- g_sinTable[riseAngle] * 0.0f` and
        // `+ g_cosTable[riseAngle] * 0.0f` terms are exactly zero and are
        // omitted from the x/z components below.
        let mut across = cos_display * outer - sin_display * slot.height[1];
        let vec2 = [
            across * cos_rise,
            sin_display * outer + cos_display * slot.height[1],
            across * sin_rise,
        ];

        across = cos_display * inner - sin_display * slot.height[1];
        let vec3 = [
            across * cos_rise,
            sin_display * inner + cos_display * slot.height[1],
            across * sin_rise,
        ];

        across = cos_display * inner - sin_display * back;
        let vec4 = [across * cos_rise, cos_display * back + sin_display * inner, across * sin_rise];

        across = cos_display * outer - sin_display * back;
        let vec1 = [across * cos_rise, cos_display * back + sin_display * outer, across * sin_rise];

        let quad = [vec1, vec2, vec3, vec4];

        match slot.flag1[0] {
            0 => emit_tei_rect_18(
                state,
                out,
                slot.alpha_b,
                quad,
                0,
                true,
                (225, 225, 80),
                (0.0, 1.0, 0.0, 1.0),
                SlotBlend::AlphaOne,
            ),
            1 => emit_tei_rect_18(
                state,
                out,
                slot.alpha_b,
                quad,
                0,
                true,
                (255, 255, 255),
                (0.0, 1.0, 0.0, 1.0),
                SlotBlend::AlphaOne,
            ),
            2 => emit_tei_rect_18_near(
                state,
                out,
                slot.alpha_b,
                quad,
                0,
                true,
                (65, 184, 240),
                (0.0, 1.0, 0.0, 1.0),
                SlotBlend::AlphaOne,
            ),
            _ => emit_tei_rect_18(
                state,
                out,
                slot.alpha_b,
                quad,
                0,
                true,
                (200, 200, 255),
                (0.0, 1.0, 0.0, 1.0),
                SlotBlend::Alpha,
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// RenderMagnum2 (73)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderMagnum2` - `EffectPrimitiveRendering.cpp:2121-2177`.
///
/// Two diagonal sweeps (`fullDisplayAngle / 20` degrees apart) are joined into
/// a curved sheet: each step emits the quad between the previous and current
/// bottom/top pairs through the 10-argument `RenderTeiRect`, whose texture
/// window is the step index. The sheet is drawn at `y = -3` with the top edge
/// lifted by `maxHeight` along `riseAngle`.
///
/// Quirks kept: the `fullDisplayAngle == 360 && order == 20` branch re-uses
/// `rotStart` for the last step, `order` saturates at 20 and the
/// `rpCnt >= 256` gate is evaluated per slot (the port counts the two triangles
/// of every emitted rect, so the value means the same). The renderer also
/// rewrites the slot's bottom/top vectors every frame; the port keeps them as
/// locals because rendering may not mutate the simulation - no other code
/// reads them.
pub fn render_magnum_2(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    let mut rect_triangles = 0usize;

    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if rect_triangles >= 256 || slot.life != 1 {
            continue;
        }

        let display = slot.full_display_angle as i32;
        let step = display / 20;

        // `count += step` with `step == 0` never terminates in the original
        // (an infinite loop); no body spawns such a slot.
        if step <= 0 {
            continue;
        }

        let mut order = 0i32;
        let mut vec_b_pre = [0.0f32; 3];
        let mut vec_t_pre = [0.0f32; 3];
        let mut count = 0i32;

        while count <= display {
            let mut angle = slot.rot_start as i32 + count;

            if angle >= 360 {
                angle -= 360;
            }

            if display == 360 && order == 20 {
                angle = slot.rot_start as i32;
            }

            let cs1 = table_cos(angle);
            let sn1 = table_sin(angle);
            let vec_b_now = [cs1 * slot.distance, -3.0, sn1 * slot.distance];

            let rx = table_cos(slot.rise_angle as i32) * slot.max_height;
            let ry = table_sin(slot.rise_angle as i32) * slot.max_height;
            let vec_t_now = [cs1 * rx + vec_b_now[0], -(ry + 3.0), sn1 * rx + vec_b_now[2]];

            if order > 0 && slot.alpha_b > 0 {
                emit_tei_rect_10(
                    state,
                    out,
                    order - 1,
                    slot.alpha_b,
                    [vec_b_pre, vec_b_now, vec_t_now, vec_t_pre],
                    0,
                );
                rect_triangles += 2;
            }

            order += 1;

            if order >= 21 {
                order = 20;
            }

            vec_b_pre = vec_b_now;
            vec_t_pre = vec_t_now;
            count += step;
        }
    }
}

// ---------------------------------------------------------------------------
// RenderSphereWind (91; shared with 77)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderSphereWind` - `EffectPrimitiveRendering.cpp:5591-5646`.
///
/// The `EF_BABY` fire-vine sphere: a sweep of `fullDisplayAngle / 20` degrees
/// builds a strip whose bottom edge sits on the `distance` ring and whose top
/// edge is displaced by `maxHeight` along `riseAngle`. Each step emits the
/// previous/current pair through the 10-argument `RenderTeiRect`.
///
/// Quirks kept: the `fullDisplayAngle == 360 && order == 20` branch re-uses
/// `rotStart`, and the step is an integer division (a `fullDisplayAngle` below
/// 20 would loop forever in the original; no spawner does that).
pub fn render_sphere_wind(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 {
            continue;
        }

        let display = slot.full_display_angle as i32;
        let step = display / 20;

        if step <= 0 {
            continue;
        }

        let mut order = 0i32;
        let mut vec1 = [0.0f32; 3];
        let mut vec3 = [0.0f32; 3];
        let mut count = 0i32;

        while count <= display {
            let mut angle = slot.rot_start as i32 + count;

            if angle >= 360 {
                angle -= 360;
            } else if angle < 0 {
                angle += 360;
            }

            if display == 360 && order == 20 {
                angle = slot.rot_start as i32;
            }

            let cs1 = table_cos(angle);
            let sn1 = table_sin(angle);
            let rise = slot.rise_angle as i32;
            let vec_m = [
                cs1 * (table_cos(rise) * slot.distance),
                cs1 * (table_sin(rise) * slot.distance),
                sn1 * slot.distance,
            ];
            let vec_a = [-(slot.max_height * table_sin(rise)), slot.max_height * table_cos(rise), 0.0];

            let vec2 = [vec_m[0] + vec_a[0], vec_m[1] + vec_a[1] + slot.height[0], vec_m[2] + vec_a[2]];
            let vec4 = [vec_m[0] - vec_a[0], vec_m[1] - vec_a[1] + slot.height[0], vec_m[2] - vec_a[2]];

            if order > 0 {
                emit_tei_rect_10(state, out, order - 1, slot.alpha_b, [vec1, vec2, vec4, vec3], 0);
            }

            order += 1;

            if order >= 21 {
                order = 20;
            }

            vec1 = vec2;
            vec3 = vec4;
            count += step;
        }
    }
}

// ---------------------------------------------------------------------------
// RenderFloorSquare (86, 92)
// ---------------------------------------------------------------------------

/// `CEffectPrim::RenderFloorSquare` - `EffectPrimitiveRendering.cpp:2637-2672`.
///
/// The four flat diamond shards of `PP_SOULBREAKER`'s `SonicWave` (prim 86) and
/// `PurpleSlash` (prim 92). Each live slot with a positive alpha emits one
/// quad of radius `distance` around `vecBNow`, turned by `rotStart + 45`
/// degrees. `flag1[1]` picks the texture and colour: `1` is the four-frame
/// sonic sheet, `2` is green, everything else uses the plain
/// `200,200,255` tint. The quad is submitted with `f1 = 1`, so the corners are
/// already absolute world positions.
pub fn render_floor_square(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 || slot.alpha_b <= 0 {
            continue;
        }

        let dist = slot.distance;
        let center = slot.vec_b_now;
        let y = center[1] + 0.0;
        let mut angle = slot.rot_start as i32 + 45;

        if angle >= 360 {
            angle -= 360;
        }

        let mut corners: Quad = [[0.0; 3]; 4];

        for corner in corners.iter_mut() {
            *corner = [dist * table_cos(angle) + center[0], y, dist * table_sin(angle) + center[2]];
            angle += if angle + 90 < 360 { 90 } else { -270 };
        }

        match slot.flag1[1] {
            1 => {
                // `const char frame = process > 0 ? process / 6 % 4 : 0;`
                let frame = if slot.process > 0 { (slot.process / 6 % 4) as i8 as i32 } else { 0 };

                emit_tei_rect_18(
                    state,
                    out,
                    slot.alpha_b,
                    corners,
                    frame,
                    false,
                    (205, 205, 255),
                    (0.0, 1.0, 0.0, 1.0),
                    SlotBlend::Alpha,
                );
            }
            2 => emit_tei_rect_18(
                state,
                out,
                slot.alpha_b,
                corners,
                0,
                false,
                (0, 255, 0),
                (0.0, 1.0, 0.0, 1.0),
                SlotBlend::Alpha,
            ),
            _ => emit_tei_rect_18(
                state,
                out,
                slot.alpha_b,
                corners,
                0,
                false,
                (200, 200, 255),
                (0.0, 1.0, 0.0, 1.0),
                SlotBlend::Alpha,
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// RenderSphereCage (61)
// ---------------------------------------------------------------------------

/// `g_heightSpeedSign` - `EffectPrimitiveRendering.cpp:46`.
const HEIGHT_SPEED_SIGN: f32 = -1.0;

/// `GuardPoint` - `EffectPrimitiveRendering.cpp:4310-4324`.
///
/// `(Rx, Ry)` is a corner of the cage cross-section (`Ry` is the vertical
/// radius, `Rx` the horizontal one); `sin2`/`cos2` rotate the point around the
/// cage axis and `sin1`/`cos1` around the vertical axis. Quirks kept: the
/// horizontal radius is truncated to `int` before it is rotated, and `Rz`
/// stays a float.
fn guard_point(rx: i32, ry: i32, sin2: f32, cos2: f32, sin1: f32, cos1: f32, vec_m: [f32; 3]) -> [f32; 3] {
    let y = ry as f32 * sin2;
    let z = ry as f32 * cos2;
    let x = rx as f32;
    let truncated = x as i32;
    let rz = z;
    let x = truncated as f32 * cos1 - rz * sin1;
    let z = truncated as f32 * sin1 + rz * cos1;

    [x + vec_m[0], y + vec_m[1], z + vec_m[2]]
}

/// One `GuardPoint` quad of `RenderSphereCage`.
///
/// The original computes `vecM` from the shell angles (`add2`, `rotStart+add`)
/// and then re-computes the four trig values for the two `GuardPoint`
/// rotations (`90+add2`, `rotStart+add-90`); both sets are carried here.
/// `radius` is the cross-section radius and `y_bias` the constant added to
/// `vecM.y`.
#[allow(clippy::too_many_arguments)]
fn sphere_cage_quad(
    radius: f32,
    y_bias: f32,
    (sin_axis, cos_axis): (f32, f32),
    (sin_band, cos_band): (f32, f32),
    (sin_guard2, cos_guard2): (f32, f32),
    (sin_guard1, cos_guard1): (f32, f32),
    distance: i32,
) -> Quad {
    let rz = radius * cos_axis;
    let vec_m = [rz * cos_band, radius * sin_axis + y_bias, rz * sin_band];

    [
        guard_point(distance, distance, sin_guard2, cos_guard2, sin_guard1, cos_guard1, vec_m),
        guard_point(-distance, distance, sin_guard2, cos_guard2, sin_guard1, cos_guard1, vec_m),
        guard_point(-distance, -distance, sin_guard2, cos_guard2, sin_guard1, cos_guard1, vec_m),
        guard_point(distance, -distance, sin_guard2, cos_guard2, sin_guard1, cos_guard1, vec_m),
    ]
}

/// `CEffectPrim::RenderSphereCage` - `EffectPrimitiveRendering.cpp:4328-4584`.
///
/// The `EF_GUARD2` black-maximize cage: three concentric shells (`maxHeight`,
/// `maxHeight + 0.2`, `maxHeight + 0.4 + height[0]`) built from three vertical
/// bands (`count2`) and five cross-sections (`count`). The shell radius shrinks
/// to `2`/`3`/`2` units, the colour comes from `flag1[0]` and the third shell
/// samples the body's second texture.
///
/// Render-time writes to the slot (`distance`, and the temp vectors the
/// decompiler reuses) are kept as locals: the official renderer is `&self`
/// here, and no motion body reads `distance` for prim 61.
pub fn render_sphere_cage(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 {
            continue;
        }

        let kind = slot.flag1[0];
        let max_height = slot.max_height;

        let colour = |default: (u8, u8, u8)| -> (u8, u8, u8) {
            match kind {
                3 => (255, 100, 0),
                2 => (255, 155, 0),
                4 => (255, 150, 255),
                5 => (100, 100, 255),
                _ => default,
            }
        };

        for count2 in 0..3i32 {
            for count in 0..5i32 {
                let add = count * 45 - 90;
                let add2 = count2 * 45 - 45;

                // `distance = (count2 - 1) * 1.0f;` negated through
                // `g_heightSpeedSign`, then mirrored around 1.5.
                let mut distance = (count2 as f32 - 1.0) * 1.0;

                if distance < 0.0 {
                    distance *= HEIGHT_SPEED_SIGN;
                }

                distance = 1.5 - distance + 1.5;
                let distance_int = distance as i32;

                let sin2 = table_sin(normalize_angle(add2));
                let cos2 = table_cos(normalize_angle(add2));
                let sin1 = table_sin(normalize_angle(slot.rot_start as i32 + add));
                let cos1 = table_cos(normalize_angle(slot.rot_start as i32 + add));
                let sin2_ring = table_sin(normalize_angle(90 + add2));
                let cos2_ring = table_cos(normalize_angle(90 + add2));
                let sin1_ring = table_sin(normalize_angle(slot.rot_start as i32 + add - 90));
                let cos1_ring = table_cos(normalize_angle(slot.rot_start as i32 + add - 90));

                let first = sphere_cage_quad(
                    max_height,
                    -(max_height + EFFECT_ICON_OUTER_RADIUS),
                    (sin2, cos2),
                    (sin1, cos1),
                    (sin2_ring, cos2_ring),
                    (sin1_ring, cos1_ring),
                    distance_int,
                );
                emit_tei_rect_18(
                    state,
                    out,
                    slot.alpha_b,
                    first,
                    0,
                    true,
                    colour((205, 255, 205)),
                    (0.0, 1.0, 0.0, 1.0),
                    SlotBlend::AlphaOne,
                );

                let second = sphere_cage_quad(
                    max_height + 0.2,
                    -max_height,
                    (sin2, cos2),
                    (sin1, cos1),
                    (sin2_ring, cos2_ring),
                    (sin1_ring, cos1_ring),
                    distance_int,
                );
                emit_tei_rect_18(
                    state,
                    out,
                    slot.alpha_b,
                    second,
                    0,
                    true,
                    colour((155, 255, 155)),
                    (0.0, 1.0, 0.0, 1.0),
                    SlotBlend::AlphaOne,
                );

                let third = sphere_cage_quad(
                    max_height + 0.4 + slot.height[0],
                    -max_height,
                    (sin2, cos2),
                    (sin1, cos1),
                    (sin2_ring, cos2_ring),
                    (sin1_ring, cos1_ring),
                    distance_int,
                );

                if kind == 5 {
                    emit_tei_rect_18(
                        state,
                        out,
                        slot.alpha_b,
                        third,
                        0,
                        true,
                        colour((225, 255, 225)),
                        (0.0, 1.0, 0.0, 1.0),
                        SlotBlend::AlphaOne,
                    );
                } else {
                    emit_tei_rect_18(
                        state,
                        out,
                        slot.alpha_b,
                        third,
                        1,
                        true,
                        colour((225, 255, 225)),
                        (0.0, 1.0, 0.0, 1.0),
                        SlotBlend::Alpha,
                    );
                }
            }
        }
    }
}

/// `CEffectPrim::RenderGroundBar` - `EffectPrimitiveRendering.cpp:3925-4030`.
///
/// The `PP_CHEMICAL3` (56) line bars: each live slot builds a ground rectangle
/// whose near edge lies on the segment `height[1]..height[1] + height[2]`
/// measured along `riseAngle`, offset sideways by `maxHeight` and backwards by
/// `height[0]`. The `flag1[0]` colour table is the original's.
pub fn render_ground_bar(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 || slot.alpha_b <= 0 {
            continue;
        }

        let angle = slot.rise_angle as i32;
        let width = slot.max_height;
        let back = angle + if angle >= 90 { -90 } else { 270 };
        let base_x = slot.height[0] * table_cos(back);
        let base_z = slot.height[0] * table_sin(back);
        let far_x = width * table_cos(back) + base_x;
        let far_z = width * table_sin(back) + base_z;

        let start = slot.height[1];
        let (start_x, start_z) = if 100.0 <= start {
            let reach = start - slot.distance;

            (reach * table_cos(angle), reach * table_sin(angle))
        } else {
            let opposite = if angle < 180 { angle + 180 } else { angle - 180 };
            let reach = slot.distance - start;

            (reach * table_cos(opposite), reach * table_sin(opposite))
        };

        let end = start + slot.height[2];
        let (end_x, end_z) = if 100.0 <= end {
            let reach = end - slot.distance;

            (reach * table_cos(angle), reach * table_sin(angle))
        } else {
            let opposite = angle + if angle >= 180 { -180 } else { 180 };
            let reach = slot.distance - end;

            (reach * table_cos(opposite), reach * table_sin(opposite))
        };

        let front = angle + if angle < 270 { 90 } else { -270 };
        let near_x = width * table_cos(front) + base_x;
        let near_z = width * table_sin(front) + base_z;
        let quad = [
            [near_x + end_x, -6.0, near_z + end_z],
            [near_x + start_x, -6.0, near_z + start_z],
            [start_x + far_x, -6.0, start_z + far_z],
            [end_x + far_x, -6.0, end_z + far_z],
        ];

        let colour = match slot.flag1[0] {
            0 => (225, 225, 255),
            1 => (205, 205, 255),
            2 => (185, 185, 255),
            10 => (255, 255, 225),
            11 => (255, 255, 205),
            12 => (255, 255, 185),
            20 => (225, 255, 225),
            21 => (205, 255, 205),
            22 => (185, 255, 185),
            30 => (105, 105, 255),
            31 => (65, 65, 255),
            32 => (25, 25, 255),
            33 => (103, 33, 177),
            34 => (66, 1, 104),
            35 => (178, 136, 236),
            36 => (241, 225, 178),
            37 => (202, 77, 65),
            38 => (148, 79, 73),
            125 => (255, 255, 255),
            126 => (215, 215, 215),
            127 => (185, 185, 185),
            _ => continue,
        };
        let forced_depth = match slot.flag1[0] {
            30..=38 => true,
            _ => false,
        };

        if forced_depth {
            emit_tei_rect_18_near(
                state,
                out,
                slot.alpha_b,
                quad,
                0,
                true,
                colour,
                (0.0, 1.0, 0.0, 1.0),
                SlotBlend::AlphaOne,
            );
        } else {
            emit_tei_rect_18(
                state,
                out,
                slot.alpha_b,
                quad,
                0,
                true,
                colour,
                (0.0, 1.0, 0.0, 1.0),
                SlotBlend::Alpha,
            );
        }
    }
}

/// `CEffectPrim::RenderFloorSquare2` -
/// `EffectPrimitiveRendering.cpp:2674-2709`.
///
/// The flat slash diamonds of prim 119 (`TexturedSlashFan`) and prim 122/125
/// (`TexturedSlashLevel`/`TexturedSlashWave`). The quad starts at
/// `riseAngle + 90` and turns a quarter per corner; `flag1[3]` picks the
/// colour pair and the blend (`0`/`1` glow, `2`/`3` straight alpha).
pub fn render_floor_square2(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 || slot.alpha_b <= 0 {
            continue;
        }

        let dist = slot.distance;
        let center = slot.vec_b_now;
        let y = center[1];
        let mut angle = slot.rise_angle as i32 + 90;

        if angle >= 360 {
            angle -= 360;
        }

        let mut corners: Quad = [[0.0; 3]; 4];

        for corner in corners.iter_mut() {
            *corner = [dist * table_cos(angle) + center[0], y, dist * table_sin(angle) + center[2]];
            angle += if angle + 90 < 360 { 90 } else { -270 };
        }

        match slot.flag1[3] {
            0 => emit_tei_rect_18(
                state,
                out,
                slot.alpha_b,
                corners,
                0,
                false,
                (80, 80, 255),
                (0.0, 1.0, 0.0, 1.0),
                SlotBlend::AlphaOne,
            ),
            1 => emit_tei_rect_18(
                state,
                out,
                slot.alpha_b,
                corners,
                0,
                false,
                (80, 255, 80),
                (0.0, 1.0, 0.0, 1.0),
                SlotBlend::AlphaOne,
            ),
            2 => emit_tei_rect_18(
                state,
                out,
                slot.alpha_b,
                corners,
                0,
                false,
                (255, 255, 255),
                (0.0, 1.0, 0.0, 1.0),
                SlotBlend::Alpha,
            ),
            3 => emit_tei_rect_18(
                state,
                out,
                slot.alpha_b,
                corners,
                0,
                false,
                (70, 70, 70),
                (0.0, 1.0, 0.0, 1.0),
                SlotBlend::Alpha,
            ),
            _ => {}
        }
    }
}

/// `CEffectPrim::RenderRotatedRing` - `EffectPrimitiveRendering.cpp:2608-2635`.
///
/// Prim 126 (`TexturedSlashCross`): a ring quad rotated around x/z by
/// `rotStart`, with the corner angles starting at `riseAngle` (the original
/// advances before using the angle).
pub fn render_rotated_ring(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 || slot.alpha_b <= 0 {
            continue;
        }

        let cs = table_cos(slot.rot_start as i32);
        let dist = slot.distance;
        let sn = table_sin(slot.rot_start as i32);
        let center = slot.vec_b_now;
        let mut angle = slot.rise_angle as i32;

        let mut corners: Quad = [[0.0; 3]; 4];

        for corner in corners.iter_mut() {
            angle += if angle + 90 < 360 { 90 } else { -270 };
            let radial = dist * table_cos(angle);
            *corner = [
                cs * radial + center[0],
                dist * table_sin(angle) + center[1],
                radial * sn + center[2],
            ];
        }

        emit_tei_rect_18(
            state,
            out,
            slot.alpha_b,
            corners,
            0,
            false,
            (80, 255, 80),
            (0.0, 1.0, 0.0, 1.0),
            SlotBlend::AlphaOne,
        );
    }
}

/// `CEffectPrim::RenderSoulLink` - `EffectPrimitiveRendering.cpp:5469-5557`.
///
/// The `EF_SOULLINK` orbit streak: the four corners of a square at
/// `distance` (45/135/225/315 degrees) are rotated toward the camera with
/// `slots[0/1].alphaT`, moved by `vecBPre` (`x` is the horizontal length,
/// `y` the vertical offset) and only drawn when the first corner is at or
/// below the ground. The original also launches `EF_SOULLIGHT` on the target
/// when the quad crosses the ground; the port has no scene spawn from a
/// renderer (documented in the module header) so only the quad is emitted.
pub fn render_soul_link(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    let rot0 = state.slots[0].alpha_t;
    let rot1 = state.slots[1].alpha_t;

    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 {
            continue;
        }

        let rx = slot.distance;
        let mut corners: Quad = [
            [table_cos(45) * rx, 0.0, table_sin(45) * rx],
            [table_cos(135) * rx, 0.0, table_sin(135) * rx],
            [table_cos(225) * rx, 0.0, table_sin(225) * rx],
            [table_cos(315) * rx, 0.0, table_sin(315) * rx],
        ];
        tei_toward_screen(&mut corners, rot0, rot1);

        let length = slot.vec_b_pre[0];
        let vec_a = [length * table_cos(rot0 as i32), slot.vec_b_pre[1], length * table_sin(rot0 as i32)];

        for corner in corners.iter_mut() {
            *corner = add(*corner, vec_a);
        }

        let alpha = slot.height[13] as i32 as i16;

        if corners[0][1] <= 0.0 {
            emit_tei_rect_18(
                state,
                out,
                alpha,
                corners,
                0,
                true,
                (125, 125, 255),
                (0.0, 1.0, 0.0, 1.0),
                SlotBlend::AlphaOne,
            );
        }
    }
}

/// `CEffectPrim::RenderSoulParticle` -
/// `EffectPrimitiveRendering.cpp:5558-5590`.
pub fn render_soul_particle(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    let rot0 = state.slots[0].alpha_t;
    let rot1 = state.slots[1].alpha_t;

    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 {
            continue;
        }

        let rx = slot.distance;
        let mut base_corners: Quad = [
            [table_cos(45) * rx, 0.0, table_sin(45) * rx],
            [table_cos(135) * rx, 0.0, table_sin(135) * rx],
            [table_cos(225) * rx, 0.0, table_sin(225) * rx],
            [table_cos(315) * rx, 0.0, table_sin(315) * rx],
        ];
        tei_toward_screen(&mut base_corners, rot0, rot1);

        for (base, count, extra_shade) in [
            (slot.vec_b_pre, slot.height[12], 40u8),
            (slot.vec_b_now, slot.height[13], 10u8),
            (slot.vec_t_pre, slot.height[14], 70u8),
        ] {
            let count = count.max(0.0) as i32;
            let alpha = count as i16;
            let corners = base_corners.map(|corner| add(corner, base));

            if corners[0][1] > 0.0 {
                continue;
            }

            let colour = match slot.height[15] as i32 {
                0 => (80, 80, 255),
                1 => (255, 200, 200),
                2 => (255, 160, 160),
                3 => (255, 120, 120),
                4 => (extra_shade, extra_shade, 255),
                _ => continue,
            };

            emit_tei_rect_18(
                state,
                out,
                alpha,
                corners,
                0,
                true,
                colour,
                (0.0, 1.0, 0.0, 1.0),
                SlotBlend::AlphaOne,
            );
        }
    }
}

/// `CEffectPrim::RenderSMA3` - `EffectPrimitiveRendering.cpp:2385-2420`,
/// dispatch `:557-558`.
///
/// Prim 120 (`PP_SMA`) has no geometry of its own: it launches the child
/// effects `EF_SMA2` / `EF_SMA3` / `EF_SMA_BLACK2` at slot 0's current
/// position, selected by `slots[0].flag1[0]` (which `CRagEffect::SMA` sets to
/// its `flag` argument - `0` for `EF_SMA`, `1` for `EF_STIN3`, `99` for
/// `EF_SMA_BLACK`):
///
/// * `0` - one `EF_SMA2` every eighth processed slot frame, with `deltaPos2 = {
///   process, process % 16 == 0 ? 0 : 1, 0 }`,
/// * `1` - one `EF_SMA3` on **every** render frame, no `deltaPos2`,
/// * `99` - one `EF_SMA_BLACK2` every eighth frame, same extra as `0`.
///
/// The official call is `LaunchEffect(...)` + `SendMsg(AM_SETPOS, ...)`; the
/// port records the request on the runtime outbox and the ported visual
/// spawns the child into the client's effect list. `&self` is enough because
/// the outbox is behind a mutex.
pub fn render_sma3(state: &EffectPrimitiveState, runtime: &EffectRuntime) {
    let slot = &state.slots[0];

    if slot.life != 1 {
        return;
    }

    let extra = [slot.process as f32, if slot.process % 16 == 0 { 0.0 } else { 1.0 }, 0.0];

    match slot.flag1[0] {
        0 => {
            if slot.process % 8 == 0 {
                runtime.launch_effect(EffectId::Sma2, slot.vec_b_now, Some(extra));
            }
        }
        1 => {
            runtime.launch_effect(EffectId::Sma3, slot.vec_b_now, None);
        }
        99 => {
            if slot.process % 8 == 0 {
                runtime.launch_effect(EffectId::SmaBlack2, slot.vec_b_now, Some(extra));
            }
        }
        _ => {}
    }
}

/// `CEffectPrim::RenderSMA2` - `EffectPrimitiveRendering.cpp:5227-5320`,
/// dispatch `:385-386`.
///
/// The stacked wave rings of `EF_SMA2` / `EF_SMA_BLACK2`: per slot, one quad
/// per ring segment between the angles `0..=fullDisplayAngle` (step
/// `fullDisplayAngle / 20`), each vertex lifted by `height[order]` along
/// `riseAngle` and all of them offset by the effect's `deltaPos`. The texture
/// window is `(order - 1) / 21 .. order / 21`; `order` is clamped to 20, so
/// the last segment repeats.
///
/// Kept quirk: `basicAngle` is an integer division; a `fullDisplayAngle`
/// below 20 would loop forever in the original, so a zero step is skipped here
/// (no body produces one - `TexturedBlueRing`/`BlueRingStack` set 315).
pub fn render_sma2(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 {
            continue;
        }

        let basic_angle = slot.full_display_angle as i32 / 20;

        if basic_angle <= 0 {
            continue;
        }

        let mut order = 0i32;
        let mut vec1 = [0.0f32; 3];
        let mut vec3 = [0.0f32; 3];
        let mut count = 0i32;

        while count <= slot.full_display_angle as i32 {
            let mut angle = count + slot.rot_start as i32;
            angle %= 360;

            if slot.full_display_angle == 360 && order == 20 {
                angle = slot.rot_start as i32;
            }

            let cs1 = table_cos(angle);
            let sn1 = table_sin(angle);

            let mut vec2 = [cs1 * slot.distance, 0.0f32, sn1 * slot.distance];

            let rx_raw = table_cos(slot.rise_angle as i32) * slot.height[order as usize];
            let ry = table_sin(slot.rise_angle as i32) * slot.height[order as usize];

            let rz = sn1 * rx_raw;
            let rx = cs1 * rx_raw;

            let mut vec4 = [vec2[0] + rx, -ry, vec2[2] + rz];

            vec2[0] += state.delta_pos[0];
            vec2[1] += state.delta_pos[1];
            vec2[2] += state.delta_pos[2];
            vec4[0] += state.delta_pos[0];
            vec4[1] += state.delta_pos[1];
            vec4[2] += state.delta_pos[2];

            if order > 0 {
                let tx1 = (order - 1) as f32 / 21.0;
                let tx2 = order as f32 / 21.0;

                // `slot.alphaT == 99` is the black stack (`BlueRingStack`);
                // every other value keeps the blue `TexturedBlueRing` colour.
                let (colour, blend) = if slot.alpha_t == 99 {
                    ((70u8, 70u8, 70u8), SlotBlend::Alpha)
                } else {
                    ((90u8, 90u8, 255u8), SlotBlend::AlphaOne)
                };

                emit_tei_rect_18(
                    state,
                    out,
                    slot.alpha_b,
                    [vec1, vec2, vec4, vec3],
                    0,
                    false,
                    colour,
                    (0.0, 1.0, tx1, tx2),
                    blend,
                );
            }

            order += 1;

            if order >= 21 {
                order = 20;
            }

            vec1 = vec2;
            vec3 = vec4;
            count += basic_angle;
        }
    }
}

/// `CEffectPrim::RenderGroundArcGlow` - `EffectPrimitiveArcs.cpp:60-166`,
/// dispatch `EffectPrimitiveRendering.cpp:373-375`.
///
/// The four ground arcs of `CRagEffect::BeginSpellSwirl`
/// (`EF_BEGINSPELL_N2..N8` / `EF_NEW_EPICLESIS_CAST`). Per slot the arc is
/// sampled on `0..=fullDisplayAngle` (step `fullDisplayAngle / 20`), the top
/// vertex is lifted by `height[i]` along `riseAngle`, and every segment is
/// submitted twice:
///
/// * once with `size = 0` (the base band, `process % totalTexture` texture),
/// * once with the `slots[ec].alphaT` kind's `size`/alpha multiplier (the glow
///   band, same texture).
///
/// `slots[ec].alphaT == 10` skips the base band and picks `process / 3 %
/// totalTexture` instead.
///
/// Kept quirks:
/// * the radius is `sin(rotStart) + distance`, not `distance`,
/// * the ring closes on `i == 20` with `fullDisplayAngle == 360` by snapping
///   the angle back to `rotStart`,
/// * `state.size` is *written* inside the renderer; the port passes the value
///   to [`emit_tei_rect_10_size`] instead of mutating the simulation,
/// * the `TT_MIN_EFFECT_ON_OFF` service option is off by default, so the
///   `alphaT` 9/11 branches double the alpha.
pub fn render_ground_arc_glow(state: &EffectPrimitiveState, out: &mut Vec<SlotTriangle>) {
    /// `g_serviceOptions.Get(TT_MIN_EFFECT_ON_OFF)` defaults to off
    /// (`rag/chemical_line.rs` documents the same choice).
    const MIN_EFFECT_ON_OFF: bool = false;

    for ec in 0..4usize {
        let slot = &state.slots[ec];

        if slot.life != 1 {
            continue;
        }

        let step = slot.full_display_angle as i32 / 20;

        // The original's `for (offset += step)` never terminates for a zero
        // step; no body reaches this (`BeginSpellSwirl` sets 360).
        if step <= 0 {
            continue;
        }

        let mut i = 0i32;
        let mut prev_b = [0.0f32; 3];
        let mut prev_t = [0.0f32; 3];
        let mut offset = 0i32;

        while offset <= slot.full_display_angle as i32 {
            // `angle -= angle / 360 * 360;` - integer division, so only angles
            // of 360 and above are folded.
            let mut angle = slot.rot_start as i32 + offset;
            angle -= angle / 360 * 360;

            if slot.full_display_angle == 360 && i == 20 {
                angle = slot.rot_start as i32;
            }

            let radius = table_sin(slot.rot_start as i32) + slot.distance;
            let cosine = table_cos(angle);
            let sine = table_sin(angle);

            let rise_x = slot.height[i as usize] * table_cos(slot.rise_angle as i32);
            let rise_y = slot.height[i as usize] * table_sin(slot.rise_angle as i32);

            let cur_b = [radius * cosine, 0.0f32, radius * sine];
            let cur_t = [rise_x * cosine + cur_b[0], -rise_y, rise_x * sine + cur_b[2]];

            if i > 0 {
                // `if (alphaT == 10) texture = process / 3 % totalTexture;`
                // skips the base band; every other kind submits it with
                // `process % totalTexture` and `state.size = 0`.
                let texture = if slot.alpha_t == 10 {
                    slot.process / 3 % state.total_texture as i32
                } else {
                    let texture = slot.process % state.total_texture as i32;

                    emit_tei_rect_10_size(state, out, i - 1, slot.alpha_b, [prev_b, cur_b, cur_t, prev_t], texture, 0.0);

                    texture
                };

                // `short alpha;` is only assigned by the branches below; every
                // ported `alphaT` (0, 3..9) is covered, and the fallback keeps
                // the slot alpha instead of an uninitialised value.
                let mut alpha = slot.alpha_b;
                let mut size = state.size;

                if slot.alpha_t <= 2 {
                    size = 17.0;
                    alpha = slot.alpha_b * 3;
                } else if slot.alpha_t == 3 {
                    size = 18.0;
                    alpha = slot.alpha_b * 3;
                } else if slot.alpha_t == 4 {
                    size = 19.0;
                    alpha = slot.alpha_b * 2;
                } else if slot.alpha_t == 5 {
                    size = 20.0;
                    alpha = slot.alpha_b * 2;
                } else if slot.alpha_t == 7 {
                    size = 21.0;
                    alpha = slot.alpha_b * 2;
                } else if slot.alpha_t == 8 {
                    size = 22.0;
                    alpha = slot.alpha_b * 2;
                } else if slot.alpha_t == 9 {
                    size = 30.0;
                    alpha = if MIN_EFFECT_ON_OFF { slot.alpha_b } else { slot.alpha_b * 2 };
                } else if slot.alpha_t == 6 {
                    size = 5.0;
                    alpha = slot.alpha_b * 2;
                } else if slot.alpha_t == 10 {
                    size = 6.0;
                    alpha = slot.alpha_b * 3;
                } else if slot.alpha_t == 11 {
                    size = 32.0;
                    alpha = if MIN_EFFECT_ON_OFF { slot.alpha_b } else { slot.alpha_b * 2 };
                }

                if alpha > 255 {
                    alpha = 255;
                }

                emit_tei_rect_10_size(state, out, i - 1, alpha, [prev_b, cur_b, cur_t, prev_t], texture, size);
            }

            i += 1;

            if i >= 21 {
                i = 20;
            }

            prev_t = cur_t;
            prev_b = cur_b;
            offset += step;
        }
    }
}

/// The client's one-step angle normalisation (`if (angle >= 360)
/// angle -= 360; if (angle < 0) angle += 360;`).
fn normalize_angle(mut angle: i32) -> i32 {
    if angle >= 360 {
        angle -= 360;
    }

    if angle < 0 {
        angle += 360;
    }

    angle
}

#[cfg(test)]
mod tests {
    use cgmath::{Deg, Matrix4, Vector3};

    use super::super::runtime::PendingEffectLaunch;
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveRecord};

    const WINDOW: ScreenSize = ScreenSize {
        width: 640.0,
        height: 480.0,
    };

    fn render(state: &EffectPrimitiveState) -> Option<SlotRenderer> {
        super::renderer(state)
    }

    fn state_of(id: PrimitiveId) -> EffectPrimitiveState {
        EffectPrimitiveState {
            count: id,
            position: [0.0, 0.0, 0.0],
            ..Default::default()
        }
    }

    /// `(cos, sin)` of a table angle, so the assertions spell out the same
    /// arithmetic the renderers use.
    fn trig(degrees: i32) -> (f32, f32) {
        (table_cos(degrees), table_sin(degrees))
    }

    #[test]
    fn size_color_matches_the_official_table() {
        // Default: white glow (`blend = 2`).
        assert_eq!(size_color(9.0), (140, 210, 140, 1.0, true));
        assert_eq!(size_color(16.0), (20, 255, 20, 1.0, true));
        // Cases that set `blend = 6` (straight alpha).
        assert_eq!(size_color(0.0), (255, 255, 255, 1.0, false));
        assert_eq!(size_color(2.0), (195, 195, 255, 1.0, false));
        assert_eq!(size_color(13.0), (255, 50, 255, 1.0, false));
        assert_eq!(size_color(32.0), (0, 0, 0, 1.0, false));
        // The one case with a texture window.
        assert_eq!(size_color(100.0), (250, 250, 255, 0.9, false));
        // Unlisted sizes keep the white glow default.
        assert_eq!(size_color(5.0), (255, 255, 255, 1.0, true));
    }

    #[test]
    fn blend_resolution_follows_the_render_flags() {
        // `RSF_ADDITIVE` wins.
        assert_eq!(resolve_blend(3, SlotBlend::Alpha), SlotBlend::Additive);
        assert_eq!(resolve_blend(0x203, SlotBlend::AlphaOne), SlotBlend::Additive);
        // `RSF_CUSTOM_BLEND` uses the triangle blend.
        assert_eq!(resolve_blend(5, SlotBlend::AlphaOne), SlotBlend::AlphaOne);
        assert_eq!(resolve_blend(5, SlotBlend::Alpha), SlotBlend::Alpha);
        // Plain `RSF_ALPHA` (and `RSF_NO_DEPTH`) force the alpha queue.
        assert_eq!(resolve_blend(1, SlotBlend::AlphaOne), SlotBlend::Alpha);
        assert_eq!(resolve_blend(0x201, SlotBlend::AlphaOne), SlotBlend::Alpha);
    }

    #[test]
    fn packed_argb_has_alpha_red_green_blue_in_that_order() {
        assert_eq!(pack_argb(180, 140, 210, 140), 0xB4_8C_D2_8C);
        assert_eq!(pack_argb(0, 255, 255, 255), 0x00FF_FFFF);
        // Negative alphas (which the motion clamps away) saturate.
        assert_eq!(pack_argb(-5, 0, 0, 0), 0x0000_0000);
    }

    #[test]
    fn every_ported_slot_id_dispatches_to_its_official_renderer() {
        let expected = [
            (PrimitiveId::PulsingColumns, SlotRenderer::Casting),
            (PrimitiveId::RippleRings, SlotRenderer::Casting),
            (PrimitiveId::Circle3d, SlotRenderer::RecordRings),
            (PrimitiveId::Spline3d, SlotRenderer::RecordQuads),
            (PrimitiveId::Spiral3d, SlotRenderer::RecordQuads),
            (PrimitiveId::Cylinder3d, SlotRenderer::ArcBand),
            (PrimitiveId::ParticleUp, SlotRenderer::FieldPanel),
            (PrimitiveId::Bottom, SlotRenderer::FieldPanel),
            (PrimitiveId::BottomOut, SlotRenderer::FieldPanel),
            (PrimitiveId::ShieldBoomerang, SlotRenderer::FieldPanel),
            (PrimitiveId::ThrowItem, SlotRenderer::FieldPanel),
            (PrimitiveId::ShieldBoomerang3, SlotRenderer::FieldPanel),
            (PrimitiveId::QuadHorn3d, SlotRenderer::Pyramid),
            (PrimitiveId::HeightRamp, SlotRenderer::ArcWalls),
            (PrimitiveId::RisingFan, SlotRenderer::FacingQuads),
            (PrimitiveId::RisingFan2, SlotRenderer::Ripple),
            (PrimitiveId::Halo, SlotRenderer::SizedSquare),
            (PrimitiveId::Wind, SlotRenderer::RiseArc),
            (PrimitiveId::TextureSpin, SlotRenderer::GroundSquare),
            (PrimitiveId::WanderingFlames, SlotRenderer::TwinGlowSquares),
            (PrimitiveId::Intimidate, SlotRenderer::OrbitSquare),
            (PrimitiveId::PulseKind, SlotRenderer::RectUp),
            (PrimitiveId::ForestLight, SlotRenderer::Pentagon),
            (PrimitiveId::SpinQuad, SlotRenderer::FanBeam),
            (PrimitiveId::AuraGlow, SlotRenderer::AuraGlow),
            (PrimitiveId::HalfSphere, SlotRenderer::HalfSphere),
            (PrimitiveId::FlashFade, SlotRenderer::BladeFins),
            (PrimitiveId::StackedRings, SlotRenderer::StackedRings),
            (PrimitiveId::PeongUp, SlotRenderer::PeongUp),
            (PrimitiveId::Casting2, SlotRenderer::Casting),
            (PrimitiveId::Bottom2, SlotRenderer::ColorPlane),
            (PrimitiveId::Slash1, SlotRenderer::Slash1),
            (PrimitiveId::PentaRing, SlotRenderer::PentaRing),
            (PrimitiveId::HitLine3, SlotRenderer::HitLine3),
            (PrimitiveId::Gi3, SlotRenderer::CastingGi3),
            (PrimitiveId::GrandCross2, SlotRenderer::GrandCross2),
            (PrimitiveId::ExpandPulse, SlotRenderer::RotatedPlate),
            (PrimitiveId::TeiHit2, SlotRenderer::FieldPanel),
            (PrimitiveId::TexturedGlow, SlotRenderer::Casting),
            (PrimitiveId::LineLink, SlotRenderer::Tube),
            (PrimitiveId::ChemicalProtection, SlotRenderer::Chemical),
            (PrimitiveId::FirstAid, SlotRenderer::FieldPanel),
            (PrimitiveId::BubbleSwirl, SlotRenderer::FieldPanel),
            (PrimitiveId::GrowFollow, SlotRenderer::SizedSquare),
            (PrimitiveId::Guard, SlotRenderer::SphereCage),
            (PrimitiveId::Magnum2, SlotRenderer::Magnum2),
            (PrimitiveId::SonicWave, SlotRenderer::FloorSquare),
            (PrimitiveId::SphereWind2, SlotRenderer::SphereWind),
            (PrimitiveId::PurpleSlash, SlotRenderer::FloorSquare),
        ];

        for (id, expected_renderer) in expected {
            assert_eq!(render(&state_of(id)), Some(expected_renderer), "id {:?}", id);
        }

        // Not slot primitives.
        assert_eq!(render(&state_of(PrimitiveId::Texture2d)), None);
        assert_eq!(render(&state_of(PrimitiveId::Texture3d)), None);
        assert_eq!(render(&state_of(PrimitiveId::Particle3d)), None);
        // `PP_3DPARTICLESPLINE` shares `RenderSegmentSprites2` (sprite pass).
        assert_eq!(render(&state_of(PrimitiveId::ParticleSpline)), None);
        // So does prim 46 (`CHIMTO`/`FVOICE`/`WINK`).
        assert_eq!(render(&state_of(PrimitiveId::ChimtoSprite)), None);
    }

    #[test]
    fn no_live_slot_and_no_record_draws_nothing() {
        // `RenderPyramid` and `RenderArcBand` are the exceptions: they build
        // their geometry from the primitive's own `size`/`heightSize`/radii and
        // have no slot gate (a zero-size band is still submitted, just
        // degenerate). Every slot-driven renderer draws nothing without a live
        // slot, and the record renderer draws nothing without a segment.
        for id in [
            PrimitiveId::PulsingColumns,
            PrimitiveId::RippleRings,
            PrimitiveId::Circle3d,
            PrimitiveId::Spline3d,
            PrimitiveId::Spiral3d,
            PrimitiveId::ParticleUp,
            PrimitiveId::ShieldBoomerang,
            PrimitiveId::ThrowItem,
            PrimitiveId::ShieldBoomerang3,
            PrimitiveId::HeightRamp,
            PrimitiveId::RisingFan,
            PrimitiveId::RisingFan2,
            PrimitiveId::Halo,
            PrimitiveId::Wind,
            PrimitiveId::TextureSpin,
            PrimitiveId::WanderingFlames,
            PrimitiveId::Intimidate,
            PrimitiveId::PulseKind,
            PrimitiveId::ForestLight,
            PrimitiveId::SpinQuad,
            PrimitiveId::AuraGlow,
            PrimitiveId::HalfSphere,
            PrimitiveId::FlashFade,
            PrimitiveId::StackedRings,
            PrimitiveId::PeongUp,
            PrimitiveId::Casting2,
            PrimitiveId::Bottom,
            PrimitiveId::Bottom2,
            PrimitiveId::BottomOut,
            PrimitiveId::Slash1,
            PrimitiveId::PentaRing,
            PrimitiveId::HitLine3,
            PrimitiveId::Gi3,
            PrimitiveId::GrandCross2,
            PrimitiveId::ExpandPulse,
            PrimitiveId::TeiHit2,
            PrimitiveId::TexturedGlow,
            PrimitiveId::LineLink,
            PrimitiveId::ChemicalProtection,
            PrimitiveId::FirstAid,
        ] {
            let mut state = state_of(id);
            // No segment records: `numSegment` is what the record renderer
            // iterates, and every body that uses it calls `InitSegment`.
            state.records.clear();
            state.num_segment = 0;

            assert!(triangles(&state).is_empty(), "id {:?} drew with no live slot", id);
        }
    }

    #[test]
    fn pyramid_draws_from_the_primitive_without_a_live_slot() {
        // `RenderPyramid` never inspects the slots; it is the stone slab's
        // own `size`/`heightSize` geometry (a zero-size slab still submits).
        let mut state = state_of(PrimitiveId::QuadHorn3d);
        state.size = 2.0;
        state.height_size = 5.0;

        assert_eq!(triangles(&state).len(), 4);
    }

    #[test]
    fn casting_ring_emits_one_quad_per_sweep_segment() {
        let mut state = state_of(PrimitiveId::PulsingColumns);
        state.size = 9.0;
        state.render_flag = 5;
        state.texture = Some("effect\\heal.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            full_display_angle: 360,
            rot_start: 0,
            distance: 10.0,
            rise_angle: 90,
            max_height: 40.0,
            alpha_b: 100,
            height: [20.0; 21],
            ..Default::default()
        };

        let triangles = triangles(&state);

        // `fullDisplayAngle / 20 = 18`, `0..=360` is 21 sweep positions and the
        // first one only seeds `vec1`/`vec3`, so 20 quads = 40 triangles.
        assert_eq!(triangles.len(), 40);
    }

    #[test]
    fn casting_ring_first_quad_has_exact_corners_and_uvs() {
        let mut state = state_of(PrimitiveId::PulsingColumns);
        state.size = 9.0;
        state.render_flag = 5;
        state.texture = Some("effect\\heal.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            full_display_angle: 360,
            rot_start: 0,
            distance: 10.0,
            rise_angle: 90,
            max_height: 40.0,
            alpha_b: 100,
            height: [20.0; 21],
            ..Default::default()
        };

        let triangles = triangles(&state);

        let (cos0, sin0) = trig(0);
        let (cos18, sin18) = trig(18);
        let (cos90, sin90) = trig(90);

        // order 0 seeds the strip: angle 0, `height[0] = 20`.
        let rx = cos90 * 20.0;
        let ry = sin90 * 20.0;
        let previous_bottom = [cos0 * 10.0, 0.0, sin0 * 10.0];
        let previous_top = [cos0 * 10.0 + cos0 * rx, -ry, sin0 * 10.0 + sin0 * rx];

        // order 1 (angle 18, `height[1] = 20`).
        let bottom = [cos18 * 10.0, 0.0, sin18 * 10.0];
        let top = [cos18 * 10.0 + cos18 * rx, -ry, sin18 * 10.0 + sin18 * rx];

        assert_eq!(triangles[0].corners, [previous_bottom, bottom, top]);
        // `texPart = order - 1 = 0`: left 0, right 1/21, v = textureV = 1.
        assert_eq!(triangles[0].texture_coordinates, [[0.0, 1.0], [1.0 / 21.0, 1.0], [
            1.0 / 21.0,
            0.0
        ],]);

        assert_eq!(triangles[1].corners, [previous_top, top, previous_bottom]);
        assert_eq!(triangles[1].texture_coordinates, [[0.0, 0.0], [1.0 / 21.0, 0.0], [0.0, 1.0],]);

        // size 9 -> (140, 210, 140), alpha 100, custom blend so the size table
        // decides (glow = true for size 9).
        assert_eq!(triangles[0].argb, pack_argb(100, 140, 210, 140));
        assert_eq!(triangles[0].blend, SlotBlend::AlphaOne);
        assert_eq!(triangles[0].texture, Some("effect\\heal.tga"));
    }

    #[test]
    fn casting_ring_texture_window_advances_with_the_height_index() {
        let mut state = state_of(PrimitiveId::PulsingColumns);
        state.size = 9.0;
        state.render_flag = 5;
        state.texture = Some("effect\\heal.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            full_display_angle: 360,
            rot_start: 0,
            distance: 10.0,
            rise_angle: 90,
            max_height: 40.0,
            alpha_b: 100,
            height: [20.0; 21],
            ..Default::default()
        };

        let triangles = triangles(&state);

        // The last quad is `texPart = 19`: u from 19/21 to 20/21.
        let last = triangles[triangles.len() - 2];
        assert_eq!(last.texture_coordinates[0], [19.0 / 21.0, 1.0]);
        assert_eq!(last.texture_coordinates[1], [20.0 / 21.0, 1.0]);
        assert_eq!(last.texture_coordinates[2], [20.0 / 21.0, 0.0]);
    }

    #[test]
    fn casting_ring_recognises_the_ring_wave_offset_marker() {
        // maxHeight 6.001 lowers the ring by 2 units (the ring-wave bodies).
        let mut state = state_of(PrimitiveId::RippleRings);
        state.size = 16.0;
        state.render_flag = 5;
        state.texture = Some("effect\\ring_white.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            full_display_angle: 360,
            rot_start: 0,
            distance: 0.0,
            rise_angle: 2,
            max_height: 6.001,
            alpha_b: 240,
            height: [0.0; 21],
            ..Default::default()
        };

        let triangles = triangles(&state);

        assert_eq!(triangles.len(), 40);
        // Every corner keeps the -2 base offset; the heights are zero.
        for triangle in &triangles {
            for corner in &triangle.corners {
                assert_eq!(corner[1], -2.0);
            }
        }
    }

    #[test]
    fn casting2_dispatches_to_the_shared_casting_renderer() {
        let mut state = state_of(PrimitiveId::Casting2);
        state.size = 0.0;
        state.render_flag = 5;
        state.texture = Some("effect\\ring_black.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            full_display_angle: 360,
            rot_start: 180,
            distance: 8.0,
            rise_angle: 90,
            max_height: 40.0,
            alpha_b: 80,
            height: [0.0; 21],
            ..Default::default()
        };

        let triangles = triangles(&state);

        // Same sweep as the other casting prims: 20 quads = 40 triangles.
        assert_eq!(triangles.len(), 40);
        assert_eq!(triangles[0].texture, Some("effect\\ring_black.tga"));
        // size 0 -> white, `blend = 6` (straight alpha) from the size table.
        assert_eq!(triangles[0].argb, pack_argb(80, 255, 255, 255));
        assert_eq!(triangles[0].blend, SlotBlend::Alpha);
    }

    #[test]
    fn color_plane_emits_one_plane_per_live_slot_with_exact_corners() {
        let mut state = state_of(PrimitiveId::Bottom2);
        state.render_flag = 5;
        state.texture = Some("effect\\ring_blue.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 100,
            height: [1.0; 21],
            max_height: 25.0,
            vec_b_now: [-1.0, 0.0, 2.0],
            vec_b_pre: [1.0, 0.0, -2.0],
            ..Default::default()
        };

        let triangles = triangles(&state);

        assert_eq!(triangles.len(), 2);
        assert_eq!(triangles[0].corners, [[-1.0, 0.0, 2.0], [1.0, 0.0, -2.0], [1.0, -25.0, -2.0]]);
        assert_eq!(triangles[1].corners, [[-1.0, -25.0, 2.0], [1.0, -25.0, -2.0], [-1.0, 0.0, 2.0]]);
        // `ty1 = 0`, `ty2 = 1`, `tx = 0`, `tx2 = 1`: the whole texture.
        assert_eq!(triangles[0].texture_coordinates, [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]);
        assert_eq!(triangles[1].texture_coordinates, [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
        // `height[0] == 1` -> (255, 155, 155) with `pw = 1` (straight alpha).
        assert_eq!(triangles[0].argb, pack_argb(100, 255, 155, 155));
        assert_eq!(triangles[0].blend, SlotBlend::Alpha);
        assert_eq!(triangles[0].texture, Some("effect\\ring_blue.tga"));
    }

    #[test]
    fn color_plane_kind_table_matches_the_decompilation() {
        let expected = [
            // kind, rgb, blend
            (1.0f32, (255, 155, 155), SlotBlend::Alpha),
            (2.0, (155, 255, 155), SlotBlend::Alpha),
            (3.0, (255, 255, 100), SlotBlend::AlphaOne),
            (4.0, (255, 150, 150), SlotBlend::AlphaOne),
            // Any other kind falls through to plain white with `pw = 0`.
            (0.0, (255, 255, 255), SlotBlend::AlphaOne),
            (7.0, (255, 255, 255), SlotBlend::AlphaOne),
        ];

        for (kind, (red, green, blue), blend) in expected {
            let mut state = state_of(PrimitiveId::Bottom2);
            state.render_flag = 5;
            state.texture = Some("effect\\safeline.bmp");
            state.slots[0] = EffectPrimitiveSlot {
                life: 1,
                alpha_b: 60,
                height: [kind; 21],
                max_height: 10.0,
                ..Default::default()
            };

            let triangles = triangles(&state);

            assert_eq!(triangles.len(), 2, "kind {kind}");
            assert_eq!(triangles[0].argb, pack_argb(60, red, green, blue), "kind {kind}");
            assert_eq!(triangles[0].blend, blend, "kind {kind}");
        }
    }

    #[test]
    fn color_plane_skips_dead_or_transparent_slots() {
        let mut state = state_of(PrimitiveId::Bottom2);
        state.render_flag = 5;
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 0,
            height: [1.0; 21],
            ..Default::default()
        };
        state.slots[1] = EffectPrimitiveSlot {
            life: 0,
            alpha_b: 200,
            height: [1.0; 21],
            ..Default::default()
        };

        assert!(triangles(&state).is_empty());
    }

    #[test]
    fn slash1_emits_three_blades_per_live_slot() {
        let mut state = state_of(PrimitiveId::Slash1);
        state.render_flag = 5;
        state.texture = Some("effect\\magic_blue.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 90,
            height: [5.0; 21],
            distance: 2.0,
            max_height: 4.0,
            rot_start: 0,
            ..Default::default()
        };

        let triangles = triangles(&state);

        // `i = 0, 1, 2` -> three quads -> six triangles.
        assert_eq!(triangles.len(), 6);

        let (cos359, sin359) = trig(359);
        let (cos0, sin0) = trig(0);
        let (cos1, sin1) = trig(1);

        // i = 0: angle 359, inner radius 5, outer 7.
        let first = [cos359 * 5.0, -0.0, sin359 * 5.0];
        let second = [first[0], -0.4, first[2]];
        let third = [cos359 * 7.0, -0.0, sin359 * 7.0];
        let fourth = [third[0], -1.6, third[2]];
        assert_eq!(triangles[0].corners, [first, third, fourth]);
        assert_eq!(triangles[1].corners, [second, fourth, first]);
        // `alphaB / 3` on the outer blades.
        assert_eq!(triangles[0].argb, pack_argb(30, 255, 255, 255));
        assert_eq!(triangles[0].blend, SlotBlend::AlphaOne);
        assert_eq!(triangles[0].texture, Some("effect\\magic_blue.tga"));
        assert_eq!(triangles[0].texture_coordinates, [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]);
        assert_eq!(triangles[1].texture_coordinates, [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);

        // i = 1: angle 0 and the full-height middle blade.
        let first = [cos0 * 5.0, -0.0, sin0 * 5.0];
        let second = [first[0], -0.8, first[2]];
        let third = [cos0 * 7.0, -0.0, sin0 * 7.0];
        let fourth = [third[0], -4.0, third[2]];
        assert_eq!(triangles[2].corners, [first, third, fourth]);
        assert_eq!(triangles[3].corners, [second, fourth, first]);
        assert_eq!(triangles[2].argb, pack_argb(90, 255, 255, 255));

        // i = 2: angle 1.
        let first = [cos1 * 5.0, -0.0, sin1 * 5.0];
        let second = [first[0], -0.4, first[2]];
        let third = [cos1 * 7.0, -0.0, sin1 * 7.0];
        let fourth = [third[0], -1.6, third[2]];
        assert_eq!(triangles[4].corners, [first, third, fourth]);
        assert_eq!(triangles[5].corners, [second, fourth, first]);
        assert_eq!(triangles[4].argb, pack_argb(30, 255, 255, 255));
    }

    #[test]
    fn slash1_wraps_the_blade_angle_and_skips_dead_slots() {
        // rotStart 0 makes i = 0 use angle -1, which wraps to 359.
        let mut state = state_of(PrimitiveId::Slash1);
        state.render_flag = 5;
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 3,
            height: [1.0; 21],
            distance: 1.0,
            max_height: 1.0,
            rot_start: 0,
            ..Default::default()
        };

        let blades = triangles(&state);
        let (cos359, sin359) = trig(359);
        assert_eq!(blades[0].corners[0], [cos359 * 1.0, -0.0, sin359 * 1.0]);
        // `alphaB / 3` truncates 3 / 3 = 1.
        assert_eq!(blades[0].argb, pack_argb(1, 255, 255, 255));

        // A slot whose `alphaB` is zero draws nothing, even when alive.
        let mut transparent = state_of(PrimitiveId::Slash1);
        transparent.render_flag = 5;
        transparent.slots[0].life = 1;
        assert!(triangles(&transparent).is_empty());
    }

    #[test]
    fn record_rings_sweep_in_arc_angle_steps() {
        let mut state = state_of(PrimitiveId::Circle3d);
        state.arc_angle = 90.0;
        state.size = 2.0;
        state.render_flag = 1;
        state.texture = Some("effect\\ring.tga");
        state.num_segment = 1;
        state.records = vec![PrimitiveRecord {
            position: [1.0, 2.0, 3.0],
            radius: 10.0,
            color: 0x1122_3344,
            matrix: crate::world::effect::port::math::Matrix::default(),
            ..Default::default()
        }];

        let triangles = triangles(&state);

        // 360 - 90 = 270, so i = 0, 90, 180, 270 -> four steps, two triangles.
        assert_eq!(triangles.len(), 8);

        let (cos0, sin0) = trig(0);
        let (cos90, sin90) = trig(90);

        let vec1 = [1.0 + 8.0 * cos0, 2.0 + 8.0 * sin0, 3.0];
        let vec2 = [1.0 + 8.0 * cos90, 2.0 + 8.0 * sin90, 3.0];
        let vec3 = [1.0 + 10.0 * cos0, 2.0 + 10.0 * sin0, 3.0];
        let vec4 = [1.0 + 10.0 * cos90, 2.0 + 10.0 * sin90, 3.0];

        assert_eq!(triangles[0].corners, [vec1, vec2, vec3]);
        assert_eq!(triangles[0].texture_coordinates, [[0.0, 1.0], [0.25, 1.0], [0.0, 0.0],]);
        assert_eq!(triangles[1].corners, [vec4, vec3, vec2]);
        assert_eq!(triangles[1].texture_coordinates, [[0.25, 0.0], [0.0, 0.0], [0.25, 1.0],]);
        assert_eq!(triangles[0].argb, 0x1122_3344);
        assert_eq!(triangles[0].texture, Some("effect\\ring.tga"));
        assert_eq!(triangles[0].blend, SlotBlend::Alpha);
    }

    #[test]
    fn record_quads_emit_the_official_triangle_order_uvs_and_colour() {
        let mut state = state_of(PrimitiveId::Spiral3d);
        state.texture = Some("effect\\spline.tga");
        state.num_segment = 2;
        // `SubmitTriangle(..., 1)` hard-codes `RSF_ALPHA`; a body that set
        // additive flags could not change this renderer's queue.
        state.render_flag = 3;
        state.records = vec![
            PrimitiveRecord {
                corners: [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
                color: 0x1122_3344,
                ..Default::default()
            },
            PrimitiveRecord {
                corners: [[2.0, 0.0, 1.0], [3.0, 0.0, 1.0], [3.0, 1.0, 1.0], [2.0, 1.0, 1.0]],
                color: 0x5566_7788,
                ..Default::default()
            },
        ];

        let triangles = triangles(&state);

        // Two records -> two triangles each, in record order.
        assert_eq!(triangles.len(), 4);

        // Triangle A: corners 0,1,2 and `uv` (0,1) (1,1) (0,0).
        assert_eq!(triangles[0].corners, [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]);
        assert_eq!(triangles[0].texture_coordinates, [[0.0, 1.0], [1.0, 1.0], [0.0, 0.0]]);
        assert_eq!(triangles[0].argb, 0x1122_3344);
        assert_eq!(triangles[0].texture, Some("effect\\spline.tga"));
        assert_eq!(triangles[0].blend, SlotBlend::Alpha);

        // Triangle B: corners 3,2,1 and `uv` (1,0) (0,0) (1,1).
        assert_eq!(triangles[1].corners, [[0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 0.0, 0.0]]);
        assert_eq!(triangles[1].texture_coordinates, [[1.0, 0.0], [0.0, 0.0], [1.0, 1.0]]);
        assert_eq!(triangles[1].argb, 0x1122_3344);

        // The second record repeats the pattern with its own colour.
        assert_eq!(triangles[2].corners, [[2.0, 0.0, 1.0], [3.0, 0.0, 1.0], [3.0, 1.0, 1.0]]);
        assert_eq!(triangles[2].argb, 0x5566_7788);
        assert_eq!(triangles[3].corners, [[2.0, 1.0, 1.0], [3.0, 1.0, 1.0], [3.0, 0.0, 1.0]]);
        assert_eq!(triangles[3].argb, 0x5566_7788);
    }

    #[test]
    fn arc_band_builds_the_flared_cylinder() {
        let mut state = state_of(PrimitiveId::Cylinder3d);
        state.arc_angle = 180.0;
        state.inner_size = 5.0;
        state.outer_size = 10.0;
        state.height_size = 4.0;
        state.argb = 0xAA_BB_CC_DD;
        state.render_flag = 1;
        state.texture = Some("effect\\lens2.tga");

        let triangles = triangles(&state);

        // 360 - 180 = 180, so i = 0, 180 -> two steps, two triangles each.
        assert_eq!(triangles.len(), 4);

        let (cos0, sin0) = trig(0);
        let (cos180, sin180) = trig(180);

        let vec1 = [5.0 * cos0, 0.0, 5.0 * sin0];
        let vec2 = [5.0 * cos180, 0.0, 5.0 * sin180];
        let vec3 = [10.0 * cos0, -4.0, 10.0 * sin0];
        let vec4 = [10.0 * cos180, -4.0, 10.0 * sin180];

        assert_eq!(triangles[0].corners, [vec1, vec3, vec2]);
        assert_eq!(triangles[1].corners, [vec4, vec3, vec2]);
        assert_eq!(triangles[0].argb, 0xAA_BB_CC_DD);
        assert_eq!(triangles[0].texture, Some("effect\\lens2.tga"));
        assert_eq!(triangles[0].blend, SlotBlend::Alpha);
    }

    #[test]
    fn field_panel_draws_a_screen_facing_square() {
        let mut state = state_of(PrimitiveId::ParticleUp);
        state.render_flag = 5;
        state.texture = Some("effect\\pok1.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 100,
            full_display_angle: 0,
            distance: 10.0,
            flag1: [0; 21],
            ..Default::default()
        };

        let built = triangles(&state);

        assert_eq!(built.len(), 2);

        let (cos0, sin0) = trig(0);
        let (cos90, sin90) = trig(90);
        let (cos180, sin180) = trig(180);
        let (cos270, sin270) = trig(270);

        let corners = [
            [10.0 * cos0, 0.0, 10.0 * sin0],
            [10.0 * cos90, 0.0, 10.0 * sin90],
            [10.0 * cos180, 0.0, 10.0 * sin180],
            [10.0 * cos270, 0.0, 10.0 * sin270],
        ];

        assert_eq!(built[0].corners, [corners[0], corners[1], corners[2]]);
        assert_eq!(built[0].texture_coordinates, [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0],]);
        assert_eq!(built[1].corners, [corners[3], corners[2], corners[0]]);
        // `slots[0].flag1[2] == 0` passes `pw = 1`, i.e. plain alpha.
        assert_eq!(built[0].blend, SlotBlend::Alpha);
        assert_eq!(built[0].argb, pack_argb(100, 255, 255, 255));

        // `flag1[2] == 1` is a `pw = 0` glow case.
        let mut glowing = state;
        glowing.slots[0].flag1[2] = 1;
        let glowing_triangles = triangles(&glowing);
        assert_eq!(glowing_triangles[0].blend, SlotBlend::AlphaOne);
    }

    #[test]
    fn field_panel_without_alpha_draws_nothing() {
        let mut state = state_of(PrimitiveId::ParticleUp);
        state.render_flag = 5;
        state.texture = Some("effect\\pok1.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 0,
            distance: 10.0,
            ..Default::default()
        };

        assert!(triangles(&state).is_empty());
    }

    #[test]
    fn bottom_panel_uses_the_bottom_colour_and_corner_order() {
        // `EffectPrimitiveRendering.cpp:575-576`: PP_BOTTOM/PP_BOTTOMOUT draw
        // through `RenderFieldPanel`. `flag1[2] == 3` flips the quad, passes
        // `pw = 0` (glow blend) and colours it (200, 200, 100);
        // `height[9] == 10` shifts it four units in x; `vecBNow` offsets it.
        for id in [PrimitiveId::Bottom, PrimitiveId::BottomOut] {
            let mut state = state_of(id);
            state.render_flag = 5;
            state.texture = Some("effect\\twirl.bmp");
            state.slots[0] = EffectPrimitiveSlot {
                life: 1,
                alpha_b: 200,
                full_display_angle: 0,
                distance: 5.0,
                vec_b_now: [1.0, 2.0, 3.0],
                ..Default::default()
            };
            state.slots[0].height[9] = 10.0;
            state.slots[0].flag1[2] = 3;

            let built = triangles(&state);

            assert_eq!(built.len(), 2, "{id:?}");
            assert_eq!(built[0].texture, Some("effect\\twirl.bmp"));
            assert_eq!(built[0].blend, SlotBlend::AlphaOne, "{id:?}");
            assert_eq!(built[0].argb, pack_argb(200, 200, 200, 100), "{id:?}");

            let (cos0, sin0) = trig(0);
            let (cos90, sin90) = trig(90);
            let (cos180, sin180) = trig(180);
            let (cos270, sin270) = trig(270);

            let corners = [
                [5.0 * cos0 + 4.0 + 1.0, 2.0, 5.0 * sin0 + 3.0],
                [5.0 * cos90 + 4.0 + 1.0, 2.0, 5.0 * sin90 + 3.0],
                [5.0 * cos180 + 4.0 + 1.0, 2.0, 5.0 * sin180 + 3.0],
                [5.0 * cos270 + 4.0 + 1.0, 2.0, 5.0 * sin270 + 3.0],
            ];
            let flipped = [corners[1], corners[0], corners[3], corners[2]];

            assert_eq!(built[0].corners, [flipped[0], flipped[1], flipped[2]], "{id:?}");
            assert_eq!(built[1].corners, [flipped[3], flipped[2], flipped[0]], "{id:?}");
        }
    }

    #[test]
    fn shield_and_item_primitives_use_the_field_panel_geometry() {
        // `EffectPrimitiveRendering.cpp:573-592` routes PP_SHIELDBOOMERANG,
        // PP_THROWITEM and PP_SHIELDBOOMERANG3 through RenderFieldPanel.
        for id in [PrimitiveId::ShieldBoomerang, PrimitiveId::ThrowItem, PrimitiveId::ShieldBoomerang3] {
            let mut state = state_of(id);
            state.texture = Some("effect\\shield_boomerang.bmp");
            state.slots[0] = EffectPrimitiveSlot {
                life: 1,
                alpha_b: 120,
                full_display_angle: 0,
                distance: 7.0,
                ..Default::default()
            };

            let built = triangles(&state);

            assert_eq!(built.len(), 2, "id {id:?}");
            assert_eq!(built[0].texture, Some("effect\\shield_boomerang.bmp"));
            assert_eq!(built[0].argb, pack_argb(120, 255, 255, 255));
        }
    }

    #[test]
    fn pyramid_has_four_sides_and_a_fifth_step_texture_window() {
        let mut state = state_of(PrimitiveId::QuadHorn3d);
        state.size = 2.0;
        state.height_size = 5.0;
        state.argb = 0xFFFF_FFFF;
        state.texture = Some("effect\\stone.bmp");

        let triangles = triangles(&state);

        assert_eq!(triangles.len(), 4);

        assert_eq!(triangles[0].corners, [[-2.0, -2.0, 0.0], [0.0, 0.0, 5.0], [2.0, -2.0, 0.0],]);
        assert_eq!(triangles[0].texture_coordinates, [[0.0, 1.0], [0.0, 0.0], [0.2, 1.0],]);

        assert_eq!(triangles[1].corners, [[-2.0, -2.0, 0.0], [0.0, 0.0, 5.0], [-2.0, 2.0, 0.0],]);
        assert_eq!(triangles[1].texture_coordinates, [[0.2, 1.0], [0.2, 0.0], [0.4, 1.0],]);

        assert_eq!(triangles[3].texture_coordinates[0], [0.6, 1.0]);
        assert_eq!(triangles[3].texture_coordinates[2], [0.8, 1.0]);
    }

    #[test]
    fn arc_walls_draws_two_bands_per_live_slot() {
        let mut state = state_of(PrimitiveId::HeightRamp);
        state.size = 9.0;
        state.render_flag = 1;
        state.texture = Some("effect\\wall.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            full_display_angle: 360,
            rot_start: 0,
            distance: 10.0,
            rise_angle: 90,
            alpha_b: 100,
            height: [
                20.0, 40.0, 60.0, 80.0, 100.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
            ..Default::default()
        };

        let triangles = triangles(&state);

        // step = 90: five sweep positions, four quads per band, two bands.
        assert_eq!(triangles.len(), 16);
    }

    #[test]
    fn facing_quads_draws_four_copies_per_live_slot() {
        let mut state = state_of(PrimitiveId::RisingFan);
        state.render_flag = 1;
        state.texture = Some("effect\\fan.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            distance: 5.0,
            flag1: {
                let mut flag1 = [0; 21];
                flag1[5] = 7;
                flag1[11] = 40;
                flag1[12] = 50;
                flag1[13] = 60;
                flag1[14] = 70;
                flag1
            },
            ..Default::default()
        };

        let triangles = triangles(&state);

        // Four positions, two triangles each.
        assert_eq!(triangles.len(), 8);
        // `flag1[5] == 7` is (250, 70, 100).
        assert_eq!(triangles[0].argb, pack_argb(80, 250, 70, 100));
        assert_eq!(triangles[2].argb, pack_argb(100, 250, 70, 100));
        assert_eq!(triangles[4].argb, pack_argb(120, 250, 70, 100));
        assert_eq!(triangles[6].argb, pack_argb(140, 250, 70, 100));
    }

    #[test]
    fn ripple_draws_ten_rings_when_every_height_is_positive() {
        let mut state = state_of(PrimitiveId::RisingFan2);
        state.render_flag = 1;
        state.texture = Some("effect\\ripple.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            distance: 3.0,
            flag1: [0; 21],
            height: [
                0.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
            ..Default::default()
        };

        let triangles = triangles(&state);

        // ring = 10..1, all alphas positive: ten quads.
        assert_eq!(triangles.len(), 20);
        // The outermost ring (ring 10) is the bright blue.
        assert_eq!(triangles[0].argb, pack_argb(10, 55, 55, 255));
        // The others are the dim blue.
        assert_eq!(triangles[2].argb, pack_argb(10, 5, 5, 255));
    }

    #[test]
    fn sized_square_uses_the_mode_table_and_the_rot_start() {
        let mut state = state_of(PrimitiveId::Halo);
        state.size = 0.0;
        state.render_flag = 1;
        state.texture = Some("effect\\halo.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            rot_start: 0,
            distance: 10.0,
            rise_angle: 0,
            alpha_b: 90,
            height: [3.0; 21],
            ..Default::default()
        };

        let triangles = triangles(&state);

        // size 0 -> mode 0, one quad.
        assert_eq!(triangles.len(), 2);

        // radius = (sin(0) + 1) * (10 * 0.1) + 10 * 0.8 = 9.
        let (cos0, sin0) = trig(0);
        let (cos90, sin90) = trig(90);
        assert_eq!(triangles[0].corners[0], [9.0 * cos0, 3.0, 9.0 * sin0]);
        assert_eq!(triangles[0].corners[1], [9.0 * cos90, 3.0, 9.0 * sin90]);
        assert_eq!(triangles[0].blend, SlotBlend::Alpha);
    }

    #[test]
    fn ground_square_follows_the_rot_start_and_the_glow_selector() {
        let mut state = state_of(PrimitiveId::TextureSpin);
        state.render_flag = 1;
        state.texture = Some("effect\\spin.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            rot_start: 0,
            distance: 10.0,
            rise_angle: 0,
            alpha_b: 90,
            height: {
                let mut height = [0.0; 21];
                height[0] = 2.0;
                height[1] = 0.0;
                height[2] = 0.0;
                height
            },
            ..Default::default()
        };

        let built = triangles(&state);

        assert_eq!(built.len(), 2);
        assert_eq!(built[0].corners[0], [9.0, 2.0, 0.0]);
        // height[2] != 1 is the glow blend, but render_flag 1 (no
        // `RSF_CUSTOM_BLEND`) forces the alpha queue.
        assert_eq!(built[0].blend, SlotBlend::Alpha);

        let mut glowing = state;
        glowing.render_flag = 5;
        assert_eq!(triangles(&glowing)[0].blend, SlotBlend::AlphaOne);
    }

    #[test]
    fn twin_glow_squares_are_forced_onto_the_near_plane() {
        let mut state = state_of(PrimitiveId::WanderingFlames);
        state.render_flag = 1;
        state.texture = Some("effect\\fork.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            distance: 5.0,
            vec_b_pre: [1.0, 0.0, 2.0],
            vec_b_now: [3.0, 0.0, 4.0],
            height: {
                let mut height = [0.0; 21];
                height[12] = 90.0;
                height[13] = 60.0;
                height
            },
            ..Default::default()
        };

        let triangles = triangles(&state);

        // flag1[0] == 0 draws two quads per copy, at vecBPre and vecBNow.
        assert_eq!(triangles.len(), 8);
        assert!(triangles.iter().all(|triangle| triangle.forced_depth == Some(1.0)));

        let (cos45, sin45) = trig(45);
        // First copy: vecBPre + corner, then `render.position` (zero).
        assert_eq!(triangles[0].corners[0], [1.0 + 5.0 * cos45, 0.0, 2.0 + 5.0 * sin45]);
        // alphas: alpha/3 and alpha for height[12] = 90.
        assert_eq!(triangles[0].argb, pack_argb(30, 255, 255, 255));
        assert_eq!(triangles[2].argb, pack_argb(90, 50, 50, 255));
    }

    #[test]
    fn rise_arc_emits_one_strip_per_height_sample() {
        let mut state = state_of(PrimitiveId::Wind);
        state.render_flag = 1;
        state.texture = Some("effect\\wind.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            full_display_angle: 42,
            rot_start: 0,
            distance: 10.0,
            rise_angle: 90,
            max_height: 40.0,
            alpha_b: 100,
            height: [20.0; 21],
            ..Default::default()
        };

        let triangles = triangles(&state);

        // step = 42 / 21 = 2, offsets 0..40 (21 positions), the first seeds.
        assert_eq!(triangles.len(), 40);
        // `maxHeight` lowers the whole strip by 40.
        assert_eq!(triangles[0].corners[0][1], -40.0);
    }

    #[test]
    fn orbit_square_builds_the_tilted_square_at_the_slot_position() {
        let mut state = state_of(PrimitiveId::Intimidate);
        state.render_flag = 1;
        state.texture = Some("effect\\orbit.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 100,
            distance: 10.0,
            full_display_angle: 0,
            rise_angle: 0,
            rot_start: 0,
            height: {
                let mut height = [0.0; 21];
                height[1] = 5.0;
                height[2] = 0.0;
                height
            },
            ..Default::default()
        };

        let built = triangles(&state);

        // `flag1[0] == 0` draws one quad.
        assert_eq!(built.len(), 2);

        // baseX = distance = 10, offsetX = baseX * cos(riseAngle) = 10,
        // offsetY/offsetZ = 0, position = 0.
        let offset = [10.0, 0.0, 0.0];
        let point = |degrees: i32| [offset[0] + 5.0 * table_cos(degrees), 5.0 * table_sin(degrees), offset[2]];
        assert_eq!(built[0].corners[0], point(0));
        assert_eq!(built[0].corners[1], point(90));
        assert_eq!(built[0].argb, pack_argb(100, 100, 150, 255));
        assert_eq!(built[0].blend, SlotBlend::Alpha);
    }

    #[test]
    fn rect_up_draws_four_sides_and_the_two_crossing_quads() {
        let mut state = state_of(PrimitiveId::PulseKind);
        state.render_flag = 5;
        state.texture = Some("effect\\pulse.tga");
        let mut height = [0.0; 21];
        height[0] = 5.0;
        height[1] = 5.0;
        height[2] = 2.0;
        height[3] = 0.0;
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 100,
            rot_start: 0,
            height,
            ..Default::default()
        };

        let built = triangles(&state);

        assert_eq!(built.len(), 8);
        // y = -1 - height[2] = -3; the first side is the quad
        // (BPre, BNow, vec2, vec1), emitted as (0, 1, 2) then (3, 2, 0).
        assert_eq!(built[0].corners, [[5.0, -1.0, 5.0], [5.0, -1.0, -5.0], [5.0, -3.0, -5.0],]);
        assert_eq!(built[1].corners, [[5.0, -3.0, 5.0], [5.0, -3.0, -5.0], [5.0, -1.0, 5.0],]);
        // height[3] == 0 is the white glow.
        assert_eq!(built[0].argb, pack_argb(100, 255, 255, 255));
        assert_eq!(built[0].blend, SlotBlend::AlphaOne);
    }

    #[test]
    fn pentagon_sweeps_six_points_between_the_two_rings() {
        let mut state = state_of(PrimitiveId::ForestLight);
        state.render_flag = 1;
        state.texture = Some("effect\\forest.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 100,
            rot_start: 0,
            max_height: 5.0,
            vec_t_now: [0.0, 0.0, 0.0],
            vec_t_pre: [0.0, 10.0, 0.0],
            flag1: {
                let mut flag1 = [0; 21];
                flag1[0] = 13;
                flag1
            },
            ..Default::default()
        };

        let built = triangles(&state);

        // point 0 seeds, points 1..=5 draw.
        assert_eq!(built.len(), 10);
        let (cos72, sin72) = trig(72);
        assert_eq!(built[0].corners[0], [5.0, 0.0, 0.0]);
        assert_eq!(built[0].corners[1], [5.0 * cos72, 0.0, 5.0 * sin72]);
        assert_eq!(built[0].corners[2], [5.0 * cos72, 10.0, 5.0 * sin72]);
        assert_eq!(built[0].argb, pack_argb(100, 80, 80, 255));
    }

    #[test]
    fn fan_beam_draws_the_narrow_and_the_wide_fan() {
        let mut state = state_of(PrimitiveId::SpinQuad);
        state.render_flag = 5;
        state.texture = Some("effect\\fan.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 100,
            distance: 10.0,
            rise_angle: 0,
            rot_start: 0,
            ..Default::default()
        };

        let built = triangles(&state);

        // Two fans, one quad each.
        assert_eq!(built.len(), 4);

        // First fan: +-2 degrees; vec1 uses the rotated cosine for x, the
        // rotated sine for y and the raw sine for z.
        let (cos358, sin358) = trig(358);
        let (cos2, sin2) = trig(2);
        assert_eq!(built[0].corners[0], [10.0 * cos358, 0.0, 10.0 * sin358]);
        assert_eq!(built[0].corners[1], [10.0 * cos2, 0.0, 10.0 * sin2]);
        assert_eq!(built[0].corners[2], [0.0, 0.0, 0.0]);
        // mode 0 -> dim cyan for both fans, glow.
        assert_eq!(built[0].argb, pack_argb(100, 0, 250, 250));
        assert_eq!(built[0].blend, SlotBlend::AlphaOne);
        assert_eq!(built[2].argb, pack_argb(100, 250, 0, 0));
    }

    #[test]
    fn four_live_slots_are_kept_under_the_official_triangle_cap() {
        let mut state = state_of(PrimitiveId::PulsingColumns);
        state.size = 9.0;
        state.render_flag = 5;
        state.texture = Some("effect\\heal.tga");
        state.slots = [EffectPrimitiveSlot {
            life: 1,
            full_display_angle: 360,
            rot_start: 0,
            distance: 10.0,
            rise_angle: 90,
            max_height: 40.0,
            alpha_b: 100,
            height: [20.0; 21],
            ..Default::default()
        }; 4];

        let triangles = triangles(&state);

        assert_eq!(triangles.len(), 160);
        assert!(triangles.len() < MAX_TRIANGLES_PER_PRIMITIVE);
    }

    // -----------------------------------------------------------------------
    // RenderTeiFacingSlots (82)
    // -----------------------------------------------------------------------

    /// A camera with the same projection conventions as the game cameras
    /// (left handed, y up, reverse-z), at the origin.
    struct TestCamera {
        eye: Point3<f32>,
        focus: Point3<f32>,
        view: Matrix4<f32>,
        projection: Matrix4<f32>,
        view_projection: Matrix4<f32>,
        direction: Vector3<f32>,
    }

    impl TestCamera {
        /// Looks along `+z` from `(0, 0, -10)`.
        fn new() -> Self {
            Self::looking_towards(Vector3::unit_z())
        }

        /// Looks along `(cos p sin y, -sin p, cos p cos y)`, i.e. orbited by
        /// `(yaw, pitch)` exactly like the client camera.
        fn orbited(yaw_degrees: f32, pitch_degrees: f32) -> Self {
            let yaw = yaw_degrees.to_radians();
            let pitch = pitch_degrees.to_radians();

            Self::looking_towards(Vector3::new(pitch.cos() * yaw.sin(), -pitch.sin(), pitch.cos() * yaw.cos()))
        }

        /// A camera 10 world units behind the origin, looking along
        /// `direction`.
        fn looking_towards(direction: Vector3<f32>) -> Self {
            let eye = Point3::new(-direction.x * 10.0, -direction.y * 10.0, -direction.z * 10.0);
            let focus = Point3::new(eye.x + direction.x, eye.y + direction.y, eye.z + direction.z);
            let view = Matrix4::look_to_lh(eye, direction, Vector3::unit_y());
            let projection = crate::graphics::perspective_reverse_lh(Deg(45.0), 4.0 / 3.0);

            Self {
                eye,
                focus,
                view,
                projection,
                view_projection: projection * view,
                direction,
            }
        }
    }

    impl Camera for TestCamera {
        fn camera_position(&self) -> Point3<f32> {
            self.eye
        }

        fn focus_point(&self) -> Point3<f32> {
            self.focus
        }

        fn generate_view_projection(&mut self, _window_size: ScreenSize) {}

        fn look_up_vector(&self) -> Vector3<f32> {
            Vector3::unit_y()
        }

        fn view_projection_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>) {
            (self.view, self.projection)
        }

        fn view_projection_matrix(&self) -> Matrix4<f32> {
            self.view_projection
        }

        fn view_direction(&self) -> Vector3<f32> {
            self.direction
        }
    }

    /// A prim-82 state with one live slot and the default `TexturedAura`
    /// settings: texture slot 0, `render_flag` 5 (`ALPHA | CUSTOM_BLEND`),
    /// `alphaB = 200`, `distance = 10`.
    fn aura_state(kind: i8) -> EffectPrimitiveState {
        let mut state = state_of(PrimitiveId::AuraGlow);
        state.render_flag = 5;
        state.texture = Some("effect\\aura.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 200,
            distance: 10.0,
            ..Default::default()
        };
        state.slots[0].flag1[2] = kind;

        state
    }

    fn aura_point(distance: f32, angle: i32) -> [f32; 3] {
        let (cos, sin) = trig(angle);

        [distance * cos, 0.0, distance * sin]
    }

    fn assert_close(left: f32, right: f32) {
        assert!((left - right).abs() < 1e-4, "{left} != {right}");
    }

    fn assert_corners_close(left: [[f32; 2]; 3], right: [[f32; 2]; 3]) {
        for (left, right) in left.into_iter().zip(right) {
            assert_close(left[0], right[0]);
            assert_close(left[1], right[1]);
        }
    }

    #[test]
    fn face_camera_angles_are_recovered_from_the_view_direction() {
        // `+z`: yaw 0, pitch 0 -> 360 and 360 - 0 + 90.
        assert_eq!(face_camera_angles(&TestCamera::new()), [360, 90]);

        // A non-integral orbit keeps the client's `(short)` truncation.
        assert_eq!(face_camera_angles(&TestCamera::orbited(30.5, 20.5)), [330, 70]);

        // The port's default camera: `view_angle = 180`, `CAMERA_PITCH = -55`.
        assert_eq!(face_camera_angles(&TestCamera::orbited(180.0, -55.0)), [180, 145]);
    }

    #[test]
    fn face_camera_rotation_turns_the_quad_towards_the_camera() {
        let camera = TestCamera::orbited(30.5, 20.5);
        let facing = face_camera_angles(&camera);
        let mut state = aura_state(0);
        state.position = [1.0, 2.0, 3.0];
        state.slots[0].vec_b_now = [1.0, 2.0, 3.0];

        let mut corners: Quad = [
            aura_point(10.0, 0),
            aura_point(10.0, 270),
            aura_point(10.0, 180),
            aura_point(10.0, 90),
        ];
        tei_toward_screen(&mut corners, facing[0], facing[1]);

        // The x/z-plane quad's normal is rotated onto the camera direction
        // (within one integer table degree).
        let edge_a = [
            corners[1][0] - corners[0][0],
            corners[1][1] - corners[0][1],
            corners[1][2] - corners[0][2],
        ];
        let edge_b = [
            corners[3][0] - corners[0][0],
            corners[3][1] - corners[0][1],
            corners[3][2] - corners[0][2],
        ];
        let normal = [
            edge_a[1] * edge_b[2] - edge_a[2] * edge_b[1],
            edge_a[2] * edge_b[0] - edge_a[0] * edge_b[2],
            edge_a[0] * edge_b[1] - edge_a[1] * edge_b[0],
        ];
        let magnitude = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        let normal = [normal[0] / magnitude, normal[1] / magnitude, normal[2] / magnitude];
        let direction = camera.view_direction();

        // Facing the camera means the normal is anti-parallel to the view
        // direction; a 1 degree table error is at most ~0.018 in the dot.
        let dot = normal[0] * direction.x + normal[1] * direction.y + normal[2] * direction.z;
        assert!((dot + 1.0).abs() < 0.02, "normal . view = {dot}");
    }

    #[test]
    fn face_camera_vertical_shift_uses_integer_division() {
        assert_eq!(face_camera_vertical_shift(480, 1.0), -30.0);
        assert_eq!(face_camera_vertical_shift(1080, 1.0), -67.0, "-67.5 truncates toward zero");
        assert_eq!(face_camera_vertical_shift(1080, CEFFECT_PRIM_DISTANCE_SCALE), -67.0);
        assert_eq!(face_camera_vertical_shift(1080, 2.0), -134.0);
    }

    #[test]
    fn aura_kind_table_reproduces_the_official_switch_and_its_fall_through() {
        // `case 2` falls through 4, 5, 10, 12 and 15.
        let two: Vec<_> = facing_rects(2).iter().map(|rect| rect.colour).collect();
        assert_eq!(two, vec![
            (130, 130, 250),
            (255, 255, 255),
            (125, 255, 125),
            (150, 150, 150),
            (50, 50, 50),
            (51, 153, 153),
        ]);

        // Entering the chain lower emits a shorter suffix.
        assert_eq!(facing_rects(4).len(), 5);
        assert_eq!(facing_rects(5).len(), 4);
        assert_eq!(facing_rects(10).len(), 3);
        assert_eq!(facing_rects(12).len(), 2);
        assert_eq!(facing_rects(15).len(), 1);

        // Case 12 falls into case 15 too: there is no `break;` after it.
        assert_eq!(facing_rects(12)[1].colour, (51, 153, 153));

        // The two-rect cases.
        assert_eq!(facing_rects(18).iter().map(|rect| rect.colour).collect::<Vec<_>>(), vec![
            (5, 5, 255),
            (155, 155, 255)
        ]);
        assert_eq!(facing_rects(19).iter().map(|rect| rect.colour).collect::<Vec<_>>(), vec![
            (5, 255, 5),
            (155, 255, 155)
        ]);
        assert_eq!(facing_rects(20).iter().map(|rect| rect.colour).collect::<Vec<_>>(), vec![
            (255, 5, 5),
            (255, 155, 155)
        ]);

        // Only 0/3/16 pass `pw = 1`; only 16 is the `extra == 3` path.
        for kind in [0, 3, 16] {
            assert!(facing_rects(kind).iter().all(|rect| rect.pw_one), "kind {kind}");
        }
        for kind in [2, 4, 5, 10, 12, 15, 17, 18, 19, 20, 32] {
            assert!(facing_rects(kind).iter().all(|rect| !rect.pw_one), "kind {kind}");
        }
        assert!(facing_rects(16)[0].extra_three);

        // Unlisted kinds emit nothing.
        assert!(facing_rects(6).is_empty());
        assert!(facing_rects(14).is_empty());
        assert!(facing_rects(33).is_empty());
    }

    #[test]
    fn aura_kind_zero_builds_the_official_square() {
        let state = aura_state(0);
        let built = triangles(&state);

        assert_eq!(built.len(), 2);
        assert_eq!(built[0].texture, Some("effect\\aura.tga"));
        assert_eq!(built[0].argb, pack_argb(200, 255, 255, 255));
        assert_eq!(built[0].blend, SlotBlend::Alpha);
        assert_eq!(built[0].projected, None);
        assert_eq!(built[0].forced_depth, None);

        // Shape 0: 0, -90, -180, -270.
        let v1 = aura_point(10.0, 0);
        let v2 = aura_point(10.0, 270);
        let v3 = aura_point(10.0, 180);
        let v4 = aura_point(10.0, 90);

        // Cases 0/3 pass `(vec1, vec2, vec3, vec4)`; the quad is split
        // `(v1, v2, v3)` then `(v4, v3, v1)` with full-texture UVs.
        assert_eq!(built[0].corners, [v1, v2, v3]);
        assert_eq!(built[1].corners, [v4, v3, v1]);
        assert_eq!(built[0].texture_coordinates, [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]);
        assert_eq!(built[1].texture_coordinates, [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
    }

    #[test]
    fn aura_shape_one_uses_the_official_spacing() {
        // `TexturedAura` kind 1 sets `flag1[2] = 0` (renderer case 0/3) and
        // `height[10] = 1` (shape 1).
        let mut state = aura_state(0);
        state.slots[0].height[10] = 1.0;

        let built = triangles(&state);

        assert_eq!(built.len(), 2);
        assert_eq!(built[0].argb, pack_argb(200, 255, 255, 255));
        assert_eq!(built[0].blend, SlotBlend::Alpha);

        // Shape 1: 0, -60, -180, -240/120.
        let v1 = aura_point(10.0, 0);
        let v2 = aura_point(10.0, 300);
        let v3 = aura_point(10.0, 180);
        let v4 = aura_point(10.0, 120);

        // Case 0/3 passes `(vec1, vec2, vec3, vec4)`.
        assert_eq!(built[0].corners, [v1, v2, v3]);
        assert_eq!(built[1].corners, [v4, v3, v1]);
    }

    #[test]
    fn aura_kind_three_offsets_the_shape_two_square() {
        let mut state = aura_state(3);
        state.slots[0].height[10] = 2.0;

        let built = triangles(&state);

        assert_eq!(built.len(), 2);
        assert_eq!(built[0].argb, pack_argb(200, 255, 255, 255));
        assert_eq!(built[0].blend, SlotBlend::Alpha);

        // Shape 2: 0, -130, -180/180, -230/130 -> with `kind == 3` the whole
        // quad also moves +3.1 in x and +6 in z.
        let shifted = |angle: i32| {
            let mut point = aura_point(10.0, angle);
            point[0] += 3.1;
            point[2] += 6.0;

            point
        };
        let v1 = shifted(0);
        let v2 = shifted(230);
        let v3 = shifted(180);
        let v4 = shifted(50);

        assert_eq!(built[0].corners, [v1, v2, v3]);
        assert_eq!(built[1].corners, [v4, v3, v1]);
    }

    #[test]
    fn aura_kind_two_emits_the_whole_fall_through_chain() {
        let state = aura_state(2);
        let built = triangles(&state);

        // Six RenderTeiRect calls, two triangles each.
        assert_eq!(built.len(), 12);

        let expected = [
            (130, 130, 250),
            (255, 255, 255),
            (125, 255, 125),
            (150, 150, 150),
            (50, 50, 50),
            (51, 153, 153),
        ];

        for (index, colour) in expected.into_iter().enumerate() {
            let argb = pack_argb(200, colour.0, colour.1, colour.2);

            assert_eq!(built[index * 2].argb, argb, "rect {index}");
            assert_eq!(built[index * 2 + 1].argb, argb, "rect {index} second triangle");
            assert_eq!(built[index * 2].blend, SlotBlend::AlphaOne);
        }

        // Shape 0 corners, in the `(vec2, vec1, vec4, vec3)` order shared by
        // every rect of the chain.
        let v1 = aura_point(10.0, 0);
        let v2 = aura_point(10.0, 270);
        let v3 = aura_point(10.0, 180);
        let v4 = aura_point(10.0, 90);

        for quad in built.chunks(2) {
            assert_eq!(quad[0].corners, [v2, v1, v4]);
            assert_eq!(quad[1].corners, [v3, v4, v2]);
        }
    }

    #[test]
    fn aura_case_four_and_twelve_enter_mid_chain() {
        let four = triangles(&aura_state(4));

        assert_eq!(four.len(), 10, "case 4 emits 4, 5, 10, 12 and 15");
        assert_eq!(four[0].argb, pack_argb(200, 255, 255, 255));

        let twelve = triangles(&aura_state(12));

        assert_eq!(twelve.len(), 4, "case 12 emits 12 and then falls into 15");
        assert_eq!(twelve[0].argb, pack_argb(200, 50, 50, 50));
        assert_eq!(twelve[2].argb, pack_argb(200, 51, 153, 153));
    }

    #[test]
    fn aura_unlisted_kind_draws_nothing() {
        assert!(triangles(&aura_state(6)).is_empty());
        assert!(triangles(&aura_state(14)).is_empty());
    }

    #[test]
    fn aura_without_a_live_slot_or_alpha_draws_nothing() {
        // No live slot.
        let mut state = state_of(PrimitiveId::AuraGlow);
        state.texture = Some("effect\\aura.tga");
        assert!(triangles(&state).is_empty());

        // Live but transparent (`alpha <= 0`).
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 0,
            distance: 10.0,
            ..Default::default()
        };
        assert!(triangles(&state).is_empty());

        // A single alpha unit is enough (the official gate is `alpha <= 0`).
        state.slots[0].alpha_b = 1;
        assert_eq!(triangles(&state).len(), 2);
    }

    #[test]
    fn aura_uses_the_slot_flag_as_the_texture_index() {
        let mut state = aura_state(0);
        state.texture_secondary = Some("effect\\aura2.tga");
        state.slots[0].flag1[0] = 1;

        assert_eq!(triangles(&state)[0].texture, Some("effect\\aura2.tga"));
    }

    #[test]
    fn aura_kind_sixteen_uses_the_face_camera_clamp() {
        let camera = TestCamera::new();
        let context = SlotRenderContext {
            camera: &camera,
            viewport: WINDOW,
            distance_scale: CEFFECT_PRIM_DISTANCE_SCALE,
            facing: [0, 0],
        };

        // `TexturedAura` kind 22 sets `flag1[2] = 16`, `distance = 4`,
        // `height[10] = 0`, `fullDisplayAngle = 135` and lifts `vecBNow`.
        let mut state = aura_state(16);
        state.position = [1.0, 2.0, 3.0];
        state.slots[0].distance = 4.0;
        state.slots[0].full_display_angle = 135;
        state.slots[0].vec_b_now = [1.0, 7.0, 3.0];

        let built = triangles_with_context(&state, Some(&context));

        assert_eq!(built.len(), 2);

        // Rebuild the same four world corners, project them and apply the
        // official rewrite independently.
        let v1 = [aura_point(4.0, 135)[0] + 1.0, 7.0, aura_point(4.0, 135)[2] + 3.0];
        let v2 = [aura_point(4.0, 45)[0] + 1.0, 7.0, aura_point(4.0, 45)[2] + 3.0];
        let v3 = [aura_point(4.0, 315)[0] + 1.0, 7.0, aura_point(4.0, 315)[2] + 3.0];
        let v4 = [aura_point(4.0, 225)[0] + 1.0, 7.0, aura_point(4.0, 225)[2] + 3.0];

        // Case 16 passes `(vec2, vec1, vec4, vec3)`.
        let quad = [v2, v1, v4, v3];
        let pixels = quad.map(|corner| project_window_pixel(&camera, corner, WINDOW).expect("in front of the camera"));
        let anchor = project_window_pixel(&camera, state.position, WINDOW).expect("in front of the camera");
        let shift = face_camera_vertical_shift(WINDOW.height as i32, CEFFECT_PRIM_DISTANCE_SCALE);

        let to_ndc = |pixel: [f32; 2]| [pixel[0] / WINDOW.width * 2.0 - 1.0, 1.0 - pixel[1] / WINDOW.height * 2.0];

        let first_width = (pixels[0][0] - pixels[1][0]).abs() * 0.5;
        let first = [
            to_ndc([anchor[0] + first_width, pixels[0][1] + shift]),
            to_ndc([anchor[0] - first_width, pixels[1][1] + shift]),
            to_ndc([anchor[0] - first_width, pixels[2][1] + shift]),
        ];

        let second_width = (pixels[3][0] - pixels[2][0]).abs() * 0.5;
        let second = [
            to_ndc([anchor[0] + second_width, pixels[3][1] + shift]),
            to_ndc([anchor[0] - second_width, pixels[2][1] + shift]),
            to_ndc([anchor[0] + second_width, pixels[0][1] + shift]),
        ];

        assert_eq!(built[0].argb, pack_argb(200, 255, 255, 255));
        assert_eq!(built[0].blend, SlotBlend::Alpha);
        assert_eq!(built[0].forced_depth, None, "the depth lives in `projected`");
        assert_eq!(built[0].texture, Some("effect\\aura.tga"));

        let first_projected = built[0].projected.expect("kind 16 is pre-transformed");
        let second_projected = built[1].projected.expect("kind 16 is pre-transformed");

        assert_eq!(first_projected.depth, 1.0);
        assert_eq!(second_projected.depth, 1.0);
        assert_corners_close(first_projected.corners, first);
        assert_corners_close(second_projected.corners, second);

        assert_eq!(built[0].texture_coordinates, [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]);
        assert_eq!(built[1].texture_coordinates, [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
    }

    #[test]
    fn aura_kind_sixteen_without_a_camera_draws_nothing() {
        // The camera-less `triangles()` helper cannot reproduce
        // `TransformVert`; the live render path always supplies the context.
        assert!(triangles(&aura_state(16)).is_empty());
    }

    #[test]
    fn aura_draws_at_the_slots_vec_b_now() {
        let mut state = aura_state(0);
        state.slots[0].vec_b_now = [5.0, -30.0, 7.0];

        let built = triangles(&state);
        let v1 = aura_point(10.0, 0);

        assert_eq!(built[0].corners[0], [v1[0] + 5.0, -30.0, v1[2] + 7.0]);
    }

    // -----------------------------------------------------------------
    // RenderHalfSphere (104)
    // -----------------------------------------------------------------

    /// `TexturedPulse`'s slot: 8 radius, 8 drop, 180 degrees of longitude.
    fn half_sphere_state() -> EffectPrimitiveState {
        let mut state = state_of(PrimitiveId::HalfSphere);
        state.texture = Some("effect\\white02.bmp");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 120,
            rot_start: 0,
            full_display_angle: 180,
            max_height: 8.0,
            distance: 8.0,
            vec_t_now: [0.0, 0.0, 5.0],
            ..Default::default()
        };
        state
    }

    #[test]
    fn half_sphere_emits_ten_latitude_rows_of_ten_quads() {
        // `angle2 = 0..162` step 18 (10 rows) by `count = 0..180` step
        // `180 / 10 = 18` (10 emitting quads per row).
        assert_eq!(triangles(&half_sphere_state()).len(), 200);
    }

    #[test]
    fn half_sphere_quad_uses_the_official_corners_uvs_and_colour() {
        let mut state = half_sphere_state();
        state.position = [1.0, 2.0, 3.0];

        let built = triangles(&state);

        let cos18 = table_cos(18);
        let sin18 = table_sin(18);
        let (distance, max_height) = (8.0f32, 8.0f32);

        // Row `angle2 == 0`, `count == 0`: `sn2 == 0`, so the row's bottom
        // ring collapses onto the slot's `vecTNow`; no quad is emitted and
        // it becomes the next pass's `vec1`/`vec3`.
        let vec1 = [0.0, 0.0, 5.0];
        let vec3 = [sin18 * distance, (cos18 * distance) - max_height, 5.0];

        // Row `angle2 == 0`, `count == 18` (table angle 18).
        let vec4 = [
            cos18 * (sin18 * distance),
            (cos18 * distance) - max_height,
            sin18 * (sin18 * distance) + 5.0,
        ];

        // First quad = `(vec1, vec2, vec4, vec3)` with `vec2 == vec1` here, so
        // its first triangle is degenerate exactly like the original's.
        assert_eq!(built[0].corners[0], [vec1[0] + 1.0, vec1[1] + 2.0, vec1[2] + 3.0]);
        assert_eq!(built[0].corners[1], built[0].corners[0]);
        assert_eq!(built[0].texture_coordinates, [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]);

        let expected_second = [
            [vec3[0] + 1.0, vec3[1] + 2.0, vec3[2] + 3.0],
            [vec4[0] + 1.0, vec4[1] + 2.0, vec4[2] + 3.0],
            [vec1[0] + 1.0, vec1[1] + 2.0, vec1[2] + 3.0],
        ];

        assert_eq!(built[1].corners, expected_second);
        assert_eq!(built[1].texture_coordinates, [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
        assert_eq!(built[1].argb, pack_argb(120, 250, 250, 255));
        assert_eq!(built[1].texture, Some("effect\\white02.bmp"));
        assert_eq!(built[1].blend, SlotBlend::Alpha);
        assert_eq!(built[1].forced_depth, None);
    }

    #[test]
    fn half_sphere_wraps_the_longitude_with_one_subtraction() {
        // `rotStart = 350`, `count = 18` -> `368 - 360 = 8`; the renderer only
        // subtracts one full turn, like the original.
        let mut state = half_sphere_state();
        state.slots[0].rot_start = 350;
        state.slots[0].max_height = 0.0;
        state.slots[0].vec_t_now = [0.0; 3];

        let built = triangles(&state);

        // Row `angle2 == 18` starts at triangle 20; its first quad runs from
        // `count == 0` (angle 350) to `count == 18` (angle 8).
        let distance = 8.0f32;
        let outer = table_sin(18) * distance;

        assert_eq!(built[20].corners[1], [
            table_cos(8) * outer,
            table_cos(18) * distance,
            table_sin(8) * outer,
        ]);
    }

    #[test]
    fn half_sphere_draws_even_at_zero_alpha() {
        // Unlike `RenderRecordRings`, `RenderHalfSphere` gates on `life` only,
        // so `TexturedPulse`'s frame-0 shell is submitted fully transparent.
        let mut state = half_sphere_state();
        state.slots[0].alpha_b = 0;

        let built = triangles(&state);

        assert_eq!(built.len(), 200);
        assert_eq!(built[0].argb, pack_argb(0, 250, 250, 255));
    }

    #[test]
    fn half_sphere_caps_at_510_triangles() {
        // Four live slots would emit 800 triangles; `RenderTeiRect` defers to
        // `if (*rpCnt >= 510) return;`. Every triangle here is treated as
        // visible, which is the 510 the original submits in that case.
        let mut state = half_sphere_state();
        let slot = state.slots[0];

        for index in 1..4usize {
            state.slots[index] = slot;
        }

        assert_eq!(triangles(&state).len(), MAX_TRIANGLES_PER_PRIMITIVE);
    }

    // -----------------------------------------------------------------
    // RenderBladeFins (154)
    // -----------------------------------------------------------------

    /// `TexturedBeam`'s slot: 0 rise angle, 10 inner radius, 5 fin length.
    fn blade_fins_state(flag: i8) -> EffectPrimitiveState {
        let mut state = state_of(PrimitiveId::FlashFade);
        // `TexturedBeam` sets `RSF_ALPHA | RSF_CUSTOM_BLEND`.
        state.render_flag = 1 | 4;
        state.texture = Some("effect\\line3.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 40,
            rise_angle: 0,
            max_height: 5.0,
            height: [10.0; 21],
            flag1: [flag; 21],
            ..Default::default()
        };
        state
    }

    fn blade_fins_pair_corners() -> ([f32; 3], [f32; 3], [f32; 3], [f32; 3]) {
        let base = [0.0f32, -6.0, 0.0];
        let base2 = [0.0f32, -6.0, 0.0];
        let mut outer = [0.0f32, -6.0, 0.0];
        let mut inner = [0.0f32, -6.0, 0.0];

        let mut angle = -25;
        if angle < 0 {
            angle += 360;
        }
        inner[0] = 10.0 * table_cos(angle);
        inner[2] = 10.0 * table_sin(angle);
        angle -= 90;
        if angle < 0 {
            angle += 360;
        }
        outer[0] = 5.0 * table_cos(angle) + inner[0];
        outer[2] = 5.0 * table_sin(angle) + inner[2];

        (base, outer, inner, base2)
    }

    #[test]
    fn blade_fins_emits_two_quads_per_live_slot() {
        // Two mirrored pairs, each two `RenderTeiRect` calls = 8 triangles.
        assert_eq!(triangles(&blade_fins_state(1)).len(), 8);
    }

    #[test]
    fn blade_fins_quad_uses_the_official_corners_and_colours() {
        let mut state = blade_fins_state(1);
        state.position = [1.0, 2.0, 3.0];

        let built = triangles(&state);
        let (base, outer, inner, base2) = blade_fins_pair_corners();
        let offset = |corner: [f32; 3]| [corner[0] + 1.0, corner[1] + 2.0, corner[2] + 3.0];

        // First pair, first call: `alphaB / 4`, flag 1 -> (255, 89, 182).
        assert_eq!(built[0].corners, [offset(base), offset(outer), offset(inner)]);
        assert_eq!(built[1].corners, [offset(base2), offset(inner), offset(base)]);
        assert_eq!(built[0].texture_coordinates, [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]);
        assert_eq!(built[1].texture_coordinates, [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
        assert_eq!(built[0].argb, pack_argb(10, 255, 89, 182));
        assert_eq!(built[1].argb, pack_argb(10, 255, 89, 182));
        assert_eq!(built[0].blend, SlotBlend::Alpha);
        assert_eq!(built[0].texture, Some("effect\\line3.tga"));
        assert_eq!(built[0].forced_depth, Some(1.0));

        // First pair, second call: full alpha, flag 1 -> (255, 25, 25).
        assert_eq!(built[2].corners, [offset(base), offset(outer), offset(inner)]);
        assert_eq!(built[2].argb, pack_argb(40, 255, 25, 25));
        assert_eq!(built[2].forced_depth, Some(1.0));
    }

    #[test]
    fn blade_fins_mirrors_the_pair_and_keeps_the_minus_six_anchor() {
        let state = blade_fins_state(1);
        let built = triangles(&state);

        // Pair 2 is `riseAngle + 25`, then the same -90 turn.
        let inner = [10.0 * table_cos(25), -6.0, 10.0 * table_sin(25)];
        let mut angle = 25 - 90;
        if angle < 0 {
            angle += 360;
        }
        let outer = [5.0 * table_cos(angle) + inner[0], -6.0, 5.0 * table_sin(angle) + inner[2]];

        assert_eq!(built[4].corners[0], [0.0, -6.0, 0.0]);
        assert_eq!(built[4].corners[1], outer);
        assert_eq!(built[4].corners[2], inner);
        // All three keep the `-6` anchor for the base corners.
        assert_eq!(built[4].corners[0][1], -6.0);
        assert_eq!(built[5].corners[0][1], -6.0);
    }

    #[test]
    fn blade_fins_colours_follow_flag1_zero() {
        let flag_two = triangles(&blade_fins_state(2));
        assert_eq!(flag_two[0].argb, pack_argb(10, 255, 255, 255));
        assert_eq!(flag_two[2].argb, pack_argb(40, 255, 255, 255));

        let flag_zero = triangles(&blade_fins_state(0));
        assert_eq!(flag_zero[0].argb, pack_argb(10, 255, 255, 255));
        assert_eq!(flag_zero[2].argb, pack_argb(40, 25, 25, 255));
    }

    #[test]
    fn blade_fins_skip_dead_and_invisible_slots() {
        let mut state = blade_fins_state(1);
        state.slots[0].alpha_b = 0;

        assert!(triangles(&state).is_empty());

        state.slots[0].alpha_b = 40;
        state.slots[0].life = 0;

        assert!(triangles(&state).is_empty());
    }

    #[test]
    fn blade_fins_adds_the_primitive_position_to_every_corner() {
        // `f1 = 0` means "add render.position"; the renderer must not skip it.
        let mut state = blade_fins_state(1);
        state.position = [100.0, 200.0, 300.0];

        assert_eq!(triangles(&state)[0].corners[0], [100.0, 194.0, 300.0]);
    }

    // -----------------------------------------------------------------
    // RenderStackedRings (103)
    // -----------------------------------------------------------------

    /// `TexturedMark`'s default slot: 10 rings, 40 radius, 2 spacing,
    /// `height[0] = 50`, `height[1] = 5`.
    fn stacked_rings_state(kind: i8) -> EffectPrimitiveState {
        let mut state = state_of(PrimitiveId::StackedRings);
        // `TexturedMark` sets `RSF_ALPHA | RSF_CUSTOM_BLEND`.
        state.render_flag = 1 | 4;
        state.texture = Some("effect\\storm2.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 100,
            max_height: 40.0,
            distance: 2.0,
            height: {
                let mut height = [0.0f32; 21];
                height[0] = 50.0;
                height[1] = 5.0;
                height
            },
            flag1: {
                let mut flag1 = [0i8; 21];
                flag1[0] = 10;
                flag1[4] = kind;
                flag1
            },
            ..Default::default()
        };
        state
    }

    #[test]
    fn stacked_rings_emits_two_triangles_per_ring() {
        assert_eq!(triangles(&stacked_rings_state(0)).len(), 20);
    }

    #[test]
    fn stacked_rings_first_ring_uses_the_official_corners_and_colour() {
        let mut state = stacked_rings_state(0);
        state.position = [1.0, 2.0, 3.0];

        let built = triangles(&state);

        // Ring 0: `y = -height[0] = -50`, `radius = maxHeight = 40`,
        // `angle1..4 = 0, 90, 180, 270`; the bottom edge (`vec1`/`vec2`) is one
        // unit below the top edge (`vec3`/`vec4`). The tables are read with the
        // same integer indices the renderer uses.
        let radius = 40.0f32;
        let vec1 = [radius * table_cos(0), -51.0, radius * table_sin(0)];
        let vec2 = [radius * table_cos(90), -51.0, radius * table_sin(90)];
        let vec3 = [radius * table_cos(180), -50.0, radius * table_sin(180)];

        assert_eq!(built[0].corners[0], [vec1[0] + 1.0, vec1[1] + 2.0, vec1[2] + 3.0]);
        assert_eq!(built[0].corners[1], [vec2[0] + 1.0, vec2[1] + 2.0, vec2[2] + 3.0]);
        assert_eq!(built[0].corners[2], [vec3[0] + 1.0, vec3[1] + 2.0, vec3[2] + 3.0]);
        assert_eq!(built[0].texture_coordinates, [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]);
        assert_eq!(built[0].argb, pack_argb(100, 50, 50, 255));
        assert_eq!(built[0].texture, Some("effect\\storm2.tga"));
        // Kind 0 passes `pw = 0`, i.e. the `ONE` destination blend.
        assert_eq!(built[0].blend, SlotBlend::AlphaOne);
        assert_eq!(built[0].forced_depth, None);

        // The second triangle is `(vec4, vec3, vec1)`.
        let vec4 = [radius * table_cos(270), -50.0, radius * table_sin(270)];
        assert_eq!(built[1].corners[0], [vec4[0] + 1.0, vec4[1] + 2.0, vec4[2] + 3.0]);
        assert_eq!(built[1].corners[1], built[0].corners[2]);
        assert_eq!(built[1].corners[2], built[0].corners[0]);
        assert_eq!(built[1].texture_coordinates, [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
    }

    #[test]
    fn stacked_rings_second_ring_rises_height_one_and_shrinks_by_distance() {
        let state = stacked_rings_state(0);

        let built = triangles(&state);

        // Ring 1: `y = -50 + height[1] = -45`,
        // `radius = 40 - (2.2 - 0.2) * distance = 36`, `angle1 = 45`.
        let radius = 36.0f32;
        let expected = [radius * table_cos(45), -46.0, radius * table_sin(45)];

        assert_eq!(built[2].corners[0], expected);
    }

    #[test]
    fn stacked_rings_kind_selects_the_official_colour_and_blend() {
        // (kind, alpha, colour, blend)
        let expected = [
            (0i8, 100i16, (50u8, 50u8, 255u8), SlotBlend::AlphaOne),
            (1, 100, (164, 136, 55), SlotBlend::Alpha),
            (2, 100, (155, 255, 155), SlotBlend::Alpha),
            (3, 100, (130, 130, 255), SlotBlend::Alpha),
            (4, 100, (255, 50, 50), SlotBlend::Alpha),
            (5, 100, (5, 5, 5), SlotBlend::Alpha),
            (6, 100, (255, 255, 255), SlotBlend::Alpha),
            (7, 100, (255, 125, 255), SlotBlend::Alpha),
            // Kind 8 divides the alpha by 10.
            (8, 10, (225, 225, 255), SlotBlend::Alpha),
            (10, 100, (125, 125, 255), SlotBlend::AlphaOne),
        ];

        for (kind, alpha, colour, blend) in expected {
            let built = triangles(&stacked_rings_state(kind));

            assert_eq!(built.len(), 20, "kind {kind}");
            assert_eq!(built[0].argb, pack_argb(alpha, colour.0, colour.1, colour.2), "kind {kind}");
            assert_eq!(built[0].blend, blend, "kind {kind}");
            assert!(built.iter().all(|triangle| triangle.argb == built[0].argb), "kind {kind}");
        }
    }

    #[test]
    fn stacked_rings_kind_eleven_overlays_white_third_alpha_and_red() {
        let built = triangles(&stacked_rings_state(11));

        // Two quads per ring: white at `alphaB / 3`, then red at `alphaB`.
        assert_eq!(built.len(), 40);
        assert_eq!(built[0].argb, pack_argb(33, 255, 255, 255));
        assert_eq!(built[0].blend, SlotBlend::Alpha);
        assert_eq!(built[2].argb, pack_argb(100, 255, 55, 55));
        assert_eq!(built[2].blend, SlotBlend::AlphaOne);
    }

    #[test]
    fn stacked_rings_skips_dead_slots_unlisted_kinds_and_zero_alpha() {
        // Kind 9 has no case in the original switch.
        assert!(triangles(&stacked_rings_state(9)).is_empty());

        let mut transparent = stacked_rings_state(0);
        transparent.slots[0].alpha_b = 0;
        assert!(triangles(&transparent).is_empty());

        let mut dead = stacked_rings_state(0);
        dead.slots[0].life = 0;
        assert!(triangles(&dead).is_empty());
    }

    // -----------------------------------------------------------------
    // RenderPeongUp (96)
    // -----------------------------------------------------------------

    /// `WhiteLightTrail`'s kind-1 slot: one 10-unit square 5 below the anchor,
    /// blue (`flag1[0] == 2`) and `alphaB = 40`.
    fn peong_up_state() -> EffectPrimitiveState {
        let mut state = state_of(PrimitiveId::PeongUp);
        // `WhiteLightTrail` sets `RSF_ALPHA | RSF_CUSTOM_BLEND`.
        state.render_flag = 1 | 4;
        state.texture = Some("effect\\whitelight.tga");
        state.position = [1.0, 2.0, 3.0];
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 40,
            distance: 10.0,
            vec_b_pre: [0.0, -5.0, 0.0],
            flag1: {
                let mut flag1 = [0i8; 21];
                flag1[0] = 2;
                flag1
            },
            ..Default::default()
        };
        state
    }

    #[test]
    fn peong_up_emits_the_four_corner_quad_with_the_blue_colour() {
        let built = triangles(&peong_up_state());

        assert_eq!(built.len(), 2);

        let radius = 10.0;
        let first = [table_cos(45) * radius + 1.0, -5.0 + 2.0, table_sin(45) * radius + 3.0];
        assert_eq!(built[0].corners[0], first);
        assert_eq!(built[0].texture, Some("effect\\whitelight.tga"));
        assert_eq!(built[0].argb, pack_argb(40, 105, 105, 225));
        // `pw = 0` selects the `SRCALPHA`/`ONE` glow blend.
        assert_eq!(built[0].blend, SlotBlend::AlphaOne);
        assert_eq!(built[0].forced_depth, None);
        assert_eq!(built[0].texture_coordinates, [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]);

        // The second triangle is `(vec4, vec3, vec1)`.
        let fourth = [table_cos(315) * radius + 1.0, -5.0 + 2.0, table_sin(315) * radius + 3.0];
        assert_eq!(built[1].corners[0], fourth);
        assert_eq!(built[1].corners[1], built[0].corners[2]);
        assert_eq!(built[1].corners[2], built[0].corners[0]);
        assert_eq!(built[1].texture_coordinates, [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
    }

    #[test]
    fn peong_up_colour_table_matches_flag1_zero() {
        let expected = [(2i8, (105u8, 105u8, 225u8)), (3, (55, 55, 55)), (1, (255, 255, 255))];

        for (flag, colour) in expected {
            let mut state = peong_up_state();
            state.slots[0].flag1[0] = flag;

            let built = triangles(&state);

            assert_eq!(built[0].argb, pack_argb(40, colour.0, colour.1, colour.2), "flag {flag}");
        }
    }

    #[test]
    fn peong_up_rotates_the_quad_with_the_face_camera_angles() {
        let mut flat = peong_up_state();
        flat.slots[0].alpha_t = 0;
        flat.slots[1].alpha_t = 0;

        let mut turned = peong_up_state();
        turned.slots[0].alpha_t = 90;
        turned.slots[1].alpha_t = 0;

        let flat_corner = triangles(&flat)[0].corners[0];
        let turned_corner = triangles(&turned)[0].corners[0];

        // `Tei_TowardScreen`'s `RotatePairY(90)` maps (x, z) -> (-z, x).
        let expected = [table_cos(45) * 10.0, 0.0, table_sin(45) * 10.0];
        let close = |left: f32, right: f32| (left - right).abs() < 1e-5;

        assert_eq!(flat_corner, [expected[0] + 1.0, -3.0, expected[2] + 3.0]);
        assert!(close(turned_corner[0], -expected[2] + 1.0), "{turned_corner:?}");
        assert_eq!(turned_corner[1], -3.0);
        assert!(close(turned_corner[2], expected[0] + 3.0), "{turned_corner:?}");
    }

    #[test]
    fn peong_up_skips_dead_slots_and_quads_above_the_anchor() {
        let mut dead = peong_up_state();
        dead.slots[0].life = 0;
        assert!(triangles(&dead).is_empty());

        let mut above = peong_up_state();
        above.slots[0].vec_b_pre[1] = 1.0;
        assert!(triangles(&above).is_empty());
    }

    // -----------------------------------------------------------------------
    // RenderHaloLayers (68) / RenderOrbitIcons (35)
    // -----------------------------------------------------------------------

    const SUPER_TEXTURES: [&str; 5] = [
        "effect\\super1.bmp",
        "effect\\super2.bmp",
        "effect\\super3.bmp",
        "effect\\super4.bmp",
        "effect\\super5.bmp",
    ];

    fn halo_state(kind: i8) -> EffectPrimitiveState {
        let mut state = state_of(PrimitiveId::AsuraCasting);
        state.render_flag = 5;
        state.texture_frames = vec!["effect\\asura3.tga", "effect\\asura2.tga", "effect\\asura1.tga"];
        state.texture = Some("effect\\asura3.tga");
        state.slots[0].life = 1;
        state.slots[0].flag1[4] = kind;
        state.slots[0].distance = 4.1;

        for index in 1..=10 {
            state.slots[0].height[index] = 100.0;
        }

        state
    }

    fn orbit_state() -> EffectPrimitiveState {
        let mut state = state_of(PrimitiveId::FloatingBubbles);
        state.render_flag = 5;
        state.texture_frames = SUPER_TEXTURES.to_vec();
        state.texture = Some(SUPER_TEXTURES[0]);
        state.slots[0].life = 1;
        state.slots[0].distance = 3.5;
        state.slots[0].height[1] = 70.0;
        state.slots[0].height[2] = 3.0;
        state.slots[0].height[3] = 10.0;
        state.slots[0].height[5] = 130.0;
        state.slots[0].height[6] = 4.0;
        state.slots[0].height[7] = 20.0;
        state.slots[0].height[9] = 190.0;
        state.slots[0].height[10] = 0.0;
        state.slots[0].height[11] = 30.0;
        state.slots[0].height[13] = 250.0;
        state.slots[0].height[14] = 1.0;
        state.slots[0].height[15] = 40.0;
        state.slots[0].vec_b_pre = [0.0, -4.0, 0.0];
        state.slots[0].vec_b_now = [0.0, -5.0, 0.0];
        state.slots[0].vec_t_pre = [0.0, -6.0, 0.0];
        state.slots[0].vec_t_now = [0.0, -7.0, 0.0];

        state
    }

    #[test]
    fn the_asura_renderers_are_dispatched() {
        assert_eq!(render(&state_of(PrimitiveId::AsuraCasting)), Some(SlotRenderer::HaloLayers));
        assert_eq!(render(&state_of(PrimitiveId::FloatingBubbles)), Some(SlotRenderer::OrbitIcons));
        assert_eq!(render(&state_of(PrimitiveId::Casting3d)), Some(SlotRenderer::Casting));
        assert_eq!(render(&state_of(PrimitiveId::WaveSpin)), Some(SlotRenderer::Casting));
    }

    #[test]
    fn halo_layers_draw_ten_black_quads_for_kind_zero() {
        let state = halo_state(0);
        let built = triangles(&state);

        // Ten layers, one quad (two triangles) each.
        assert_eq!(built.len(), 20);

        let (cos45, sin45) = trig(45);
        let (cos135, sin135) = trig(135);
        let (cos315, sin315) = trig(315);

        // Layer i = 10: `step = 0`, so the radius is the slot distance; the
        // corners are `vec2, vec1, vec4, vec3` lifted by 30.
        let b_now = [4.1 * cos135, -30.0, sin135 * 4.1];
        let b_pre = [4.1 * cos45, -30.0, sin45 * 4.1];
        let t_pre = [4.1 * cos315, -30.0, sin315 * 4.1];
        let close = |left: [f32; 3], right: [f32; 3]| {
            (left[0] - right[0]).abs() < 1e-4 && (left[1] - right[1]).abs() < 1e-4 && (left[2] - right[2]).abs() < 1e-4
        };
        assert!(close(built[0].corners[0], b_now), "{:?}", built[0].corners[0]);
        assert!(close(built[0].corners[1], b_pre), "{:?}", built[0].corners[1]);
        assert!(close(built[0].corners[2], t_pre), "{:?}", built[0].corners[2]);

        // Kind 0 draws black through the straight alpha queue and uses
        // `textures[flag1[0]]`.
        assert_eq!(built[0].argb, pack_argb(100, 0, 0, 0));
        assert_eq!(built[0].blend, SlotBlend::Alpha);
        assert_eq!(built[0].texture, Some("effect\\asura3.tga"));
        assert_eq!(built[0].texture_coordinates, [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]);
    }

    #[test]
    fn halo_layers_kind_three_doubles_the_outermost_layer() {
        let built = triangles(&halo_state(3));

        // Eleven quads: layer 10 draws twice.
        assert_eq!(built.len(), 22);
        assert_eq!(built[0].argb, pack_argb(50, 10, 10, 10));
        assert_eq!(built[2].argb, pack_argb(100, 10, 10, 10));
        assert_eq!(built[4].argb, pack_argb(100, 10, 10, 10));
    }

    #[test]
    fn halo_layers_kind_five_adds_the_scaled_outer_pass() {
        let built = triangles(&halo_state(5));

        // Two quads per layer, ten layers.
        assert_eq!(built.len(), 40);

        // First pass: white, straight alpha.
        assert_eq!(built[0].argb, pack_argb(100, 255, 255, 255));
        assert_eq!(built[0].blend, SlotBlend::Alpha);

        // Second pass: blue glow at 1.1x the radius.
        let (cos45, _) = trig(45);
        assert_eq!(built[2].argb, pack_argb(100, 5, 5, 255));
        assert_eq!(built[2].blend, SlotBlend::AlphaOne);
        let expected = 4.1 * 1.1 * cos45;
        assert!((built[2].corners.iter().map(|corner| corner[0]).fold(0.0f32, f32::max) - expected).abs() < 1e-4);
    }

    #[test]
    fn orbit_icons_draw_four_quads_and_pick_the_texture_by_index() {
        let built = triangles(&orbit_state());

        assert_eq!(built.len(), 8);

        let (cos10, sin10) = trig(10);
        let (cos45, sin45) = trig(45);
        let (cos135, sin135) = trig(135);

        // Record 0 keeps its order when `(index == 0 || index == 2) ==
        // |angle - roll| < 80`; index 3 makes both sides false, so the
        // corners swap to `vec2, vec1, vec4, vec3`.
        let offset = [cos10 * EFFECT_ICON_RADIUS, -4.0, sin10 * EFFECT_ICON_RADIUS];
        let first = [3.5 * cos135 + offset[0], offset[1], 3.5 * sin135 + offset[2]];
        let second = [3.5 * cos45 + offset[0], offset[1], 3.5 * sin45 + offset[2]];
        let close = |left: [f32; 3], right: [f32; 3]| {
            (left[0] - right[0]).abs() < 1e-4 && (left[1] - right[1]).abs() < 1e-4 && (left[2] - right[2]).abs() < 1e-4
        };
        assert!(close(built[0].corners[0], first), "{:?}", built[0].corners[0]);
        assert!(close(built[0].corners[1], second), "{:?}", built[0].corners[1]);

        // Alpha from `height[1]`, texture from `height[2]`.
        assert_eq!(built[0].argb, pack_argb(70, 100, 100, 255));
        assert_eq!(built[0].texture, Some("effect\\super4.bmp"));
        assert_eq!(built[0].blend, SlotBlend::AlphaOne);
    }

    #[test]
    fn orbit_icons_switch_to_red_for_kind_one_and_keep_it() {
        let mut state = orbit_state();
        state.slots[1] = state.slots[0];
        state.slots[0].flag1[4] = 1;

        let built = triangles(&state);
        assert_eq!(built.len(), 16);

        // The original overwrites its colour parameters, so every later slot is
        // red too.
        assert_eq!(built[0].argb, pack_argb(70, 255, 20, 20));
        assert_eq!(built[8].argb, pack_argb(70, 255, 20, 20));
    }

    #[test]
    fn orbit_icons_subtract_the_master_name_offset() {
        let mut state = orbit_state();
        state.slots[0].height[20] = 6.0;

        let built = triangles(&state);

        assert_eq!(built[0].corners[0][1], -4.0 - 6.0);
        assert_eq!(built[0].corners[1][1], -4.0 - 6.0);
    }

    // -----------------------------------------------------------------------
    // RenderDoubleGumGang (99) / RenderHaloLayers (147)
    // -----------------------------------------------------------------------

    /// `SuperSlashSpread(2, value)`'s slot: `distance = 2`, four records at
    /// alpha 40/80/120/160, textures `height[2|6|10|14]`, angles
    /// `height[3|7|11|15]`.
    fn double_gum_gang_state() -> EffectPrimitiveState {
        let mut state = state_of(PrimitiveId::DoubleGumGang);
        // `state.renderFlag = RSF_ALPHA | RSF_CUSTOM_BLEND;`
        state.render_flag = 1 | 4;
        state.texture_frames = SUPER_TEXTURES.to_vec();
        state.texture = Some(SUPER_TEXTURES[0]);
        state.slots[0].life = 1;
        state.slots[0].process = 40;
        state.slots[0].distance = 2.0;
        state.slots[0].height[0] = 10.0;
        state.slots[0].height[1] = 40.0;
        state.slots[0].height[2] = 3.0;
        state.slots[0].height[3] = 10.0;
        state.slots[0].height[5] = 80.0;
        state.slots[0].height[6] = 4.0;
        state.slots[0].height[7] = 20.0;
        state.slots[0].height[9] = 120.0;
        state.slots[0].height[10] = 0.0;
        state.slots[0].height[11] = 30.0;
        state.slots[0].height[13] = 160.0;
        state.slots[0].height[14] = 1.0;
        state.slots[0].height[15] = 40.0;
        state.slots[0].vec_b_pre = [0.0, -4.0, 0.0];
        state.slots[0].vec_b_now = [0.0, -5.0, 0.0];
        state.slots[0].vec_t_pre = [0.0, -6.0, 0.0];
        state.slots[0].vec_t_now = [0.0, -7.0, 0.0];

        state
    }

    #[test]
    fn the_super_slash_renderers_are_dispatched() {
        assert_eq!(render(&state_of(PrimitiveId::DoubleGumGang)), Some(SlotRenderer::DoubleGumGang));
        assert_eq!(render(&state_of(PrimitiveId::TexturedSlam)), Some(SlotRenderer::HaloLayers));
    }

    #[test]
    fn double_gum_gang_draws_three_scales_of_four_icon_quads() {
        let built = triangles(&double_gum_gang_state());

        // Three scales x four slot vectors x one quad (two triangles).
        assert_eq!(built.len(), 24);

        let (cos10, sin10) = trig(10);
        let (cos45, sin45) = trig(45);
        let (cos135, sin135) = trig(135);
        let close = |left: [f32; 3], right: [f32; 3]| {
            (left[0] - right[0]).abs() < 1e-4 && (left[1] - right[1]).abs() < 1e-4 && (left[2] - right[2]).abs() < 1e-4
        };

        // Layer 0 (`rdist = 1.0`, radius 2), record 0: texture 3 makes both
        // sides of the visibility test differ, so the corners swap to
        // `vec2, vec1, vec4, vec3`. Offset y is the body's random -4.
        let offset = [cos10 * DOUBLE_GUMGANG_ICON_LENGTH, -4.0, sin10 * DOUBLE_GUMGANG_ICON_LENGTH];
        let first = [2.0 * cos135 + offset[0], offset[1], 2.0 * sin135 + offset[2]];
        let second = [2.0 * cos45 + offset[0], offset[1], 2.0 * sin45 + offset[2]];
        assert!(close(built[0].corners[0], first), "{:?}", built[0].corners[0]);
        assert!(close(built[0].corners[1], second), "{:?}", built[0].corners[1]);

        // Blue layer 0, alpha `min(40, process)`, texture `height[2] = 3`,
        // `pw = 0` -> `SRCALPHA`/`ONE`.
        assert_eq!(built[0].argb, pack_argb(40, 100, 100, 255));
        assert_eq!(built[0].texture, Some(SUPER_TEXTURES[3]));
        assert_eq!(built[0].blend, SlotBlend::AlphaOne);

        // Layer 1 (i = 1) uses the darker blue at `distance * 1.2`.
        assert_eq!(built[8].argb, pack_argb(40, 0, 0, 120));

        // Layer 2 (i = 2) scales by 2.0 and keeps the same blue.
        assert_eq!(built[16].argb, pack_argb(40, 0, 0, 120));
        let (cos135, sin135) = trig(135);
        let offset = [cos10 * DOUBLE_GUMGANG_ICON_LENGTH, -4.0, sin10 * DOUBLE_GUMGANG_ICON_LENGTH];
        let expected = [2.0 * 2.0 * cos135 + offset[0], offset[1], 2.0 * 2.0 * sin135 + offset[2]];
        assert!(close(built[16].corners[0], expected), "{:?}", built[16].corners[0]);
    }

    #[test]
    fn double_gum_gang_kind_selects_the_colour_table() {
        for (kind, colour) in [(2i8, (255u8, 100u8, 100u8)), (1, (90, 90, 80))] {
            let mut state = double_gum_gang_state();
            state.slots[0].flag1[4] = kind;

            assert_eq!(
                triangles(&state)[0].argb,
                pack_argb(40, colour.0, colour.1, colour.2),
                "kind {kind}"
            );
        }
    }

    #[test]
    fn double_gum_gang_alpha_is_capped_by_the_process_counter() {
        let mut state = double_gum_gang_state();
        state.slots[0].process = 25;

        let built = triangles(&state);

        // Every one of the four records is capped at the same process value.
        for triangle in built.iter().take(4) {
            assert_eq!(triangle.argb, pack_argb(25, 100, 100, 255));
        }
    }

    #[test]
    fn double_gum_gang_follows_the_face_camera_angles() {
        let mut turned = double_gum_gang_state();
        turned.slots[0].alpha_t = 90;
        turned.slots[1].alpha_t = 0;

        let built = triangles(&turned);
        let (cos10, sin10) = trig(10);
        let (cos45, sin45) = trig(45);

        // `RotatePairY(90)` maps the 135 degree base corner to (x, z) =
        // (-sin135, cos135) * 2, then the icon offset is added unrotated.
        let offset = [cos10 * DOUBLE_GUMGANG_ICON_LENGTH, -4.0, sin10 * DOUBLE_GUMGANG_ICON_LENGTH];
        let expected = [-2.0 * sin45 + offset[0], offset[1], 2.0 * cos45 + offset[2]];
        let close = |left: f32, right: f32| (left - right).abs() < 1e-4;

        assert!(close(built[0].corners[0][0], expected[0]), "{:?}", built[0].corners[0]);
        assert_eq!(built[0].corners[0][1], expected[1]);
        assert!(close(built[0].corners[0][2], expected[2]), "{:?}", built[0].corners[0]);
    }

    /// `TexturedSlam(name, kind)`'s kind-6 slot: `distance = 21`, ten layers at
    /// alpha 100, `flag1[4] = 6`.
    fn textured_slam_state() -> EffectPrimitiveState {
        let mut state = state_of(PrimitiveId::TexturedSlam);
        state.render_flag = 1 | 4;
        state.texture = Some("effect\\priest_spell.bmp");
        state.slots[0].life = 1;
        state.slots[0].distance = 21.0;
        state.slots[0].flag1[4] = 6;

        for index in 1..=10 {
            state.slots[0].height[index] = 100.0;
        }

        state
    }

    #[test]
    fn textured_slam_draws_the_halo_layers_with_stored_angles() {
        let built = triangles(&textured_slam_state());

        // Ten layers, two quads each for kind 6 (the 1.1x outer pass).
        assert_eq!(built.len(), 40);
        assert_eq!(built[0].argb, pack_argb(100, 255, 255, 255));
        assert_eq!(built[0].blend, SlotBlend::AlphaOne);
        assert_eq!(built[0].texture, Some("effect\\priest_spell.bmp"));
        assert_eq!(built[2].argb, pack_argb(100, 5, 5, 255));

        // The first pass is dropped by 40 (`kind 6`); layer 10 sits on the
        // 35/145 degree diagonal, and the quad is `(vec2, vec1, vec4, vec3)`.
        let (cos145, sin145) = trig(145);
        let close = |left: f32, right: f32| (left - right).abs() < 1e-4;
        assert!(close(built[0].corners[0][0], 21.0 * cos145));
        assert!(close(built[0].corners[0][1], -40.0));
        assert!(close(built[0].corners[0][2], 21.0 * sin145));
    }

    #[test]
    fn textured_slam_ignores_the_camera_facing() {
        let camera = TestCamera::new();
        let context = SlotRenderContext {
            camera: &camera,
            viewport: WINDOW,
            distance_scale: CEFFECT_PRIM_DISTANCE_SCALE,
            facing: [90, 45],
        };

        let state = textured_slam_state();
        let with_camera = triangles_with_context(&state, Some(&context));
        let without = triangles(&state);

        // `PrimRisingFan` does not call `FaceCamera`, so prim 147 keeps the
        // constructor's zeroed `alphaT` and ignores the camera angles.
        assert_eq!(with_camera, without);

        // Prim 68 still uses them.
        let asura = halo_state(0);
        let turned = triangles_with_context(&asura, Some(&context));
        assert_ne!(turned, triangles(&asura));
    }

    #[test]
    fn penta_ring_sweeps_five_quads_per_pass() {
        let mut state = state_of(PrimitiveId::PentaRing);
        state.render_flag = 5;
        state.texture = Some("effect\\cloud11.tga");
        state.slots[0].life = 1;
        state.slots[0].alpha_b = 100;
        state.slots[0].max_height = 10.0;
        state.slots[0].rot_start = 0;
        state.slots[0].height[0] = 0.0;
        state.slots[0].vec_b_now = [0.0, 0.0, 0.0];
        state.slots[0].vec_t_now = [0.0, 10.0, 0.0];

        let built = triangles(&state);

        // Two passes, six ring points each, five quads, two triangles.
        assert_eq!(built.len(), 20);
        assert_eq!(built[0].argb, pack_argb(100, 255, 255, 55));
        assert_eq!(built[0].blend, SlotBlend::Alpha);
        assert_eq!(built[0].texture, Some("effect\\cloud11.tga"));
        assert_eq!(built[0].forced_depth, None);

        // `direction = (rotStart + 90) % 360`; the first quad spans angle
        // 0 -> 72 on pass 0 (radius = maxHeight).
        let (cos90, sin90) = trig(90);
        let (cos0, sin0) = trig(0);
        let across = 10.0 * cos0;
        let first = [across * cos90, 10.0 * sin0, across * sin90];
        let (cos72, sin72) = trig(72);
        let across72 = 10.0 * cos72;
        let second = [across72 * cos90, 10.0 * sin72, across72 * sin90];
        let second_top = [second[0], second[1] + 10.0, second[2]];

        for (actual, expected) in built[0].corners.into_iter().zip([first, second, second_top]) {
            for axis in 0..3 {
                assert_close(actual[axis], expected[axis]);
            }
        }
    }

    #[test]
    fn penta_ring_kinds_four_and_five_force_the_near_plane() {
        for kind in [4.0f32, 5.0] {
            let mut state = state_of(PrimitiveId::PentaRing);
            state.render_flag = 5;
            state.slots[0].life = 1;
            state.slots[0].alpha_b = 100;
            state.slots[0].max_height = 10.0;
            state.slots[0].height[4] = kind;

            let built = triangles(&state);

            assert!(!built.is_empty());
            for triangle in &built {
                assert_eq!(triangle.forced_depth, Some(1.0), "kind {kind}");
            }
        }
    }

    #[test]
    fn hitline3_builds_the_four_swept_cross_sections() {
        let mut state = state_of(PrimitiveId::HitLine3);
        state.render_flag = 5;
        state.texture = Some("effect\\white02.bmp");

        let slot = &mut state.slots[0];
        slot.life = 1;
        slot.alpha_b = 200;
        slot.process = 1;
        slot.max_height = 10.0;
        slot.distance = 20.0;
        slot.rise_angle = 0;
        slot.alpha_t = 3;
        slot.vec_t_now[0] = 1.0;
        slot.vec_t_now[1] = 4.0;
        slot.height[0] = 45.0;
        slot.height[1] = 90.0;
        slot.height[2] = 45.0;
        slot.height[3] = 90.0;
        slot.height[4] = 45.0;
        slot.height[5] = 90.0;

        let built = triangles(&state);

        // Four samples, three cross sections plus three halo sections, each
        // two triangles: 4 * 6 * 2.
        assert_eq!(built.len(), 48);
        assert_eq!(built[0].argb, pack_argb(200, 105, 105, 105));
        assert_eq!(built[0].blend, SlotBlend::AlphaOne);
        assert_eq!(built[0].texture, Some("effect\\white02.bmp"));

        // First emit is cross section i = 1 of sample j = 0.
        let (cos0, sin0) = trig(0);
        let (_, sin45) = trig(45);
        let (_, sin90) = trig(90);
        let (cos120, sin120) = trig(120);
        let (cos270, sin270) = trig(270);

        let speed_a = 45.0 / 90.0 * 20.0;
        let a = [speed_a * cos0, -20.0 * sin45 * 1.0, speed_a * sin0];
        let speed_b = 90.0 / 90.0 * 20.0;
        let b = [speed_b * cos0, -20.0 * sin90 * 1.0, speed_b * sin0];

        // `Get_Angle(a.x - b.x, a.z - b.z)` is 180, then `+ 90` wraps to 270.
        let angle = super::super::primitive::get_angle(a[0] - b[0], a[2] - b[2]) as i32 + 90;
        assert_eq!(angle, 270);
        let tx = 10.0 * cos120;
        let ry = 10.0 * sin120 - 5.0;
        let rx = tx * cos270;
        let rz = tx * sin270;

        let vec2 = [a[0] + rx, a[1] + ry, a[2] + rz];
        for axis in 0..3 {
            assert_close(built[0].corners[0][axis], vec2[axis]);
        }
    }

    #[test]
    fn rotated_plate_builds_the_shifted_quad_and_its_colours() {
        let mut state = state_of(PrimitiveId::ExpandPulse);
        state.render_flag = 5;
        state.texture = Some("effect\\alpha_center.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 100,
            full_display_angle: 0,
            rise_angle: 0,
            height: {
                let mut height = [0.0; 21];
                height[1] = 1.0;
                height
            },
            distance: 10.0,
            ..Default::default()
        };
        state.slots[0].flag1[0] = 1;
        state.slots[0].flag1[1] = 24;

        let built = triangles(&state);

        assert_eq!(built.len(), 2);
        assert_eq!(built[0].texture, Some("effect\\alpha_center.tga"));
        assert_eq!(built[0].argb, pack_argb(100, 255, 255, 255));
        // `flag1[0] == 1` uses `pw = 0`, i.e. the glow queue; render_flag 5
        // enables the per-call blend.
        assert_eq!(built[0].blend, SlotBlend::AlphaOne);

        // shift = 24 * 1; outer = 34; inner = -14; back = -1. With display and
        // rise both zero the quad is the plain rectangle.
        assert_close(built[0].corners[0][0], 34.0);
        assert_close(built[0].corners[0][1], -1.0);
        assert_close(built[0].corners[1][0], 34.0);
        assert_close(built[0].corners[1][1], 1.0);
        assert_close(built[0].corners[2][0], -14.0);
        assert_close(built[0].corners[2][1], 1.0);
    }

    #[test]
    fn rotated_plate_flag_two_forces_the_near_plane() {
        let mut state = state_of(PrimitiveId::ExpandPulse);
        state.render_flag = 5;
        state.slots[0].life = 1;
        state.slots[0].alpha_b = 80;
        state.slots[0].distance = 10.0;
        state.slots[0].height[1] = 1.0;
        state.slots[0].flag1[0] = 2;
        state.slots[0].flag1[1] = 16;

        let built = triangles(&state);

        assert_eq!(built.len(), 2);
        for triangle in &built {
            assert_eq!(triangle.forced_depth, Some(1.0));
            assert_eq!(triangle.argb, pack_argb(80, 65, 184, 240));
        }
    }

    #[test]
    fn grand_cross2_builds_the_skirt_and_the_flat_cross() {
        let mut state = state_of(PrimitiveId::GrandCross2);
        state.render_flag = 5;
        state.texture = Some("effect\\ring_yellow.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 100,
            rot_start: 0,
            height: {
                let mut height = [0.0; 21];
                height[0] = 24.0;
                height[1] = 3.2;
                height[2] = 5.0;
                height
            },
            ..Default::default()
        };

        let built = triangles(&state);

        // Four skirt quads plus the two flat cross quads.
        assert_eq!(built.len(), 12);
        assert_eq!(built[0].texture, Some("effect\\ring_yellow.tga"));

        // Skirt: (vecBPre, vecBNow, lowered(vecBNow)).
        assert_close(built[0].corners[0][0], 24.0);
        assert_close(built[0].corners[0][1], -1.0);
        assert_close(built[0].corners[0][2], 3.2);
        assert_close(built[0].corners[1][2], -3.2);
        assert_close(built[0].corners[2][1], -6.0);
        assert_eq!(built[0].argb, pack_argb(100, 255, 255, 255));
        assert_eq!(built[0].blend, SlotBlend::Alpha);

        // Last two quads (triangles 8..12) are the flat cross at alpha / 2.
        assert_close(built[8].corners[0][1], -1.0);
        assert_eq!(built[8].argb, pack_argb(50, 255, 255, 255));
        assert_eq!(built[8].blend, SlotBlend::AlphaOne);
    }

    #[test]
    fn grand_cross_2_casting_ring_sweeps_nine_segments_per_slot() {
        let mut state = state_of(PrimitiveId::Gi3);
        state.render_flag = 5;
        state.size = 1.0;
        state.texture = Some("effect\\ring_white.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 10,
            full_display_angle: 90,
            rot_start: 180,
            rise_angle: 90,
            distance: 19.9,
            max_height: 120.0,
            height: [120.0; 21],
            ..Default::default()
        };
        state.slots[0].flag1[0] = 23;
        state.slots[0].flag1[1] = 23;

        let built = triangles(&state);

        // 90 / 9 = 10 degrees per step -> nine sweep positions, the first only
        // seeds the previous vectors, so eight quads = sixteen triangles.
        assert_eq!(built.len(), 16);
        assert_eq!(built[0].texture, Some("effect\\ring_white.tga"));
        // `state.size == 1` -> (255, 175, 175) and the glow destination blend.
        assert_eq!(built[0].argb, pack_argb(10, 255, 175, 175));
        assert_eq!(built[0].blend, SlotBlend::AlphaOne);

        // First emitted quad starts at the order-0 bottom (`angle = 180`)
        // shifted by `flag1[0]`/`flag1[1]`.
        let (cos180, sin180) = trig(180);
        let previous_bottom = [cos180 * 19.9, 0.0, sin180 * 19.9];
        assert_close(built[0].corners[0][0], previous_bottom[0] + 23.0);
        assert_close(built[0].corners[0][1], 0.0);
        assert_close(built[0].corners[0][2], previous_bottom[2] + 23.0);
    }
    #[test]
    fn tube_draws_two_passes_of_ten_segments() {
        let mut state = state_of(PrimitiveId::LineLink);
        state.render_flag = 5;
        state.texture = Some("effect\\alpha_center.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 100,
            max_height: 5.0,
            process: 0,
            vec_t_now: [0.0, 0.0, 0.0],
            vec_t_pre: [10.0, 0.0, 0.0],
            ..Default::default()
        };

        let built = triangles(&state);

        // Two passes x ten swept segments x two triangles.
        assert_eq!(built.len(), 40);

        // Default kind: pass 0 is yellow with `alphaB`, pass 1 blue with
        // `2 * alphaB`; both use the straight alpha blend (`pw = 0`).
        assert_eq!(built[0].argb, pack_argb(100, 255, 255, 0));
        assert_eq!(built[0].blend, SlotBlend::Alpha);
        assert_eq!(built[20].argb, pack_argb(200, 0, 0, 255));
        assert_eq!(built[20].blend, SlotBlend::Alpha);

        // `dirX = -1` -> heading 180, side 270: the +x endpoint's cross-section
        // sweeps `across` along -z. The first emitted quad starts at step 1.
        let (cos36, sin36) = trig(36);
        let (cos270, sin270) = trig(270);
        let across = 5.0 * cos36;
        assert_close(built[0].corners[1][0], across * cos270);
        assert_close(built[0].corners[1][1], 5.0 * sin36);
        assert_close(built[0].corners[1][2], across * sin270);
    }

    #[test]
    fn tube_kind_four_keeps_the_halved_alpha_in_both_passes() {
        let mut state = state_of(PrimitiveId::LineLink);
        state.render_flag = 5;
        state.texture = Some("effect\\alpha_center.tga");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 101,
            max_height: 5.0,
            process: 0,
            vec_t_now: [0.0, 0.0, 0.0],
            vec_t_pre: [10.0, 0.0, 0.0],
            ..Default::default()
        };
        state.slots[0].height[4] = 4.0;

        let built = triangles(&state);

        assert_eq!(built.len(), 40);
        // `alpha = alphaB / 2` for pass 0, and the same variable is reused for
        // pass 1 (the quirk kept from the decompilation).
        assert_eq!(built[0].argb, pack_argb(50, 50, 50, 50));
        assert_eq!(built[0].blend, SlotBlend::AlphaOne);
        assert_eq!(built[20].argb, pack_argb(50, 50, 50, 50));
        assert_eq!(built[20].blend, SlotBlend::AlphaOne);
    }

    #[test]
    fn chemical_draws_the_two_wedges_per_live_slot() {
        let mut state = state_of(PrimitiveId::ChemicalProtection);
        state.render_flag = 5;
        state.texture = Some("effect\\shockwave_c.bmp");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 100,
            rise_angle: 0,
            distance: 10.0,
            ..Default::default()
        };

        let built = triangles(&state);

        // Two wedge quads (two triangles each) for the single live slot.
        assert_eq!(built.len(), 4);

        // Wedge 1 (+/- 3 degrees): alpha is `int(alphaB * 0.5)`, white.
        assert_eq!(built[0].argb, pack_argb(50, 255, 255, 255));
        assert_eq!(built[0].texture_coordinates, [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]);
        let (cos357, sin357) = trig(357);
        let (cos3, sin3) = trig(3);
        assert_close(built[0].corners[1][0], cos357 * 10.0);
        assert_close(built[0].corners[1][2], sin357 * 10.0);
        assert_close(built[0].corners[2][0], cos3 * 10.0);
        assert_close(built[0].corners[2][2], sin3 * 10.0);

        // Wedge 2 (+/- 5 degrees): full alphaB, blue.
        assert_eq!(built[2].argb, pack_argb(100, 130, 130, 255));
        let (cos355, sin355) = trig(355);
        let (cos5, sin5) = trig(5);
        assert_close(built[2].corners[1][0], cos355 * 10.0);
        assert_close(built[2].corners[1][2], sin355 * 10.0);
        assert_close(built[2].corners[2][0], cos5 * 10.0);
        assert_close(built[2].corners[2][2], sin5 * 10.0);
    }

    #[test]
    fn chemical_without_alpha_draws_nothing() {
        let mut state = state_of(PrimitiveId::ChemicalProtection);
        state.render_flag = 5;
        state.texture = Some("effect\\shockwave_c.bmp");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 0,
            distance: 10.0,
            ..Default::default()
        };

        assert!(triangles(&state).is_empty());
    }

    #[test]
    fn magnum_two_sweeps_twenty_one_steps_and_closes_on_rot_start() {
        let mut state = state_of(PrimitiveId::Magnum2);
        state.render_flag = 5;
        state.texture = Some("effect\\ring_yellow.tga");
        state.size = 5.0;
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 120,
            full_display_angle: 360,
            rot_start: 0,
            distance: 5.0,
            max_height: 20.0,
            rise_angle: 70,
            ..Default::default()
        };

        let built = triangles(&state);

        // 21 sweep positions -> 20 quads -> 40 triangles.
        assert_eq!(built.len(), 40);
        // The first quad connects sweep 0 to sweep 18; its first corner is
        // sweep 0's bottom vertex.
        let (cos0, sin0) = trig(0);
        assert_eq!(built[0].corners[0], [5.0 * cos0, -3.0, 5.0 * sin0]);
        // `fullDisplayAngle == 360 && order == 20` re-uses rotStart for the
        // closing step, so the last quad's second corner is the sweep-0 bottom.
        assert_close(built[38].corners[1][0], 5.0 * cos0);
        assert_close(built[38].corners[1][1], -3.0);
        assert_close(built[38].corners[1][2], 5.0 * sin0);
        // size 5 is the white glow default of the 10-argument RenderTeiRect.
        assert_eq!(built[0].argb, pack_argb(120, 255, 255, 255));
        assert_eq!(built[0].blend, SlotBlend::AlphaOne);
        assert_eq!(built[0].texture, Some("effect\\ring_yellow.tga"));
    }

    #[test]
    fn sphere_wind_builds_the_baby_shell() {
        let mut state = state_of(PrimitiveId::SphereWind2);
        state.render_flag = 5;
        state.texture = Some("effect\\fire_ivy.bmp");
        state.size = 10.0;
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 40,
            full_display_angle: 180,
            rot_start: 0,
            distance: 5.0,
            max_height: 2.0,
            rise_angle: 90,
            height: {
                let mut height = [0.0; 21];
                height[0] = -10.0;
                height
            },
            ..Default::default()
        };

        let built = triangles(&state);

        // 21 steps -> 20 quads -> 40 triangles.
        assert_eq!(built.len(), 40);
        // At sweep 0 with riseAngle 90 the bottom is (0, 5, 0) and the top
        // edge is displaced by maxHeight along -x, then the whole pair is
        // lifted by height[0] = -10.
        assert_close(built[0].corners[0][0], -2.0);
        assert_close(built[0].corners[0][1], -5.0);
        assert_close(built[0].corners[0][2], 0.0);
        // The second triangle starts at the quad's fourth corner.
        assert_close(built[1].corners[0][0], 2.0);
        assert_close(built[1].corners[0][1], -5.0);
        assert_eq!(built[0].argb, pack_argb(40, 255, 89, 182));
        assert_eq!(built[0].blend, SlotBlend::AlphaOne);
        assert_eq!(built[0].texture, Some("effect\\fire_ivy.bmp"));
    }

    #[test]
    fn floor_square_picks_the_sonic_frame_and_the_flat_colour() {
        let mut state = state_of(PrimitiveId::SonicWave);
        state.render_flag = 5;
        state.texture_frames = vec![
            "effect\\sonic_wave1.tga",
            "effect\\sonic_wave2.tga",
            "effect\\sonic_wave3.tga",
            "effect\\sonic_wave4.tga",
        ];
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 200,
            process: 7,
            rot_start: 0,
            distance: 20.0,
            flag1: {
                let mut flag1 = [0i8; 21];
                flag1[1] = 1;
                flag1
            },
            ..Default::default()
        };

        let built = triangles(&state);

        assert_eq!(built.len(), 2);
        // process 7 -> 7 / 6 % 4 = frame 1.
        assert_eq!(built[0].texture, Some("effect\\sonic_wave2.tga"));
        // rotStart + 45 -> diamond corner at 45 degrees.
        let (cos45, sin45) = trig(45);
        assert_close(built[0].corners[0][0], 20.0 * cos45);
        assert_close(built[0].corners[0][2], 20.0 * sin45);
        assert_eq!(built[0].argb, pack_argb(200, 205, 205, 255));
        assert_eq!(built[0].blend, SlotBlend::Alpha);

        // `flag1[1] == 2` is green, anything else `200,200,255`.
        state.slots[0].flag1[1] = 2;
        assert_eq!(triangles(&state)[0].argb, pack_argb(200, 0, 255, 0));
        state.slots[0].flag1[1] = 0;
        assert_eq!(triangles(&state)[0].argb, pack_argb(200, 200, 200, 255));
    }

    #[test]
    fn sphere_cage_draws_three_shells_and_switches_the_third_texture() {
        let mut state = state_of(PrimitiveId::Guard);
        state.render_flag = 5;
        state.texture = Some("effect\\a01.bmp");
        state.texture_secondary = Some("effect\\a01.bmp");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 100,
            rot_start: 0,
            distance: 2.0,
            max_height: 10.0,
            flag1: {
                let mut flag1 = [0i8; 21];
                flag1[0] = 1;
                flag1
            },
            ..Default::default()
        };

        let built = triangles(&state);

        // 3 bands * 5 cross-sections * 3 shells = 45 quads = 90 triangles.
        assert_eq!(built.len(), 90);
        assert_eq!(built[0].argb, pack_argb(100, 205, 255, 205));
        assert_eq!(built[0].blend, SlotBlend::AlphaOne);
        assert_eq!(built[0].texture, Some("effect\\a01.bmp"));
        assert_eq!(built[2].argb, pack_argb(100, 155, 255, 155));
        assert_eq!(built[4].argb, pack_argb(100, 225, 255, 225));
        // The third shell passes `pw = 1` (plain alpha) and samples textures[1].
        assert_eq!(built[4].blend, SlotBlend::Alpha);
        assert_eq!(built[4].texture, Some("effect\\a01.bmp"));

        // `flag1[0] == 5` makes the third shell a texture-0 glow instead.
        state.slots[0].flag1[0] = 5;
        let blue = triangles(&state);
        assert_eq!(blue[0].argb, pack_argb(100, 100, 100, 255));
        assert_eq!(blue[4].argb, pack_argb(100, 100, 100, 255));
        assert_eq!(blue[4].blend, SlotBlend::AlphaOne);
    }

    #[test]
    fn bubble_swirl_field_panel_uses_the_render_time_camera_angles() {
        let camera = TestCamera::new();
        let context = SlotRenderContext {
            camera: &camera,
            viewport: WINDOW,
            distance_scale: CEFFECT_PRIM_DISTANCE_SCALE,
            facing: [90, 0],
        };

        let mut state = state_of(PrimitiveId::BubbleSwirl);
        state.render_flag = 5;
        state.texture = Some("effect\\cross_old.bmp");
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 100,
            full_display_angle: 0,
            distance: 10.0,
            flag1: {
                let mut flag1 = [0i8; 21];
                flag1[2] = 7;
                flag1
            },
            ..Default::default()
        };

        // Without a camera the stored (zeroed) `alphaT` angles are used. Case
        // 7 flips the quad, so the first submitted corner is the 90-degree one.
        let stored = triangles(&state);
        let (cos90, sin90) = trig(90);
        assert_close(stored[0].corners[0][0], 10.0 * cos90);
        assert_close(stored[0].corners[0][2], 10.0 * sin90);

        // `FaceCamera` angles from the context turn the quad by 90 degrees
        // around y; case 7 flips the corner order, so the first submitted
        // corner is the rotated 90-degree corner.
        let camera_built = triangles_with_context(&state, Some(&context));
        assert_close(camera_built[0].corners[0][0], -10.0);
        assert_close(camera_built[0].corners[0][2], 0.0);
        assert_eq!(camera_built[0].blend, SlotBlend::AlphaOne);
        assert_eq!(camera_built[0].argb, pack_argb(100, 255, 255, 255));
    }

    #[test]
    fn render_sma3_launches_ef_sma3_on_every_render_frame() {
        let runtime = EffectRuntime::new(200, [0.0, 0.0], 1);
        let mut state = state_of(PrimitiveId::Sma);
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            process: 7,
            vec_b_now: [3.0, 4.0, 5.0],
            flag1: {
                let mut flag1 = [0i8; 21];
                flag1[0] = 1;
                flag1
            },
            ..Default::default()
        };

        // `flag1[0] == 1` (EF_STIN3's `SMA(1)`) has no frame gate: every
        // `Render` call launches one comet.
        render_sma3(&state, &runtime);
        render_sma3(&state, &runtime);

        assert_eq!(runtime.take_effect_launches(), vec![
            PendingEffectLaunch {
                effect: EffectId::Sma3,
                position: [3.0, 4.0, 5.0],
                delta_pos2: None,
            },
            PendingEffectLaunch {
                effect: EffectId::Sma3,
                position: [3.0, 4.0, 5.0],
                delta_pos2: None,
            },
        ]);

        // The launcher itself submits no geometry (`triangles` is empty too).
        assert!(triangles(&state).is_empty());
    }

    #[test]
    fn render_sma3_gates_the_ring_variants_on_every_eighth_frame() {
        let mut state = state_of(PrimitiveId::Sma);
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            process: 7,
            vec_b_now: [1.0, 2.0, 3.0],
            flag1: {
                let mut flag1 = [0i8; 21];
                flag1[0] = 0;
                flag1
            },
            ..Default::default()
        };

        // `flag1[0] == 0` -> `EF_SMA2`, `process % 8 == 0` only.
        let runtime = EffectRuntime::new(100, [0.0, 0.0], 1);
        render_sma3(&state, &runtime);
        assert!(runtime.take_effect_launches().is_empty());

        state.slots[0].process = 8;
        render_sma3(&state, &runtime);
        assert_eq!(runtime.take_effect_launches(), vec![PendingEffectLaunch {
            effect: EffectId::Sma2,
            position: [1.0, 2.0, 3.0],
            // `process % 16 == 0 ? 0 : 1`.
            delta_pos2: Some([8.0, 1.0, 0.0]),
        }]);

        state.slots[0].process = 16;
        render_sma3(&state, &runtime);
        assert_eq!(runtime.take_effect_launches(), vec![PendingEffectLaunch {
            effect: EffectId::Sma2,
            position: [1.0, 2.0, 3.0],
            delta_pos2: Some([16.0, 0.0, 0.0]),
        }]);

        // `flag1[0] == 99` -> `EF_SMA_BLACK2`, same cadence.
        state.slots[0].flag1[0] = 99;
        state.slots[0].process = 24;
        render_sma3(&state, &runtime);
        assert_eq!(runtime.take_effect_launches(), vec![PendingEffectLaunch {
            effect: EffectId::SmaBlack2,
            position: [1.0, 2.0, 3.0],
            delta_pos2: Some([24.0, 1.0, 0.0]),
        }]);
    }

    #[test]
    fn render_sma3_ignores_a_dead_slot() {
        let runtime = EffectRuntime::new(200, [0.0, 0.0], 1);
        let mut state = state_of(PrimitiveId::Sma);
        state.slots[0].life = 0;
        state.slots[0].flag1[0] = 1;

        render_sma3(&state, &runtime);

        assert!(runtime.take_effect_launches().is_empty());
    }

    fn sma2_state() -> EffectPrimitiveState {
        let mut state = state_of(PrimitiveId::Sma2);
        state.texture = Some("effect\\ring_blue.tga");
        state.render_flag = 5;
        state.delta_pos = [10.0, 0.0, 20.0];
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 100,
            full_display_angle: 315,
            distance: 3.4,
            max_height: 13.0,
            rot_start: 0,
            rise_angle: 55,
            height: {
                let mut height = [0.0f32; 21];
                for (index, value) in height.iter_mut().enumerate() {
                    *value = index as f32 * 0.5;
                }
                height
            },
            ..Default::default()
        };
        state
    }

    #[test]
    fn render_sma2_emits_one_quad_per_ring_segment() {
        let state = sma2_state();

        let built = triangles(&state);

        // 22 sample angles (0..=315 step 15); the first has no predecessor, so
        // 21 quads = 42 triangles.
        assert_eq!(built.len(), 42);
        assert_eq!(built[0].texture, Some("effect\\ring_blue.tga"));
        // `tick_alpha` blue branch (`pw = 0` -> `ONE` destination).
        assert_eq!(built[0].blend, SlotBlend::AlphaOne);
        assert_eq!(built[0].argb, pack_argb(100, 90, 90, 255));
        // The texture window runs `0/21 .. 1/21` on the first quad.
        assert_close(built[0].texture_coordinates[0][0], 0.0);
        assert_close(built[0].texture_coordinates[1][0], 1.0 / 21.0);
        // The quad is offset by the effect's `deltaPos`, and the first vertex
        // sits on the ring radius at angle 0: 3.4 * cos(0) + 10 = 13.4.
        let (cos0, _) = trig(0);
        assert_close(built[0].corners[0][0], 10.0 + 3.4 * cos0);
    }

    #[test]
    fn render_sma2_uses_the_grey_branch_for_alpha_t_ninety_nine() {
        let mut state = sma2_state();
        state.slots[0].alpha_t = 99;

        let built = triangles(&state);

        assert_eq!(built[0].blend, SlotBlend::Alpha);
        assert_eq!(built[0].argb, pack_argb(100, 70, 70, 70));
    }

    fn ground_arc_state() -> EffectPrimitiveState {
        let mut state = state_of(PrimitiveId::WaveHeights);
        state.position = [5.0, 0.0, 5.0];
        state.total_texture = 3;
        state.render_flag = 5;
        state.texture = Some("effect\\W_bubble01.tga");
        state.texture_frames = vec!["effect\\W_bubble01.tga", "effect\\W_bubble02.tga", "effect\\W_bubble03.tga"];
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_b: 100,
            full_display_angle: 360,
            max_height: 23.0,
            process: 0,
            rot_start: 180,
            rise_angle: 75,
            distance: 4.3,
            alpha_t: 6,
            height: [23.0; 21],
            ..Default::default()
        };
        state
    }

    #[test]
    fn render_ground_arc_glow_submits_the_base_and_glow_bands() {
        let state = ground_arc_state();

        let built = triangles(&state);

        // 21 sample angles; `i > 0` submits two quads per segment -> 20 * 2 * 2.
        assert_eq!(built.len(), 80);
        // First triangle of the first segment is the base band (`size = 0`).
        assert_eq!(built[0].texture, Some("effect\\W_bubble01.tga"));
        assert_eq!(built[0].blend, SlotBlend::Alpha);
        assert_eq!(built[0].argb, pack_argb(100, 255, 255, 255));
        // The second triangle pair of the same segment is the `alphaT == 6`
        // glow band: `size = 5` (white glow, `alpha = alphaB * 2` clamped to
        // 200).
        assert_eq!(built[2].blend, SlotBlend::AlphaOne);
        assert_eq!(built[2].argb, pack_argb(200, 255, 255, 255));
        // The 10-argument `RenderTeiRect` adds `render.position`, and the
        // radius quirk reads `sin(rotStart) + distance` (rotStart is 180, so
        // the radius is `4.3 + sin(180)`). The first submitted vertex is
        // `prevB` from segment 0, i.e. the ring point at `rotStart + 0`.
        let (cos180, sin180) = trig(180);
        let radius = sin180 + 4.3;
        assert_close(built[0].corners[0][0], 5.0 + radius * cos180);
        // `curB` of the first submitted segment samples angle `rotStart + 18`.
        let (cos198, _) = trig(198);
        assert_close(built[0].corners[1][0], 5.0 + radius * cos198);
    }

    #[test]
    fn render_ground_arc_glow_picks_the_texture_from_the_process_counter() {
        let mut state = ground_arc_state();

        state.slots[0].process = 2;
        assert_eq!(triangles(&state)[0].texture, Some("effect\\W_bubble03.tga"));

        // `process % totalTexture` wraps.
        state.slots[0].process = 3;
        assert_eq!(triangles(&state)[0].texture, Some("effect\\W_bubble01.tga"));
    }
}
