//! Hotbar window: one slim row of ten large skill slots with a round
//! transparency toggle at the right end.
//!
//! The window is a fixed-size HUD element: it is opened at the bottom center of
//! the screen (the client positions it, so it stays centered at every
//! resolution and interface scale) and has no title bar, so the row is as slim
//! as possible. The transparency toggle only removes the window background and
//! chrome; the slot cards, skill icons and the toggle itself stay opaque.

use korangar_interface::application::Application;
use korangar_components::skill_box;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::Element;
use korangar_interface::event::{ClickHandler, EventQueue};
use korangar_interface::layout::area::PartialArea;
use korangar_interface::layout::{Resolver, Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window, WindowTheme};
use ragnarok_packets::HotbarSlot;
use rust_state::{ArrayLookupExt, OptionExt, Path, PathExt, Selector, State};

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::interface::resource::SkillSource;
use crate::interface::windows::WindowClass;
use crate::settings::{InterfaceSettings, InterfaceSettingsPathExt};
use crate::state::hotbar::VISIBLE_HOTBAR_SLOTS;
use crate::state::localization::LocalizationPathExt;
use crate::state::skills::{LearnableSkill, LearnedSkill, LearnedSkillPath};
use crate::state::theme::{InterfaceThemePathExt, InterfaceThemeType};
use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme};

/// Size of one hotbar slot in interface pixels.
///
/// The default slot size of the shared skill box component is 40 pixels; the
/// hotbar uses 2.5 times that so the icons are large enough to be used as the
/// main skill bar.
pub const HOTBAR_SLOT_SIZE: f32 = 100.0;
/// Clickable size of the round transparency toggle in interface pixels.
pub const HOTBAR_TOGGLE_SIZE: f32 = 20.0;
/// Gap between the last slot and the transparency toggle.
pub const HOTBAR_TOGGLE_GAP: f32 = 4.0;
/// Width of the whole hotbar row in interface pixels.
///
/// Ten large slots plus the round toggle. At the maximum supported interface
/// scale (1.25) the row is 1280 screen pixels wide, which is exactly the width
/// of a 1280x720 window.
pub const HOTBAR_ROW_WIDTH: f32 =
    VISIBLE_HOTBAR_SLOTS as f32 * HOTBAR_SLOT_SIZE + HOTBAR_TOGGLE_GAP + HOTBAR_TOGGLE_SIZE;

pub struct HotbarWindow<A, B> {
    hotbar_path: A,
    skills_path: B,
}

impl<A, B> HotbarWindow<A, B> {
    pub fn new(hotbar_path: A, skills_path: B) -> Self {
        Self { hotbar_path, skills_path }
    }
}

/// Cell resolvers for the hotbar row.
///
/// Every slot gets a fixed `HOTBAR_SLOT_SIZE` x `HOTBAR_SLOT_SIZE` cell and the
/// last child (the transparency toggle) gets the remaining cell at the right
/// end. The row height is fixed, so `give_back` is a no-op and the row element
/// commits the height itself.
struct HotbarCellResolvers<'a, App: Application> {
    slots_left: f32,
    top: f32,
    toggle_left: f32,
    toggle_top: f32,
    text_layouter: &'a App::TextLayouter,
}

impl<'a, App: Application> Resolvers<'a, App> for HotbarCellResolvers<'a, App> {
    fn for_index(&self, index: usize) -> Resolver<'a, App> {
        let area = match index < VISIBLE_HOTBAR_SLOTS {
            true => PartialArea {
                left: self.slots_left + index as f32 * HOTBAR_SLOT_SIZE,
                top: self.top,
                width: HOTBAR_SLOT_SIZE,
                height: Some(HOTBAR_SLOT_SIZE),
            },
            false => PartialArea {
                left: self.toggle_left,
                top: self.toggle_top,
                width: HOTBAR_TOGGLE_SIZE,
                height: Some(HOTBAR_TOGGLE_SIZE),
            },
        };

        Resolver::new(area, 0.0, self.text_layouter)
    }

    fn give_back(&mut self, _: Resolver<'a, App>) {}
}

struct HotbarRowLayoutInfo<Slots, Toggle> {
    slots: Slots,
    toggle: Toggle,
}

/// One slim row: the ten skill slots and the round transparency toggle.
struct HotbarRow<Slots, Toggle> {
    slots: Slots,
    toggle: Toggle,
}

impl<Slots, Toggle> HotbarRow<Slots, Toggle> {
    fn new(slots: Slots, toggle: Toggle) -> Self {
        Self { slots, toggle }
    }
}

impl<Slots, Toggle> Element<ClientState> for HotbarRow<Slots, Toggle>
where
    Slots: Element<ClientState>,
    Toggle: Element<ClientState>,
{
    type LayoutInfo = HotbarRowLayoutInfo<Slots::LayoutInfo, Toggle::LayoutInfo>;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let available_area = resolver.push_available_area();
            let toggle_top = available_area.top + (HOTBAR_SLOT_SIZE - HOTBAR_TOGGLE_SIZE) / 2.0;

            let mut cell_resolvers = HotbarCellResolvers::<ClientState> {
                slots_left: available_area.left,
                top: available_area.top,
                toggle_left: available_area.left
                    + VISIBLE_HOTBAR_SLOTS as f32 * HOTBAR_SLOT_SIZE
                    + HOTBAR_TOGGLE_GAP,
                toggle_top,
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

            resolver.commit_used_height(HOTBAR_SLOT_SIZE);

            HotbarRowLayoutInfo { slots, toggle }
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

/// Build a selector for a hotbar window value that depends on the "transparent
/// hotbar" setting.
///
/// In transparent mode the window background, border and shadow are replaced by
/// fully transparent values, so only the slot cards, skill icons and the
/// toggle are visible. The normal values are taken from the active theme, which
/// makes this work for the default and the classic theme alike.
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
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| match *transparent_path.follow_safe(state) {
        true => transparent_value,
        false => select(theme_path.follow_safe(state)),
    })
}

impl<A, B> CustomWindow<ClientState> for HotbarWindow<A, B>
where
    A: Path<ClientState, [Option<LearnableSkill>; VISIBLE_HOTBAR_SLOTS]>,
    B: Path<ClientState, Vec<LearnedSkill>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Hotbar)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let settings_path = client_state().interface_settings();
        let transparent_path = settings_path.transparent_hotbar();
        let window_theme_path = client_theme().window();

        let background_color =
            hotbar_window_value(window_theme_path, transparent_path, |window| window.background_color, Color::TRANSPARENT);
        let shadow_color = hotbar_window_value(window_theme_path, transparent_path, |window| window.shadow_color, Color::TRANSPARENT);
        let shadow_padding = hotbar_window_value(
            window_theme_path,
            transparent_path,
            |window| window.shadow_padding,
            ShadowPadding::uniform(0.0),
        );
        let border = hotbar_window_value(window_theme_path, transparent_path, |window| window.border, 0.0);
        let corner_diameter = hotbar_window_value(
            window_theme_path,
            transparent_path,
            |window| window.corner_diameter,
            CornerDiameter::uniform(0.0),
        );

        let slots = std::array::from_fn::<_, VISIBLE_HOTBAR_SLOTS, _>(|slot| {
            let learnable_skill_path = self.hotbar_path.array_index(slot).unwrapped();
            let learned_skill_path = LearnedSkillPath::new(learnable_skill_path, self.skills_path);

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

        window! {
            title: client_state().localization().hotbar_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            // Slim HUD row: no title bar, no window padding, only the slots.
            title_height: 0.0,
            title_gap: 0.0,
            gaps: 0.0,
            background_color: background_color,
            shadow_color: shadow_color,
            shadow_padding: shadow_padding,
            border: border,
            corner_diameter: corner_diameter,
            // The frame is color based so that the transparent mode works in
            // every theme and the background can be hidden without hiding the
            // opaque slot cards.
            skinned: false,
            closable: false,
            resizable: false,
            minimum_width: HOTBAR_ROW_WIDTH,
            maximum_width: HOTBAR_ROW_WIDTH,
            minimum_height: HOTBAR_SLOT_SIZE,
            maximum_height: HOTBAR_SLOT_SIZE,
            elements: (
                HotbarRow::new(slots, transparency_toggle),
            ),
        }
    }
}
