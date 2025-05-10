use chrono::{DateTime, Local};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::utils::conversation::ChatMessage;

use super::msg_context::MsgContextInfo;

// Global logger instance
lazy_static::lazy_static! {
    static ref LOGGER: Arc<Mutex<Logger>> = Arc::new(Mutex::new(Logger::default()));
}

impl Default for Logger {
    fn default() -> Self {
        Self::new("logs")
    }
}

pub struct Logger {
    log_dir: String,
}

impl Logger {
    pub fn new(log_dir: &str) -> Self {
        // Ensure logs directory exists
        if !Path::new(log_dir).exists() {
            fs::create_dir_all(log_dir).expect("Failed to create logs directory");
        }

        Self {
            log_dir: log_dir.to_string(),
        }
    }

    // Log OpenAI request and response
    pub fn log_openai_conversation(
        &self,
        msg_ctx: &MsgContextInfo,
        messages: &[ChatMessage],
        response: &str,
        duration: Duration,
    ) -> std::io::Result<()> {
        let now: DateTime<Local> = Local::now();
        let timestamp = now.format("%Y-%m-%d %H:%M:%S %z").to_string();
        let log_file_path = format!("{}/conversations.log", self.log_dir);

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file_path)?;

        // Write separator and timestamp
        writeln!(
            file,
            "\n\n====================================================="
        )?;
        writeln!(file, "Channel ID: {}", msg_ctx.channel_id)?;
        if let Some(channel_name) = &msg_ctx.channel_name {
            writeln!(file, "Channel Name: {channel_name}")?;
        }
        if let Some(guild_id) = msg_ctx.guild_id {
            writeln!(file, "Guild ID: {guild_id}")?;
        }
        if let Some(guild_name) = &msg_ctx.guild_name {
            writeln!(file, "Guild Name: {guild_name}")?;
        }
        writeln!(file, "Timestamp: {timestamp} (KST)")?;
        writeln!(file, "API Call Duration: {duration:.2?}")?;
        writeln!(
            file,
            "====================================================="
        )?;

        // Write request messages
        writeln!(file, "\n[REQUEST]")?;
        for message in messages {
            writeln!(file, "{message}")?;
        }

        // Write response
        writeln!(file, "\n[RESPONSE]")?;
        writeln!(file, "{response}")?;

        Ok(())
    }
}

// Helper functions to interact with the global logger

/// Log an OpenAI conversation (request and response)
pub async fn log_openai_conversation(
    msg_ctx: &MsgContextInfo,
    messages: &[ChatMessage],
    response: &str,
    duration: Duration,
) -> std::io::Result<()> {
    let logger = LOGGER.lock().await;
    logger.log_openai_conversation(msg_ctx, messages, response, duration)
}
