//! Buff/status effect bar showing the player's active status effects.
//!
//! The buff bar is a completely transparent vertical strip on the right side of
//! the screen, below the minimap. It has no window frame, background or chrome
//! at all: only the status effect icons, their remaining time and the tooltip
//! are drawn, so the game world stays visible around them. Icons are stacked in
//! one column and the strip is positioned by the client (not by a cached window
//! position), so it stays on the right edge at every resolution and interface
//! scale. The window is opened when the first status effect of the local player
//! starts and closed again when the last one ends.

use ro_next_interface::element::store::{ElementStore, ElementStoreMut};
use ro_next_interface::element::{BaseLayoutInfo, Element};
use ro_next_interface::layout::area::Area;
use ro_next_interface::layout::tooltip::TooltipExt;
use ro_next_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use ro_next_interface::window::{CustomWindow, Window};
use rust_state::State;

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::loaders::OverflowBehavior;
use crate::renderer::LayoutExt;
use crate::state::localization::LocalizationPathExt;
use crate::state::status_effects::StatusEffect;
use crate::state::theme::{ColorTokensPathExt, DesignTokensPathExt, InterfaceThemePathExt, InterfaceThemeType, TypographyTokensPathExt};
use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme, this_entity};

/// Size of one buff icon in interface pixels.
const ICON_SIZE: f32 = 34.0;
/// Gap between two icons of the vertical strip.
const ICON_GAP: f32 = 3.0;
/// Width of the transparent status strip in interface pixels.
///
/// The strip is a single column of icons; the width is fixed so the icons do
/// not jump around while buffs are added or removed.
pub const BAR_STRIP_WIDTH: f32 = ICON_SIZE;
/// Maximum height of the strip in interface pixels.
///
/// The window only clips its content, so a very long buff list cannot grow
/// over the whole screen. About 24 icons fit into this height.
const BAR_STRIP_MAXIMUM_HEIGHT: f32 = 900.0;

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
    /// Number of icon rows of the vertical strip (one icon per row).
    fn rows(effect_count: usize) -> usize {
        effect_count.max(1)
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
            let height = rows as f32 * ICON_SIZE + rows.saturating_sub(1) as f32 * ICON_GAP;

            Self::LayoutInfo {
                area: resolver.with_height(height.min(BAR_STRIP_MAXIMUM_HEIGHT)),
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
        use ro_next_interface::prelude::*;

        let own_entity_id = local_entity_id(state);

        let tokens = client_theme().tokens();
        let color_tokens = tokens.color();
        let typography = tokens.typography();
        // The strip has no background, so the countdown and the fallback names
        // are drawn with a dark outline to stay readable on every map.
        let overlay_text_color = *state.get(&color_tokens.text_overlay());
        let outline_color = Color::rgba(0.0, 0.0, 0.0, 0.85);
        let fallback_name_font_size = *state.get(&typography.font_xs());
        let remaining_font_size = *state.get(&typography.font_sm());

        for (index, effect) in own_effects(state, own_entity_id).enumerate() {
            let area = Area {
                left: layout_info.area.left,
                top: layout_info.area.top + index as f32 * (ICON_SIZE + ICON_GAP),
                width: ICON_SIZE,
                height: ICON_SIZE,
            };

            // No background rectangle: the icon itself is the only thing drawn,
            // so the strip is fully transparent around and between the icons.
            match effect.icon() {
                Some(texture) => layout.add_texture(area, texture.clone(), Color::WHITE, true),
                None => layout.add_text(
                    area,
                    effect.name(),
                    fallback_name_font_size,
                    overlay_text_color,
                    outline_color,
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
                    outline_color,
                    HorizontalAlignment::Center { offset: 0.0, border: 2.0 },
                    VerticalAlignment::Bottom { offset: 1.0 },
                    OverflowBehavior::Shrink,
                );
            }

            // Keep the tooltip, but do not mark the window as hovered: the
            // transparent strip must not block clicks into the game world.
            if area.check().run(layout)
                && let Some(tooltip) = label.map(|label| label.tooltip.as_str())
            {
                layout.add_tooltip(tooltip, BuffBarTooltip.tooltip_id());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BuffBarElement, BuffLabelKey};

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
    fn rows_stack_one_icon_per_row() {
        assert_eq!(BuffBarElement::rows(0), 1);
        assert_eq!(BuffBarElement::rows(1), 1);
        assert_eq!(BuffBarElement::rows(4), 4);
        assert_eq!(BuffBarElement::rows(12), 12);
    }
}

/// Window class of the buff bar.
pub struct BuffBarWindow;

impl CustomWindow<ClientState> for BuffBarWindow {
    fn window_class() -> Option<crate::interface::windows::WindowClass> {
        Some(crate::interface::windows::WindowClass::BuffBar)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        window! {
            title: client_state().localization().chat_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            // Fully transparent overlay: no background, frame, shadow or title
            // bar. Only the status effect icons of the element are visible.
            skinned: false,
            title_height: 0.0,
            title_gap: 0.0,
            gaps: 0.0,
            border: 0.0,
            corner_diameter: CornerDiameter::uniform(0.0),
            background_color: Color::TRANSPARENT,
            shadow_color: Color::TRANSPARENT,
            shadow_padding: ShadowPadding::uniform(0.0),
            closable: false,
            resizable: false,
            minimum_width: BAR_STRIP_WIDTH,
            maximum_width: BAR_STRIP_WIDTH,
            minimum_height: 0.0,
            maximum_height: BAR_STRIP_MAXIMUM_HEIGHT,
            elements: (BuffBarElement::default(),),
        }
    }
}
