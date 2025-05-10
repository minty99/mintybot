#![feature(let_chains)]

use serenity::{async_trait, model::channel::Message, model::gateway::Ready, prelude::*};
use std::future::Future;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

use mintybot::conversation::add_user_message;
use mintybot::discord;
use mintybot::msg_context::MsgContextInfo;
use mintybot::openai::get_chatgpt_response;
use mintybot::statics::DISCORD_TOKEN;
use mintybot::utils::admin_commands::process_admin_command;
use mintybot::utils::persistence::{load_state, save_state};

/// Handles bot mention detection and content processing
async fn handle_bot_mentions(ctx: &Context, msg: &Message) -> (bool, String) {
    let mintybot_role_id = if let Some(guild_id) = msg.guild_id {
        let roles = ctx.http.get_guild_roles(guild_id).await.unwrap();
        roles
            .iter()
            .find(|role| role.name == "MintyBot")
            .map(|role| role.id)
    } else {
        None
    };

    let role_mentioned = if let Some(mintybot_role_id) = mintybot_role_id {
        msg.mention_roles.contains(&mintybot_role_id)
    } else {
        false
    };

    let regular_mentioned = msg.mentions_me(&ctx.http).await.unwrap_or(false);

    let bot_user = ctx.http.get_current_user().await.unwrap();

    let regular_mention = format!("<@{}>", bot_user.id);
    let role_mention = mintybot_role_id
        .map(|id| format!("<@&{id}>"))
        .unwrap_or_default();

    let contains_mention = regular_mentioned || role_mentioned;

    let content_without_mention = msg
        .content
        .clone()
        .replace(&regular_mention, "") // regular discord mention
        .replace(&role_mention, "") // role mention
        .trim()
        .to_string();

    (contains_mention, content_without_mention)
}

/// Process a message that mentions the bot and send a response
async fn process_bot_mention(
    ctx: &Context,
    msg_ctx: &MsgContextInfo,
    content: String,
    name: String,
) {
    // Add the user's message to the conversation history
    add_user_message(msg_ctx.channel_id, content.clone(), Some(name)).await;

    // Send the message to ChatGPT and handle the response
    match get_chatgpt_response(msg_ctx).await {
        Ok(response) => {
            // Send the response back to Discord
            if let Err(why) = discord::say(ctx, msg_ctx.channel_id, &response).await {
                tracing::error!("Error sending ChatGPT response: {:?}", why);
            }
        }
        Err(err) => {
            tracing::error!("Error getting ChatGPT response: {:?}", err);
            // Send an error message to the channel
            let error_message = format!(
                "Sorry, I couldn't get a response from ChatGPT at the moment. Error: {err}"
            );
            if let Err(why) = discord::say(ctx, msg_ctx.channel_id, error_message).await {
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
        let author = msg.author.clone();

        tracing::debug!("Text: {:?}", msg);

        // Skip messages from bots
        if author.bot {
            return;
        }

        // Check if the bot is mentioned in the message
        let is_mentioned = msg.mentions_me(&ctx.http).await.unwrap_or(false);
        let (contains_text_mention, content_without_mention) =
            handle_bot_mentions(&ctx, &msg).await;

        if is_mentioned || contains_text_mention {
            // Create message context info
            let msg_ctx = MsgContextInfo::from_message(&ctx, &msg).await;

            // Send a typing indicator while processing
            let _ = msg.channel_id.broadcast_typing(&ctx.http).await;

            // Log the received message
            tracing::info!("Received mention with message: {}", content_without_mention);

            // Check if this is an admin command and process it if so
            if process_admin_command(&ctx, &msg, &msg_ctx, &content_without_mention).await {
                return;
            }

            let selected_name = get_best_name_of_author(&ctx, &msg_ctx).await;

            // Process the mention and send a response
            process_bot_mention(&ctx, &msg_ctx, content_without_mention, selected_name).await;
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

async fn get_best_name_of_author(ctx: &Context, msg_ctx: &MsgContextInfo) -> String {
    let nick = match msg_ctx.guild_id {
        Some(guild_id) => msg_ctx.author.nick_in(&ctx.http, guild_id).await,
        None => None,
    };
    let display_name = msg_ctx.author.global_name.clone();
    let user_name = Some(msg_ctx.author.name.clone());

    vec![nick, display_name, user_name]
        .into_iter()
        .find(|opt| opt.is_some())
        .flatten()
        .unwrap()
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

    // Load the bot state from disk
    if let Err(e) = load_state().await {
        tracing::error!("Failed to load bot state: {}", e);
        // Continue with default state if loading fails
    }

    // Set up a clean shutdown handler to save state when the bot is terminated
    setup_shutdown_handler();

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot
    let mut client = create_discord_client(intents).await?;

    // Start the client and handle any errors
    if let Err(why) = client.start().await {
        tracing::error!("Client error: {:?}", why);
        // Save state before exiting due to error
        if let Err(e) = save_state().await {
            tracing::error!("Failed to save state on shutdown: {}", e);
        }
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

/// Set up a clean shutdown handler to save state when the bot is terminated
fn setup_shutdown_handler() {
    // Use tokio's signal handling to catch termination signals
    tokio::spawn(async {
        // Create a future that completes when SIGINT or SIGTERM is received
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Failed to install signal handler")
                .recv()
                .await;
        };

        // Wait for either signal
        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }

        // Save state before shutting down
        tracing::info!("Received shutdown signal, saving state...");
        if let Err(e) = save_state().await {
            tracing::error!("Failed to save state on shutdown: {}", e);
        } else {
            tracing::info!("State saved successfully, shutting down.");
        }

        // Exit the process
        std::process::exit(0);
    });
}
