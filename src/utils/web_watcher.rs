use std::fmt::Debug;
use std::sync::Arc;

use serenity::client::Context;

use super::{discord::send_dm_to_dev, web::get_html_of_given_url};

#[allow(dead_code)]
pub async fn watch_web_site<T>(
    ctx: Arc<Context>,
    url: &str,
    parser: impl Fn(String) -> T,
    prev: Option<T>,
) -> eyre::Result<T>
where
    T: PartialEq + Debug,
{
    if prev.is_none() {
        tracing::info!("Starting to watch: {}", url);
    }
    let html = get_html_of_given_url(url).await?;

    let curr = parser(html);

    if let Some(prev) = prev
        && prev != curr
    {
        tracing::trace!("Changes detected: {:?}", curr);
        let mut msg = String::new();
        msg.push_str(&format!("Changes detected: {curr:?}\n"));
        send_dm_to_dev(&ctx, &msg).await?;
    }

    Ok(curr)
}
