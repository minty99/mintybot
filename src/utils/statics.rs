use std::{env, fs, sync::Arc};

use lazy_static::lazy_static;
use serenity::model::id::UserId;

lazy_static! {
    pub static ref DEV_USER_ID: Arc<UserId> = Arc::new(
        fs::read_to_string(".dev_user_id")
            .or_else(|_| env::var("MINTYBOT_DEV_USER_ID"))
            .expect("Dev user id should be stored at .dev_user_id or DEV_USER_ID env variable")
            .trim_end()
            .parse::<u64>()
            .expect("Dev user id should be a number")
            .into()
    );

    pub static ref DISCORD_TOKEN: Arc<String> = Arc::new(
        fs::read_to_string(".discord_token")
            .or_else(|_| env::var("MINTYBOT_DISCORD_TOKEN"))
            .expect(
                "Discord token should be stored at .discord_token or DISCORD_TOKEN env variable"
            )
            .trim_end()
            .to_string()
    );
}
