use chrono::{Date, DateTime, Datelike, NaiveDateTime, Timelike, Weekday};
use chrono_tz::{Europe::Stockholm, Tz};

use rust_decimal_macros::dec;
use rusty_money::iso::SEK;
use serde::{Deserialize, Deserializer};

type Money = rusty_money::Money<'static, rusty_money::iso::Currency>;

const NORDPOOL_URL_HOUR: &str = "https://www.nordpoolgroup.com/api/marketdata/page/10";

fn deserialize_partial_iso8601<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    NaiveDateTime::parse_from_str(&String::deserialize(deserializer)?, "%Y-%m-%dT%H:%M:%S")
        .map_err(serde::de::Error::custom)
}

#[derive(Deserialize, Debug)]
struct Response {
    data: Data,
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct Data {
    rows: Vec<Row>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct Row {
    #[serde(deserialize_with = "deserialize_partial_iso8601")]
    start_time: NaiveDateTime,
    columns: Vec<Column>,
    is_extra_row: bool,
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct Column {
    name: String,
    value: String,
}

pub async fn get_prices(area: &str, end_date: Date<Tz>) -> Result<Vec<TotalPrice>, reqwest::Error> {
    let url = format!(
        "{NORDPOOL_URL_HOUR}?currency=SEK&endDate={}",
        end_date.format("%d-%m-%Y")
    );
    let response = reqwest::get(url).await?.json::<Response>().await?;

    Ok(response
        .data
        .rows
        .iter()
        .filter(|r| !r.is_extra_row)
        .flat_map(|r| {
            r.columns
                .iter()
                .find(|c| c.name == area)
                .and_then(|c| Money::from_str(&c.value, rusty_money::iso::SEK).ok())
                .and_then(|price: Money| {
                    r.start_time
                        .and_local_timezone(Stockholm)
                        .single()
                        .map(|local_start_time| TotalPrice::compute(local_start_time, price / 1000))
                })
        })
        .collect())
}

pub struct TotalPrice {
    start_time: DateTime<Tz>,
    energy: Money,
    vat: Money,
    fee: Money,
    tax: Money,
}

impl TotalPrice {
    pub fn compute(start_time: DateTime<Tz>, energy: Money) -> Self {
        Self {
            start_time,
            energy: energy.clone(),
            vat: energy * dec!(0.25),
            fee: match (start_time.weekday(), start_time.time().hour()) {
                (_, 0..=5) | (_, 22..) | (Weekday::Sat, _) | (Weekday::Sun, _) => {
                    Money::from_minor(12, SEK)
                }
                (_, _) => Money::from_minor(70, SEK),
            },
            tax: Money::from_minor(45, SEK),
        }
    }
    pub fn sum(&self) -> Money {
        self.energy.to_owned() + self.fee.to_owned() + self.tax.to_owned() + self.vat.to_owned()
    }
    pub fn start_time(&self) -> DateTime<Tz> {
        self.start_time
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use chrono_tz::Europe::Stockholm;

    #[tokio::test]
    async fn get_prices() {
        let prices = super::get_prices("SE3", Stockholm.ymd(2022, 11, 9))
            .await
            .expect("Error fetching prices.");

        println!(
            "{:26}{:8}{:8}{:8}{:8}{:8}",
            "Hour", "Energy", "VAT", "Fee", "Tax", "Total"
        );
        for price in prices.iter() {
            println!(
                "{:24}{:>8}{:>8}{:>8}{:>8}{:>8}",
                price.start_time.to_string(),
                price.energy.to_string(),
                price.vat.to_string(),
                price.fee.to_string(),
                price.tax.to_string(),
                price.sum().to_string()
            );
        }
        assert_eq!(prices.len(), 24);
    }
}
