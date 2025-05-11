use crate::utils::conversation::ChatMessage;
use crate::utils::persistence::get_current_model;
use serde::{Deserialize, Serialize};

/// Request structure for OpenAI Responses API
#[derive(Debug, Serialize)]
pub struct ResponsesRequest {
    model: String,
    input: Vec<ChatMessage>,
}

impl ResponsesRequest {
    pub async fn new(messages: Vec<ChatMessage>) -> Self {
        let model = get_current_model().await;
        Self {
            model,
            input: messages,
        }
    }
}

/// Response structure from OpenAI API
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OpenAiResponse {
    pub id: String,
    pub output: Vec<OutputItem>,
    pub usage: ResponsesUsage,
}

/// Output item in the OpenAI response
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum OutputItem {
    #[serde(rename = "message")]
    Message(MessageOutput),
    #[serde(other)]
    Other,
}

/// Message output structure
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct MessageOutput {
    pub id: String,
    pub status: String,
    pub role: String, // always "assistant"
    pub content: Vec<ContentItem>,
}

/// Content item in a message
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ContentItem {
    #[serde(rename = "output_text")]
    Text { text: String },
    #[serde(other)]
    Other,
}

/// Token usage information
#[derive(Copy, Clone, Debug, Deserialize)]
pub struct ResponsesUsage {
    pub input_tokens: u32,
    pub input_tokens_details: InputTokensDetails,
    pub output_tokens: u32,
    pub output_tokens_details: OutputTokensDetails,
    pub total_tokens: u32,
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct InputTokensDetails {
    pub cached_tokens: u32,
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct OutputTokensDetails {
    pub reasoning_tokens: u32,
}
