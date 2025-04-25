use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serenity::model::id::ChannelId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

// Maximum number of messages to keep in conversation history per channel
const MAX_HISTORY_LENGTH: usize = 100;
const SYSTEM_PROMPT: &str = "너는 MintyBot이라는 Discord 봇이야. 친구들의 질문에 재미있게 대답해 줘. 굳이 친절할 필요는 없고, 재미있는 친구가 되어 주면 충분해. 가끔은 조금 무례해도 괜찮아.";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: String,
    pub content: String,
}

impl ConversationMessage {
    pub fn user(content: String) -> Self {
        Self {
            role: "user".to_string(),
            content,
        }
    }

    pub fn assistant(content: String) -> Self {
        Self {
            role: "assistant".to_string(),
            content,
        }
    }

    pub fn system(content: String) -> Self {
        Self {
            role: "system".to_string(),
            content,
        }
    }
}

#[derive(Debug, Default)]
pub struct ConversationHistory {
    // Map of channel ID to conversation history
    conversations: HashMap<ChannelId, Vec<ConversationMessage>>,
}

impl ConversationHistory {
    pub fn new() -> Self {
        Self {
            conversations: HashMap::new(),
        }
    }

    // Add a message to the conversation history for a specific channel
    pub fn add_message(&mut self, channel_id: ChannelId, message: ConversationMessage) {
        let history = self.conversations.entry(channel_id).or_insert_with(|| {
            // Initialize with system message for new conversations
            vec![ConversationMessage::system(SYSTEM_PROMPT.to_string())]
        });

        // Add the new message
        history.push(message);

        // Trim history if it exceeds the maximum length
        if history.len() > MAX_HISTORY_LENGTH {
            // Keep the system message (at index 0) and the most recent messages
            *history = history
                .iter()
                .enumerate()
                .filter(|(i, _)| *i == 0 || *i > history.len() - MAX_HISTORY_LENGTH)
                .map(|(_, msg)| msg.clone())
                .collect();
        }
    }

    // Get the conversation history for a specific channel
    pub fn get_history(&self, channel_id: ChannelId) -> Vec<ConversationMessage> {
        self.conversations
            .get(&channel_id)
            .cloned()
            .unwrap_or_else(|| vec![ConversationMessage::system(SYSTEM_PROMPT.to_string())])
    }

    // Clear the conversation history for a specific channel
    #[allow(dead_code)]
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
pub async fn add_user_message(channel_id: ChannelId, content: String) {
    let mut manager = CONVERSATION_MANAGER.lock().await;
    manager.add_message(channel_id, ConversationMessage::user(content));
}

pub async fn add_assistant_message(channel_id: ChannelId, content: String) {
    let mut manager = CONVERSATION_MANAGER.lock().await;
    manager.add_message(channel_id, ConversationMessage::assistant(content));
}

pub async fn get_conversation_history(channel_id: ChannelId) -> Vec<ConversationMessage> {
    let manager = CONVERSATION_MANAGER.lock().await;
    manager.get_history(channel_id)
}

#[allow(dead_code)]
pub async fn clear_conversation_history(channel_id: ChannelId) {
    let mut manager = CONVERSATION_MANAGER.lock().await;
    manager.clear_history(channel_id);
}
