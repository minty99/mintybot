use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl Display for ChatMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let role_str = format!("<{}>", self.role);
        let content_str = self.content.to_string();

        write!(f, "{role_str} {content_str}")
    }
}

impl ChatMessage {
    /// Create a new user message
    /// If name is provided, it will be prepended to the content in the format "(name) content"
    pub fn user(content: String, name: String) -> Self {
        let formatted_content = format!("({name}) {content}");

        Self {
            role: "user".to_string(),
            content: formatted_content,
        }
    }

    /// Create a new assistant message
    pub fn assistant(content: String) -> Self {
        Self {
            role: "assistant".to_string(),
            content,
        }
    }

    /// Create a new developer message
    pub fn developer(content: String) -> Self {
        Self {
            role: "developer".to_string(),
            content,
        }
    }
}
