use do_soft::models::{request, proxy, config, enums};
use std::fs::{File, OpenOptions};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex as TokioMutex};
use tokio::{task};
use std::io::Write;
use std::error::Error as StdError;
use tokio::task::JoinHandle;


struct TestContext {
    file: std::fs::File,
    manager: Arc<TokioMutex<enums::LinkManager>>,
}

async fn setup_test_context() -> TestContext {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("./parsing.txt")
        .expect("Failed to create/open file");

    let manager = Arc::new(TokioMutex::new(enums::LinkManager::new()));
    {
        let mut manager_use = manager.lock().await;
        manager_use.read_from_file("./parsing.txt").expect("Failed to read from file");
    }

    TestContext { file, manager }
}


#[tokio::test]
async fn test_file_initialization() {
    let context = setup_test_context().await;
    assert!(context.file.metadata().unwrap().is_file());
}

#[tokio::test]
async fn test_manager_initialization() {
    let context = setup_test_context().await;
    let mut manager_use = context.manager.lock().await;
    assert!(manager_use.read_from_file("./parsing.txt").is_ok());
}

#[tokio::test]
async fn test_spawn_task() {
    let context = setup_test_context().await;
    let (tx, mut rx) = mpsc::channel(150);
    let manager_clone = Arc::clone(&context.manager);

    tokio::spawn(async move {
        let tx_clone = tx.clone();
        let client = Arc::new(reqwest::Client::builder()
            .pool_max_idle_per_host(50)
            .build()
            .expect("Failed to build client"));
        let client_clone = Arc::clone(&client);
        let mut dox_acc = request::DoxbinAccount::new(client_clone);
        dox_acc.upload_proxies();
        dox_acc.change_profile(enums::ModeChange::Cookie).await;
        dox_acc.subscribe_on_pastes(enums::ModeSubscribeOnPastes::Comment {
            text: "ddd".to_string(),
            mode_comment: enums::ModeComment::Paste,
            anon: false,
        }, tx_clone, manager_clone).await;
    });

    // Проверьте, что канал был создан и что в нем могут появляться данные
    let response = rx.recv().await;
    assert!(response.is_some());
}

#[tokio::test]
async fn test_process_handles() {
    let mut context = setup_test_context().await;
    let mut handles = vec![];
    let mut successes = 0;
    let mut failures = 0;

    let handle = task::spawn(async {
        Some(("test_user".to_string(), "test_link".to_string()))
    });

    handles.push(handle);

    process_handles(&mut handles, &mut successes, &mut failures, &mut context.file).await;

    assert_eq!(successes, 1);
    assert_eq!(failures, 0);
}


async fn process_handles(
    handles: &mut Vec<JoinHandle<Option<(String, String)>>>,
    successes: &mut usize,
    failures: &mut usize,
    file: &mut std::fs::File,
) {
    for handle in handles.iter_mut() {
        match handle.await {
            Ok(Some((usernamee, linkk))) => { println!("Create Paste: {}", linkk); *successes += 1; writeln!(file, "{}_;_{}", usernamee, linkk);},
            Ok(None) => *failures += 1,
            Err(e) => { println!("error suspend thread {:?}", e); *failures += 1 },
        }
    }
    handles.clear();
}