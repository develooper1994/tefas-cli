use tefas::{FetchBatchRequest, TefasClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = TefasClient::from_defaults()?;

    let merged = client
        .query_names(
            vec![
                "fonBilgiGetir".to_string(),
                "fonGetiriBazliBilgiGetir".to_string(),
            ],
            Some(8),
            vec![],
            None,
        )
        .await?;

    println!(
        "query keys: {}",
        merged.as_object().map(|m| m.len()).unwrap_or(0)
    );

    let fetched = client
        .fetch_urls(
            FetchBatchRequest::new(
                vec![
                    "https://www.tefas.gov.tr".to_string(),
                    "https://www.takasbank.com.tr".to_string(),
                ],
                Some(8),
                8,
            ),
            true,
        )
        .await;

    println!("fetched urls: {}", fetched.len());

    Ok(())
}
