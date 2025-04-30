use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::from_str;
use std::collections::HashMap;
use std::fs::File;
use std::fs::{self};
use std::io::Write;
use std::io::{self, BufRead};

#[allow(dead_code)]
#[derive(Deserialize)]
struct Listing {
    id: i32,
    listing_name: String,
    item_name: String,
    variant: Option<String>,
    buy_price: Option<f32>,
    buy_price_total: Option<f32>,
    sell_price: Option<f32>,
    sell_price_total: Option<f32>,
    lot_size: i32,
    is_dynamic: bool,
    item_stock: Option<i32>,
    currency_stock: Option<f32>,
    created_at: String,
    updated_at: String,
    enchantments: Vec<Enchantment>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Enchantment {
    enchantment_name: String,
    level: Option<i32>,
    forbidden: bool,
}

#[derive(Deserialize)]
struct ApiResponse {
    listings: Vec<Listing>,
}

struct Item {
    name: String,
    lot_size: i32,
    buy_price: f32,
    mid_price: f32,
    sell_price: f32,
    item_stock: i32,
    currency_stock: f32,
    updated_at: DateTime<Utc>,
}

pub fn fetch() {
    let api_url = "https://webstore.bulbastore.uk/api/listings";

    let mut items = Vec::new();

    match ureq::get(api_url).call() {
        Ok(mut response) => {
            if response.status() == 200 {
                let response_text = match response.body_mut().read_to_string() {
                    Ok(text) => text,
                    Err(err) => {
                        eprintln!("error reading response body: {}", err);
                        return;
                    }
                };

                let api_response: ApiResponse = match from_str(&response_text) {
                    Ok(data) => data,
                    Err(err) => {
                        eprintln!("error parsing JSON: {}", err);
                        return;
                    }
                };

                for item in api_response.listings {
                    if item.is_dynamic {
                        items.push(Item {
                            name: item.listing_name,
                            lot_size: item.lot_size,
                            buy_price: item.buy_price_total.unwrap_or(0.0),
                            mid_price: item.currency_stock.unwrap_or(0.0)
                                / item.item_stock.unwrap_or(0) as f32
                                * item.lot_size as f32,
                            sell_price: item.sell_price_total.unwrap_or(0.0),
                            item_stock: item.item_stock.unwrap_or(0),
                            currency_stock: item.currency_stock.unwrap_or(0.0),
                            updated_at: DateTime::parse_from_rfc3339(&item.updated_at)
                                .expect("failed to parse datetime string")
                                .with_timezone(&Utc),
                        });
                    }
                }
            } else {
                eprintln!("error reading response: {}", response.status());
            }
        }
        Err(err) => {
            eprintln!("error making request: {}", err);
        }
    }

    items.sort_by(|a, b| a.updated_at.cmp(&b.updated_at));

    fs::create_dir_all("data").unwrap();

    let mut file = File::create(format!(
        "data/{}.csv",
        items.last().unwrap().updated_at.format("%Y-%m-%d-%H-%M-%S")
    ))
    .unwrap();

    for item in items.iter() {
        writeln!(
            file,
            "{},{},{},{},{},{},{},{}",
            item.lot_size,
            item.name,
            item.buy_price,
            item.mid_price,
            item.sell_price,
            item.item_stock,
            item.currency_stock,
            item.updated_at.format("%Y-%m-%d %H:%M:%S")
        )
        .unwrap();
    }

    let dir_path = "data";

    // get all CSV files in the data directory
    let mut csv_files: Vec<_> = fs::read_dir(dir_path)
        .unwrap()
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                let path = e.path();
                if path.extension().unwrap() == "csv" {
                    Some(path)
                } else {
                    None
                }
            })
        })
        .collect();

    // sort the files based on their timestamps in descending order
    csv_files.sort_by_key(|path| path.file_name().unwrap().to_str().unwrap().to_string());
    csv_files.reverse();

    // exclude the latest file (created during the current run)
    if csv_files.is_empty() {
        panic!(
            "{:?}",
            io::Error::new(io::ErrorKind::NotFound, "no CSV files found",)
        );
    }

    let mut prev_file = &csv_files[0];
    let latest_file = &csv_files[0];

    if csv_files.len() > 1 {
        prev_file = &csv_files[1];
    }

    let path = prev_file;

    // open the file in read-only mode (ignoring errors)
    let input = File::open(path).unwrap();
    let buffered = io::BufReader::new(input);

    // create a HashMap to store the result
    let mut prev: HashMap<String, (i32, f32)> = HashMap::new();

    // iterate through each line in the file
    for line in buffered.lines().map_while(Result::ok) {
        let parts: Vec<&str> = line.split(',').collect();

        // ensure we have enough parts to process
        if parts.len() >= 8 {
            // get the key from the second value
            let key = parts[1].to_string();

            // get the last three values
            let value1 = parts[5].to_string().parse::<i32>().unwrap();
            let value2 = parts[6].to_string().parse::<f32>().unwrap();

            // insert into the HashMap
            prev.insert(key, (value1, value2));
        }
    }

    let path = latest_file;

    // open the file in read-only mode (ignoring errors)
    let input = File::open(path).unwrap();
    let buffered = io::BufReader::new(input);

    // create a HashMap to store the result
    let mut latest: HashMap<String, (i32, f32)> = HashMap::new();

    // iterate through each line in the file
    for line in buffered.lines().map_while(Result::ok) {
        let parts: Vec<&str> = line.split(',').collect();

        // ensure we have enough parts to process
        if parts.len() >= 8 {
            // get the key from the second value
            let key = parts[1].to_string();

            // get the last three values
            let value1 = parts[5].to_string().parse::<i32>().unwrap();
            let value2 = parts[6].to_string().parse::<f32>().unwrap();

            // insert into the HashMap
            latest.insert(key, (value1, value2));
        }
    }

    for item in items.iter() {
        let item_stock_diff = item.item_stock - prev.get(&item.name).unwrap_or(&(0, 0.0)).0;
        let currency_stock_diff = item.currency_stock - prev.get(&item.name).unwrap_or(&(0, 0.0)).1;

        println!(
            "{} | {:8.2} {:8.2} {:8.2} | {:8} {:8.2} {} {} | {:2} {}",
            item.updated_at.format("%Y-%m-%d %H:%M:%S"),
            item.buy_price,
            item.mid_price,
            item.sell_price,
            item.item_stock,
            item.currency_stock,
            if item_stock_diff != 0 {
                format!("{:8}", item_stock_diff)
            } else {
                "        ".to_string()
            },
            if currency_stock_diff != 0.0 {
                format!("{:8.2}", currency_stock_diff)
            } else {
                "        ".to_string()
            },
            item.lot_size,
            item.name,
        );
    }
}
