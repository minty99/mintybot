use serenity::{all::CreateMessage, model::prelude::ChannelId, prelude::Context};

use super::statics::DEV_USER_ID;

/// Send a message to a Discord channel, automatically handling message chunking for long messages
pub async fn say(
    ctx: &Context,
    channel: ChannelId,
    msg: impl std::fmt::Display,
) -> eyre::Result<()> {
    // Convert the message to a string
    let content = msg.to_string();

    // Discord has a 2000 character limit per message
    const DISCORD_MESSAGE_LIMIT: usize = 2000;

    if content.len() <= DISCORD_MESSAGE_LIMIT {
        // Send as a single message if it's short enough
        channel
            .say(&ctx.http, content)
            .await
            .map_err(|e| eyre::eyre!("{}", e))?;
    } else {
        send_chunked_message(ctx, channel, content).await?;
    }

    Ok(())
}

/// Split a long message into chunks and send them sequentially
async fn send_chunked_message(
    ctx: &Context,
    channel: ChannelId,
    content: String,
) -> eyre::Result<()> {
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
        channel
            .say(&ctx.http, chunk)
            .await
            .map_err(|e| eyre::eyre!("{}", e))?;

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

    // Ensure max_size is at a valid character boundary
    let safe_max_size = find_safe_boundary(text, max_size);

    // 1. First try to find a good break point (newline or space) within safe_max_size
    if let Some(pos) = text[..safe_max_size].rfind('\n') {
        return pos + 1; // Include the newline
    }

    if let Some(pos) = text[..safe_max_size].rfind(' ') {
        return pos + 1; // Include the space
    }

    // 2. If no good break point found, use the safe character boundary
    safe_max_size
}

/// Find a safe character boundary at or before the given position
fn find_safe_boundary(text: &str, pos: usize) -> usize {
    let mut safe_pos = std::cmp::min(pos, text.len());
    while safe_pos > 0 && !text.is_char_boundary(safe_pos) {
        safe_pos -= 1;
    }
    safe_pos
}

/// Send a direct message to the developer
pub async fn send_dm_to_dev(ctx: &Context, msg: &str) -> eyre::Result<()> {
    if let Ok(user) = DEV_USER_ID.to_user(&ctx.http).await {
        let message = CreateMessage::new().content(msg);
        user.dm(&ctx.http, message)
            .await
            .map_err(|e| eyre::eyre!("{}", e))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_chunk_break_point_with_newline() {
        // Test with newline character
        let text = "This is a test\nwith a newline";
        let max_size = 15; // After "This is a test"
        assert_eq!(find_chunk_break_point(text, max_size), 15); // Should break at newline + 1
    }

    #[test]
    fn test_find_chunk_break_point_with_space() {
        // Test with space character
        let text = "This is a test with spaces";
        let max_size = 15; // After "This is a test"
        assert_eq!(find_chunk_break_point(text, max_size), 15); // Should break at space + 1
    }

    #[test]
    fn test_find_chunk_break_point_with_korean() {
        // Test with Korean characters (multi-byte)
        let text = "안녕하세요 반갑습니다";
        let max_size = 10; // In the middle of Korean text

        // Get result
        let result = find_chunk_break_point(text, max_size);

        // Verify the result is a valid character boundary
        assert!(text.is_char_boundary(result));

        // Verify the result is less than or equal to max_size
        assert!(result <= max_size);

        // Since max_size is 10, which is in the middle of '세' (bytes 9..12),
        // the function should return 9 (the start of '세')
        assert_eq!(result, 9);
    }

    #[test]
    fn test_find_chunk_break_point_with_no_good_break() {
        // Test with no good break point (no space or newline)
        let text = "안녕하세요반갑습니다";
        let max_size = 10; // In the middle of Korean text

        // The result should be a valid character boundary
        let result = find_chunk_break_point(text, max_size);
        assert!(text.is_char_boundary(result));

        // Should find a safe character boundary
        assert!(result <= max_size);
    }

    #[test]
    fn test_find_chunk_break_point_with_exact_size() {
        // Test with exact size match
        let text = "This is a test";
        let max_size = text.len();
        assert_eq!(find_chunk_break_point(text, max_size), text.len());
    }

    #[test]
    fn test_find_chunk_break_point_with_larger_size() {
        // Test with max_size larger than text length
        let text = "This is a test";
        let max_size = text.len() + 10;
        assert_eq!(find_chunk_break_point(text, max_size), text.len());
    }
}
