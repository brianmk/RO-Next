//! Transient notification HUD overlay.
//!
//! A single transparent overlay at the top center of the screen that stacks
//! the currently visible notifications:
//!
//! * server broadcasts / announces (`ZC_BROADCAST`, 0x009A and `ZC_BROADCAST2`,
//!   0x01C3),
//! * item pickup toasts (`ZC_ITEM_PICKUP_ACK`, 0x0A37),
//! * level-up indicators (derived from the base/job level stat update),
//! * quest banners (`ZC_QUEST_NOTIFY`, 0x09F9).
//!
//! Entries are created by the network event loop, so the overlay never has to
//! parse packets itself.

use ro_next_interface::application::Size;
use ro_next_interface::element::Element;
use ro_next_interface::element::store::{ElementStore, ElementStoreMut};
use ro_next_interface::layout::area::Area;
use ro_next_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use ro_next_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use ro_next_interface::window::{CustomWindow, Window};
use rust_state::State;

use super::WindowClass;
use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::loaders::OverflowBehavior;
use crate::state::notifications::{HudNotification, NotificationKind};
use crate::state::theme::{ColorTokensPathExt, DesignTokensPathExt, InterfaceThemePathExt, InterfaceThemeType, TypographyTokensPathExt};
use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme};

/// Width of the notification overlay in interface pixels.
pub const NOTIFICATION_HUD_WIDTH: f32 = 520.0;
/// Maximum height before the oldest entries are simply not drawn.
const NOTIFICATION_HUD_MAXIMUM_HEIGHT: f32 = 260.0;
/// Horizontal/vertical padding of one notification row.
const ROW_PADDING: f32 = 5.0;
/// Gap between two notification rows.
const ROW_GAP: f32 = 3.0;

fn kind_background(kind: NotificationKind) -> Color {
    match kind {
        NotificationKind::Announce => Color::rgba(0.06, 0.06, 0.08, 0.82),
        NotificationKind::Pickup => Color::rgba(0.08, 0.18, 0.08, 0.82),
        NotificationKind::LevelUp => Color::rgba(0.24, 0.18, 0.04, 0.88),
        NotificationKind::Quest => Color::rgba(0.10, 0.10, 0.22, 0.86),
    }
}

fn kind_text_color(kind: NotificationKind) -> Color {
    match kind {
        NotificationKind::Announce => Color::rgb_u8(255, 232, 150),
        NotificationKind::Pickup => Color::rgb_u8(170, 255, 170),
        NotificationKind::LevelUp => Color::rgb_u8(255, 220, 90),
        NotificationKind::Quest => Color::rgb_u8(190, 210, 255),
    }
}

struct NotificationRow {
    text: String,
    height: f32,
    notification: HudNotification,
}

struct NotificationLayoutInfo {
    area: Area,
    rows: Vec<NotificationRow>,
}

#[derive(Default)]
struct NotificationHudElement;

impl Element<ClientState> for NotificationHudElement {
    type LayoutInfo = NotificationLayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let tokens = client_theme().tokens();
            let typography = tokens.typography();
            let color_tokens = tokens.color();
            let font_size = *state.get(&typography.font_body());
            let text_color = *state.get(&color_tokens.text_inverse());

            let mut total_height = 0.0;
            let mut rows = Vec::new();

            for notification in state.follow(client_state().notifications()).entries() {
                let (size, _) = resolver.get_text_dimensions(
                    &notification.text,
                    text_color,
                    text_color,
                    font_size,
                    HorizontalAlignment::Center {
                        offset: 0.0,
                        border: ROW_PADDING,
                    },
                    OverflowBehavior::LineBreak,
                );

                if total_height + size.height() + ROW_PADDING * 2.0 > NOTIFICATION_HUD_MAXIMUM_HEIGHT {
                    break;
                }

                if total_height != 0.0 {
                    total_height += ROW_GAP;
                }

                let height = size.height() + ROW_PADDING * 2.0;
                total_height += height;

                rows.push(NotificationRow {
                    text: notification.text.clone(),
                    height,
                    notification: notification.clone(),
                });
            }

            let area = resolver.with_height(total_height);

            NotificationLayoutInfo { area, rows }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let tokens = client_theme().tokens();
        let typography = tokens.typography();
        let font_size = *state.get(&typography.font_body());

        let mut offset = 0.0;
        for row in &layout_info.rows {
            if offset != 0.0 {
                offset += ROW_GAP;
            }

            let row_area = Area {
                left: layout_info.area.left,
                top: layout_info.area.top + offset,
                width: layout_info.area.width,
                height: row.height,
            };

            layout.add_rectangle(
                row_area,
                CornerDiameter::uniform(3.0),
                kind_background(row.notification.kind),
                Color::TRANSPARENT,
                ShadowPadding::uniform(0.0),
            );

            layout.add_text(
                row_area,
                &row.text,
                font_size,
                kind_text_color(row.notification.kind),
                Color::BLACK,
                HorizontalAlignment::Center {
                    offset: 0.0,
                    border: ROW_PADDING,
                },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::LineBreak,
            );

            offset += row.height;
        }
    }
}

/// The transparent notification overlay window.
pub struct NotificationHudWindow;

impl CustomWindow<ClientState> for NotificationHudWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::NotificationHud)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        window! {
            // No title: the card is just the notification rows; the literal
            // "Notifications" header was drawn over the top row.
            title: "",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            // Fully transparent overlay: only the notification rows are drawn.
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
            minimum_width: NOTIFICATION_HUD_WIDTH,
            maximum_width: NOTIFICATION_HUD_WIDTH,
            minimum_height: 0.0,
            maximum_height: NOTIFICATION_HUD_MAXIMUM_HEIGHT,
            elements: (NotificationHudElement,),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{kind_background, kind_text_color};
    use crate::state::notifications::NotificationKind;

    #[test]
    fn every_notification_kind_has_a_readable_text_color() {
        for kind in [
            NotificationKind::Announce,
            NotificationKind::Pickup,
            NotificationKind::LevelUp,
            NotificationKind::Quest,
        ] {
            let color = kind_text_color(kind);
            assert!(color.red + color.green + color.blue > 1.5, "{kind:?} text is too dark");

            let background = kind_background(kind);
            assert!(background.alpha >= 0.8, "{kind:?} background is too transparent");
        }
    }
}
