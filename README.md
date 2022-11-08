# Nordpool
A Rust library for fetching Nord Pool spot prices. Only Swedish regions & SEK currencies supported for now.

## Example
```rust
let prices = nordpool::get_prices("SE3", Stockholm.ymd(2022, 11, 9))
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
```