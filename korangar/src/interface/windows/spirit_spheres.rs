//! Spirit sphere counter overlay.
//!
//! The Champion skills "Dangerous Soul Collect" (Zen, `CH_SOULCOLLECT`) fill
//! all five spirit spheres at once and "Critical Explosion" (Fury,
//! `MO_EXPLOSIONSPIRITS`) consumes five of them while starting its buff. The
//! map server reports the resulting count with `ZC_SPIRITS` (0x01D0) /
//! `ZC_SPIRITS2` (0x01E1); the client used to ignore both packets, so using
//! these skills looked like it did nothing at all.
//!
//! This is a compact, frameless overlay with one pip per spirit sphere and a
//! tooltip with the exact count. It is opened while the local player has at
//! least one sphere and closed again when the last sphere is gone, so it does
//! not take up HUD space during normal play.

use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{BaseLayoutInfo, Element};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::tooltip::TooltipExt;
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::State;

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::loaders::OverflowBehavior;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::{ColorTokensPathExt, DesignTokensPathExt, InterfaceThemePathExt, InterfaceThemeType, TypographyTokensPathExt};
use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme, this_entity};

/// Size of one sphere pip in interface pixels.
const PIP_SIZE: f32 = 13.0;
/// Gap between two pips.
const PIP_GAP: f32 = 3.0;
/// Padding around the row of pips.
const BAR_PADDING: f32 = 3.0;
/// Number of pips drawn without an overflow label. The server caps the spirit
/// spheres at five for the Champion skills (a Raising Dragon soul link raises
/// the limit while it is active); the extra count is shown as `+N`.
const MAX_PIPS: usize = 5;
/// Fixed width of the overlay, so the pips don't shift while the count changes.
const BAR_WIDTH: f32 = BAR_PADDING * 2.0 + MAX_PIPS as f32 * PIP_SIZE + (MAX_PIPS - 1) as f32 * PIP_GAP;
/// Fixed height of the overlay.
const BAR_HEIGHT: f32 = BAR_PADDING * 2.0 + PIP_SIZE;
/// Fill color of an active spirit sphere.
const SPHERE_COLOR: Color = Color::rgb(0.36, 0.72, 1.0);

/// Tooltip id of the sphere counter.
struct SpiritSphereTooltip;

/// Purely visual element; the sphere count is read from the client state.
///
/// The overflow label and the tooltip are cached and rebuilt in
/// `create_layout_info` (which has `&mut self`), because the layout code must
/// not allocate per frame and may only borrow from `self` for the lifetime of
/// the layout.
#[derive(Default)]
pub struct SpiritSphereElement {
    amount: i16,
    overflow_text: Option<String>,
    tooltip_text: String,
}

impl SpiritSphereElement {
    fn amount(state: &State<ClientState>) -> i16 {
        state
            .try_get(&this_entity())
            .map(|entity| state.follow(client_state().spirit_orbs()).get(entity.get_entity_id()))
            .unwrap_or(0)
    }

    fn refresh(&mut self, state: &State<ClientState>) {
        let amount = Self::amount(state).max(0);

        if amount != self.amount {
            self.amount = amount;
        }

        if amount as usize > MAX_PIPS {
            self.overflow_text = Some(format!("+{}", amount as usize - MAX_PIPS));
        } else {
            self.overflow_text = None;
        }

        let tooltip = format!("Spirit spheres: {amount}\nDangerous Soul Collect/Zen fills 5, Critical Explosion/Fury consumes 5.");
        if self.tooltip_text != tooltip {
            self.tooltip_text = tooltip;
        }
    }
}

impl Element<ClientState> for SpiritSphereElement {
    type LayoutInfo = BaseLayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        self.refresh(state);

        with_single_resolver(resolvers, |resolver| Self::LayoutInfo {
            area: resolver.with_height(BAR_HEIGHT),
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

        let amount = self.amount;
        let filled = (amount as usize).min(MAX_PIPS);

        let background_color = *state.get(&theme().window().background_color());
        let border_color = *state.get(&theme().window().highlight_color());

        layout.add_rectangle(
            layout_info.area,
            CornerDiameter::uniform(6.0),
            background_color,
            border_color,
            ShadowPadding::diagonal(1.0, 3.0),
        );

        for index in 0..MAX_PIPS {
            let area = Area {
                left: layout_info.area.left + BAR_PADDING + index as f32 * (PIP_SIZE + PIP_GAP),
                top: layout_info.area.top + BAR_PADDING,
                width: PIP_SIZE,
                height: PIP_SIZE,
            };

            let color = match index < filled {
                true => SPHERE_COLOR,
                false => *state.get(&client_theme().tokens().color().spirit_sphere_empty()),
            };

            layout.add_rectangle(
                area,
                CornerDiameter::uniform(PIP_SIZE / 2.0),
                color,
                border_color,
                ShadowPadding::diagonal(0.0, 0.0),
            );
        }

        // More spheres than pips (only possible while a Raising Dragon soul
        // link is active): label the overflow on the last pip.
        if let Some(overflow) = self.overflow_text.as_deref() {
            let tokens = client_theme().tokens();
            let color_tokens = tokens.color();
            let typography = tokens.typography();
            let overlay_text_color = *state.get(&color_tokens.text_overlay());
            let font_size = *state.get(&typography.font_xs());

            let area = Area {
                left: layout_info.area.left + BAR_PADDING + (MAX_PIPS - 1) as f32 * (PIP_SIZE + PIP_GAP),
                top: layout_info.area.top + BAR_PADDING,
                width: PIP_SIZE,
                height: PIP_SIZE,
            };

            layout.add_text(
                area,
                overflow,
                font_size,
                overlay_text_color,
                Color::BLACK,
                HorizontalAlignment::Center { offset: 0.0, border: 0.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );
        }

        if layout_info.area.check().run(layout) {
            layout.add_tooltip(&self.tooltip_text, SpiritSphereTooltip.tooltip_id());
            layout.set_hovered();
        }
    }
}

/// Window class of the spirit sphere counter.
pub struct SpiritSphereWindow;

impl CustomWindow<ClientState> for SpiritSphereWindow {
    fn window_class() -> Option<crate::interface::windows::WindowClass> {
        Some(crate::interface::windows::WindowClass::SpiritSpheres)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: client_state().localization().hotbar_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            skinned: false,
            title_height: 0.0,
            gaps: 0.0,
            minimum_width: BAR_WIDTH,
            maximum_width: BAR_WIDTH,
            minimum_height: BAR_HEIGHT,
            maximum_height: BAR_HEIGHT,
            elements: (SpiritSphereElement::default(),),
        }
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::EntityId;

    use crate::state::spirit_orbs::SpiritOrbs;

    #[test]
    fn sphere_count_is_read_from_the_state() {
        let mut orbs = SpiritOrbs::default();
        orbs.set(EntityId(7), 5);
        assert_eq!(orbs.get(EntityId(7)), 5);

        // Consuming all spheres removes the entry, so the overlay hides.
        orbs.set(EntityId(7), 0);
        assert_eq!(orbs.get(EntityId(7)), 0);
    }
}
