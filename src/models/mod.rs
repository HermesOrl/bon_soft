pub mod enums;
pub mod response;
pub mod request;

pub mod config;

pub const XSRF_TOKEN_LINKS: [&str; 4] = [
    "https://doxbin.org/",
    "https://doxbin.org/.well-known/ddos-guard/check?context=free_splash",
    "https://doxbin.org/.well-known/ddos-guard/id/T5q8bswinyHijR3O",
    "https://check.ddos-guard.net/set/id/T5q8bswinyHijR3O"];


pub mod proxy;
