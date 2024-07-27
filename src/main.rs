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
    let response = dox_acc.get(&"https://doxbin.org/").await;
    println!("Status code: {:?}", response);
    let response = dox_acc.get(&"https://doxbin.org/.well-known/ddos-guard/check?context=free_splash").await;
    println!("Status code: {:?}", response);
    let response = dox_acc.get(&"https://doxbin.org/.well-known/ddos-guard/id/T5q8bswinyHijR3O").await;
    println!("Status code: {:?}", response);
    let response = dox_acc.get(&"https://check.ddos-guard.net/set/id/T5q8bswinyHijR3O").await;
    println!("Status code: {:?}", response);
    let json_payload = read_json_from_file("payload.json")?;
    match dox_acc.post("https://doxbin.org/.well-known/ddos-guard/mark/", &json_payload).await {
        Ok(status) => println!("POST request status code: {}", status),
        Err(e) => eprintln!("POST request error: {}", e),
    }
    let response = dox_acc.get(&"https://doxbin.org/").await;
    println!("Status code: {:?}", response);
    // let response = dox_acc.get(&"https://doxbin.org/").await;
    // println!("Status code: {:?}", response);
    Ok(())
}


fn read_json_from_file<P: AsRef<Path>>(path: P) -> Result<Value, Box<dyn StdError>> {
    let data = fs::read_to_string(path)?;
    let json: Value = serde_json::from_str(&data)?;
    Ok(json)
}