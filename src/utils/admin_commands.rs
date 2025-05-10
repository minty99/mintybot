use serenity::model::channel::Message;
use serenity::model::id::UserId;
use serenity::prelude::*;

use crate::conversation::{ChatMessage, clear_conversation_history};
use crate::discord;
use crate::msg_context::MsgContextInfo;
use crate::openai::change_model;
use crate::statics::DEV_USER_ID;
use crate::utils::persistence::{BOT_STATE, save_state};

/// Enum representing different admin command types
#[derive(Debug)]
pub enum AdminCommand {
    Forget,
    Model(String),
    Status,
    DevMessage(String),
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
        if let Err(e) = discord::say(
            ctx,
            msg_ctx.channel_id,
            "You are not admin. Request denied.",
        )
        .await
        {
            tracing::error!("Failed to send message: {e}");
        }
        return false;
    }

    match command {
        AdminCommand::Forget => handle_forget_command(ctx, msg_ctx).await,
        AdminCommand::Model(model_name) => handle_model_command(ctx, msg_ctx, &model_name).await,
        AdminCommand::Status => handle_status_command(ctx, msg_ctx).await,
        AdminCommand::DevMessage(message) => handle_dev_command(ctx, msg_ctx, &message).await,
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
    clear_conversation_history(channel_id).await;

    // Send confirmation message
    if let Err(why) = discord::say(ctx, channel_id, "Conversation history has been cleared.").await
    {
        tracing::error!("Error sending confirmation message: {:?}", why);
    }
}

/// Handles the model change command from authorized users
async fn handle_model_command(ctx: &Context, msg_ctx: &MsgContextInfo, model_name: &str) {
    let channel_id = msg_ctx.channel_id;

    // Trim the model name and check if it's empty
    let model_name = model_name.trim();
    if model_name.is_empty() {
        let _ = channel_id
            .say(&ctx.http, "Please specify a model name.")
            .await;
        return;
    }

    // Change the model and get the response
    let response = change_model(model_name).await;

    // Send the response
    let _ = channel_id.say(&ctx.http, response).await;
}

/// Handles the status command to display bot state information
async fn handle_status_command(ctx: &Context, msg_ctx: &MsgContextInfo) {
    let channel_id = msg_ctx.channel_id;

    // Get the bot state
    let state = BOT_STATE.lock().await;

    // Get current model
    let current_model = &state.current_model;

    // Get conversation history count for this channel
    let channel_history_count = state
        .conversations
        .get(&channel_id)
        .map_or(0, |history| history.len());

    // Get total conversation history count across all channels
    let total_history_count: usize = state
        .conversations
        .values()
        .map(|history| history.len())
        .sum();

    // Count number of channels with conversation history
    let channel_count = state.conversations.len();

    // Format the status message
    let status_message = format!(
        "\
**Bot Status**
- Current model: `{current_model}`
- This channel history: {channel_history_count} messages
- Total history: {total_history_count} messages across {channel_count} channels",
    );

    // Send the status message
    if let Err(why) = discord::say(ctx, channel_id, &status_message).await {
        tracing::error!("Error sending status message: {:?}", why);
    }
}

/// Handles the developer message command
async fn handle_dev_command(ctx: &Context, msg_ctx: &MsgContextInfo, dev_message: &str) {
    let channel_id = msg_ctx.channel_id;

    // Trim the developer message and check if it's empty
    let dev_message = dev_message.trim();
    if dev_message.is_empty() {
        let _ = channel_id
            .say(&ctx.http, "Please specify a developer message.")
            .await;
        return;
    }

    // Add the developer message to the conversation history
    let dev_message = dev_message.to_string();
    {
        let mut state = BOT_STATE.lock().await;
        state.add_message(channel_id, ChatMessage::developer(dev_message));
    }

    // Save state
    let _ = save_state().await;

    // Send confirmation
    let _ = channel_id
        .say(
            &ctx.http,
            "Developer message added to conversation history.",
        )
        .await;
}
