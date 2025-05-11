use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::utils::conversation::{ChatMessage, add_assistant_message, get_conversation_history};
use crate::utils::logger::log_openai_conversation;
use crate::utils::msg_context::MsgContextInfo;
use crate::utils::persistence::{BOT_STATE, save_state};
use crate::utils::statics::OPENAI_TOKEN;

// Model constants
const DEFAULT_TEMPERATURE: f32 = 0.7;

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

impl ChatCompletionRequest {
    async fn new(messages: Vec<ChatMessage>) -> Self {
        // Get model from persistent state
        let model = BOT_STATE.lock().await.current_model.clone();
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
pub async fn get_chatgpt_response(msg_ctx: &MsgContextInfo) -> eyre::Result<String> {
    // Get conversation history for this channel
    let history = get_conversation_history(msg_ctx.channel_id).await;

    // Create and send the request to OpenAI, measuring the time it takes
    let start_time = Instant::now();
    let response_content = send_chat_completion_request(history.clone()).await?;
    let duration = start_time.elapsed();

    // Log the conversation (request and response)
    if let Err(e) = log_openai_conversation(msg_ctx, &history, &response_content, duration).await {
        tracing::error!("Failed to log OpenAI conversation: {e}");
    }

    // Store the assistant's response in the conversation history
    add_assistant_message(msg_ctx.channel_id, response_content.clone()).await;

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
async fn process_openai_response(response: Response) -> eyre::Result<String> {
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(eyre::eyre!("OpenAI API error: {}", error_text));
    }

    let completion: ChatCompletionResponse = response.json().await?;
    tracing::debug!("OpenAI response: {:#?}", completion);

    completion
        .choices
        .first()
        .map(|choice| choice.message.content.clone())
        .ok_or_else(|| eyre::eyre!("No response from ChatGPT"))
}

/// Change the model used for ChatGPT requests
pub async fn change_model(model_name: &str) -> String {
    // Update model in persistent state
    let old_model;
    {
        let mut state = BOT_STATE.lock().await;
        old_model = state.current_model.clone();
        state.current_model = model_name.to_string();
    }

    // Save the state
    if let Err(e) = save_state().await {
        tracing::error!("Failed to save state after model change: {}", e);
    }

    format!("Model changed from {old_model} to {model_name}")
}

/// Get the current model name
pub async fn get_current_model() -> String {
    BOT_STATE.lock().await.current_model.clone()
}
