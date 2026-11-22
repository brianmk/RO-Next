use std::sync::Arc;

use korangar_interface::MouseMode;
use korangar_networking::InventoryItem;
use ragnarok_packets::{HotbarSlot, SkillId, SkillLevel, SkillType, TilePosition};

use crate::graphics::Texture;
use crate::interface::resource::{ItemSource, SkillSource};
use crate::loaders::Sprite;
use crate::state::ClientState;
use crate::state::skills::LearnableSkill;
use crate::world::{Actions, ResourceMetadata, SpriteAnimationState};

#[derive(Debug, Clone)]
pub enum MouseInputMode {
    RotateCamera,
    Walk {
        destination: TilePosition,
    },
    MoveItem {
        source: ItemSource,
        item: InventoryItem<ResourceMetadata>,
    },
    /// The player pressed the left mouse button on an item box. The actual drag
    /// (`MoveItem`) only starts after the button has been held for a moment or
    /// the mouse moved a few pixels, so a plain click never picks an item up.
    PrepareItemDrag {
        source: ItemSource,
        item: InventoryItem<ResourceMetadata>,
    },
    MoveSkill {
        source: SkillSource,
        skill: LearnableSkill,
    },
    /// The player clicked a skill (hotbar or skill tree) and is now selecting
    /// a target for it, like in the original client. The next left click in
    /// the world casts the skill.
    TargetSkill {
        /// The hotbar slot the skill was selected from, if any. Used to adjust
        /// the cast level with the mouse wheel like the original client.
        slot: Option<HotbarSlot>,
        /// Id of the skill to cast.
        skill_id: SkillId,
        skill_type: SkillType,
        /// The level to cast the skill at. Starts at the level the skill was
        /// selected with and can be adjusted with the mouse wheel while
        /// selecting a target.
        level: SkillLevel,
    },
}

impl From<MouseInputMode> for MouseMode<ClientState> {
    fn from(mode: MouseInputMode) -> Self {
        MouseMode::Custom { mode }
    }
}

pub enum Grabbed {
    Texture(Arc<Texture>),
    Action(Arc<Sprite>, Arc<Actions>, SpriteAnimationState),
}

pub trait MouseModeExt {
    fn is_rotating_camera(&self) -> bool;

    fn walk_destination(&self) -> Option<TilePosition>;

    fn is_grabbing(&self) -> bool;

    fn grabbed(&self) -> Option<Grabbed>;

    fn skill_target(&self) -> Option<(Option<HotbarSlot>, SkillId, SkillType, SkillLevel)>;

    /// Returns the item/source of a pending (not yet started) item drag.
    fn prepare_item_drag(&self) -> Option<(ItemSource, InventoryItem<ResourceMetadata>)>;
}

impl MouseModeExt for MouseMode<ClientState> {
    fn is_rotating_camera(&self) -> bool {
        matches!(self, MouseMode::Custom {
            mode: MouseInputMode::RotateCamera
        })
    }

    fn walk_destination(&self) -> Option<TilePosition> {
        match self {
            MouseMode::Custom {
                mode: MouseInputMode::Walk { destination },
            } => Some(*destination),
            _ => None,
        }
    }

    fn is_grabbing(&self) -> bool {
        matches!(
            self,
            MouseMode::Custom {
                mode: MouseInputMode::MoveItem { .. },
            } | MouseMode::Custom {
                mode: MouseInputMode::MoveSkill { .. },
            }
        )
    }

    fn grabbed(&self) -> Option<Grabbed> {
        match self {
            MouseMode::Custom {
                mode: MouseInputMode::MoveItem { item, .. },
            } => item.metadata.texture.as_ref().map(|texture| Grabbed::Texture(texture.clone())),
            MouseMode::Custom {
                mode: MouseInputMode::MoveSkill { skill, .. },
            } => Some(Grabbed::Action(
                skill.sprite.clone()?,
                skill.actions.clone()?,
                skill.animation_state.clone(),
            )),
            _ => None,
        }
    }

    fn prepare_item_drag(&self) -> Option<(ItemSource, InventoryItem<ResourceMetadata>)> {
        match self {
            MouseMode::Custom {
                mode: MouseInputMode::PrepareItemDrag { source, item },
            } => Some((*source, item.clone())),
            _ => None,
        }
    }

    fn skill_target(&self) -> Option<(Option<HotbarSlot>, SkillId, SkillType, SkillLevel)> {
        match self {
            MouseMode::Custom {
                mode:
                    MouseInputMode::TargetSkill {
                        slot,
                        skill_id,
                        skill_type,
                        level,
                    },
            } => Some((*slot, *skill_id, *skill_type, *level)),
            _ => None,
        }
    }
}
