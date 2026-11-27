//! The `.str` draw pass - `CRagEffect::Render` (`RagEffect.cpp:14713-14730`)
//! and `RenderAniClip`/`SetAniClipCorners` (`RagEffect.cpp:14658-14839`).
//!
//! [`super::str_player`] owns the per-key transform state (`ProcessEZ2STR`);
//! this module is the renderer half. It turns the layers `ProcessEZ2STR`
//! marked as drawn this frame into screen-space quads and hands them to the
//! client's [`EffectRenderer`], which is the port's equivalent of the DX9
//! primitive that `RenderAniClip` submits.
//!
//! ## The official geometry
//!
//! 1. `SetAniClipCorners` writes four **independent** vertices from
//!    `ax[0..4]`/`ay[0..4]`. The x and y branches are chosen separately (`ax[0]
//!    < ax[1]`, `ay[1] < ay[2]`) and each writes a coordinate *and* the
//!    matching texture coordinate, so a mirrored key keeps its UVs. The four
//!    vertices are submitted as a `D3DPT_TRIANGLESTRIP` (`RenderPrimitive`'s
//!    default): triangles `{v0, v1, v2}` and `{v1, v2, v3}`.
//! 2. `RenderAniClip` scales every corner by `ScaleDistance(rhw) =
//!    viewport.scaleX * rhw / 7` and rotates it by `rz / 2.8444... / 180 *
//!    3.141592` with the client trig tables. Screen x is `((transform.x - 320)
//!    + rotatedX) * scale + tlvertX`, screen y is `((transform.y - 240) +
//!    rotatedY) * scale + tlvertY`.
//! 3. The texture is `clip.Texture((int)aniframe)`: the float truncates, a
//!    negative index clamps to 0, and an index at or past `textureCount` falls
//!    back to `g_texMgr.Default()`. The port has no default texture, so such a
//!    layer draws nothing.
//! 4. The diffuse is the packed `crA/crR/crG/crB` (each channel truncated to a
//!    byte), and every UV is multiplied by `ImageWidth / TextureWidth`
//!    (`ImageHeight / TextureHeight`) to compensate for texture-atlas padding.
//!    The port uploads textures unpadded, so that ratio is exactly 1.
//!
//! ## Port conventions
//!
//! `EffectRenderer::render_effect` treats its corner argument as a
//! window-pixel offset from the projected anchor: it subtracts
//! [`EFFECT_ORIGIN`] and then re-adds the projected anchor in normalised
//! screen space. Passing `official_corner + EFFECT_ORIGIN` therefore
//! reproduces the official screen coordinate exactly (the same convention
//! [`super::sprite_pass`] uses), which is why [`quad`] already bakes the
//! origin into the corners.
//!
//! `ProcessEZ2STR` also offsets the whole layer in screen space. The offset is
//! per-effect runtime state ([`super::runtime::EffectRuntime::str_offset`]):
//! most bodies call `ProcessEZ2STR(0, -80, 0)` (the runtime default,
//! [`super::runtime::DEFAULT_STR_OFFSET`]), ShieldCharge's dispatch calls
//! `ProcessEZ2STRC65300` = `(0, 0, 0)` (`RagEffect.cpp:1884-1885`), Devotion
//! calls `(0, -20, 0)` (`RagEffect.cpp:24604-24606`), and the holder passes
//! the current value through [`screen_offset`].
//!
//! `layer 0` is **not** drawn: `CRagEffect::Render` starts its loop at layer 1
//! (`RagEffect.cpp:14724`), exactly like `CEZ2STRParticle::DrawScreen`
//! (`Particle.cpp:472`). Clip 0 is the resource's control layer that drives
//! the timeline.
//!
//! ## Blend pair
//!
//! `RenderAniClip` submits `primitive->sourceBlend = transform->srcalpha` and
//! `primitive->destinationBlend = transform->destalpha` (`RagEffect.cpp:
//! 14799-14801`), i.e. the **per-keyframe** DX9 `D3DBLEND` codes the `.str`
//! file stores. The port carries them through [`super::str_player::Transform`]
//! and [`blend_factors`] translates them to wgpu factors; the forward effect
//! pass then applies its premultiplied-alpha translation
//! (`graphics/passes/forward/effect.rs`). The dominant official `.str` pair is
//! `SRCALPHA`/`DESTALPHA` (`5`/`7`) - 101,315 key frames in 1,926 of the
//! 2,063 archived `.str` files, including every key frame of
//! `Concentration.str` - so the previous hard-coded straight-alpha pair
//! (`5`/`6`) drew those additive layers with `OneMinusSrcAlpha` and turned the
//! black background of the 24-bit `.bmp` effect textures into an opaque black
//! square.
//!
//! ## Remaining gap
//!
//! `RenderAniClip` also uses `mtpreset` as the multi-texture mode. The port
//! carries only the first texture stage, which is exact for every key frame in
//! the ported effect table (`mtpreset == 0`); a key frame with a non-zero mode
//! would need the second stage threaded through
//! [`super::str_player::Transform`] as well.

use cgmath::{Rad, Vector2};
use wgpu::BlendFactor;

use super::super::EFFECT_ORIGIN;
use super::angles::{client_cos, client_sin};
use super::str_player::{Clip, Ez2StrPlayer, Transform};
use crate::graphics::Color;
use crate::renderer::EffectRenderer;
use crate::world::Camera;

/// The x/y half of the runtime's [`super::runtime::EffectRuntime::str_offset`]
/// in the vector type [`quad`] consumes. `ProcessEZ2STR` also adds its z to
/// `state.tlvertZ` (`RagEffect.cpp:1420`), but `RenderAniClip` projects the
/// screen quad from `tlvertX/Y` only, so z has no effect on this pass.
pub fn screen_offset(offset: [f32; 3]) -> Vector2<f32> {
    Vector2::new(offset[0], offset[1])
}

/// `RenderAniClip`'s fixed 640x480 reference-screen origin: the keyframe's
/// `x`/`y` are absolute coordinates in that space, so the local offset is
/// `x - 320` / `y - 240`.
const REFERENCE_SCREEN_CENTER: Vector2<f32> = Vector2::new(320.0, 240.0);

/// `2.8444444444444`: the `.str` angle unit is 1024 per turn, i.e.
/// `1024 / 360` degrees per unit (`RenderAniClip` divides by it before
/// converting to radians).
const ANGLE_UNITS_PER_DEGREE: f64 = 2.844_444_444_444_4;

/// One `RenderVertex` of `SetAniClipCorners`.
///
/// The x branch writes `x`/`u`, the (independent) y branch writes `y`/`v`, so
/// this mirrors the official vertex exactly instead of deriving the corners
/// from `ax`/`ay` pairs.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AniClipCorner {
    pub x: f32,
    pub y: f32,
    pub u: f32,
    pub v: f32,
}

/// The four corners/UVs of one `.str` keyframe, already in the layout
/// [`EffectRenderer::render_effect`] expects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrQuad {
    /// Window-pixel corners (plus [`EFFECT_ORIGIN`], see the module docs) in
    /// the order top left, top right, bottom left, bottom right.
    pub corners: [Vector2<f32>; 4],
    /// UVs in the order the effect renderer samples:
    /// bottom right, top right, top left, bottom left.
    pub uvs: [Vector2<f32>; 4],
}

/// One `.str` layer that `CRagEffect::Render` draws on this frame.
#[derive(Debug, Clone, Copy)]
pub struct VisibleLayer<'a> {
    /// Index into the resource's clip list; always `>= 1`.
    pub layer: usize,
    pub clip: &'a Clip,
    pub transform: &'a Transform,
    /// Resolved texture path of the current animation frame.
    pub texture: &'a str,
}

/// `SetAniClipCorners` - `RagEffect.cpp:14658-14695`.
///
/// The two `if`s are independent on purpose: a keyframe may be mirrored on one
/// axis only, and each branch moves the UV with its coordinate. `vertices[1].v`
/// comes from `ay[0]` and `vertices[0].v` from `ay[3]` in the unmirrored
/// branch; that is the original assignment, not a transcription slip (compare
/// the equivalent `CEZ2STRParticle::SetClipCorners` in `Particle.cpp:36-67`).
pub fn ani_clip_corners(transform: &Transform) -> [AniClipCorner; 4] {
    let mut vertices = [AniClipCorner::default(); 4];

    if transform.ax[0] < transform.ax[1] {
        vertices[1].x = transform.ax[0];
        vertices[1].u = transform.u;
        vertices[3].x = transform.ax[1];
        vertices[3].u = transform.u + transform.us;
        vertices[0].x = transform.ax[3];
        vertices[0].u = transform.u;
        vertices[2].x = transform.ax[2];
        vertices[2].u = transform.u + transform.us;
    } else {
        vertices[1].x = transform.ax[1];
        vertices[1].u = transform.u + transform.us;
        vertices[3].x = transform.ax[0];
        vertices[3].u = transform.u;
        vertices[0].x = transform.ax[2];
        vertices[0].u = transform.u + transform.us;
        vertices[2].x = transform.ax[3];
        vertices[2].u = transform.u;
    }

    if transform.ay[1] < transform.ay[2] {
        vertices[3].y = transform.ay[1];
        vertices[3].v = transform.v;
        vertices[2].y = transform.ay[2];
        vertices[2].v = transform.v + transform.vs;
        vertices[1].y = transform.ay[0];
        vertices[1].v = transform.v;
        vertices[0].y = transform.ay[3];
        vertices[0].v = transform.v + transform.vs;
    } else {
        vertices[3].y = transform.ay[2];
        vertices[3].v = transform.v + transform.vs;
        vertices[2].y = transform.ay[1];
        vertices[2].v = transform.v;
        vertices[1].y = transform.ay[3];
        vertices[1].v = transform.v + transform.vs;
        vertices[0].y = transform.ay[0];
        vertices[0].v = transform.v;
    }

    vertices
}

/// `ClientSin(ToRadians(rz / 2.8444444444444))` / the cosine of
/// `RenderAniClip` (`RagEffect.cpp:14756-14757`).
pub fn rotation(transform: &Transform) -> (f32, f32) {
    // The original casts the double division back to `float` before the
    // degree-to-radian conversion, and uses the `3.141592` literal.
    let degrees = (transform.rz as f64 / ANGLE_UNITS_PER_DEGREE) as f32;
    let radians = degrees / 180.0 * 3.141_592;
    (client_sin(radians), client_cos(radians))
}

/// `RenderAniClip`'s per-vertex screen transform.
///
/// `scale` is `ScaleDistance(rhw)` (window pixels per `.str` unit);
/// `screen_offset` is the `ProcessEZ2STR` argument applied after scaling.
/// The returned corners already include [`EFFECT_ORIGIN`] so that
/// [`EffectRenderer::render_effect`] lands them on the official pixel.
pub fn quad(transform: &Transform, scale: f32, screen_offset: Vector2<f32>) -> StrQuad {
    let vertices = ani_clip_corners(transform);
    let (sine, cosine) = rotation(transform);

    let corners = vertices.map(|vertex| {
        let rotated_x = vertex.x * cosine - vertex.y * sine;
        let rotated_y = vertex.x * sine + vertex.y * cosine;

        Vector2::new(
            ((transform.x - REFERENCE_SCREEN_CENTER.x) + rotated_x) * scale + screen_offset.x + EFFECT_ORIGIN.x,
            ((transform.y - REFERENCE_SCREEN_CENTER.y) + rotated_y) * scale + screen_offset.y + EFFECT_ORIGIN.y,
        )
    });

    // The renderer pairs `corners[0]` with `uvs[2]`, `[1]` with `[1]`, `[2]`
    // with `[3]` and `[3]` with `[0]`; this order keeps every official vertex
    // with its own UV while preserving the strip's triangle set.
    let uvs = [
        Vector2::new(vertices[3].u, vertices[3].v),
        Vector2::new(vertices[1].u, vertices[1].v),
        Vector2::new(vertices[0].u, vertices[0].v),
        Vector2::new(vertices[2].u, vertices[2].v),
    ];

    StrQuad { corners, uvs }
}

/// `const int frame = (int)transform->aniframe;` with the
/// `frame < 0 ? 0 : frame` clamp of `EZeffectClip::Texture`.
///
/// `None` means the original would use `g_texMgr.Default()` (the index is at or
/// past `textureCount`); the port draws nothing for it.
pub fn texture_index(clip: &Clip, transform: &Transform) -> Option<usize> {
    let frame = transform.aniframe as i32;

    if frame < 0 {
        return (clip.texture_count > 0).then_some(0);
    }

    let frame = frame as u32;

    (frame < clip.texture_count).then_some(frame as usize)
}

/// `EZeffectClip::Texture` for one layer: the animation frame resolved through
/// the clip's texture list.
///
/// `textures` is the per-clip list the loader built (`effect\...`, with the
/// same directory prefix rule as `loaders/effect/mod.rs`). A missing or empty
/// entry is the original's default texture, which the port does not draw.
pub fn texture_for<'a>(clip: &Clip, textures: &'a [String], transform: &Transform) -> Option<&'a str> {
    let index = texture_index(clip, transform)?;
    let texture = textures.get(index)?;

    (!texture.is_empty()).then_some(texture.as_str())
}

/// `CRagEffect::Render` - the layers this frame's `ProcessEZ2STR` marked as
/// drawn.
///
/// The loop starts at layer 1 and skips clip 0 (the control layer), and it
/// only draws a layer whose `layerDrawn` flag is set. A layer without a
/// resolvable texture is skipped, like a failed `g_texMgr.Load`.
pub fn visible_layers<'a>(player: Option<&'a Ez2StrPlayer>, clips: &'a [Clip], textures: &'a [Vec<String>]) -> Vec<VisibleLayer<'a>> {
    let Some(player) = player else {
        return Vec::new();
    };

    let mut visible = Vec::new();

    // `for (int layer = 1; layer < clipCount; ++layer)`.
    for layer in 1..clips.len() {
        if !player.layer_drawn.get(layer).copied().unwrap_or(false) {
            continue;
        }

        let clip = &clips[layer];
        let transform = &player.transforms[layer];
        let layer_textures = textures.get(layer).map_or(&[][..], Vec::as_slice);

        let Some(texture) = texture_for(clip, layer_textures, transform) else {
            continue;
        };

        visible.push(VisibleLayer {
            layer,
            clip,
            transform,
            texture,
        });
    }

    visible
}

/// `(((crA << 8) + red) << 8 + green) << 8 + blue`, with the red/green/blue
/// channels truncated to a byte and only the low byte of `crA` surviving the
/// shift.
pub fn keyframe_color(transform: &Transform) -> Color {
    let channel = |value: f32| (value as i32 as u8) as f32 / 255.0;

    Color::rgba(
        channel(transform.cr_r),
        channel(transform.cr_g),
        channel(transform.cr_b),
        channel(transform.cr_a),
    )
}

/// One DX9 `D3DBLEND` code as the equivalent wgpu [`BlendFactor`].
///
/// `source` says which of the two `SetRenderState` calls the code came from
/// (`D3DRS_SRCBLEND` vs `D3DRS_DESTBLEND`); the two `BOTH*` shorthands select a
/// different factor for each stage.
///
/// `None` means the code is not a valid `D3DBLEND` value; the official client
/// would hand it to `IDirect3DDevice9::SetRenderState` unchanged.
fn d3d_blend_factor(code: i32, source: bool) -> Option<BlendFactor> {
    let factor = match code {
        1 => BlendFactor::Zero,
        2 => BlendFactor::One,
        // `D3DBLEND_SRCCOLOR` / `D3DBLEND_INVSRCCOLOR`.
        3 => BlendFactor::Src,
        4 => BlendFactor::OneMinusSrc,
        5 => BlendFactor::SrcAlpha,
        6 => BlendFactor::OneMinusSrcAlpha,
        7 => BlendFactor::DstAlpha,
        8 => BlendFactor::OneMinusDstAlpha,
        // `D3DBLEND_DESTCOLOR` / `D3DBLEND_INVDESTCOLOR`.
        9 => BlendFactor::Dst,
        10 => BlendFactor::OneMinusDst,
        11 => BlendFactor::SrcAlphaSaturated,
        // `D3DBLEND_BOTHSRCALPHA`: the obsolete shorthand for
        // `SRCBLEND = SRCALPHA`, `DESTBLEND = INVSRCALPHA`.
        12 if source => BlendFactor::SrcAlpha,
        12 => BlendFactor::OneMinusSrcAlpha,
        // `D3DBLEND_BOTHINVSRCALPHA`: the inverse shorthand.
        13 if source => BlendFactor::OneMinusSrcAlpha,
        13 => BlendFactor::SrcAlpha,
        _ => return None,
    };

    Some(factor)
}

/// `RenderAniClip`'s blend pair for one key frame
/// (`RagEffect.cpp:14799-14801`).
///
/// The factors are the `.str` file's `srcalpha`/`destalpha` DX9 codes, already
/// translated to the straight-alpha factors the effect renderer expects (the
/// forward effect pass turns a `SrcAlpha` source factor into `One` because
/// effect textures and colours are premultiplied). `Concentration.str` uses
/// `5`/`7`, so this returns
/// [`BlendFactor::SrcAlpha`]/[`BlendFactor::DstAlpha`], which the pass draws as
/// `One`/`DstAlpha` - the preloaded `.str` additive pair.
///
/// An unset/invalid pair falls back to the straight-alpha pair of the
/// `RSF_ALPHA` queue. Every absolute key frame in the ported resources has a
/// valid pair; the fallback only covers synthetic/default transforms, where the
/// original would pass the invalid code to D3D.
pub fn blend_factors(transform: &Transform) -> (BlendFactor, BlendFactor) {
    match (
        d3d_blend_factor(transform.source_blend_factor, true),
        d3d_blend_factor(transform.destination_blend_factor, false),
    ) {
        (Some(source), Some(destination)) => (source, destination),
        _ => (BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha),
    }
}

/// The full `.str` pass the holder calls after its primitives.
#[allow(clippy::too_many_arguments)]
pub fn render(
    renderer: &mut EffectRenderer,
    camera: &dyn Camera,
    textures: &crate::loaders::TextureLoader,
    anchor: cgmath::Point3<f32>,
    player: Option<&Ez2StrPlayer>,
    clips: &[Clip],
    layer_textures: &[Vec<String>],
    screen_offset: Vector2<f32>,
) {
    let visible = visible_layers(player, clips, layer_textures);

    // Most ported effects have no `.str` resource or nothing drawn this frame;
    // do not project the anchor twice for them.
    if visible.is_empty() {
        return;
    }

    let Some(scale) = super::sprite_pass::distance_scale(camera, anchor, renderer.window_size()) else {
        return;
    };

    for layer in visible {
        let Ok(texture) = textures.get_or_load(layer.texture, crate::loaders::ImageType::Color) else {
            continue;
        };

        let quad = quad(layer.transform, scale, screen_offset);
        let (source_blend_factor, destination_blend_factor) = blend_factors(layer.transform);

        renderer.render_effect(
            camera,
            anchor,
            texture,
            quad.corners,
            quad.uvs,
            Vector2::new(0.0, 0.0),
            // The rotation is already baked into `quad` with the official
            // cosine/sine; `render_effect` must not rotate a second time.
            Rad(0.0),
            keyframe_color(layer.transform),
            source_blend_factor,
            destination_blend_factor,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::super::str_player::KeyFrame;
    use super::*;

    fn key(transform: Transform) -> KeyFrame {
        KeyFrame { frame: 0, transform }
    }

    fn clip(texture_count: u32, frames: Vec<KeyFrame>) -> Clip {
        Clip { texture_count, frames }
    }

    /// A clean 20x40 rectangle with distinct UVs: `ax` is left/right, `ay` is
    /// top/bottom.
    fn rectangle() -> Transform {
        Transform {
            x: 320.0,
            y: 240.0,
            u: 0.25,
            v: 0.5,
            us: 0.125,
            vs: 0.25,
            ax: [-10.0, 10.0, 10.0, -10.0],
            ay: [-20.0, -20.0, 20.0, 20.0],
            ..Default::default()
        }
    }

    #[test]
    fn corners_follow_the_official_x_and_y_branches() {
        let corners = ani_clip_corners(&rectangle());

        // Unmirrored rectangle: v1 = top left, v3 = top right, v0 = bottom
        // left, v2 = bottom right, and the UV travels with each vertex.
        assert_eq!(corners[1], AniClipCorner {
            x: -10.0,
            y: -20.0,
            u: 0.25,
            v: 0.5,
        });
        assert_eq!(corners[3], AniClipCorner {
            x: 10.0,
            y: -20.0,
            u: 0.375,
            v: 0.5,
        });
        assert_eq!(corners[0], AniClipCorner {
            x: -10.0,
            y: 20.0,
            u: 0.25,
            v: 0.75,
        });
        assert_eq!(corners[2], AniClipCorner {
            x: 10.0,
            y: 20.0,
            u: 0.375,
            v: 0.75,
        });
    }

    #[test]
    fn mirrored_axis_swaps_the_corner_and_its_uv() {
        let mut transform = rectangle();
        // Mirror on x only: `ax[0] >= ax[1]` now, while the y branch is
        // unchanged. The x branch swaps the left/right assignment and the u
        // that goes with it.
        transform.ax = [10.0, -10.0, -10.0, 10.0];

        let corners = ani_clip_corners(&transform);

        assert_eq!(corners[1].x, -10.0);
        assert_eq!(corners[1].u, 0.375, "u+us follows the right edge");
        assert_eq!(corners[3].x, 10.0);
        assert_eq!(corners[3].u, 0.25);
        assert_eq!(corners[1].y, -20.0, "the y branch is untouched");
        assert_eq!(corners[1].v, 0.5);
    }

    #[test]
    fn quad_translates_against_the_640x480_reference_screen() {
        let mut transform = rectangle();
        transform.x = 325.0;
        transform.y = 247.0;

        let quad = quad(&transform, 1.0, Vector2::new(0.0, 0.0));

        // Local offset = transform - (320, 240) + corner, in reference pixels;
        // the quad then carries EFFECT_ORIGIN so the renderer cancels it.
        let local_x = |x: f32| 325.0 - 320.0 + x;
        let local_y = |y: f32| 247.0 - 240.0 + y;

        // v0 = bottom left, v1 = top left, v2 = bottom right, v3 = top right.
        assert_eq!(
            quad.corners[0],
            Vector2::new(local_x(-10.0) + EFFECT_ORIGIN.x, local_y(20.0) + EFFECT_ORIGIN.y)
        );
        assert_eq!(
            quad.corners[1],
            Vector2::new(local_x(-10.0) + EFFECT_ORIGIN.x, local_y(-20.0) + EFFECT_ORIGIN.y)
        );
        assert_eq!(
            quad.corners[2],
            Vector2::new(local_x(10.0) + EFFECT_ORIGIN.x, local_y(20.0) + EFFECT_ORIGIN.y)
        );
        assert_eq!(
            quad.corners[3],
            Vector2::new(local_x(10.0) + EFFECT_ORIGIN.x, local_y(-20.0) + EFFECT_ORIGIN.y)
        );

        // Renderer layout: bottom right, top right, top left, bottom left.
        assert_eq!(quad.uvs[0], Vector2::new(0.375, 0.5));
        assert_eq!(quad.uvs[1], Vector2::new(0.25, 0.5));
        assert_eq!(quad.uvs[2], Vector2::new(0.25, 0.75));
        assert_eq!(quad.uvs[3], Vector2::new(0.375, 0.75));
    }

    #[test]
    fn scale_and_the_process_ez2str_offset_are_applied() {
        let mut transform = rectangle();
        transform.x = 330.0;
        transform.y = 250.0;

        let quad = quad(&transform, 2.0, screen_offset(super::super::runtime::DEFAULT_STR_OFFSET));

        // v0 = bottom left: x = ((330 - 320) + -10) * 2 + 0 + 319 = 319,
        // y = ((250 - 240) + 20) * 2 - 80 + 291 = 271.
        // v3 = top right: x = ((330 - 320) + 10) * 2 + 0 + 319 = 359,
        // y = ((250 - 240) + -20) * 2 - 80 + 291 = 191.
        assert_eq!(quad.corners[0], Vector2::new(319.0, 271.0));
        assert_eq!(quad.corners[3], Vector2::new(359.0, 191.0), "scale doubles the 20x40 rectangle");
    }

    #[test]
    fn rz_rotates_with_the_official_angle_unit() {
        let mut transform = rectangle();
        // 1024 units per turn: 256 units == 90 degrees. `rz / 2.8444... == 90`.
        transform.rz = 256.0;

        let (sine, cosine) = rotation(&transform);

        assert!((sine - 1.0).abs() < 1e-5, "sin(90 deg) = {sine}");
        assert!(cosine.abs() < 1e-5, "cos(90 deg) = {cosine}");

        let quad = quad(&transform, 1.0, Vector2::new(0.0, 0.0));

        // v1 = (-10, -20) rotated by 90 degrees: rx = -(-20) = 20,
        // rotated y = -10.
        assert!((quad.corners[1].x - (20.0 + EFFECT_ORIGIN.x)).abs() < 1e-4);
        assert!((quad.corners[1].y - (-10.0 + EFFECT_ORIGIN.y)).abs() < 1e-4);
    }

    #[test]
    fn colour_truncates_each_channel_to_a_byte() {
        let transform = Transform {
            cr_r: 10.9,
            cr_g: 20.4,
            cr_b: 30.0,
            cr_a: 40.7,
            ..Default::default()
        };

        let color = keyframe_color(&transform);

        assert!((color.red - 10.0 / 255.0).abs() < 1e-6);
        assert!((color.green - 20.0 / 255.0).abs() < 1e-6);
        assert!((color.blue - 30.0 / 255.0).abs() < 1e-6);
        assert!((color.alpha - 40.0 / 255.0).abs() < 1e-6);
    }

    #[test]
    fn colour_wraps_negative_and_oversized_channels_like_the_c_cast() {
        let transform = Transform {
            cr_r: -1.0,
            cr_g: 300.0,
            cr_b: 0.0,
            cr_a: -2.0,
            ..Default::default()
        };

        let color = keyframe_color(&transform);

        assert!((color.red - 255.0 / 255.0).abs() < 1e-6);
        assert!((color.green - 44.0 / 255.0).abs() < 1e-6, "300 as u8 is 44");
        assert!((color.alpha - 254.0 / 255.0).abs() < 1e-6, "-2 as i32 as u8 is 254");
    }

    #[test]
    fn concentration_uses_the_official_src_alpha_dest_alpha_pair() {
        // Every absolute key frame of `Concentration.str` (data.grf, version
        // 148) stores `srcalpha = 5`, `destalpha = 7`. `RenderAniClip` passes
        // them through unchanged; the forward effect pass turns `SrcAlpha`
        // into `One`, so the GPU state is the additive `(One, DstAlpha)` pair.
        let transform = Transform {
            source_blend_factor: 5,
            destination_blend_factor: 7,
            ..Default::default()
        };

        assert_eq!(
            blend_factors(&transform),
            (BlendFactor::SrcAlpha, BlendFactor::DstAlpha),
            "the black-backed pokjuk/lens BMPs must be added, not alpha-blended"
        );
    }

    #[test]
    fn straight_alpha_control_stays_straight_alpha() {
        // `Magnificat.str` has 164 key frames with `(5, 6)`; those must keep
        // the `RSF_ALPHA` pair instead of being swept into the additive fix.
        let transform = Transform {
            source_blend_factor: 5,
            destination_blend_factor: 6,
            ..Default::default()
        };

        assert_eq!(
            blend_factors(&transform),
            (BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha)
        );
    }

    #[test]
    fn every_d3d_blend_code_maps_to_its_wgpu_factor() {
        // Stage-independent codes.
        for code in 1..=11 {
            let (source, destination) = (d3d_blend_factor(code, true), d3d_blend_factor(code, false));
            assert_eq!(source, destination, "code {code} is the same in both stages");
            assert!(source.is_some(), "code {code} is a valid D3DBLEND");
        }

        assert_eq!(d3d_blend_factor(1, true), Some(BlendFactor::Zero));
        assert_eq!(d3d_blend_factor(2, true), Some(BlendFactor::One));
        assert_eq!(d3d_blend_factor(3, true), Some(BlendFactor::Src));
        assert_eq!(d3d_blend_factor(4, true), Some(BlendFactor::OneMinusSrc));
        assert_eq!(d3d_blend_factor(5, true), Some(BlendFactor::SrcAlpha));
        assert_eq!(d3d_blend_factor(6, true), Some(BlendFactor::OneMinusSrcAlpha));
        assert_eq!(d3d_blend_factor(7, true), Some(BlendFactor::DstAlpha));
        assert_eq!(d3d_blend_factor(8, true), Some(BlendFactor::OneMinusDstAlpha));
        assert_eq!(d3d_blend_factor(9, true), Some(BlendFactor::Dst));
        assert_eq!(d3d_blend_factor(10, true), Some(BlendFactor::OneMinusDst));
        assert_eq!(d3d_blend_factor(11, true), Some(BlendFactor::SrcAlphaSaturated));

        // `_devil.str` stores `(12, 12)`. `D3DBLEND_BOTHSRCALPHA` means
        // `SRCBLEND = SRCALPHA`, `DESTBLEND = INVSRCALPHA`, so the same file
        // value resolves differently per stage: the old stage-blind mapping
        // drew the destination as `SrcAlpha`.
        assert_eq!(d3d_blend_factor(12, true), Some(BlendFactor::SrcAlpha));
        assert_eq!(d3d_blend_factor(12, false), Some(BlendFactor::OneMinusSrcAlpha));
        // `D3DBLEND_BOTHINVSRCALPHA` is the mirror image.
        assert_eq!(d3d_blend_factor(13, true), Some(BlendFactor::OneMinusSrcAlpha));
        assert_eq!(d3d_blend_factor(13, false), Some(BlendFactor::SrcAlpha));

        assert_eq!(d3d_blend_factor(0, true), None);
        assert_eq!(d3d_blend_factor(0, false), None);
        assert_eq!(d3d_blend_factor(14, true), None);
        assert_eq!(d3d_blend_factor(14, false), None);
    }

    #[test]
    fn both_alpha_shorthand_resolves_per_stage() {
        // `_devil.str`/`devil.str` are the only archived `.str` files that use
        // the obsolete shorthand; both write `(12, 12)`. The official client
        // gets `(SRCALPHA, INVSRCALPHA)` from D3D, which the effect pass then
        // draws as the preloaded `(One, OneMinusSrcAlpha)` pair.
        let transform = Transform {
            source_blend_factor: 12,
            destination_blend_factor: 12,
            ..Default::default()
        };

        assert_eq!(
            blend_factors(&transform),
            (BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha)
        );
    }

    #[test]
    fn unset_blend_codes_fall_back_to_straight_alpha() {
        // A default/incomplete transform carries no D3D codes; the official
        // client would submit the invalid value, so the port keeps its old
        // straight-alpha pair for this synthetic case only.
        assert_eq!(
            blend_factors(&Transform::default()),
            (BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha)
        );
    }

    #[test]
    fn texture_index_truncates_and_falls_back_past_the_texture_count() {
        let mut transform = Transform::default();
        let three_textures = clip(3, Vec::new());

        transform.aniframe = 1.9;
        assert_eq!(texture_index(&three_textures, &transform), Some(1));

        transform.aniframe = -0.5;
        assert_eq!(texture_index(&three_textures, &transform), Some(0), "negative clamps to 0");

        transform.aniframe = 3.0;
        assert_eq!(texture_index(&three_textures, &transform), None, "g_texMgr.Default()");

        let empty = clip(0, Vec::new());
        transform.aniframe = 0.0;
        assert_eq!(texture_index(&empty, &transform), None);
    }

    #[test]
    fn texture_list_resolves_the_animated_frame() {
        let clip = clip(2, Vec::new());
        let textures = vec!["effect\\first.bmp".to_owned(), "effect\\second.bmp".to_owned()];

        let mut transform = Transform::default();
        transform.aniframe = 0.0;
        assert_eq!(texture_for(&clip, &textures, &transform), Some("effect\\first.bmp"));

        transform.aniframe = 1.0;
        assert_eq!(texture_for(&clip, &textures, &transform), Some("effect\\second.bmp"));

        transform.aniframe = 2.0;
        assert_eq!(texture_for(&clip, &textures, &transform), None);
    }

    #[test]
    fn animation_frames_are_selected_over_time() {
        // `anitype 5` ping-pongs `aniframe` over `textureCount - 1` (the
        // player's verified `apply_delta` branch); the renderer only truncates
        // the float and resolves it through the clip's texture list.
        let clips = vec![clip(3, vec![key(Transform {
            kind: 1,
            anitype: 5,
            anidelta: 1.0,
            ..Default::default()
        })])];
        let textures = vec![
            vec![
                "effect\\c0.bmp".to_owned(),
                "effect\\c1.bmp".to_owned(),
                "effect\\c2.bmp".to_owned(),
            ],
            vec![],
            vec![],
        ];

        let mut player = Ez2StrPlayer::new(&clips);
        let mut seen = Vec::new();

        for _ in 0..6 {
            player.process(&clips);
            seen.push(texture_for(&clips[0], &textures[0], &player.transforms[0]).map(str::to_owned));
            player.state_cnt += 1;
        }

        // The official `anitype 5` arm is
        // `frame = (int)(aniframe + anidelta * elapsed)`, folded over
        // `textureCount - 1 == 2` - the already-advanced `aniframe` is part of
        // the sum, so the result is 0, 1, 1, 0, 0, 1 rather than a clean
        // triangle wave. The player is the source of truth here; this test
        // pins the renderer's texture selection to it.
        assert_eq!(seen, vec![
            Some("effect\\c0.bmp".to_owned()),
            Some("effect\\c1.bmp".to_owned()),
            Some("effect\\c1.bmp".to_owned()),
            Some("effect\\c0.bmp".to_owned()),
            Some("effect\\c0.bmp".to_owned()),
            Some("effect\\c1.bmp".to_owned()),
        ]);
    }

    #[test]
    fn no_player_draws_nothing() {
        let clips = vec![clip(1, vec![key(Transform::default())])];

        assert!(visible_layers(None, &clips, &[]).is_empty());
    }

    #[test]
    fn a_layer_that_was_not_drawn_is_skipped() {
        let clips = vec![clip(1, vec![key(Transform::default())]), clip(1, vec![key(Transform::default())])];
        let player = Ez2StrPlayer::new(&clips);

        // `InitEZ2STRFrame` leaves every `layerDrawn` false until the first
        // `ProcessEZ2STR`.
        assert!(visible_layers(Some(&player), &clips, &[]).is_empty());
    }

    #[test]
    fn only_layers_from_one_up_are_drawn() {
        let clips = vec![
            clip(1, vec![key(Transform::default())]),
            clip(1, vec![key(Transform::default())]),
            clip(1, vec![key(Transform::default())]),
        ];
        let mut player = Ez2StrPlayer::new(&clips);

        player.layer_drawn = vec![true, true, true];
        player.transforms[0].aniframe = 0.0;
        player.transforms[1].aniframe = 0.0;
        player.transforms[2].aniframe = 0.0;

        let textures = vec![vec!["effect\\zero.bmp".to_owned()], vec!["effect\\one.bmp".to_owned()], vec![
            "effect\\two.bmp".to_owned(),
        ]];

        let visible = visible_layers(Some(&player), &clips, &textures);

        assert_eq!(visible.iter().map(|layer| layer.layer).collect::<Vec<_>>(), vec![1, 2]);
        assert_eq!(visible[0].texture, "effect\\one.bmp");
        assert_eq!(visible[1].texture, "effect\\two.bmp");
    }

    #[test]
    fn a_layer_without_a_texture_is_skipped() {
        let clips = vec![clip(1, vec![key(Transform::default())]), clip(1, vec![key(Transform::default())])];
        let mut player = Ez2StrPlayer::new(&clips);
        player.layer_drawn = vec![true, true];

        // No list at all, and then a list with an empty name.
        assert!(visible_layers(Some(&player), &clips, &[]).is_empty());
        assert!(visible_layers(Some(&player), &clips, &[vec![], vec![String::new()]]).is_empty());
    }
}
