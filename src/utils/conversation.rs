use serde::{Deserialize, Serialize};
use serenity::model::id::ChannelId;
use std::fmt::Display;

use crate::utils::persistence::{BOT_STATE, save_state};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Display for ChatMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let role_str = format!("<{}>", self.role);
        let name_str = self
            .name
            .as_ref()
            .map(|name| format!("({name})"))
            .unwrap_or_default();
        let content_str = self.content.to_string();

        write!(
            f,
            "{}",
            itertools::join([role_str, name_str, content_str], " ")
        )
    }
}

impl ChatMessage {
    /// Create a new user message
    pub fn user(content: String, name: Option<String>) -> Self {
        Self {
            role: "user".to_string(),
            content,
            name,
        }
    }

    /// Create a new assistant message
    pub fn assistant(content: String) -> Self {
        Self {
            role: "assistant".to_string(),
            content,
            name: Some("MintyBot".to_string()),
        }
    }

    /// Create a new developer message
    pub fn developer(content: String) -> Self {
        Self {
            role: "developer".to_string(),
            content,
            name: None,
        }
    }
}

// Helper functions to interact with the global conversation manager

/// Add a message to the persistent state
async fn add_message_to_state(channel_id: ChannelId, message: ChatMessage) {
    // Add to the persistent state
    {
        let mut state = BOT_STATE.lock().await;
        state.add_message(channel_id, message);
    }

    // Save state
    let _ = save_state().await;
}

/// Add a user message to the conversation history
pub async fn add_user_message(channel_id: ChannelId, content: String, username: Option<String>) {
    let message = ChatMessage::user(content, username);
    add_message_to_state(channel_id, message).await;
}

/// Add an assistant message to the conversation history
pub async fn add_assistant_message(channel_id: ChannelId, content: String) {
    let message = ChatMessage::assistant(content);
    add_message_to_state(channel_id, message).await;
}

/// Get the conversation history for a specific channel, including the system prompt
pub async fn get_conversation_history(channel_id: ChannelId) -> Vec<ChatMessage> {
    let state = BOT_STATE.lock().await;
    state.get_conversation(channel_id)
}

/// Clear the conversation history for a specific channel
pub async fn clear_conversation_history(channel_id: ChannelId) {
    // Clear from persistent state
    {
        let mut state = BOT_STATE.lock().await;
        state.remove_conversation(channel_id);
    }

    // Save state
    let _ = save_state().await;
}
