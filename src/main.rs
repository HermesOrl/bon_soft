mod models;
use serde::de::StdError;
use models::request;


#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    println!("Hello, world!");
    let mut dox_acc = request::DoxbinAccount::new();
    let mut captcha = request::Captcha::new();
    // match dox_acc.generate_xsrf_token().await {
    //     Some(_) => println!("Create xsrf"),
    //     None => eprintln!("Not created")
    // }
    match dox_acc.create_account().await {
        Some(_) => println!("Create acc"),
        None => eprintln!("Not create acc")
    }
    // captcha.get_token().await;
    Ok(())
}

