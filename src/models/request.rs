use std::{fs, io};
use std::path::Path;
use reqwest::{Client, Error, get, header::HeaderMap, Proxy};
use cookie::{Cookie, CookieJar};
use std::sync::{Arc, Mutex};
use serde::de::StdError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use super::{request, XSRF_TOKEN_LINKS};
use super::enums::{ApiCaptchaResponseGetCode, Payload, read_json_from_file_captcha, ApiCaptchaResponse, DoxBinAccount, DoxBinAccountSession, DoxBinAccountGetXsrf, ResponseParsing, LinkManager, ModeSubscribeOnPastes, ModeComment, ModeChange, ParameterComment};
use super::config::{generate_password, generate_username};
use dotenv::dotenv;
use std::env;
use std::io::Write;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read};
use tokio::time::{sleep, Duration};
use regex::Regex;
use scraper::{Html, Selector};
use super::proxy::SProxies;
use tokio::runtime::Runtime;

pub struct DoxbinAccount {
    client: Arc<Client>,
    cookie_jar: Arc<Mutex<CookieJar>>,
    headers: HeaderMap,
    captcha_client: Captcha,
    proxy_manager: SProxies,
}

pub struct DoxbinAccountStorage {
    accounts_storage: Vec<DoxBinAccount>
}

impl DoxbinAccountStorage {

    pub fn new() -> Self {
        DoxbinAccountStorage {
            accounts_storage: Vec::new(),
        }
    }
    pub fn load_from_file(&mut self) -> Option<(i32, i32)> {
        let file = File::open("./results.txt");
        let file = match file {
            Ok(file) => file,
            Err(_) => return None,
        };
        let reader = BufReader::new(file);

        let mut accs_count = 0;
        let mut accs_count_with_session = 0;

        for line in reader.lines() {
            let line = match line {
                Ok(line) => line,
                Err(_) => return None,
            };
            let mut login = String::new();
            let mut password = String::new();
            let mut session: DoxBinAccountSession = DoxBinAccountSession::WithoutSession;
            if line.contains(':') {
                accs_count += 1;
                let cloned_line = line.clone();
                let parts: Vec<&str> = cloned_line.split(":").collect();
                login = parts[0].trim().to_string();
                password = parts[1].trim().to_string();

            }
            if line.contains('\t') {
                let cloned_line = line.clone();
                let parts: Vec<&str> = cloned_line.split("\t").collect();
                accs_count_with_session += 1;
                session = DoxBinAccountSession::WithSession {session_str: parts[1].trim().to_string()}
            }
            self.accounts_storage.push(DoxBinAccount{login, password, session})
        }
        self.print_storage();
        Some((accs_count, accs_count_with_session))
    }

    fn print_storage(&self) {
        for i in &self.accounts_storage {
            println!("{}:{}\t{:?}", i.login, i.password,i.session)
        }
    }

    pub async fn _auth(&self, session: String) {
        let client = Arc::new(Client::builder()
            .pool_max_idle_per_host(50)
            .build()
            .expect("Failed to build client auth doxbin storage"));
        let client_clone = Arc::clone(&client);
        let mut dox_acc = request::DoxbinAccount::new(client_clone);
        dox_acc.generate_xsrf_token(DoxBinAccountGetXsrf::ExistAccount {session }).await.unwrap()
    }
}

impl DoxbinAccount {
    pub fn new(client: Arc<Client>) -> Self {
        DoxbinAccount {
            client,
            headers: HeaderMap::new(),
            cookie_jar: Arc::new(Mutex::new(CookieJar::new())),
            captcha_client: Captcha::new(),
            proxy_manager: SProxies::new(),
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
    async fn get(&self, url: &str) -> Result<(usize, String), Error> {
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
        let rtrn =( response.status().as_u16() as usize, response.text().await?);
        Ok(rtrn)
    }
    async fn post(&self, url: &str, body: &Value) -> Result<(usize, String, String), Error> {
        let mut request = self.client.post(url).headers(self.headers.clone()).json(body);
        if let Some(cookie_header) = self.get_cookie_header(url) {
            request = request.header(reqwest::header::COOKIE, cookie_header);
        }
        let response = request.send().await?;
        let mut session_value = String::new();
        {
            let mut jar = self.cookie_jar.lock().unwrap();
            for cookie in response.headers().get_all(reqwest::header::SET_COOKIE).iter() {
                if let Ok(cookie_str) = cookie.to_str() {
                    if let Ok(parsed_cookie) = Cookie::parse(cookie_str.to_string()) {
                        jar.add(parsed_cookie.clone());
                        if parsed_cookie.name() == "session" {
                            session_value = parsed_cookie.value().to_string(); // Сохраняем значение
                        }
                    }
                }
            }
        }


        Ok((response.status().as_u16() as usize, session_value, response.text().await.unwrap()))
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
    pub async fn generate_xsrf_token(&self, type_generate: DoxBinAccountGetXsrf) -> Option<()> {
        match type_generate {
            DoxBinAccountGetXsrf::ExistAccount {session} => {
                let mut jar = self.cookie_jar.lock().unwrap();
                let cookie = Cookie::new("session", session.clone());
                let cookie = Cookie::build(cookie)
                    .domain("doxbin.org")
                    .path("/")
                    .http_only(false)
                    .secure(false)
                    .finish();
                jar.add(cookie);
            },
            _ => {}
        }
        for link in XSRF_TOKEN_LINKS.iter() {
            match self.get(link.clone()).await {
                Ok(_) => {}
                Err(_) => { return None }
            }
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
        if let Ok((status_code, text)) = self.get(&"https://doxbin.org/").await {
            // println!("{}", text.clone())
        }
        self.check_xsrf_token()
    }
    pub async fn create_account(&self) -> Option<(String, String, String)> {
        if self.generate_xsrf_token(DoxBinAccountGetXsrf::NewAccount).await.is_some() {
            if let Ok((status, text)) = self.get("https://doxbin.org/register").await {
                let re = Regex::new(r#"<input type="hidden" name="_token" value="([^"]+)""#).expect("Failed to create regex");
                if let Some(captures) = re.captures(&text) {
                    if let Some(value) = captures.get(1) {
                        let _token = value.as_str();
                        let _code = self.captcha_client.get_token().await.unwrap_or_default();
                        println!("Token value: {}", _code);
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
                        match self.post("https://doxbin.org/register", &json_payload).await {
                            Ok((resp_code, _message, _)) if resp_code == 200 => {
                                println!("{}:{}\tSession: {}", snm, pswd, _message);
                                return Some((snm, pswd, _message));
                            }
                            Err(e) => {
                                eprintln!("Error match get session {:?}", e)
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        None
    }
    pub async fn pars_past(&self) -> Option<(Vec<ResponseParsing>)> {
        let mut manager = LinkManager::new();
        manager.read_from_file("./parsing.txt").ok();
        let mut results: Vec<ResponseParsing> = Vec::new();
        for iter in 1..1000 {
            if let Ok((status_code, html)) = self.get(&format!("https://doxbin.org/?page={}", iter)).await {
                let document = Html::parse_document(&html);
                let tbody_selector = Selector::parse("tbody").unwrap();
                let tr_selector = Selector::parse("tr.doxentry").unwrap();
                let a_selector = Selector::parse("a").unwrap();
                let user_selector = Selector::parse("a.dox-username").unwrap();

                let tbodies = document.select(&tbody_selector).collect::<Vec<_>>();
                if tbodies.len() < 2 {
                    continue
                }

                let mut file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .append(true)
                    .open("parsing.txt")
                    .expect("Unable to open file");
                for element in tbodies[1].select(&tr_selector) {
                    let link_elem = element.select(&a_selector).next().unwrap();
                    let link = link_elem.value().attr("href").unwrap_or_default().to_string();
                    println!("Iter: {}", iter);

                    let user_elem = element.select(&user_selector).next();
                    let user = user_elem.map_or(String::from("Unknown"), |e| e.inner_html().trim().to_string());
                    let id = element.value().attr("id").unwrap_or_default().to_string();
                    if manager.add_link(link.clone()) {
                        writeln!(file, "{}_;_{}_;_{}", id, user, link).expect("REASON")
                    }
                }


            }
        }
        return Some(results)
    }

    pub fn upload_proxies(&mut self) -> Option<()> {
        self.proxy_manager.add_from_file("./proxies.txt").expect("TODO: failed upload proxies");
        return Some(())
    }

    fn clear_cookies(&mut self) -> Option<()> {
        let mut cooks = self.cookie_jar.lock().unwrap();
        *cooks = CookieJar::new();
        return Some(())
    }

    async fn change_cookies(&mut self) -> Option<()> {
        self.clear_cookies();
        if let Some(()) = self.generate_xsrf_token(DoxBinAccountGetXsrf::NewAccount).await {
            return Some(())
        }
        None
    }

    fn change_proxy(&mut self) -> Option<()> {
        match self.proxy_manager.get_next_proxy() {
            Ok(proxy_sproxy) => {
                let new_client = Arc::new(Client::builder()
                    .proxy(Proxy::all(&proxy_sproxy.proxy_url).ok()?)
                    .build()
                    .expect("Failed to build client in change_proxy"));
                self.client = new_client;
                return Some(());
            }
            Err(e) => {eprintln!("Failed get next proxy {}", e)}
        }
        None
    }

    pub async fn check_ip(&mut self) -> Option<()> {
        if let Ok((status_code, text)) = self.get(&"https://httpbin.org/ip").await {
            println!("Not change ip: {}", text);
        }
        if let Some(()) = self.change_proxy() {
            if let Ok((status_code, text)) = self.get(&"https://httpbin.org/ip").await {
                println!("Changed ip: {}", text);
                return Some(())
            }
        }
        return None
    }

    pub async fn change_profile(&mut self, mode_change: ModeChange) -> Option<()> {
        match mode_change {
            ModeChange::Proxy => {
                return self.change_proxy();
            },
            ModeChange::Cookie => {
                return self.change_cookies().await;
            },
            ModeChange::All => {
                if let Some(()) = self.change_proxy() {
                    return self.change_cookies().await
                }
            },
        }
        None
    }

    async fn comment_paste(&mut self, link: String, message_paste: String) -> Option<()>{
        if let Ok((status_code, text)) = self.get(&link).await {
            let re = Regex::new(r#"<input type="hidden" name="_token" value="([^"]+)""#).expect("Failed to create regex");
            if let Some(captures) = re.captures(&text) {
                if let Some(value) = captures.get(1) {
                    let _token = value.as_str();
                    let _code = self.captcha_client.get_token().await?;
                    println!("{}", _code);
                    let doxid = text.split("doxid = ")
                        .nth(1)
                        .and_then(|s| s.split(";").next())?;
                    println!("doxid: {}", doxid);
                    let payload_json = json!({
                        "doxId": doxid,
                        "name": "Anonymous",
                        "comment": message_paste,
                        "_token": _token,
                        "hcaptcha_token": _code,
                    });
                    if let Ok((status_code, _, response_html)) = self.post(&"https://doxbin.org/comment", &payload_json).await {
                        println!("{}", response_html);
                        println!("status code create paste: {}", status_code);
                        if let cont = response_html.contains("\"status\":\"done\"") {
                            return Some(())
                        }
                    }
                }
            }
        }
        None
    }
    pub async fn paste(&mut self, mode_paste: ModeComment, parameters_comment: ParameterComment) -> Option<()> {
        self.change_proxy();
        if !&parameters_comment.anon {
            if let Some((username, password, session_str)) = self.create_account().await {
                println!("Log print\t{}:{}|{}", username, password, session_str);
            }
        }
        match mode_paste {
            ModeComment::Paste => {
                return self.comment_paste(parameters_comment.link, parameters_comment.text).await;
            },
            ModeComment::Profile => {},
            ModeComment::PasteAndProfile => {},
        }
        None
    }

    pub async fn subscribe_on_pastes(&mut self, mode: ModeSubscribeOnPastes) {
        let mut manager = LinkManager::new();
        manager.read_from_file("./parsing.txt").ok();
        for iter in 1..15000 {
            let mut count_add = 0;
            if let Ok((status_code, html)) = self.get(&"https://doxbin.org/?page=1").await {
                let document = Html::parse_document(&html);
                let tbody_selector = Selector::parse("tbody").unwrap();
                let tr_selector = Selector::parse("tr.doxentry").unwrap();
                let a_selector = Selector::parse("a").unwrap();
                let user_selector = Selector::parse("a.dox-username").unwrap();

                let tbodies = document.select(&tbody_selector).collect::<Vec<_>>();
                if tbodies.len() < 2 {
                    continue
                }

                let mut file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .append(true)
                    .open("parsing.txt")
                    .expect("Unable to open file");
                for element in tbodies[1].select(&tr_selector) {
                    let link_elem = element.select(&a_selector).next().unwrap();
                    let link = link_elem.value().attr("href").unwrap_or_default().to_string();
                    // println!("Iter: {}", iter);

                    let user_elem = element.select(&user_selector).next();
                    let user = user_elem.map_or(String::from("Unknown"), |e| e.inner_html().trim().to_string());
                    let id = element.value().attr("id").unwrap_or_default().to_string();
                    if manager.add_link(link.clone()) {
                        count_add += 1;
                        writeln!(file, "{}_;_{}_;_{}", id, user.clone(), link.clone()).expect("REASON");
                        if let ModeSubscribeOnPastes::Comment { text,  mode_comment, anon} = mode.clone() {
                            if let Some(()) = self.paste(mode_comment, ParameterComment{username: user, link, anon, text}).await {
                                println!("Comment success")
                            }
                        }
                    }
                }
            }
            println!("Add {} users. Sleeping....", count_add);
            sleep(Duration::from_secs(50)).await;
        }
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
            data_site_key: "9e62bf30-dc6e-4024-9192-aacaa13ce824".to_string(),
        }
    }

    async fn get_code(&self, task_id: usize) -> Option<String> {
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