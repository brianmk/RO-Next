use ro_next_interface::element::Element;
use ro_next_interface::element::store::{ElementStore, ElementStoreMut};
use ro_next_interface::event::{ClickHandler, EventQueue};
use ro_next_interface::layout::area::Area;
use ro_next_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use ro_next_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use ro_next_interface::window::{CustomWindow, Window};
use rust_state::{Path, State};

use crate::graphics::{CornerDiameter, ShadowPadding};
use crate::input::keybinds::{self, ALL_SHORTCUT_ACTIONS};
use crate::input::{InputEvent, ShortcutAction};
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::settings::{InterfaceSettings, InterfaceSettingsCapabilities, InterfaceSettingsCapabilitiesPathExt, InterfaceSettingsPathExt};
use crate::state::localization::{LocalizationPathExt, QolTextsPathExt};
use crate::state::theme::{ColorTokensPathExt, DesignTokensPathExt, InterfaceThemePathExt, InterfaceThemeType, TypographyTokensPathExt};
use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme};

/// Height of one key binding row in interface pixels.
const KEYBIND_ROW_HEIGHT: f32 = 22.0;
/// Number of columns of the key binding list.
const KEYBIND_COLUMNS: usize = 2;

/// Click handler that starts capturing a new key for an action.
struct BeginRebindHandler {
    action: ShortcutAction,
}

impl ClickHandler<ClientState> for BeginRebindHandler {
    fn handle_click(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        queue.queue(InputEvent::BeginRebind { action: self.action });
    }
}

/// One row of the key binding editor.
struct KeybindRow {
    label: String,
    key: String,
    capturing: bool,
}

/// List of the interface shortcuts and their current keys.
///
/// Clicking a row starts capturing the next key press in the input system. The
/// currently captured row shows the localized "press a key" hint instead of
/// its binding.
#[derive(Default)]
struct KeybindEditor {
    rows: Vec<KeybindRow>,
    handlers: Vec<BeginRebindHandler>,
}

impl Element<ClientState> for KeybindEditor {
    type LayoutInfo = Area;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        // The labels are strings that have to live as long as the layout, so
        // they are prepared here and not inside `lay_out`.
        let capturing_action = state.follow(client_state().key_rebind()).capturing();
        let capturing_text = state.get(&client_state().localization().qol().keybind_press_key_text()).clone();

        self.rows = ALL_SHORTCUT_ACTIONS
            .iter()
            .copied()
            .map(|action| {
                let capturing = capturing_action == Some(action);

                KeybindRow {
                    label: action.label().to_owned(),
                    key: match capturing {
                        true => capturing_text.clone(),
                        false => keybinds::binding_label(action),
                    },
                    capturing,
                }
            })
            .collect();
        self.handlers = ALL_SHORTCUT_ACTIONS
            .iter()
            .copied()
            .map(|action| BeginRebindHandler { action })
            .collect();

        with_single_resolver(resolvers, |resolver| {
            let rows_per_column = self.rows.len().div_ceil(KEYBIND_COLUMNS);

            resolver.with_height(KEYBIND_ROW_HEIGHT * rows_per_column as f32)
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        area: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let tokens = client_theme().tokens();
        let typography = tokens.typography();
        let color_tokens = tokens.color();
        let label_font_size = *state.get(&typography.font_body());
        let key_font_size = *state.get(&typography.font_sm());
        let label_color = *state.get(&color_tokens.text_primary());
        let key_color = *state.get(&color_tokens.text_value());
        let row_background = *state.get(&color_tokens.surface_raised());
        let row_border = *state.get(&color_tokens.border());
        let accent_color = *state.get(&color_tokens.text_accent());

        let rows_per_column = self.rows.len().div_ceil(KEYBIND_COLUMNS);
        let column_width = area.width / KEYBIND_COLUMNS as f32;

        for (index, row) in self.rows.iter().enumerate() {
            let column = index / rows_per_column;
            let row_in_column = index % rows_per_column;
            let left = area.left + column as f32 * column_width;
            let top = area.top + row_in_column as f32 * KEYBIND_ROW_HEIGHT;
            let row_area = Area {
                left,
                top,
                width: column_width - 8.0,
                height: KEYBIND_ROW_HEIGHT,
            };

            if row_area.check().run(layout) {
                layout.register_click_handler(ro_next_interface::layout::MouseButton::Left, &self.handlers[index]);
            }

            let key_area = Area {
                left: left + column_width - 8.0 - 112.0,
                top: top + 2.0,
                width: 108.0,
                height: KEYBIND_ROW_HEIGHT - 4.0,
            };

            layout.add_rectangle(
                key_area,
                CornerDiameter::uniform(4.0),
                row_background,
                row_border,
                ShadowPadding::uniform(0.0),
            );

            let (key_text, key_text_color) = match row.capturing {
                true => (row.key.as_str(), accent_color),
                false => (row.key.as_str(), key_color),
            };

            layout.add_text(
                row_area,
                &row.label,
                label_font_size,
                label_color,
                label_color,
                HorizontalAlignment::Left { offset: 2.0, border: 0.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );

            layout.add_text(
                key_area,
                key_text,
                key_font_size,
                key_text_color,
                key_text_color,
                HorizontalAlignment::Center { offset: 0.0, border: 0.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );
        }
    }
}

pub struct InterfaceSettingsWindow<A, B> {
    settings_path: A,
    capabilities_path: B,
}

impl<A, B> InterfaceSettingsWindow<A, B> {
    pub fn new(settings_path: A, capabilities_path: B) -> Self {
        Self {
            settings_path,
            capabilities_path,
        }
    }
}

impl<A, B> CustomWindow<ClientState> for InterfaceSettingsWindow<A, B>
where
    A: Path<ClientState, InterfaceSettings>,
    B: Path<ClientState, InterfaceSettingsCapabilities>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::InterfaceSettings)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        crate::logging::log_trace!("interface_settings: building Interface Settings window");

        let elements = (
            split! {
                children: (
                    text! {
                        text: client_state().localization().language_text(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.language(),
                        options: self.capabilities_path.languages(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: client_state().localization().scaling_text(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.scaling(),
                        options: self.capabilities_path.scalings(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: client_state().localization().menu_theme_text(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.menu_theme(),
                        options: self.capabilities_path.menu_themes(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: client_state().localization().in_game_theme_text(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.in_game_theme(),
                        options: self.capabilities_path.in_game_themes(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: client_state().localization().world_theme_text(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.world_theme(),
                        options: self.capabilities_path.world_themes(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: client_state().localization().level_99_aura_text(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.level_99_aura_style(),
                        options: self.capabilities_path.level_99_aura_styles(),
                    }
                )
            },
            state_button! {
                text: "Double hotbar",
                state: self.settings_path.double_hotbar(),
                event: Toggle(self.settings_path.double_hotbar()),
            },
            state_button! {
                text: client_state().localization().show_character_names_text(),
                state: self.settings_path.show_character_names(),
                event: Toggle(self.settings_path.show_character_names()),
            },
            state_button! {
                text: client_state().localization().show_chat_bubbles_text(),
                state: self.settings_path.show_chat_bubbles(),
                event: Toggle(self.settings_path.show_chat_bubbles()),
            },
            state_button! {
                text: client_state().localization().chat_bubble_background_text(),
                state: self.settings_path.chat_bubble_background(),
                event: Toggle(self.settings_path.chat_bubble_background()),
            },
            state_button! {
                text: client_state().localization().show_skill_names_text(),
                state: self.settings_path.show_skill_names_above_characters(),
                event: Toggle(self.settings_path.show_skill_names_above_characters()),
            },
            text! {
                text: client_state().localization().qol().keybindings_text(),
                height: 18.0,
                color: client_theme().tokens().color().text_accent(),
                overflow_behavior: OverflowBehavior::Shrink,
            },
            text! {
                text: client_state().localization().qol().keybind_hint_text(),
                height: 18.0,
                color: client_theme().tokens().color().text_hint(),
                overflow_behavior: OverflowBehavior::Shrink,
            },
            KeybindEditor::default(),
            button! {
                text: client_state().localization().qol().keybind_reset_button_text(),
                event: InputEvent::ResetKeybinds,
            },
        );

        crate::logging::log_trace!("interface_settings: constructed Interface Settings elements, creating window description");

        window! {
            title: client_state().localization().interface_settings_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            resizable: true,
            elements,
        }
    }
}
