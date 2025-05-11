use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::utils::conversation::ChatMessage;
use crate::utils::logger::log_openai_conversation;
use crate::utils::msg_context::MsgContextInfo;
use crate::utils::persistence::get_current_model;
use crate::utils::persistence::{add_message, get_conversation_history};
use crate::utils::statics::OPENAI_TOKEN;

// Responses API Request and Response structures
#[derive(Debug, Serialize)]
struct ResponsesRequest {
    model: String,
    input: Vec<ChatMessage>,
}

impl ResponsesRequest {
    async fn new(messages: Vec<ChatMessage>) -> Self {
        // Get model from the global wrapper function
        let model = get_current_model().await;
        Self {
            model,
            input: messages,
        }
    }
}

// 출력 항목의 타입을 구분하는 Enum
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum OutputItem {
    #[serde(rename = "message")]
    Message(MessageOutput),
    #[serde(other)]
    Other,
}

// message 타입의 출력 항목을 위한 구조체
#[derive(Debug, Deserialize)]
struct MessageOutput {
    _id: String,
    status: String,
    _role: String, // always "assistant"
    content: Vec<ContentItem>,
}

// content 항목을 위한 구조체
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ContentItem {
    #[serde(rename = "output_text")]
    Text { text: String },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenAiResponse {
    id: String,
    output: Vec<OutputItem>,
    usage: ResponsesUsage,
}

#[derive(Debug, Deserialize)]
struct ResponsesUsage {
    input_tokens: u32,
    input_tokens_details: TokenDetails,
    output_tokens: u32,
    output_tokens_details: TokenDetails,
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct TokenDetails {
    cached_tokens: u32,
    reasoning_tokens: u32,
}

/// Get a response from OpenAI for the conversation in the specified channel
pub async fn get_openai_response(msg_ctx: &MsgContextInfo) -> eyre::Result<String> {
    // Get conversation history for this channel
    let history = get_conversation_history(msg_ctx.channel_id).await;

    // Create and send the request to OpenAI, measuring the time it takes
    let start_time = Instant::now();
    let response_content = send_responses_api_request(history.clone()).await?;
    let duration = start_time.elapsed();

    // Log the conversation (request and response)
    if let Err(e) = log_openai_conversation(msg_ctx, &history, &response_content, duration).await {
        tracing::error!("Failed to log OpenAI conversation: {e}");
    }

    // Store the assistant's response in the conversation history
    let message = ChatMessage::assistant(response_content.clone());
    add_message(msg_ctx.channel_id, message).await;

    Ok(response_content)
}

/// Send a request to the OpenAI Responses API
async fn send_responses_api_request(messages: Vec<ChatMessage>) -> eyre::Result<String> {
    let client = Client::new();
    let request = ResponsesRequest::new(messages).await;

    let response = client
        .post("https://api.openai.com/v1/responses")
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

    let response_data: OpenAiResponse = response.json().await?;
    tracing::debug!("OpenAI response: {:#?}", response_data);

    // Log token usage information
    tracing::info!(
        "Token usage - Input: {} ({} cached, {} reasoning), Output: {} ({} cached, {} reasoning), Total: {}",
        response_data.usage.input_tokens,
        response_data.usage.input_tokens_details.cached_tokens,
        response_data.usage.input_tokens_details.reasoning_tokens,
        response_data.usage.output_tokens,
        response_data.usage.output_tokens_details.cached_tokens,
        response_data.usage.output_tokens_details.reasoning_tokens,
        response_data.usage.total_tokens
    );

    // Find the first message output and extract its text content
    let content = response_data
        .output
        .iter()
        .find_map(|item| {
            if let OutputItem::Message(msg_output) = item {
                // Find the first text content in the message
                msg_output.content.iter().find_map(|content_item| {
                    if msg_output.status != "completed" {
                        tracing::warn!(
                            "Message output status is not completed: {}",
                            msg_output.status
                        );
                        return None;
                    }
                    if let ContentItem::Text { text, .. } = content_item {
                        Some(text.clone())
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        })
        .ok_or_else(|| eyre::eyre!("No valid text response from OpenAI"))?;

    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenvy::dotenv;
    use std::env;

    #[tokio::test]
    #[ignore = "This test calls the OpenAI API, which incurs a cost. It is ignored by default to avoid incurring a cost without intent."]
    async fn test_send_responses_api_request() {
        tracing_subscriber::fmt::init();

        // Initialize environment variables from .env file
        dotenv().ok();

        // Check if API key is available
        let token_var = env::var("MINTYBOT_OPENAI_TOKEN");
        if token_var.is_err() || token_var.as_ref().unwrap().is_empty() {
            panic!("MINTYBOT_OPENAI_TOKEN not set or empty.");
        }

        // Create a test conversation
        let messages = vec![
            // Create system message
            ChatMessage {
                role: "system".to_string(),
                content: "You are a helpful assistant.".to_string(),
                name: None,
            },
            // Add a user message
            ChatMessage::user(
                "Hello, what is the capital of South Korea?".to_string(),
                None,
            ),
        ];

        // Send the actual API request
        let result = send_responses_api_request(messages).await;

        // Verify the result
        assert!(result.is_ok(), "API request failed: {:?}", result.err());

        let response = result.unwrap();

        // Verify response is not empty
        assert!(!response.is_empty(), "Response from OpenAI was empty");

        // Check if response mentions 'Seoul' (capital of South Korea)
        assert!(
            response.to_lowercase().contains("seoul"),
            "Response doesn't contain the expected capital city: {response}"
        );
    }
}
