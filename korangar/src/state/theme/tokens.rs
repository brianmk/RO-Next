//! Design tokens (v0) of the interface.
//!
//! The interface used to hardcode colors, font sizes, spacings and corner
//! radii directly in every window, which is why the default theme, the classic
//! Ragnarok Online skin and newly added windows kept drifting apart. This
//! module is the single place where those values live.
//!
//! The tokens are deliberately *semantic* (`text_primary`,
//! `surface_raised`, `state_danger`, `font_body`, ...) instead of
//! appearance-based (`dark_gray`, `orange`): a window asks for the role of a
//! value, the theme decides what it looks like. The built-in default (dark)
//! theme and the built-in classic theme provide their own token sets, and a
//! theme file can override every token because the whole block is part of
//! [`InterfaceTheme`](super::InterfaceTheme) and serialized with serde
//! defaults (old theme files without a `tokens` block still load and get the
//! default token set).
//!
//! This is v0: the highest-traffic windows are migrated, the remaining
//! hardcoded values are listed in `notes/korangar-p0-design-tokens-report.md`
//! and enforced by `scripts/ui-token-budget.sh` so the count can only go down.

use korangar_interface::element::StateElement;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

use crate::graphics::{Color, CornerDiameter};
use crate::loaders::FontSize;

/// Semantic color tokens.
///
/// Defaults describe the built-in dark interface, [`ColorTokens::classic`]
/// describes the light classic Ragnarok Online interface.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, RustState, StateElement)]
pub struct ColorTokens {
    // --- surfaces -----------------------------------------------------
    /// Base background of an opaque window.
    pub surface_base: Color,
    /// Slightly raised surface, e.g. item cards and list rows.
    pub surface_raised: Color,
    /// Translucent panel drawn inside a window (equipment stats panel).
    pub surface_panel: Color,
    /// Translucent overlay background (chat, compact HUD).
    pub surface_overlay: Color,
    /// Recessed surface (empty slots, sunken list backgrounds).
    pub surface_sunken: Color,
    /// Surface of a selected/active row.
    pub surface_selected: Color,
    /// Emphasized surface, e.g. a hovered character card.
    pub surface_highlight: Color,
    /// Outline/border of cards, panels and slots.
    pub border: Color,
    /// Thin separator line inside a panel.
    pub divider: Color,
    // --- text ---------------------------------------------------------
    /// Primary text on window surfaces.
    pub text_primary: Color,
    /// Secondary text (labels, hints).
    pub text_secondary: Color,
    /// Disabled text.
    pub text_disabled: Color,
    /// Highlighted/action text (usually orange in the dark theme, the classic
    /// client uses a dark blue).
    pub text_accent: Color,
    /// Values (numbers, prices, remaining times).
    pub text_value: Color,
    /// Text on an inverted (light-on-dark) surface.
    pub text_inverse: Color,
    /// Text on translucent overlay surfaces (chat, buff bar); readable in
    /// every theme, so it is the same in the classic token set.
    pub text_overlay: Color,
    /// Highlighted text on translucent overlay surfaces.
    pub text_overlay_accent: Color,
    // --- interaction states ------------------------------------------
    /// Hovered background.
    pub state_hover: Color,
    /// Pressed/active background.
    pub state_active: Color,
    /// Disabled background.
    pub state_disabled: Color,
    /// Error/danger color.
    pub state_danger: Color,
    /// Success color.
    pub state_success: Color,
    /// Warning color.
    pub state_warning: Color,
    /// Information color.
    pub state_info: Color,
    /// Favorite marker of an item.
    pub state_favorite: Color,
    /// Keyboard focus ring.
    pub focus_ring: Color,
    /// Selection color (marking text, selected list entries).
    pub selection: Color,
    /// Drop shadow of floating elements.
    pub shadow: Color,
    // --- item/skill slots --------------------------------------------
    /// Background of an empty equipment/skill slot.
    pub slot_background: Color,
    /// Background of a hovered slot.
    pub slot_hovered_background: Color,
    /// Background of a slot that is being dragged.
    pub slot_drag_background: Color,
    /// Background of a slot that is being dragged and hovered.
    pub slot_drag_hovered_background: Color,
    /// Border of an empty slot.
    pub slot_border: Color,
    /// Border of a filled slot.
    pub slot_filled_border: Color,
    /// Label of an empty slot.
    pub slot_label: Color,
    /// Background of the paper doll canvas.
    pub canvas_background: Color,
    // --- overlay markers ---------------------------------------------
    /// Refinement level text in the paper doll.
    pub refine_text: Color,
    /// Background pill behind the refinement level.
    pub refine_background: Color,
    // --- chat message classes ----------------------------------------
    /// Normal chat text.
    pub chat_text: Color,
    /// Broadcast message text.
    pub chat_broadcast: Color,
    /// Server message text.
    pub chat_server: Color,
    /// Informational message text.
    pub chat_information: Color,
    /// Error message text.
    pub chat_error: Color,
    /// Background of the focused chat input row (always dark).
    pub chat_input_focused_background: Color,
    // --- progress bars -----------------------------------------------
    /// Value text drawn inside a progress bar.
    pub bar_value_text: Color,
    // --- minimap -----------------------------------------------------
    /// Background of the minimap canvas (behind the map image).
    pub minimap_background: Color,
    /// Fallback player marker on the minimap (used only when the generated
    /// direction arrow texture is not available).
    pub minimap_marker_fallback: Color,
    /// Outline of a minimap marker (player fallback and server markers).
    pub minimap_marker_outline: Color,
    // --- HUD overlays ------------------------------------------------
    /// Empty spirit sphere of the Champion soul link counter.
    pub spirit_sphere_empty: Color,
    /// Fill of the party member health bar.
    pub party_health_fill: Color,
    /// Secondary text that is drawn on an always-dark overlay surface (minimap
    /// loading hint, debug overlays); it therefore stays light in every theme.
    pub overlay_secondary_text: Color,
    /// Faint white highlight of the debug/analysis overlays.
    pub overlay_highlight: Color,
    /// Hot (longest) frame bar of the profiler.
    pub profiler_bar_hot: Color,
    /// Muted hint text, e.g. the tip in the skill description window.
    pub text_hint: Color,
    /// Ghost text of the chat input line, drawn on the dark input background.
    pub chat_ghost_text: Color,
    // --- item info card slots ----------------------------------------
    /// Background of a card slot in the item info window.
    pub item_slot_background: Color,
    /// Border of a card slot in the item info window.
    pub item_slot_border: Color,
    /// Inset of a card slot (empty socket or unknown card).
    pub item_slot_inner: Color,
    /// Background of a card slot that the item does not have.
    pub item_slot_unavailable_background: Color,
    /// Border of a card slot that the item does not have.
    pub item_slot_unavailable_border: Color,
    /// Text of a card slot that the item does not have.
    pub item_slot_unavailable_text: Color,
    /// Name of a card in the item info window.
    pub item_slot_card_name: Color,
}

impl Default for ColorTokens {
    fn default() -> Self {
        Self {
            // surfaces
            surface_base: Color::monochrome_u8(30),
            surface_raised: Color::rgb_u8(40, 40, 40),
            surface_panel: Color::rgba_u8(0, 0, 0, 40),
            surface_overlay: Color::rgba_u8(0, 0, 0, 180),
            surface_sunken: Color::monochrome_u8(20),
            surface_selected: Color::rgb_u8(90, 90, 90),
            surface_highlight: Color::rgb_u8(110, 110, 110),
            border: Color::rgba_u8(0, 0, 0, 100),
            divider: Color::rgba_u8(255, 255, 255, 24),
            // text
            text_primary: Color::monochrome_u8(220),
            text_secondary: Color::monochrome_u8(190),
            text_disabled: Color::rgb_u8(130, 130, 130),
            text_accent: Color::rgb_u8(255, 160, 60),
            text_value: Color::rgb_u8(250, 230, 130),
            text_inverse: Color::rgb_u8(0, 0, 0),
            text_overlay: Color::rgb_u8(255, 255, 255),
            text_overlay_accent: Color::rgb_u8(255, 160, 60),
            // states
            state_hover: Color::rgb_u8(60, 60, 60),
            state_active: Color::rgb_u8(80, 80, 80),
            state_disabled: Color::rgb_u8(55, 55, 55),
            state_danger: Color::rgb_u8(220, 100, 100),
            state_success: Color::rgb_u8(67, 163, 83),
            state_warning: Color::rgb_u8(230, 200, 80),
            state_info: Color::rgb_u8(0, 129, 163),
            state_favorite: Color::rgb_u8(255, 200, 60),
            focus_ring: Color::rgb_u8(255, 175, 30),
            selection: Color::rgb_u8(0, 0, 160),
            shadow: Color::rgba_u8(0, 0, 0, 100),
            // slots
            slot_background: Color::rgba_u8(24, 24, 28, 150),
            slot_hovered_background: Color::rgb_u8(70, 70, 76),
            slot_drag_background: Color::rgb_u8(150, 150, 70),
            slot_drag_hovered_background: Color::rgb_u8(70, 150, 150),
            slot_border: Color::rgba_u8(120, 120, 130, 110),
            slot_filled_border: Color::rgba_u8(88, 130, 180, 200),
            slot_label: Color::rgba_u8(160, 160, 170, 130),
            canvas_background: Color::rgba_u8(0, 0, 0, 50),
            // overlay markers
            refine_text: Color::rgb_u8(255, 190, 80),
            refine_background: Color::rgba_u8(0, 0, 0, 170),
            // chat message classes
            chat_text: Color::rgb_u8(255, 255, 255),
            chat_broadcast: Color::rgb_u8(255, 255, 255),
            chat_server: Color::rgb_u8(255, 255, 255),
            chat_information: Color::rgb_u8(255, 255, 255),
            chat_error: Color::rgb_u8(255, 255, 255),
            chat_input_focused_background: Color::rgba(0.0, 0.0, 0.0, 0.8),
            bar_value_text: Color::rgba_u8(255, 255, 255, 235),
            // minimap
            minimap_background: Color::rgb_u8(22, 24, 28),
            minimap_marker_fallback: Color::rgb_u8(80, 255, 110),
            minimap_marker_outline: Color::rgba_u8(0, 0, 0, 200),
            // HUD overlays
            spirit_sphere_empty: Color::rgba_u8(24, 34, 46, 190),
            party_health_fill: Color::rgb_u8(84, 196, 104),
            overlay_secondary_text: Color::monochrome_u8(200),
            overlay_highlight: Color::rgba_u8(255, 255, 255, 100),
            profiler_bar_hot: Color::rgba_u8(255, 80, 0, 150),
            text_hint: Color::rgb_u8(160, 160, 160),
            chat_ghost_text: Color::rgba_u8(205, 205, 205, 230),
            // item info card slots
            item_slot_background: Color::rgba_u8(28, 28, 34, 140),
            item_slot_border: Color::rgba_u8(120, 120, 130, 120),
            item_slot_inner: Color::rgba_u8(0, 0, 0, 70),
            item_slot_unavailable_background: Color::rgba_u8(86, 86, 92, 70),
            item_slot_unavailable_border: Color::rgba_u8(120, 120, 126, 60),
            item_slot_unavailable_text: Color::rgba_u8(160, 160, 166, 90),
            item_slot_card_name: Color::rgba_u8(235, 235, 240, 220),
        }
    }
}

impl ColorTokens {
    /// Tokens of the built-in classic (light) Ragnarok Online interface.
    ///
    /// Only the tokens that are drawn on the light classic window surfaces
    /// differ; overlay text (chat, buff bar) and accent colors stay the same
    /// so that the classic theme keeps the game's familiar look.
    pub fn classic() -> Self {
        Self {
            surface_base: Color::rgb_u8(240, 240, 240),
            surface_raised: Color::rgb_u8(247, 247, 247),
            surface_panel: Color::rgba_u8(0, 0, 0, 24),
            surface_overlay: Color::rgba_u8(0, 0, 0, 160),
            surface_sunken: Color::rgb_u8(225, 225, 225),
            surface_selected: Color::rgb_u8(196, 209, 234),
            surface_highlight: Color::rgb_u8(225, 232, 245),
            border: Color::rgba_u8(0, 0, 0, 70),
            divider: Color::rgba_u8(0, 0, 0, 40),
            text_primary: Color::rgb_u8(0, 0, 0),
            text_secondary: Color::rgb_u8(40, 40, 60),
            text_disabled: Color::rgb_u8(130, 130, 130),
            text_value: Color::rgb_u8(120, 80, 0),
            text_inverse: Color::rgb_u8(255, 255, 255),
            state_hover: Color::rgb_u8(225, 225, 232),
            state_active: Color::rgb_u8(205, 205, 212),
            state_disabled: Color::rgb_u8(225, 225, 225),
            state_danger: Color::rgb_u8(160, 30, 30),
            selection: Color::rgb_u8(0, 0, 160),
            shadow: Color::rgba_u8(0, 0, 0, 80),
            slot_background: Color::rgba_u8(245, 245, 245, 170),
            slot_hovered_background: Color::rgb_u8(225, 225, 232),
            slot_drag_background: Color::rgb_u8(200, 190, 90),
            slot_drag_hovered_background: Color::rgb_u8(90, 170, 170),
            slot_border: Color::rgba_u8(90, 90, 100, 140),
            slot_filled_border: Color::rgba_u8(60, 100, 160, 220),
            slot_label: Color::rgba_u8(60, 60, 70, 200),
            canvas_background: Color::rgba_u8(255, 255, 255, 60),
            ..Self::default()
        }
    }
}

/// Semantic typography tokens.
///
/// The scale is a strict ladder; windows ask for a role instead of a magic
/// number. Interface scaling is applied on top of these values by the text
/// layouter, exactly like before.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, RustState, StateElement)]
pub struct TypographyTokens {
    /// Smallest text, e.g. the remaining seconds on a buff icon.
    pub font_xs: FontSize,
    /// Very small text, e.g. the buff stack count.
    pub font_sm: FontSize,
    /// Small caption, e.g. the HUD action buttons.
    pub font_caption: FontSize,
    /// Label text, e.g. the character overview header.
    pub font_label: FontSize,
    /// Body text of compact panels.
    pub font_body: FontSize,
    /// Default button text.
    pub font_button: FontSize,
    /// Larger body text (chat messages).
    pub font_body_lg: FontSize,
    /// Subtitle, e.g. shop item amounts.
    pub font_subtitle: FontSize,
    /// Large text, e.g. shop item names.
    pub font_lg: FontSize,
    /// Character names on the character selection screen.
    pub font_title: FontSize,
    /// Window titles / headings.
    pub font_heading: FontSize,
}

impl Default for TypographyTokens {
    fn default() -> Self {
        Self {
            font_xs: FontSize(9.0),
            font_sm: FontSize(10.0),
            font_caption: FontSize(10.5),
            font_label: FontSize(11.0),
            font_body: FontSize(12.0),
            font_button: FontSize(13.0),
            font_body_lg: FontSize(14.0),
            font_subtitle: FontSize(15.0),
            font_lg: FontSize(16.0),
            font_title: FontSize(18.0),
            font_heading: FontSize(20.0),
        }
    }
}

/// Spacing scale used for gaps, paddings and offsets.
///
/// `0..xxxl` is the generic scale; the named fields are the semantic values
/// that were repeated as literals in the windows before.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, RustState, StateElement)]
pub struct SpacingTokens {
    pub none: f32,
    pub xxs: f32,
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub xxl: f32,
    pub xxxl: f32,
    /// Gap between two chat messages.
    pub chat_message_gap: f32,
    /// Horizontal border of inline text (left/right offset).
    pub text_border: f32,
    /// Gap between two icon slots (buff bar).
    pub icon_gap: f32,
    /// Padding around the buff icon grid.
    pub bar_padding: f32,
    /// Padding between a card and its content.
    pub card_padding: f32,
}

impl Default for SpacingTokens {
    fn default() -> Self {
        Self {
            none: 0.0,
            xxs: 1.0,
            xs: 2.0,
            sm: 3.0,
            md: 4.0,
            lg: 5.0,
            xl: 6.0,
            xxl: 8.0,
            xxxl: 10.0,
            chat_message_gap: 5.0,
            text_border: 3.0,
            icon_gap: 3.0,
            bar_padding: 2.0,
            card_padding: 3.0,
        }
    }
}

/// Corner radii used by windows, cards and slots.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, RustState, StateElement)]
pub struct RadiusTokens {
    pub none: CornerDiameter,
    pub small: CornerDiameter,
    pub medium: CornerDiameter,
    pub large: CornerDiameter,
    pub extra_large: CornerDiameter,
    pub pill: CornerDiameter,
}

impl Default for RadiusTokens {
    fn default() -> Self {
        Self {
            none: CornerDiameter::uniform(0.0),
            small: CornerDiameter::uniform(2.0),
            medium: CornerDiameter::uniform(4.0),
            large: CornerDiameter::uniform(6.0),
            extra_large: CornerDiameter::uniform(14.0),
            pill: CornerDiameter::uniform(25.0),
        }
    }
}

/// The complete design token set of an interface theme.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, RustState, StateElement)]
pub struct DesignTokens {
    pub color: ColorTokens,
    pub typography: TypographyTokens,
    pub spacing: SpacingTokens,
    pub radius: RadiusTokens,
}

impl DesignTokens {
    /// Token set of the built-in classic (light) interface theme.
    pub fn classic() -> Self {
        Self {
            color: ColorTokens::classic(),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ColorTokens, DesignTokens, TypographyTokens};

    fn luminance(color: crate::graphics::Color) -> f32 {
        0.2126 * color.red + 0.7152 * color.green + 0.0722 * color.blue
    }

    /// The two text roles that every theme has to keep readable.
    #[test]
    fn text_tokens_have_enough_contrast_in_both_themes() {
        let themes = [("default", DesignTokens::default()), ("classic", DesignTokens::classic())];

        for (name, tokens) in themes {
            let primary = luminance(tokens.color.text_primary);
            let base = luminance(tokens.color.surface_base);
            let contrast = (primary - base).abs();
            assert!(
                contrast > 0.4,
                "{name}: primary text ({primary:.2}) on base surface ({base:.2}) has too little contrast"
            );

            // Overlay text is drawn on the dark chat overlay in every theme.
            assert!(
                luminance(tokens.color.text_overlay) > 0.7,
                "{name}: overlay text must stay light because the overlay is always dark"
            );
        }
    }

    #[test]
    fn default_tokens_are_the_default_of_the_struct() {
        let tokens = DesignTokens::default();
        assert_eq!(tokens.typography.font_body_lg, TypographyTokens::default().font_body_lg);
        assert_eq!(tokens.color.text_accent, ColorTokens::default().text_accent);
    }

    #[test]
    fn classic_tokens_differ_only_where_the_light_interface_needs_it() {
        let default = ColorTokens::default();
        let classic = ColorTokens::classic();

        // Text on window surfaces flips from light to dark in the light theme.
        assert!(luminance(default.text_primary) > 0.7);
        assert!(luminance(classic.text_primary) < 0.2);

        // The overlay text stays light in both themes (it is always drawn on the
        // dark chat overlay).
        assert_eq!(default.text_overlay, classic.text_overlay);

        // Accent colors are theme-independent.
        assert_eq!(default.text_accent, classic.text_accent);
    }

    /// The overlay and data-driven tokens that replaced the hardcoded window
    /// colors must be identical in both built-in themes: those elements looked
    /// the same in the default and classic theme before the migration and have
    /// to keep doing so.
    #[test]
    fn migrated_overlay_tokens_are_theme_independent() {
        let default = ColorTokens::default();
        let classic = ColorTokens::classic();
        let tokens = [
            ("minimap_background", default.minimap_background, classic.minimap_background),
            (
                "minimap_marker_fallback",
                default.minimap_marker_fallback,
                classic.minimap_marker_fallback,
            ),
            (
                "minimap_marker_outline",
                default.minimap_marker_outline,
                classic.minimap_marker_outline,
            ),
            ("spirit_sphere_empty", default.spirit_sphere_empty, classic.spirit_sphere_empty),
            ("party_health_fill", default.party_health_fill, classic.party_health_fill),
            (
                "overlay_secondary_text",
                default.overlay_secondary_text,
                classic.overlay_secondary_text,
            ),
            ("overlay_highlight", default.overlay_highlight, classic.overlay_highlight),
            ("profiler_bar_hot", default.profiler_bar_hot, classic.profiler_bar_hot),
            ("text_hint", default.text_hint, classic.text_hint),
            ("chat_ghost_text", default.chat_ghost_text, classic.chat_ghost_text),
            (
                "item_slot_background",
                default.item_slot_background,
                classic.item_slot_background,
            ),
            ("item_slot_border", default.item_slot_border, classic.item_slot_border),
            ("item_slot_inner", default.item_slot_inner, classic.item_slot_inner),
            (
                "item_slot_unavailable_background",
                default.item_slot_unavailable_background,
                classic.item_slot_unavailable_background,
            ),
            (
                "item_slot_unavailable_border",
                default.item_slot_unavailable_border,
                classic.item_slot_unavailable_border,
            ),
            (
                "item_slot_unavailable_text",
                default.item_slot_unavailable_text,
                classic.item_slot_unavailable_text,
            ),
            ("item_slot_card_name", default.item_slot_card_name, classic.item_slot_card_name),
        ];

        for (name, default_value, classic_value) in tokens {
            assert_eq!(
                default_value, classic_value,
                "{name} changed with the theme, but it used to be theme independent"
            );
        }
    }

    /// The migration from hardcoded literals to tokens must not change the
    /// appearance: every token keeps the exact value the window used before.
    #[test]
    fn migrated_tokens_keep_the_legacy_values() {
        let tokens = DesignTokens::default();
        let color = tokens.color;

        // Minimap.
        assert_eq!(color.minimap_background, crate::graphics::Color::rgb_u8(22, 24, 28));
        assert_eq!(color.minimap_marker_fallback, crate::graphics::Color::rgb_u8(80, 255, 110));
        assert_eq!(color.minimap_marker_outline, crate::graphics::Color::rgba_u8(0, 0, 0, 200));
        // HUD overlays.
        assert_eq!(color.spirit_sphere_empty, crate::graphics::Color::rgba_u8(24, 34, 46, 190));
        assert_eq!(color.party_health_fill, crate::graphics::Color::rgb_u8(84, 196, 104));
        assert_eq!(color.overlay_secondary_text, crate::graphics::Color::monochrome_u8(200));
        assert_eq!(color.overlay_highlight, crate::graphics::Color::rgba_u8(255, 255, 255, 100));
        assert_eq!(color.profiler_bar_hot, crate::graphics::Color::rgba_u8(255, 80, 0, 150));
        assert_eq!(color.text_hint, crate::graphics::Color::rgb_u8(160, 160, 160));
        assert_eq!(color.chat_ghost_text, crate::graphics::Color::rgba_u8(205, 205, 205, 230));
        // Item info card slots.
        assert_eq!(color.item_slot_background, crate::graphics::Color::rgba_u8(28, 28, 34, 140));
        assert_eq!(color.item_slot_border, crate::graphics::Color::rgba_u8(120, 120, 130, 120));
        assert_eq!(color.item_slot_inner, crate::graphics::Color::rgba_u8(0, 0, 0, 70));
        assert_eq!(
            color.item_slot_unavailable_background,
            crate::graphics::Color::rgba_u8(86, 86, 92, 70)
        );
        assert_eq!(
            color.item_slot_unavailable_border,
            crate::graphics::Color::rgba_u8(120, 120, 126, 60)
        );
        assert_eq!(
            color.item_slot_unavailable_text,
            crate::graphics::Color::rgba_u8(160, 160, 166, 90)
        );
        assert_eq!(color.item_slot_card_name, crate::graphics::Color::rgba_u8(235, 235, 240, 220));

        // Typography pixel sizes used by the migrated windows.
        let typography = tokens.typography;
        assert_eq!(typography.font_xs.0, 9.0);
        assert_eq!(typography.font_body.0, 12.0);
        assert_eq!(typography.font_button.0, 13.0);
        assert_eq!(typography.font_body_lg.0, 14.0);
        assert_eq!(typography.font_subtitle.0, 15.0);
        assert_eq!(typography.font_title.0, 18.0);
    }

    /// The typography scale must stay ordered; otherwise a window can end up
    /// with a "small" font that is larger than a "large" one.
    #[test]
    fn typography_scale_is_ordered() {
        let tokens = TypographyTokens::default();
        let scale = [
            tokens.font_xs.0,
            tokens.font_sm.0,
            tokens.font_caption.0,
            tokens.font_label.0,
            tokens.font_body.0,
            tokens.font_button.0,
            tokens.font_body_lg.0,
            tokens.font_subtitle.0,
            tokens.font_lg.0,
            tokens.font_title.0,
            tokens.font_heading.0,
        ];
        assert!(
            scale.windows(2).all(|window| window[0] <= window[1]),
            "typography scale is not ordered: {scale:?}"
        );
    }
}
