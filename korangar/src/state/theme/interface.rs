#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer, print_debug};
use korangar_interface::components::button::ButtonTheme;
use korangar_interface::components::collapsible::CollapsibleTheme;
use korangar_interface::components::drop_down::DropDownTheme;
use korangar_interface::components::field::FieldTheme;
use korangar_interface::components::state_button::StateButtonTheme;
use korangar_interface::components::text::TextTheme;
use korangar_interface::components::text_box::TextBoxTheme;
use korangar_interface::element::StateElement;
use korangar_interface::layout::tooltip::TooltipTheme;
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use korangar_interface::skin::{ButtonSkin, DropDownSkin, ImageSlice, SkinTheme, Slice, StateButtonSkin, WindowSkin};
use korangar_interface::window::{StateWindow, WindowTheme};
use rust_state::RustState;
use serde::{Deserialize, Serialize};

use super::tokens::DesignTokens;
use crate::graphics::{Color, CornerDiameter, ScreenSize, ShadowPadding};
use crate::loaders::{FontSize, OverflowBehavior};
use crate::renderer::skin::SkinImage;
use crate::settings::{CLASSIC_THEME_NAME, IN_GAME_THEMES_PATH, MENU_THEMES_PATH};
use crate::state::ClientState;

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum InterfaceThemeType {
    #[default]
    InGame,
    Menu,
}

#[derive(Serialize, Deserialize, RustState, StateElement)]
pub struct DebugButtonTheme {
    foreground_color: Color,
    hovered_background_color: Color,
}

#[derive(Serialize, Deserialize, RustState, StateElement)]
pub struct ChatTheme {
    pub window_color: Color,
    pub text_box_background_color: Color,
    /// Background colors used when the chat window is drawn transparently.
    pub transparent_window_color: Color,
    pub transparent_text_box_background_color: Color,
    /// Title color of the chat window.
    pub window_title_color: Color,
    pub hovered_window_title_color: Color,
    /// Foreground color of the chat input line.
    pub text_box_foreground_color: Color,
    /// Foreground color of the chat input line while it is focused.
    ///
    /// The focused chat input is drawn on a dark background, so it must not
    /// reuse a theme color that is meant for the light skinned text boxes
    /// (the classic theme uses black text there). Defaults to a near-white
    /// color for old theme files that do not contain the field.
    #[serde(default = "default_chat_input_focused_foreground_color")]
    pub text_box_focused_foreground_color: Color,
    /// Color of the ghost text of the chat input line.
    pub ghost_foreground_color: Color,
}

/// Default focused color of the chat input text: near-white, it is drawn on a
/// dark input background.
fn default_chat_input_focused_foreground_color() -> Color {
    Color::rgb_u8(235, 235, 235)
}

#[derive(Serialize, Deserialize, RustState, StateElement)]
pub struct SkillTreeTheme {
    /// Background color for an occupied slot.
    slot_background_color: Color,
    slot_outline: ShadowPadding,
    /// Color used for showing level or skill requirements, both outlines and
    /// text.
    requirement_color: Color,
    /// Outline color of a hovered slot.
    hover_color: Color,
    /// Corner diameter of the occupied and unoccupied slots.
    slot_corner_diameter: CornerDiameter,
    unoccupied_slot_color: Color,
    unoccupied_slot_outline: ShadowPadding,
    unlearned_skill_color: Color,
    name_font_size: FontSize,
    name_text_color: Color,
    highlight_color: Color,
    points_color: Color,
    /// Color used for coloring pending skll points.
    pending_points_color: Color,
    points_font_size: FontSize,
    arrow_color: Color,
    /// Color used when the selected skill level is lower than the learned skill
    /// level.
    lower_level_color: Color,
}

#[derive(Serialize, Deserialize, RustState, StateElement)]
pub struct GlobalTheme {
    /// Color indicating that a grabbed resource can be dropped in the
    /// highlighted area.
    drop_area_color: Color,
    /// Color indicating that a grabbed resource is inside the highlighted area
    /// and can be dropped there.
    hovered_drop_area_color: Color,
    /// Alpha to multiply the area color by to get the fill color of the area.
    /// The original color will be used for the outline only.
    fill_alpha: f32,
    /// Outline for the highlighted area.
    drop_area_outline: ShadowPadding,
}

/// Colors of the character overview window.
///
/// The overview draws HP, SP, base EXP and job EXP as progress bars, so each
/// bar has a fill color and a background (track) color. The HP fill switches to
/// [`Self::low_health_color`] when the character is low on health, which gives
/// the classic red/green health bar look.
#[derive(Serialize, Deserialize, RustState, StateElement)]
pub struct CharacterOverviewTheme {
    /// Background of the progress bars (the part that is not filled).
    pub bar_background_color: Color,
    /// Fill color of the health points bar.
    pub health_color: Color,
    /// Fill color of the health points bar when the character is low on health.
    pub low_health_color: Color,
    /// Background of the health points bar (shows the missing health).
    pub health_background_color: Color,
    /// Fill color of the spell points bar.
    pub spell_point_color: Color,
    /// Background of the spell points bar.
    pub spell_point_background_color: Color,
    /// Fill color of the base experience bar.
    pub experience_color: Color,
    /// Background of the base experience bar.
    pub experience_background_color: Color,
    /// Fill color of the job experience bar.
    pub job_experience_color: Color,
    /// Background of the job experience bar.
    pub job_experience_background_color: Color,
    /// Color of the small labels ("HP", "SP", "EXP", ...).
    pub label_color: Color,
    /// Color of the values ("1234 / 2345", "45.2 %", ...).
    pub value_color: Color,
}

impl Default for CharacterOverviewTheme {
    fn default() -> Self {
        Self {
            bar_background_color: Color::monochrome_u8(40),
            health_color: Color::rgb_u8(67, 163, 83),
            low_health_color: Color::rgb_u8(206, 49, 116),
            health_background_color: Color::rgb_u8(72, 22, 22),
            spell_point_color: Color::rgb_u8(0, 129, 163),
            spell_point_background_color: Color::rgb_u8(14, 36, 52),
            experience_color: Color::rgb_u8(230, 200, 80),
            experience_background_color: Color::rgb_u8(56, 48, 18),
            job_experience_color: Color::rgb_u8(170, 130, 230),
            job_experience_background_color: Color::rgb_u8(40, 28, 60),
            label_color: Color::monochrome_u8(190),
            value_color: Color::monochrome_u8(235),
        }
    }
}

/// Colors of the party window roster and HP bars.
///
/// The classic theme uses a light background, so the roster text has to be dark
/// and offline members are drawn in a darker gray than the (too light) default
/// color to stay readable.
#[derive(Serialize, Deserialize, RustState, StateElement)]
pub struct PartyTheme {
    /// Color of a party member name.
    pub member_name_color: Color,
    /// Color of a party member that is offline.
    pub offline_member_name_color: Color,
    /// Color of the health text ("123 / 456") next to the member.
    pub health_text_color: Color,
    /// Background of the member health bar.
    pub health_bar_background_color: Color,
}

impl Default for PartyTheme {
    fn default() -> Self {
        Self {
            member_name_color: Color::rgb_u8(230, 230, 230),
            offline_member_name_color: Color::rgb_u8(150, 150, 150),
            health_text_color: Color::rgb_u8(170, 170, 170),
            health_bar_background_color: Color::rgb_u8(36, 36, 36),
        }
    }
}

/// Colors of item cards (inventory, shop buy/sell lists).
///
/// The default theme uses dark cards, the classic theme uses white/light cards
/// like the original client.
#[derive(Serialize, Deserialize, RustState, StateElement)]
pub struct ItemCardTheme {
    /// Background of an item card.
    pub background_color: Color,
    /// Background of an item card while it is hovered.
    pub hovered_background_color: Color,
    /// Background of an item card that is picked up (dragged).
    pub drag_background_color: Color,
    /// Background of a dragged item card that is hovered over a drop target.
    pub drag_hovered_background_color: Color,
    /// Border/shadow color of an item card.
    pub border_color: Color,
    /// Color of the item name/labels on the card.
    pub text_color: Color,
    /// Color of highlighted text on the card (price, amount, selected price).
    pub highlight_color: Color,
}

impl Default for ItemCardTheme {
    fn default() -> Self {
        Self {
            background_color: Color::rgb_u8(40, 40, 40),
            hovered_background_color: Color::rgb_u8(60, 60, 60),
            drag_background_color: Color::rgb_u8(180, 180, 80),
            drag_hovered_background_color: Color::rgb_u8(80, 180, 180),
            border_color: Color::rgba_u8(0, 0, 0, 100),
            text_color: Color::monochrome_u8(220),
            highlight_color: Color::rgb_u8(255, 160, 60),
        }
    }
}

#[derive(Serialize, Deserialize, RustState, StateElement, StateWindow)]
#[window_title("Theme Inspector")]
pub struct InterfaceTheme {
    #[hidden_element]
    pub window: WindowTheme<ClientState>,
    #[hidden_element]
    pub text: TextTheme<ClientState>,
    #[hidden_element]
    pub button: ButtonTheme<ClientState>,
    #[hidden_element]
    pub state_button: StateButtonTheme<ClientState>,
    #[hidden_element]
    pub text_box: TextBoxTheme<ClientState>,
    #[hidden_element]
    pub collapsible: CollapsibleTheme<ClientState>,
    #[hidden_element]
    pub drop_down: DropDownTheme<ClientState>,
    #[hidden_element]
    pub field: FieldTheme<ClientState>,
    #[hidden_element]
    pub tooltip: TooltipTheme<ClientState>,
    /// Image based skin of the interface. All fields are optional, a default
    /// skin means that the interface is drawn using colors and geometry only.
    #[hidden_element]
    pub skin: SkinTheme<ClientState>,
    pub debug_button: DebugButtonTheme,
    pub chat: ChatTheme,
    pub skill_tree: SkillTreeTheme,
    pub global: GlobalTheme,
    /// Colors of the character overview bars. The field is optional in the
    /// persisted theme files, so themes that were saved before the bars were
    /// added keep loading and use the default colors.
    #[serde(default)]
    pub character_overview: CharacterOverviewTheme,
    /// Colors of the item cards. Optional in persisted theme files, old themes
    /// keep loading and use the default dark cards.
    #[serde(default)]
    pub item_card: ItemCardTheme,
    /// Semantic design tokens (colors, typography, spacing, radii) that the
    /// migrated windows use instead of hardcoded literals. Optional in
    /// persisted theme files: themes written before the tokens existed load
    /// and get the default (dark) token set.
    #[serde(default)]
    pub tokens: DesignTokens,
    /// Colors of the party window roster. Optional in persisted theme files,
    /// old themes keep loading and use the default colors.
    #[serde(default)]
    pub party: PartyTheme,
}

impl InterfaceTheme {
    pub fn load(theme_type: InterfaceThemeType, name: &str) -> Self {
        #[cfg(feature = "debug")]
        let timer = Timer::new("Load theme");

        if name == CLASSIC_THEME_NAME {
            crate::logging::log_debug!("theme: using built-in classic theme for {:?}", theme_type);
            return Self::default_classic();
        }

        let path = match theme_type {
            InterfaceThemeType::InGame => format!("{IN_GAME_THEMES_PATH}/{name}.ron"),
            InterfaceThemeType::Menu => format!("{MENU_THEMES_PATH}/{name}.ron"),
        };

        #[cfg(feature = "debug")]
        print_debug!("loading theme from file {}", path.magenta());

        crate::logging::log_debug!("theme: loading theme `{}` for {:?} from {}", name, theme_type, path);

        let theme = std::fs::read_to_string(&path)
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
            .unwrap_or_else(|| {
                crate::logging::log_warn!("theme: failed to load theme `{}` from {}, using default", name, path);

                match theme_type {
                    InterfaceThemeType::Menu => Self::default_menu(),
                    InterfaceThemeType::InGame => Self::default_in_game(),
                }
            });

        #[cfg(feature = "debug")]
        timer.stop();

        theme
    }

    #[cfg(feature = "debug")]
    pub fn save(&self, theme_type: InterfaceThemeType, name: &str) {
        let timer = Timer::new("Save theme");

        let path = match theme_type {
            InterfaceThemeType::InGame => format!("{IN_GAME_THEMES_PATH}/{name}.ron"),
            InterfaceThemeType::Menu => format!("{MENU_THEMES_PATH}/{name}.ron"),
        };

        print_debug!("saving theme to file {}", path.magenta());

        let data = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::new()).unwrap();

        if let Err(error) = std::fs::write(path, data) {
            print_debug!(
                "[{}] failed to save theme to {}: {:?}",
                "error".red(),
                name.magenta(),
                error.red()
            );
        }

        timer.stop();
    }

    fn default_menu() -> Self {
        Self {
            window: WindowTheme {
                title_color: Color::rgb_u8(200, 150, 150),
                hovered_title_color: Color::rgb_u8(250, 200, 200),
                background_color: Color::monochrome_u8(30),
                highlight_color: Color::rgb_u8(255, 160, 60),
                shadow_color: Color::rgba_u8(0, 0, 0, 100),
                shadow_padding: ShadowPadding::diagonal(5.0, 10.0),
                gaps: 25.0,
                border: 30.0,
                corner_diameter: CornerDiameter::uniform(50.0),
                close_button_size: ScreenSize { width: 45.0, height: 35.0 },
                close_button_corner_diameter: CornerDiameter::uniform(25.0),
                minimum_width: 300.0,
                maximum_width: 600.0,
                minimum_height: 50.0,
                maximum_height: 700.0,
                title_height: 45.0,
                title_gap: 20.0,
                font_size: FontSize(20.0),
                horizontal_alignment: HorizontalAlignment::Center { offset: 0.0, border: 5.0 },
                vertical_alignment: VerticalAlignment::Center { offset: 0.0 },
                overflow_behavior: OverflowBehavior::Shrink,
                anchor_color: Color::rgb_u8(130, 105, 160),
                closest_anchor_color: Color::rgb_u8(255, 175, 30),
            },
            text: TextTheme {
                color: Color::monochrome_u8(220),
                highlight_color: Color::rgb_u8(255, 160, 60),
                height: 15.0,
                font_size: FontSize(16.0),
                horizontal_alignment: HorizontalAlignment::Left { offset: 6.0, border: 3.0 },
                vertical_alignment: VerticalAlignment::Center { offset: 0.0 },
                overflow_behavior: OverflowBehavior::LineBreak,
            },
            button: ButtonTheme {
                background_color: Color::monochrome_u8(80),
                foreground_color: Color::monochrome_u8(180),
                highlight_color: Color::rgb_u8(255, 160, 60),
                hovered_background_color: Color::monochrome_u8(120),
                hovered_foreground_color: Color::monochrome_u8(220),
                disabled_background_color: Color::monochrome_u8(50),
                disabled_foreground_color: Color::monochrome_u8(100),
                shadow_color: Color::rgba_u8(0, 0, 0, 100),
                shadow_padding: ShadowPadding::diagonal(2.0, 5.0),
                height: 30.0,
                corner_diameter: CornerDiameter::uniform(30.0),
                font_size: FontSize(16.0),
                horizontal_alignment: HorizontalAlignment::Center { offset: 0.0, border: 5.0 },
                vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
                overflow_behavior: OverflowBehavior::Shrink,
            },
            state_button: StateButtonTheme {
                background_color: Color::monochrome_u8(80),
                foreground_color: Color::monochrome_u8(180),
                highlight_color: Color::rgb_u8(255, 160, 60),
                hovered_background_color: Color::monochrome_u8(120),
                hovered_foreground_color: Color::monochrome_u8(220),
                disabled_background_color: Color::monochrome_u8(50),
                disabled_foreground_color: Color::monochrome_u8(100),
                checkbox_color: Color::rgb_u8(255, 100, 100),
                hovered_checkbox_color: Color::rgb_u8(255, 140, 140),
                disabled_checkbox_color: Color::monochrome_u8(180),
                shadow_color: Color::rgba_u8(0, 0, 0, 100),
                shadow_padding: ShadowPadding::diagonal(2.0, 5.0),
                height: 24.0,
                corner_diameter: CornerDiameter::uniform(24.0),
                font_size: FontSize(16.0),
                horizontal_alignment: HorizontalAlignment::Left { offset: 50.0, border: 3.0 },
                vertical_alignment: VerticalAlignment::Center { offset: 0.0 },
                overflow_behavior: OverflowBehavior::Shrink,
            },
            text_box: TextBoxTheme {
                background_color: Color::monochrome_u8(45),
                foreground_color: Color::monochrome_u8(180),
                highlight_color: Color::rgb_u8(255, 160, 60),
                hovered_background_color: Color::monochrome_u8(75),
                hovered_foreground_color: Color::monochrome_u8(220),
                focused_background_color: Color::monochrome_u8(120),
                focused_foreground_color: Color::monochrome_u8(255),
                ghost_foreground_color: Color::monochrome_u8(100),
                hide_icon_color: Color::rgb_u8(200, 180, 180),
                hovered_hide_icon_color: Color::rgb_u8(250, 200, 200),
                shadow_color: Color::rgba_u8(0, 0, 0, 100),
                shadow_padding: ShadowPadding::diagonal(2.0, 5.0),
                height: 30.0,
                corner_diameter: CornerDiameter::uniform(30.0),
                font_size: FontSize(16.0),
                horizontal_alignment: HorizontalAlignment::Center { offset: 0.0, border: 5.0 },
                vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
                overflow_behavior: OverflowBehavior::LineBreak,
            },
            collapsible: CollapsibleTheme {
                background_color: Color::monochrome_u8(45),
                secondary_background_color: Color::monochrome_u8(30),
                foreground_color: Color::monochrome_u8(200),
                highlight_color: Color::rgb_u8(255, 160, 60),
                hovered_foreground_color: Color::rgb_u8(250, 200, 200),
                corner_diameter: CornerDiameter::uniform(20.0),
                icon_color: Color::monochrome_u8(170),
                icon_size: 15.0,
                shadow_color: Color::rgba_u8(0, 0, 0, 100),
                shadow_padding: ShadowPadding::diagonal(2.0, 5.0),
                gaps: 5.0,
                border: 10.0,
                title_height: 30.0,
                font_size: FontSize(16.0),
                horizontal_alignment: HorizontalAlignment::Left { offset: 0.0, border: 3.0 },
                vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
                overflow_behavior: OverflowBehavior::Shrink,
            },
            drop_down: DropDownTheme {
                item_background_color: Color::monochrome_u8(65),
                item_foreground_color: Color::monochrome_u8(180),
                item_highlight_color: Color::rgb_u8(255, 160, 60),
                item_hovered_background_color: Color::monochrome_u8(105),
                item_hovered_foreground_color: Color::monochrome_u8(220),
                item_shadow_color: Color::rgba_u8(0, 0, 0, 100),
                item_shadow_padding: ShadowPadding::diagonal(2.0, 5.0),
                item_disabled_foreground_color: Color::monochrome_u8(110),
                item_disabled_background_color: Color::monochrome_u8(50),
                item_height: 30.0,
                item_corner_diameter: CornerDiameter::uniform(30.0),
                item_font_size: FontSize(16.0),
                item_horizontal_alignment: HorizontalAlignment::Center { offset: 0.0, border: 5.0 },
                item_vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
                item_overflow_behavior: OverflowBehavior::Shrink,
                list_corner_diameter: CornerDiameter::uniform(30.0),
                list_background_color: Color::monochrome_u8(40),
                list_shadow_color: Color::rgba_u8(0, 0, 0, 100),
                list_shadow_padding: ShadowPadding::diagonal(5.0, 10.0),
                list_gaps: 8.0,
                list_border: 5.0,
                list_maximum_height: 700.0,
                button_background_color: Color::monochrome_u8(80),
                button_foreground_color: Color::monochrome_u8(180),
                button_highlight_color: Color::rgb_u8(255, 160, 60),
                button_hovered_background_color: Color::monochrome_u8(120),
                button_hovered_foreground_color: Color::monochrome_u8(220),
                button_shadow_color: Color::rgba_u8(0, 0, 0, 100),
                button_shadow_padding: ShadowPadding::diagonal(2.0, 5.0),
                button_height: 30.0,
                button_corner_diameter: CornerDiameter::uniform(30.0),
                button_font_size: FontSize(16.0),
                button_horizontal_alignment: HorizontalAlignment::Center { offset: 0.0, border: 5.0 },
                button_vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
                button_overflow_behavior: OverflowBehavior::Shrink,
            },
            field: FieldTheme {
                background_color: Color::monochrome_u8(80),
                foreground_color: Color::monochrome_u8(180),
                highlight_color: Color::rgb_u8(255, 160, 60),
                shadow_color: Color::rgba_u8(0, 0, 0, 100),
                shadow_padding: ShadowPadding::diagonal(2.0, 5.0),
                height: 30.0,
                corner_diameter: CornerDiameter::uniform(30.0),
                font_size: FontSize(16.0),
                horizontal_alignment: HorizontalAlignment::Center { offset: 0.0, border: 5.0 },
                vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
                overflow_behavior: OverflowBehavior::LineBreak,
            },
            tooltip: TooltipTheme {
                background_color: Color::rgba_u8(15, 15, 15, 200),
                foreground_color: Color::monochrome_u8(235),
                highlight_color: Color::rgb_u8(255, 160, 60),
                shadow_color: Color::rgba_u8(0, 0, 0, 100),
                shadow_padding: ShadowPadding::diagonal(2.0, 5.0),
                font_size: FontSize(16.0),
                corner_diameter: CornerDiameter::uniform(8.0),
                border: 8.0,
                gap: 4.0,
                mouse_offset: 20.0,
                overflow_behavior: OverflowBehavior::LineBreak,
            },
            skin: SkinTheme::default(),
            debug_button: DebugButtonTheme {
                foreground_color: Color::rgb_u8(255, 167, 89),
                hovered_background_color: Color::rgb_u8(225, 199, 115),
            },
            chat: ChatTheme {
                window_color: Color::TRANSPARENT,
                text_box_background_color: Color::TRANSPARENT,
                transparent_window_color: Color::TRANSPARENT,
                transparent_text_box_background_color: Color::TRANSPARENT,
                window_title_color: Color::rgb_u8(200, 150, 150),
                hovered_window_title_color: Color::rgb_u8(250, 200, 200),
                text_box_foreground_color: Color::monochrome_u8(220),
                text_box_focused_foreground_color: Color::rgb_u8(235, 235, 235),
                ghost_foreground_color: Color::monochrome_u8(170),
            },
            skill_tree: SkillTreeTheme {
                slot_background_color: Color::monochrome_u8(40),
                slot_outline: ShadowPadding::uniform(3.0),
                requirement_color: Color::rgba_u8(200, 100, 200, 255),
                hover_color: Color::WHITE,
                slot_corner_diameter: CornerDiameter::uniform(20.0),
                unoccupied_slot_color: Color::rgba_u8(30, 30, 30, 160),
                unoccupied_slot_outline: ShadowPadding::uniform(2.0),
                unlearned_skill_color: Color::monochrome_u8(80),
                name_font_size: FontSize(12.0),
                name_text_color: Color::WHITE,
                highlight_color: Color::rgb_u8(255, 160, 60),
                points_color: Color::WHITE,
                pending_points_color: Color::rgb_u8(0, 255, 220),
                points_font_size: FontSize(12.0),
                arrow_color: Color::monochrome_u8(160),
                lower_level_color: Color::rgb_u8(255, 200, 255),
            },
            global: GlobalTheme {
                drop_area_color: Color::rgba_u8(255, 60, 200, 120),
                hovered_drop_area_color: Color::rgba_u8(0, 255, 200, 130),
                fill_alpha: 0.1,
                drop_area_outline: ShadowPadding::uniform(4.0),
            },
            character_overview: CharacterOverviewTheme::default(),
            item_card: ItemCardTheme::default(),
            tokens: DesignTokens::default(),
            party: PartyTheme::default(),
        }
    }

    fn default_in_game() -> Self {
        Self {
            window: WindowTheme {
                title_color: Color::rgb_u8(185, 155, 155),
                hovered_title_color: Color::rgb_u8(240, 155, 155),
                background_color: Color::monochrome_u8(50),
                highlight_color: Color::rgb_u8(255, 160, 60),
                shadow_color: Color::rgba_u8(0, 0, 0, 100),
                shadow_padding: ShadowPadding::diagonal(5.0, 10.0),
                gaps: 8.0,
                border: 10.0,
                corner_diameter: CornerDiameter::uniform(14.0),
                close_button_size: ScreenSize { width: 40.0, height: 18.0 },
                close_button_corner_diameter: CornerDiameter::uniform(12.0),
                minimum_width: 300.0,
                maximum_width: 600.0,
                minimum_height: 40.0,
                maximum_height: 700.0,
                title_height: 25.0,
                title_gap: 2.0,
                font_size: FontSize(15.0),
                horizontal_alignment: HorizontalAlignment::Center { offset: 0.0, border: 5.0 },
                vertical_alignment: VerticalAlignment::Center { offset: 0.0 },
                overflow_behavior: OverflowBehavior::Shrink,
                anchor_color: Color::rgb_u8(140, 105, 130),
                closest_anchor_color: Color::rgb_u8(255, 175, 30),
            },
            text: TextTheme {
                color: Color::monochrome_u8(220),
                highlight_color: Color::rgb_u8(255, 160, 60),
                height: 14.0,
                font_size: FontSize(14.0),
                horizontal_alignment: HorizontalAlignment::Left { offset: 3.0, border: 3.0 },
                vertical_alignment: VerticalAlignment::Center { offset: 0.0 },
                overflow_behavior: OverflowBehavior::LineBreak,
            },
            button: ButtonTheme {
                background_color: Color::monochrome_u8(120),
                foreground_color: Color::monochrome_u8(220),
                highlight_color: Color::rgb_u8(255, 160, 60),
                hovered_background_color: Color::monochrome_u8(150),
                hovered_foreground_color: Color::monochrome_u8(250),
                disabled_background_color: Color::monochrome_u8(100),
                disabled_foreground_color: Color::monochrome_u8(160),
                shadow_color: Color::rgba_u8(0, 0, 0, 100),
                shadow_padding: ShadowPadding::diagonal(2.0, 5.0),
                height: 20.0,
                corner_diameter: CornerDiameter::uniform(10.0),
                font_size: FontSize(14.0),
                horizontal_alignment: HorizontalAlignment::Center { offset: 0.0, border: 5.0 },
                vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
                overflow_behavior: OverflowBehavior::Shrink,
            },
            state_button: StateButtonTheme {
                background_color: Color::monochrome_u8(120),
                foreground_color: Color::monochrome_u8(220),
                highlight_color: Color::rgb_u8(255, 160, 60),
                hovered_background_color: Color::monochrome_u8(150),
                hovered_foreground_color: Color::monochrome_u8(250),
                disabled_background_color: Color::monochrome_u8(80),
                disabled_foreground_color: Color::monochrome_u8(120),
                checkbox_color: Color::monochrome_u8(200),
                hovered_checkbox_color: Color::monochrome_u8(250),
                disabled_checkbox_color: Color::monochrome_u8(120),
                shadow_color: Color::rgba_u8(0, 0, 0, 100),
                shadow_padding: ShadowPadding::diagonal(2.0, 5.0),
                height: 20.0,
                corner_diameter: CornerDiameter::uniform(10.0),
                font_size: FontSize(14.0),
                horizontal_alignment: HorizontalAlignment::Left { offset: 30.0, border: 3.0 },
                vertical_alignment: VerticalAlignment::Center { offset: 0.0 },
                overflow_behavior: OverflowBehavior::Shrink,
            },
            text_box: TextBoxTheme {
                background_color: Color::monochrome_u8(40),
                foreground_color: Color::monochrome_u8(220),
                highlight_color: Color::rgb_u8(255, 160, 60),
                hovered_background_color: Color::monochrome_u8(60),
                hovered_foreground_color: Color::monochrome_u8(250),
                focused_background_color: Color::monochrome_u8(110),
                focused_foreground_color: Color::monochrome_u8(255),
                ghost_foreground_color: Color::monochrome_u8(130),
                hide_icon_color: Color::monochrome_u8(180),
                hovered_hide_icon_color: Color::rgb_u8(250, 200, 200),
                shadow_color: Color::rgba_u8(0, 0, 0, 100),
                shadow_padding: ShadowPadding::diagonal(2.0, 5.0),
                height: 20.0,
                corner_diameter: CornerDiameter::uniform(10.0),
                font_size: FontSize(14.0),
                horizontal_alignment: HorizontalAlignment::Left { offset: 15.0, border: 3.0 },
                vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
                overflow_behavior: OverflowBehavior::LineBreak,
            },
            collapsible: CollapsibleTheme {
                background_color: Color::monochrome_u8(75),
                secondary_background_color: Color::monochrome_u8(55),
                foreground_color: Color::monochrome_u8(170),
                highlight_color: Color::rgb_u8(255, 160, 60),
                hovered_foreground_color: Color::rgb_u8(250, 200, 200),
                corner_diameter: CornerDiameter::uniform(10.0),
                icon_color: Color::monochrome_u8(120),
                icon_size: 10.0,
                shadow_color: Color::rgba_u8(0, 0, 0, 100),
                shadow_padding: ShadowPadding::diagonal(2.0, 5.0),
                gaps: 4.0,
                border: 5.0,
                title_height: 20.0,
                font_size: FontSize(14.0),
                horizontal_alignment: HorizontalAlignment::Left { offset: 0.0, border: 3.0 },
                vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
                overflow_behavior: OverflowBehavior::Shrink,
            },
            drop_down: DropDownTheme {
                item_background_color: Color::monochrome_u8(80),
                item_foreground_color: Color::monochrome_u8(180),
                item_highlight_color: Color::rgb_u8(255, 160, 60),
                item_hovered_background_color: Color::monochrome_u8(120),
                item_hovered_foreground_color: Color::monochrome_u8(220),
                item_shadow_color: Color::rgba_u8(0, 0, 0, 100),
                item_shadow_padding: ShadowPadding::diagonal(2.0, 5.0),
                item_disabled_foreground_color: Color::monochrome_u8(110),
                item_disabled_background_color: Color::monochrome_u8(50),
                item_height: 20.0,
                item_corner_diameter: CornerDiameter::uniform(10.0),
                item_font_size: FontSize(14.0),
                item_horizontal_alignment: HorizontalAlignment::Center { offset: 0.0, border: 5.0 },
                item_vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
                item_overflow_behavior: OverflowBehavior::Shrink,
                list_corner_diameter: CornerDiameter::uniform(8.0),
                list_background_color: Color::monochrome_u8(40),
                list_shadow_color: Color::rgba_u8(0, 0, 0, 100),
                list_shadow_padding: ShadowPadding::diagonal(5.0, 10.0),
                list_gaps: 4.0,
                list_border: 4.0,
                list_maximum_height: 500.0,
                button_background_color: Color::monochrome_u8(120),
                button_foreground_color: Color::monochrome_u8(220),
                button_highlight_color: Color::rgb_u8(255, 160, 60),
                button_hovered_background_color: Color::monochrome_u8(150),
                button_hovered_foreground_color: Color::monochrome_u8(250),
                button_shadow_color: Color::rgba_u8(0, 0, 0, 100),
                button_shadow_padding: ShadowPadding::diagonal(2.0, 5.0),
                button_height: 20.0,
                button_corner_diameter: CornerDiameter::uniform(10.0),
                button_font_size: FontSize(14.0),
                button_horizontal_alignment: HorizontalAlignment::Center { offset: 0.0, border: 5.0 },
                button_vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
                button_overflow_behavior: OverflowBehavior::Shrink,
            },
            field: FieldTheme {
                background_color: Color::monochrome_u8(100),
                foreground_color: Color::monochrome_u8(220),
                highlight_color: Color::rgb_u8(255, 160, 60),
                shadow_color: Color::rgba_u8(0, 0, 0, 100),
                shadow_padding: ShadowPadding::diagonal(2.0, 5.0),
                height: 20.0,
                corner_diameter: CornerDiameter::uniform(10.0),
                font_size: FontSize(14.0),
                horizontal_alignment: HorizontalAlignment::Left { offset: 10.0, border: 3.0 },
                vertical_alignment: VerticalAlignment::Center { offset: -2.0 },
                overflow_behavior: OverflowBehavior::LineBreak,
            },
            tooltip: TooltipTheme {
                background_color: Color::rgba_u8(15, 15, 15, 200),
                foreground_color: Color::monochrome_u8(235),
                highlight_color: Color::rgb_u8(255, 160, 60),
                shadow_color: Color::rgba_u8(0, 0, 0, 100),
                shadow_padding: ShadowPadding::diagonal(2.0, 5.0),
                font_size: FontSize(14.0),
                corner_diameter: CornerDiameter::uniform(5.0),
                border: 4.0,
                gap: 3.0,
                mouse_offset: 16.0,
                overflow_behavior: OverflowBehavior::LineBreak,
            },
            skin: SkinTheme::default(),
            debug_button: DebugButtonTheme {
                foreground_color: Color::rgb_u8(255, 167, 89),
                hovered_background_color: Color::rgb_u8(225, 199, 115),
            },
            chat: ChatTheme {
                window_color: Color::rgba_u8(0, 0, 0, 200),
                text_box_background_color: Color::rgba_u8(0, 0, 0, 150),
                transparent_window_color: Color::TRANSPARENT,
                transparent_text_box_background_color: Color::TRANSPARENT,
                window_title_color: Color::rgb_u8(185, 155, 155),
                hovered_window_title_color: Color::rgb_u8(240, 155, 155),
                text_box_foreground_color: Color::monochrome_u8(220),
                text_box_focused_foreground_color: Color::rgb_u8(235, 235, 235),
                ghost_foreground_color: Color::monochrome_u8(170),
            },
            skill_tree: SkillTreeTheme {
                slot_background_color: Color::monochrome_u8(40),
                slot_outline: ShadowPadding::uniform(3.0),
                requirement_color: Color::rgba_u8(200, 100, 200, 255),
                hover_color: Color::WHITE,
                slot_corner_diameter: CornerDiameter::uniform(20.0),
                unoccupied_slot_color: Color::rgba_u8(30, 30, 30, 160),
                unoccupied_slot_outline: ShadowPadding::uniform(2.0),
                unlearned_skill_color: Color::monochrome_u8(80),
                name_font_size: FontSize(12.0),
                name_text_color: Color::WHITE,
                highlight_color: Color::rgb_u8(255, 160, 60),
                points_color: Color::WHITE,
                pending_points_color: Color::rgb_u8(0, 255, 220),
                points_font_size: FontSize(12.0),
                arrow_color: Color::monochrome_u8(160),
                lower_level_color: Color::rgb_u8(255, 200, 255),
            },
            global: GlobalTheme {
                drop_area_color: Color::rgba_u8(255, 60, 200, 120),
                hovered_drop_area_color: Color::rgba_u8(0, 255, 200, 130),
                fill_alpha: 0.1,
                drop_area_outline: ShadowPadding::uniform(4.0),
            },
            character_overview: CharacterOverviewTheme::default(),
            item_card: ItemCardTheme::default(),
            tokens: DesignTokens::default(),
            party: PartyTheme::default(),
        }
    }

    /// The classic Ragnarok Online interface theme.
    ///
    /// This preset is a faithful recreation of the original client's interface:
    /// beige/white windows with a blue gradient title bar, classic beveled
    /// buttons, the classic text colors and the classic interface textures that
    /// are loaded from the game archives.
    ///
    /// The theme is selectable in the interface settings. It is not the
    /// default, so players that prefer the modern flat korangar theme are
    /// not affected.
    fn default_classic() -> Self {
        let mut theme = Self::default_in_game();
        theme.tokens = DesignTokens::classic();

        // Classic bar colors: black labels, dark-blue values, dark tracks and
        // the classic green/blue/yellow progress colors on the light interface.
        theme.character_overview = CharacterOverviewTheme {
            bar_background_color: Color::rgb_u8(180, 180, 180),
            health_color: Color::rgb_u8(0, 170, 0),
            low_health_color: Color::rgb_u8(200, 0, 0),
            health_background_color: Color::rgb_u8(96, 24, 24),
            spell_point_color: Color::rgb_u8(0, 80, 200),
            spell_point_background_color: Color::rgb_u8(16, 24, 64),
            experience_color: Color::rgb_u8(210, 170, 0),
            experience_background_color: Color::rgb_u8(80, 64, 16),
            job_experience_color: Color::rgb_u8(120, 60, 180),
            job_experience_background_color: Color::rgb_u8(40, 20, 60),
            label_color: Color::BLACK,
            value_color: Color::rgb_u8(0, 0, 128),
        };

        // Classic item cards are white/light like the original client, with
        // black names and the classic dark blue prices.
        theme.item_card = ItemCardTheme {
            background_color: Color::rgb_u8(250, 250, 250),
            hovered_background_color: Color::rgb_u8(196, 209, 234),
            drag_background_color: Color::rgb_u8(232, 220, 150),
            drag_hovered_background_color: Color::rgb_u8(170, 200, 235),
            border_color: Color::rgb_u8(130, 130, 130),
            text_color: Color::rgb_u8(0, 0, 0),
            highlight_color: Color::rgb_u8(0, 0, 128),
        };

        // Classic party roster: dark text on the light window, offline members in
        // a clearly darker gray so the names stay readable.
        theme.party = PartyTheme {
            member_name_color: Color::rgb_u8(0, 0, 0),
            offline_member_name_color: Color::rgb_u8(80, 80, 80),
            health_text_color: Color::rgb_u8(60, 60, 60),
            health_bar_background_color: Color::rgb_u8(190, 190, 190),
        };

        theme.window = WindowTheme {
            title_color: Color::rgb_u8(0, 0, 0),
            hovered_title_color: Color::rgb_u8(0, 0, 128),
            background_color: Color::rgb_u8(244, 244, 244),
            highlight_color: Color::rgb_u8(0, 0, 160),
            shadow_color: Color::TRANSPARENT,
            shadow_padding: ShadowPadding::uniform(0.0),
            gaps: 10.0,
            // The window frame has a 14 pixel border, the extra four pixels are
            // used as padding between the frame and the contents.
            border: 18.0,
            corner_diameter: CornerDiameter::uniform(0.0),
            close_button_size: ScreenSize { width: 42.0, height: 20.0 },
            close_button_corner_diameter: CornerDiameter::uniform(0.0),
            minimum_width: 300.0,
            maximum_width: 600.0,
            minimum_height: 60.0,
            maximum_height: 700.0,
            title_height: 22.0,
            title_gap: 2.0,
            font_size: FontSize(14.0),
            horizontal_alignment: HorizontalAlignment::Left { offset: 22.0, border: 4.0 },
            vertical_alignment: VerticalAlignment::Center { offset: 1.0 },
            overflow_behavior: OverflowBehavior::Shrink,
            anchor_color: Color::rgb_u8(140, 105, 130),
            closest_anchor_color: Color::rgb_u8(255, 175, 30),
        };

        theme.text = TextTheme {
            color: Color::rgb_u8(0, 0, 0),
            highlight_color: Color::rgb_u8(0, 0, 160),
            height: 16.0,
            font_size: FontSize(13.0),
            horizontal_alignment: HorizontalAlignment::Left { offset: 4.0, border: 2.0 },
            vertical_alignment: VerticalAlignment::Center { offset: 0.0 },
            overflow_behavior: OverflowBehavior::LineBreak,
        };

        theme.button = ButtonTheme {
            background_color: Color::rgb_u8(247, 247, 247),
            foreground_color: Color::rgb_u8(16, 16, 80),
            highlight_color: Color::rgb_u8(0, 0, 160),
            hovered_background_color: Color::rgb_u8(196, 209, 234),
            hovered_foreground_color: Color::WHITE,
            disabled_background_color: Color::rgb_u8(235, 235, 235),
            disabled_foreground_color: Color::rgb_u8(130, 130, 130),
            shadow_color: Color::TRANSPARENT,
            shadow_padding: ShadowPadding::uniform(0.0),
            height: 20.0,
            corner_diameter: CornerDiameter::uniform(0.0),
            font_size: FontSize(13.0),
            horizontal_alignment: HorizontalAlignment::Center { offset: 0.0, border: 4.0 },
            vertical_alignment: VerticalAlignment::Center { offset: 0.0 },
            overflow_behavior: OverflowBehavior::Shrink,
        };

        theme.state_button = StateButtonTheme {
            background_color: Color::rgb_u8(247, 247, 247),
            foreground_color: Color::rgb_u8(16, 16, 80),
            highlight_color: Color::rgb_u8(0, 0, 160),
            hovered_background_color: Color::rgb_u8(196, 209, 234),
            hovered_foreground_color: Color::WHITE,
            disabled_background_color: Color::rgb_u8(235, 235, 235),
            disabled_foreground_color: Color::rgb_u8(130, 130, 130),
            checkbox_color: Color::rgb_u8(60, 60, 60),
            hovered_checkbox_color: Color::rgb_u8(0, 0, 0),
            disabled_checkbox_color: Color::rgb_u8(150, 150, 150),
            shadow_color: Color::TRANSPARENT,
            shadow_padding: ShadowPadding::uniform(0.0),
            height: 20.0,
            corner_diameter: CornerDiameter::uniform(0.0),
            font_size: FontSize(13.0),
            horizontal_alignment: HorizontalAlignment::Left { offset: 26.0, border: 3.0 },
            vertical_alignment: VerticalAlignment::Center { offset: 0.0 },
            overflow_behavior: OverflowBehavior::Shrink,
        };

        theme.text_box = TextBoxTheme {
            background_color: Color::WHITE,
            foreground_color: Color::rgb_u8(0, 0, 0),
            highlight_color: Color::rgb_u8(0, 0, 160),
            hovered_background_color: Color::rgb_u8(255, 255, 240),
            hovered_foreground_color: Color::rgb_u8(0, 0, 0),
            focused_background_color: Color::WHITE,
            focused_foreground_color: Color::rgb_u8(0, 0, 0),
            ghost_foreground_color: Color::rgb_u8(120, 120, 120),
            hide_icon_color: Color::rgb_u8(90, 90, 90),
            hovered_hide_icon_color: Color::rgb_u8(0, 0, 0),
            shadow_color: Color::TRANSPARENT,
            shadow_padding: ShadowPadding::uniform(0.0),
            height: 20.0,
            corner_diameter: CornerDiameter::uniform(0.0),
            font_size: FontSize(13.0),
            horizontal_alignment: HorizontalAlignment::Left { offset: 6.0, border: 3.0 },
            vertical_alignment: VerticalAlignment::Center { offset: 0.0 },
            overflow_behavior: OverflowBehavior::LineBreak,
        };

        theme.collapsible = CollapsibleTheme {
            background_color: Color::rgb_u8(238, 238, 238),
            secondary_background_color: Color::rgb_u8(228, 228, 228),
            foreground_color: Color::rgb_u8(16, 16, 80),
            highlight_color: Color::rgb_u8(0, 0, 160),
            hovered_foreground_color: Color::rgb_u8(160, 30, 30),
            corner_diameter: CornerDiameter::uniform(2.0),
            icon_color: Color::rgb_u8(80, 80, 80),
            icon_size: 10.0,
            shadow_color: Color::TRANSPARENT,
            shadow_padding: ShadowPadding::uniform(0.0),
            gaps: 4.0,
            border: 5.0,
            title_height: 20.0,
            font_size: FontSize(13.0),
            horizontal_alignment: HorizontalAlignment::Left { offset: 0.0, border: 3.0 },
            vertical_alignment: VerticalAlignment::Center { offset: 0.0 },
            overflow_behavior: OverflowBehavior::Shrink,
        };

        theme.drop_down = DropDownTheme {
            item_background_color: Color::WHITE,
            item_foreground_color: Color::rgb_u8(0, 0, 0),
            item_highlight_color: Color::rgb_u8(0, 0, 160),
            item_hovered_background_color: Color::rgb_u8(196, 209, 234),
            item_hovered_foreground_color: Color::rgb_u8(0, 0, 0),
            item_shadow_color: Color::TRANSPARENT,
            item_shadow_padding: ShadowPadding::uniform(0.0),
            item_disabled_foreground_color: Color::rgb_u8(130, 130, 130),
            item_disabled_background_color: Color::rgb_u8(235, 235, 235),
            item_height: 20.0,
            item_corner_diameter: CornerDiameter::uniform(0.0),
            item_font_size: FontSize(13.0),
            item_horizontal_alignment: HorizontalAlignment::Center { offset: 0.0, border: 4.0 },
            item_vertical_alignment: VerticalAlignment::Center { offset: 0.0 },
            item_overflow_behavior: OverflowBehavior::Shrink,
            list_corner_diameter: CornerDiameter::uniform(0.0),
            list_background_color: Color::WHITE,
            list_shadow_color: Color::rgba_u8(0, 0, 0, 120),
            list_shadow_padding: ShadowPadding::diagonal(1.0, 2.0),
            list_gaps: 2.0,
            list_border: 3.0,
            list_maximum_height: 500.0,
            button_background_color: Color::rgb_u8(247, 247, 247),
            button_foreground_color: Color::rgb_u8(16, 16, 80),
            button_highlight_color: Color::rgb_u8(0, 0, 160),
            button_hovered_background_color: Color::rgb_u8(196, 209, 234),
            button_hovered_foreground_color: Color::WHITE,
            button_shadow_color: Color::TRANSPARENT,
            button_shadow_padding: ShadowPadding::uniform(0.0),
            button_height: 20.0,
            button_corner_diameter: CornerDiameter::uniform(0.0),
            button_font_size: FontSize(13.0),
            button_horizontal_alignment: HorizontalAlignment::Center { offset: 0.0, border: 4.0 },
            button_vertical_alignment: VerticalAlignment::Center { offset: 0.0 },
            button_overflow_behavior: OverflowBehavior::Shrink,
        };

        theme.field = FieldTheme {
            background_color: Color::rgb_u8(240, 240, 240),
            foreground_color: Color::rgb_u8(0, 0, 0),
            highlight_color: Color::rgb_u8(0, 0, 160),
            shadow_color: Color::TRANSPARENT,
            shadow_padding: ShadowPadding::uniform(0.0),
            height: 20.0,
            corner_diameter: CornerDiameter::uniform(0.0),
            font_size: FontSize(13.0),
            horizontal_alignment: HorizontalAlignment::Left { offset: 6.0, border: 3.0 },
            vertical_alignment: VerticalAlignment::Center { offset: 0.0 },
            overflow_behavior: OverflowBehavior::LineBreak,
        };

        theme.tooltip = TooltipTheme {
            background_color: Color::rgba_u8(240, 240, 250, 235),
            foreground_color: Color::rgb_u8(0, 0, 0),
            highlight_color: Color::rgb_u8(0, 0, 160),
            shadow_color: Color::rgba_u8(0, 0, 0, 80),
            shadow_padding: ShadowPadding::diagonal(1.0, 2.0),
            font_size: FontSize(13.0),
            corner_diameter: CornerDiameter::uniform(2.0),
            border: 5.0,
            gap: 3.0,
            mouse_offset: 16.0,
            overflow_behavior: OverflowBehavior::LineBreak,
        };

        theme.skill_tree = SkillTreeTheme {
            slot_background_color: Color::rgb_u8(245, 245, 245),
            slot_outline: ShadowPadding::uniform(1.0),
            requirement_color: Color::rgb_u8(120, 0, 0),
            hover_color: Color::rgb_u8(0, 0, 0),
            slot_corner_diameter: CornerDiameter::uniform(0.0),
            unoccupied_slot_color: Color::rgba_u8(210, 210, 210, 160),
            unoccupied_slot_outline: ShadowPadding::uniform(1.0),
            unlearned_skill_color: Color::rgb_u8(170, 170, 170),
            name_font_size: FontSize(12.0),
            name_text_color: Color::rgb_u8(0, 0, 0),
            highlight_color: Color::rgb_u8(0, 0, 160),
            points_color: Color::rgb_u8(0, 0, 0),
            pending_points_color: Color::rgb_u8(0, 0, 200),
            points_font_size: FontSize(12.0),
            arrow_color: Color::rgb_u8(80, 80, 80),
            lower_level_color: Color::rgb_u8(160, 0, 0),
        };

        // The chat window is drawn as a translucent overlay (it is excluded from the
        // window skin), so it keeps the dark classic chat background with light text.
        theme.chat = ChatTheme {
            window_color: Color::rgba_u8(0, 0, 0, 160),
            text_box_background_color: Color::rgba_u8(0, 0, 0, 120),
            transparent_window_color: Color::TRANSPARENT,
            transparent_text_box_background_color: Color::TRANSPARENT,
            window_title_color: Color::WHITE,
            hovered_window_title_color: Color::rgb_u8(255, 255, 220),
            text_box_foreground_color: Color::WHITE,
            // The classic skinned text boxes use black text on a light background, but the
            // chat input is drawn on its own dark translucent background, so it needs a light
            // focused color as well.
            text_box_focused_foreground_color: Color::WHITE,
            ghost_foreground_color: Color::rgb_u8(170, 170, 170),
        };

        theme.skin = SkinTheme {
            window: WindowSkin {
                frame: Some(image(SkinImage::WindowFrame, Slice::uniform(14.0))),
                title_bar: Some(image(SkinImage::WindowTitleBar, Slice::horizontal(12.0, 12.0))),
                close_button: Some(image(SkinImage::WindowCloseButton, Slice::NONE)),
                hovered_close_button: Some(image(SkinImage::WindowCloseButtonHovered, Slice::NONE)),
            },
            button: classic_button_skin(),
            state_button: StateButtonSkin {
                background: classic_button_skin(),
                unchecked: Some(image(SkinImage::CheckboxUnchecked, Slice::NONE)),
                checked: Some(image(SkinImage::CheckboxChecked, Slice::NONE)),
            },
            drop_down: DropDownSkin {
                button: classic_button_skin(),
            },
            text_box: Some(image(SkinImage::TextBox, Slice::uniform(2.0))),
            field: Some(image(SkinImage::TextBox, Slice::uniform(2.0))),
            tooltip: None,
        };

        theme
    }
}

/// Create a classic skin image with a white tint.
fn image(image: SkinImage, slices: Slice) -> ImageSlice<ClientState> {
    ImageSlice {
        image,
        color: Color::WHITE,
        slices,
        smooth: false,
    }
}

/// Create the classic button skin used for regular and toggle buttons.
fn classic_button_skin() -> ButtonSkin<ClientState> {
    ButtonSkin {
        normal: Some(image(SkinImage::ButtonNormal, Slice::horizontal(6.0, 6.0))),
        hovered: Some(image(SkinImage::ButtonHovered, Slice::horizontal(6.0, 6.0))),
        pressed: Some(image(SkinImage::ButtonPressed, Slice::horizontal(6.0, 6.0))),
        disabled: Some(ImageSlice {
            image: SkinImage::ButtonNormal,
            color: Color::rgb_u8(180, 180, 180),
            slices: Slice::horizontal(6.0, 6.0),
            smooth: false,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{CharacterOverviewTheme, InterfaceTheme, InterfaceThemeType};

    /// Removes a single field (including its value) from a serialized RON
    /// struct.
    ///
    /// Used to emulate a theme file that was written before the character
    /// overview bars (and therefore the `character_overview` field) existed.
    fn remove_field(serialized: &str, field: &str) -> String {
        let key = format!("{field}:");
        let start = serialized.find(&key).expect("field exists in serialized theme");

        // Find the end of the field value by balancing the parentheses of the
        // nested struct. RON structs never contain parentheses inside strings
        // for the theme values, so a simple counter is enough here.
        let mut depth = 0usize;
        let mut end = None;

        for (offset, character) in serialized[start..].char_indices() {
            match character {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(start + offset + 1);
                        break;
                    }
                }
                _ => {}
            }
        }

        let end = end.expect("field value is terminated");

        // Also remove the comma that separated the field from the previous one.
        let mut start = start;
        while start > 0 && serialized.as_bytes()[start - 1].is_ascii_whitespace() {
            start -= 1;
        }
        if start > 0 && serialized.as_bytes()[start - 1] == b',' {
            start -= 1;
        }

        format!("{}{}", &serialized[..start], &serialized[end..])
    }

    #[test]
    fn theme_files_without_character_overview_still_load() {
        let theme = InterfaceTheme::default_in_game();
        let serialized = ron::ser::to_string_pretty(&theme, ron::ser::PrettyConfig::new()).unwrap();

        let old_serialized = remove_field(&serialized, "character_overview");
        assert!(!old_serialized.contains("character_overview"));

        let old_theme: InterfaceTheme = ron::from_str(&old_serialized).expect("old theme file must still load");

        assert_eq!(
            old_theme.character_overview.health_color,
            CharacterOverviewTheme::default().health_color
        );
    }

    /// Relative luminance of a color as perceived on screen (0.0 = black, 1.0 =
    /// white).
    fn luminance(color: crate::graphics::Color) -> f32 {
        0.2126 * color.red + 0.7152 * color.green + 0.0722 * color.blue
    }

    #[test]
    fn chat_input_focused_text_is_readable_in_all_themes() {
        // The focused chat input is always drawn on a dark background, so every theme
        // must provide a light focused text color, even the classic theme whose
        // skinned text boxes use black text on a light background.
        let themes = [
            ("default menu", InterfaceTheme::default_menu()),
            ("default in-game", InterfaceTheme::default_in_game()),
            (
                "classic",
                InterfaceTheme::load(InterfaceThemeType::InGame, crate::settings::CLASSIC_THEME_NAME),
            ),
        ];

        for (name, theme) in themes {
            let focused_text = theme.chat.text_box_focused_foreground_color;
            assert!(
                luminance(focused_text) > 0.7,
                "{name}: chat input focused text {focused_text:?} is too dark for the dark input background"
            );

            let ghost_text = theme.chat.ghost_foreground_color;
            assert!(
                luminance(ghost_text) > 0.4,
                "{name}: chat input placeholder {ghost_text:?} is too dark to read"
            );
        }

        // Regression: the classic theme used to fall back to its light text box theme,
        // whose focused foreground color is black.
        let classic = InterfaceTheme::load(InterfaceThemeType::InGame, crate::settings::CLASSIC_THEME_NAME);
        assert_ne!(
            classic.chat.text_box_focused_foreground_color,
            classic.text_box.focused_foreground_color
        );
    }

    #[test]
    fn theme_files_without_chat_input_focused_color_still_load() {
        let theme = InterfaceTheme::default_in_game();
        let serialized = ron::ser::to_string_pretty(&theme, ron::ser::PrettyConfig::new()).unwrap();

        let old_serialized = remove_field(&serialized, "text_box_focused_foreground_color");
        assert!(!old_serialized.contains("text_box_focused_foreground_color"));

        let old_theme: InterfaceTheme = ron::from_str(&old_serialized).expect("old theme file must still load");

        assert_eq!(
            old_theme.chat.text_box_focused_foreground_color,
            super::default_chat_input_focused_foreground_color()
        );
    }

    #[test]
    fn classic_theme_has_readable_bar_colors() {
        let classic = InterfaceTheme::load(InterfaceThemeType::InGame, crate::settings::CLASSIC_THEME_NAME);
        let default = InterfaceTheme::default_in_game();

        // The classic theme uses the light interface, so the labels have to be
        // dark and the bars must not reuse the dark theme colors.
        assert_ne!(classic.character_overview.label_color, default.character_overview.label_color);
        assert_ne!(classic.character_overview.health_color, default.character_overview.health_color);
    }

    #[test]
    fn default_theme_characters_overview_is_the_default_of_the_struct() {
        let theme = InterfaceTheme::default_in_game();
        let expected = CharacterOverviewTheme::default();

        assert_eq!(theme.character_overview.health_color, expected.health_color);
        assert_eq!(theme.character_overview.spell_point_color, expected.spell_point_color);
        assert_eq!(theme.character_overview.experience_color, expected.experience_color);
    }
}
