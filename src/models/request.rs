use std::{fs, io};
use std::path::Path;
use reqwest::{Client, Error, get, header::HeaderMap, Proxy};
use cookie::{Cookie, CookieJar};
use std::sync::{Arc, Mutex};
use serde::de::StdError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use super::{request, XSRF_TOKEN_LINKS};
use super::enums::{ApiCaptchaResponseGetCode, Payload, read_json_from_file_captcha,
                   ApiCaptchaResponse, DoxBinAccount, DoxBinAccountSession, DoxBinAccountGetXsrf, ResponseParsing,
                   LinkManager, ModeSubscribeOnPastes, ModeComment, ModeChange, ParameterComment, ParameterCommentAccount,
                   ParameterCommentAccountUseExist, ResponseChannel, ResponseChannelInfo};
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
use tokio::sync::mpsc;
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::Barrier;


pub struct DoxbinAccount {
    client: Arc<Client>,
    cookie_jar: Arc<Mutex<CookieJar>>,
    headers: HeaderMap,
    captcha_client: Captcha,
    proxy_manager: Arc<Mutex<SProxies>>,
    current_proxy: String,
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


        // TODO: WARNING, PROXY DONT WORK !!!
        // TODO: WARNING, PROXY DONT WORK !!!


        // TODO: WARNING, PROXY DONT WORK !!!
        let client_clone = Arc::clone(&client);
        // TODO: WARNING, PROXY DONT WORK !!!
        let proxy_manager = Arc::new(Mutex::new(SProxies::new()));
        // TODO: WARNING, PROXY DONT WORK !!!
        let mut proxy_manager_clone = Arc::clone(&proxy_manager);
        // TODO: WARNING, PROXY DONT WORK !!!
        let mut dox_acc = request::DoxbinAccount::new(client_clone, proxy_manager_clone);
        // TODO: WARNING, PROXY DONT WORK !!!


        // TODO: WARNING, PROXY DONT WORK !!!
        // TODO: WARNING, PROXY DONT WORK !!!


        dox_acc.generate_xsrf_token(DoxBinAccountGetXsrf::ExistAccount {session }).await.unwrap()
    }
}

impl DoxbinAccount {

    pub fn new(client: Arc<Client>, proxy_manager: Arc<Mutex<SProxies>>) -> Self {
        DoxbinAccount {
            client,
            headers: HeaderMap::new(),
            cookie_jar: Arc::new(Mutex::new(CookieJar::new())),
            captcha_client: Captcha::new(),
            proxy_manager,
            current_proxy: String::new(),
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
        // println!("{:?}", rtrn);
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
    fn check_session_token(&self) -> Option<()> {
        let mut jar = self.cookie_jar.lock().unwrap();
        if jar.get("session").is_some() {
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
                            Ok((resp_code, _message, response_)) if resp_code == 200 => {
                                println!("{}:{}\tSession: {}", snm, pswd, _message);
                                println!("response create account: {}", response_);
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
        let mut mut_proxy_manager = self.proxy_manager.lock().unwrap();
        mut_proxy_manager.add_from_file("./proxies.txt").expect("TODO: failed upload proxies");
        return Some(())
    }
    fn clear_cookies(&mut self) -> Option<()> {
        let mut cooks = self.cookie_jar.lock().unwrap();
        *cooks = CookieJar::new();
        return Some(())
    }
    async fn change_cookies(&mut self) -> Option<()> {
        self.clear_cookies();
        return self.generate_xsrf_token(DoxBinAccountGetXsrf::NewAccount).await;
    }
    fn change_proxy(&mut self) -> Option<()> {
        let mut mut_proxy_manager = self.proxy_manager.lock().unwrap();

        match mut_proxy_manager.get_next_proxy() {
            Ok(proxy_sproxy) => {
                self.current_proxy = proxy_sproxy.proxy_url.clone();
                let new_client = Arc::new(Client::builder()
                    .proxy(Proxy::all(&proxy_sproxy.proxy_url.clone()).ok()?)
                    .build()
                    .expect("Failed to build client in change_proxy"));
                self.client = new_client;
                println!("New proxy: {}", proxy_sproxy.proxy_url.clone());
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
                    let result_change_cookie = self.change_cookies().await;
                    return result_change_cookie;
                }
            },
        }
        None
    }
    async fn comment_paste(&mut self, parameters_comment: ParameterComment, barrier: Arc<Barrier>) -> Option<()>{
        if let Ok((status_code, text)) = self.get(&parameters_comment.link).await {
            let re = Regex::new(r#"<input type="hidden" name="_token" value="([^"]+)""#).expect("Failed to create regex");
            if let Some(captures) = re.captures(&text) {
                if let Some(value) = captures.get(1) {
                    let _token = value.as_str();
                    barrier.wait().await;
                    println!("attempt get captcha token");
                    let _code = self.captcha_client.get_token().await?;
                    let doxid = text.split("doxid = ")
                        .nth(1)
                        .and_then(|s| s.split(";").next())?;
                    let payload_json = json!({
                        "doxId": doxid,
                        "name": "Anonymous",
                        "comment": parameters_comment.text,
                        "_token": _token,
                        "hcaptcha_token": _code,
                    });
                    println!("attempt start paste");
                    if let Ok((status_code, _, response_html)) = self.post(&"https://doxbin.org/comment", &payload_json).await {
                        println!("status code create paste: {}\t\tProxy: {}", status_code, self.current_proxy);
                        if response_html.contains("\"status\":\"done\"") && status_code == 200 {
                            return Some(())
                        }
                    }
                }
            }
        }
        None
    }

    pub async fn paste(&mut self, mode_paste: ModeComment, parameters_comment: ParameterComment, barrier: Arc<Barrier>) -> Option<()> {
        match &parameters_comment.parameter_account {
            ParameterCommentAccount::CreateNew => {
                if let Some((username, password, session_str)) = self.create_account().await {
                    println!("Log print\t{}:{}|{}", username, password, session_str);
                }
            }
            ParameterCommentAccount::UseExist {exist_type} => {
                match exist_type {
                    ParameterCommentAccountUseExist::ExistAnon  => {
                        self.check_xsrf_token()?;
                    }
                    ParameterCommentAccountUseExist::ExistReg => {
                        self.check_session_token()?;
                    }
                }
            }
            ParameterCommentAccount::Anon => {
                self.change_profile(ModeChange::Cookie).await?
            }
        }
        barrier.wait().await;
        println!("start function comment");
        match mode_paste {
            ModeComment::Paste => {
                return self.comment_paste(parameters_comment, barrier).await;
            },
            ModeComment::Profile => {},
            ModeComment::PasteAndProfile => {},
        }
        None
    }

    fn parse_html(&self, html: &str) -> Vec<LinkData> {
        let mut link_data = Vec::new();
        let document = Html::parse_document(html);
        let tbody_selector = Selector::parse("tbody").unwrap();
        let tr_selector = Selector::parse("tr.doxentry").unwrap();
        let a_selector = Selector::parse("a").unwrap();
        let user_selector = Selector::parse("a.dox-username").unwrap();

        let tbodies = document.select(&tbody_selector).collect::<Vec<_>>();
        if tbodies.len() < 2 {
            return link_data;
        }

        for element in tbodies[1].select(&tr_selector) {
            let link_elem = element.select(&a_selector).next().unwrap();
            let link = link_elem.value().attr("href").unwrap_or_default().to_string();
            let user_elem = element.select(&user_selector).next();
            let user = user_elem.map_or(String::from("Unknown"), |e| e.inner_html().trim().to_string());
            let id = element.value().attr("id").unwrap_or_default().to_string();

            link_data.push(LinkData { id, user, link });
        }

        link_data
    }
    pub async fn subscribe_on_pastes(&mut self, mode: ModeSubscribeOnPastes, sender: mpsc::Sender<ResponseChannel>, link_manager: Arc<TokioMutex<LinkManager>>) {
        let mut htmll = String::new();
        for iter in 1..2 {
            let mut count_add = 0;
            if let Ok((status_code, html)) = self.get(&format!("https://doxbin.org/?page={}", iter)).await {
                htmll = html.clone();
                let link_data = self.parse_html(&html);
                for data in link_data {
                    if count_add >= 10 {
                        break
                    }
                    let mut manager = link_manager.lock().await;
                    if manager.add_link(data.link.clone()) {
                        count_add += 1;
                        if let ModeSubscribeOnPastes::Comment { text, mode_comment, anon } = mode.clone() {
                            if sender.send(ResponseChannel::Sending{data: ResponseChannelInfo {link: data.link.clone(), username: data.user.clone() }}).await.is_err() {
                                println!("error send message in channel");
                            }
                        }
                    }
                }
            }
            println!("Add {} users. Sleeping....", count_add);
            if count_add == 0 {
                if htmll.contains("<title>Doxbin | Checking Browser</title>") {
                    println!("bad browser");
                }

            }
            sleep(Duration::from_secs(10)).await;
        }
        drop(sender)
    }

    pub async fn ttt(&mut self,sender: mpsc::Sender<ResponseChannel>) {
        if sender.send(ResponseChannel::Sending { data: ResponseChannelInfo { link: "asd".to_string(), username: "asd".to_string() } }).await.is_err() {
            println!("error send message in channel");
        }
    }


}

#[derive(Debug)]
struct LinkData {
    id: String,
    user: String,
    link: String,
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