//! prim 64 (`TexturedPulseKind`) - spawned by `CRagEffect::TexturedPulseKind`
//! (`RagEffect.cpp:11135-11207`).
//!
//! Like prim 66, the compiled client has **no motion case** for 64: the single
//! slot is configured at spawn and drawn by the renderer.

use super::EffectPrimitiveState;

/// No motion in the original (`EffectPrimitiveMotion.cpp` has no `case 64`).
pub fn on_process(_state: &mut EffectPrimitiveState) {}
