#![feature(let_chains)]

use dotenvy::dotenv;
use fs2::FileExt;
use serenity::all::UserId;
use serenity::{async_trait, model::channel::Message, model::gateway::Ready, prelude::*};
use std::fs::File;
use std::path::Path;

use mintybot::discord;
use mintybot::msg_context::MsgContextInfo;
use mintybot::openai::get_openai_response;
use mintybot::statics::DISCORD_TOKEN;
use mintybot::utils::admin_commands::process_admin_command;
use mintybot::utils::conversation::ChatMessage;
use mintybot::utils::persistence::add_message;
use mintybot::utils::persistence::{load_state, save_state};

fn clean_message_content(msg: &Message, user_id: UserId) -> String {
    let mut content = msg.content.clone();

    // Remove bot mention
    let user_mention = format!("<@{user_id}>");
    let user_mention_nick = format!("<@!{user_id}>");

    content = content.replace(&user_mention, "");
    content = content.replace(&user_mention_nick, "");

    // Remove role mentions
    for role in &msg.mention_roles {
        let role_mention = format!("<@&{role}>");
        content = content.replace(&role_mention, "");
    }

    content.trim().to_string()
}

async fn check_mentioned(ctx: &Context, msg: &Message) -> bool {
    // Extract the necessary information from current_user and drop the reference immediately
    let (user_id, user_name) = {
        let current_user = ctx.cache.current_user();
        (current_user.id, current_user.name.clone())
    };

    let role_mentioned = match msg.guild_id {
        Some(guild_id) => match guild_id.member(&ctx, user_id).await {
            Ok(member) => {
                // Try to get roles from cache first, then fall back to HTTP request if needed
                let roles = match member.roles(ctx) {
                    Some(roles) => roles,
                    None => {
                        // Roles not in cache, fetch them via HTTP
                        match guild_id.roles(&ctx.http).await {
                            Ok(all_roles) => {
                                // Filter roles to only include those assigned to the member
                                all_roles
                                    .into_iter()
                                    .filter(|(role_id, _)| member.roles.contains(role_id))
                                    .map(|(_, role)| role)
                                    .collect()
                            }
                            Err(err) => {
                                tracing::error!("Failed to fetch guild roles: {}", err);
                                Vec::new()
                            }
                        }
                    }
                };

                roles
                    .into_iter()
                    .find_map(|role| (role.name == user_name).then_some(role.id))
                    .map(|id| msg.mention_roles.contains(&id))
                    .unwrap_or(false)
            }
            Err(err) => {
                tracing::error!("Failed to get member: {}", err);
                false
            }
        },
        None => false,
    };
    let regular_mentioned = msg.mentions_me(&ctx).await.unwrap_or(false);

    regular_mentioned || role_mentioned
}

/// Process a message that mentions the bot and send a response
async fn process_bot_mention(
    ctx: &Context,
    msg_ctx: &MsgContextInfo,
    content: String,
    name: String,
) {
    // Add the user's message to the conversation history
    let message = ChatMessage::user(content.clone(), name);
    add_message(msg_ctx.channel_id, message).await;

    // Send the message to OpenAI and handle the response
    match get_openai_response(msg_ctx).await {
        Ok(response) => {
            // Send the response back to Discord
            if let Err(why) = discord::say(ctx, msg_ctx.channel_id, &response).await {
                tracing::error!("Error sending OpenAI response: {:?}", why);
            }
        }
        Err(err) => {
            tracing::error!("Error getting OpenAI response: {:?}", err);
            // Send an error message to the channel
            let error_message =
                format!("Sorry, I couldn't get a response from OpenAI at the moment. Error: {err}");
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

        // Skip messages from bots
        if author.bot {
            return;
        }

        // Check if the bot is mentioned in the message
        let is_mentioned = check_mentioned(&ctx, &msg).await;
        let content_without_mention = clean_message_content(&msg, ctx.cache.current_user().id);

        if is_mentioned {
            // Create message context info
            let msg_ctx = MsgContextInfo::from_message(&ctx, &msg).await;

            // Send a typing indicator while processing
            let _ = msg.channel_id.broadcast_typing(&ctx.http).await;

            // Log the received message
            tracing::debug!("Request: {:#?}", msg);

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
    async fn ready(&self, ctx: Context, ready: Ready) {
        let bot_name = ready.user.name.clone();
        tracing::info!("{} is connected!", bot_name);

        // Notify developer that the bot has started
        notify_bot_startup(&ctx, &bot_name).await;
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

/// Acquire a file lock to ensure only one instance of the bot is running
fn acquire_instance_lock() -> eyre::Result<File> {
    // Create data directory if it doesn't exist
    let lock_path = Path::new("data");
    if !lock_path.exists() {
        std::fs::create_dir_all(lock_path)?;
    }

    // Try to acquire the lock file
    let lock_file_path = lock_path.join("mintybot.lock");
    let file = File::create(&lock_file_path)?;

    // Try to acquire an exclusive lock
    match file.try_lock_exclusive() {
        Ok(_) => {
            tracing::info!("Successfully acquired instance lock");
            Ok(file)
        }
        Err(e) => {
            tracing::error!(
                "Failed to acquire instance lock: another instance of MintyBot is already running"
            );
            Err(eyre::eyre!(
                "Another instance of MintyBot is already running: {}",
                e
            ))
        }
    }
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Load .env file if present
    dotenv().ok();

    // Initialize the tracing subscriber for logging
    tracing_subscriber::fmt::init();

    // Ensure only one instance of the bot is running
    let _lock_file = acquire_instance_lock()?;
    tracing::info!("MintyBot instance lock acquired - this is the only running instance");

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
