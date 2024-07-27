mod models;

use std::fs;
use std::path::Path;
use serde::de::StdError;
use serde_json::Value;
use models::request;


#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    println!("Hello, world!");
    let mut dox_acc = request::DoxbinAccount::new();
    match dox_acc.generate_xsrf_token().await {
        Some(_) => println!("Create xsrf"),
        None => eprintln!("Not created")
    }
    Ok(())
}

