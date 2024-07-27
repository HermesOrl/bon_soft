use std::{fs, io};
use std::path::Path;
use reqwest::{Client, Error, header::HeaderMap};
use cookie::{Cookie, CookieJar};
use std::sync::{Arc, Mutex};
use serde::de::StdError;
use serde_json::Value;
use super::XSRF_TOKEN_LINKS;

pub struct DoxbinAccount {
    client: Client,
    cookie_jar: Arc<Mutex<CookieJar>>,
    headers: HeaderMap,
}

impl DoxbinAccount {
    pub fn new() -> Self {
        let client = Client::builder().build().expect("Failed to build client");

        DoxbinAccount {
            client,
            headers: HeaderMap::new(),
            cookie_jar: Arc::new(Mutex::new(CookieJar::new())),
        }
    }

    fn get_cookie_header(&self, url: &str) -> Option<String> {
        let jar = self.cookie_jar.lock().unwrap();
        let url = reqwest::Url::parse(url).expect("Invalid URL");
        let cookies = jar.iter()
            .filter(|cookie| {
                cookie.domain().map_or(false, |domain| url.domain().map_or(false, |url_domain| url_domain.ends_with(domain)))
            })
            .map(|cookie| format!("{}={}", cookie.name(), cookie.value()))
            .collect::<Vec<_>>()
            .join("; ");
        if cookies.is_empty() { None } else { Some(cookies) }
    }

    async fn get(&mut self, url: &str) -> Result<usize, Error> {
        let mut request = self.client.get(url).headers(self.headers.clone());
        if let Some(cookie_header) = self.get_cookie_header(url) {
            request = request.header(reqwest::header::COOKIE, cookie_header);
        }
        let response = request.send().await?;
        {
            let mut jar = self.cookie_jar.lock().unwrap();
            for cookie in response.headers().get_all(reqwest::header::SET_COOKIE).iter() {
                if let Ok(cookie_str) = cookie.to_str() {
                    if let Ok(parsed_cookie) = Cookie::parse(cookie_str.to_string()) {
                        jar.add(parsed_cookie);
                    }
                }
            }
        }
        Ok(response.status().as_u16() as usize)
    }

    async fn post(&mut self, url: &str, body: &Value) -> Result<(), Error> {
        let mut request = self.client.post(url).headers(self.headers.clone()).json(body);
        if let Some(cookie_header) = self.get_cookie_header(url) {
            request = request.header(reqwest::header::COOKIE, cookie_header);
        }
        let response = request.send().await?;
        {
            let mut jar = self.cookie_jar.lock().unwrap();
            for cookie in response.headers().get_all(reqwest::header::SET_COOKIE).iter() {
                if let Ok(cookie_str) = cookie.to_str() {
                    if let Ok(parsed_cookie) = Cookie::parse(cookie_str.to_string()) {
                        jar.add(parsed_cookie);
                    }
                }
            }
        }


        Ok(())
    }

    fn print_cookies(&self, message: &str) {
        let jar = self.cookie_jar.lock().unwrap();
        for cookie in jar.iter() {
            println!(
                "{}: name={}, value={}, domain={}, expires={:?}",
                message,
                cookie.name(),
                cookie.value(),
                cookie.domain().unwrap_or(""),
                cookie.expires()
            );
        }
    }

    fn check_xsrf_token(&self) -> Option<()> {
        let mut jar = self.cookie_jar.lock().unwrap();
        if jar.get("XSRF-TOKEN").is_some() {
            return Some(());
        }
        None
    }
    pub async fn generate_xsrf_token(&mut self) -> Option<()> {
        for link in XSRF_TOKEN_LINKS.iter() {
            self.get(link.clone()).await.expect("TODO: panic message");
        }
        let json_payload = match read_json_from_file("payload.json") {
            Ok(payload) => payload,
            Err(e) => {
                eprintln!("Error reading JSON payload: {}", e);
                return None;
            }
        };
        match self.post("https://doxbin.org/.well-known/ddos-guard/mark/", &json_payload).await {
            // Ok(status) => { println!("POST request status code: {}", status) },
            // Err(e) => eprintln!("POST request error: {}", e),
            _ => {}
        }
        self.get(&"https://doxbin.org/").await;
        // self.print_cookies("asd");
        self.check_xsrf_token()
    }
}
fn read_json_from_file<P: AsRef<Path>>(path: P) -> Result<Value, Box<dyn StdError>> {
    let data = fs::read_to_string(path)?;
    let json: Value = serde_json::from_str(&data)?;
    Ok(json)
}