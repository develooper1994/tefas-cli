pub mod fund_page;

pub use fund_page::{FundPageMeta, FundPageOutput, parse_document_typed};

use serde_json::Value;

/// Trait for common TEFAS page operations.
pub trait TefasPage {
    type Output;

    /// The URL pattern for this page.
    fn url(base_url: &str, code: &str) -> String;

    /// Validate if the HTML content is a valid response (not WAF blocked or error page).
    fn validate(html: &str) -> bool;

    /// Parse the HTML content into the designated output type.
    fn parse(html: &str) -> Self::Output;
}

pub struct FundPage;

impl TefasPage for FundPage {
    type Output = (Value, Value);

    fn url(base_url: &str, code: &str) -> String {
        format!("{}/tr/fon-analiz-sayfasi?fonkod={}", base_url, code)
    }

    fn validate(html: &str) -> bool {
        !fund_page::contains_failureconfig(html)
    }

    fn parse(html: &str) -> Self::Output {
        fund_page::parse_document(html)
    }
}
