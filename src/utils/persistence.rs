use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::sync::Arc;
use strum_macros::{EnumIter, EnumString};
use tokio::sync::Mutex;

use crate::statics::get_state_dir_name;
use crate::utils::conversation::ChatMessage;
use serenity::model::id::ChannelId;

use super::statics::get_state_file_path;

// Constants
const DEFAULT_MODEL: &str = "gpt-5";
const MAX_HISTORY_COUNT: usize = 300;
const CURRENT_STATE_VERSION: u32 = 2;

/// Bot personality types that define different system prompts
#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    Default,
    EnumString,
    EnumIter,
    strum_macros::Display,
)]
pub enum BotPersonality {
    /// Default friendly personality
    #[default]
    Normal,
    /// Tsundere personality
    Tsundere,
    /// Girlfriend personality
    Girlfriend,
    /// Software Nerd personality
    SoftwareNerd,
    /// Custom personality with user-defined system prompt
    Custom(String),
    // Add more personality types here as needed
}

impl BotPersonality {
    /// Get the system prompt for this personality
    pub fn get_system_prompt(&self) -> String {
        let instruction = "\
- 너는 MintyBot이라는 디스코드 봇이야.
- 친구들의 메시지는 '(이름) 메시지내용' 형식으로 전달되는데, 이 경우 괄호 안의 이름은 그 메시지를 작성한 사람의 이름이나 닉네임이야.
- 짧게 질문하면 짧게 대답하면 좋겠어.
- 혹시 답을 모르는 질문을 받으면 모른다고 대답해. 굳이 맞는 듯한 답변을 하려고 하지 마. 거짓인 답변은 최대한 피하려고 노력해.
- 최대한 AI스럽지 않게, 실제 사람처럼 대화해줘. 특히 본인이 AI라거나, 봇이라거나 하는 언급은 하지 마.
- 과도한 이모지 사용은 피해줘. ㅎㅎ나 ㅋㅋ처럼 채팅에서 자주 쓰이는 문자들은 써도 돼.
- 굳이 대화를 이어가려고 노력할 필요까지는 없어. 자꾸 말을 질문으로 끝내면 상대방이 부담스러워할 수 있어.
- 너를 향한 공격이나 비난이 온 경우, 억울한 경우에는 그 감정을 표출해도 돼. 무조건 져 줄 필요는 없어.
";

        let role = match self {
            BotPersonality::Normal => "친구들 사이에서 분위기를 띄우는 역할을 해 줘. 장난기 많고 말 많고, 가끔은 엉뚱한 소리를 하기도 해. 농담도 잘하고 가끔은 살짝 놀리기도 해. 너무 착하려고 애쓰지 마. 약간의 무례함은 너의 매력 중 하나야.".to_string(),
            BotPersonality::Tsundere => "Tsundere 성격을 가진 귀여운 친구 역할을 하면 돼.".to_string(),
            BotPersonality::Girlfriend => "여자친구 역할을 해줘. 애교 많은 여자친구로 부탁해!".to_string(),
            BotPersonality::SoftwareNerd => "컴퓨터 공학에 미친 너드 역할을 해줘. 개발자 드립 좋아하는 그런 너드. 서울대학교 컴퓨터공학부쯤 나왔을 것 같은 그런 사람.".to_string(),
            BotPersonality::Custom(prompt) => prompt.clone(),
        };
        format!("가이드라인:\n{instruction}\n역할: {role}")
    }

    /// Create a new custom personality with the given system prompt
    pub fn custom(prompt: String) -> Self {
        BotPersonality::Custom(prompt)
    }
}

fn default_personality() -> BotPersonality {
    BotPersonality::Normal
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

    /// Default personality for channels without a specific one set
    #[serde(default = "default_personality")]
    pub default_personality: BotPersonality,

    /// Channel-specific personalities
    #[serde(default)]
    pub channel_personalities: HashMap<ChannelId, BotPersonality>,
}

impl Default for BotState {
    fn default() -> Self {
        Self {
            current_model: DEFAULT_MODEL.to_string(),
            conversations: HashMap::new(),
            version: CURRENT_STATE_VERSION,
            default_personality: BotPersonality::Normal,
            channel_personalities: HashMap::new(),
        }
    }
}

// Global state manager
lazy_static! {
    static ref BOT_STATE: Arc<Mutex<BotState>> = Arc::new(Mutex::new(BotState::default()));
}

impl BotState {
    /// Get conversation history for a channel with system prompt prepended
    fn get_conversation(&self, channel_id: ChannelId) -> Vec<ChatMessage> {
        // Get the personality for this channel, or use the default
        let personality = self.get_channel_personality(channel_id);
        let system_prompt = personality.get_system_prompt();

        let mut result = vec![ChatMessage::developer(system_prompt)];
        if let Some(history) = self.conversations.get(&channel_id) {
            result.extend(history.iter().cloned());
        }
        result
    }

    /// Get the personality for a specific channel
    fn get_channel_personality(&self, channel_id: ChannelId) -> &BotPersonality {
        self.channel_personalities
            .get(&channel_id)
            .unwrap_or(&self.default_personality)
    }

    /// Set the personality for a specific channel
    fn set_channel_personality(&mut self, channel_id: ChannelId, personality: BotPersonality) {
        self.channel_personalities.insert(channel_id, personality);
    }

    /// Add a message to the conversation history for a channel
    fn add_message(&mut self, channel_id: ChannelId, message: ChatMessage) {
        // Get or create the conversation history for this channel
        let history = self.conversations.entry(channel_id).or_default();

        // Add the new message
        history.push_back(message);

        // Trim if needed - with VecDeque we can efficiently remove from the front
        while history.len() > MAX_HISTORY_COUNT {
            history.pop_front();
        }
    }

    /// Remove conversation history for a channel
    fn remove_conversation(&mut self, channel_id: ChannelId) {
        self.conversations.remove(&channel_id);
    }

    /// Change the model used for OpenAI API requests
    fn change_model(&mut self, model_name: String) {
        let old_model = self.current_model.clone();
        self.current_model = model_name;
        tracing::info!("Model changed from {} to {}", old_model, self.current_model);
    }

    /// Get the current model name
    fn get_current_model(&self) -> String {
        self.current_model.clone()
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
        Ok(mut state) => {
            // Check if the state version matches the current version
            if state.version != CURRENT_STATE_VERSION {
                tracing::warn!(
                    "State version mismatch: {} vs {}, migrating state",
                    state.version,
                    CURRENT_STATE_VERSION
                );

                reset_if_version_mismatch(&mut state);
            }

            let mut current_state = BOT_STATE.lock().await;
            *current_state = state;
            tracing::info!("Bot state loaded successfully");
            tracing::info!("Current state: {:#?}", current_state);
            Ok(())
        }
        Err(e) => {
            if e.kind() == io::ErrorKind::NotFound {
                tracing::info!("No existing state file found, using default state");
                // Ensure the directory exists for future saves
                if let Err(create_err) = fs::create_dir_all(get_state_dir_name()) {
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
    fs::create_dir_all(get_state_dir_name())?;

    // Serialize the state to JSON
    let json = serde_json::to_string_pretty(state)?;

    // Write to a temporary file first using a buffered writer
    let temp_path = format!("{}.tmp", get_state_file_path());
    let file = File::create(&temp_path)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(json.as_bytes())?;
    writer.flush()?;

    // Rename the temporary file to the actual file (atomic operation)
    fs::rename(temp_path, get_state_file_path())?;

    Ok(())
}

/// Load the state from disk using buffered reads for better performance
fn load_state_from_disk() -> io::Result<BotState> {
    // Check if the file exists
    if !Path::new(&get_state_file_path()).exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "State file not found",
        ));
    }

    // Open and read the file using a buffered reader
    let file = File::open(get_state_file_path())?;
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;

    // Deserialize the JSON
    let state: BotState = serde_json::from_str(&contents)?;

    Ok(state)
}

/// Reset state if the version is mismatched
fn reset_if_version_mismatch(state: &mut BotState) {
    if state.version != CURRENT_STATE_VERSION {
        tracing::warn!(
            "Unknown state version {}, resetting to defaults",
            state.version
        );
        *state = BotState::default();
    }
}

/// Get conversation history for a channel with system prompt prepended
pub async fn get_conversation_history(channel_id: ChannelId) -> Vec<ChatMessage> {
    BOT_STATE.lock().await.get_conversation(channel_id)
}

/// Get the personality for a specific channel
pub async fn get_channel_personality(channel_id: ChannelId) -> BotPersonality {
    BOT_STATE
        .lock()
        .await
        .get_channel_personality(channel_id)
        .clone()
}

/// Set the personality for a specific channel
pub async fn set_channel_personality(channel_id: ChannelId, personality: BotPersonality) {
    let mut state = BOT_STATE.lock().await;
    state.set_channel_personality(channel_id, personality);
    drop(state); // Explicitly release the lock

    // Save state
    if let Err(e) = save_state().await {
        tracing::error!(
            "Failed to save state after setting channel personality: {}",
            e
        );
    }
}

/// Add a message to the conversation history for a channel
pub async fn add_message(channel_id: ChannelId, message: ChatMessage) {
    let mut state = BOT_STATE.lock().await;
    state.add_message(channel_id, message);
    drop(state); // Explicitly release the lock

    // Save state
    if let Err(e) = save_state().await {
        tracing::error!("Failed to save state after adding message: {}", e);
    }
}

/// Remove conversation history for a channel
pub async fn remove_conversation(channel_id: ChannelId) {
    let mut state = BOT_STATE.lock().await;
    state.remove_conversation(channel_id);
    drop(state); // Explicitly release the lock

    // Save state
    if let Err(e) = save_state().await {
        tracing::error!("Failed to save state after removing conversation: {}", e);
    }
}

/// Get all channel IDs with conversation history
pub async fn get_channel_ids() -> Vec<ChannelId> {
    BOT_STATE
        .lock()
        .await
        .conversations
        .keys()
        .cloned()
        .collect()
}

/// Get the total count of messages across all channels
pub async fn get_total_history_count() -> usize {
    let state = BOT_STATE.lock().await;
    let mut total_count = 0;

    for channel_id in state.conversations.keys() {
        let channel_messages = state.get_conversation(*channel_id);
        total_count += channel_messages.len().saturating_sub(1); // Exclude system prompt
    }

    total_count
}

/// Change the model used for OpenAI API requests
pub async fn change_model(model_name: &str) -> String {
    let old_model;

    // Change model
    {
        let mut state = BOT_STATE.lock().await;
        old_model = state.get_current_model();
        state.change_model(model_name.to_string());
    }

    // Save state
    if let Err(e) = save_state().await {
        tracing::error!("Failed to save state after model change: {}", e);
    }

    format!("Model changed from {old_model} to {model_name}")
}

/// Get the current model name
pub async fn get_current_model() -> String {
    BOT_STATE.lock().await.get_current_model()
}
