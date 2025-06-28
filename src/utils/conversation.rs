use crate::utils::openai_schema::ContentItem;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Vec<ContentItem>,
}

impl Display for ChatMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let role_str = format!("<{}>", self.role);
        let content_str = self
            .content
            .iter()
            .map(|item| match item {
                ContentItem::InputText { text } => text.clone(),
                ContentItem::InputImage { image_url } => format!("[Image: {image_url}]"),
                ContentItem::OutputText { text } => text.clone(),
                ContentItem::Other => "[Unknown content]".to_string(),
            })
            .collect::<Vec<_>>()
            .join(" ");

        write!(f, "{role_str} {content_str}")
    }
}

impl ChatMessage {
    /// Create a new user message with text content
    /// If name is provided, it will be prepended to the content in the format "(name) content"
    pub fn user(content: String, name: String) -> Self {
        let formatted_content = format!("({name}) {content}");

        Self {
            role: "user".to_string(),
            content: vec![ContentItem::InputText {
                text: formatted_content,
            }],
        }
    }

    /// Create a new user message with both text and image content
    pub fn user_with_image(text_content: String, name: String, image_url: String) -> Self {
        let formatted_content = format!("({name}) {text_content}");

        Self {
            role: "user".to_string(),
            content: vec![
                ContentItem::InputText {
                    text: formatted_content,
                },
                ContentItem::InputImage { image_url },
            ],
        }
    }

    /// Create a new assistant message
    pub fn assistant(content: String) -> Self {
        Self {
            role: "assistant".to_string(),
            content: vec![ContentItem::OutputText { text: content }],
        }
    }

    /// Create a new developer message
    pub fn developer(content: String) -> Self {
        Self {
            role: "developer".to_string(),
            content: vec![ContentItem::InputText { text: content }],
        }
    }
}
