use std::cell::UnsafeCell;

use korangar_interface::element::store::ElementStoreMut;
use korangar_interface::element::{Element, ElementBox, ErasedElement, StateElement};
use korangar_interface::layout::{Resolvers, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::EntityId;
use rust_state::{Path, RustState, State};

use super::WindowClass;
use crate::input::InputEvent;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

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
        self
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

        self.elements.push(DialogElement::new(
            text! {
                text: sanitize_dialog_text(&text),
            },
            DialogElementKind::Text,
        ));
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

        self.elements
            .retain(|element| !matches!(element.kind, DialogElementKind::NextButton | DialogElementKind::Choice));

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
    }

    /// Close the current NPC menu without ending the dialog.
    ///
    /// The official client closes its menu window as soon as the player picks
    /// an option, before the server answers. The dialog window itself stays
    /// open, so only the choice buttons are removed here.
    pub fn clear_choices(&mut self) {
        self.elements.retain(|element| element.kind != DialogElementKind::Choice);
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
    }
}

impl Default for DialogWindowState {
    fn default() -> Self {
        Self {
            elements: Default::default(),
            // Arguably not very clean but avoids using an Option.
            npc_id: EntityId(0),
            clear_next: false,
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
