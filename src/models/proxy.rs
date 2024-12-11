use std::fs::File;
use std::io;
use std::io::BufRead;
use std::path::Path;

#[derive(Debug, Clone)]
enum ProxyType {
    Http,
    Https,
    Socks5,
    Unknown(String),
}

#[derive(Debug, Clone)]
enum ProxyFormat {
    IpFirst,
    AuthFirst,
    Unknown,
}

#[derive(Debug)]
#[derive(Clone)]
pub struct SProxy {
    pub proxy_url: String,
    proxy_type: ProxyType,
    proxy_format: ProxyFormat,
    health: usize,
    status: bool,
    checked: bool
}

pub struct SProxies {
    proxies: Vec<SProxy>,
}

impl SProxies {
    pub fn new() -> SProxies {
        let mut s_proxies = SProxies {
            proxies: Vec::new(),
        };
        // Заполняем прокси из файла
        s_proxies.add_from_file("./proxies.txt");
        s_proxies
    }
    pub fn add_from_file(&mut self, file_path: &str) -> io::Result<()> {
        let path = Path::new(file_path);
        let file = File::open(&path)?;
        let reader = io::BufReader::new(file);

        for line in reader.lines() {
            if let Ok(GLine) = line  {
                match GLine.trim() {
                    "" => continue,
                    _ => {
                        let result: (ProxyType, ProxyFormat) = self.check_proxy(GLine.clone());
                        // println!("adding proxy for file: {}", GLine.clone());
                        self.add_proxy(GLine, result.0, result.1);
                    }
                }
            }
        }
        Ok(())
    }
    fn check_proxy(&self, proxy_line: String) -> (ProxyType, ProxyFormat) {
        use regex::Regex;

        let ip_first_re = Regex::new(r"^(http|https|socks5)://(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}:\d+@[^:]+:[^@]+)").unwrap();
        let auth_first_re = Regex::new(r"^(http|https|socks5)://([^:]+:[^@]+@\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}:\d+)").unwrap();


        let mut proxy_line_split: Vec<&str> = proxy_line.split("://").collect();
        let proxy_type = match proxy_line_split.get(0) {
            Some(&"http") => ProxyType::Http,
            Some(&"https") => ProxyType::Https,
            Some(&"socks5") => ProxyType::Socks5,
            _ => ProxyType::Socks5,
        };

        let proxy_format = if ip_first_re.is_match(&*proxy_line) {
            ProxyFormat::IpFirst
        } else if auth_first_re.is_match(&*proxy_line) {
            ProxyFormat::AuthFirst
        } else {
            ProxyFormat::Unknown
        };

        (proxy_type, proxy_format)
    }

    fn add_proxy(&mut self, proxy_url: String, proxy_type: ProxyType, proxy_format: ProxyFormat) {
        self.proxies.push(SProxy {proxy_url, proxy_type, proxy_format, health: 3 , status: true, checked: false }) // Valid: (checked, valid_status)
    }

    fn check_proxies(&mut self) -> io::Result<()> {
        for proxy in self.proxies.iter_mut() {
            proxy.checked = true;
        }
        Ok(())
    }

    pub fn get_proxies(&self) -> Vec<SProxy> {
        self.proxies.clone()
    }
    pub fn get_next_proxy(&mut self) -> io::Result<(SProxy)> {
        for proxy in self.proxies.iter_mut() {
            let status_health: bool = if proxy.health >= 1 {true}else{false};
            if proxy.status && status_health {
                proxy.health -= 1;
                if proxy.health == 0 {
                    proxy.status = false;
                }

                return Ok(proxy.clone());
            }
        }
        Err(io::Error::new(io::ErrorKind::Other, "Proxy not existed"))
    }

}