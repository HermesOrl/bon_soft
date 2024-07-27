use reqwest::{Client, Error, header::HeaderMap};
use cookie::{Cookie, CookieJar};
use std::sync::{Arc, Mutex};
use serde_json::Value;

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

    pub async fn get(&mut self, url: &str) -> Result<usize, Error> {
        println!("{}", url);
        self.print_cookies("ДО ВЫПОЛНЕНИЯ запроса");

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

        self.print_cookies("После запроса");

        Ok(response.status().as_u16() as usize)
    }

    pub async fn post(&mut self, url: &str, body: &Value) -> Result<usize, Error> {
        println!("{}", url);
        self.print_cookies("ДО ВЫПОЛНЕНИЯ запроса");

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

        self.print_cookies("После запроса");

        Ok(response.status().as_u16() as usize)
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
}
