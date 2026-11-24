//! Chat and notification push helpers (message tabs, localized templates,
//! interface sounds).
use crate::*;

impl Client {
    /// Pushes an informational message into the chat window.
    pub(crate) fn push_chat_information(&mut self, message: String) {
        self.client_state
            .follow_mut(client_state().chat_messages())
            .push(ChatMessage::new(message, MessageColor::Information));
    }

    /// Push a message into a specific chat tab.
    pub(crate) fn push_chat_message(&mut self, message: ChatMessage) {
        self.client_state.follow_mut(client_state().chat_messages()).push(message);
    }

    /// Show a transient notification in the top HUD overlay.
    ///
    /// The overlay is optional through the interface settings; when it is
    /// disabled the notification is dropped (the chat line / other effect of
    /// the same packet is unaffected).
    pub(crate) fn push_notification(&mut self, kind: NotificationKind, text: String, client_tick: ClientTick) {
        let enabled = *self
            .client_state
            .follow(client_state().interface_settings().show_notification_hud());

        if !enabled {
            return;
        }

        self.client_state
            .follow_mut(client_state().notifications())
            .push(kind, text, client_tick.0);
        self.open_notification_hud_window();
    }

    /// Plays the configured interface click sound.
    ///
    /// The path lives in `data\client\ui_audio.ron`; if the data file has no
    /// menu click sound the call is a no-op instead of loading an empty path.
    pub(crate) fn play_main_menu_click_sound(&self) {
        if let Some(sound_effect) = self.main_menu_click_sound_effect {
            self.audio_engine.play_sound_effect(sound_effect);
        }
    }

    /// Pushes a localized chat feedback message into the chat window.
    ///
    /// `template` is a localization path (for example
    /// `client_state().localization().chat_skill_passive()`) whose text uses
    /// the same `{name}` placeholders as the protocol result messages;
    /// rendering goes through `ro_next_messages::render_template`, so the
    /// client has one placeholder implementation and the result-code table
    /// stays the only protocol message table.
    pub(crate) fn push_localized_chat<P>(&mut self, template: P, args: &[(&str, &str)])
    where
        P: rust_state::Path<ClientState, String>,
    {
        let rendered = self.render_localized(template, args);

        self.push_chat_information(rendered);
    }

    /// Renders a localization template with named `{...}` placeholders.
    ///
    /// Used for the client feedback that goes to the error window; the chat
    /// feedback goes through [`Self::push_localized_chat`].
    pub(crate) fn render_localized<P>(&self, template: P, args: &[(&str, &str)]) -> String
    where
        P: rust_state::Path<ClientState, String>,
    {
        ro_next_messages::render_template(self.client_state.follow(template), args)
    }
}
