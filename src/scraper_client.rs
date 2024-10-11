use crate::errors::ScraperError;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use reqwest::{Client, IntoUrl};
use std::time::Duration;
use std::time::Instant;
use tokio::time::sleep;

pub struct ScraperClient {
    client: Client,
    request_id: u64,
    stats: ScraperClientStats,
    max_retries: u8,
    retry_delay: Duration,
}

// Stats struct for tracking usage (optional)
#[derive(Default)]
struct ScraperClientStats {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
}

impl ScraperClient {
    /// Create a new scraper client with default timeout and retry configuration
    pub fn new_http() -> Self {
        Self::new_with_config(Duration::from_secs(30), 3, Duration::from_secs(2))
    }

    /// Create a new scraper client with a custom timeout, retries, and delay between retries
    fn new_with_config(timeout: Duration, max_retries: u8, retry_delay: Duration) -> Self {
        let client = Client::builder()
            .default_headers(Self::default_headers())
            .timeout(timeout)
            .pool_idle_timeout(timeout)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            request_id: 0,
            stats: ScraperClientStats::default(),
            max_retries,
            retry_delay,
        }
    }

    /// Default headers for the client
    fn default_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Rust ScraperClient/1.0"),
        );
        headers
    }

    /// Asynchronously fetch the content of the web page with retry logic
    pub async fn fetch_url<U: Copy + IntoUrl>(&mut self, url: U) -> Result<String, ScraperError> {
        self.request_id += 1;
        println!("Fetching page with request ID: {}", self.request_id);

        let mut attempts = 0;
        let start_time = Instant::now();

        // Retry loop
        while attempts <= self.max_retries {
            attempts += 1;
            match self.client.get(url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        let body = response.text().await?;
                        self.record_success();
                        println!(
                            "Successfully fetched on attempt {} after {:?}",
                            attempts,
                            start_time.elapsed()
                        );
                        return Ok(body);
                    } else {
                        eprintln!(
                            "Attempt {}: Request failed with status: {}",
                            attempts,
                            response.status()
                        );
                    }
                }
                Err(e) => {
                    eprintln!("Attempt {}: Request error: {}", attempts, e);
                }
            }

            if attempts <= self.max_retries {
                println!("Retrying in {:?}...", self.retry_delay);
                sleep(self.retry_delay).await;
            }
        }

        self.record_failure();

        Err(ScraperError::CustomError(format!(
            "Failed to fetch page after {} attempts in {:?}",
            attempts,
            start_time.elapsed()
        )))
    }

    /// Track a successful request in the stats
    fn record_success(&mut self) {
        self.stats.total_requests += 1;
        self.stats.successful_requests += 1;
    }

    /// Track a failed request in the stats
    fn record_failure(&mut self) {
        self.stats.total_requests += 1;
        self.stats.failed_requests += 1;
    }

    /// Print the current statistics (total requests, successes, failures)
    pub fn print_stats(&self) {
        println!(
            "Total Requests: {}, Successful: {}, Failed: {}",
            self.stats.total_requests, self.stats.successful_requests, self.stats.failed_requests
        );
    }
}
