use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::utils::conversation::ChatMessage;
use serenity::model::id::ChannelId;

// Constants
const DEFAULT_MODEL: &str = "gpt-4.1";
const DEFAULT_MAX_HISTORY: usize = 100;
const STATE_FILE_PATH: &str = "data/bot_state.json";
const STATE_DIR_PATH: &str = "data";
const DEFAULT_SYSTEM_PROMPT: &str = "너는 MintyBot이라는 Discord 봇이야. 친구들의 질문에 재미있게 대답해 줘. 굳이 친절할 필요는 없고, 재미있는 친구가 되어 주면 충분해. 가끔은 조금 무례해도 괜찮아.";

// Helper functions for serde defaults
fn default_max_history() -> usize {
    DEFAULT_MAX_HISTORY
}

fn default_system_prompt() -> String {
    DEFAULT_SYSTEM_PROMPT.to_string()
}

/// Structure to hold all persistent bot state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotState {
    /// The current model being used
    pub current_model: String,

    /// Conversation history for each channel
    pub conversations: HashMap<ChannelId, VecDeque<ChatMessage>>,

    /// Version of the state format for future compatibility
    pub version: u32,

    /// Maximum number of messages to keep per channel
    #[serde(default = "default_max_history")]
    pub max_history_length: usize,

    /// System prompt that will be prepended to conversations
    #[serde(default = "default_system_prompt")]
    pub system_prompt: String,
}

impl Default for BotState {
    fn default() -> Self {
        Self {
            current_model: DEFAULT_MODEL.to_string(),
            conversations: HashMap::new(),
            version: 1,
            max_history_length: DEFAULT_MAX_HISTORY,
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
        }
    }
}

// Global state manager
lazy_static! {
    pub static ref BOT_STATE: Arc<Mutex<BotState>> = Arc::new(Mutex::new(BotState::default()));
}

impl BotState {
    /// Create a new bot state
    pub fn new() -> Self {
        Self::default()
    }

    /// Get conversation history for a channel with system prompt prepended
    pub fn get_conversation(&self, channel_id: ChannelId) -> Vec<ChatMessage> {
        let mut result = vec![ChatMessage::developer(self.system_prompt.clone())];
        if let Some(history) = self.conversations.get(&channel_id) {
            result.extend(history.iter().cloned());
        }
        result
    }

    /// Add a message to the conversation history for a channel
    pub fn add_message(&mut self, channel_id: ChannelId, message: ChatMessage) {
        // Get or create the conversation history for this channel
        let history = self.conversations.entry(channel_id).or_default();

        // Add the new message
        history.push_back(message);

        // Trim if needed - with VecDeque we can efficiently remove from the front
        while history.len() > self.max_history_length {
            history.pop_front();
        }
    }

    /// Remove conversation history for a channel
    pub fn remove_conversation(&mut self, channel_id: ChannelId) {
        self.conversations.remove(&channel_id);
    }

    /// Get all channel IDs with conversation history
    pub fn get_channel_ids(&self) -> Vec<ChannelId> {
        self.conversations.keys().cloned().collect()
    }
}

/// Save the current bot state to disk
/// This is called whenever the state changes to ensure no data is lost
pub async fn save_state() -> io::Result<()> {
    let state = BOT_STATE.lock().await.clone();
    save_state_to_disk(&state)
}

/// Load the bot state from disk
pub async fn load_state() -> io::Result<()> {
    match load_state_from_disk() {
        Ok(state) => {
            let mut current_state = BOT_STATE.lock().await;
            *current_state = state;
            tracing::info!("Bot state loaded successfully");
            Ok(())
        }
        Err(e) => {
            if e.kind() == io::ErrorKind::NotFound {
                tracing::info!("No existing state file found, using default state");
                // Ensure the directory exists for future saves
                if let Err(create_err) = fs::create_dir_all(STATE_DIR_PATH) {
                    tracing::error!("Failed to create state directory: {}", create_err);
                    return Err(create_err);
                }
                Ok(())
            } else {
                tracing::error!("Failed to load bot state: {}", e);
                Err(e)
            }
        }
    }
}

/// Save the state to disk using buffered writes for better performance
fn save_state_to_disk(state: &BotState) -> io::Result<()> {
    // Ensure the directory exists
    fs::create_dir_all(STATE_DIR_PATH)?;

    // Serialize the state to JSON
    let json = serde_json::to_string_pretty(state)?;

    // Write to a temporary file first using a buffered writer
    let temp_path = format!("{STATE_FILE_PATH}.tmp");
    let file = File::create(&temp_path)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(json.as_bytes())?;
    writer.flush()?;

    // Rename the temporary file to the actual file (atomic operation)
    fs::rename(temp_path, STATE_FILE_PATH)?;

    Ok(())
}

/// Load the state from disk using buffered reads for better performance
fn load_state_from_disk() -> io::Result<BotState> {
    // Check if the file exists
    if !Path::new(STATE_FILE_PATH).exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "State file not found",
        ));
    }

    // Open and read the file using a buffered reader
    let file = File::open(STATE_FILE_PATH)?;
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;

    // Deserialize the JSON
    let state: BotState = serde_json::from_str(&contents)?;

    Ok(state)
}
