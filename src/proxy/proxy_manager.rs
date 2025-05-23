use std::collections::HashMap;
use std::sync::Arc;

use crate::proxy::fake::FakeProxy;
use crate::proxy::outbound::{AnyStream, OutboundHandler};
use crate::config::{Config, Proxy};
use crate::proxy::direct::DirectProxy;
use crate::proxy::vmess::VmessProxy;
use uuid::Uuid;

pub struct ProxyManager {
    handlers: HashMap<String, Arc<dyn OutboundHandler>>,
}

impl ProxyManager {
    pub fn new(config: &Config) -> Self {
        let mut handlers: HashMap<String, Arc<dyn OutboundHandler>> = HashMap::new();

        for proxy in &config.proxies {
            match proxy {
                Proxy::Trojan { name, .. } => {
                    handlers.insert(name.clone(), Arc::new(FakeProxy));
                }

                Proxy::VMess { name, server, port, uuid } => {
                    let uuid = Uuid::parse_str(uuid).expect("Invalid VMess UUID");
                    let vmess = VmessProxy::new(name.clone(), server.clone(), *port, uuid);
                    handlers.insert(name.clone(), Arc::new(vmess));
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
