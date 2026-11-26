//! Actor render phases written by ported effect bodies.
//!
//! `world::effect::port` records the official effect bodies' direct writes to
//! the caster's `CRenderObject::RenderObjectState` bytes and replays them onto
//! [`crate::world::Entity`] (`render.tintPhase`, `render.shrinkPhase`,
//! `render.outlineAlpha`, ...; see `port/runtime.rs` and `port/visual.rs`).
//! This module is the client-side consumer: the pure maths of the decompiled
//! render passes, separated from the sprite draw calls so it stays testable
//! without a GPU.
//!
//! Every formula below is a transliteration of the decompilation:
//!
//! * [`draw_tinted_quad`] - `CRenderObject::DrawTintedQuad`,
//!   `RenderObject.cpp:826-846` (called from `DrawStatusOverlays`,
//!   `RenderObject.cpp:720-745`, and `GameObject.cpp:400-428`).
//! * [`tint_rise_units`] / [`tint_rise_alpha`] - the flag-`0x20000` rise,
//!   `Player.cpp:1206-1218`.
//! * [`pulse_scale`] - `CRenderObject::ScaleByPulse`,
//!   `RenderObject.cpp:749-767` (also `ScaleByFadePulse`,
//!   `RenderObject.cpp:3009-3017`, and `CGameActor::GetNameOffset`,
//!   `GameActor.cpp:932-940`, which use the same ramp).
//! * [`outline_alpha`] - the outline copy color, `Player.cpp:1139-1190`.

use crate::graphics::Color;

/// Fixed quad colour of the official flag-`0x4000` tinted body
/// (`CRenderObject::DrawStatusOverlays` calls `DrawTintedQuad(quad, 5, 5, 255,
/// palette)`; `RenderObject.cpp:739`).
pub const TINT_BLUE: [u8; 3] = [5, 5, 255];

/// Fixed quad colour of the official flag-`0x40000` tinted body
/// (`DrawTintedQuad(quad, 255, 5, 5, palette)`; `RenderObject.cpp:743`).
pub const TINT_RED: [u8; 3] = [255, 5, 5];

/// Flat outline color of the official flag-`0x800` copies
/// (`(alpha << 24) + 0x3737FF`; `Player.cpp:1170`).
pub const OUTLINE_BLUE: Color = Color::rgb_u8(0x37, 0x37, 0xFF);

/// Flat outline color of the official flag-`0x80000` undead copies
/// (`(alpha << 24) + 0x056405`; `Player.cpp:1173`).
pub const OUTLINE_UNDEAD: Color = Color::rgb_u8(0x05, 0x64, 0x05);

/// One `CRenderObject::DrawTintedQuad` result.
///
/// The official function overwrites the whole quad color and then draws the
/// same quad twice; `None` (see [`draw_tinted_quad`]) means it draws nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TintedQuad {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
    /// Number of `DrawQuad` calls; always [`Self::DRAW_COUNT`] when the quad is
    /// drawn at all.
    pub draws: u8,
}

impl TintedQuad {
    /// The number of times `DrawTintedQuad` draws the quad
    /// (`RenderObject.cpp:838-839`).
    pub const DRAW_COUNT: u8 = 2;

    /// The quad color as `0xAARRGGBB` (the official packing
    /// `((alpha << 8) + red) << 8 + green << 8 + blue`). Kept as the exact
    /// packing assertion of the tests; the render path builds an `f32`
    /// [`Color`] instead.
    #[cfg_attr(not(test), allow(dead_code))]
    pub const fn packed(self) -> u32 {
        (self.alpha as u32) << 24 | (self.red as u32) << 16 | (self.green as u32) << 8 | self.blue as u32
    }

    pub fn color(self) -> Color {
        Color::rgba_u8(self.red, self.green, self.blue, self.alpha)
    }
}

/// `CRenderObject::DrawTintedQuad(SpriteQuad&, short red, short green, short
/// blue, ...)` - `RenderObject.cpp:826-846`.
///
/// The caller passes the fixed color channels (see [`TINT_BLUE`] /
/// [`TINT_RED`]); this function computes the alpha ramp and whether the quad is
/// drawn:
///
/// ```c
/// if (phase <= 10)       color = ((phase * 15) << 24) | rgb;
/// else if (phase <= 20)  color = (0xA0 << 24) | rgb;
/// else if (phase <= 50)  color = ((51 - phase) * 5 << 24) | rgb;
/// if (tintPhase <= 50) { DrawQuad(quad); DrawQuad(quad); }
/// ```
///
/// `phase > 50` therefore draws nothing at all (the colour branches and the
/// draw guard share the same limit). `phase == 0` is drawn but fully
/// transparent; the caller can skip it without changing the image.
pub fn draw_tinted_quad(phase: u8, red: u8, green: u8, blue: u8) -> Option<TintedQuad> {
    let alpha = if phase <= 10 {
        phase * 15
    } else if phase <= 20 {
        0xA0
    } else if phase <= 50 {
        (51 - phase) * 5
    } else {
        return None;
    };

    Some(TintedQuad {
        red,
        green,
        blue,
        alpha,
        draws: TintedQuad::DRAW_COUNT,
    })
}

/// The fixed RGB of the two official `DrawTintedQuad` call sites, when the
/// actor's current tint is recognisably one of them.
///
/// `DrawTintedQuad` is called with blue `(5, 5, 255)` behind flag `0x4000` and
/// red `(255, 5, 5)` behind flag `0x40000` (`RenderObject.cpp:739-743`); the
/// flag word is not bridged to the entity. The reachable blue/red bodies
/// (`EF_BLUEBODY`, `EF_MADNESS_BLUE`, `EF_MADNESS_RED`) already put exactly
/// that hue on the caster through their descriptor body flash, so a strongly
/// blue or red tint snaps to the exact official constant; any other tint
/// (Landbody's warm flash, a plain white actor) is left as it is.
pub fn official_tint_rgb(tint: Color) -> Option<[u8; 3]> {
    let blue_dominant = tint.blue > tint.red * 1.5 && tint.blue > tint.green * 1.5;
    let red_dominant = tint.red > tint.green * 1.5 && tint.red > tint.blue * 1.5;

    if blue_dominant {
        Some(TINT_BLUE)
    } else if red_dominant {
        Some(TINT_RED)
    } else {
        None
    }
}

/// The flag-`0x20000` rise of `Player.cpp:1206-1218` in screen units:
/// `(25 - tintPhase) * 20`.
///
/// The original multiplies this by `400 / desiredDistance` and clamps the
/// result to `>= 0` before shifting the quad up:
///
/// ```c
/// const int riseUnits = (25 - this->render.tintPhase) * 20;
/// const float rise = std::max(400.0f / camera.desiredDistance * riseUnits, 0.0f);
/// ```
///
/// On the Rust side the value is kept in units because the client draws
/// world-space sprites and has no screen-space body quad to lift; see
/// [`Common::render_tint_phase`](crate::world::entity::Common). It is ported
/// and tested now so the sprite-quad pass only has to supply the projection.
#[cfg_attr(not(test), allow(dead_code))]
pub fn tint_rise_units(phase: u8) -> i32 {
    (25 - i32::from(phase)) * 20
}

/// The flag-`0x20000` alpha of `Player.cpp:1206-1218`:
/// `(tintPhase - 10) * 17`, clamped to `0..=255`.
///
/// The decompiled `short alpha = (short)((tintPhase - 10) * 17)` can be
/// negative (early phases) and is then clamped to zero by the
/// `std::max<short>(alpha, 0)` in the color expression; values above `255` are
/// clamped to `255` by the explicit `if (alpha > 255)`.
#[cfg_attr(not(test), allow(dead_code))]
pub fn tint_rise_alpha(phase: u8) -> u8 {
    ((i32::from(phase) - 10) * 17).clamp(0, 255) as u8
}

/// `CRenderObject::ScaleByPulse(float& scale)` - `RenderObject.cpp:749-767`.
///
/// ```c
/// const unsigned phase = shrinkPhase * 4u + 90u;
/// const unsigned index = phase >= 90u ? phase : 90u;
/// if (index < 270) scale = ((g_sinTable[index] + 1.0f) * 0.25f + 0.5f) * scale;
/// else scale = scale * 0.5f;
/// ```
///
/// `ScaleByFadePulse` (`RenderObject.cpp:3009-3017`) and
/// `CGameActor::GetNameOffset` (`GameActor.cpp:932-940`) use the same ramp; the
/// latter's `index > 270` is equivalent because `g_sinTable[270] == -1`.
///
/// The value is the multiplier the caller applies to the actor scale; `1.0` at
/// `shrinkPhase == 0`, `0.5` from `shrinkPhase == 45` up (including any phase
/// the byte can hold past the official `0..=45` write range).
pub fn pulse_scale(shrink_phase: u8) -> f32 {
    let index = u32::from(shrink_phase) * 4 + 90;

    if index < 270 {
        (sin_table(index as usize) + 1.0) * 0.25 + 0.5
    } else {
        0.5
    }
}

/// Outline alpha factor of `render.outlineAlpha` (`unsigned char`), matching
/// the `(alpha << 24) | rgb` color of the official outline copies
/// (`Player.cpp:1142`, `RenderObject.cpp:2072`).
pub fn outline_alpha(alpha: u8) -> f32 {
    f32::from(alpha) / 255.0
}

/// `g_sinTable[index]` from the decompiled client.
///
/// `CreateTrigonometricTable` (`ClientAngles.cpp:24-34`) fills
/// `g_sinTable[index] = ClientSin(index / 180.0f * 3.141592f)`, i.e. the
/// literal `3.141592f` (not `PI`) and a double-precision `sin` narrowed to
/// `f32`. The pulse ramp reads indices `90..270` only, so the positive half is
/// the only part needed here.
fn sin_table(index: usize) -> f32 {
    let angle = index as f32 / 180.0f32 * 3.141592f32;

    (angle as f64).sin() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blue(phase: u8) -> Option<TintedQuad> {
        draw_tinted_quad(phase, TINT_BLUE[0], TINT_BLUE[1], TINT_BLUE[2])
    }

    fn red(phase: u8) -> Option<TintedQuad> {
        draw_tinted_quad(phase, TINT_RED[0], TINT_RED[1], TINT_RED[2])
    }

    // `DrawTintedQuad` alpha ramp, exact values from `RenderObject.cpp:826-846`.
    #[test]
    fn tint_alpha_ramp_matches_the_decompiled_branches() {
        assert_eq!(blue(0).unwrap().alpha, 0, "0 * 15");
        assert_eq!(blue(1).unwrap().alpha, 15);
        assert_eq!(blue(5).unwrap().alpha, 75);
        assert_eq!(blue(10).unwrap().alpha, 150, "last `phase * 15` frame");

        // `phase <= 20` pins 0xA0 regardless of the phase.
        assert_eq!(blue(11).unwrap().alpha, 0xA0);
        assert_eq!(blue(15).unwrap().alpha, 0xA0);
        assert_eq!(blue(20).unwrap().alpha, 0xA0);

        // `phase <= 50` ramps (51 - phase) * 5 back down.
        assert_eq!(blue(21).unwrap().alpha, 150);
        assert_eq!(blue(30).unwrap().alpha, 105);
        assert_eq!(blue(49).unwrap().alpha, 10);
        assert_eq!(blue(50).unwrap().alpha, 5, "last drawn frame");
    }

    #[test]
    fn tint_is_not_drawn_past_phase_fifty() {
        // The colour branches and the `if (tintPhase <= 50)` draw guard share
        // the same limit: 51..=255 produce no quad at all.
        for phase in [51u8, 52, 100, 151, 199, 200, 255] {
            assert_eq!(blue(phase), None, "phase {phase}");
            assert_eq!(red(phase), None, "phase {phase}");
        }
    }

    #[test]
    fn tint_keeps_the_caller_rgb_and_packs_argb() {
        let quad = blue(20).unwrap();
        assert_eq!((quad.red, quad.green, quad.blue), (5, 5, 255));
        assert_eq!(quad.draws, 2, "DrawTintedQuad calls DrawQuad twice");
        assert_eq!(quad.packed(), 0xA0_05_05_FF);

        let quad = red(30).unwrap();
        assert_eq!((quad.red, quad.green, quad.blue), (255, 5, 5));
        assert_eq!(quad.packed(), 0x69_FF_05_05, "(51 - 30) * 5 == 105 == 0x69");
    }

    #[test]
    fn tint_color_is_argb_bytes_over_255() {
        let color = blue(10).unwrap().color();
        assert!((color.red - 5.0 / 255.0).abs() < 1e-6);
        assert!((color.green - 5.0 / 255.0).abs() < 1e-6);
        assert!((color.blue - 1.0).abs() < 1e-6);
        assert!((color.alpha - 150.0 / 255.0).abs() < 1e-6);
    }

    // `Player.cpp:1200-1218` - `riseUnits = (25 - tintPhase) * 20` and
    // `alpha = clamp((tintPhase - 10) * 17, 0, 255)`.
    #[test]
    fn tint_rise_units_and_alpha_match_the_rise_path() {
        assert_eq!(tint_rise_units(0), 500);
        assert_eq!(tint_rise_units(10), 300);
        assert_eq!(tint_rise_units(11), 280);
        assert_eq!(tint_rise_units(25), 0, "last positive rise unit");
        assert_eq!(tint_rise_units(26), -20);
        assert_eq!(tint_rise_units(50), -500);
        assert_eq!(tint_rise_units(199), -3480);
        assert_eq!(tint_rise_units(255), -4600);

        assert_eq!(tint_rise_alpha(0), 0, "negative alpha clamps to zero");
        assert_eq!(tint_rise_alpha(10), 0);
        assert_eq!(tint_rise_alpha(11), 17);
        assert_eq!(tint_rise_alpha(20), 170);
        assert_eq!(tint_rise_alpha(25), 255, "15 * 17 == 255");
        assert_eq!(tint_rise_alpha(26), 255, "272 clamps to 255");
        assert_eq!(tint_rise_alpha(50), 255);
        assert_eq!(tint_rise_alpha(100), 255);
        assert_eq!(tint_rise_alpha(199), 255);
        assert_eq!(tint_rise_alpha(255), 255);
    }

    // `ScaleByPulse`, exact `f32` results of
    // `((g_sinTable[shrinkPhase * 4 + 90] + 1) * 0.25 + 0.5)`.
    #[test]
    fn pulse_scale_matches_the_decompiled_ramp() {
        assert_eq!(pulse_scale(0), 1.0, "sin(90 deg) == 1");
        assert!((pulse_scale(1) - 0.999_391_0).abs() < 1e-6);
        assert!((pulse_scale(5) - 0.984_923_2).abs() < 1e-6);
        assert!((pulse_scale(10) - 0.941_511_2).abs() < 1e-6);
        assert!((pulse_scale(11) - 0.929_835_1).abs() < 1e-6);
        assert!((pulse_scale(20) - 0.793_412_2).abs() < 1e-6);
        assert!((pulse_scale(22) - 0.758_725_0).abs() < 1e-6);
        assert!((pulse_scale(30) - 0.625_000_2).abs() < 1e-6);
        assert!((pulse_scale(40) - 0.515_076_9).abs() < 1e-6);
        assert!((pulse_scale(44) - 0.500_609_0).abs() < 1e-6);
    }

    #[test]
    fn pulse_scale_is_half_from_phase_forty_five() {
        // `index >= 270` takes the `scale * 0.5f` branch; `sin(270 deg)` is -1,
        // so phase 45's value coincides with the ramp's minimum.
        assert_eq!(pulse_scale(45), 0.5);
        assert_eq!(pulse_scale(46), 0.5);
        assert_eq!(pulse_scale(100), 0.5);
        assert_eq!(pulse_scale(199), 0.5);
        assert_eq!(pulse_scale(255), 0.5);
    }

    #[test]
    fn official_tint_rgb_snaps_only_saturated_blue_and_red() {
        // `EF_BLUEBODY` / `EF_MADNESS_BLUE` body flash.
        assert_eq!(official_tint_rgb(Color::rgb(0.02, 0.02, 1.0)), Some(TINT_BLUE));
        // `EF_MADNESS_RED` body flash.
        assert_eq!(official_tint_rgb(Color::rgb(1.0, 0.02, 0.02)), Some(TINT_RED));
        // Landbody's warm flash and a plain white actor are not one of the two
        // fixed call-site colours.
        assert_eq!(official_tint_rgb(Color::rgb(0.9, 0.85, 0.7)), None);
        assert_eq!(official_tint_rgb(Color::WHITE), None);
    }

    #[test]
    fn outline_alpha_is_the_byte_over_255() {
        assert_eq!(outline_alpha(0), 0.0);
        assert!((outline_alpha(50) - 50.0 / 255.0).abs() < 1e-6);
        assert!((outline_alpha(100) - 100.0 / 255.0).abs() < 1e-6);
        assert!((outline_alpha(199) - 199.0 / 255.0).abs() < 1e-6);
        assert!((outline_alpha(200) - 200.0 / 255.0).abs() < 1e-6);
    }
}
