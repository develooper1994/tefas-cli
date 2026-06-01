use tefas::{FetchBatchRequest, TefasClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = TefasClient::from_defaults()?;

    let results = client
        .fetch_urls(
            FetchBatchRequest::new(
                vec![
                    "https://www.tefas.gov.tr".to_string(),
                    "https://www.takasbank.com.tr".to_string(),
                ],
                Some(4),
                4,
            ),
            true,
        )
        .await;

    println!("fetched urls: {}", results.len());
    Ok(())
}
