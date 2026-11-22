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

/// The status effects of the local player.
///
/// The client state tracks the status effects of every entity in the view (the
/// server sends area status packets), but the buff bar only displays the
/// player's own effects.
fn own_effects(state: &State<ClientState>) -> Vec<&StatusEffect> {
    let own_entity_id = state.try_get(&this_entity()).map(|entity| entity.get_entity_id().0);

    state
        .follow(client_state().status_effects())
        .effects()
        .iter()
        .filter(|effect| Some(effect.entity_id()) == own_entity_id)
        .collect()
}

/// The icon grid of the buff bar.
#[derive(Default)]
struct BuffBarElement {
    /// Tooltip text per visible buff, refreshed every frame.
    tooltips: Vec<String>,
    /// Remaining-time overlay text per visible buff, refreshed every frame.
    remaining_texts: Vec<Option<String>>,
}

impl BuffBarElement {
    fn rows(effect_count: usize) -> usize {
        effect_count.div_ceil(ICONS_PER_ROW).max(1)
    }

    fn effect_tooltip(effect: &StatusEffect, client_tick: u64) -> String {
        let mut tooltip = match effect.remaining_in_seconds(client_tick) {
            Some(seconds) => {
                let minutes = seconds / 60;
                let seconds = seconds % 60;
                format!("{}\n{}:{:02} remaining", effect.name(), minutes, seconds)
            }
            None => effect.name().to_owned(),
        };

        // Status-specific values (elemental enchant, exp boosts, ...) are only
        // sent for a few statuses; they are shown when available.
        for line in effect.value_lines() {
            tooltip.push('\n');
            tooltip.push_str(&line);
        }

        tooltip
    }

    /// Short remaining time text drawn over the icon, e.g. `12` or `1:05`.
    fn remaining_text(effect: &StatusEffect, client_tick: u64) -> Option<String> {
        let seconds = effect.remaining_in_seconds(client_tick)?;

        match seconds >= 60 {
            true => Some(format!("{}:{:02}", seconds / 60, seconds % 60)),
            false => Some(seconds.to_string()),
        }
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
            let status_effects = state.follow(client_state().status_effects());
            let client_tick = status_effects.current_client_tick();
            let effects = own_effects(state);

            self.tooltips = effects.iter().map(|effect| Self::effect_tooltip(effect, client_tick)).collect();
            self.remaining_texts = effects.iter().map(|effect| Self::remaining_text(effect, client_tick)).collect();

            let rows = Self::rows(effects.len());
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

        let effects = own_effects(state);

        let background_color = *state.get(&theme().window().background_color());
        let border_color = *state.get(&theme().window().highlight_color());
        let name_color = *state.get(&theme().window().title_color());

        let tokens = client_theme().tokens();
        let color_tokens = tokens.color();
        let typography = tokens.typography();
        let overlay_text_color = *state.get(&color_tokens.text_overlay());
        let fallback_name_font_size = *state.get(&typography.font_xs());
        let remaining_font_size = *state.get(&typography.font_sm());

        for (index, effect) in effects.iter().enumerate() {
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

            if let Some(Some(remaining)) = self.remaining_texts.get(index) {
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
                && let Some(tooltip) = self.tooltips.get(index)
            {
                layout.add_tooltip(tooltip, BuffBarTooltip.tooltip_id());
                layout.set_hovered();
            }
        }
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
