#![feature(let_chains)]

mod utils;

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use utils::discord::{self, send_dm_to_dev};
use utils::statics::DISCORD_TOKEN;

struct MintyBotHandler {}

#[async_trait]
impl EventHandler for MintyBotHandler {
    // Set a handler for the `message` event - so that whenever a new message
    // is received - the closure (or function) passed will be called.
    //
    // Event handlers are dispatched through a threadpool, and so multiple
    // events can be dispatched simultaneously.
    async fn message(&self, ctx: Context, msg: Message) {
        let content = msg.content;
        let channel_id = msg.channel_id;
        let _guild_id = msg.guild_id;

        if content == "!weather" {
            let weather_info = kma::get_weather().await;
            match weather_info {
                Ok(info) => {
                    discord::say(&ctx, channel_id, info).await;
                }
                Err(why) => {
                    discord::say(&ctx, channel_id, "Internal error occured.".to_string()).await;
                    tracing::error!("Error getting weather: {:?}", why);
                }
            }
        }
    }

    // Set a handler to be called on the `ready` event. This is called when a
    // shard is booted, and a READY payload is sent by Discord. This payload
    // contains data like the current user's guild Ids, current user data,
    // private channels, and more.
    //
    // In this case, just print what the current user's username is.
    #[allow(unused_variables)]
    async fn ready(&self, ctx: Context, ready: Ready) {
        tracing::info!("{} is connected!", ready.user.name);

        send_dm_to_dev(&ctx, &format!("{} started.", ready.user.name))
            .await
            .ok();

        // Spawn a background task to run the job every hour
        // For example, use web_watcher::watch_web_site to watch a website
    }
}

#[allow(dead_code)]
fn spawn_periodic_task<F, T, Fut>(ctx: Arc<Context>, f: F, period: u64)
where
    F: Fn(Arc<Context>, Option<T>) -> Fut + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    Fut: Future<Output = eyre::Result<T>> + Send,
{
    tokio::spawn(async move {
        let mut prev_result = None;
        loop {
            let result = f(ctx.clone(), prev_result.clone()).await;
            match result {
                Ok(value) => {
                    prev_result = Some(value);
                }
                Err(err) => {
                    tracing::warn!("Error: {:?}", err);
                }
            }
            tokio::time::sleep(Duration::from_secs(period)).await;
        }
    });
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let mut client = Client::builder(&**DISCORD_TOKEN, intents)
        .event_handler(MintyBotHandler {})
        .await
        .expect("Err creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        tracing::error!("Client error: {:?}", why);
    }
}
