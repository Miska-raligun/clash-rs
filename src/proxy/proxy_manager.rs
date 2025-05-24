use std::collections::HashMap;
use std::sync::Arc;

use crate::proxy::fake::FakeProxy;
use crate::proxy::outbound::{AnyStream, OutboundHandler};
use crate::config::{Config, Proxy};
use crate::proxy::direct::DirectProxy;
use crate::proxy::vmess::VmessProxy;
use crate::proxy::trojan::TrojanProxy;
use uuid::Uuid;

pub struct ProxyManager {
    handlers: HashMap<String, Arc<dyn OutboundHandler>>,
}

impl ProxyManager {
    pub fn new(config: &Config) -> Self {
        let mut handlers: HashMap<String, Arc<dyn OutboundHandler>> = HashMap::new();

        for proxy in &config.proxies {
            match proxy {
                Proxy::Trojan { name, server, port, password, sni } => {
                        let handler = TrojanProxy::new(name.clone(), server.clone(), *port, password.clone(), sni.clone());
                        handlers.insert(name.clone(), Arc::new(handler));
                    }

                Proxy::VMess {
                    name,
                    server,
                    port,
                    uuid,
                    alter_id,
                    network,
                    ws_path,
                    ws_headers,
                    ..
                } => {
                    let uuid = Uuid::parse_str(uuid).unwrap();
                    let proxy = VmessProxy {
                        name: name.clone(),
                        server: server.clone(),
                        port: *port,
                        uuid,
                        alter_id: alter_id.unwrap_or(0),
                        network: network.clone(),
                        ws_path: ws_path.clone(),
                        ws_host: ws_headers.as_ref().and_then(|h| h.get("Host").cloned()),
                        
                    };
                    handlers.insert(name.clone(), Arc::new(proxy));
                }

                Proxy::Unknown => {}
            }
        }

        if !handlers.contains_key("DIRECT") {
            handlers.insert("DIRECT".into(), Arc::new(DirectProxy));
        }

        Self { handlers }
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn OutboundHandler>> {
        self.handlers.get(name).cloned()
    }

    pub fn first(&self) -> Option<Arc<dyn OutboundHandler>> {
        self.handlers.values().next().cloned()
    }
}
