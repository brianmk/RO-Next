//! Faithful (1:1) port of the official client's effect subsystem.
//!
//! This module is the destination of the port described in
//! `default-workspace/ro-next-fx-audit/PLAN.md`. The rule for everything in
//! here is: **reproduce the decompiled code, not the visual impression of it**.
//! The official effects are per-frame integer state machines; their look *is*
//! the arithmetic, the frame gates and the lookup tables. Where the original
//! has a quirk (an off-by-one, an uninitialised variable, a swapped pair of
//! coordinates), the port keeps it and marks it `QUIRK:`.
//!
//! Structure:
//!
//! * [`clock`] - the fixed 16 ms simulation step (`Timer.cpp`).
//! * [`primitive`] - `CEffectPrim` state machine and the ported primitives.
//! * [`runtime`] - the per-frame driver that mirrors `EffectLists::Update ->
//!   CEffect::Update -> OnProcess -> ++stateCnt` and the separate primitive
//!   update pass.
//! * [`rag`] - ported effect bodies, one function per original method.
//!
//! Nothing here is wired into the live render path yet; that happens once the
//! primitive set the skill needs is ported (see the plan's W2/W6). Keeping it
//! hermetic is what makes the per-frame tests possible.
//!
//! Not wired into the live render path yet (see `PLAN.md` W2/W6), so the
//! module currently allows dead code.
#![allow(dead_code)]

pub mod angles;
pub mod clock;
pub mod holder;
pub mod math;
pub mod primitive;
pub mod rag;
pub mod rag_batch_a;
pub mod rag_batch_b;
pub mod rag_batch_c;
pub mod rag_batch_d;
pub mod rag_batch_e;
pub mod rag_batch_f;
pub mod rag_batch_g;
pub mod registry;
pub mod registry_batch_a;
pub mod registry_batch_b;
pub mod registry_batch_c;
pub mod registry_batch_d;
pub mod registry_batch_e;
pub mod registry_batch_f;
pub mod registry_batch_g;
pub mod registry_class_change;
pub mod registry_textured_aura;
pub mod render3d;
pub mod render_slots;
pub mod rng;
pub mod runtime;
pub mod skill_map;
pub mod sound_sink;
pub mod sprite_pass;
pub mod str_player;
pub mod str_render;
pub mod str_table;
pub mod visual;

/// `frameFreq = 1000 / 60` from `Timer.cpp:32`, in milliseconds.
pub const FRAME_FREQUENCY_MS: u64 = 1000 / 60;
