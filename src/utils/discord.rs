use serenity::{model::prelude::ChannelId, prelude::Context};

pub async fn say(ctx: &Context, channel: ChannelId, msg: impl std::fmt::Display) {
    if let Err(why) = channel.say(&ctx.http, msg).await {
        println!("Error sending message: {:?}", why);
    }
}
