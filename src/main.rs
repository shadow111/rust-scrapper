use rusqlite::Connection;
use rust_assignment::errors::ScraperError;
use rust_assignment::holiday_processor::HolidayProcessor;
use rust_assignment::scraper_client::ScraperClient;

#[tokio::main]
async fn main() -> Result<(), ScraperError> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let mut scraper_client = ScraperClient::new_http();
    let conn = Connection::open_in_memory()?;

    let raw_html = scraper_client
        .fetch_url(
            "https://www.commerce.wa.gov.au/labour-relations/public-holidays-western-australia",
        )
        .await?;
    scraper_client.print_stats();

    let mut processor = HolidayProcessor::new(raw_html);
    processor.run().await?;
    processor.pretty_print();

    processor.save_to_db(&conn).await?;
    processor.fetch_from_db(&conn).await?;

    Ok(())
}
