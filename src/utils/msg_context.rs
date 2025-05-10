use serenity::model::channel::Message;
use serenity::model::id::{ChannelId, GuildId, UserId};
use serenity::model::user::User;
use serenity::prelude::Context;

/// A struct that holds Discord context information about a message
#[derive(Debug, Clone)]
pub struct MsgContextInfo {
    pub channel_id: ChannelId,
    pub channel_name: Option<String>,
    pub guild_id: Option<GuildId>,
    pub guild_name: Option<String>,
    pub author_id: UserId,
    pub author: User,
}

impl MsgContextInfo {
    /// Create a new MsgContextInfo from a Message
    pub async fn from_message(ctx: &Context, msg: &Message) -> Self {
        let channel_id = msg.channel_id;
        let author = msg.author.clone();
        let author_id = author.id;

        // Try to get channel and guild information
        let channel_name = channel_id.name(&ctx.http).await.ok();
        let mut guild_id = None;
        let mut guild_name = None;

        // Try to get channel information
        if let Ok(channel) = channel_id.to_channel(&ctx.http).await {
            // Try to get guild information if this is a guild channel
            if let Some(guild_channel) = channel.guild() {
                let guild_id_value = guild_channel.guild_id;
                guild_id = Some(guild_id_value);

                // Try to get guild name
                if let Ok(guild) = guild_id_value.to_partial_guild(&ctx.http).await {
                    guild_name = Some(guild.name);
                }
            }
        }

        Self {
            channel_id,
            channel_name,
            guild_id,
            guild_name,
            author_id,
            author,
        }
    }
}
