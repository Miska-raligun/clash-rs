use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(rename = "socks-port")]
    pub socks_port: Option<u16>,

    pub port: Option<u16>,
    #[serde(rename = "redir-port")]
    pub redir_port: Option<u16>,
    #[serde(rename = "tproxy-port")]
    pub tproxy_port: Option<u16>,

    #[serde(rename = "allow-lan")]
    pub allow_lan: Option<bool>,
    pub mode: Option<String>,
    #[serde(rename = "log-level")]
    pub log_level: Option<String>,
    #[serde(rename = "external-controller")]
    pub external_controller: Option<String>,

    pub proxies: Vec<Proxy>,
    #[serde(rename = "proxy-groups")]
    pub proxy_groups: Vec<ProxyGroup>,
    pub rules: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Proxy {
    #[serde(rename = "trojan")]
    Trojan {
        name: String,
        server: String,
        port: u16,
        password: String,
        #[serde(default)]
        sni: Option<String>,
    },
    #[serde(rename = "vmess")]
    VMess {
        name: String,
        server: String,
        port: u16,
        uuid: String,

        #[serde(rename = "alterId")]     
        #[serde(default)]
        alter_id: Option<u32>,

        cipher: Option<String>,
        udp: Option<bool>,
        network: Option<String>,
        #[serde(rename = "ws-path")]
        ws_path: Option<String>,
        #[serde(rename = "ws-headers")]
        ws_headers: Option<HashMap<String, String>>,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct ProxyGroup {
    pub name: String,
    #[serde(rename = "type")]
    pub group_type: String,
    pub proxies: Vec<String>,
}

use std::fs;

impl Config {
    pub fn load(path: &str) -> Self {
        let content = fs::read_to_string(path).expect("Failed to read config file");
        serde_yaml::from_str(&content).expect("Failed to parse config file")
    }
}
