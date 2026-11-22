//! Buff/status effect bar showing the player's active status effects.
//!
//! The bar is a compact, frameless window with one icon per active buff. Each
//! icon shows the official status/skill icon from the client data (with a text
//! fallback when the data is missing), the remaining time and a tooltip with
//! the buff name. The window is opened when the first status effect starts and
//! closed again when the last one ends, so it is invisible when there are no
//! buffs.

use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{BaseLayoutInfo, Element};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::tooltip::TooltipExt;
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::State;

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::loaders::OverflowBehavior;
use crate::renderer::LayoutExt;
use crate::state::localization::LocalizationPathExt;
use crate::state::status_effects::StatusEffect;
use crate::state::theme::{ColorTokensPathExt, DesignTokensPathExt, InterfaceThemePathExt, InterfaceThemeType, TypographyTokensPathExt};
use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme, this_entity};

/// Size of one buff icon slot in interface pixels.
const ICON_SIZE: f32 = 34.0;
/// Gap between two icon slots.
const ICON_GAP: f32 = 3.0;
/// Number of icons per row before wrapping into another row.
const ICONS_PER_ROW: usize = 10;
/// Extra padding around the icon grid.
const BAR_PADDING: f32 = 2.0;

/// Width of the buff bar window (fixed so that the window does not jump around
/// while buffs are added/removed; the background is invisible anyway).
const BAR_WIDTH: f32 = BAR_PADDING * 2.0 + ICONS_PER_ROW as f32 * ICON_SIZE + (ICONS_PER_ROW - 1) as f32 * ICON_GAP;

/// Tooltip id used for the buff icons.
struct BuffBarTooltip;

/// The status effects of the local player, without allocating a second list.
fn own_effects<'a>(state: &'a State<ClientState>, own_entity_id: Option<u32>) -> impl Iterator<Item = &'a StatusEffect> + 'a {
    state
        .follow(client_state().status_effects())
        .effects()
        .iter()
        .filter(move |effect| Some(effect.entity_id()) == own_entity_id)
}

fn local_entity_id(state: &State<ClientState>) -> Option<u32> {
    state.try_get(&this_entity()).map(|entity| entity.get_entity_id().0)
}

/// Identity of a rendered buff label.
///
/// A label only has to be recomposed when one of these values changes; the
/// remaining time is part of the key because the countdown text is drawn on the
/// icon. This is what turns the per-frame `format!` churn into a no-op on
/// frames where the second did not change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BuffLabelKey {
    status_index: u16,
    remaining_seconds: Option<u32>,
    values: [i32; 3],
}

impl BuffLabelKey {
    fn new(effect: &StatusEffect, client_tick: u64) -> Self {
        Self {
            status_index: effect.status_index(),
            remaining_seconds: effect.remaining_in_seconds(client_tick),
            values: effect.values(),
        }
    }
}

/// Cached tooltip + remaining-time strings of one buff icon.
///
/// The strings are reused in place, so a buff that stays active for a minute
/// allocates its labels once instead of once per frame.
struct CachedBuffLabel {
    key: BuffLabelKey,
    tooltip: String,
    remaining_text: Option<String>,
}

impl CachedBuffLabel {
    fn new(effect: &StatusEffect, client_tick: u64) -> Self {
        let mut label = Self {
            key: BuffLabelKey::new(effect, client_tick),
            tooltip: String::new(),
            remaining_text: None,
        };
        label.update(effect, client_tick);
        label
    }

    fn matches(&self, key: BuffLabelKey) -> bool {
        self.key == key
    }

    fn update(&mut self, effect: &StatusEffect, client_tick: u64) {
        use std::fmt::Write;

        self.key = BuffLabelKey::new(effect, client_tick);

        self.tooltip.clear();
        self.tooltip.push_str(effect.name());

        if let Some(seconds) = self.key.remaining_seconds {
            let _ = write!(self.tooltip, "\n{}:{:02} remaining", seconds / 60, seconds % 60);
        }

        // Status-specific values (elemental enchant, exp boosts, ...) are only
        // sent for a few statuses; they are shown when available.
        for line in effect.value_lines() {
            self.tooltip.push('\n');
            self.tooltip.push_str(&line);
        }

        match self.key.remaining_seconds {
            Some(seconds) => {
                let remaining_text = self.remaining_text.get_or_insert_with(String::new);
                remaining_text.clear();
                if seconds >= 60 {
                    let _ = write!(remaining_text, "{}:{:02}", seconds / 60, seconds % 60);
                } else {
                    let _ = write!(remaining_text, "{seconds}");
                }
            }
            None => self.remaining_text = None,
        }
    }
}

/// The icon grid of the buff bar.
#[derive(Default)]
struct BuffBarElement {
    /// Cached labels, one per visible buff. Rebuilt in place, never collected
    /// into a temporary `Vec` or `String` per frame.
    labels: Vec<CachedBuffLabel>,
}

impl BuffBarElement {
    fn rows(effect_count: usize) -> usize {
        effect_count.div_ceil(ICONS_PER_ROW).max(1)
    }

    /// Refresh the cached labels for the current buff set; labels whose key did
    /// not change keep their existing strings (no allocation).
    fn refresh_labels(&mut self, state: &State<ClientState>, client_tick: u64, own_entity_id: Option<u32>) {
        let mut index = 0;

        for effect in own_effects(state, own_entity_id) {
            let key = BuffLabelKey::new(effect, client_tick);

            match self.labels.get_mut(index) {
                Some(label) if label.matches(key) => {}
                Some(label) => label.update(effect, client_tick),
                None => self.labels.push(CachedBuffLabel::new(effect, client_tick)),
            }

            index += 1;
        }

        self.labels.truncate(index);
    }
}

impl Element<ClientState> for BuffBarElement {
    type LayoutInfo = BaseLayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let client_tick = state.follow(client_state().status_effects()).current_client_tick();
            let own_entity_id = local_entity_id(state);

            self.refresh_labels(state, client_tick, own_entity_id);

            let rows = Self::rows(self.labels.len());
            let height = BAR_PADDING * 2.0 + rows as f32 * ICON_SIZE + (rows.saturating_sub(1)) as f32 * ICON_GAP;

            Self::LayoutInfo {
                area: resolver.with_height(height),
            }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        use korangar_interface::prelude::*;

        let own_entity_id = local_entity_id(state);

        let background_color = *state.get(&theme().window().background_color());
        let border_color = *state.get(&theme().window().highlight_color());
        let name_color = *state.get(&theme().window().title_color());

        let tokens = client_theme().tokens();
        let color_tokens = tokens.color();
        let typography = tokens.typography();
        let overlay_text_color = *state.get(&color_tokens.text_overlay());
        let fallback_name_font_size = *state.get(&typography.font_xs());
        let remaining_font_size = *state.get(&typography.font_sm());

        for (index, effect) in own_effects(state, own_entity_id).enumerate() {
            let column = index % ICONS_PER_ROW;
            let row = index / ICONS_PER_ROW;

            let area = Area {
                left: layout_info.area.left + BAR_PADDING + column as f32 * (ICON_SIZE + ICON_GAP),
                top: layout_info.area.top + BAR_PADDING + row as f32 * (ICON_SIZE + ICON_GAP),
                width: ICON_SIZE,
                height: ICON_SIZE,
            };

            layout.add_rectangle(
                area,
                CornerDiameter::uniform(5.0),
                background_color,
                border_color,
                ShadowPadding::diagonal(1.0, 3.0),
            );

            let icon_area = Area {
                left: area.left + 2.0,
                top: area.top + 2.0,
                width: area.width - 4.0,
                height: area.height - 4.0,
            };

            match effect.icon() {
                Some(texture) => layout.add_texture(icon_area, texture.clone(), Color::WHITE, true),
                None => layout.add_text(
                    area,
                    effect.name(),
                    fallback_name_font_size,
                    overlay_text_color,
                    name_color,
                    HorizontalAlignment::Center { offset: 0.0, border: 2.0 },
                    VerticalAlignment::Center { offset: 0.0 },
                    OverflowBehavior::Shrink,
                ),
            }

            let label = self.labels.get(index);

            if let Some(Some(remaining)) = label.map(|label| label.remaining_text.as_deref()) {
                layout.add_text(
                    area,
                    remaining,
                    remaining_font_size,
                    overlay_text_color,
                    name_color,
                    HorizontalAlignment::Center { offset: 0.0, border: 2.0 },
                    VerticalAlignment::Bottom { offset: 1.0 },
                    OverflowBehavior::Shrink,
                );
            }

            if area.check().run(layout)
                && let Some(tooltip) = label.map(|label| label.tooltip.as_str())
            {
                layout.add_tooltip(tooltip, BuffBarTooltip.tooltip_id());
                layout.set_hovered();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BuffBarElement, BuffLabelKey, ICONS_PER_ROW};

    #[test]
    fn label_key_changes_with_the_countdown_second() {
        let first = BuffLabelKey {
            status_index: 10,
            remaining_seconds: Some(61),
            values: [0; 3],
        };
        let same_second = BuffLabelKey {
            status_index: 10,
            remaining_seconds: Some(61),
            values: [0; 3],
        };
        let next_second = BuffLabelKey {
            status_index: 10,
            remaining_seconds: Some(60),
            values: [0; 3],
        };

        assert_eq!(first, same_second);
        assert_ne!(first, next_second);
    }

    #[test]
    fn label_key_changes_with_status_values() {
        let without_values = BuffLabelKey {
            status_index: 9,
            remaining_seconds: Some(30),
            values: [0; 3],
        };
        let with_values = BuffLabelKey {
            status_index: 9,
            remaining_seconds: Some(30),
            values: [1, 2, 3],
        };

        assert_ne!(without_values, with_values);
    }

    #[test]
    fn rows_wrap_at_the_configured_icon_count() {
        assert_eq!(BuffBarElement::rows(0), 1);
        assert_eq!(BuffBarElement::rows(1), 1);
        assert_eq!(BuffBarElement::rows(ICONS_PER_ROW), 1);
        assert_eq!(BuffBarElement::rows(ICONS_PER_ROW + 1), 2);
    }
}

/// Window class of the buff bar.
pub struct BuffBarWindow;

impl CustomWindow<ClientState> for BuffBarWindow {
    fn window_class() -> Option<crate::interface::windows::WindowClass> {
        Some(crate::interface::windows::WindowClass::BuffBar)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: client_state().localization().chat_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            skinned: false,
            title_height: 0.0,
            gaps: 0.0,
            minimum_width: BAR_WIDTH,
            maximum_width: BAR_WIDTH,
            minimum_height: 0.0,
            maximum_height: 200.0,
            elements: (BuffBarElement::default(),),
        }
    }
}
