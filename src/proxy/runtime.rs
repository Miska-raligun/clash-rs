use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct ProxyGroup {
    current: Arc<RwLock<String>>,
}

impl ProxyGroup {
    pub fn new(default: &str) -> Self {
        Self {
            current: Arc::new(RwLock::new(default.to_string())),
        }
    }

    pub fn get(&self) -> String {
        self.current.read().unwrap().clone()
    }

    pub fn set(&self, name: &str) {
        *self.current.write().unwrap() = name.to_string();
    }
}

#[derive(Clone)]
pub struct ProxyRuntime {
    groups: Arc<RwLock<HashMap<String, ProxyGroup>>>,
}

impl ProxyRuntime {
    pub fn new() -> Self {
        Self {
            groups: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register_group(&self, group_name: &str, default: &str) {
        self.groups
            .write()
            .unwrap()
            .insert(group_name.to_string(), ProxyGroup::new(default));
    }

    pub fn get_group(&self, name: &str) -> Option<ProxyGroup> {
        self.groups.read().unwrap().get(name).cloned()
    }
}

