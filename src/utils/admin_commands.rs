use serenity::model::channel::Message;
use serenity::model::id::UserId;
use serenity::prelude::*;
use std::str::FromStr;
use strum::IntoEnumIterator;

use crate::discord;
use crate::msg_context::MsgContextInfo;
use crate::statics::DEV_USER_ID;
use crate::utils::conversation::ChatMessage;
use crate::utils::persistence::{
    BotPersonality, add_message, change_model, get_channel_personality, get_conversation_history,
    get_current_model, get_total_history_count, remove_conversation, set_channel_personality,
};

use super::persistence::get_channel_ids;

/// Enum representing different admin command types
#[derive(Debug)]
pub enum AdminCommand {
    Forget,
    Model(String),
    Status,
    DevMessage(String),
    GetPersonality,
    SetPersonality(String),
}

/// Process an admin command if present in the message
pub async fn process_admin_command(
    ctx: &Context,
    _msg: &Message,
    msg_ctx: &MsgContextInfo,
    content: &str,
) -> bool {
    let Some(command) = parse_admin_command(content) else {
        return false;
    };

    // check admin
    if !is_admin(msg_ctx.author_id) {
        let _ = discord::say(
            ctx,
            msg_ctx.channel_id,
            "You are not admin. Request denied.",
        )
        .await;
        return false;
    }

    match command {
        AdminCommand::Forget => handle_forget_command(ctx, msg_ctx).await,
        AdminCommand::Model(model_name) => handle_model_command(ctx, msg_ctx, &model_name).await,
        AdminCommand::Status => handle_status_command(ctx, msg_ctx).await,
        AdminCommand::DevMessage(message) => handle_dev_command(ctx, msg_ctx, &message).await,
        AdminCommand::GetPersonality => handle_get_personality_command(ctx, msg_ctx).await,
        AdminCommand::SetPersonality(personality) => {
            handle_set_personality_command(ctx, msg_ctx, &personality).await
        }
    }

    true
}

/// Parse a message to check if it contains an admin command
fn parse_admin_command(content: &str) -> Option<AdminCommand> {
    let content = content.trim();

    if content == "<forget>" {
        return Some(AdminCommand::Forget);
    }

    if let Some(model_name) = content.strip_prefix("<model>") {
        return Some(AdminCommand::Model(model_name.trim().to_string()));
    }

    if content == "<status>" {
        return Some(AdminCommand::Status);
    }

    if let Some(dev_message) = content.strip_prefix("<dev>") {
        return Some(AdminCommand::DevMessage(dev_message.trim().to_string()));
    }

    if content == "<personality>" {
        return Some(AdminCommand::GetPersonality);
    }

    if let Some(personality) = content.strip_prefix("<personality>") {
        return Some(AdminCommand::SetPersonality(personality.trim().to_string()));
    }

    None
}

/// Check if the user is an admin (developer)
fn is_admin(author_id: UserId) -> bool {
    author_id == **DEV_USER_ID
}

/// Handles the forget command from authorized users
async fn handle_forget_command(ctx: &Context, msg_ctx: &MsgContextInfo) {
    let channel_id = msg_ctx.channel_id;

    // Clear conversation history for this channel
    remove_conversation(channel_id).await;

    // Send confirmation message
    let _ = discord::say(ctx, channel_id, "Conversation history has been cleared.").await;
}

/// Handles the model change command from authorized users
async fn handle_model_command(ctx: &Context, msg_ctx: &MsgContextInfo, model_name: &str) {
    let channel_id = msg_ctx.channel_id;

    // Trim the model name and check if it's empty
    let model_name = model_name.trim();
    if model_name.is_empty() {
        let _ = discord::say(ctx, channel_id, "Please specify a model name.").await;
        return;
    }

    // Change the model and get the response
    let response = change_model(model_name).await;

    // Send the response
    let _ = discord::say(ctx, channel_id, response).await;
}

/// Handles the status command to display bot state information
async fn handle_status_command(ctx: &Context, msg_ctx: &MsgContextInfo) {
    let channel_id = msg_ctx.channel_id;

    let current_model = get_current_model().await;
    let personality = get_channel_personality(channel_id).await;

    let channel_history = get_conversation_history(channel_id).await;
    let channel_history_count = channel_history.len().saturating_sub(1); // exclude system prompt

    let channel_ids = get_channel_ids().await;
    let channel_count = channel_ids.len();

    let total_history_count = get_total_history_count().await;

    let status_message = format!(
        "\
**Bot Status**
- Current model: `{current_model}`
- Current personality: `{personality}`
- This channel history: {channel_history_count} messages
- Total history: {total_history_count} messages across {channel_count} channels",
    );

    let _ = discord::say(ctx, channel_id, &status_message).await;
}

/// Handles the developer message command
async fn handle_dev_command(ctx: &Context, msg_ctx: &MsgContextInfo, dev_message: &str) {
    let channel_id = msg_ctx.channel_id;

    // Trim the developer message and check if it's empty
    let dev_message = dev_message.trim();
    if dev_message.is_empty() {
        let _ = discord::say(ctx, channel_id, "Please specify a developer message.").await;
        return;
    }

    // Add the developer message to the conversation history
    let dev_message = dev_message.to_string();
    add_message(channel_id, ChatMessage::developer(dev_message)).await;

    // Send confirmation
    let _ = discord::say(
        ctx,
        channel_id,
        "Developer message added to conversation history.",
    )
    .await;
}

/// Handles the get personality command
async fn handle_get_personality_command(ctx: &Context, msg_ctx: &MsgContextInfo) {
    let channel_id = msg_ctx.channel_id;

    // Get the current personality for this channel
    let personality = get_channel_personality(channel_id).await;

    // Get the system prompt for this personality
    let system_prompt = personality.get_system_prompt();

    // Format the message
    let message = format!(
        "**Current Personality**: `{personality}`\n\n**System Prompt**:\n```\n{system_prompt}\n```"
    );

    // Send the message
    let _ = discord::say(ctx, channel_id, &message).await;
}

/// Handles the set personality command
async fn handle_set_personality_command(
    ctx: &Context,
    msg_ctx: &MsgContextInfo,
    personality_input: &str,
) {
    let channel_id = msg_ctx.channel_id;

    // Trim the personality input and check if it's empty
    let personality_input = personality_input.trim();
    if personality_input.is_empty() {
        let _ = discord::say(ctx, channel_id, "Please specify a personality name.").await;
        return;
    }

    // Check for custom personality format: "custom <system prompt>"
    let personality = if personality_input.to_lowercase().starts_with("custom ") {
        // Extract the custom system prompt (everything after "custom ")
        let custom_prompt = personality_input[7..].trim().to_string();

        if custom_prompt.is_empty() {
            let _ = discord::say(
                ctx,
                channel_id,
                "Please provide a system prompt after 'custom'.",
            )
            .await;
            return;
        }

        // Create a custom personality with the provided prompt
        BotPersonality::custom(custom_prompt)
    } else {
        // Try to parse as a predefined personality
        match BotPersonality::from_str(personality_input) {
            Ok(p) => p,
            Err(_) => {
                // List all available personalities using EnumIter
                let mut available_personalities: Vec<String> = BotPersonality::iter()
                    .filter(|p| !matches!(p, BotPersonality::Custom(_))) // Filter out Custom
                    .map(|p| p.to_string())
                    .collect();

                // Add custom option
                available_personalities.push("Custom <system prompt>".to_string());

                let _ = discord::say(
                    ctx,
                    channel_id,
                    format!(
                        "Unknown personality: {personality_input}\nAvailable personalities: {}",
                        available_personalities.join(", ")
                    ),
                )
                .await;
                return;
            }
        }
    };

    // Set the personality for this channel
    set_channel_personality(channel_id, personality.clone()).await;

    // Send confirmation
    let _ = discord::say(
        ctx,
        channel_id,
        format!("Personality set to {personality} for this channel."),
    )
    .await;
}
