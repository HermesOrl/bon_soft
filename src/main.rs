mod models;

use models::request;


#[tokio::main]
async fn main() {
    println!("Hello, world!");
    let mut dox_acc = request::DoxbinAccount::new();
    let response = dox_acc.get(&"").await;
    println!("Response: {:?}", response);
}
