use serenity::{model::prelude::ChannelId, prelude::Context};

use super::statics::DEV_USER_ID;

/// Send a message to a Discord channel, automatically handling message chunking for long messages
pub async fn say(ctx: &Context, channel: ChannelId, msg: impl std::fmt::Display) -> eyre::Result<()> {
    // Convert the message to a string
    let content = msg.to_string();
    
    // Discord has a 2000 character limit per message
    const DISCORD_MESSAGE_LIMIT: usize = 2000;
    
    if content.len() <= DISCORD_MESSAGE_LIMIT {
        // Send as a single message if it's short enough
        channel.say(&ctx.http, content).await.map_err(|e| eyre::eyre!("{}", e))?;
    } else {
        send_chunked_message(ctx, channel, content).await?;
    }
    
    Ok(())
}

/// Split a long message into chunks and send them sequentially
async fn send_chunked_message(ctx: &Context, channel: ChannelId, content: String) -> eyre::Result<()> {
    // Discord has a 2000 character limit per message
    const DISCORD_MESSAGE_LIMIT: usize = 2000;
    
    // Split the message into chunks
    let mut remaining = content.as_str();
    
    while !remaining.is_empty() {
        let chunk_size = std::cmp::min(DISCORD_MESSAGE_LIMIT, remaining.len());
        
        // Try to find a good breaking point (newline or space)
        let actual_size = find_chunk_break_point(remaining, chunk_size);
        
        // Send this chunk
        let chunk = &remaining[..actual_size];
        channel.say(&ctx.http, chunk).await.map_err(|e| eyre::eyre!("{}", e))?;
        
        // Move to the next chunk
        remaining = &remaining[actual_size..];
    }
    
    Ok(())
}

/// Find an appropriate break point for message chunking
fn find_chunk_break_point(text: &str, max_size: usize) -> usize {
    if max_size >= text.len() {
        return text.len();
    }
    
    // Look for a newline or space to break at
    if let Some(pos) = text[..max_size].rfind('\n') {
        pos + 1 // Include the newline
    } else if let Some(pos) = text[..max_size].rfind(' ') {
        pos + 1 // Include the space
    } else {
        max_size // No good break point found, just cut at max_size
    }
}

/// Send a direct message to the developer
pub async fn send_dm_to_dev(ctx: &Context, msg: &str) -> eyre::Result<()> {
    if let Ok(user) = DEV_USER_ID.to_user(&ctx.http).await {
        user.dm(&ctx.http, |m| m.content(msg)).await.map_err(|e| eyre::eyre!("{}", e))?;
    }

    Ok(())
}
