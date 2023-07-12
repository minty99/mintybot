use serenity::{model::prelude::ChannelId, prelude::Context};

use super::statics::DEV_USER_ID;

pub async fn say(ctx: &Context, channel: ChannelId, msg: impl std::fmt::Display) {
    if let Err(why) = channel.say(&ctx.http, msg).await {
        tracing::error!("Error sending message: {:?}", why);
    }
}

pub async fn send_dm_to_dev(ctx: &Context, msg: &str) -> eyre::Result<()> {
    if let Ok(user) = DEV_USER_ID.to_user(&ctx.http).await {
        user.dm(&ctx.http, |m| m.content(msg)).await?;
    }

    Ok(())
}
