#![feature(let_chains)]

mod utils;

use serenity::{async_trait, model::channel::Message, model::gateway::Ready, prelude::*};
use std::future::Future;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

use utils::conversation::add_user_message;
use utils::discord;
use utils::openai::get_chatgpt_response;
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
        let content = msg.content.clone();
        let channel_id = msg.channel_id;
        let _guild_id = msg.guild_id;

        // Check if the bot is mentioned in the message (either through Discord mentions or text mentions)
        let is_mentioned = msg.mentions_me(&ctx.http).await.unwrap_or(false);
        let bot_username = ctx.http.get_current_user().await.unwrap().name;
        let bot_user_id = ctx.http.get_current_user().await.unwrap().id;
        let contains_text_mention = content.contains(&format!("@{bot_username}"));

        if (is_mentioned || contains_text_mention) && !msg.author.bot {
            // Extract the message content without the mention
            let content_without_mention = content
                .replace(&format!("<@{bot_user_id}>"), "")
                .replace(&format!("@{bot_username}"), "")
                .trim()
                .to_string();

            // Send a typing indicator while processing
            let _ = channel_id.broadcast_typing(&ctx.http).await;

            // Log the received message
            tracing::info!("Received mention with message: {}", content_without_mention);

            // Add the user's message to the conversation history
            add_user_message(channel_id, content_without_mention.clone()).await;

            // Send the message to ChatGPT
            match get_chatgpt_response(channel_id).await {
                Ok(response) => {
                    // Send the response back to Discord
                    if let Err(why) = discord::say(&ctx, channel_id, &response).await {
                        tracing::error!("Error sending ChatGPT response: {:?}", why);
                    }
                }
                Err(err) => {
                    tracing::error!("Error getting ChatGPT response: {:?}", err);
                    // Send an error message to the channel
                    if let Err(why) = discord::say(
                        &ctx,
                        channel_id,
                        format!("Sorry, I couldn't get a response from ChatGPT at the moment. Error: {err}"),
                    )
                    .await
                    {
                        tracing::error!("Error sending error message: {:?}", why);
                    }
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

        discord::send_dm_to_dev(&ctx, &format!("{} started.", ready.user.name))
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
            sleep(Duration::from_secs(period)).await;
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
