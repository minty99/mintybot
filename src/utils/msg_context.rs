use lazy_static::lazy_static;
use serenity::model::id::{ChannelId, GuildId};
use serenity::prelude::Context;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A struct that holds Discord context information about a message
#[derive(Debug, Clone)]
pub struct MsgContextInfo {
    pub channel_id: ChannelId,
    pub channel_name: Option<String>,
    pub guild_id: Option<GuildId>,
    pub guild_name: Option<String>,
}

// Global cache for MsgContextInfo
lazy_static! {
    static ref CONTEXT_CACHE: Arc<Mutex<HashMap<ChannelId, MsgContextInfo>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

impl MsgContextInfo {
    /// Create a new MsgContextInfo from a message context, using cache when available
    pub async fn from_channel_id(ctx: &Context, channel_id: ChannelId) -> Self {
        // Check if we already have this channel in the cache
        let mut cache = CONTEXT_CACHE.lock().await;

        // If we have a cached entry, return a clone of it
        if let Some(cached_info) = cache.get(&channel_id) {
            return cached_info.clone();
        }

        // Otherwise, fetch the information from Discord API
        let channel_name = channel_id.name(&ctx.http).await.ok();
        let mut guild_id = None;
        let mut guild_name = None;

        // Try to get channel information
        if let Ok(channel) = channel_id.to_channel(&ctx.http).await {
            // Try to get guild information if this is a guild channel. Private channel is not implemented yet.
            if let Some(guild_channel) = channel.guild() {
                let guild_id_value = guild_channel.guild_id;
                guild_id = Some(guild_id_value);

                // Try to get guild name
                if let Ok(guild) = guild_id_value.to_partial_guild(&ctx.http).await {
                    guild_name = Some(guild.name);
                }
            }
        }

        // Create the new context info
        let info = Self {
            channel_id,
            channel_name,
            guild_id,
            guild_name,
        };

        // Store in cache for future use
        cache.insert(channel_id, info.clone());

        info
    }
}
