//! Champion Asura/`SuperSlashFan` bodies: the primitive helpers the Champion
//! pass was blocked on, plus the effect bodies they unblock.
//!
//! | official body | `RagEffect.cpp` | primitive | ported ids |
//! | --- | --- | --- | --- |
//! | `AsuraStrike` | `:14187-14305` | 30, 68 | `EF_BEGINASURA` (328), `EF_BEGINASURA11` (510) |
//! | `TexturedQuadRingSized` | `:17437-17608` | 30 | helper for `AsuraStrike`/`WhiteYellowRings` |
//! | `WhiteYellowRings` | `:18656-18662` | 30 | `EF_AURABLADE` (367) |
//! | `SuperSlashFan` | `:16964-17023` | 35 | `EF_GUMGANG` (203), `EF_GUMGANGNPC` (455), `EF_STEELBODY` (254) |
//! | `SuperSlashSpread` | `:17017-17066` | 99 | `EF_DOUBLEGUMGANG` (418), 2 (485), 3 (486) |
//! | `TexturedCrossLevel` | `:5246-5292` | 31 | `EF_GUMGANG2` (261) |
//! | `TexturedSkillIcon` | `:3774-3840` | 68 | `EF_BEGINASURA1..7` (467-473) |
//!
//! ## The Asura Strike kanji
//!
//! The six kanji of 阿修羅覇凰拳 are `effect\asura1.tga`..`effect\asura6.tga`,
//! hard-coded in `AsuraStrike` itself (`RagEffect.cpp:14201-14210`,
//! `:14249-14260`), not a skill-icon lookup. The body spawns them as two
//! three-slot `PP_ASURACASTING` (68) primitives at frames 1 and 21
//! ([`ASURA_KANJI_COLUMNS`]); `RenderHaloLayers` then builds the ten stacked
//! quads per slot from `slot.height[0]` (x offset), `slot.distance` and the
//! per-layer `-30` lift ([`super::super::render_slots::render_halo_layers`]).
//! `AsuraStrike` copies `state.parent->render.position` into its own
//! `render.position` on **every** frame before the frame gates, so the second
//! group is anchored to the caster's frame-21 position, not the cast position.
//! [`EffectRuntime::copy_parent_position`] reproduces that copy when the client
//! feeds the caster position.
//!
//! `EF_BEGINASURA1..7` are a different visual: seven `effect\hanmoonN.tga`
//! glyph flashes through `TexturedSkillIcon(..., level = 0, mode = 0)`, whose
//! texture is the caller's `name` (resolved, not a skill-icon lookup).
//!
//! The official dispatch sites:
//!
//! * `EF_GUMGANG` - `RagEffect.cpp:24166-24168` (`SuperSlashFan(1, 0)`),
//! * `EF_GUMGANGNPC` - `:24169-24171` (`SuperSlashFan(2, 1)`),
//! * `EF_DOUBLEGUMGANG` - `:24172-24174` (`SuperSlashSpread(2, 2)`),
//! * `EF_DOUBLEGUMGANG2` - `:24175-24177` (`SuperSlashSpread(2, 1)`),
//! * `EF_DOUBLEGUMGANG3` - `:24178-24180` (`SuperSlashSpread(2, 0)`),
//! * `EF_STEELBODY` - `:24181-24183` (`SuperSlashFan(4, 0)`),
//! * `EF_GUMGANG2` - `:23506-23508`
//!   (`TexturedCrossLevel("effect\\ring_yellow.tga", 0)`),
//! * `EF_AURABLADE` - `:10368-10370` (`WhiteYellowRings()`, main `OnProcess`
//!   switch),
//! * `EF_BEGINASURA` - `:10212-10214` (`AsuraStrike(0)`, main switch),
//! * `EF_BEGINASURA1..7` - `:23627-23647` (`TexturedSkillIcon("hanmoonN.tga",
//!   0, 0)`),
//! * `EF_BEGINASURA11` - `:21512-21514` (`AsuraStrike(1)`, `OnProcess3`).
//!
//! Lifetimes come from `EffectTypes.cpp`: 203 `99999` (`:212`), 254
//! `99999999` (`:263`), 261 `300` (`:270`), 328 `1000` (`:337`), 367 `60`
//! (`:376`), 418 `99999999` (`:427`), 455 `1500` (`:464`), 467-473 `300` each
//! (`:476-482`), 485/486 `99999` (`:494-495`), 510 `1000` (`:519`).
//!
//! Every body copies `state.parent->render.position` into `render.position`
//! before its frame gate. For the frame-0 helpers the spawn anchor already is
//! that position, but [`asura_strike`] and [`textured_skill_icon`] can fire on
//! later frames, so they call [`EffectRuntime::copy_parent_position`] - the
//! same copy - and get the caster's position of the frame they fire on.
//! Primitive positions are seeded from it by `CEffectPrim::Initialize` and by
//! `FollowMaster` in the motion bodies.
//!
//! ## RNG order
//!
//! `SuperSlashFan` is the only one of these bodies that draws random numbers
//! (`ClampedRandomAtZero`, `RagEffect.cpp:174`), twelve per live slot in the
//! decompiled order: `10` (vecBPre.y), `5` (height[2]), `360` (height[3]),
//! `10` (vecBNow.y), `5` (height[6]), `360` (height[7]), `10` (vecTPre.y),
//! `5` (height[10]), `360` (height[11]), `10` (vecTNow.y), `5` (height[14]),
//! `360` (height[15]). The port consumes them from [`EffectRuntime::rng`] in
//! that exact order; `SuperSlashFan(4, 0)` therefore uses 48 values against a
//! single shared sequence. `SuperSlashSpread` draws the same twelve per slot.
//!
//! `TexturedSkillIcon`'s `level > 0`/`mode == 2` branches call `rand()` for a
//! skill-icon id. They are **not** the Asura kanji path: only
//! `EF_RECOGNIZED`/`EF_RECOGNIZED2` (Recognized Spell, not ported) take them
//! (`RagEffect.cpp:20142`, `:20209`), and those resolve
//! `유저인터페이스\item\<CSkillInfo::IdName(id)>.bmp` - the opaque item icon,
//! not a glyph. All seven ported `hanmoon` bodies pass `level == 0`,
//! `mode == 0`, so the port keeps the branch shape but leaves the skill-icon
//! texture unresolved (the hermetic runtime does not own the skill table). No
//! RNG is consumed there.

use ragnarok_packets::EffectId;

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

// ---------------------------------------------------------------------------
// Primitive helpers
// ---------------------------------------------------------------------------

/// `CRagEffect::TexturedQuadRingSized` - `RagEffect.cpp:17437-17608`.
///
/// Arms one prim 30 with four concentric rings: `maxHeight` 25/24/23/22 for
/// kind 2, 15/14/13/12 for kind 22, else 20/19/18/17; `rotStart` 180/270/0/90;
/// `alphaB` `baseAlpha + 135/90/45/0`; all four at `distance = 4.1` and
/// `riseAngle = 80`. The kind also selects `state.size` (the renderer's colour
/// table), defaulting to 5.0; kind 48 is the one value applied after the chain.
///
/// Quirks kept: the `duration < 56` bump sits *after* the frame-0 gate, the
/// twenty-entry `size` chain is spelled out literally, and a lifetime of at
/// most 60 additionally zeroes every `alphaB` and starts the slots at
/// `process = index * -5` so the rings stagger in.
pub(crate) fn textured_quad_ring_sized(runtime: &mut EffectRuntime, base_alpha: i16, name: &'static str, kind: i8) {
    // `CopyVector(this->render.position, parent.position);` - `runtime.position`
    // already is that anchor.

    // `if (this->state.stateCnt != 0) return;`
    if runtime.frame != 0 {
        return;
    }

    // `if (this->state.duration < 56) this->state.duration = 56;`
    if runtime.lifetime < 56 {
        runtime.lifetime = 56;
    }

    let lifetime = runtime.lifetime;

    let index = runtime.launch(PrimitiveId::Casting3d, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(index);

    // `state.renderFlag = RSF_ALPHA | RSF_CUSTOM_BLEND;`
    state.render_flag = 1 | 4;
    state.follow_parent = true;
    state.duration = lifetime;
    state.texture = Some(name);

    state.size = match kind {
        1 => 8.0,
        2 => 5.0,
        3 => 10.0,
        4 => 11.0,
        5 => 4.0,
        6 => 12.0,
        15 => 7.0,
        20 => 24.0,
        21 => 25.0,
        22 => 26.0,
        23 => 27.0,
        31 => 31.0,
        42 => 42.0,
        43 => 43.0,
        44 => 44.0,
        45 => 45.0,
        46 => 46.0,
        47 => 47.0,
        49 => 49.0,
        50 => 50.0,
        _ => 5.0,
    };

    if kind == 48 {
        state.size = 48.0;
    }

    let max_height = if kind == 2 {
        [25.0, 24.0, 23.0, 22.0]
    } else if kind == 22 {
        [15.0, 14.0, 13.0, 12.0]
    } else {
        [20.0, 19.0, 18.0, 17.0]
    };
    let rot_start = [180i16, 270, 0, 90];
    let alpha_bonus = [135, 90, 45, 0];

    for slot_index in 0..4usize {
        let slot = &mut state.slots[slot_index];

        slot.life = 1;
        slot.process = 0;
        slot.full_display_angle = 360;
        slot.max_height = max_height[slot_index];
        slot.rot_start = rot_start[slot_index];
        slot.alpha_b = base_alpha + alpha_bonus[slot_index];
        slot.distance = 4.1;
        slot.rise_angle = 80;
        slot.alpha_t = kind as i16;
        slot.vec_b_now[1] = 0.0;

        for height in 0..21usize {
            slot.height[height] = 0.0;
            slot.flag1[height] = 0;
        }

        if lifetime <= 60 {
            slot.alpha_b = 0;
            slot.process = slot_index as i32 * -5;
        }
    }
}

/// `CRagEffect::WhiteYellowRings` - `RagEffect.cpp:18656-18662`.
fn white_yellow_rings(runtime: &mut EffectRuntime) {
    // `CopyVector(...)` before the gate; see the module docs.

    if runtime.frame == 0 {
        textured_quad_ring_sized(runtime, 45, "effect\\ring_white.tga", 22);
        textured_quad_ring_sized(runtime, 25, "effect\\ring_yellow.tga", 22);
    }
}

/// `CRagEffect::SuperSlashFan` - `RagEffect.cpp:16964-17023`.
///
/// Arms one prim 35 (`RenderOrbitIcons`) with `count` live slots at
/// `distance = 3.5` and the five `effect\superN.bmp` icons. Each slot carries
/// four icon records `(height, alpha, textureIndex, angle)`, and the random
/// draws are taken in the decompiled order documented in the module header.
fn super_slash_fan(runtime: &mut EffectRuntime, count: i8, value: i8) {
    // `CopyVector(...)`; `runtime.position` is the anchor.

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;
    let index = runtime.launch(PrimitiveId::FloatingBubbles, [0.0, 0.0, 0.0]);

    // Take the twelve draws per slot up front, in the decompiled order
    // `10, 5, 360` per texture block. The borrow checker needs them before the
    // primitive is borrowed; the sequence is unchanged.
    let mut slot_draws = vec![[0i32; 12]; count.max(0) as usize];

    for draws in &mut slot_draws {
        for (draw, range) in draws.iter_mut().zip([10, 5, 360, 10, 5, 360, 10, 5, 360, 10, 5, 360]) {
            *draw = runtime.rng.clamped_random(range);
        }
    }

    let state = runtime.primitive_mut(index);

    // `state.renderFlag = 0; state.renderFlag |= 1u; state.renderFlag |= 5u;`
    state.render_flag = 0;
    state.render_flag |= 1;
    state.render_flag |= 5;
    state.duration = lifetime;
    state.total_texture = 5;
    state.texture = Some("effect\\super1.bmp");
    state.texture_secondary = Some("effect\\super2.bmp");
    state.texture_tertiary = Some("effect\\super3.bmp");
    state.texture_frames = vec![
        "effect\\super1.bmp",
        "effect\\super2.bmp",
        "effect\\super3.bmp",
        "effect\\super4.bmp",
        "effect\\super5.bmp",
    ];

    for slot in &mut state.slots {
        slot.life = 0;
    }

    for (i, draws) in slot_draws.iter().enumerate() {
        let slot = &mut state.slots[i];

        slot.life = 1;
        slot.distance = 3.5;

        slot.vec_b_pre[1] = -draws[0] as f32;
        slot.height[0] = 10.0;
        slot.height[1] = 70.0;
        slot.height[2] = draws[1] as f32;
        slot.height[3] = draws[2] as f32;
        slot.vec_b_now[1] = -draws[3] as f32;
        slot.height[4] = 10.0;
        slot.height[5] = 130.0;
        slot.height[6] = draws[4] as f32;
        slot.height[7] = draws[5] as f32;
        slot.vec_t_pre[1] = -draws[6] as f32;
        slot.height[8] = 10.0;
        slot.height[9] = 190.0;
        slot.height[10] = draws[7] as f32;
        slot.height[11] = draws[8] as f32;
        slot.vec_t_now[1] = -draws[9] as f32;
        slot.height[12] = 10.0;
        slot.height[13] = 250.0;
        slot.height[14] = draws[10] as f32;
        slot.height[15] = draws[11] as f32;
        slot.flag1[4] = value;
    }
}

/// `CRagEffect::SuperSlashSpread` - `RagEffect.cpp:17017-17066`.
///
/// Arms one prim 99 (`RenderDoubleGumGang`) with `count` live slots at
/// `distance = 2.0` and the same five `effect\superN.bmp` textures as
/// [`super_slash_fan`]. The slot layout is prim 35's four
/// `(height, alpha, textureIndex, angle)` records, but the alpha seeds are
/// 40/80/120/160 - the renderer fades each ring in against `slot.process` - and
/// `height[2|6|10|14]` is the texture index. `flag1[4] = value` selects the
/// renderer's colour table (2 red, 1 grey, 0 blue) and `state.size` is zeroed.
///
/// Quirks kept: `process` is explicitly reset to 0, `height[0|4|8|12]` stay at
/// the `10.0` the original writes, and the twelve draws per slot follow the
/// same `10, 5, 360` order as `SuperSlashFan`.
fn super_slash_spread(runtime: &mut EffectRuntime, count: i8, value: i8) {
    // `CopyVector(...)`; `runtime.position` is the anchor.

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;
    let index = runtime.launch(PrimitiveId::DoubleGumGang, [0.0, 0.0, 0.0]);

    // Same twelve draws per slot as `SuperSlashFan`, block by block, taken
    // before the primitive is borrowed.
    let mut slot_draws = vec![[0i32; 12]; count.max(0) as usize];

    for draws in &mut slot_draws {
        for (draw, range) in draws.iter_mut().zip([10, 5, 360, 10, 5, 360, 10, 5, 360, 10, 5, 360]) {
            *draw = runtime.rng.clamped_random(range);
        }
    }

    let state = runtime.primitive_mut(index);

    // `state.renderFlag = RSF_ALPHA | RSF_CUSTOM_BLEND;`
    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.total_texture = 5;
    state.texture = Some("effect\\super1.bmp");
    state.texture_secondary = Some("effect\\super2.bmp");
    state.texture_tertiary = Some("effect\\super3.bmp");
    state.texture_frames = vec![
        "effect\\super1.bmp",
        "effect\\super2.bmp",
        "effect\\super3.bmp",
        "effect\\super4.bmp",
        "effect\\super5.bmp",
    ];
    // `state.size = 0.0f;`
    state.size = 0.0;

    for slot in &mut state.slots {
        slot.life = 0;
    }

    for (i, draws) in slot_draws.iter().enumerate() {
        let slot = &mut state.slots[i];

        slot.process = 0;
        slot.life = 1;
        slot.distance = 2.0;
        slot.vec_b_pre[1] = -draws[0] as f32;
        slot.height[0] = 10.0;
        slot.height[1] = 40.0;
        slot.height[2] = draws[1] as f32;
        slot.height[3] = draws[2] as f32;
        slot.vec_b_now[1] = -draws[3] as f32;
        slot.height[4] = 10.0;
        slot.height[5] = 80.0;
        slot.height[6] = draws[4] as f32;
        slot.height[7] = draws[5] as f32;
        slot.vec_t_pre[1] = -draws[6] as f32;
        slot.height[8] = 10.0;
        slot.height[9] = 120.0;
        slot.height[10] = draws[7] as f32;
        slot.height[11] = draws[8] as f32;
        slot.vec_t_now[1] = -draws[9] as f32;
        slot.height[12] = 10.0;
        slot.height[13] = 160.0;
        slot.height[14] = draws[10] as f32;
        slot.height[15] = draws[11] as f32;
        slot.flag1[4] = value;
    }
}

/// `CVolcanoEffect`'s ground shockwave for `EF_GUMGANG2` - the segmented
/// `ring_yellow.tga` wave that the official client draws in addition to
/// `TexturedCrossLevel`.
///
/// The original builds 4 rows x 22 radial segments as `CParticle` quads
/// (`ThrowEffects.cpp:619-714`). RO-Next has no generic particle renderer for
/// that effect, so the same 360-degree wave is assembled from the ported
/// `PP_CHEMICALPROTECTION` wedge primitive: nine primitives with four slots
/// each reproduce the 36 ten-degree segments. The chemical motion provides the
/// 20-frame fade-in and the 20-frame fade-out; the texture is the same
/// `ring_yellow.tga` the official wave uses.
fn fury_shockwaves(runtime: &mut EffectRuntime) {
    if runtime.frame != 0 {
        return;
    }

    // `MM_QUAKE` is sent by every official `EF_GUMGANG2` dispatch
    // (`MO_EXPLOSIONSPIRITS`, `MO_STEELBODY`, `SL_SKA`).
    runtime.quake();

    let lifetime = runtime.lifetime;

    for group in 0..9usize {
        let index = runtime.launch(PrimitiveId::ChemicalProtection, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(index);

        state.render_flag = 1 | 4;
        // EF_GUMGANG2's official lifetime is 300 frames; EF_GUMGANG (the
        // persistent Fury aura) is 99999, so do not inherit that lifetime for
        // the shockwave.
        state.duration = lifetime.min(300);
        state.total_texture = 1;
        state.texture = Some("effect\\ring_yellow.tga");
        state.size = 0.0;

        for slot_index in 0..4usize {
            let slot = &mut state.slots[slot_index];

            slot.life = 1;
            slot.process = 0;
            slot.distance = 35.0;
            slot.rise_angle = ((group * 4 + slot_index) * 10) as i16;
            slot.alpha_b = 0;
        }
    }
}

/// `CRagEffect::TexturedCrossLevel` - `RagEffect.cpp:5246-5292`.
///
/// Arms one prim 31 (`PrimWaveSpin`) with four rings at `distance` 1..4,
/// `riseAngle = 80`, `maxHeight = 25` and `alphaT = level`; `state.accel` is
/// set to the exact `99.0` marker the motion body tests.
fn textured_cross_level(runtime: &mut EffectRuntime, name: &'static str, level: i8) {
    // `CopyVector(...)`; `runtime.position` is the anchor.

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;
    let index = runtime.launch(PrimitiveId::WaveSpin, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(index);

    // `state.renderFlag = RSF_ALPHA | RSF_CUSTOM_BLEND;`
    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.texture = Some(name);
    state.size = 0.0;
    state.accel = 99.0;

    for (slot_index, slot) in state.slots.iter_mut().enumerate() {
        slot.life = 1;
        slot.process = 0;
        slot.full_display_angle = 360;
        slot.max_height = 25.0;
        slot.rot_start = slot_index as i16 * 90;
        slot.alpha_b = 0;
        slot.distance = slot_index as f32 + 1.0;
        slot.rise_angle = 80;
        slot.alpha_t = level as i16;
        slot.vec_b_now[1] = 0.0;

        for height in 0..21usize {
            slot.height[height] = 0.0;
            slot.flag1[height] = 0;
        }
    }
}

/// `CRagEffect::TexturedSkillIcon` - `RagEffect.cpp:3774-3840`.
///
/// Fires on the frame the caller selected: `level == 0` means frame 0,
/// otherwise frame `level`. Arms one prim 68 with four slots cleared and slot
/// 0 live, `size = 0`, `alphaB = 0`, `flag1[0] = 0`.
///
/// `level == 0` (every ported call site) uses the caller's `name` texture -
/// for `EF_BEGINASURA1..7` that is the resolved `effect\hanmoonN.tga`. The
/// `mode == 2`/`level > 0` branches resolve an **item** icon through
/// `CSkillInfo::IdName` plus `rand()`; they belong to
/// `EF_RECOGNIZED`/`EF_RECOGNIZED2`, not to Asura, and no ported body takes
/// them (the port leaves the texture unresolved there; see the module docs).
fn textured_skill_icon(runtime: &mut EffectRuntime, name: &'static str, level: i16, mode: i16) {
    // `CopyVector(this->render.position, this->state.parent->render.position);`
    // also runs before the frame gate.
    runtime.copy_parent_position();

    // `if (this->state.stateCnt != (level == 0 ? 0 : level)) return;`
    let gate = if level == 0 { 0 } else { level as i32 };

    if runtime.frame != gate {
        return;
    }

    let lifetime = runtime.lifetime;
    let index = runtime.launch(PrimitiveId::AsuraCasting, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(index);

    // `state.renderFlag = RSF_ALPHA | RSF_CUSTOM_BLEND;`
    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.total_texture = 1;
    state.texture = if mode == 2 || level > 0 { None } else { Some(name) };
    state.size = 0.0;

    for slot in &mut state.slots {
        slot.life = 0;
    }

    state.slots[0].life = 1;
    state.slots[0].process = 0;

    if level > 0 {
        state.slots[0].distance = 5.0;
        state.slots[0].height[0] = 1.0;
        state.slots[0].flag1[4] = 4;
    } else {
        state.slots[0].distance = if mode == 3 { 7.0 } else { 18.0 };
        state.slots[0].height[0] = 0.0;
        state.slots[0].flag1[4] = 1;
    }

    state.slots[0].alpha_b = 0;
    state.slots[0].flag1[0] = 0;

    for height in 1..21usize {
        state.slots[0].height[height] = 0.0;
    }

    state.slots[0].flag1[5] = 0;
}

/// The six Asura Strike kanji, in the official `state.textures[]` order of the
/// two `PP_ASURACASTING` groups (`RagEffect.cpp:14201-14210`, `:14249-14260`).
///
/// Group 0 is the frame-1 column (`asura3`, `asura2`, `asura1` in slots 0, 1,
/// 2) and group 1 the frame-21 column (`asura4`, `asura5`, `asura6`). The
/// `asura11..16` variants are the level >= 4016 (`EF_BEGINASURA11`) set.
pub const ASURA_KANJI_COLUMNS: [[&str; 3]; 2] = [["effect\\asura3.tga", "effect\\asura2.tga", "effect\\asura1.tga"], [
    "effect\\asura4.tga",
    "effect\\asura5.tga",
    "effect\\asura6.tga",
]];

/// `EF_BEGINASURA11`'s kanji set (`RagEffect.cpp:14207-14209`,
/// `:14255-14257`), same slot order as [`ASURA_KANJI_COLUMNS`].
pub const ASURA_KANJI_COLUMNS_MONSTER: [[&str; 3]; 2] = [["effect\\asura13.tga", "effect\\asura12.tga", "effect\\asura11.tga"], [
    "effect\\asura14.tga",
    "effect\\asura15.tga",
    "effect\\asura16.tga",
]];

/// `CRagEffect::AsuraStrike` - `RagEffect.cpp:14187-14305`.
///
/// Frame 0 raises the lifetime to 350, plays `effect\EF_BeginSpell.wav` and
/// spawns the two white ring sets; frame 1 spawns the first prim 68 kanji
/// column (`asura3/2/1.tga` / `asura13/12/11.tga`), frame 21 the second
/// (`asura4/5/6.tga` / `asura14/15/16.tga`). Both columns carry three slots
/// with `process = index * 20 - 40` (first) / `-40 - index * 20` (second),
/// `flag1[0] = index`, and kind-dependent `distance`/`height[0]`.
fn asura_strike(runtime: &mut EffectRuntime, kind: i8) {
    // `CopyVector(this->render.position, this->state.parent->render.position);`
    // runs on every frame, before the frame gates.
    runtime.copy_parent_position();

    if runtime.frame == 0 {
        // `if (this->state.duration < 350) this->state.duration = 350;`
        if runtime.lifetime < 350 {
            runtime.lifetime = 350;
        }

        runtime.play_sound("effect\\EF_BeginSpell.wav");
        textured_quad_ring_sized(runtime, 45, "effect\\ring_white.tga", 2);
        textured_quad_ring_sized(runtime, 25, "effect\\ring_white.tga", 2);
    }

    if runtime.frame == 1 {
        let lifetime = runtime.lifetime;
        let index = runtime.launch(PrimitiveId::AsuraCasting, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(index);

        // `state.renderFlag = 0; |= 1; |= 5;`
        state.render_flag = 0;
        state.render_flag |= 1;
        state.render_flag |= 5;
        state.duration = lifetime;
        state.total_texture = 3;

        let textures = if kind == 0 {
            ASURA_KANJI_COLUMNS[0]
        } else {
            ASURA_KANJI_COLUMNS_MONSTER[0]
        };

        state.texture = Some(textures[0]);
        state.texture_secondary = Some(textures[1]);
        state.texture_tertiary = Some(textures[2]);
        state.texture_frames = textures.to_vec();
        state.size = 0.0;

        for slot in &mut state.slots {
            slot.life = 0;
        }

        for slot_index in 0..3usize {
            let slot = &mut state.slots[slot_index];

            slot.life = 1;
            slot.process = slot_index as i32 * 20 - 40;
            slot.alpha_b = 0;
            slot.flag1[0] = slot_index as i8;

            if kind == 0 {
                slot.distance = 18.0;
                slot.height[0] = (-6 - slot_index as i32 * 12) as f32;
                slot.flag1[4] = 0;
            } else {
                slot.distance = 24.0;
                slot.height[0] = (-8 - slot_index as i32 * 16) as f32;
                slot.flag1[4] = 3;
            }

            for height in 1..11usize {
                slot.height[height] = 0.0;
            }
        }
    }

    if runtime.frame == 21 {
        let lifetime = runtime.lifetime;
        let index = runtime.launch(PrimitiveId::AsuraCasting, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(index);

        state.render_flag = 0;
        state.render_flag |= 1;
        state.render_flag |= 5;
        state.duration = lifetime;
        state.total_texture = 3;

        let textures = if kind == 0 {
            ASURA_KANJI_COLUMNS[1]
        } else {
            ASURA_KANJI_COLUMNS_MONSTER[1]
        };

        state.texture = Some(textures[0]);
        state.texture_secondary = Some(textures[1]);
        state.texture_tertiary = Some(textures[2]);
        state.texture_frames = textures.to_vec();
        state.size = 0.0;

        for slot in &mut state.slots {
            slot.life = 0;
        }

        for slot_index in 0..3usize {
            let slot = &mut state.slots[slot_index];

            slot.life = 1;
            slot.process = -40 - slot_index as i32 * 20;
            slot.alpha_b = 0;
            slot.flag1[0] = slot_index as i8;

            if kind == 0 {
                slot.distance = 18.0;
                slot.height[0] = (6 + slot_index as i32 * 12) as f32;
                slot.flag1[4] = 0;
            } else {
                slot.distance = 24.0;
                slot.height[0] = (8 + slot_index as i32 * 16) as f32;
                slot.flag1[4] = 3;
            }

            for height in 1..11usize {
                slot.height[height] = 0.0;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Effect bodies
// ---------------------------------------------------------------------------

/// `EF_AURABLADE` (367) - `case EF_AURABLADE:` (`RagEffect.cpp:10368-10370`),
/// lifetime 60 (`EffectTypes.cpp:376`).
#[derive(Debug, Default)]
pub struct AuraBlade;

impl PortedEffect for AuraBlade {
    fn lifetime(&self) -> i32 {
        60
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        white_yellow_rings(runtime);

        true
    }
}

/// `EF_GUMGANG` (203) - `case EF_GUMGANG:` (`RagEffect.cpp:24166-24168`),
/// lifetime 99999 (`EffectTypes.cpp:212`).
#[derive(Debug, Default)]
pub struct Gumgang;

impl PortedEffect for Gumgang {
    fn lifetime(&self) -> i32 {
        99_999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // `EF_GUMGANG` is parented to the actor; keep the anchor on the live
        // caster so the orbit icons follow movement.
        runtime.copy_parent_position();

        // The official Fury cast launches TWO effects:
        //   EF_GUMGANG  -> SuperSlashFan(1, 0)
        //   EF_GUMGANG2 -> CVolcanoEffect's yellow ground shockwave
        // (`GameModePackets.cpp:18832-18838`). The client maps the skill to a
        // single ported body, so compose both official bodies here; otherwise
        // the shockwave is never spawned.
        super_slash_fan(runtime, 1, 0);
        fury_shockwaves(runtime);

        true
    }
}

/// `EF_GUMGANGNPC` (455) - `case EF_GUMGANGNPC:` (`RagEffect.cpp:24169-24171`),
/// lifetime 1500 (`EffectTypes.cpp:464`).
///
/// Same `SuperSlashFan` fan as `EF_GUMGANG`, with two live slots and
/// `flag1[4] = 1` (the renderer's grey/red icon colour).
#[derive(Debug, Default)]
pub struct GumgangNpc;

impl PortedEffect for GumgangNpc {
    fn lifetime(&self) -> i32 {
        1500
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        super_slash_fan(runtime, 2, 1);

        true
    }
}

/// `EF_STEELBODY` (254) - `case EF_STEELBODY:` (`RagEffect.cpp:24181-24183`),
/// lifetime 99999999 (`EffectTypes.cpp:263`).
#[derive(Debug, Default)]
pub struct SteelBody;

impl PortedEffect for SteelBody {
    fn lifetime(&self) -> i32 {
        99_999_999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        super_slash_fan(runtime, 4, 0);

        true
    }
}

/// `EF_DOUBLEGUMGANG` (418) - `case EF_DOUBLEGUMGANG:`
/// (`RagEffect.cpp:24172-24174`), lifetime 99999999 (`EffectTypes.cpp:427`).
///
/// `SuperSlashSpread(2, 2)`: two prim 99 slots with the red colour table.
#[derive(Debug, Default)]
pub struct DoubleGumGang;

impl PortedEffect for DoubleGumGang {
    fn lifetime(&self) -> i32 {
        99_999_999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        super_slash_spread(runtime, 2, 2);

        true
    }
}

/// `EF_DOUBLEGUMGANG2` (485) - `case EF_DOUBLEGUMGANG2:`
/// (`RagEffect.cpp:24175-24177`), lifetime 99999 (`EffectTypes.cpp:494`).
///
/// `SuperSlashSpread(2, 1)`: two prim 99 slots with the grey colour table.
#[derive(Debug, Default)]
pub struct DoubleGumGang2;

impl PortedEffect for DoubleGumGang2 {
    fn lifetime(&self) -> i32 {
        99_999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        super_slash_spread(runtime, 2, 1);

        true
    }
}

/// `EF_DOUBLEGUMGANG3` (486) - `case EF_DOUBLEGUMGANG3:`
/// (`RagEffect.cpp:24178-24180`), lifetime 99999 (`EffectTypes.cpp:495`).
///
/// `SuperSlashSpread(2, 0)`: two prim 99 slots with the blue colour table.
#[derive(Debug, Default)]
pub struct DoubleGumGang3;

impl PortedEffect for DoubleGumGang3 {
    fn lifetime(&self) -> i32 {
        99_999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        super_slash_spread(runtime, 2, 0);

        true
    }
}

/// `EF_GUMGANG2` (261) - `case EF_GUMGANG2:` (`RagEffect.cpp:23506-23508`),
/// lifetime 300 (`EffectTypes.cpp:270`).
#[derive(Debug, Default)]
pub struct Gumgang2;

impl PortedEffect for Gumgang2 {
    fn lifetime(&self) -> i32 {
        300
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        textured_cross_level(runtime, "effect\\ring_yellow.tga", 0);
        fury_shockwaves(runtime);

        true
    }
}

/// `EF_BEGINASURA` (328) - `case EF_BEGINASURA:` (`RagEffect.cpp:10212-10214`,
/// main `OnProcess` switch), lifetime 1000 (`EffectTypes.cpp:337`).
#[derive(Debug, Default)]
pub struct BeginAsura;

impl PortedEffect for BeginAsura {
    fn lifetime(&self) -> i32 {
        1000
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        asura_strike(runtime, 0);

        true
    }
}

/// `EF_BEGINASURA11` (510) - `case EF_BEGINASURA11:`
/// (`RagEffect.cpp:21512-21514`, `OnProcess3`), lifetime 1000
/// (`EffectTypes.cpp:519`).
#[derive(Debug, Default)]
pub struct BeginAsura11;

impl PortedEffect for BeginAsura11 {
    fn lifetime(&self) -> i32 {
        1000
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        asura_strike(runtime, 1);

        true
    }
}

/// `EF_BEGINASURA1..7` (467-473) - the seven `hanmoon` icon flashes
/// (`RagEffect.cpp:23627-23647`), lifetime 300 each
/// (`EffectTypes.cpp:476-482`).
#[derive(Debug)]
pub struct BeginAsuraIcon {
    texture: &'static str,
}

impl BeginAsuraIcon {
    pub const fn new(texture: &'static str) -> Self {
        Self { texture }
    }
}

impl PortedEffect for BeginAsuraIcon {
    fn lifetime(&self) -> i32 {
        300
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        textured_skill_icon(runtime, self.texture, 0, 0);

        true
    }
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// The seven `EF_BEGINASURA1..7` icon textures, in id order
/// (`RagEffect.cpp:23627-23647`). Note the official order is 1, 2, 3, 4, 7, 5,
/// 6.
pub const ASURA_ICON_TEXTURES: [&str; 7] = [
    "effect\\hanmoon1.tga",
    "effect\\hanmoon2.tga",
    "effect\\hanmoon3.tga",
    "effect\\hanmoon4.tga",
    "effect\\hanmoon7.tga",
    "effect\\hanmoon5.tga",
    "effect\\hanmoon6.tga",
];

/// Creates the ported body for an Asura/Champion-fan effect id.
pub fn spawn(effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    Some(match effect {
        EffectId::Gumgang => Box::new(Gumgang),
        EffectId::Gumgangnpc => Box::new(GumgangNpc),
        EffectId::Doublegumgang => Box::new(DoubleGumGang),
        EffectId::Doublegumgang2 => Box::new(DoubleGumGang2),
        EffectId::Doublegumgang3 => Box::new(DoubleGumGang3),
        EffectId::Steelbody => Box::new(SteelBody),
        EffectId::Gumgang2 => Box::new(Gumgang2),
        EffectId::Volcano => super::volcano::spawn(EffectId::Volcano).unwrap(),
        EffectId::Deluge => super::volcano::spawn(EffectId::Deluge).unwrap(),
        EffectId::Violentgale => super::volcano::spawn(EffectId::Violentgale).unwrap(),
        EffectId::Landprotector => super::volcano::spawn(EffectId::Landprotector).unwrap(),
        EffectId::Ganbantein => super::volcano::spawn(EffectId::Ganbantein).unwrap(),
        EffectId::Gumgang3 => super::volcano::spawn(EffectId::Gumgang3).unwrap(),
        EffectId::Gumgang4 => super::volcano::spawn(EffectId::Gumgang4).unwrap(),
        EffectId::Gumgang5 => super::volcano::spawn(EffectId::Gumgang5).unwrap(),
        EffectId::Gumgang6 => super::volcano::spawn(EffectId::Gumgang6).unwrap(),
        EffectId::Gumgang7 => super::volcano::spawn(EffectId::Gumgang7).unwrap(),
        EffectId::Gumgang8 => super::volcano::spawn(EffectId::Gumgang8).unwrap(),
        EffectId::Gumgang9 => super::volcano::spawn(EffectId::Gumgang9).unwrap(),
        EffectId::Aurablade => Box::new(AuraBlade),
        EffectId::Beginasura => Box::new(BeginAsura),
        EffectId::Beginasura11 => Box::new(BeginAsura11),
        EffectId::Beginasura1 => Box::new(BeginAsuraIcon::new(ASURA_ICON_TEXTURES[0])),
        EffectId::Beginasura2 => Box::new(BeginAsuraIcon::new(ASURA_ICON_TEXTURES[1])),
        EffectId::Beginasura3 => Box::new(BeginAsuraIcon::new(ASURA_ICON_TEXTURES[2])),
        EffectId::Beginasura4 => Box::new(BeginAsuraIcon::new(ASURA_ICON_TEXTURES[3])),
        EffectId::Beginasura5 => Box::new(BeginAsuraIcon::new(ASURA_ICON_TEXTURES[4])),
        EffectId::Beginasura6 => Box::new(BeginAsuraIcon::new(ASURA_ICON_TEXTURES[5])),
        EffectId::Beginasura7 => Box::new(BeginAsuraIcon::new(ASURA_ICON_TEXTURES[6])),
        _ => return None,
    })
}

/// `EffectLifetime` for [`ids`] (`EffectTypes.cpp`).
pub fn lifetimes() -> &'static [(EffectId, i32)] {
    &[
        // `EffectTypes.cpp:212`.
        (EffectId::Gumgang, 99_999),
        // `EffectTypes.cpp:464`.
        (EffectId::Gumgangnpc, 1500),
        // `EffectTypes.cpp:427`.
        (EffectId::Doublegumgang, 99_999_999),
        // `EffectTypes.cpp:494-495`.
        (EffectId::Doublegumgang2, 99_999),
        (EffectId::Doublegumgang3, 99_999),
        // `EffectTypes.cpp:263`.
        (EffectId::Steelbody, 99_999_999),
        // `EffectTypes.cpp:270`.
        (EffectId::Gumgang2, 300),
        // `EffectTypes.cpp:337`.
        (EffectId::Beginasura, 1000),
        // `EffectTypes.cpp:376`.
        (EffectId::Aurablade, 60),
        // `EffectTypes.cpp:476-482`.
        (EffectId::Beginasura1, 300),
        (EffectId::Beginasura2, 300),
        (EffectId::Beginasura3, 300),
        (EffectId::Beginasura4, 300),
        (EffectId::Beginasura5, 300),
        (EffectId::Beginasura6, 300),
        (EffectId::Beginasura7, 300),
        // `EffectTypes.cpp:519`.
        (EffectId::Beginasura11, 1000),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::angles::Angles;
    use crate::world::effect::port::primitive::PrimitiveId;
    use crate::world::effect::port::render_slots;
    use crate::world::effect::port::runtime::{EffectRng, step};

    fn runtime(effect: &dyn PortedEffect, seed: u64) -> EffectRuntime {
        EffectRuntime::new(effect.lifetime(), [0.0, 0.0], seed)
    }

    #[test]
    fn entries_match_their_official_lifetimes() {
        for &(effect, lifetime) in lifetimes() {
            let body = spawn(effect).unwrap_or_else(|| panic!("{effect:?} does not spawn"));
            assert_eq!(body.lifetime(), lifetime, "{effect:?}");
        }
    }

    #[test]
    fn every_body_spawns_a_textured_primitive_on_frame_zero() {
        for &(effect, _) in lifetimes() {
            let mut body = spawn(effect).unwrap();
            let mut runtime = runtime(body.as_ref(), 7);

            step(body.as_mut(), &mut runtime);

            assert!(!runtime.primitives.is_empty(), "{effect:?}");
            assert!(
                runtime.primitives.iter().any(|primitive| primitive.state.texture.is_some()),
                "{effect:?} spawned no textured primitive"
            );
        }
    }

    #[test]
    fn aurablade_spawns_the_two_yellow_white_rings() {
        let mut body = AuraBlade;
        let mut runtime = runtime(&body, 3);

        // Drive the body alone so the pristine slot values can be inspected.
        body.on_process(&mut runtime);

        assert_eq!(runtime.primitives.len(), 2);

        let white = &runtime.primitives[0].state;
        assert_eq!(white.count, PrimitiveId::Casting3d);
        assert_eq!(white.count.id(), 30);
        assert_eq!(white.texture, Some("effect\\ring_white.tga"));
        assert_eq!(white.size, 26.0);
        assert_eq!(white.render_flag, 5);
        assert_eq!(white.slots[0].max_height, 15.0);
        assert_eq!(white.slots[0].rot_start, 180);
        // `EF_AURABLADE`'s lifetime is exactly 60, so `TexturedQuadRingSized`'s
        // `duration <= 60` tail zeroes the alpha and staggers the slots by
        // `index * -5`.
        assert_eq!(white.slots[0].alpha_b, 0);
        assert_eq!(white.slots[1].alpha_b, 0);
        assert_eq!(white.slots[2].alpha_b, 0);
        assert_eq!(white.slots[3].alpha_b, 0);
        assert_eq!(white.slots[0].process, 0);
        assert_eq!(white.slots[1].process, -5);
        assert_eq!(white.slots[2].process, -10);
        assert_eq!(white.slots[3].process, -15);
        assert_eq!(white.slots[3].max_height, 12.0);
        assert_eq!(white.slots[0].alpha_t, 22);
        assert_eq!(white.slots[3].rot_start, 90);

        let yellow = &runtime.primitives[1].state;
        assert_eq!(yellow.texture, Some("effect\\ring_yellow.tga"));
        assert_eq!(yellow.slots[0].alpha_b, 0);
        assert_eq!(yellow.slots[0].process, 0);
        assert_eq!(yellow.slots[3].process, -15);
    }

    #[test]
    fn the_ring_helper_is_frame_zero_only() {
        let mut body = AuraBlade;
        let mut runtime = runtime(&body, 3);

        step(&mut body, &mut runtime);
        step(&mut body, &mut runtime);

        assert_eq!(runtime.primitives.len(), 2);
    }

    #[test]
    fn super_slash_fan_arms_one_slot_with_the_five_super_textures() {
        let mut body = Gumgang;
        let mut runtime = runtime(&body, 11);

        body.on_process(&mut runtime);

        let primitive = &runtime.primitives[0].state;
        assert_eq!(primitive.count, PrimitiveId::FloatingBubbles);
        assert_eq!(primitive.count.id(), 35);
        assert_eq!(primitive.render_flag, 5);
        assert_eq!(primitive.total_texture, 5);
        assert_eq!(primitive.texture, Some("effect\\super1.bmp"));
        assert_eq!(primitive.texture_frames.len(), 5);
        assert_eq!(primitive.slots[0].flag1[4], 0);
        assert_eq!(primitive.slots[1].life, 0);
        assert_eq!(primitive.slots[0].distance, 3.5);
        assert_eq!(primitive.slots[0].height[0], 10.0);
        assert_eq!(primitive.slots[0].height[1], 70.0);
        assert_eq!(primitive.slots[0].height[5], 130.0);
        assert_eq!(primitive.slots[0].height[9], 190.0);
        assert_eq!(primitive.slots[0].height[13], 250.0);
    }

    #[test]
    fn super_slash_fan_rng_order_is_angle_then_index_then_offset() {
        let mut body = Gumgang;
        let mut runtime = runtime(&body, 42);

        let mut expected = EffectRng::new(42);

        // Twelve draws, block by block: y offset (10), texture index (5),
        // orbit angle (360).
        let mut draws = [(0i32, 0i32, 0i32); 4];
        for draw in &mut draws {
            let y = expected.clamped_random(10);
            let index = expected.clamped_random(5);
            let angle = expected.clamped_random(360);

            *draw = (y, index, angle);
        }

        body.on_process(&mut runtime);

        let slot = &runtime.primitives[0].state.slots[0];

        assert_eq!(slot.vec_b_pre[1], -(draws[0].0) as f32);
        assert_eq!(slot.height[2], draws[0].1 as f32);
        assert_eq!(slot.height[3], draws[0].2 as f32);

        assert_eq!(slot.vec_b_now[1], -(draws[1].0) as f32);
        assert_eq!(slot.height[6], draws[1].1 as f32);
        assert_eq!(slot.height[7], draws[1].2 as f32);

        assert_eq!(slot.vec_t_pre[1], -(draws[2].0) as f32);
        assert_eq!(slot.height[10], draws[2].1 as f32);
        assert_eq!(slot.height[11], draws[2].2 as f32);

        assert_eq!(slot.vec_t_now[1], -(draws[3].0) as f32);
        assert_eq!(slot.height[14], draws[3].1 as f32);
        assert_eq!(slot.height[15], draws[3].2 as f32);
    }

    #[test]
    fn steelbody_arms_four_slots() {
        let mut body = SteelBody;
        let mut runtime = runtime(&body, 5);

        body.on_process(&mut runtime);

        let primitive = &runtime.primitives[0].state;
        assert_eq!(primitive.slots.iter().filter(|slot| slot.life == 1).count(), 4);
        assert_eq!(primitive.slots[3].distance, 3.5);
        assert_eq!(primitive.slots[3].height[13], 250.0);
        assert_eq!(primitive.slots[3].flag1[4], 0);
    }

    /// `SuperSlashFan(4, 0)`'s primitive state, field by field
    /// (`RagEffect.cpp:16964-17023`, `:24181-24183`).
    #[test]
    fn steelbody_fields_match_super_slash_fan() {
        let mut body = SteelBody;
        let mut runtime = runtime(&body, 5);

        body.on_process(&mut runtime);

        let primitive = &runtime.primitives[0].state;

        assert_eq!(primitive.count, PrimitiveId::FloatingBubbles);
        assert_eq!(primitive.count.id(), 35);
        // `renderFlag = 0; renderFlag |= 1u; renderFlag |= 5u;`
        assert_eq!(primitive.render_flag, 5);
        assert_eq!(primitive.total_texture, 5);
        assert_eq!(primitive.texture, Some("effect\\super1.bmp"));
        assert_eq!(primitive.texture_frames, vec![
            "effect\\super1.bmp",
            "effect\\super2.bmp",
            "effect\\super3.bmp",
            "effect\\super4.bmp",
            "effect\\super5.bmp",
        ]);
        assert_eq!(primitive.duration, 99_999_999);

        for slot in &primitive.slots {
            assert_eq!(slot.life, 1);
            assert_eq!(slot.distance, 3.5);
            assert_eq!(slot.flag1[4], 0);

            // Four `(height 10, alpha, textureIndex < 5, orbit angle < 360)`
            // records: alpha 70/130/190/250.
            assert_eq!(
                (slot.height[0], slot.height[4], slot.height[8], slot.height[12]),
                (10.0, 10.0, 10.0, 10.0)
            );
            assert_eq!(
                (slot.height[1], slot.height[5], slot.height[9], slot.height[13]),
                (70.0, 130.0, 190.0, 250.0)
            );
            for index in [2usize, 6, 10, 14] {
                assert!((0.0..5.0).contains(&slot.height[index]), "texture index {}", slot.height[index]);
            }
            for index in [3usize, 7, 11, 15] {
                assert!((0.0..360.0).contains(&slot.height[index]), "orbit angle {}", slot.height[index]);
            }
            // `vecBPre/vecBNow/vecTPre/vecTNow.y = -ClampedRandomAtZero(10)`.
            for y in [slot.vec_b_pre[1], slot.vec_b_now[1], slot.vec_t_pre[1], slot.vec_t_now[1]] {
                assert!((-9.0..=0.0).contains(&y), "vertical offset {y}");
            }
        }
    }

    /// `RenderOrbitIcons`'s exact quad geometry for the Steel Body fan
    /// (`EffectPrimitiveRender.cpp:4764-4897`, dispatch `:491-493` with
    /// `(1.0f, 100, 100, 255)`).
    ///
    /// Each icon is a `distance * scale = 3.5` half-diagonal square placed on a
    /// `g_effectIconRadius = 4.0` orbit; its corners are the 45/135/225/315
    /// diagonals, swapped to `(2, 1, 4, 3)` when the record faces away from the
    /// camera. The icon quad is therefore `3.5 * sqrt(2) = 4.95` world units on
    /// a side (less than one cell), not a texture-sized billboard.
    #[test]
    fn steelbody_orbit_icon_geometry_is_exact() {
        let mut body = SteelBody;
        let mut runtime = runtime(&body, 5);
        body.on_process(&mut runtime);

        let state = runtime.primitive_mut(0);
        // The camera-less `triangles` path reads the stored `FaceCamera`
        // angles; zero keeps `Tei_TowardScreen` an identity.
        state.slots[0].alpha_t = 0;
        state.slots[1].alpha_t = 0;

        // Deterministic icon records: two that keep the corner order and two
        // that swap it (`icon_faces_us` differs from the texture test).
        for slot in &mut state.slots {
            slot.height[3] = 10.0;
            slot.height[7] = 100.0;
            slot.height[11] = 250.0;
            slot.height[15] = 300.0;
            slot.height[2] = 0.0;
            slot.height[6] = 4.0;
            slot.height[10] = 0.0;
            slot.height[14] = 1.0;
            slot.vec_b_pre = [0.0, -1.0, 0.0];
            slot.vec_b_now = [0.0, -2.0, 0.0];
            slot.vec_t_pre = [0.0, -3.0, 0.0];
            slot.vec_t_now = [0.0, -4.0, 0.0];
            slot.height[20] = 0.0;
        }

        let triangles = render_slots::triangles(state);

        // 4 live slots x 4 icon records x 2 triangles.
        assert_eq!(triangles.len(), 32);

        let angles = Angles::shared();
        let cos = |degrees: usize| angles.raw_cosine(degrees);
        let sin = |degrees: usize| angles.raw_sine(degrees);

        // `distance * scale = 3.5 * 1.0`.
        let radius = 3.5f32;
        let corner = |angle: usize| [radius * cos(angle), 0.0, radius * sin(angle)];

        // The four icons of slot 0: (orbit angle, texture slot, owner y,
        // first triangle order). `icon_faces_us` is `10 < 70`, `100 < 70`,
        // `250` neither, `300 > 270`.
        let icons: [(usize, &'static str, f32, bool); 4] = [
            (10, "effect\\super1.bmp", -1.0, true),
            (100, "effect\\super5.bmp", -2.0, true),
            (250, "effect\\super1.bmp", -3.0, false),
            (300, "effect\\super2.bmp", -4.0, false),
        ];

        for (icon, &(angle, texture, owner_y, keep_order)) in icons.iter().enumerate() {
            let triangle = &triangles[icon * 2];
            let offset = [4.0 * cos(angle), owner_y, 4.0 * sin(angle)];
            let add = |c: [f32; 3]| [c[0] + offset[0], c[1] + offset[1], c[2] + offset[2]];

            let expected = if keep_order {
                [add(corner(45)), add(corner(135)), add(corner(225))]
            } else {
                [add(corner(135)), add(corner(45)), add(corner(315))]
            };

            let close = |left: [f32; 3], right: [f32; 3]| {
                (left[0] - right[0]).abs() < 1e-4 && (left[1] - right[1]).abs() < 1e-4 && (left[2] - right[2]).abs() < 1e-4
            };
            for corner_index in 0..3 {
                assert!(
                    close(triangle.corners[corner_index], expected[corner_index]),
                    "icon {icon} corner {corner_index}: {:?} != {:?}",
                    triangle.corners[corner_index],
                    expected[corner_index]
                );
            }

            assert_eq!(triangle.texture, Some(texture));
            // `RenderOrbitIcons(wtm, 1.0f, 100, 100, 255)` and the per-record
            // alpha 70/130/190/250.
            let alpha = 70 + icon as u32 * 60;
            assert_eq!(triangle.argb, (alpha << 24) | (100 << 16) | (100 << 8) | 255);
        }

        // One icon is a `3.5 * sqrt(2)` world-unit square (the table's
        // integer-grade cos/sin, so compare against the same table).
        let side = {
            let dx = corner(45)[0] - corner(135)[0];
            let dz = corner(45)[2] - corner(135)[2];
            (dx * dx + dz * dz).sqrt()
        };
        assert!((side - 4.9497).abs() < 1e-2, "icon side {side}");
        assert!(side < 5.0, "an icon must stay under one 5-unit cell, got {side}");
    }

    #[test]
    fn gumgangnpc_is_the_two_slot_fan_with_value_one() {
        let mut body = GumgangNpc;
        let mut runtime = runtime(&body, 23);

        body.on_process(&mut runtime);

        let primitive = &runtime.primitives[0].state;
        assert_eq!(primitive.count, PrimitiveId::FloatingBubbles);
        assert_eq!(primitive.slots.iter().filter(|slot| slot.life == 1).count(), 2);
        assert_eq!(primitive.slots[0].flag1[4], 1);
        assert_eq!(primitive.slots[1].flag1[4], 1);
        assert_eq!(primitive.slots[1].height[13], 250.0);
    }

    #[test]
    fn double_gumgang_is_the_nested_spread_sheet() {
        let mut body = DoubleGumGang;
        let mut runtime = runtime(&body, 31);

        body.on_process(&mut runtime);

        let primitive = &runtime.primitives[0].state;
        assert_eq!(primitive.count, PrimitiveId::DoubleGumGang);
        assert_eq!(primitive.count.id(), 99);
        assert_eq!(primitive.render_flag, 5);
        assert_eq!(primitive.size, 0.0);
        assert_eq!(primitive.total_texture, 5);
        assert_eq!(primitive.texture_frames.len(), 5);
        assert_eq!(primitive.slots.iter().filter(|slot| slot.life == 1).count(), 2);
        assert_eq!(primitive.slots[0].distance, 2.0);
        assert_eq!(primitive.slots[0].process, 0);
        assert_eq!(primitive.slots[0].height[1], 40.0);
        assert_eq!(primitive.slots[0].height[5], 80.0);
        assert_eq!(primitive.slots[0].height[9], 120.0);
        assert_eq!(primitive.slots[0].height[13], 160.0);
        assert_eq!(primitive.slots[0].flag1[4], 2);
        assert_eq!(primitive.slots[1].flag1[4], 2);
        assert_eq!(primitive.slots[1].distance, 2.0);
    }

    #[test]
    fn super_slash_spread_rng_order_matches_the_fan() {
        let mut body = DoubleGumGang3;
        let mut runtime = runtime(&body, 42);

        let mut expected = EffectRng::new(42);
        let mut draws = [(0i32, 0i32, 0i32); 8];

        for draw in &mut draws {
            let y = expected.clamped_random(10);
            let index = expected.clamped_random(5);
            let angle = expected.clamped_random(360);

            *draw = (y, index, angle);
        }

        body.on_process(&mut runtime);

        for (slot_index, slot_draws) in draws.chunks(4).enumerate() {
            let slot = &runtime.primitives[0].state.slots[slot_index];

            for (record, (_y, index, angle)) in slot_draws.iter().enumerate() {
                let base = record * 4;

                assert_eq!(slot.height[base + 2], *index as f32, "slot {slot_index} record {record}");
                assert_eq!(slot.height[base + 3], *angle as f32, "slot {slot_index} record {record}");
            }

            assert_eq!(slot.vec_b_pre[1], -(slot_draws[0].0) as f32);
            assert_eq!(slot.vec_b_now[1], -(slot_draws[1].0) as f32);
            assert_eq!(slot.vec_t_pre[1], -(slot_draws[2].0) as f32);
            assert_eq!(slot.vec_t_now[1], -(slot_draws[3].0) as f32);
        }

        // `EF_DOUBLEGUMGANG3` is the blue table (`value == 0`).
        assert_eq!(runtime.primitives[0].state.slots[0].flag1[4], 0);
    }

    #[test]
    fn gumgang2_is_wave_spin_with_accel_99() {
        let mut body = Gumgang2;
        let mut runtime = runtime(&body, 9);

        step(&mut body, &mut runtime);

        let primitive = &runtime.primitives[0].state;
        assert_eq!(primitive.count, PrimitiveId::WaveSpin);
        assert_eq!(primitive.count.id(), 31);
        assert_eq!(primitive.texture, Some("effect\\ring_yellow.tga"));
        assert_eq!(primitive.accel, 99.0);
        assert_eq!(primitive.render_flag, 5);
        assert_eq!(primitive.slots[2].rot_start, 183);
        assert_eq!(primitive.slots[3].rot_start, 273);
        assert_eq!(primitive.slots[3].distance, 4.1);
        assert_eq!(primitive.slots[3].rise_angle, 79);
        assert_eq!(primitive.slots[0].alpha_t, 0);
    }

    #[test]
    fn gumgang2_adds_the_ground_shockwave_and_shakes_the_screen() {
        let mut body = Gumgang2;
        let mut runtime = runtime(&body, 9);

        step(&mut body, &mut runtime);

        assert_eq!(runtime.shakes, 1);
        assert_eq!(runtime.primitives.len(), 10, "one wave-spin + nine shockwave groups");
        let wave = &runtime.primitives[1].state;
        assert_eq!(wave.count, PrimitiveId::ChemicalProtection);
        assert_eq!(wave.texture, Some("effect\\ring_yellow.tga"));
        assert_eq!(wave.slots[0].rise_angle, 0);
        assert_eq!(wave.slots[3].rise_angle, 30);
        assert_eq!(runtime.primitives[9].state.slots[3].rise_angle, 350);
    }

    #[test]
    fn begin_asura_arms_rings_at_zero_and_the_first_column_at_one() {
        let mut body = BeginAsura;
        let mut runtime = runtime(&body, 13);

        body.on_process(&mut runtime);

        // Frame 0: two rings + the spell wave.
        assert_eq!(runtime.sounds, vec!["effect\\EF_BeginSpell.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 2);
        assert_eq!(runtime.primitives[0].state.count, PrimitiveId::Casting3d);
        assert_eq!(runtime.primitives[0].state.slots[0].max_height, 25.0);
        assert_eq!(runtime.primitives[0].state.slots[0].alpha_b, 180);

        runtime.frame = 1;
        body.on_process(&mut runtime);

        // Frame 1: the first column, pristine.
        assert_eq!(runtime.primitives.len(), 3);
        let column = &runtime.primitives[2].state;
        assert_eq!(column.count, PrimitiveId::AsuraCasting);
        assert_eq!(column.count.id(), 68);
        assert_eq!(column.texture, Some("effect\\asura3.tga"));
        assert_eq!(column.texture_frames.len(), 3);
        assert_eq!(column.duration, 1000);
        assert_eq!(column.slots[0].process, -40);
        assert_eq!(column.slots[1].process, -20);
        assert_eq!(column.slots[2].process, 0);
        assert_eq!(column.slots[2].flag1[0], 2);
        assert_eq!(column.slots[0].height[0], -6.0);
        assert_eq!(column.slots[0].flag1[4], 0);
        assert_eq!(column.slots[0].distance, 18.0);

        runtime.frame = 21;
        body.on_process(&mut runtime);

        // Frame 21: the second column.
        assert_eq!(runtime.primitives.len(), 4);
        let column = &runtime.primitives[3].state;
        assert_eq!(column.texture, Some("effect\\asura4.tga"));
        assert_eq!(column.slots[0].process, -40);
        assert_eq!(column.slots[1].process, -60);
        assert_eq!(column.slots[2].process, -80);
        assert_eq!(column.slots[0].height[0], 6.0);
    }

    #[test]
    fn begin_asura_11_uses_the_second_texture_set() {
        let mut body = BeginAsura11;
        let mut runtime = runtime(&body, 13);

        body.on_process(&mut runtime);
        runtime.frame = 1;
        body.on_process(&mut runtime);

        let column = &runtime.primitives[2].state;
        assert_eq!(column.texture, Some("effect\\asura13.tga"));
        assert_eq!(column.texture_secondary, Some("effect\\asura12.tga"));
        assert_eq!(column.slots[0].distance, 24.0);
        assert_eq!(column.slots[0].height[0], -8.0);
        assert_eq!(column.slots[0].flag1[4], 3);
    }

    #[test]
    fn begin_asura_icons_fire_at_frame_zero_with_their_hanmoon_texture() {
        for (id, texture) in [
            (EffectId::Beginasura1, "effect\\hanmoon1.tga"),
            (EffectId::Beginasura2, "effect\\hanmoon2.tga"),
            (EffectId::Beginasura3, "effect\\hanmoon3.tga"),
            (EffectId::Beginasura4, "effect\\hanmoon4.tga"),
            (EffectId::Beginasura5, "effect\\hanmoon7.tga"),
            (EffectId::Beginasura6, "effect\\hanmoon5.tga"),
            (EffectId::Beginasura7, "effect\\hanmoon6.tga"),
        ] {
            let mut body = spawn(id).unwrap();
            let mut runtime = runtime(body.as_ref(), 17);

            body.on_process(&mut runtime);

            let primitive = &runtime.primitives[0].state;
            assert_eq!(primitive.count, PrimitiveId::AsuraCasting);
            assert_eq!(primitive.texture, Some(texture));
            assert_eq!(primitive.slots[0].life, 1);
            assert_eq!(primitive.slots[0].flag1[4], 1);
            assert_eq!(primitive.slots[0].distance, 18.0);
            assert_eq!(primitive.slots[0].height[0], 0.0);
            assert_eq!(primitive.slots[0].flag1[0], 0);
        }
    }

    #[test]
    fn asura_kanji_texture_sets_are_exact() {
        assert_eq!(ASURA_KANJI_COLUMNS, [
            ["effect\\asura3.tga", "effect\\asura2.tga", "effect\\asura1.tga"],
            ["effect\\asura4.tga", "effect\\asura5.tga", "effect\\asura6.tga"],
        ]);
        assert_eq!(ASURA_KANJI_COLUMNS_MONSTER, [
            ["effect\\asura13.tga", "effect\\asura12.tga", "effect\\asura11.tga"],
            ["effect\\asura14.tga", "effect\\asura15.tga", "effect\\asura16.tga"],
        ]);

        // Drive both Asura bodies and pin the actual `state.textures[]` slots,
        // not just the constants: group 0 at frame 1, group 1 at frame 21.
        for (id, columns) in [
            (EffectId::Beginasura, ASURA_KANJI_COLUMNS),
            (EffectId::Beginasura11, ASURA_KANJI_COLUMNS_MONSTER),
        ] {
            let mut body = spawn(id).unwrap();
            let mut runtime = runtime(body.as_ref(), 5);

            body.on_process(&mut runtime);
            runtime.frame = 1;
            body.on_process(&mut runtime);

            let first = &runtime.primitives.last().unwrap().state;
            assert_eq!(first.texture, Some(columns[0][0]), "{id:?}");
            assert_eq!(first.texture_secondary, Some(columns[0][1]), "{id:?}");
            assert_eq!(first.texture_tertiary, Some(columns[0][2]), "{id:?}");
            assert_eq!(first.texture_frames, columns[0].to_vec(), "{id:?}");

            runtime.frame = 21;
            body.on_process(&mut runtime);

            let second = &runtime.primitives.last().unwrap().state;
            assert_eq!(second.texture, Some(columns[1][0]), "{id:?}");
            assert_eq!(second.texture_secondary, Some(columns[1][1]), "{id:?}");
            assert_eq!(second.texture_tertiary, Some(columns[1][2]), "{id:?}");
            assert_eq!(second.texture_frames, columns[1].to_vec(), "{id:?}");
        }
    }

    /// `CopyVector(render.position, parent->render.position)` - the official
    /// `AsuraStrike` runs it on every frame before the gates, so the frame-21
    /// column follows a caster that moved since frame 1.
    #[test]
    fn asura_kanji_columns_anchor_to_the_casters_frame_position() {
        let mut body = BeginAsura;
        let mut runtime = runtime(&body, 5);

        runtime.position = [0.0, 0.0, 0.0];
        body.on_process(&mut runtime);

        runtime.parent_position = Some([10.0, 2.0, 30.0]);
        runtime.frame = 1;
        body.on_process(&mut runtime);

        let first = runtime.primitives.len() - 1;
        // Primitive state is official (Y-down) simulation space; the parent
        // anchor is port-space (Y-up), so its +2 becomes -2.
        assert_eq!(runtime.primitives[first].state.position, [10.0, -2.0, 30.0]);

        runtime.parent_position = Some([14.0, 2.0, 38.0]);
        runtime.frame = 21;
        body.on_process(&mut runtime);

        let second = runtime.primitives.len() - 1;
        assert_eq!(runtime.primitives[second].state.position, [14.0, -2.0, 38.0]);
        // The frame-1 column is not re-anchored by later frames.
        assert_eq!(runtime.primitives[first].state.position, [10.0, -2.0, 30.0]);
    }

    #[test]
    fn copy_parent_position_is_a_no_op_without_a_parent() {
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 5);
        runtime.position = [1.0, 2.0, 3.0];

        runtime.copy_parent_position();
        assert_eq!(runtime.position, [1.0, 2.0, 3.0]);

        runtime.parent_position = Some([4.0, 5.0, 6.0]);
        runtime.copy_parent_position();
        assert_eq!(runtime.position, [4.0, 5.0, 6.0]);
    }

    /// The seven `EF_BEGINASURA1..7` glyphs set `flag1[4] = 1`, which makes
    /// `PrimAsuraCasting` run `FollowMaster` every frame. The port reads the
    /// caster position through [`EffectRuntime::copy_parent_position`]; this
    /// pins both the frame-0 launch and the later follow.
    #[test]
    fn hanmoon_kanji_follows_the_caster() {
        let mut body = BeginAsuraIcon::new(ASURA_ICON_TEXTURES[0]);
        let mut runtime = runtime(&body, 5);

        runtime.position = [0.0, 0.0, 0.0];
        body.on_process(&mut runtime);
        assert_eq!(runtime.primitives[0].state.position, [0.0, 0.0, 0.0]);

        runtime.parent_position = Some([5.0, 0.0, 7.0]);
        runtime.frame = 1;
        body.on_process(&mut runtime);
        runtime.process_primitives();

        assert_eq!(runtime.primitives[0].state.position, [5.0, 0.0, 7.0]);
    }

    /// `RenderHaloLayers` layer geometry for the Asura kanji column: ten
    /// concentric quads per slot at radius `distance + (10 - i) * 3`, x offset
    /// `height[0]`, lifted 30 units and translated by `render.position`.
    #[test]
    fn asura_kanji_halo_layer_geometry_is_exact() {
        let mut body = BeginAsura;
        let mut runtime = runtime(&body, 5);
        runtime.parent_position = Some([100.0, 5.0, -40.0]);
        runtime.frame = 1;
        body.on_process(&mut runtime);

        let index = runtime.primitives.len() - 1;
        let state = runtime.primitive_mut(index);

        // Camera-less `triangles` uses the stored `FaceCamera` angles; zero
        // keeps `Tei_TowardScreen` an identity.
        state.slots[0].alpha_t = 0;
        state.slots[1].alpha_t = 0;
        for layer in 1..=10usize {
            state.slots[0].height[layer] = 100.0;
        }

        let triangles = render_slots::triangles(state);

        // Only slot 0 has non-zero heights, so it emits ten layer quads.
        assert_eq!(triangles.len(), 20);
        assert_eq!(triangles[0].texture, Some("effect\\asura3.tga"));

        // The slot vectors are official (Y-down): the parent anchor
        // `[100, 5, -40]` is `[100, -5, -40]` in that space.
        let origin = [100.0, -5.0, -40.0];
        let height0 = -6.0;
        let distance = 18.0;

        // `for (int i = 10; i >= 1; --i)`: the first quad is the innermost
        // layer (radius 18) and `triangles[18]` the outermost (radius 45).
        assert_eq!(triangles[0].corners, expected_halo_triangle(origin, height0, distance, 10));
        assert_eq!(triangles[18].corners, expected_halo_triangle(origin, height0, distance, 1));
    }

    /// One `RenderTeiRect(vec2, vec1, vec4, vec3)` first triangle of
    /// `RenderHaloLayers` with `FaceCamera` angles zero.
    fn expected_halo_triangle(origin: [f32; 3], height0: f32, distance: f32, layer: usize) -> [[f32; 3]; 3] {
        let angles = Angles::shared();
        let radius = distance + (10 - layer) as f32 * 3.0;
        let corner = |angle: usize| [height0 + radius * angles.raw_cosine(angle), -30.0, angles.raw_sine(angle) * radius];

        // Quad order is `vec2, vec1, vec4, vec3`; its first triangle is
        // `(vec2, vec1, vec4)` (`push_quad`'s `[0, 1, 2]`).
        let vec1 = corner(45);
        let vec2 = corner(135);
        let vec4 = corner(315);

        [vec2, vec1, vec4].map(|corner| [origin[0] + corner[0], origin[1] + corner[1], origin[2] + corner[2]])
    }

    #[test]
    fn textured_skill_icon_waits_for_the_level_frame() {
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 1);

        textured_skill_icon(&mut runtime, "effect\\hanmoon1.tga", 5, 0);
        assert!(runtime.primitives.is_empty());

        runtime.frame = 5;
        textured_skill_icon(&mut runtime, "effect\\hanmoon1.tga", 5, 0);

        assert_eq!(runtime.primitives.len(), 1);

        let slot = &runtime.primitives[0].state.slots[0];
        assert_eq!(slot.distance, 5.0);
        assert_eq!(slot.flag1[4], 4);
        // The `level > 0` icon branch has no ported skill-icon lookup.
        assert_eq!(runtime.primitives[0].state.texture, None);
    }
}
