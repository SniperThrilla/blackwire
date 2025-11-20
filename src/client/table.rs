use crate::ByteSender;
use crate::client::types::ClientInfo;
use crate::net::mac::{Mac, generate_mac};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

pub struct ClientTable {
    map: Mutex<HashMap<Mac, Arc<ClientInfo>>>,
}

pub type SharedClientTable = Arc<ClientTable>;

impl ClientTable {
    pub fn new() -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
        }
    }

    pub fn insert(&self, mac: Mac, info: Arc<ClientInfo>) {
        let mut lock = self.map.lock().unwrap();
        lock.insert(mac, info);
    }

    pub fn remove(&self, mac: Mac) -> Option<Arc<ClientInfo>> {
        let mut lock = self.map.lock().unwrap();
        lock.remove(&mac)
    }

    pub fn get(&self, mac: Mac) -> Option<Arc<ClientInfo>> {
        let lock = self.map.lock().unwrap();
        lock.get(&mac).cloned()
    }

    pub fn generate_unique_mac(&self) -> Mac {
        let lock = self.map.lock().unwrap();
        generate_mac(&mut lock.keys())
    }

    pub fn all_macs(&self) -> Vec<Mac> {
        let lock = self.map.lock().unwrap();
        lock.keys().cloned().collect()
    }

    pub fn add_new_client(&self, addr: SocketAddr, bs: ByteSender) -> Arc<ClientInfo> {
        let mac = self.generate_unique_mac();
        let info = ClientInfo {
            mac: mac,
            sender: bs,
            addr: addr,
        };

        let safe = Arc::new(info);
        self.insert(mac, Arc::clone(&safe));
        safe
    }
}
