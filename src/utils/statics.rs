use std::{env, sync::Arc};

use lazy_static::lazy_static;
use serenity::model::id::UserId;

lazy_static! {
    pub static ref DEV_USER_ID: Arc<UserId> = Arc::new(
        env::var("MINTYBOT_DEV_USER_ID")
            .expect("MINTYBOT_DEV_USER_ID environment variable must be set")
            .trim_end()
            .parse::<u64>()
            .expect("Dev user id should be a number")
            .into()
    );
    pub static ref DISCORD_TOKEN: Arc<String> = Arc::new(
        env::var("MINTYBOT_DISCORD_TOKEN")
            .expect("MINTYBOT_DISCORD_TOKEN environment variable must be set")
            .trim_end()
            .to_string()
    );
    pub static ref OPENAI_TOKEN: Arc<String> = Arc::new(
        env::var("MINTYBOT_OPENAI_TOKEN")
            .expect("MINTYBOT_OPENAI_TOKEN environment variable must be set")
            .trim_end()
            .to_string()
    );
}
