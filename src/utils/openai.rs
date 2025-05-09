use lazy_static::lazy_static;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serenity::model::id::ChannelId;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

use crate::utils::conversation::{ChatMessage, add_assistant_message, get_conversation_history};
use crate::utils::logger::log_openai_conversation;
use crate::utils::statics::OPENAI_TOKEN;

// Model constants
const DEFAULT_MODEL: &str = "gpt-4.1";
const DEFAULT_TEMPERATURE: f32 = 0.7;

// Global model name that can be changed
lazy_static! {
    static ref CURRENT_MODEL: Arc<Mutex<String>> = Arc::new(Mutex::new(DEFAULT_MODEL.to_string()));
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

impl ChatCompletionRequest {
    async fn new(messages: Vec<ChatMessage>) -> Self {
        let model = CURRENT_MODEL.lock().await.clone();
        Self {
            model,
            messages,
            temperature: DEFAULT_TEMPERATURE,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponseChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatCompletionResponseChoice>,
}

/// Get a response from ChatGPT for the conversation in the specified channel
pub async fn get_chatgpt_response(channel_id: ChannelId) -> eyre::Result<String> {
    // Get conversation history for this channel
    let history = get_conversation_history(channel_id).await;

    // Create and send the request to OpenAI, measuring the time it takes
    let start_time = Instant::now();
    let response_content = send_chat_completion_request(history.clone()).await?;
    let duration = start_time.elapsed();

    // Log the conversation (request and response)
    if let Err(e) = log_openai_conversation(channel_id, &history, &response_content, duration).await
    {
        eprintln!("Failed to log OpenAI conversation: {e}");
    }

    // Store the assistant's response in the conversation history
    add_assistant_message(channel_id, response_content.clone()).await;

    Ok(response_content)
}

/// Send a chat completion request to the OpenAI API
async fn send_chat_completion_request(messages: Vec<ChatMessage>) -> eyre::Result<String> {
    let client = Client::new();
    let request = ChatCompletionRequest::new(messages).await;

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", *OPENAI_TOKEN))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;

    process_openai_response(response).await
}

/// Process the response from OpenAI API
async fn process_openai_response(response: reqwest::Response) -> eyre::Result<String> {
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(eyre::eyre!("OpenAI API error: {}", error_text));
    }

    let completion: ChatCompletionResponse = response.json().await?;

    completion
        .choices
        .first()
        .map(|choice| choice.message.content.clone())
        .ok_or_else(|| eyre::eyre!("No response from ChatGPT"))
}

/// Change the model used for ChatGPT requests
pub async fn change_model(model_name: &str) -> String {
    let mut current_model = CURRENT_MODEL.lock().await;
    let old_model = current_model.clone();
    *current_model = model_name.to_string();

    format!("Model changed from {old_model} to {model_name}")
}

/// Get the current model name
#[expect(dead_code)]
pub async fn get_current_model() -> String {
    CURRENT_MODEL.lock().await.clone()
}
