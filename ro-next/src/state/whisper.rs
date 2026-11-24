//! Whisper conversations.
//!
//! The chat window has a separate whisper tab, but a whisper is a
//! conversation: the classic client opens a small window per partner with the
//! recent history and a reply box. This state keeps one bounded history per
//! partner and the partner that is currently shown in the whisper window.
//!
//! The server does not echo a whisper back to the sender, so outgoing messages
//! are recorded locally when the whisper packet is sent. Incoming whispers are
//! recorded from `ZC_WHISPER` (0x09DE).

use ro_next_interface::element::StateElement;
use rust_state::RustState;

/// Maximum number of messages kept per conversation.
///
/// Older messages are dropped first; the official client also keeps a bounded
/// history per whisper window.
pub const MAX_WHISPER_MESSAGES: usize = 80;

/// One line of a whisper conversation.
#[derive(Clone, Debug, PartialEq, Eq, RustState, StateElement)]
pub struct WhisperMessage {
    pub text: String,
    /// `true` for a message the local player sent.
    pub outgoing: bool,
}

/// The history of one whisper partner.
#[derive(Clone, Debug, Default)]
pub struct WhisperConversation {
    partner: String,
    messages: Vec<WhisperMessage>,
}

impl WhisperConversation {
    pub fn partner(&self) -> &str {
        &self.partner
    }

    pub fn messages(&self) -> &[WhisperMessage] {
        &self.messages
    }
}

/// All whisper conversations of the session.
#[derive(Default, RustState, StateElement)]
pub struct WhisperConversations {
    /// Partner currently shown in the whisper window. Empty while no
    /// conversation was opened yet (the window is only opened with a partner).
    pub active_partner: String,
    /// Text of the reply box.
    pub input_text: String,
    /// Bounded histories, one per partner.
    #[hidden_element]
    conversations: Vec<WhisperConversation>,
}

impl WhisperConversations {
    /// Open (or create) the conversation with `partner` and make it the active
    /// one. The reply box is cleared when the partner changes.
    pub fn open(&mut self, partner: &str) {
        if self.active_partner == partner {
            return;
        }

        self.active_partner = partner.to_owned();
        self.input_text.clear();

        if !self.conversations.iter().any(|conversation| conversation.partner == partner) {
            self.conversations.push(WhisperConversation {
                partner: partner.to_owned(),
                messages: Vec::new(),
            });
        }
    }

    fn conversation_mut(&mut self, partner: &str) -> &mut WhisperConversation {
        let index = match self.conversations.iter().position(|conversation| conversation.partner == partner) {
            Some(index) => index,
            None => {
                self.conversations.push(WhisperConversation {
                    partner: partner.to_owned(),
                    messages: Vec::new(),
                });
                self.conversations.len() - 1
            }
        };

        &mut self.conversations[index]
    }

    /// Record a message the local player sent.
    pub fn push_outgoing(&mut self, partner: &str, text: impl Into<String>) {
        self.open(partner);
        self.push(partner, text, true);
    }

    /// Record a message another player sent.
    pub fn push_incoming(&mut self, partner: &str, text: impl Into<String>) {
        self.push(partner, text, false);
    }

    fn push(&mut self, partner: &str, text: impl Into<String>, outgoing: bool) {
        let text = text.into();

        if text.is_empty() {
            return;
        }

        let conversation = self.conversation_mut(partner);

        if conversation.messages.len() >= MAX_WHISPER_MESSAGES {
            conversation.messages.remove(0);
        }

        conversation.messages.push(WhisperMessage { text, outgoing });
    }

    /// Messages of the currently active conversation.
    pub fn active_messages(&self) -> &[WhisperMessage] {
        self.conversations
            .iter()
            .find(|conversation| conversation.partner == self.active_partner)
            .map(WhisperConversation::messages)
            .unwrap_or(&[])
    }

    /// History of one partner, for tests and diagnostics.
    pub fn messages(&self, partner: &str) -> &[WhisperMessage] {
        self.conversations
            .iter()
            .find(|conversation| conversation.partner == partner)
            .map(WhisperConversation::messages)
            .unwrap_or(&[])
    }

    /// Forget every conversation (e.g. on logout).
    pub fn clear(&mut self) {
        self.active_partner.clear();
        self.input_text.clear();
        self.conversations.clear();
    }
}

impl WhisperMessage {
    /// Full line as shown in the whisper window.
    pub fn display(&self, partner: &str) -> String {
        match self.outgoing {
            true => format!("You: {}", self.text),
            false => format!("{partner}: {}", self.text),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_WHISPER_MESSAGES, WhisperConversations};

    #[test]
    fn opening_a_conversation_sets_the_partner_and_clears_the_reply_box() {
        let mut whispers = WhisperConversations::default();
        whispers.active_partner = "old".to_owned();
        whispers.input_text = "draft".to_owned();

        whispers.open("Alice");

        assert_eq!(whispers.active_partner, "Alice");
        assert!(whispers.input_text.is_empty());
        assert!(whispers.active_messages().is_empty());
    }

    #[test]
    fn incoming_and_outgoing_messages_are_kept_per_partner() {
        let mut whispers = WhisperConversations::default();

        whispers.push_incoming("Alice", "hi");
        whispers.push_outgoing("Bob", "hello Bob");
        whispers.push_outgoing("Alice", "hello Alice");

        assert_eq!(whispers.messages("Alice").len(), 2);
        assert_eq!(whispers.messages("Bob").len(), 1);
        assert_eq!(whispers.messages("Alice")[0].display("Alice"), "Alice: hi");
        assert_eq!(whispers.messages("Alice")[1].display("Alice"), "You: hello Alice");
        assert_eq!(
            whispers.active_partner, "Alice",
            "sending to a partner makes that conversation active"
        );
    }

    #[test]
    fn history_is_bounded() {
        let mut whispers = WhisperConversations::default();

        for index in 0..MAX_WHISPER_MESSAGES + 20 {
            whispers.push_incoming("Alice", format!("message {index}"));
        }

        assert_eq!(whispers.messages("Alice").len(), MAX_WHISPER_MESSAGES);
        assert_eq!(
            whispers.messages("Alice").last().unwrap().text,
            format!("message {}", MAX_WHISPER_MESSAGES + 19)
        );
    }

    #[test]
    fn empty_messages_are_ignored() {
        let mut whispers = WhisperConversations::default();

        whispers.push_incoming("Alice", "");
        whispers.push_outgoing("Alice", "");

        assert!(whispers.messages("Alice").is_empty());
    }
}
