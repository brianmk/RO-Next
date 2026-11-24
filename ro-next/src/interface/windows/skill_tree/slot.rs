use std::cell::Cell;

use ragnarok_packets::{SkillLevel, SkillType};
use ro_next_interface::MouseMode;
use ro_next_interface::element::store::{ElementStore, ElementStoreMut};
use ro_next_interface::element::{BaseLayoutInfo, Element};
use ro_next_interface::event::{ClickHandler, Event, EventQueue};
use ro_next_interface::layout::tooltip::TooltipExt;
use ro_next_interface::layout::{Icon, MouseButton, Resolvers, WindowLayout, with_single_resolver};
use ro_next_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use rust_state::{Path, State};

use super::{SkillTreeWindowState, SkillTreeWindowStatePathExt};
use crate::graphics::{Color, ScreenPosition, ShadowPadding};
use crate::input::{InputEvent, MouseInputMode};
use crate::interface::components::skill_box::draw_cooldown_overlay;
use crate::interface::resource::SkillSource;
use crate::loaders::OverflowBehavior;
use crate::renderer::LayoutExt;
use crate::state::skills::{LearnableSkill, LearnedSkill, SkillAcquisition};
use crate::state::theme::{InterfaceThemePathExt, SkillTreeThemePathExt};
use crate::state::{ClientState, client_theme};

struct LevelDisplay {
    level: Option<SkillLevel>,
    maximum_level: SkillLevel,
    string: Option<String>,
}

impl Default for LevelDisplay {
    fn default() -> Self {
        Self {
            level: None,
            maximum_level: SkillLevel(0),
            string: Default::default(),
        }
    }
}

impl LevelDisplay {
    fn update(&mut self, new_level: Option<SkillLevel>, new_maximum_level: SkillLevel) {
        if self.string.is_none() || new_level != self.level || new_maximum_level != self.maximum_level {
            self.string = match new_level {
                Some(level) => Some(format!("{}/{}", level.0, new_maximum_level.0)),
                None => Some(new_maximum_level.0.to_string()),
            };
            self.level = new_level;
            self.maximum_level = new_maximum_level;
        }
    }
}

/// Regular click handler.
struct SkillSlotClickHandler<A, B, C> {
    learnable_skill_path: A,
    learned_skill_path: B,
    window_state_path: C,
    source: SkillSource,
}

impl<A, B, C> SkillSlotClickHandler<A, B, C> {
    fn new(learnable_skill_path: A, learned_skill_path: B, window_state_path: C, source: SkillSource) -> Self {
        Self {
            learnable_skill_path,
            learned_skill_path,
            window_state_path,
            source,
        }
    }
}

impl<A, B, C> ClickHandler<ClientState> for SkillSlotClickHandler<A, B, C>
where
    A: Path<ClientState, LearnableSkill, false>,
    B: Path<ClientState, LearnedSkill, false>,
    C: Path<ClientState, SkillTreeWindowState>,
{
    fn handle_click(&self, state: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        // Unwrapping here is fine since we only register the handler if the slot has a
        // skill.
        let mut learnable_skill = state.try_get(&self.learnable_skill_path).unwrap().clone();

        if *state.get(&self.window_state_path.currently_skilling()) {
            if learnable_skill.acquisition != SkillAcquisition::Job
                || state
                    .try_get(&self.learned_skill_path)
                    .is_some_and(|learned_skill| !learned_skill.upgradable)
            {
                return;
            }

            let skill_id = learnable_skill.skill_id;

            queue.queue(InputEvent::DistributePointsForSkill { skill_id });
        } else {
            let learned_skill = state.try_get(&self.learned_skill_path);

            // Only skills the player has actually learned can be picked up.
            // Whether the skill can still be upgraded is irrelevant here: a
            // skill at its maximum level must still be usable from the hotbar.
            if learned_skill.is_none_or(|skill| skill.skill_level.0 == 0) {
                return;
            }

            if let Some(cast_level) = state
                .get(&self.window_state_path.chosen_skill_level())
                .get(&learnable_skill.skill_id)
                .or_else(|| learned_skill.map(|skill| &skill.skill_level))
                .cloned()
            {
                learnable_skill.maximum_level = cast_level;
            }

            queue.queue(Event::SetMouseMode {
                mouse_mode: MouseMode::Custom {
                    mode: MouseInputMode::MoveSkill {
                        skill: learnable_skill,
                        source: self.source,
                    },
                },
            });
        }
    }
}

/// Double click handler: uses the skill like double clicking it in the hotbar
/// does (self cast skills are cast immediately, skills that need a target
/// switch to the target selection mouse mode). Unlearned and passive skills are
/// ignored.
struct SkillSlotUseHandler<A, B> {
    learned_skill_path: A,
    window_state_path: B,
}

impl<A, B> SkillSlotUseHandler<A, B> {
    fn new(learned_skill_path: A, window_state_path: B) -> Self {
        Self {
            learned_skill_path,
            window_state_path,
        }
    }
}

impl<A, B> ClickHandler<ClientState> for SkillSlotUseHandler<A, B>
where
    A: Path<ClientState, LearnedSkill, false>,
    B: Path<ClientState, SkillTreeWindowState>,
{
    fn handle_click(&self, state: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        // While distributing skill points a double click selects the level like
        // a single click does; it must not use the skill.
        if *state.get(&self.window_state_path.currently_skilling()) {
            return;
        }

        let Some(learned_skill) = state.try_get(&self.learned_skill_path) else {
            return;
        };

        if learned_skill.skill_level.0 == 0 || learned_skill.skill_type == SkillType::Passive {
            return;
        }

        let skill_id = learned_skill.skill_id;
        let level = state
            .get(&self.window_state_path.chosen_skill_level())
            .get(&skill_id)
            .cloned()
            .unwrap_or(learned_skill.skill_level);

        queue.queue(InputEvent::UseSkill { skill_id, level });
    }
}

/// Right click handler: opens the skill description window at the mouse
/// position. The position is refreshed every frame while the slot is hovered.
struct SkillSlotDescriptionHandler<A, B> {
    learnable_skill_path: A,
    learned_skill_path: B,
    position: Cell<ScreenPosition>,
}

impl<A, B> SkillSlotDescriptionHandler<A, B> {
    fn new(learnable_skill_path: A, learned_skill_path: B) -> Self {
        Self {
            learnable_skill_path,
            learned_skill_path,
            position: Cell::new(ScreenPosition { left: 0.0, top: 0.0 }),
        }
    }
}

impl<A, B> ClickHandler<ClientState> for SkillSlotDescriptionHandler<A, B>
where
    A: Path<ClientState, LearnableSkill, false>,
    B: Path<ClientState, LearnedSkill, false>,
{
    fn handle_click(&self, state: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        let Some(learnable_skill) = state.try_get(&self.learnable_skill_path) else {
            return;
        };
        let learned_skill = state.try_get(&self.learned_skill_path);

        queue.queue(InputEvent::OpenSkillDescription {
            name: learnable_skill.skill_name.clone(),
            learned_level: learned_skill.map(|learned_skill| learned_skill.skill_level),
            maximum_level: learnable_skill.maximum_level,
            skill_type: learned_skill.map(|learned_skill| learned_skill.skill_type),
            spell_point_cost: learned_skill.map(|learned_skill| learned_skill.spell_point_cost),
            attack_range: learned_skill.map(|learned_skill| learned_skill.attack_range),
            upgradable: learned_skill.is_some_and(|learned_skill| learned_skill.upgradable),
            position: self.position.get(),
        });
    }
}

struct ChooseLowerClickHandler<A, B> {
    learned_skill_path: A,
    window_state_path: B,
}

impl<A, B> ChooseLowerClickHandler<A, B> {
    fn new(learned_skill_path: A, window_state_path: B) -> Self {
        Self {
            learned_skill_path,
            window_state_path,
        }
    }
}

impl<A, B> ClickHandler<ClientState> for ChooseLowerClickHandler<A, B>
where
    A: Path<ClientState, LearnedSkill, false>,
    B: Path<ClientState, SkillTreeWindowState>,
{
    fn handle_click(&self, state: &State<ClientState>, _: &mut EventQueue<ClientState>) {
        if let Some(learned_skill) = state.try_get(&self.learned_skill_path) {
            let skill_id = learned_skill.skill_id;
            let current_level = state
                .get(&self.window_state_path.chosen_skill_level())
                .get(&skill_id)
                .cloned()
                .unwrap_or(learned_skill.skill_level);

            let new_chosen_skill_level = SkillLevel(current_level.0.saturating_sub(1));

            state.update_value_with(self.window_state_path.chosen_skill_level(), move |chosen_skill_level| {
                chosen_skill_level.insert(skill_id, new_chosen_skill_level);
            });
        }
    }
}

struct ChooseHigherClickHandler<A, B> {
    learned_skill_path: A,
    window_state_path: B,
}

impl<A, B> ChooseHigherClickHandler<A, B> {
    fn new(learned_skill_path: A, window_state_path: B) -> Self {
        Self {
            learned_skill_path,
            window_state_path,
        }
    }
}

impl<A, B> ClickHandler<ClientState> for ChooseHigherClickHandler<A, B>
where
    A: Path<ClientState, LearnedSkill, false>,
    B: Path<ClientState, SkillTreeWindowState>,
{
    fn handle_click(&self, state: &State<ClientState>, _: &mut EventQueue<ClientState>) {
        if let Some(learned_skill) = state.try_get(&self.learned_skill_path) {
            let skill_id = learned_skill.skill_id;
            let current_level = state
                .get(&self.window_state_path.chosen_skill_level())
                .get(&skill_id)
                .cloned()
                .unwrap();

            let new_chosen_skill_level = SkillLevel(current_level.0.saturating_add(1));
            let maximum_skill_level = learned_skill.skill_level;

            state.update_value_with(self.window_state_path.chosen_skill_level(), move |chosen_skill_level| {
                if new_chosen_skill_level == maximum_skill_level {
                    chosen_skill_level.remove(&skill_id);
                } else {
                    chosen_skill_level.insert(skill_id, new_chosen_skill_level);
                }
            });
        }
    }
}

pub struct SkillSlot<A, B, C> {
    learnable_skill_path: A,
    learned_skill_path: B,
    window_state_path: C,
    click_handler: SkillSlotClickHandler<A, B, C>,
    description_handler: SkillSlotDescriptionHandler<A, B>,
    use_handler: SkillSlotUseHandler<B, C>,
    choose_lower_handler: ChooseLowerClickHandler<B, C>,
    choose_higher_handler: ChooseHigherClickHandler<B, C>,
    level_display: LevelDisplay,
}

impl<A, B, C> SkillSlot<A, B, C>
where
    A: Copy,
    B: Copy,
    C: Copy,
{
    pub fn new(learnable_skill_path: A, learned_skill_path: B, window_state_path: C, source: SkillSource) -> Self {
        Self {
            learnable_skill_path,
            learned_skill_path,
            window_state_path,
            click_handler: SkillSlotClickHandler::new(learnable_skill_path, learned_skill_path, window_state_path, source),
            description_handler: SkillSlotDescriptionHandler::new(learnable_skill_path, learned_skill_path),
            use_handler: SkillSlotUseHandler::new(learned_skill_path, window_state_path),
            choose_lower_handler: ChooseLowerClickHandler::new(learned_skill_path, window_state_path),
            choose_higher_handler: ChooseHigherClickHandler::new(learned_skill_path, window_state_path),
            level_display: LevelDisplay::default(),
        }
    }
}

impl<A, B, C> Element<ClientState> for SkillSlot<A, B, C>
where
    A: Path<ClientState, LearnableSkill, false>,
    B: Path<ClientState, LearnedSkill, false>,
    C: Path<ClientState, SkillTreeWindowState>,
{
    type LayoutInfo = BaseLayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            const ELEMENT_HEIGHT: f32 = 70.0;

            let area = resolver.with_height(ELEMENT_HEIGHT);

            if let Some(learnable_skill) = state.try_get(&self.learnable_skill_path) {
                let pending_skill_points_path = self.window_state_path.pending_skill_points();
                let pending_skill_points = state
                    .get(&pending_skill_points_path)
                    .iter()
                    .filter(|skill_id| **skill_id == learnable_skill.skill_id)
                    .count() as u16;

                let mut skill_level = state.try_get(&self.learned_skill_path).map(|skill| skill.skill_level);

                if pending_skill_points >= 1 {
                    skill_level.get_or_insert(SkillLevel(0)).0 += pending_skill_points;
                } else if let Some(chosen_skill_level) = state
                    .get(&self.window_state_path.chosen_skill_level())
                    .get(&learnable_skill.skill_id)
                {
                    skill_level = Some(*chosen_skill_level);
                }

                let highlighted_skill = *state.get(&self.window_state_path.highlighted_skill());
                let required_skill_level = highlighted_skill.and_then(|skill_id| learnable_skill.required_for_skills.get(&skill_id));

                match required_skill_level {
                    Some(required_skill_level) => self.level_display.update(skill_level, *required_skill_level),
                    None => self.level_display.update(skill_level, learnable_skill.maximum_level),
                }
            }

            Self::LayoutInfo { area }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        // TODO: Should this also be part of the theme?
        const SLOT_SIZE: f32 = 40.0;
        const ARROW_SIZE: f32 = 10.0;
        const ARROW_SPACING: f32 = 25.0;

        let sprite_area = layout_info.area.interior(
            SLOT_SIZE,
            SLOT_SIZE,
            HorizontalAlignment::Center { offset: 0.0, border: 0.0 },
            VerticalAlignment::Center { offset: 0.0 },
        );

        if let Some(skill) = state.try_get(&self.learnable_skill_path) {
            let is_hovered = sprite_area.check().run(layout);
            // Double clicks on the level arrows must keep selecting the level,
            // so the "use skill" handler is only registered when no arrow is
            // hovered.
            let mut over_arrow = false;
            let highlighted_skill = *state.get(&self.window_state_path.highlighted_skill());
            let learned_skill = state.try_get(&self.learned_skill_path);

            let pending_skill_points = state
                .get(&self.window_state_path.pending_skill_points())
                .iter()
                .filter(|skill_id| **skill_id == skill.skill_id)
                .count() as u16;
            let has_pending_points = pending_skill_points > 0;

            let required_skill_level = highlighted_skill.and_then(|skill_id| skill.required_for_skills.get(&skill_id));
            let should_be_highlighted = required_skill_level.is_some();
            let requirements_met = required_skill_level.is_some_and(|required_skill_level| {
                learned_skill.map(|skill| skill.skill_level.0).unwrap_or_default() + pending_skill_points >= required_skill_level.0
            });

            let outline_color = match should_be_highlighted {
                true => *state.get(&client_theme().skill_tree().requirement_color()),
                false => *state.get(&client_theme().skill_tree().hover_color()),
            };

            let outline = match is_hovered {
                _ if should_be_highlighted && !requirements_met => *state.get(&client_theme().skill_tree().slot_outline()),
                true => *state.get(&client_theme().skill_tree().slot_outline()),
                false => ShadowPadding::default(),
            };

            layout.add_rectangle(
                sprite_area,
                *state.get(&client_theme().skill_tree().slot_corner_diameter()),
                *state.get(&client_theme().skill_tree().slot_background_color()),
                outline_color,
                outline,
            );

            if let Some(learned_skill) = learned_skill {
                let chosen_skill_level = state
                    .get(&self.window_state_path.chosen_skill_level())
                    .get(&learned_skill.skill_id)
                    .cloned();
                let is_not_highest_level = chosen_skill_level.is_some();

                let text_color = match is_not_highest_level {
                    _ if should_be_highlighted => outline_color,
                    _ if has_pending_points => *state.get(&client_theme().skill_tree().pending_points_color()),
                    true => *state.get(&client_theme().skill_tree().lower_level_color()),
                    false => *state.get(&client_theme().skill_tree().points_color()),
                };

                layout.add_text(
                    layout_info.area,
                    self.level_display.string.as_ref().unwrap(),
                    *state.get(&client_theme().skill_tree().points_font_size()),
                    text_color,
                    *state.get(&client_theme().skill_tree().highlight_color()),
                    HorizontalAlignment::Center { offset: 0.0, border: 3.0 },
                    VerticalAlignment::Bottom { offset: 0.0 },
                    OverflowBehavior::Shrink,
                );

                let is_level_selectable = skill.can_select_level;

                if is_level_selectable && chosen_skill_level.unwrap_or(learned_skill.skill_level).0 > 1 {
                    let left_arrow_area = layout_info.area.interior(
                        ARROW_SIZE,
                        ARROW_SIZE,
                        HorizontalAlignment::Center {
                            offset: -ARROW_SPACING,
                            border: 0.0,
                        },
                        VerticalAlignment::Bottom { offset: 0.0 },
                    );

                    layout.add_icon(
                        left_arrow_area,
                        Icon::ArrowLeft,
                        *state.get(&client_theme().skill_tree().arrow_color()),
                    );

                    if left_arrow_area.check().run(layout) {
                        over_arrow = true;
                        layout.register_click_handler(MouseButton::Left, &self.choose_lower_handler);
                    }
                }

                if is_level_selectable && is_not_highest_level {
                    let right_arrow_area = layout_info.area.interior(
                        ARROW_SIZE,
                        ARROW_SIZE,
                        HorizontalAlignment::Center {
                            offset: ARROW_SPACING,
                            border: 0.0,
                        },
                        VerticalAlignment::Bottom { offset: 0.0 },
                    );

                    layout.add_icon(
                        right_arrow_area,
                        Icon::ArrowRight,
                        *state.get(&client_theme().skill_tree().arrow_color()),
                    );

                    if right_arrow_area.check().run(layout) {
                        over_arrow = true;
                        layout.register_click_handler(MouseButton::Left, &self.choose_higher_handler);
                    }
                }
            } else if should_be_highlighted {
                layout.add_text(
                    layout_info.area,
                    self.level_display.string.as_ref().unwrap(),
                    *state.get(&client_theme().skill_tree().points_font_size()),
                    outline_color,
                    *state.get(&client_theme().skill_tree().highlight_color()),
                    HorizontalAlignment::Center { offset: 0.0, border: 3.0 },
                    VerticalAlignment::Bottom { offset: 0.0 },
                    OverflowBehavior::Shrink,
                );
            } else if has_pending_points {
                layout.add_text(
                    layout_info.area,
                    self.level_display.string.as_ref().unwrap(),
                    *state.get(&client_theme().skill_tree().points_font_size()),
                    *state.get(&client_theme().skill_tree().pending_points_color()),
                    *state.get(&client_theme().skill_tree().highlight_color()),
                    HorizontalAlignment::Center { offset: 0.0, border: 3.0 },
                    VerticalAlignment::Bottom { offset: 0.0 },
                    OverflowBehavior::Shrink,
                );
            }

            layout.add_text(
                layout_info.area,
                &skill.skill_name,
                *state.get(&client_theme().skill_tree().name_font_size()),
                *state.get(&client_theme().skill_tree().name_text_color()),
                *state.get(&client_theme().skill_tree().highlight_color()),
                HorizontalAlignment::Center { offset: 0.0, border: 3.0 },
                VerticalAlignment::Top { offset: 0.0 },
                OverflowBehavior::Shrink,
            );

            let color = match learned_skill.is_some() || has_pending_points {
                true => Color::WHITE,
                false => *state.get(&client_theme().skill_tree().unlearned_skill_color()),
            };

            if let Some(actions) = &skill.actions
                && let Some(sprite) = &skill.sprite
            {
                // Skills don't all share the same icon size. The ACT frames can be
                // larger than the texture (Weapons Research does this), so the fit
                // scale has to use the largest frame clip and not just the texture
                // size, otherwise such icons overflow (and get clipped by) the slot.
                let sprite_scale = {
                    use cgmath::Vector2;

                    let mut maximum_dimension = 1.0f32;

                    for action in &actions.actions {
                        for motion in &action.motions {
                            for sprite_clip in &motion.sprite_clips {
                                let dimensions = sprite_clip
                                    .size
                                    .map(|size| Vector2::new(size.x as f32, size.y as f32))
                                    .or_else(|| {
                                        sprite.textures.get(sprite_clip.sprite_number as usize).map(|texture| {
                                            let size = texture.get_size();
                                            Vector2::new(size.width as f32, size.height as f32)
                                        })
                                    })
                                    .unwrap_or(Vector2::new(1.0, 1.0));

                                maximum_dimension = maximum_dimension.max(dimensions.x).max(dimensions.y);
                            }
                        }
                    }

                    sprite_area.width.min(sprite_area.height) / maximum_dimension
                };

                layout.add_sprite(sprite_area, actions, sprite, &skill.animation_state, color, sprite_scale);
            }

            draw_cooldown_overlay(state, layout, sprite_area, skill.skill_id, 1.0);

            if is_hovered {
                layout.register_click_handler(MouseButton::Left, &self.click_handler);

                // Right click opens the skill description at the mouse position.
                self.description_handler.position.set(layout.get_mouse_position());
                layout.register_click_handler(MouseButton::Right, &self.description_handler);

                if !over_arrow {
                    layout.register_click_handler(MouseButton::DoubleLeft, &self.use_handler);
                }

                struct SkillSlotTooltip;
                layout.add_tooltip(&skill.skill_name, SkillSlotTooltip.tooltip_id());

                state.update_value(self.window_state_path.highlighted_skill(), Some(skill.skill_id));
            }
        } else {
            layout.add_rectangle(
                sprite_area,
                *state.get(&client_theme().skill_tree().slot_corner_diameter()),
                Color::TRANSPARENT,
                *state.get(&client_theme().skill_tree().unoccupied_slot_color()),
                *state.get(&client_theme().skill_tree().unoccupied_slot_outline()),
            );
        }
    }
}
