mod models;

use models::request;


#[tokio::main]
async fn main() {
    println!("Hello, world!");
    let mut dox_acc = request::DoxbinAccount::new();
    let response = dox_acc.get(&"https://doxbin.org/").await;
    let response = dox_acc.get(&"https://doxbin.org/.well-known/ddos-guard/check?context=free_splash").await;
}
