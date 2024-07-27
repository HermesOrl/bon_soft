use std::sync::Arc;
use reqwest::{Client, Url};
use reqwest::cookie::{CookieStore, Jar};
use reqwest::header::HeaderMap;
use reqwest::Error;
pub struct DoxbinAccount {
    client: Client,
    cookie_jar: Arc<Jar>,
    headers: HeaderMap,
}

impl DoxbinAccount {
    pub fn new() -> Self {
        let cookie_jar = Arc::new(Jar::default());
        let client = Client::builder()
            .cookie_store(true)
            .cookie_provider(Arc::clone(&cookie_jar))
            .build()
            .expect("Failed to build client");

        DoxbinAccount {
            client,cookie_jar,headers: HeaderMap::new()
        }
    }
    pub async fn get(&mut self, url: &str) -> Result<String, Error> {
        let response = self.client.get(url)
            .headers(self.headers.clone())
            .send()
            .await?;

        for (key, value) in response.headers().iter() {
            self.headers.insert(key,value.clone());
            println!()
        }
        if let Ok(parsed_url) = Url::parse(url) {
            let cookies = self.cookie_jar.cookies(&parsed_url);
            println!("Update cookie: {:?}", cookies);
        }
        let body = response.text().await?;
        Ok(body)
    }
}