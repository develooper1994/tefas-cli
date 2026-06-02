use tefas::TefasClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = TefasClient::from_defaults()?;

    let merged = client
        .query_names(
            vec![
                "fonBilgiGetir".to_string(),
                "fonGetiriBazliBilgiGetir".to_string(),
            ],
            Some(4),
            vec![],
            None,
        )
        .await?;

    println!(
        "query keys: {}",
        merged.as_object().map(|m| m.len()).unwrap_or(0)
    );
    Ok(())
}
