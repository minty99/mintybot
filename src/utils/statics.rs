use std::{
    env,
    sync::{Arc, OnceLock},
};

use lazy_static::lazy_static;
use serenity::model::id::UserId;

// Whether the bot is running in dev mode
pub static IS_DEV_MODE: OnceLock<bool> = OnceLock::new();

// Initialize the dev mode flag
pub fn is_dev_mode() -> bool {
    *IS_DEV_MODE.get_or_init(|| std::env::args().any(|arg| arg == "--dev"))
}

fn get_discord_token_env_name() -> String {
    if is_dev_mode() {
        "MINTYBOT_DISCORD_TOKEN_DEV".to_string()
    } else {
        "MINTYBOT_DISCORD_TOKEN".to_string()
    }
}

lazy_static! {
    pub static ref DEV_USER_ID: Arc<UserId> = Arc::new(
        env::var("MINTYBOT_DEV_USER_ID")
            .expect("MINTYBOT_DEV_USER_ID environment variable must be set")
            .trim_end()
            .parse::<u64>()
            .expect("Dev user id should be a number")
            .into()
    );
    pub static ref DISCORD_TOKEN: Arc<String> = Arc::new({
        env::var(get_discord_token_env_name())
            .unwrap_or_else(|_| {
                panic!(
                    "{} environment variable must be set",
                    get_discord_token_env_name()
                )
            })
            .trim_end()
            .to_string()
    });
    pub static ref OPENAI_TOKEN: Arc<String> = Arc::new({
        env::var("MINTYBOT_OPENAI_TOKEN")
            .expect("MINTYBOT_OPENAI_TOKEN environment variable must be set")
            .trim_end()
            .to_string()
    });
}
