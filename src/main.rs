use tokio::task;
mod models;
use serde::de::StdError;
use models::{request, proxy, enums::{DoxBinAccountGetXsrf, ModeSubscribeOnPastes, ModeComment, ParameterCommentAccountUseExist}};
use models::enums::{ResponseChannel, ResponseChannelInfo};
use reqwest::{Client, Proxy};
use std::sync::{Arc, Mutex};
use std::fs::OpenOptions;
use std::io::Write;
use tokio::task::spawn_blocking;
use tokio::sync::mpsc;
use crate::models::enums::{ModeChange, ParameterComment, ParameterCommentAccount};
use std::thread;

#[cfg(test)]
mod tests {
    use crate::models::enums::{ModeChange, ParameterComment, ParameterCommentAccount};
    use super::*;
    use tokio;
    #[tokio::test]
    async fn tttd() {
        let mut message_count = 0;
        let (tx, mut rx) = mpsc::channel(150);
        tokio::spawn(async move {
            let client = Arc::new(Client::builder()
                        .pool_max_idle_per_host(50)
                        .build()
                        .expect("Failed to build client auth doxbin storage"));
            let client_clone = Arc::clone(&client);
            let mut dox_acc = request::DoxbinAccount::new(client_clone);
            dox_acc.upload_proxies();
            dox_acc.change_profile(ModeChange::Cookie).await;
            dox_acc.subscribe_on_pastes(ModeSubscribeOnPastes::Comment {
                text: "ddd".to_string(),
                mode_comment: ModeComment::Paste,
                anon: false,
            }, tx).await;
        });
        let mut handles = vec![];
        while let Some(response) = rx.recv().await {
            if let ResponseChannel::Sending {data} = response {
                let handle = task::spawn(async move {
                    let client = Arc::new(Client::builder()
                        .pool_max_idle_per_host(50)
                        .build()
                        .expect("Failed to build client"));
                    let client_clone = Arc::clone(&client);
                    let mut dox_acc = request::DoxbinAccount::new(client_clone);
                    dox_acc.change_profile(ModeChange::Cookie).await;
                    if let Some(value) = dox_acc.paste(ModeComment::Paste,
       ParameterComment {
                           username: data.username,
                           link: data.link,
                           parameter_account: ParameterCommentAccount::UseExist
                           {
                               exist_type: ParameterCommentAccountUseExist::ExistAnon
                           },
                           text: "check".to_string()
                       }).await {
                        println!("{}", value)
                    }

                });
                println!("start thread");
                handles.push(handle);
                message_count += 1;
            }
            if message_count >= 5 {
                println!("wait");
                for handle in handles.iter_mut() {
                    if let Err(e) = handle.await {
                        println!("error suspend thread {:?}", e);
                    }
                }
                handles.clear();
                message_count = 0;
            }
        }
    }

}
#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    let (tx, mut rx) = mpsc::channel(150);
    tokio::spawn(async move {
        let client = Arc::new(Client::builder()
            .pool_max_idle_per_host(50)
            .build()
            .expect("Failed to build client auth doxbin storage"));
        let client_clone = Arc::clone(&client);
        let mut dox_acc = request::DoxbinAccount::new(client_clone);
        dox_acc.upload_proxies();
        dox_acc.change_profile(ModeChange::Cookie).await;
        dox_acc.subscribe_on_pastes(ModeSubscribeOnPastes::Comment {
            text: "".to_string(),
            mode_comment: ModeComment::Paste,
            anon: false,
        }, tx).await;
    });
    let mut handles = vec![];
    while let Some(response) = rx.recv().await {
        if let ResponseChannel::Sending {data} = response {
            let client = Arc::new(Client::builder()
                .pool_max_idle_per_host(50)
                .build()
                .expect("Failed to build client"));
            let client_clone = Arc::clone(&client);
            let handle = task::spawn(async move {
                let mut dox_acc = request::DoxbinAccount::new(client_clone);
                dox_acc.change_profile(ModeChange::Cookie).await;
                dox_acc.paste(ModeComment::Paste,
                              ParameterComment {
                                  username: data.username, link: data.link,
                                  parameter_account: ParameterCommentAccount::UseExist
                                  {
                                      exist_type: ParameterCommentAccountUseExist::ExistAnon
                                  },
                                  text: "check".to_string()
                              }).await
            });
            handles.push(handle);
        }
    }
    if !handles.is_empty() {
        for handle in handles {
            handle.await;
        }
    }
    // let client = Arc::new(Client::builder()
    //     .pool_max_idle_per_host(50)
    //     .build()
    //     .expect("Failed to build client auth doxbin storage"));
    // let client_clone = Arc::clone(&client);
    // let mut dox_acc = request::DoxbinAccount::new(client_clone);
    // dox_acc.upload_proxies();
    Ok(())
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

