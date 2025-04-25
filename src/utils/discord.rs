use serenity::{model::prelude::ChannelId, prelude::Context};

use super::statics::DEV_USER_ID;

pub async fn say(ctx: &Context, channel: ChannelId, msg: impl std::fmt::Display) -> Result<(), serenity::Error> {
    // Convert the message to a string
    let content = msg.to_string();
    
    // Discord has a 2000 character limit per message
    const DISCORD_MESSAGE_LIMIT: usize = 2000;
    
    if content.len() <= DISCORD_MESSAGE_LIMIT {
        // Send as a single message if it's short enough
        channel.say(&ctx.http, content).await?;
    } else {
        // Split the message into chunks
        let mut remaining = content.as_str();
        
        while !remaining.is_empty() {
            let chunk_size = std::cmp::min(DISCORD_MESSAGE_LIMIT, remaining.len());
            
            // Try to find a good breaking point (newline or space)
            let mut actual_size = chunk_size;
            if chunk_size < remaining.len() {
                // Look for a newline or space to break at
                if let Some(pos) = remaining[..chunk_size].rfind('\n') {
                    actual_size = pos + 1; // Include the newline
                } else if let Some(pos) = remaining[..chunk_size].rfind(' ') {
                    actual_size = pos + 1; // Include the space
                }
            }
            
            // Send this chunk
            let chunk = &remaining[..actual_size];
            channel.say(&ctx.http, chunk).await?;
            
            // Move to the next chunk
            remaining = &remaining[actual_size..];
        }
    }
    
    Ok(())
}

pub async fn send_dm_to_dev(ctx: &Context, msg: &str) -> eyre::Result<()> {
    if let Ok(user) = DEV_USER_ID.to_user(&ctx.http).await {
        user.dm(&ctx.http, |m| m.content(msg)).await?;
    }

    Ok(())
}
