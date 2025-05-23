use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct ProxyGroup {
    current: Arc<RwLock<String>>,
}

impl ProxyGroup {
    pub fn new(default: String) -> Self {
        Self {
            current: Arc::new(RwLock::new(default)),
        }
    }

    pub fn get(&self) -> String {
        self.current.read().unwrap().clone()
    }

    pub fn set(&self, name: String) {
        *self.current.write().unwrap() = name;
    }
}
