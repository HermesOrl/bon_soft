use std::collections::HashSet;
use std::{fs, io};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug)]
pub struct CommonResponse {
    pub(crate) errorId: i32,
    pub(crate) status: String,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum ApiCaptchaResponseGetCode {
    Ready {
        #[serde(flatten)]
        common: CommonResponse,
        solution: Value,
        cost: String,
        ip: String,
        createTime: u64,
        endTime: u64,
        solveCount: i32,
    },
    Processing {
        #[serde(flatten)]
        common: CommonResponse,
    },
    #[serde(rename_all = "camelCase")]
    Error {
        #[serde(flatten)]
        common: CommonResponse,
        errorCode: String,
        errorDescription: String,
    },
}


#[derive(Serialize, Deserialize)]
pub struct Payload {
    pub(crate) clientKey: String,
    pub(crate) task: Task,
}

#[derive(Serialize, Deserialize)]
pub struct Task {
    r#type: String,
    pub websiteURL: String,
    pub websiteKey: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiCaptchaResponse {
    pub errorId: i32,
    pub taskId: u64,
}

pub fn read_json_from_file_captcha(file_path: &str) -> Result<Payload, Box<dyn std::error::Error>> {
    let mut file = fs::File::open(file_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let payload: Payload = serde_json::from_str(&content)?;
    Ok(payload)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PayloadCaptchaSend {
    pub clientKey: String,
    pub taskId: u64,
}

#[derive(Debug)]
pub enum DoxBinAccountSession {
    WithSession { session_str: String },
    WithoutSession,
}

pub struct DoxBinAccount {
    pub login: String,
    pub password: String,
    pub session: DoxBinAccountSession,
}
pub enum DoxBinAccountGetXsrf {
    NewAccount,
    ExistAccount { session: String }
}

#[derive(Debug)]
pub struct  ResponseParsing {
    pub link: String,
    pub user: String,
    pub id: String,

}

pub struct LinkManager {
    links: HashSet<String>,
}

impl LinkManager {
    pub fn new() -> Self {
        Self {
            links: HashSet::new(),
        }
    }
    pub fn read_from_file(&mut self, file_path: &str) -> io::Result<()> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let parts: Vec<&str> = line.split("_;_").collect();
            // println!("{:?}", parts);
            if parts.len() == 3 {
                self.links.insert(parts[2].to_string());
            }
        }
        Ok(())
    }
    pub fn add_link(&mut self, link: String) -> bool {
        // for linkkk in self.links.clone() {
        //     println!("{}", linkkk)
        // }
        if self.links.contains(&link) {
            false
        } else {
            self.links.insert(link.to_string());
            true
        }
    }
}

#[derive(Clone)]
pub enum ModeComment {
    Paste,
    Profile,
    PasteAndProfile,
}

pub enum ParameterCommentAccountUseExist {
    ExistReg,
    ExistAnon,
}

pub enum ParameterCommentAccount {
    CreateNew,
    UseExist {exist_type: ParameterCommentAccountUseExist },
    Anon,
}

pub struct ParameterComment {
    pub username: String,
    pub link: String,
    pub parameter_account: ParameterCommentAccount,
    pub text: String
}

#[derive(Clone)]
pub enum ModeSubscribeOnPastes{
    Ignore,
    Comment { text: String, mode_comment: ModeComment, anon: bool }
}


pub enum ModeChange {
    Cookie,
    Proxy,
    All
}
#[derive(Debug)]
pub struct ResponseChannel {
    pub link: String,
    pub username: String,
}