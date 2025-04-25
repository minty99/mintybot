use eyre::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serenity::model::id::ChannelId;

use crate::utils::conversation::{
    add_assistant_message, get_conversation_history, ConversationMessage,
};
use crate::utils::statics::OPENAI_TOKEN;

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

#[derive(Debug, Deserialize)]
struct ChatCompletionResponseChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatCompletionResponseChoice>,
}

pub async fn get_chatgpt_response(channel_id: ChannelId) -> Result<String> {
    let client = Client::new();

    // Get conversation history for this channel
    let history = get_conversation_history(channel_id).await;

    // Convert ConversationMessage to ChatMessage
    let messages: Vec<ChatMessage> = history.into_iter().map(ChatMessage::from).collect();

    let request = ChatCompletionRequest {
        model: "gpt-4.1-mini".to_string(),
        messages,
        temperature: 0.7,
    };

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", *OPENAI_TOKEN))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(eyre::eyre!("OpenAI API error: {}", error_text));
    }

    let completion: ChatCompletionResponse = response.json().await?;

    if let Some(choice) = completion.choices.first() {
        let response_content = choice.message.content.clone();

        // Store the assistant's response in the conversation history
        add_assistant_message(channel_id, response_content.clone()).await;

        Ok(response_content)
    } else {
        Err(eyre::eyre!("No response from ChatGPT"))
    }
}
