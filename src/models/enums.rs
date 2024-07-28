use std::fs;
use std::io::Read;
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