use crate::errors::ScraperError;
use log::{info, warn};
use regex::Regex;
use rusqlite::{params, Connection};
use scraper::{Html, Selector};

#[derive(Debug)]
struct Holiday {
    year: String,
    date: String,
    name: String,
}

pub struct HolidayProcessor {
    raw_html: String,
    holidays: Vec<Holiday>,
}

impl HolidayProcessor {
    pub fn new(html: String) -> Self {
        Self {
            raw_html: html,
            holidays: vec![],
        }
    }

    pub async fn run(&mut self) -> Result<(), ScraperError> {
        let document: Html = Html::parse_document(&self.raw_html);

        let year_selector = Selector::parse("thead th")
            .map_err(|err| ScraperError::SelectorError(err.to_string()))?;
        let row_selector = Selector::parse("tbody tr")
            .map_err(|err| ScraperError::SelectorError(err.to_string()))?;
        let name_selector = Selector::parse("th strong")
            .map_err(|err| ScraperError::SelectorError(err.to_string()))?;
        let date_selector =
            Selector::parse("td").map_err(|err| ScraperError::SelectorError(err.to_string()))?;

        let mut year_iter = document.select(&year_selector).skip(1); // Skip the empty first column for names
        let mut years = Vec::new();
        let re = Regex::new(r"\s+")?;

        // Extract all years from the <thead>
        while let Some(year_element) = year_iter.next() {
            let year_text = year_element.inner_html().trim().to_string();
            years.push(year_text);
        }

        let mut row_iter = document.select(&row_selector);

        // Iterate over the rows in the <tbody>
        while let Some(row) = row_iter.next() {
            if let Some(name_element) = row.select(&name_selector).next() {
                let holiday_name = name_element
                    .clone()
                    .inner_html()
                    .replace("<br>", " ")
                    .replace("&amp;", "&")
                    .replace("&nbsp;", " ");

                let mut date_iter = row.select(&date_selector);
                let mut year_iter = years.iter();

                while let (Some(date_element), Some(year)) = (date_iter.next(), year_iter.next()) {
                    let holiday_date = date_element
                        .clone()
                        .inner_html()
                        .replace("<br>", " ")
                        .replace("&amp;", "&")
                        .replace("&nbsp;", " ");

                    self.holidays.push(Holiday {
                        year: year.clone(),
                        date: re.replace_all(&holiday_date, " ").trim().to_string(),
                        name: holiday_name.trim().to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    pub fn pretty_print(&self) {
        if self.holidays.is_empty() {
            warn!("No holidays available in local data.");
            return;
        }
        info!("--- Holidays from Local Data ---");
        for holiday in &self.holidays {
            info!(
                "Year: {}, Holiday: {}, Date: {}",
                holiday.year, holiday.name, holiday.date
            );
        }
        info!("--- End of Local Data ---\n");
    }

    pub async fn save_to_db(&self, conn: &Connection) -> Result<(), ScraperError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS holidays (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                date TEXT NOT NULL,
                year TEXT NOT NULL
            )",
            [],
        )?;

        for holiday in &self.holidays {
            conn.execute(
                "INSERT INTO holidays (name, date, year) VALUES (?1, ?2, ?3)",
                params![holiday.name, holiday.date, holiday.year],
            )?;
        }
        Ok(())
    }

    pub async fn fetch_from_db(&self, conn: &Connection) -> Result<(), ScraperError> {
        let mut stmt = conn.prepare("SELECT name, date, year FROM holidays")?;
        let holiday_iter = stmt.query_map([], |row| {
            Ok(Holiday {
                name: row.get(0)?,
                date: row.get(1)?,
                year: row.get(2)?,
            })
        })?;

        info!("--- Holidays from Database ---");
        for holiday in holiday_iter {
            let holiday = holiday?;
            info!(
                "Holiday: {}, Date: {}, Year: {}",
                holiday.name, holiday.date, holiday.year
            );
        }
        info!("--- End of Database Data ---\n");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_holiday_processor_valid_html() {
        let html = r#"
            <table>
                <thead>
                    <tr><th>Holiday</th><th>2023</th><th>2024</th></tr>
                </thead>
                <tbody>
                    <tr>
                        <th><strong>New Year's Day</strong></th>
                        <td>January 1</td><td>January 1</td>
                    </tr>
                    <tr>
                        <th><strong>Christmas Day</strong></th>
                        <td>December 25</td><td>December 25</td>
                    </tr>
                </tbody>
            </table>
        "#
        .to_string();

        let mut processor = HolidayProcessor::new(html);
        processor.run().await.expect("Processor failed");

        assert_eq!(processor.holidays.len(), 4);

        assert_eq!(processor.holidays[0].year, "2023");
        assert_eq!(processor.holidays[0].name, "New Year's Day");
        assert_eq!(processor.holidays[0].date, "January 1");

        assert_eq!(processor.holidays[1].year, "2024");
        assert_eq!(processor.holidays[1].name, "New Year's Day");
        assert_eq!(processor.holidays[1].date, "January 1");

        assert_eq!(processor.holidays[2].year, "2023");
        assert_eq!(processor.holidays[2].name, "Christmas Day");
        assert_eq!(processor.holidays[2].date, "December 25");

        assert_eq!(processor.holidays[3].year, "2024");
        assert_eq!(processor.holidays[3].name, "Christmas Day");
        assert_eq!(processor.holidays[3].date, "December 25");
    }

    #[tokio::test]
    async fn test_holiday_processor_empty_html() {
        let html = "".to_string();
        let mut processor = HolidayProcessor::new(html);

        let result = processor.run().await;
        assert!(result.is_ok());
        assert_eq!(
            processor.holidays.len(),
            0,
            "No holidays should be parsed from empty HTML"
        );
    }

    #[tokio::test]
    async fn test_holiday_processor_invalid_html() {
        let html =
            r#"<html><table><thead><tr><th></th></tr></thead><tbody></tbody></table></html>"#
                .to_string();
        let mut processor = HolidayProcessor::new(html);

        let result = processor.run().await;
        assert!(result.is_ok());
        assert_eq!(
            processor.holidays.len(),
            0,
            "No holidays should be parsed from invalid HTML structure"
        );
    }

    #[tokio::test]
    async fn test_holiday_processor_special_characters() {
        let html = r#"
            <table>
                <thead>
                    <tr><th>Holiday</th><th>2023</th></tr>
                </thead>
                <tbody>
                    <tr>
                        <th><strong>Labor &amp; Workers' Day</strong></th>
                        <td>May 1&nbsp;&nbsp;</td>
                    </tr>
                    <tr>
                        <th><strong>Independence<br>Day</strong></th>
                        <td>July 4</td>
                    </tr>
                </tbody>
            </table>
        "#
        .to_string();

        let mut processor = HolidayProcessor::new(html);
        processor.run().await.expect("Processor failed");

        assert_eq!(processor.holidays.len(), 2);

        assert_eq!(processor.holidays[0].year, "2023");
        assert_eq!(processor.holidays[0].name, "Labor & Workers' Day");
        assert_eq!(processor.holidays[0].date, "May 1");

        assert_eq!(processor.holidays[1].year, "2023");
        assert_eq!(processor.holidays[1].name, "Independence Day");
        assert_eq!(processor.holidays[1].date, "July 4");
    }

    #[tokio::test]
    async fn test_holiday_processor_missing_dates() {
        let html = r#"
            <table>
                <thead>
                    <tr><th>Holiday</th><th>2023</th></tr>
                </thead>
                <tbody>
                    <tr>
                        <th><strong>Holiday with No Date</strong></th>
                        <td></td>
                    </tr>
                </tbody>
            </table>
        "#
        .to_string();

        let mut processor = HolidayProcessor::new(html);
        processor.run().await.expect("Processor failed");

        assert_eq!(processor.holidays.len(), 1);

        assert_eq!(processor.holidays[0].year, "2023");
        assert_eq!(processor.holidays[0].name, "Holiday with No Date");
        assert_eq!(processor.holidays[0].date, "");
    }
}
