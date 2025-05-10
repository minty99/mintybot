use dotenv::dotenv;
use mintybot::statics::DISCORD_TOKEN;
use mintybot::utils::discord;
use serenity::async_trait;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _: Ready) {
        // Hardcoded message to send to the developer
        let message = "<todo>";

        // Send message to developer using the existing function
        if let Err(e) = discord::send_dm_to_dev(&ctx, message).await {
            println!("Failed to send message: {e}");
        } else {
            println!("Message sent successfully");
        }

        // Exit the program after sending the message
        std::process::exit(0);
    }
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    // Create Discord client with minimal intents
    let intents = GatewayIntents::empty();
    let mut client = Client::builder(&**DISCORD_TOKEN, intents)
        .event_handler(Handler)
        .await
        .map_err(|e| eyre::eyre!("Failed to create Discord client: {}", e))?;

    // Start the client
    if let Err(e) = client.start().await {
        println!("Client error: {e}");
        return Err(eyre::eyre!("Client error: {}", e));
    }

    Ok(())
}
