use tokio::task;
mod models;
use serde::de::StdError;
use models::{request, proxy, enums::{DoxBinAccountGetXsrf, ModeSubscribeOnPastes, ModeComment}};
use reqwest::{Client, Proxy};
use std::sync::{Arc, Mutex};
use std::fs::OpenOptions;
use std::io::Write;
use tokio::task::spawn_blocking;
use tokio::sync::mpsc;


#[cfg(test)]
mod tests {
    use crate::models::enums::{ModeChange, ParameterComment};
    use super::*;
    use tokio;
    use tokio::runtime::Runtime;

    #[tokio::test]
    async fn check() {

        let client = Arc::new(Client::builder()
            .pool_max_idle_per_host(50)
            .build()
            .expect("Failed to build client auth doxbin storage"));
        let client_clone = Arc::clone(&client);
        let mut dox_acc = request::DoxbinAccount::new(client_clone);
        dox_acc.upload_proxies();
        let result_change_proxy = dox_acc.change_profile(ModeChange::All).await;
        assert!(!matches!(result_change_proxy, None));
        let result = dox_acc.paste(ModeComment::Paste, ParameterComment {
            username: "asd".to_string(),
            link: "https://doxbin.org/upload/YopDreiProhax1".to_string(),
            anon: true,
            text: "leee".to_string(),
        }).await;

        assert!(!matches!(result, None));
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    let client = Arc::new(Client::builder()
        .pool_max_idle_per_host(50)
        .build()
        .expect("Failed to build client auth doxbin storage"));
    let client_clone = Arc::clone(&client);
    let mut dox_acc = request::DoxbinAccount::new(client_clone);
    dox_acc.upload_proxies();
    // if let Some(()) = dox_acc.paste()



    // if let Some(()) = dox_acc.generate_xsrf_token(DoxBinAccountGetXsrf::NewAccount).await {
    //     dox_acc.subscribe_on_pastes(ModeSubscribeOnPastes::Ignore).await
    // };




    // let mut dox_acc_storage = request::DoxbinAccountStorage::new();
    // dox_acc_storage._auth("dae232b41db709ec5cae89473b4b1dd6".to_string()).await;
    // if let Some((total_count, session_count)) = dox_acc_storage.load_from_file() {
    //     println!("{total_count}, {session_count}");
    // }
    Ok(())
    // threads_reg().await
}

async fn threads_reg() -> Result<(), Box<dyn StdError>> {
    let mut proxy_manager = proxy::SProxies::new();
    proxy_manager.add_from_file("./proxies.txt").expect("TODO: panic message proxy");
    // for i in 0..50 {
    //     match proxy_manager.get_next_proxy() {
    //         Ok(proxyy) => println!("proxy: {:?}", proxyy),
    //         Err((e)) => eprintln!("Error: {:?}", e)
    //     }
    // }
    // Ok(())
    let num_tasks = 10;
    let (tx, mut rx) = mpsc::channel(100);
    let file = Arc::new(Mutex::new(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open("results.txt")
            .expect("Unable to open file"),
    ));

    let file_clone = Arc::clone(&file);
    let write_handle = task::spawn(async move {
        while let Some((username, password, session)) = rx.recv().await {
            let file_clone = Arc::clone(&file_clone);
            spawn_blocking(move || {
                let mut file = file_clone.lock().unwrap();
                writeln!(file, "{}:{}\t{}", username, password, session).expect("Unable to write to file");
            }).await.expect("Failed to write to file");
        }
    });

    let mut handles = vec![];
    for thread_id in 0..num_tasks {
        match proxy_manager.get_next_proxy() {
            Ok(proxyy) => {
                println!("proxy: {:?}", proxyy.proxy_url);
                let client = Arc::new(Client::builder()
                    .pool_max_idle_per_host(50)
                    .proxy(Proxy::all(proxyy.proxy_url)?)
                    .build()
                    .expect("Failed to build client"));
                let client_clone = Arc::clone(&client);
                let tx_clone = tx.clone();
                let handle = task::spawn(async move {
                    let mut dox_acc = request::DoxbinAccount::new(client_clone);
                    match dox_acc.create_account().await {
                        Some((username, password, session)) => {
                            println!("[{}] Create acc\t{}:{}\tSession: {}", thread_id, username, password, session);
                            tx_clone.send((username, password, session)).await.expect("Failed to send result");
                        }
                        None => eprintln!("[{}] Not create acc", thread_id),
                    }
                });
                handles.push(handle);
            }
            Err((e)) => eprintln!("Error: {:?}", e)
        }
    }
    drop(tx);
    for handle in handles {
        handle.await?;
    }
    write_handle.await?;

    Ok(())
}