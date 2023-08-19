use scraper::Html;
use std::time::{Duration, Instant};

use crate::utils::maple_types::MapleUser;

pub async fn get_maple_user(name: &str) -> eyre::Result<MapleUser> {
    let url = String::from("https://maple.gg/u/") + &name;
    let client = reqwest::Client::new();
    let before = Instant::now();
    let response = client
        .get(url)
        .timeout(Duration::from_secs(3))
        .send()
        .await?;
    let after = Instant::now();

    println!(
        "GET {} ({}) [{} ms]",
        response.url(),
        response.status(),
        (after - before).as_millis()
    );

    let document = Html::parse_document(&response.text().await?);

    Ok(MapleUser::from(document))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_maple_user() {
        let name = String::from("숍하이퍼");
        let result = get_maple_user(&name).await;
        println!("{:?}", result);
    }
}
