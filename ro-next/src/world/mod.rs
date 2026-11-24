mod action;
mod animation;
mod cameras;
mod cast_bar;
mod chat_bubble;
pub mod damage_font;
mod effect;
pub(crate) use self::effect::spirit_sphere_renderer;
mod entity;
mod fade_state;
mod ground_item;
mod guild_emblem;
mod head_marks;
mod item_details;
pub(crate) mod jobs;
mod library;
pub(crate) mod skills;
pub(crate) use self::library::{
    LayerAnchor, LayerKind, LayerPlacement, LayerPrimitive, LayerVertical, MAX_LAYERS_PER_PHASE, MonsterDatabase, MonsterEntry,
    NavigationData, NavigationDestination, SkillEffectLayer, SkillPhase, StatusAuraKind, StatusAuraSpec, StatusEffectCategory,
    StatusRefreshPolicy, StatusVisualKind, StatusVisualScope, StatusVisualSpec, StatusVisualState, caster_action_for_skill,
    curated_skill_sound_files, default_caster_action, generic_cast_sound_files, resolve_status_visual_state, skill_action_override,
    skill_cast_circle_color, skill_effect_layers, skill_effect_sound_files, skill_has_effect, skill_sound_files, status_effect_info,
    status_effect_refresh_policy, status_effect_value_lines, status_visual, validate_alias_data, validate_compiled_aliases,
    validate_compiled_skill_effects, validate_compiled_status_effects, validate_compiled_status_visuals, validate_skill_effect_data,
    validate_status_effect_data, validate_status_visual_data,
};
mod light;
mod map;
mod model;
mod object;
mod particles;
mod pathing;
mod robe;
mod shield;
mod skill_action;
mod skill_cast_effect;
mod skill_completion;
mod skill_name_text;
mod skill_sound;
mod sound;
pub(crate) mod target_policy;
mod video;
pub(crate) mod weapon;

use std::sync::Arc;

pub use self::action::*;
pub use self::animation::*;
pub use self::cameras::*;
pub use self::cast_bar::*;
pub use self::chat_bubble::*;
pub use self::damage_font::*;
pub use self::effect::*;
pub use self::entity::*;
pub use self::fade_state::*;
pub use self::ground_item::*;
pub use self::guild_emblem::*;
pub use self::head_marks::*;
pub use self::item_details::*;
pub use self::jobs::AppearanceAudit;
pub use self::library::*;
pub use self::light::*;
pub use self::map::*;
pub use self::model::*;
pub use self::object::*;
pub use self::particles::*;
pub use self::pathing::*;
pub use self::robe::*;
pub use self::shield::*;
pub use self::skill_action::*;
pub use self::skill_cast_effect::*;
pub use self::skill_completion::*;
pub use self::skill_name_text::*;
pub use self::skill_sound::*;
pub use self::skills::SkillsSummary;
pub(crate) use self::skills::{CLOUD_DEDUP_PHASE, validate as validate_skill_data, validate_compiled as validate_compiled_skills};
pub use self::sound::*;
pub use self::video::*;
use crate::graphics::Texture;

pub struct ResourceSetBuffer<K> {
    visible: Vec<K>,
}

impl<K> Default for ResourceSetBuffer<K> {
    fn default() -> Self {
        Self { visible: Vec::new() }
    }
}

impl<K> ResourceSetBuffer<K> {
    pub(super) fn create_set(&mut self, initializer: impl FnOnce(&mut Vec<K>)) -> ResourceSet<'_, K> {
        self.visible.clear();

        initializer(&mut self.visible);

        ResourceSet { visible: &self.visible }
    }
}

#[derive(Default)]
pub struct ResourceSet<'a, K> {
    visible: &'a [K],
}

impl<K> ResourceSet<'_, K> {
    pub(super) fn iterate_visible(&self) -> std::slice::Iter<'_, K> {
        self.visible.iter()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ResourceMetadata {
    pub texture: Option<Arc<Texture>>,
    pub name: String,
    /// Item stats evaluated from the client item description.
    pub stats: ItemStats,
    /// Raw item description lines from the client item data.
    pub description: Vec<String>,
    /// Resolved display names of the cards slotted into the item.
    ///
    /// Cards are part of the server packet, not of the item description, so
    /// they have to be resolved together with the rest of the item metadata.
    pub card_names: CardNames,
    /// Number of card slots of the item (`slotCount` in `iteminfo.lub`).
    pub slot_count: u8,
    /// Pre-composed tooltip text: composed item name, card slots, random
    /// options and the favorite mark.
    ///
    /// The tooltip is composed once when the item metadata is loaded so that
    /// the interface does not have to allocate it on every frame.
    pub tooltip_text: Arc<str>,
}
