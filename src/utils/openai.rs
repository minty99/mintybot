use reqwest::Client;
use serde::{Deserialize, Serialize};
use serenity::model::id::ChannelId;

use crate::utils::conversation::{
    add_assistant_message, get_conversation_history, ConversationMessage,
};
use crate::utils::statics::OPENAI_TOKEN;

// Model constants
const DEFAULT_MODEL: &str = "gpt-4.1-mini";
const DEFAULT_TEMPERATURE: f32 = 0.7;

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

impl From<ConversationMessage> for ChatMessage {
    fn from(msg: ConversationMessage) -> Self {
        Self {
            role: msg.role,
            content: msg.content,
        }
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

impl ChatCompletionRequest {
    fn new(messages: Vec<ChatMessage>) -> Self {
        Self {
            model: DEFAULT_MODEL.to_string(),
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

    // Convert ConversationMessage to ChatMessage
    let messages: Vec<ChatMessage> = history.into_iter().map(ChatMessage::from).collect();

    // Create and send the request to OpenAI
    let response_content = send_chat_completion_request(messages).await?;

    // Store the assistant's response in the conversation history
    add_assistant_message(channel_id, response_content.clone()).await;

    Ok(response_content)
}

/// Send a chat completion request to the OpenAI API
async fn send_chat_completion_request(messages: Vec<ChatMessage>) -> eyre::Result<String> {
    let client = Client::new();
    let request = ChatCompletionRequest::new(messages);

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
