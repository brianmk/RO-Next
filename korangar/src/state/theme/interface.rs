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

use crate::graphics::{Color, CornerDiameter, ScreenSize, ShadowPadding};
use crate::loaders::{FontSize, OverflowBehavior};
use crate::renderer::skin::SkinImage;
use crate::settings::{CLASSIC_THEME_NAME, IN_GAME_THEMES_PATH, MENU_THEMES_PATH};
use crate::state::ClientState;

#[derive(Default, Debug, Clone, Copy)]
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
    window_color: Color,
    text_box_background_color: Color,
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
}

impl InterfaceTheme {
    pub fn load(theme_type: InterfaceThemeType, name: &str) -> Self {
        #[cfg(feature = "debug")]
        let timer = Timer::new("Load theme");

        if name == CLASSIC_THEME_NAME {
            return Self::default_classic();
        }

        let path = match theme_type {
            InterfaceThemeType::InGame => format!("{IN_GAME_THEMES_PATH}/{name}.ron"),
            InterfaceThemeType::Menu => format!("{MENU_THEMES_PATH}/{name}.ron"),
        };

        #[cfg(feature = "debug")]
        print_debug!("loading theme from file {}", path.magenta());

        let theme = std::fs::read_to_string(&path)
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
            .unwrap_or_else(|| {
                #[cfg(feature = "debug")]
                print_debug!("[{}] failed to load theme {}", "error".red(), name.magenta());

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
