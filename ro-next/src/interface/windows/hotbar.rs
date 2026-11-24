//! Hotbar window: two slim rows of nine large skill slots with the
//! transparency toggle and the bar switch on the right.
//!
//! The 2019 server keeps a hotbar as four tabs with 38 entries each (four rows
//! of nine, then two quick slots). The window draws two visible rows of the
//! active tab; `Ctrl+F12` rotates the visible rows through the four regular
//! rows of the bar and the bar switch swaps the two server bars (tabs 0 and
//! 1). `F12` hides and shows the whole window (handled by the input layer).
//!
//! The window is a fixed-size HUD element: it is opened at the bottom center of
//! the screen (the client positions it, so it stays centered at every
//! resolution and interface scale) and has no title bar, so the rows are as
//! slim as possible. The transparency toggle only removes the window
//! background and chrome; the slot cards, skill icons and the toggles stay
//! opaque.

use ragnarok_packets::HotbarSlot;
use ro_next_components::skill_box;
use ro_next_interface::application::Application;
use ro_next_interface::element::Element;
use ro_next_interface::element::store::{ElementStore, ElementStoreMut};
use ro_next_interface::event::{ClickHandler, EventQueue};
use ro_next_interface::layout::area::PartialArea;
use ro_next_interface::layout::tooltip::TooltipExt;
use ro_next_interface::layout::{Resolver, Resolvers, WindowLayout, with_single_resolver};
use ro_next_interface::window::{CustomWindow, Window, WindowTheme};
use rust_state::{ArrayLookupExt, OptionExt, Path, PathExt, Selector, State};

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::input::InputEvent;
use crate::interface::resource::SkillSource;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::settings::{InterfaceSettings, InterfaceSettingsPathExt};
use crate::state::hotbar::{HOTBAR_SLOTS_PER_ROW, Hotbar, VISIBLE_HOTBAR_SLOTS};
use crate::state::localization::{LocalizationPathExt, QolTextsPathExt};
use crate::state::skills::{LearnableSkill, LearnedSkill, LearnedSkillPath};
use crate::state::theme::{ColorTokensPathExt, DesignTokensPathExt, InterfaceThemePathExt, InterfaceThemeType, TypographyTokensPathExt};
use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme};

/// Size of one hotbar slot in interface pixels.
pub const HOTBAR_SLOT_SIZE: f32 = 52.0;
/// Gap between two slots (also shows the window background, which makes the
/// transparency toggle visible between the opaque slot cards).
pub const HOTBAR_SLOT_GAP: f32 = 1.7;
/// Clickable size of the round transparency toggle in interface pixels.
pub const HOTBAR_TOGGLE_SIZE: f32 = 17.0;
/// Size of the bar switch button (bar 1 / bar 2) in interface pixels.
pub const HOTBAR_BAR_SWITCH_SIZE: f32 = 17.0;
/// Horizontal gap between the slot grid and the right-hand controls.
pub const HOTBAR_CONTROLS_GAP: f32 = 4.0;
/// Small padding around the slot rows, so the window background is visible
/// above and below the cards (and disappears in transparent mode).
pub const HOTBAR_PADDING: f32 = 1.7;

/// Width of one row of slots.
const HOTBAR_ROW_WIDTH: f32 = HOTBAR_SLOTS_PER_ROW as f32 * HOTBAR_SLOT_SIZE + (HOTBAR_SLOTS_PER_ROW - 1) as f32 * HOTBAR_SLOT_GAP;
/// Width of the whole hotbar in interface pixels (two rows plus the controls).
pub const HOTBAR_WINDOW_WIDTH: f32 = HOTBAR_ROW_WIDTH + HOTBAR_CONTROLS_GAP + HOTBAR_TOGGLE_SIZE;
/// Height of the whole hotbar window in interface pixels.
pub const HOTBAR_WINDOW_HEIGHT: f32 = 2.0 * HOTBAR_SLOT_SIZE + HOTBAR_SLOT_GAP + HOTBAR_PADDING * 2.0;

pub struct HotbarWindow<A, B, C> {
    skills_path: A,
    learned_skills_path: B,
    hotbar_path: C,
}

impl<A, B, C> HotbarWindow<A, B, C> {
    pub fn new(skills_path: A, learned_skills_path: B, hotbar_path: C) -> Self {
        Self {
            skills_path,
            learned_skills_path,
            hotbar_path,
        }
    }
}

/// Cell resolvers for the hotbar grid.
///
/// Every slot gets a fixed `HOTBAR_SLOT_SIZE` cell. The two right-hand cells
/// are the transparency toggle (top row) and the bar switch (bottom row).
struct HotbarCellResolvers<'a, App: Application> {
    slots_left: f32,
    row_top: f32,
    toggle_left: f32,
    toggle_top: f32,
    switch_left: f32,
    switch_top: f32,
    text_layouter: &'a App::TextLayouter,
}

impl<'a, App: Application> Resolvers<'a, App> for HotbarCellResolvers<'a, App> {
    fn for_index(&self, index: usize) -> Resolver<'a, App> {
        let area = match index < VISIBLE_HOTBAR_SLOTS {
            true => {
                let row = index / HOTBAR_SLOTS_PER_ROW;
                let column = index % HOTBAR_SLOTS_PER_ROW;

                PartialArea {
                    left: self.slots_left + column as f32 * (HOTBAR_SLOT_SIZE + HOTBAR_SLOT_GAP),
                    top: self.row_top + row as f32 * (HOTBAR_SLOT_SIZE + HOTBAR_SLOT_GAP),
                    width: HOTBAR_SLOT_SIZE,
                    height: Some(HOTBAR_SLOT_SIZE),
                }
            }
            false if index == VISIBLE_HOTBAR_SLOTS => PartialArea {
                left: self.toggle_left,
                top: self.toggle_top,
                width: HOTBAR_TOGGLE_SIZE,
                height: Some(HOTBAR_TOGGLE_SIZE),
            },
            false => PartialArea {
                left: self.switch_left,
                top: self.switch_top,
                width: HOTBAR_BAR_SWITCH_SIZE,
                height: Some(HOTBAR_BAR_SWITCH_SIZE),
            },
        };

        Resolver::new(area, 0.0, self.text_layouter)
    }

    fn give_back(&mut self, _: Resolver<'a, App>) {}
}

struct HotbarRowsLayoutInfo<Slots, Toggle, Switch> {
    slots: Slots,
    toggle: Toggle,
    bar_switch: Switch,
}

/// The two skill rows, the transparency toggle and the bar switch.
struct HotbarRows<Slots, Toggle, Switch> {
    slots: Slots,
    toggle: Toggle,
    bar_switch: Switch,
}

impl<Slots, Toggle, Switch> HotbarRows<Slots, Toggle, Switch> {
    fn new(slots: Slots, toggle: Toggle, bar_switch: Switch) -> Self {
        Self { slots, toggle, bar_switch }
    }
}

impl<Slots, Toggle, Switch> Element<ClientState> for HotbarRows<Slots, Toggle, Switch>
where
    Slots: Element<ClientState>,
    Toggle: Element<ClientState>,
    Switch: Element<ClientState>,
{
    type LayoutInfo = HotbarRowsLayoutInfo<Slots::LayoutInfo, Toggle::LayoutInfo, Switch::LayoutInfo>;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let available_area = resolver.push_available_area();
            let row_top = available_area.top + HOTBAR_PADDING;
            let toggle_left = available_area.left + HOTBAR_ROW_WIDTH + HOTBAR_CONTROLS_GAP;
            let toggle_top = row_top + (HOTBAR_SLOT_SIZE - HOTBAR_TOGGLE_SIZE) / 2.0;
            let switch_top = row_top + HOTBAR_SLOT_SIZE + HOTBAR_SLOT_GAP + (HOTBAR_SLOT_SIZE - HOTBAR_BAR_SWITCH_SIZE) / 2.0;

            let mut cell_resolvers = HotbarCellResolvers::<ClientState> {
                slots_left: available_area.left,
                row_top,
                toggle_left,
                toggle_top,
                switch_left: toggle_left,
                switch_top,
                text_layouter: resolver.get_text_layouter(),
            };

            let slots = self.slots.create_layout_info(state, store.child_store(0), &mut cell_resolvers);

            let toggle_area = PartialArea {
                left: cell_resolvers.toggle_left,
                top: cell_resolvers.toggle_top,
                width: HOTBAR_TOGGLE_SIZE,
                height: Some(HOTBAR_TOGGLE_SIZE),
            };
            let mut toggle_resolver = Resolver::new(toggle_area, 0.0, resolver.get_text_layouter());
            let toggle = self.toggle.create_layout_info(state, store.child_store(1), &mut toggle_resolver);

            let switch_area = PartialArea {
                left: cell_resolvers.switch_left,
                top: cell_resolvers.switch_top,
                width: HOTBAR_BAR_SWITCH_SIZE,
                height: Some(HOTBAR_BAR_SWITCH_SIZE),
            };
            let mut switch_resolver = Resolver::new(switch_area, 0.0, resolver.get_text_layouter());
            let bar_switch = self
                .bar_switch
                .create_layout_info(state, store.child_store(2), &mut switch_resolver);

            resolver.commit_used_height(HOTBAR_WINDOW_HEIGHT);

            HotbarRowsLayoutInfo { slots, toggle, bar_switch }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        self.slots.lay_out(state, store.child_store(0), &layout_info.slots, layout);
        self.toggle.lay_out(state, store.child_store(1), &layout_info.toggle, layout);
        self.bar_switch
            .lay_out(state, store.child_store(2), &layout_info.bar_switch, layout);
    }
}

/// Click handler for the hotbar transparency toggle.
///
/// Besides toggling the state (which is read every frame, so the change is
/// applied live) the interface settings are saved immediately, so the choice
/// survives crashes like the other interface settings.
struct TransparentHotbarToggle<A, B> {
    value_path: A,
    settings_path: B,
}

impl<A, B> ClickHandler<ClientState> for TransparentHotbarToggle<A, B>
where
    A: Path<ClientState, bool>,
    B: Path<ClientState, InterfaceSettings>,
{
    fn handle_click(&self, state: &State<ClientState>, _: &mut EventQueue<ClientState>) {
        state.update_value_with(self.value_path, |transparent| *transparent = !*transparent);
        state.update_value_with(self.settings_path, |settings| settings.save());
    }
}

/// Click handler for the bar switch (bar 1 / bar 2).
struct HotbarBarSwitchHandler;

impl ClickHandler<ClientState> for HotbarBarSwitchHandler {
    fn handle_click(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        queue.queue(InputEvent::SwitchHotbarBar);
    }
}

/// The bar switch button: shows the number of the active server bar and
/// switches between bar 1 and bar 2.
struct HotbarBarSwitch<A> {
    hotbar_path: A,
    tooltip: String,
}

impl<A> HotbarBarSwitch<A> {
    fn new(hotbar_path: A) -> Self {
        Self {
            hotbar_path,
            tooltip: String::new(),
        }
    }
}

impl<A> Element<ClientState> for HotbarBarSwitch<A>
where
    A: Path<ClientState, Hotbar>,
{
    type LayoutInfo = ro_next_interface::layout::area::Area;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        self.tooltip = state.get(&client_state().localization().qol().hotbar_bar_switch_tooltip()).clone();

        with_single_resolver(resolvers, |resolver| resolver.with_height(HOTBAR_BAR_SWITCH_SIZE))
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        area: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        use ro_next_interface::prelude::*;

        let color_tokens = client_theme().tokens().color();
        let typography = client_theme().tokens().typography();
        let active_tab = state.follow(self.hotbar_path).active_tab();
        let height = area.height.min(area.width);
        let box_area = ro_next_interface::layout::area::Area {
            left: area.left,
            top: area.top,
            width: area.width,
            height,
        };

        layout.add_rectangle(
            box_area,
            CornerDiameter::uniform(5.0),
            *state.get(&color_tokens.surface_raised()),
            *state.get(&color_tokens.border()),
            ShadowPadding::uniform(0.0),
        );

        layout.add_text(
            box_area,
            match active_tab {
                0 => "1",
                _ => "2",
            },
            *state.get(&typography.font_sm()),
            *state.get(&color_tokens.text_overlay()),
            *state.get(&color_tokens.text_overlay_accent()),
            HorizontalAlignment::Center { offset: 0.0, border: 0.0 },
            VerticalAlignment::Center { offset: 0.0 },
            OverflowBehavior::Shrink,
        );

        if box_area.check().run(layout) {
            layout.register_click_handler(ro_next_interface::layout::MouseButton::Left, &HotbarBarSwitchHandler);

            struct HotbarBarSwitchTooltip;
            layout.add_tooltip(&self.tooltip, HotbarBarSwitchTooltip.tooltip_id());
        }
    }
}

/// Build a selector for a hotbar window value that depends on the "transparent
/// hotbar" setting.
///
/// In transparent mode the window background, border and shadow are replaced by
/// fully transparent values, so only the slot cards, skill icons and the
/// toggles are visible. The normal values are taken from the active theme,
/// which makes this work for the default and the classic theme alike.
fn hotbar_window_value<Theme, Transparent, Value, Select>(
    theme_path: Theme,
    transparent_path: Transparent,
    select: Select,
    transparent_value: Value,
) -> impl Selector<ClientState, Value>
where
    Theme: Path<ClientState, WindowTheme<ClientState>> + Copy + 'static,
    Transparent: Path<ClientState, bool> + Copy + 'static,
    Value: Copy + Default + 'static,
    Select: Fn(&WindowTheme<ClientState>) -> Value + 'static,
{
    ro_next_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| match *transparent_path.follow_safe(state) {
        true => transparent_value,
        false => select(theme_path.follow_safe(state)),
    })
}

impl<A, B, C> CustomWindow<ClientState> for HotbarWindow<A, B, C>
where
    A: Path<ClientState, [Option<LearnableSkill>; VISIBLE_HOTBAR_SLOTS]>,
    B: Path<ClientState, Vec<LearnedSkill>>,
    C: Path<ClientState, Hotbar>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Hotbar)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        let settings_path = client_state().interface_settings();
        let transparent_path = settings_path.transparent_hotbar();
        let window_theme_path = client_theme().window();

        let background_color = hotbar_window_value(
            window_theme_path,
            transparent_path,
            |window| window.background_color,
            Color::TRANSPARENT,
        );
        let shadow_color = hotbar_window_value(
            window_theme_path,
            transparent_path,
            |window| window.shadow_color,
            Color::TRANSPARENT,
        );
        let shadow_padding = hotbar_window_value(
            window_theme_path,
            transparent_path,
            |window| window.shadow_padding,
            ShadowPadding::uniform(0.0),
        );
        let corner_diameter = hotbar_window_value(
            window_theme_path,
            transparent_path,
            |window| window.corner_diameter,
            CornerDiameter::uniform(0.0),
        );

        let slots = std::array::from_fn::<_, VISIBLE_HOTBAR_SLOTS, _>(|slot| {
            let learnable_skill_path = self.skills_path.array_index(slot).unwrapped();
            let learned_skill_path = LearnedSkillPath::new(learnable_skill_path, self.learned_skills_path);

            skill_box! {
                learnable_skill_path,
                learned_skill_path,
                source: SkillSource::Hotbar { slot: HotbarSlot(slot as u16) },
                size: HOTBAR_SLOT_SIZE,
                // The official client only shows the skill icon in the hotbar;
                // the level text belongs to the skill tree.
                show_level: false,
            }
        });

        let transparency_toggle = round_toggle! {
            tooltip: client_state().localization().hotbar_transparency_button_tooltip(),
            state: transparent_path,
            event: TransparentHotbarToggle {
                value_path: transparent_path,
                settings_path,
            },
            size: 14.0,
            hit_size: HOTBAR_TOGGLE_SIZE,
            alignment: HorizontalAlignment::Center { offset: 0.0, border: 0.0 },
            shadow_color: Color::TRANSPARENT,
            shadow_padding: ShadowPadding::uniform(0.0),
        };

        let bar_switch = HotbarBarSwitch::new(self.hotbar_path);

        window! {
            title: client_state().localization().hotbar_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            // Slim HUD element: no title bar, no window padding, only the rows.
            title_height: 0.0,
            title_gap: 0.0,
            gaps: 0.0,
            background_color: background_color,
            shadow_color: shadow_color,
            shadow_padding: shadow_padding,
            border: 0.0,
            corner_diameter: corner_diameter,
            // The frame is color based so that the transparent mode works in
            // every theme and the background can be hidden without hiding the
            // opaque slot cards.
            skinned: false,
            closable: false,
            resizable: false,
            minimum_width: HOTBAR_WINDOW_WIDTH,
            maximum_width: HOTBAR_WINDOW_WIDTH,
            minimum_height: HOTBAR_WINDOW_HEIGHT,
            maximum_height: HOTBAR_WINDOW_HEIGHT,
            elements: (
                HotbarRows::new(slots, transparency_toggle, bar_switch),
            ),
        }
    }
}
