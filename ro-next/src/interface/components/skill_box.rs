use ragnarok_packets::{SkillId, SkillLevel};
use ro_next_interface::MouseMode;
use ro_next_interface::element::store::{ElementStore, ElementStoreMut};
use ro_next_interface::element::{BaseLayoutInfo, Element};
use ro_next_interface::event::{ClickHandler, DropHandler, Event, EventQueue};
use ro_next_interface::layout::area::Area;
use ro_next_interface::layout::tooltip::TooltipExt;
use ro_next_interface::layout::{MouseButton, Resolvers, WindowLayout, with_single_resolver};
use ro_next_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use rust_state::{Path, State};

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::input::{InputEvent, MouseInputMode};
use crate::interface::resource::SkillSource;
use crate::loaders::OverflowBehavior;
use crate::renderer::LayoutExt;
use crate::settings::GameSettingsPathExt;
use crate::state::skills::{LearnableSkill, LearnedSkill};
use crate::state::theme::{
    ColorTokensPathExt, DesignTokensPathExt, GlobalThemePathExt, InterfaceThemePathExt, SkillTreeThemePathExt, TypographyTokensPathExt,
};
use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme};

/// Default size of a skill slot in interface pixels.
///
/// The `skill_box!` component exposes a `size` option so that a larger variant
/// can be used by the hotbar while every other window keeps the default size.
pub const DEFAULT_SKILL_BOX_SIZE: f32 = 40.0;

struct LevelDisplay {
    maximum_level: SkillLevel,
    string: Option<String>,
}

impl Default for LevelDisplay {
    fn default() -> Self {
        Self {
            maximum_level: SkillLevel(0),
            string: Default::default(),
        }
    }
}

impl LevelDisplay {
    fn update(&mut self, new_maximum_level: SkillLevel) {
        if self.string.is_none() || new_maximum_level != self.maximum_level {
            self.string = Some(new_maximum_level.0.to_string());
            self.maximum_level = new_maximum_level;
        }
    }
}

struct SkillBoxHandler<A> {
    skill_path: A,
    source: SkillSource,
}

impl<A> SkillBoxHandler<A> {
    fn new(skill_path: A, source: SkillSource) -> Self {
        Self { skill_path, source }
    }
}

impl<A> ClickHandler<ClientState> for SkillBoxHandler<A>
where
    A: Path<ClientState, LearnableSkill, false>,
{
    fn handle_click(&self, state: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        // Unwrapping here is fine since we only register the handler if the slot has a
        // skill.
        let skill = state.try_get(&self.skill_path).unwrap().clone();

        queue.queue(Event::SetMouseMode {
            mouse_mode: MouseMode::Custom {
                mode: MouseInputMode::MoveSkill {
                    skill,
                    source: self.source,
                },
            },
        });
    }
}

impl<P> DropHandler<ClientState> for SkillBoxHandler<P>
where
    P: Path<ClientState, LearnableSkill, false>,
{
    fn handle_drop(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>, mouse_mode: &MouseMode<ClientState>) {
        if let MouseMode::Custom {
            mode: MouseInputMode::MoveSkill { source, skill },
        } = mouse_mode
        {
            // Dropping a hotbar skill onto its own slot is a simple click,
            // which uses the skill like in the original client.
            if let (SkillSource::Hotbar { slot }, SkillSource::Hotbar { slot: destination_slot }) = (*source, self.source)
                && slot == destination_slot
            {
                queue.queue(InputEvent::UseSkillInSlot { slot });
                return;
            }

            queue.queue(InputEvent::MoveSkill {
                source: *source,
                destination: self.source,
                skill: skill.clone(),
            });
        }
    }
}

/// Draws the cooldown sweep and the remaining seconds on a skill slot.
///
/// The sweep is a translucent dark curtain that shrinks from the top as the
/// cooldown runs out; the number is the remaining time rounded up. Both the
/// hotbar and the skill tree use this helper so the display cannot drift apart.
/// Static strings for the remaining cooldown seconds.
///
/// `WindowLayout::add_text` borrows the text for the whole layout lifetime, so
/// the display text has to be a `&'static str`. A fixed table (one short
/// string per second, capped at 999) avoids allocating or leaking per frame.
fn cooldown_text(seconds: u32) -> &'static str {
    static TEXTS: std::sync::OnceLock<[&'static str; 1000]> = std::sync::OnceLock::new();

    let texts = TEXTS.get_or_init(|| std::array::from_fn(|second| Box::leak(second.to_string().into_boxed_str()) as &'static str));

    texts[(seconds.min(999)) as usize]
}

pub fn draw_cooldown_overlay(
    state: &State<ClientState>,
    layout: &mut WindowLayout<ClientState>,
    area: Area,
    skill_id: SkillId,
    size_scale: f32,
) {
    let cooldowns_path = client_state().skill_cooldowns();
    let cooldowns = state.get(&cooldowns_path);

    let Some(remaining) = cooldowns.remaining(skill_id) else {
        return;
    };

    let sweep_height = area.height * remaining.fraction;

    if sweep_height > 0.5 {
        let sweep_area = Area {
            left: area.left,
            top: area.top,
            width: area.width,
            height: sweep_height,
        };

        layout.add_rectangle(
            sweep_area,
            CornerDiameter::uniform(6.0 * size_scale),
            Color::rgba(0.0, 0.0, 0.0, 0.55),
            Color::TRANSPARENT,
            ShadowPadding::uniform(0.0),
        );
    }

    if *state.get(&client_state().game_settings().show_cooldown_numbers()) {
        let typography = client_theme().tokens().typography();
        let font_size = <crate::loaders::FontSize as ro_next_interface::application::FontSize>::scaled(
            state.get(&typography.font_subtitle()),
            size_scale,
        );

        layout.add_text(
            area,
            cooldown_text(remaining.seconds),
            font_size,
            *state.get(&client_theme().tokens().color().text_overlay()),
            *state.get(&client_theme().tokens().color().text_overlay_accent()),
            HorizontalAlignment::Center { offset: 0.0, border: 0.0 },
            VerticalAlignment::Center { offset: 0.0 },
            OverflowBehavior::Shrink,
        );
    }
}

pub struct SkillBox<A, B> {
    learnable_skill_path: A,
    learned_skill_path: B,
    handler: SkillBoxHandler<A>,
    level_display: LevelDisplay,
    source: SkillSource,
    /// Height (and width) of the square slot in interface pixels.
    size: f32,
    /// Whether the (maximum) skill level is drawn in the bottom right corner.
    /// The hotbar hides it; only the icon is shown there, like the official
    /// client.
    show_level: bool,
}

impl<A, B> SkillBox<A, B>
where
    A: Copy,
    B: Copy,
{
    /// This function is supposed to be called from a component macro
    /// and not intended to be called manually.
    #[inline(always)]
    pub fn component_new(learnable_skill_path: A, learned_skill_path: B, source: SkillSource, size: f32, show_level: bool) -> Self {
        Self {
            learnable_skill_path,
            learned_skill_path,
            handler: SkillBoxHandler::new(learnable_skill_path, source),
            level_display: LevelDisplay::default(),
            source,
            size: size.max(1.0),
            show_level,
        }
    }
}

impl<A, B> Element<ClientState> for SkillBox<A, B>
where
    A: Path<ClientState, LearnableSkill, false>,
    B: Path<ClientState, LearnedSkill, false>,
{
    type LayoutInfo = BaseLayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let area = resolver.with_height(self.size);

            if self.show_level
                && let Some(learnable_skill) = state.try_get(&self.learnable_skill_path)
            {
                self.level_display.update(learnable_skill.maximum_level);
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
        use ro_next_interface::prelude::*;

        let is_drop_target = match layout.get_mouse_mode() {
            MouseMode::Custom {
                mode: MouseInputMode::MoveSkill { source, .. },
            } => *source != self.source,
            _ => false,
        };

        let is_hovered = match is_drop_target {
            true => layout_info.area.check().any_mouse_mode().run(layout),
            false => layout_info.area.check().run(layout),
        };

        // The slot is designed for a 40 pixel box; a larger box (hotbar) scales
        // the corner radius, shadow and level text with it so nothing looks
        // lost in the enlarged card.
        let size_scale = (self.size / DEFAULT_SKILL_BOX_SIZE).clamp(1.0, 1.75);
        let color_tokens = client_theme().tokens().color();
        let skill_font_size = *state.get(&client_theme().tokens().typography().font_body());
        let level_font_size = <crate::loaders::FontSize as ro_next_interface::application::FontSize>::scaled(&skill_font_size, size_scale);
        let background_color = match !is_drop_target && is_hovered {
            true => *state.get(&color_tokens.state_hover()),
            false => *state.get(&color_tokens.surface_raised()),
        };

        layout.add_rectangle(
            layout_info.area,
            // Square slot with a small corner radius: the y rounded "pill" corners
            // used to leave large dark areas around the skill icon.
            CornerDiameter::uniform(6.0 * size_scale),
            background_color,
            *state.get(&color_tokens.border()),
            ShadowPadding::diagonal(2.0 * size_scale, 5.0 * size_scale),
        );

        if is_drop_target {
            let color = match is_hovered {
                true => *state.get(&client_theme().global().hovered_drop_area_color()),
                false => *state.get(&client_theme().global().drop_area_color()),
            };

            layout.add_rectangle(
                layout_info.area,
                *state.get(&theme().window().corner_diameter()),
                color.multiply_alpha(*state.get(&client_theme().global().fill_alpha())),
                color,
                *state.get(&client_theme().global().drop_area_outline()),
            );

            if is_hovered {
                // Since we are not in default mouse mode we need to mark the window as
                // hovered.
                layout.set_hovered();

                layout.register_drop_handler(&self.handler);
            }
        }

        if let Some(learnable_skill) = state.try_get(&self.learnable_skill_path) {
            let color = match state
                .try_get(&self.learned_skill_path)
                .is_some_and(|learned_skill| learned_skill.skill_level.0 >= learnable_skill.maximum_level.0)
            {
                true => Color::WHITE,
                false => *state.get(&client_theme().skill_tree().unlearned_skill_color()),
            };

            if let Some(actions) = &learnable_skill.actions
                && let Some(sprite) = &learnable_skill.sprite
            {
                // Scale the skill art to fill the whole square slot instead of drawing the
                // native icon size and leaving the dark background visible around it.
                let sprite_scale = sprite
                    .textures
                    .first()
                    .map(|texture| {
                        let size = texture.get_size();
                        let icon_size = size.width.max(size.height) as f32;

                        layout_info.area.width.min(layout_info.area.height) / icon_size
                    })
                    .unwrap_or(1.0);

                layout.add_sprite(
                    layout_info.area,
                    actions,
                    sprite,
                    &learnable_skill.animation_state,
                    color,
                    sprite_scale,
                );
            }

            draw_cooldown_overlay(state, layout, layout_info.area, learnable_skill.skill_id, size_scale);

            if is_hovered {
                layout.register_click_handler(MouseButton::Left, &self.handler);

                struct SkillBoxTooltip;
                layout.add_tooltip(&learnable_skill.skill_name, SkillBoxTooltip.tooltip_id());
            }

            if self.show_level {
                layout.add_text(
                    layout_info.area,
                    self.level_display.string.as_ref().unwrap(),
                    level_font_size,
                    *state.get(&color_tokens.text_overlay()),
                    *state.get(&color_tokens.text_overlay_accent()),
                    // TODO: Put this in the theme
                    HorizontalAlignment::Right {
                        offset: 3.0 * size_scale,
                        border: 3.0 * size_scale,
                    },
                    // TODO: Put this in the theme
                    VerticalAlignment::Bottom { offset: 3.0 * size_scale },
                    OverflowBehavior::Shrink,
                );
            }
        }
    }
}
