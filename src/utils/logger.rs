use chrono::{DateTime, Local};
use serenity::model::id::ChannelId;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::utils::conversation::ChatMessage;

// Global logger instance
lazy_static::lazy_static! {
    static ref LOGGER: Arc<Mutex<Logger>> = Arc::new(Mutex::new(Logger::new()));
}

pub struct Logger {
    log_dir: String,
}

impl Logger {
    pub fn new() -> Self {
        let log_dir = "logs".to_string();

        // Ensure logs directory exists
        if !Path::new(&log_dir).exists() {
            fs::create_dir_all(&log_dir).expect("Failed to create logs directory");
        }

        Self { log_dir }
    }

    // Log OpenAI request and response
    pub fn log_openai_conversation(
        &self,
        channel_id: ChannelId,
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
        writeln!(file, "Channel ID: {channel_id}")?;
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
    channel_id: ChannelId,
    messages: &[ChatMessage],
    response: &str,
    duration: std::time::Duration,
) -> std::io::Result<()> {
    let logger = LOGGER.lock().await;
    logger.log_openai_conversation(channel_id, messages, response, duration)
}
