use tokio::task;
mod models;
use serde::de::StdError;
use models::request;
use reqwest::Client;
use std::sync::{Arc, Mutex};
use std::fs::OpenOptions;
use std::io::Write;
use tokio::task::spawn_blocking;

#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    let num_tasks = 2;
    let client = Arc::new(Client::builder()
        .pool_max_idle_per_host(10)
        .build()
        .expect("Failed to build client"));

    let file = Arc::new(Mutex::new(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open("results.txt")
            .expect("Unable to open file"),
    ));

    let mut handles = vec![];

    for thread_id in 0..num_tasks {
        let file_clone = Arc::clone(&file);
        let client_clone = Arc::clone(&client);
        let handle = task::spawn(async move {
            let mut dox_acc = request::DoxbinAccount::new(client_clone);
            match dox_acc.create_account().await {
                Some((username, password)) => {
                    println!("[{}] Create acc\t{}:{}", thread_id, username, password);
                    let file_clone = Arc::clone(&file_clone);
                    spawn_blocking(move || {
                        let mut file = file_clone.lock().unwrap();
                        writeln!(file, "{}:{}", username, password).expect("Unable to write to file");
                    }).await.expect("Failed to write to file");
                }
                None => eprintln!("[{}] Not create acc", thread_id),
            }
        });
        handles.push(handle);
    }


    for handle in handles {
        handle.await?;
    }
    // let mut cc = request::Captcha::new();
    // if let Some(asdasd) =  cc.get_token().await {
    //     println!("{}", asdasd);
    // }
    Ok(())
}

