use std::{fs, io};
use std::path::Path;
use reqwest::{Client, Error, get, header::HeaderMap};
use cookie::{Cookie, CookieJar};
use std::sync::{Arc, Mutex};
use serde::de::StdError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use super::XSRF_TOKEN_LINKS;
use super::enums::{ApiCaptchaResponseGetCode, Payload, read_json_from_file_captcha, ApiCaptchaResponse};
use super::config::{generate_password, generate_username};
use dotenv::dotenv;
use std::env;
use std::io::Read;
use tokio::time::{sleep, Duration};
use regex::Regex;

pub struct DoxbinAccount {
    client: Arc<Client>,
    cookie_jar: Arc<Mutex<CookieJar>>,
    headers: HeaderMap,
    captcha_client: Captcha
}

impl DoxbinAccount {
    pub fn new(client: Arc<Client>) -> Self {

        // let client = Client::builder().build().expect("Failed to build client");

        DoxbinAccount {
            client,
            headers: HeaderMap::new(),
            cookie_jar: Arc::new(Mutex::new(CookieJar::new())),
            captcha_client: Captcha::new(),
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

    async fn get(&mut self, url: &str) -> Result<(usize, String), Error> {
        // if url == "https://doxbin.org/register" {
        //     self.print_cookies("DO");
        // }
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
        // if url == "https://doxbin.org/register" {
        //     self.print_cookies("POSLE");
        // }
        let rtrn =( response.status().as_u16() as usize, response.text().await?);
        Ok(rtrn)
    }

    async fn post(&mut self, url: &str, body: &Value) -> Result<usize, Error> {
        let mut request = self.client.post(url).headers(self.headers.clone()).json(body);
        if let Some(cookie_header) = self.get_cookie_header(url) {
            request = request.header(reqwest::header::COOKIE, cookie_header);
        }
        let response = request.send().await?;
        // println!("{}", response.status());
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


        Ok((response.status().as_u16() as usize))
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

    pub async fn create_account(&mut self) -> Option<(String, String)> {
        if self.generate_xsrf_token().await.is_some() {
            if let Ok((status, text)) = self.get("https://doxbin.org/register").await {
                let re = Regex::new(r#"<input type="hidden" name="_token" value="([^"]+)""#).expect("Failed to create regex");
                if let Some(captures) = re.captures(&text) {
                    if let Some(value) = captures.get(1) {
                        let _token = value.as_str();
                        let _code = self.captcha_client.get_token().await.unwrap_or_default();
                        // println!("Token value: {}", _token);
                        let pswd = generate_password();
                        let snm = generate_username();
                        let json_payload = json!({
                            "username": snm,
                            "email": "",
                            "password": pswd,
                            "confpass": pswd,
                            "_token": _token,
                            "hcaptcha_token": _code,
                        });
                        if let resp_code = self.post("https://doxbin.org/register", &json_payload).await.expect("TODO: panic message") == 200 {
                            // println!("{}:{}", snm, pswd);
                            return Some((snm, pswd));
                        }
                    }
                }
            }
        }
        None
    }
}
fn read_json_from_file<P: AsRef<Path>>(path: P) -> Result<Value, Box<dyn StdError>> {
    let data = fs::read_to_string(path)?;
    let json: Value = serde_json::from_str(&data)?;
    Ok(json)
}



pub struct Captcha {
    pub api_key: String,
    pub data_site_key: String,
}

impl Captcha {
    pub fn new() -> Self {
        dotenv().ok();
        Captcha {
            api_key: env::var("API2CAPTCHA").expect("API2CAPTCHA NOT FOUND").to_string(),
            data_site_key: "c902269c-b6ad-4309-b393-c8c9fd010011".to_string(),
        }
    }

    pub async fn get_code(&self, task_id: usize) -> Option<String> {
        let apikey = self.api_key.clone();
        let client = Client::builder().build().expect("Failed to build client captcha GET_CODE");
        let mut json_payload = json!({
            "clientKey": apikey,
            "taskId": task_id,
        });
        for i in 0..20 {
            let response = client.post("https://api.2captcha.com/getTaskResult")
                .json(&json_payload)
                .send().await.expect("Failed to send request get code");

            let response_json: Value = response.json().await.expect("Failed to parse response JSON");
            // println!("{:?}", response_json);
            match serde_json::from_value::<ApiCaptchaResponseGetCode>(response_json.clone()) {
                Ok(api_response) => {
                    match api_response {
                        ApiCaptchaResponseGetCode::Processing { common } => {
                            // println!("Processing: errorId: {}, status: {}", common.errorId, common.status);
                            sleep(Duration::from_secs(5)).await;
                        },
                        ApiCaptchaResponseGetCode::Error { common, errorCode, errorDescription } => {
                            // println!("Error: errorId: {}, status: {}, errorCode: {}, errorDescription: {}", common.errorId, common.status, errorCode, errorDescription);
                            break
                        },
                        ApiCaptchaResponseGetCode::Ready { common, solution, cost, ip, createTime, endTime, solveCount } => {
                            // println!("Ready: errorId: {}, status: {}, solution: {:?}, cost: {}, ip: {}, createTime: {}, endTime: {}, solveCount: {}",
                            //          common.errorId, common.status, solution, cost, ip, createTime, endTime, solveCount);
                            if let Some(g_recaptcha_response) = solution.get("gRecaptchaResponse").and_then(|v| v.as_str()) {
                                return Some(g_recaptcha_response.to_string());
                            }
                            break;
                        },
                    }
                }
                Err(e) => {
                    eprintln!("Failed to deserialize JSON to ApiResponse: {}", e);
                    break;
                }
            }
        }
        None
    }
    pub async fn get_token(&self) -> Option<String> {

        let mut json_payload: Payload = match read_json_from_file_captcha("payload_create_captcha.json") {
            Ok(payload) => payload,
            Err(e) => {
                eprintln!("Error reading JSON payload captcha: {}", e);
                return None;
            }
        };
        json_payload.clientKey = self.api_key.to_string();
        json_payload.task.websiteURL = "doxbin.org".to_string();
        json_payload.task.websiteKey = self.data_site_key.to_string();

        let client = Client::builder().build().expect("Failed to build client captcha");
        let response = client.post("https://api.2captcha.com/createTask")
            .json(&json_payload)
            .send()
            .await.expect("TODO: Captcha create panic");
        match response.json::<ApiCaptchaResponse>().await {
            Ok(response_json) => {
                println!("{:?}", response_json.taskId);
                if let Some(_code_captcha) = self.get_code(response_json.taskId as usize).await {
                    // println!("{:?}", _code_captcha);
                    return Some(_code_captcha);
                }
            }
            Err(e) => {
                eprintln!("Failed to parse response JSON: {}", e);
            }
        }
        None

    }
}