use tokio::task;
mod models;
use serde::de::StdError;
use models::request;

use std::sync::{Arc, Mutex};
use std::fs::OpenOptions;
use std::io::Write;

#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    let num_tasks = 30;

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
        let handle = task::spawn(async move {
            let mut dox_acc = request::DoxbinAccount::new();
            match dox_acc.create_account().await {
                Some((username, password)) => {
                    println!("[{}] Create acc\t{}:{}", thread_id, username, password);
                    let mut file = file_clone.lock().unwrap();
                    writeln!(file, "{}:{}", username, password).expect("Unable to write to file");
                }
                None => eprintln!("[{}] Not create acc", thread_id),
            }
        });
        handles.push(handle);
    }


    for handle in handles {
        handle.await?;
    }

    Ok(())
}

