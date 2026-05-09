use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FundIdentity {
    pub code: String,
    pub isin: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FundProfile {
    pub identity: FundIdentity,
    pub name: Option<String>,
    pub category: Option<String>,
    pub risk_value: Option<u8>,
    pub total_value_tl: Option<f64>,
    pub market_share_pct: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FundReturns {
    pub daily_pct: Option<f64>,
    pub one_month_pct: Option<f64>,
    pub three_month_pct: Option<f64>,
    pub six_month_pct: Option<f64>,
    pub one_year_pct: Option<f64>,
    pub three_year_pct: Option<f64>,
    pub five_year_pct: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryOperationName(pub String);

impl QueryOperationName {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    Parse(String),
    Network(String),
    Protocol(String),
    Validation(String),
    RateLimit(String),
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "parse error: {msg}"),
            Self::Network(msg) => write!(f, "network error: {msg}"),
            Self::Protocol(msg) => write!(f, "protocol error: {msg}"),
            Self::Validation(msg) => write!(f, "validation error: {msg}"),
            Self::RateLimit(msg) => write!(f, "rate limit error: {msg}"),
        }
    }
}

impl std::error::Error for DomainError {}

#[cfg(test)]
mod tests;
