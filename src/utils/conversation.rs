use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Display for ChatMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let role_str = format!("<{}>", self.role);
        let name_str = self
            .name
            .as_ref()
            .map(|name| format!("({name})"))
            .unwrap_or_default();
        let content_str = self.content.to_string();

        write!(
            f,
            "{}",
            itertools::join([role_str, name_str, content_str], " ")
        )
    }
}

impl ChatMessage {
    /// Create a new user message
    pub fn user(content: String, name: Option<String>) -> Self {
        Self {
            role: "user".to_string(),
            content,
            name,
        }
    }

    /// Create a new assistant message
    pub fn assistant(content: String) -> Self {
        Self {
            role: "assistant".to_string(),
            content,
            name: Some("MintyBot".to_string()),
        }
    }

    /// Create a new developer message
    pub fn developer(content: String) -> Self {
        Self {
            role: "developer".to_string(),
            content,
            name: None,
        }
    }
}
