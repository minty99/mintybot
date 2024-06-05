mod kma;
mod maple;
mod utils;

use std::{env, fs};

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use utils::discord;

struct MintyBotHandler {
    #[allow(dead_code)]
    discord_token: String,
    kma_service_key: String,
}

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

        if content == "!weather" {
            let weather_info = kma::get_weather(&self.kma_service_key).await;
            match weather_info {
                Ok(info) => {
                    discord::say(&ctx, channel_id, info).await;
                }
                Err(why) => {
                    discord::say(&ctx, channel_id, format!("Internal error occured: {}", why))
                        .await;
                }
            }
        } else if content.starts_with("!m") || content.starts_with("!maple ") {
            let args = content.split(' ').collect::<Vec<&str>>();
            if args.len() != 2 {
                discord::say(
                    &ctx,
                    channel_id,
                    "[!maple 캐릭터이름] 또는 [!m 캐릭터이름] 으로 명령해주세요.",
                )
                .await;
                return;
            }
            let character_name = args[1];
            let maple_user = maple::get_maple_user(character_name).await;
            match maple_user {
                Ok(maple_user) => {
                    discord::say(&ctx, channel_id, maple_user).await;
                }
                Err(why) => {
                    discord::say(&ctx, channel_id, format!("Internal error occured: {}", why)).await
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
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    // Configure the client with your Discord bot token in the token file.
    let discord_token = fs::read_to_string(".discord_token")
        .or_else(|_| env::var("MINTYBOT_DISCORD_TOKEN"))
        .expect("Discord token should be stored at .discord_token or DISCORD_TOKEN env variable")
        .trim_end()
        .to_string();

    let kma_service_key = fs::read_to_string(".kma_api_key")
        .or_else(|_| env::var("MINTYBOT_KMA_SERVICE_KEY"))
        .expect("KMA service key should be stored at .kma_api_key or KMA_SERVICE_KEY env variable")
        .trim_end()
        .to_string();

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let mut client = Client::builder(&discord_token, intents)
        .event_handler(MintyBotHandler {
            discord_token,
            kma_service_key,
        })
        .await
        .expect("Err creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
