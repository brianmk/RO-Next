use std::cell::UnsafeCell;
use std::sync::Arc;

use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox, ErasedElement, StateElement};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{Resolvers, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::{EntityId, SkillId};
use rust_state::{Path, RustState, State};

use super::WindowClass;
use crate::graphics::{Color, Texture};
use crate::input::InputEvent;
use crate::interface::markup::{MarkupClickHandler, MarkupLayout, layout_markup, render_markup};
use crate::renderer::LayoutExt;
use crate::state::chat_markup::{has_markup, parse_chat_markup};
use crate::state::localization::{LocalizationPathExt, UiTextsPathExt};
use crate::state::theme::{ColorTokensPathExt, DesignTokensPathExt, InterfaceThemePathExt, InterfaceThemeType, TypographyTokensPathExt};
use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme};

/// The kind of a single dialog element.
///
/// This is needed to remove whole groups of elements when the server sends a
/// new script packet: "Next"-buttons have to be removed when a "Close"- or a
/// menu packet arrives and the whole choice list of a menu has to be replaced
/// when a new menu arrives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DialogElementKind {
    /// A text line of the dialog transcript.
    Text,
    /// The "Next" button that advances to the next text page.
    NextButton,
    /// The "Close" button that ends the dialog.
    CloseButton,
    /// One option of the current NPC menu.
    Choice,
    /// The generic menu Cancel button (choice 255 / `-1`).
    CancelButton,
    /// The NPC text/number input field and its submit button.
    Input,
    /// The NPC illustration sent with `ZC_SHOW_IMAGE2` (0x01B3).
    Image,
}

/// A small wrapper struct that serves two purposes:
/// - Making the elements nicer to construct by putting the [`UnsafeCell::new`]
///   and [`Box::new`] behind a function call.
/// - Storing information about the kind of element so that we can remove whole
///   groups of elements individually.
#[derive(RustState, StateElement)]
pub struct DialogElement {
    /// Stores the UI element.
    // TODO: Unfortunately this has to be an unsafe cell as of now. Ideally this can be changed
    // later.
    #[hidden_element]
    element: UnsafeCell<ElementBox<ClientState>>,
    /// The kind of this element. Hidden from the state paths, only used
    /// internally to filter the element list.
    #[hidden_element]
    kind: DialogElementKind,
}

impl DialogElement {
    /// Creates a new dialog element.
    #[inline(always)]
    fn new<E>(element: E, kind: DialogElementKind) -> Self
    where
        E: Element<ClientState> + 'static,
    {
        Self {
            element: UnsafeCell::new(ErasedElement::new(element)),
            kind,
        }
    }
}

/// A dialog text line that contains interactive markup (item/NAVI/URL links).
///
/// Lines without markup keep using the normal `text!` component, so plain NPC
/// text is rendered exactly like before.
struct MarkupTextElement {
    text: String,
    click_handler: MarkupClickHandler,
}

struct MarkupTextLayout {
    area: korangar_interface::layout::area::Area,
    layout: MarkupLayout,
}

impl MarkupTextElement {
    fn new(text: String) -> Self {
        Self {
            text,
            click_handler: MarkupClickHandler::new(),
        }
    }
}

impl Element<ClientState> for MarkupTextElement {
    type LayoutInfo = MarkupTextLayout;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let runs = parse_chat_markup(&self.text);
            let tokens = client_theme().tokens();
            let typography = tokens.typography();
            let color_tokens = tokens.color();
            let font_size = *state.get(&typography.font_body());
            let color = *state.get(&color_tokens.text_primary());
            let highlight_color = *state.get(&color_tokens.text_accent());
            let available_width = resolver.push_available_area().width;

            let layout = layout_markup(
                resolver.get_text_layouter(),
                &runs,
                available_width,
                font_size,
                color,
                highlight_color,
            );
            let area = resolver.with_height(layout.height);

            MarkupTextLayout { area, layout }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut korangar_interface::layout::WindowLayout<'a, ClientState>,
    ) {
        let tokens = client_theme().tokens();
        let typography = tokens.typography();
        let color_tokens = tokens.color();
        let font_size = *state.get(&typography.font_body());
        let color = *state.get(&color_tokens.text_primary());
        let highlight_color = *state.get(&color_tokens.text_accent());
        let link_color = *state.get(&color_tokens.text_accent());

        render_markup(
            &layout_info.layout,
            layout_info.area,
            0.0,
            layout,
            font_size,
            color,
            highlight_color,
            link_color,
            &self.click_handler,
        );
    }
}

/// Maximum height of the illustration inside the dialog window.
const DIALOG_IMAGE_HEIGHT: f32 = 200.0;

/// The illustration the server sent with `ZC_SHOW_IMAGE2` (0x01B3).
///
/// The texture is loaded from the client archive by
/// `Client::load_cutin_texture`; the element only scales it into the dialog
/// width while preserving the aspect ratio.
struct DialogImage {
    texture: Arc<Texture>,
}

impl DialogImage {
    fn new(texture: Arc<Texture>) -> Self {
        Self { texture }
    }
}

impl Element<ClientState> for DialogImage {
    type LayoutInfo = Area;

    fn create_layout_info(
        &mut self,
        _state: &State<ClientState>,
        _store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| resolver.with_height(DIALOG_IMAGE_HEIGHT))
    }

    fn lay_out<'a>(
        &'a self,
        _state: &'a State<ClientState>,
        _store: ElementStore<'a>,
        area: &'a Self::LayoutInfo,
        layout: &mut korangar_interface::layout::WindowLayout<'a, ClientState>,
    ) {
        let size = self.texture.get_size();
        let texture_width = size.width.max(1) as f32;
        let texture_height = size.height.max(1) as f32;

        // Fit the illustration into the dialog without distorting it.
        let scale = (area.width / texture_width).min(area.height / texture_height);
        let width = texture_width * scale;
        let height = texture_height * scale;

        let image_area = Area {
            left: area.left + (area.width - width) / 2.0,
            top: area.top + (area.height - height) / 2.0,
            width,
            height,
        };

        layout.add_texture(image_area, self.texture.clone(), Color::WHITE, false);
    }
}

/// Internal state of the dialog window.
#[derive(RustState, StateElement)]
pub struct DialogWindowState {
    /// All current dialog elements.
    elements: Vec<DialogElement>,
    /// The entity id of the NPC the player is talking to.
    npc_id: EntityId,
    /// Whether or not the elements should be cleared the next time
    /// [`start`](Self::start) is called.
    clear_next: bool,
    /// Current text of the NPC input field (`ZC_OPEN_EDITDLG(STR)`).
    pub input_text: String,
    /// Whether the pending input expects a string (instead of a number).
    pub string_input: bool,
    /// When set, the choice buttons are a Warp Portal destination list
    /// (`ZC_WARPLIST`) instead of an NPC menu. The picked entry answers the
    /// server with `CZ_SELECT_WARPPOINT`, which then creates the portal units.
    #[hidden_element]
    warp_point: Option<WarpPointList>,
}

/// ZST for the focus id of the NPC input text box.
pub struct DialogInputBox;

/// Pending Warp Portal destination list of the dialog window.
pub struct WarpPointList {
    skill_id: SkillId,
    maps: Vec<String>,
}

/// Removes characters that can never be displayed in a dialog transcript.
///
/// NPC dialogs are server-sent strings (usually EUC-KR decoded, sometimes with
/// leftover Latin-1 bytes or control characters). The renderer itself replaces
/// every character that none of the loaded fonts can draw (the font coverage
/// gate in `loaders::font`; Korean is covered by the shipped `NanumGothic`
/// fallback), so this function only strips characters without a visible glyph
/// and normalizes line endings. The client's color markup (`^RRGGBB`) is pure
/// ASCII and is kept, so colored dialog lines keep their colors.
fn sanitize_dialog_text(text: &str) -> String {
    let mut sanitized = String::with_capacity(text.len());

    for character in text.chars() {
        match character {
            '\r' => {}
            '\n' | '\t' => sanitized.push(character),
            character if character.is_control() => {}
            character => sanitized.push(character),
        }
    }

    // A dialog line is never rendered with trailing whitespace; trimming it keeps
    // the layout (and the layout cache key) stable for server strings that end
    // with a stray space.
    sanitized.trim_end().to_owned()
}

impl DialogWindowState {
    /// Initialize the dialog. This is important so we have the correct entity
    /// id when sending packets to the server.
    pub fn initialize(&mut self, npc_id: EntityId) -> &mut Self {
        self.npc_id = npc_id;
        self.warp_point = None;
        self
    }

    /// Shows the NPC text/number input field (`ZC_OPEN_EDITDLG(STR)`).
    ///
    /// rAthena scripts that call `input()` or ask for a numeric amount wait for
    /// `CZ_INPUT_EDITDLG(STR)`; without this field the dialog would stall.
    pub fn start_input(&mut self, npc_id: EntityId, string_input: bool) {
        use korangar_interface::prelude::*;

        self.initialize(npc_id);
        self.elements.clear();
        self.clear_next = false;
        self.string_input = string_input;
        self.input_text.clear();

        let input_npc_id = self.npc_id;

        self.elements.push(DialogElement::new(
            text_box! {
                ghost_text: if string_input { "Enter text" } else { "Enter a number" },
                state: client_state().dialog_window().input_text(),
                input_handler: DefaultHandler::<_, _, 64>::new(
                    client_state().dialog_window().input_text(),
                    InputEvent::SubmitDialogInput {
                        npc_id: input_npc_id,
                        string_input,
                    },
                ),
                focus_id: DialogInputBox,
            },
            DialogElementKind::Input,
        ));

        self.elements.push(DialogElement::new(
            button! {
                text: "OK",
                event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                    queue.queue(InputEvent::SubmitDialogInput {
                        npc_id: input_npc_id,
                        string_input,
                    });
                },
            },
            DialogElementKind::Input,
        ));
    }

    /// Shows the Warp Portal destination list (`ZC_WARPLIST`).
    ///
    /// The server sends this when a Warp Portal (`AL_WARP`) finished casting;
    /// the portal is created only after the client answers with the chosen
    /// destination. The list replaces the current NPC dialog content, like the
    /// official client's separate warp point window.
    pub fn start_warp_point_list(&mut self, skill_id: SkillId, maps: Vec<String>) {
        use korangar_interface::prelude::*;

        self.elements.clear();
        self.clear_next = false;

        self.elements.push(DialogElement::new(
            text! {
                text: client_state().localization().ui().dialog_choose_warp_destination(),
            },
            DialogElementKind::Text,
        ));

        maps.iter().enumerate().for_each(|(index, map_name)| {
            let label = map_name.trim_end_matches(".gat").replace('_', " ");

            self.elements.push(DialogElement::new(
                button! {
                    text: label,
                    event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                        queue.queue(InputEvent::SelectWarpPoint { option: index });
                    },
                },
                DialogElementKind::Choice,
            ));
        });

        self.warp_point = Some(WarpPointList { skill_id, maps });
    }

    /// Destination of the `option`-th Warp Portal choice, if the window is
    /// currently showing a destination list.
    pub fn warp_point_destination(&self, option: usize) -> Option<(SkillId, String)> {
        let warp_point = self.warp_point.as_ref()?;
        let map_name = warp_point.maps.get(option)?.clone();

        Some((warp_point.skill_id, map_name))
    }

    /// Skill id of the pending Warp Portal destination list, if any.
    pub fn warp_point_skill(&self) -> Option<SkillId> {
        self.warp_point.as_ref().map(|warp_point| warp_point.skill_id)
    }

    /// Leaves the Warp Portal destination mode.
    pub fn clear_warp_point(&mut self) {
        self.warp_point = None;
    }

    /// Replace the dialog illustration (`ZC_SHOW_IMAGE2`, 0x01B3).
    ///
    /// A previous image is removed first, so the dialog never stacks multiple
    /// cut-ins; the element is placed above the text lines like the official
    /// client's cut-in window.
    pub fn set_image(&mut self, texture: Arc<Texture>) {
        self.elements.retain(|element| element.kind != DialogElementKind::Image);
        self.elements
            .insert(0, DialogElement::new(DialogImage::new(texture), DialogElementKind::Image));
    }

    /// Remove the dialog illustration (`ImageLocation::ClearAll`).
    pub fn clear_image(&mut self) {
        self.elements.retain(|element| element.kind != DialogElementKind::Image);
    }

    /// Whether the dialog currently shows an illustration.
    #[cfg(test)]
    pub fn has_image(&self) -> bool {
        self.elements.iter().any(|element| element.kind == DialogElementKind::Image)
    }

    /// Add text to the dialog.
    ///
    /// Characters that no loaded font can render are replaced with `?` by the
    /// text renderer, so this can never show `.notdef` squares; colored markup
    /// (`^RRGGBB`) is preserved.
    pub fn add_text(&mut self, text: String) {
        use korangar_interface::prelude::*;

        if self.clear_next {
            self.elements.clear();
            self.clear_next = false;
        }

        let text = sanitize_dialog_text(&text);

        if has_markup(&text) {
            self.elements
                .push(DialogElement::new(MarkupTextElement::new(text), DialogElementKind::Text));
        } else {
            self.elements.push(DialogElement::new(
                text! {
                    text: text,
                },
                DialogElementKind::Text,
            ));
        }
    }

    /// Add add next button to the dialog.
    ///
    /// This also sets the internal state to clear the dialog the next time
    /// [`start`](Self::start) is called.
    pub fn add_next_button(&mut self) {
        use korangar_interface::prelude::*;

        let npc_id = self.npc_id;

        self.elements.push(DialogElement::new(
            button! {
                text: client_state().localization().next_button_text(),
                event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                    queue.queue(InputEvent::NextDialog { npc_id });
                },
            },
            DialogElementKind::NextButton,
        ));

        self.clear_next = true;
    }

    /// Add a close button to the dialog.
    ///
    /// This also removes any existing "Next"-buttons and the options of an
    /// open menu, like the official client does when it receives a
    /// "Close"-packet.
    pub fn add_close_button(&mut self) {
        use korangar_interface::prelude::*;

        self.elements.retain(|element| {
            !matches!(
                element.kind,
                DialogElementKind::NextButton | DialogElementKind::Choice | DialogElementKind::CancelButton
            )
        });

        let npc_id = self.npc_id;

        self.elements.push(DialogElement::new(
            button! {
                text: client_state().localization().close_button_text(),
                event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                    queue.queue(InputEvent::CloseDialog { npc_id });
                },
            },
            DialogElementKind::CloseButton,
        ));
    }

    /// Add multiple buttons, one for each choice.
    ///
    /// The choice list of the current NPC menu is *replaced*: the official
    /// client closes and reopens its menu window when it receives a menu
    /// packet, so old options must never stay around below the new ones. Any
    /// "Next"-button of the dialog is removed as well.
    ///
    /// The remaining text lines of the dialog transcript and the "Close"
    /// button stay visible, just like in the official client where the dialog
    /// window and the menu window are separate windows.
    pub fn add_choice_buttons(&mut self, choices: Vec<String>) {
        use korangar_interface::prelude::*;

        self.elements
            .retain(|element| !matches!(element.kind, DialogElementKind::NextButton | DialogElementKind::Choice));

        let npc_id = self.npc_id;

        choices.into_iter().enumerate().for_each(|(index, text)| {
            // Menu options are single-line buttons; a line break would be laid
            // out inside the button, so it is turned into a space.
            let text = sanitize_dialog_text(&text).replace('\n', " ");

            self.elements.push(DialogElement::new(
                button! {
                    text: text,
                    event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                        queue.queue(InputEvent::ChooseDialogOption { npc_id, option: index as i8 + 1 });
                    },
                },
                DialogElementKind::Choice,
            ))
        });

        // Every NPC menu gets an explicit cancel button: the 2019 server
        // accepts choice 255 (i8 -1) and ends the menu; without it a menu with
        // no Back/Close entry (older custom warpers, for example) strands the
        // player because this client has no X on menu dialogs.
        self.elements.push(DialogElement::new(
            button! {
                text: "Cancel",
                event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                    queue.queue(InputEvent::ChooseDialogOption { npc_id, option: -1 });
                },
            },
            DialogElementKind::CancelButton,
        ));
    }

    /// Close the current NPC menu without ending the dialog.
    ///
    /// The official client closes its menu window as soon as the player picks
    /// an option, before the server answers. The dialog window itself stays
    /// open, so only the choice buttons are removed here.
    pub fn clear_choices(&mut self) {
        self.elements
            .retain(|element| !matches!(element.kind, DialogElementKind::Choice | DialogElementKind::CancelButton));
    }

    /// The number of elements of the given kind that are currently stored.
    #[cfg(test)]
    fn count_kind(&self, kind: DialogElementKind) -> usize {
        self.elements.iter().filter(|element| element.kind == kind).count()
    }

    /// Number of choice buttons currently displayed.
    #[cfg(test)]
    pub fn choice_count(&self) -> usize {
        self.count_kind(DialogElementKind::Choice)
    }

    /// Number of "Next"-buttons currently displayed.
    #[cfg(test)]
    pub fn next_button_count(&self) -> usize {
        self.count_kind(DialogElementKind::NextButton)
    }

    /// Number of menu Cancel buttons currently displayed.
    #[cfg(test)]
    pub fn cancel_button_count(&self) -> usize {
        self.count_kind(DialogElementKind::CancelButton)
    }

    /// Number of "Close"-buttons currently displayed.
    #[cfg(test)]
    pub fn close_button_count(&self) -> usize {
        self.count_kind(DialogElementKind::CloseButton)
    }

    /// Number of text lines currently displayed.
    #[cfg(test)]
    pub fn text_count(&self) -> usize {
        self.count_kind(DialogElementKind::Text)
    }

    /// End the dialog.
    ///
    /// This has no side effects.
    pub fn end(&mut self) {
        self.elements.clear();
        self.clear_next = false;
        self.warp_point = None;
    }
}

impl Default for DialogWindowState {
    fn default() -> Self {
        Self {
            elements: Default::default(),
            // Arguably not very clean but avoids using an Option.
            npc_id: EntityId(0),
            clear_next: false,
            input_text: String::new(),
            string_input: false,
            warp_point: None,
        }
    }
}

/// Wrapper struct for collecting all [`DialogElement::element`]s into a single
/// element.
struct InnerElement<A> {
    dialog_elements_path: A,
}

impl<A> Element<ClientState> for InnerElement<A>
where
    A: Path<ClientState, Vec<DialogElement>>,
{
    type LayoutInfo = ();

    fn create_layout_info(&mut self, state: &State<ClientState>, mut store: ElementStoreMut, resolvers: &mut dyn Resolvers<ClientState>) {
        with_single_resolver(resolvers, |resolver| {
            state
                .get(&self.dialog_elements_path)
                .iter()
                .enumerate()
                .for_each(|(index, dialog_element)| {
                    // We only create this mutable reference for the lifetime of this scope, and
                    // since nothing is captured from the element this is safe.
                    let element = unsafe { &mut *dialog_element.element.get() };

                    element.create_layout_info(state, store.child_store(index as u64), resolver)
                });
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: korangar_interface::element::store::ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut korangar_interface::layout::WindowLayout<'a, ClientState>,
    ) {
        state
            .get(&self.dialog_elements_path)
            .iter()
            .enumerate()
            .for_each(|(index, dialog_element)| {
                // There are no mutable references at this point in time and the immutable
                // reference will be dropped after the interface is rendered, making this safe.
                let element = unsafe { &*dialog_element.element.get() };

                element.lay_out(state, store.child_store(index as u64), &(), layout)
            });
    }
}

/// A window representing a dialog with an NPC.
pub struct DialogWindow<A> {
    /// Path to the [`DialogWindowState`].
    window_state_path: A,
}

impl<A> DialogWindow<A> {
    /// Creates a new dialog window.
    ///
    /// This does not modify the [`DialogWindowState`].
    pub fn new(window_state_path: A) -> Self {
        Self { window_state_path }
    }
}

impl<A> CustomWindow<ClientState> for DialogWindow<A>
where
    A: Path<ClientState, DialogWindowState>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Dialog)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: client_state().localization().dialog_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            // NPC menus (for example the warper destination lists) can be very
            // long. The dialog content is therefore scrollable and shows a
            // scroll bar when it does not fit into the window.
            elements: scroll_view! {
                scroll_bar: true,
                fit_content_height: true,
                children: InnerElement {
                    dialog_elements_path: self.window_state_path.elements(),
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_menu(options: &[&str]) -> DialogWindowState {
        let mut state = DialogWindowState::default();
        state.add_choice_buttons(options.iter().map(|option| option.to_string()).collect());
        state
    }

    #[test]
    fn a_new_menu_replaces_the_previous_menu_instead_of_appending_to_it() {
        let mut state = DialogWindowState::default();
        state.add_text("Where would you like to go?".to_owned());
        state.add_choice_buttons(vec!["Towns".to_owned(), "Fields".to_owned(), "Dungeons".to_owned()]);
        assert_eq!(state.choice_count(), 3);

        // The server answers a menu selection with another menu, for example
        // the town list of the warper.
        state.add_choice_buttons(vec!["Prontera".to_owned(), "Alberta".to_owned()]);

        assert_eq!(state.choice_count(), 2, "the old menu must not stay visible below the new one");
        assert_eq!(
            state.text_count(),
            1,
            "the transcript stays visible, like in the official client"
        );
    }

    #[test]
    fn picking_an_option_closes_the_menu_but_keeps_the_dialog() {
        let mut state = DialogWindowState::default();
        state.add_text("Choose.".to_owned());
        state.add_choice_buttons(vec!["A".to_owned(), "B".to_owned()]);

        state.clear_choices();

        assert_eq!(state.choice_count(), 0);
        assert_eq!(state.text_count(), 1);
    }

    #[test]
    fn a_new_menu_removes_the_next_button() {
        let mut state = DialogWindowState::default();
        state.add_text("First page.".to_owned());
        state.add_next_button();
        assert_eq!(state.next_button_count(), 1);

        state.add_choice_buttons(vec!["A".to_owned()]);

        assert_eq!(state.next_button_count(), 0);
        assert_eq!(state.choice_count(), 1);
        assert_eq!(state.cancel_button_count(), 1, "every menu gets a generic Cancel button");
        state.clear_choices();
        assert_eq!(state.cancel_button_count(), 0);
    }

    #[test]
    fn a_close_button_removes_the_next_button_and_the_menu() {
        let mut state = state_with_menu(&["A", "B"]);
        state.add_text("Text.".to_owned());
        state.add_next_button();
        assert_eq!(state.next_button_count(), 1);

        state.add_close_button();

        assert_eq!(state.next_button_count(), 0);
        assert_eq!(state.choice_count(), 0);
        assert_eq!(state.close_button_count(), 1);
        assert_eq!(state.text_count(), 1);
    }

    #[test]
    fn the_next_button_clears_the_previous_page() {
        let mut state = DialogWindowState::default();
        state.add_text("First page.".to_owned());
        state.add_next_button();

        state.add_text("Second page.".to_owned());

        assert_eq!(state.text_count(), 1);
        assert_eq!(state.next_button_count(), 0);
    }

    #[test]
    fn dialog_text_keeps_the_server_text_and_color_markup() {
        // Korean and the KS X 1001 symbols of the official quest scripts are
        // covered by the fallback font, so the dialog keeps them verbatim.
        let mut state = DialogWindowState::default();
        state.add_text("^FF0000[알프]^000000 ⊂∑∀ ∠ ∫∝∝ ▲▼".to_owned());

        assert_eq!(state.text_count(), 1);
    }

    #[test]
    fn dialog_text_drops_control_characters_but_keeps_line_breaks() {
        let mut state = DialogWindowState::default();
        state.add_text("first\nsecond\u{0000}\u{0007}   ".to_owned());

        assert_eq!(state.text_count(), 1);
    }

    #[test]
    fn sanitized_dialog_text_keeps_markup_and_removes_controls() {
        assert_eq!(
            sanitize_dialog_text("^00FF00hello^000000\r\nworld\u{0000}  "),
            "^00FF00hello^000000\nworld"
        );
        assert_eq!(sanitize_dialog_text("⊂∑∀ ∠ ∫∝∝"), "⊂∑∀ ∠ ∫∝∝");
    }

    #[test]
    fn ending_the_dialog_removes_everything() {
        let mut state = state_with_menu(&["A"]);
        state.add_text("Text.".to_owned());
        state.add_next_button();

        state.end();

        assert_eq!(state.choice_count(), 0);
        assert_eq!(state.text_count(), 0);
        assert_eq!(state.next_button_count(), 0);
    }
}
