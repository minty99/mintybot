pub async fn get_html_of_given_url(url: &str) -> eyre::Result<String> {
    let response = reqwest::get(url).await?;
    let html = response.text().await?;
    Ok(html)
}
