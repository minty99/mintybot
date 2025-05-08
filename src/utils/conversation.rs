use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serenity::model::id::ChannelId;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;
use tokio::sync::Mutex;

// Configuration constants
const MAX_HISTORY_LENGTH: usize = 100;
const SYSTEM_PROMPT: &str = "너는 MintyBot이라는 Discord 봇이야. 친구들의 질문에 재미있게 대답해 줘. 굳이 친절할 필요는 없고, 재미있는 친구가 되어 주면 충분해. 가끔은 조금 무례해도 괜찮아.";

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

#[derive(Debug, Default)]
pub struct ConversationHistory {
    // Map of channel ID to conversation history
    conversations: HashMap<ChannelId, Vec<ChatMessage>>,
}

impl ConversationHistory {
    /// Create a new conversation history manager
    pub fn new() -> Self {
        Self {
            conversations: HashMap::new(),
        }
    }

    /// Add a message to the conversation history for a specific channel
    pub fn add_message(&mut self, channel_id: ChannelId, message: ChatMessage) {
        let history = self.get_or_create_history(channel_id);

        // Add the new message
        history.push(message);

        // Trim history if it exceeds the maximum length
        self.trim_history_if_needed(channel_id);
    }

    /// Get or create a conversation history for a channel
    fn get_or_create_history(&mut self, channel_id: ChannelId) -> &mut Vec<ChatMessage> {
        self.conversations.entry(channel_id).or_insert_with(|| {
            // Initialize with system message for new conversations
            vec![ChatMessage::developer(SYSTEM_PROMPT.to_string())]
        })
    }

    /// Trim the history if it exceeds the maximum length
    fn trim_history_if_needed(&mut self, channel_id: ChannelId) {
        let Some(history) = self.conversations.get_mut(&channel_id) else {
            return;
        };

        if history.len() > MAX_HISTORY_LENGTH {
            // Keep the system message (at index 0) and the most recent messages
            let system_message = history[0].clone();
            let recent_messages: Vec<ChatMessage> = history
                .iter()
                .skip(history.len() - MAX_HISTORY_LENGTH + 1) // +1 to account for system message
                .cloned()
                .collect();

            // Rebuild history with system message and recent messages
            *history = vec![system_message];
            history.extend(recent_messages);
        }
    }

    /// Get the conversation history for a specific channel
    pub fn get_history(&self, channel_id: ChannelId) -> Vec<ChatMessage> {
        self.conversations
            .get(&channel_id)
            .cloned()
            .unwrap_or_else(|| vec![ChatMessage::developer(SYSTEM_PROMPT.to_string())])
    }

    /// Clear the conversation history for a specific channel
    pub fn clear_history(&mut self, channel_id: ChannelId) {
        self.conversations.remove(&channel_id);
    }
}

// Global conversation history manager
lazy_static! {
    pub static ref CONVERSATION_MANAGER: Arc<Mutex<ConversationHistory>> =
        Arc::new(Mutex::new(ConversationHistory::new()));
}

// Helper functions to interact with the global conversation manager

/// Add a user message to the conversation history
pub async fn add_user_message(channel_id: ChannelId, content: String, username: Option<String>) {
    let mut manager = CONVERSATION_MANAGER.lock().await;
    manager.add_message(channel_id, ChatMessage::user(content, username));
}

/// Add an assistant message to the conversation history
pub async fn add_assistant_message(channel_id: ChannelId, content: String) {
    let mut manager = CONVERSATION_MANAGER.lock().await;
    manager.add_message(channel_id, ChatMessage::assistant(content));
}

/// Get the conversation history for a specific channel
pub async fn get_conversation_history(channel_id: ChannelId) -> Vec<ChatMessage> {
    let manager = CONVERSATION_MANAGER.lock().await;
    manager.get_history(channel_id)
}

/// Clear the conversation history for a specific channel
pub async fn clear_conversation_history(channel_id: ChannelId) {
    let mut manager = CONVERSATION_MANAGER.lock().await;
    manager.clear_history(channel_id);
}
