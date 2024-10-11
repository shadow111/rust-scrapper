use thiserror::Error as ThisError;
#[derive(ThisError, Debug)]
pub enum ScraperError {
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
    #[error("Selector error: {0}")]
    SelectorError(String),
    #[error("Fetch error: {0}")]
    FetchError(#[from] reqwest::Error),
    #[error("SqliteConnectionError: {0}")]
    SqliteConnectionError(#[from] rusqlite::Error),
    #[error("Error: {0}")]
    CustomError(String),
}
