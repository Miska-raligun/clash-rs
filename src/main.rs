mod config;
mod proxy;

use std::sync::Arc;
use config::Config;
use proxy::proxy_manager::ProxyManager;
use proxy::runtime::ProxyRuntime;
use proxy::socks5::start_socks5_server;
use proxy::http::start_http_server;

#[tokio::main]
async fn main() {
    let config = Config::load("config.yaml");
    let manager = Arc::new(ProxyManager::new(&config));
    let runtime = Arc::new(ProxyRuntime::new());

    // 注册 proxy-group[1] 默认项
    let (group_name, proxy_candidates) = if let Some(group) = config.proxy_groups.get(1) {
        let default_proxy = group
            .proxies
            .get(0)
            .expect("No proxies in the first proxy-group");
        runtime.register_group(&group.name, default_proxy);
        println!(
            "[Init] Registered proxy group: {} -> default: {}",
            group.name, default_proxy
        );

        // ✅ 在这里构造 tuple 并返回
        (group.name.clone(), group.proxies.clone())
    } else {
        panic!("No proxy-group defined in config.");
    };

    // ✅ 然后传入 HTTP 控制器
    tokio::spawn(start_http_server(
        runtime.clone(),
        group_name.clone(),
        proxy_candidates.clone(),
    ));

    let port = config.socks_port.unwrap_or(7891);
    start_socks5_server(
        &format!("0.0.0.0:{}", port),
        manager.clone(),
        runtime.clone(),
    )
    .await
    .unwrap();
}




