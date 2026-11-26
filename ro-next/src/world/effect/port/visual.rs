//! Client-facing adapter for the official-effect port.
//!
//! [`PortedEffectHolder`] deliberately separates the simulation (the exact
//! 16 ms clock and the primitive state machines) from the client's effect list:
//! it is not an [`EffectBase`]. `PortedVisual` is the bridge. It owns one
//! holder, steps it with the wall-clock delta the client passes to every
//! effect, and forwards rendering to `holder.render`.
//!
//! Rendering support is what the holder implements: the
//! `PP_2DTEXTURE`/`PP_2DFLASH` screen quads anchored to the effect's world
//! position, the world-space quads of `PP_3DTEXTURE` (15) and
//! `PP_3DCROSSTEXTURE` (16) built by [`super::render3d`], the
//! `RenderSegmentSprites2` sprite pass of [`super::sprite_pass`] for the
//! `.spr` particle families, and the slot/record primitives of
//! [`super::render_slots`] (rings, cylinders, columns, fans, halo, wind, arc
//! walls and the `TexturedAura` glow of prim 82) as additive triangle lists.
//! The remaining primitive families (`PP_3DSPHERE`, the arc/sweep families and
//! the effect ids whose `Render*` member is not ported yet) are skipped rather
//! than guessed.
//!
//! The sprite pass needs the client's [`SpriteLoader`]/[`ActionLoader`]; they
//! are handed to the holder here so it can resolve the ACT frame count while
//! the effect is stepped and draw the sprite quads.

use std::collections::HashSet;
use std::sync::Arc;

use cgmath::Point3;

use super::holder::PortedEffectHolder;
use super::runtime::EffectRuntime;
use crate::loaders::{ActionLoader, ImageType, SpriteLoader, TextureLoader};
use crate::renderer::EffectRenderer;
use crate::world::effect::EffectBase;
use crate::world::{Camera, PointLightManager};

/// A live ported effect in the client's effect list.
pub struct PortedVisual {
    holder: PortedEffectHolder,
    /// `None` only in unit tests, which have no wgpu device. Rendering is a
    /// no-op without a loader; simulation and lifetime stay testable.
    texture_loader: Option<Arc<TextureLoader>>,
    /// Assets already requested from the loader. Ported bodies launch their
    /// primitives over time, so the texture list grows after construction.
    prefetched: HashSet<&'static str>,
    gets_deleted: bool,
}

impl PortedVisual {
    pub fn new(
        holder: PortedEffectHolder,
        texture_loader: Arc<TextureLoader>,
        sprite_loader: Arc<SpriteLoader>,
        action_loader: Arc<ActionLoader>,
    ) -> Self {
        let mut visual = Self {
            holder: holder.with_sprite_loaders(sprite_loader, action_loader),
            texture_loader: Some(texture_loader),
            prefetched: HashSet::new(),
            gets_deleted: false,
        };

        visual.prefetch_textures();

        visual
    }

    pub fn runtime(&self) -> &EffectRuntime {
        self.holder.runtime()
    }

    pub fn runtime_mut(&mut self) -> &mut EffectRuntime {
        self.holder.runtime_mut()
    }

    pub fn is_finished(&self) -> bool {
        self.gets_deleted || self.holder.is_finished()
    }

    /// Requests every texture the effect currently references.
    ///
    /// The holder renders through the loader's cache, so this is the same
    /// `get_or_load` path the other effects use (`HitFlash::spawn`,
    /// `CastCircle::new`); doing it during the update pass keeps the first
    /// visible frame from hitching on a disk read. A missing texture is not
    /// fatal: the primitive simply draws nothing, as with the descriptor
    /// effects.
    fn prefetch_textures(&mut self) {
        let Some(texture_loader) = &self.texture_loader else {
            return;
        };

        for path in self.holder.textures() {
            if !self.prefetched.insert(path) {
                continue;
            }

            if texture_loader.get_or_load(path, ImageType::Color).is_err() {
                crate::logging::log_debug!("ported effect: missing texture '{}'", path);
            }
        }
    }
}

impl EffectBase for PortedVisual {
    fn update(&mut self, _entities: &[crate::world::Entity], delta_time: f32) -> bool {
        if self.gets_deleted {
            return false;
        }

        let alive = self.holder.update(delta_time as f64);
        self.prefetch_textures();

        alive
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, _: &mut PointLightManager, _: &dyn Camera) {}

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        if self.gets_deleted {
            return;
        }

        let Some(texture_loader) = &self.texture_loader else {
            return;
        };

        // 2-D screen quads, the world-space `PP_3DTEXTURE`/
        // `PP_3DCROSSTEXTURE` quads and the `.spr` particle quads; the holder
        // owns the primitive dispatch.
        self.holder.render(renderer, camera, texture_loader);
    }

    fn culling_position(&self) -> Option<Point3<f32>> {
        let [x, y, z] = self.holder.runtime().position;

        Some(Point3::new(x, y, z))
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::EffectId;

    use super::*;

    fn visual(effect: EffectId) -> PortedVisual {
        PortedVisual {
            holder: PortedEffectHolder::new(effect, 1).expect("registered effect"),
            texture_loader: None,
            prefetched: HashSet::new(),
            gets_deleted: false,
        }
    }

    #[test]
    fn registered_effect_yields_a_live_visual() {
        let mut visual = visual(EffectId::Bash);

        assert!(!visual.is_finished());
        assert_eq!(visual.runtime().frame, 0);

        assert!(visual.update(&[], 0.016));
        assert_eq!(visual.runtime().frame, 1, "one 16 ms step");

        assert!(!visual.is_finished());
    }

    #[test]
    fn finishes_after_the_effect_lifetime() {
        let mut visual = visual(EffectId::Bash);

        // Bash is 40 steps long; `update(0.016)` consumes exactly one step.
        for _ in 0..40 {
            visual.update(&[], 0.016);
        }

        assert!(visual.is_finished());
        assert_eq!(visual.runtime().frame, 40);
        assert!(!visual.update(&[], 0.016), "stays finished");
    }

    #[test]
    fn update_accumulates_short_frames() {
        let mut visual = visual(EffectId::Bash);

        for _ in 0..5 {
            visual.update(&[], 0.01);
        }

        assert_eq!(visual.runtime().frame, 3, "50 ms = three 16 ms steps");
    }

    #[test]
    fn deletion_stops_updates() {
        let mut visual = visual(EffectId::Bash);

        visual.mark_for_deletion();

        assert!(visual.is_finished());
        assert!(!visual.update(&[], 0.016));
    }

    #[test]
    fn unported_effect_has_no_holder() {
        assert!(PortedEffectHolder::new(EffectId::Thunderstorm, 1).is_none());
    }
}
