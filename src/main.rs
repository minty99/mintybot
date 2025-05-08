#![feature(let_chains)]

mod utils;

use serenity::model::id::{ChannelId, UserId};
use serenity::model::user::CurrentUser;
use serenity::{async_trait, model::channel::Message, model::gateway::Ready, prelude::*};
use std::future::Future;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

use utils::conversation::{CONVERSATION_MANAGER, clear_conversation_history};
use utils::conversation::{ConversationMessage, add_user_message};
use utils::discord;
use utils::openai::{change_model, get_chatgpt_response};
use utils::statics::DEV_USER_ID;
use utils::statics::DISCORD_TOKEN;

/// Handles bot mention detection and content processing
fn handle_bot_mentions(content: &str, bot_user: CurrentUser) -> (bool, String) {
    let bot_user_id = bot_user.id;
    let bot_username = bot_user.name;
    let regular_mention = format!("<@{bot_user_id}>");
    let text_mention = format!("@{bot_username}");

    let contains_mention = content.contains(&regular_mention) || content.contains(&text_mention);

    let content_without_mention = content
        .replace(&regular_mention, "") // regular discord mention
        .replace(&text_mention, "") // text mention with username
        .trim()
        .to_string();

    (contains_mention, content_without_mention)
}

/// Handles the forget command from authorized users
async fn handle_forget_command(ctx: &Context, channel_id: ChannelId, author_id: UserId) {
    if author_id != **DEV_USER_ID {
        let _ = channel_id
            .say(&ctx.http, "You are not admin. Request denied.")
            .await;
        return;
    }

    // Clear conversation history for this channel
    clear_conversation_history(channel_id).await;

    // Send confirmation message
    if let Err(why) = discord::say(ctx, channel_id, "Conversation history has been cleared.").await
    {
        tracing::error!("Error sending confirmation message: {:?}", why);
    }
}

/// Handles the model change command from authorized users
async fn handle_model_command(
    ctx: &Context,
    channel_id: ChannelId,
    author_id: UserId,
    model_name: &str,
) {
    // Only allow the developer to change the model
    if author_id != **DEV_USER_ID {
        let _ = channel_id
            .say(&ctx.http, "You are not admin. Request denied.")
            .await;
        return;
    }
    
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

/// Handles the developer message command
async fn handle_dev_command(
    ctx: &Context,
    channel_id: ChannelId,
    author_id: UserId,
    dev_message: &str,
) {
    // Only allow the developer to send developer messages
    if author_id != **DEV_USER_ID {
        let _ = channel_id
            .say(&ctx.http, "You are not admin. Request denied.")
            .await;
        return;
    }
    
    // Trim the developer message and check if it's empty
    let dev_message = dev_message.trim();
    if dev_message.is_empty() {
        let _ = channel_id
            .say(&ctx.http, "Please specify a developer message.")
            .await;
        return;
    }

    // Add the developer message to the conversation history
    let mut manager = CONVERSATION_MANAGER.lock().await;
    manager.add_message(
        channel_id,
        ConversationMessage::developer(dev_message.to_string()),
    );
    drop(manager); // Release the lock

    // Send confirmation
    let _ = channel_id
        .say(
            &ctx.http,
            "Developer message added to conversation history.",
        )
        .await;
}

/// Process a message that mentions the bot and send a response
async fn process_bot_mention(
    ctx: &Context,
    channel_id: ChannelId,
    content: String,
    name: Option<String>,
) {
    // Add the user's message to the conversation history
    add_user_message(channel_id, content.clone(), name).await;

    // Send the message to ChatGPT and handle the response
    match get_chatgpt_response(channel_id).await {
        Ok(response) => {
            // Send the response back to Discord
            if let Err(why) = discord::say(ctx, channel_id, &response).await {
                tracing::error!("Error sending ChatGPT response: {:?}", why);
            }
        }
        Err(err) => {
            tracing::error!("Error getting ChatGPT response: {:?}", err);
            // Send an error message to the channel
            let error_message = format!(
                "Sorry, I couldn't get a response from ChatGPT at the moment. Error: {err}"
            );
            if let Err(why) = discord::say(ctx, channel_id, error_message).await {
                tracing::error!("Error sending error message: {:?}", why);
            }
        }
    }
}

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
        let author = msg.author.clone();

        // Skip messages from bots
        if author.bot {
            return;
        }

        tracing::debug!("Text: {}", content);

        // Check if the bot is mentioned in the message
        let is_mentioned = msg.mentions_me(&ctx.http).await.unwrap_or(false);
        let (contains_text_mention, content_without_mention) =
            handle_bot_mentions(&content, ctx.http.get_current_user().await.unwrap());

        if is_mentioned || contains_text_mention {
            // Send a typing indicator while processing
            let _ = channel_id.broadcast_typing(&ctx.http).await;

            // Check if this is a forget command
            if content_without_mention.trim() == "<forget>" {
                handle_forget_command(&ctx, channel_id, author.id).await;
                return;
            }

            // Check if this is a model change command
            if let Some(model_name) = content_without_mention.trim().strip_prefix("<model>") {
                handle_model_command(&ctx, channel_id, author.id, model_name).await;
                return;
            }

            // Check if this is a developer message command
            if let Some(dev_message) = content_without_mention.trim().strip_prefix("<dev>") {
                handle_dev_command(&ctx, channel_id, author.id, dev_message).await;
                return;
            }

            // Log the received message
            tracing::info!("Received mention with message: {}", content_without_mention);

            // Get the name of the message author
            let name = Some(author.name.clone());

            // Process the mention and send a response
            process_bot_mention(&ctx, channel_id, content_without_mention, name).await;
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
        let bot_name = ready.user.name.clone();
        tracing::info!("{} is connected!", bot_name);

        // Notify developer that the bot has started
        notify_bot_startup(&ctx, &bot_name).await;

        // Here you could spawn background tasks if needed
        // spawn_periodic_tasks(Arc::new(ctx));
    }
}

/// Notify the developer that the bot has started
async fn notify_bot_startup(ctx: &Context, bot_name: &str) {
    let startup_message = format!("{bot_name} started.");
    if let Err(err) = discord::send_dm_to_dev(ctx, &startup_message).await {
        tracing::error!("Failed to send startup notification: {:?}", err);
    }
}

/// Spawn periodic background tasks
#[allow(dead_code)]
fn spawn_periodic_tasks(_ctx: Arc<Context>) {
    // Example: spawn a web watcher task
    // spawn_periodic_task(ctx, web_watcher::watch_website, 3600);
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
async fn main() -> eyre::Result<()> {
    // Initialize the tracing subscriber for logging
    tracing_subscriber::fmt::init();

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot
    let mut client = create_discord_client(intents).await?;

    // Start the client and handle any errors
    if let Err(why) = client.start().await {
        tracing::error!("Client error: {:?}", why);
        return Err(eyre::eyre!("Client error: {:?}", why));
    }

    Ok(())
}

/// Create and configure the Discord client
async fn create_discord_client(intents: GatewayIntents) -> eyre::Result<Client> {
    Client::builder(&**DISCORD_TOKEN, intents)
        .event_handler(MintyBotHandler {})
        .await
        .map_err(|e| eyre::eyre!("Failed to create Discord client: {}", e))
}
