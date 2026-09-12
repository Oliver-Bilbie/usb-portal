use crate::notification::Notification;
use log::*;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    mem,
    net::Ipv4Addr,
    sync::Mutex,
    time,
};
use usb_portal_lib::types::{ConnectedDevice, MissingDevice, Server};

pub struct State {
    notifications: Mutex<Vec<Notification>>,
    servers: Mutex<HashMap<Ipv4Addr, Server>>,
    failed_pings: Mutex<HashMap<Server, u8>>,
    discovery_threads: Mutex<VecDeque<tokio::task::JoinHandle<()>>>,
    connected_devices: Mutex<Vec<ConnectedDevice>>,
    missing_devices: Mutex<HashSet<MissingDevice>>,
}

impl State {
    pub fn new() -> State {
        State {
            notifications: Mutex::new(Vec::new()),
            servers: Mutex::new(HashMap::new()),
            failed_pings: Mutex::new(HashMap::new()),
            discovery_threads: Mutex::new(VecDeque::new()),
            connected_devices: Mutex::new(Vec::new()),
            missing_devices: Mutex::new(HashSet::new()),
        }
    }

    pub fn get_notifications(&self) -> Vec<Notification> {
        let mut items = match self.notifications.lock() {
            Ok(v) => v.clone(),
            Err(e) => {
                error!("Unable to fetch notifications\n{}", e);
                vec![]
            }
        };
        items.sort();
        items
    }

    pub fn push_notification(&self, level: log::Level, message: String) {
        match self.notifications.lock() {
            Ok(mut v) => {
                v.push(Notification {
                    id: uuid::Uuid::new_v4(),
                    level,
                    timestamp: time::Instant::now(),
                    message,
                });
            }
            Err(e) => {
                error!("Unable to fetch notifications\n{}", e);
            }
        }
    }

    pub fn dismiss_notification(&self, id: &str) {
        match self.notifications.lock() {
            Ok(mut v) => {
                v.retain(|n| n.id.to_string() != id);
            }
            Err(e) => {
                error!("Unable to fetch notifications\n{}", e);
            }
        }
    }

    pub fn get_servers_list(&self) -> Vec<Server> {
        match self.servers.lock() {
            Ok(v) => v.values().cloned().collect(),
            Err(e) => {
                error!("Unable to fetch server list\n{}", e);
                Vec::new()
            }
        }
    }

    pub fn get_server(&self, ip: &Ipv4Addr) -> Option<Server> {
        match self.servers.lock() {
            Ok(v) => v.get(ip).cloned(),
            Err(e) => {
                error!("Unable to fetch server list\n{}", e);
                None
            }
        }
    }

    pub fn add_server(&self, server: Server) {
        match self.servers.lock() {
            Ok(mut v) => {
                v.insert(server.ip, server);
            }
            Err(e) => {
                error!("Unable to fetch server list\n{}", e);
            }
        }
    }

    pub fn drop_server(&self, ip: &Ipv4Addr) {
        match self.servers.lock() {
            Ok(mut v) => {
                v.remove(ip);
            }
            Err(e) => {
                error!("Unable to fetch server list\n{}", e);
            }
        }
    }

    pub fn push_discovery(&self, discovery: tokio::task::JoinHandle<()>) {
        match self.discovery_threads.lock() {
            Ok(mut queue) => {
                queue.push_back(discovery);
            }
            Err(e) => {
                error!("Unable to fetch discovery jobs\n{}", e);
            }
        }
    }

    pub fn get_missing_servers(&self) -> Vec<Server> {
        match self.failed_pings.lock() {
            Ok(servers) => servers
                .iter()
                .filter_map(|(s, ct)| {
                    if *ct == 0 {
                        return None;
                    }
                    Some(s.clone())
                })
                .collect(),
            Err(e) => {
                error!("Unable to fetch failed pings\n{}", e);
                Vec::new()
            }
        }
    }

    pub fn increment_failed_pings(&self, server: Server) -> u8 {
        match self.failed_pings.lock() {
            Ok(mut servers) => {
                let mut count = 1;
                servers
                    .entry(server)
                    .and_modify(|n| {
                        count = n.saturating_add(1);
                        *n = count;
                    })
                    .or_insert(1);
                count
            }
            Err(e) => {
                error!("Unable to fetch failed pings\n{}", e);
                0
            }
        }
    }

    pub fn reset_failed_pings(&self, server: &Server) {
        match self.failed_pings.lock() {
            Ok(mut servers) => {
                servers.remove(server);
            }
            Err(e) => {
                error!("Unable to fetch failed pings\n{}", e);
            }
        }
    }

    pub async fn wait_for_discovery(&self) {
        loop {
            let mut queue = mem::replace(
                &mut *self.discovery_threads.lock().unwrap(),
                VecDeque::new(),
            );
            if queue.is_empty() {
                return;
            }
            while let Some(d) = queue.pop_front() {
                let _ = d.await;
            }
        }
    }

    pub fn get_connected_devices(&self) -> Vec<ConnectedDevice> {
        match self.connected_devices.lock() {
            Ok(v) => v.clone(),
            Err(e) => {
                error!("Unable to fetch previously connected device list\n{}", e);
                Vec::new()
            }
        }
    }

    pub fn add_connected_device(&self, device: ConnectedDevice) {
        match self.connected_devices.lock() {
            Ok(mut v) => {
                v.push(device.clone());
            }
            Err(e) => {
                error!("Unable to fetch previously connected device list\n{}", e);
            }
        }
        self.drop_missing_device(&MissingDevice {
            device: device.device,
            server_ip: device.server_ip,
        });
    }

    pub fn drop_connected_device(&self, device: &ConnectedDevice) {
        match self.connected_devices.lock() {
            Ok(mut v) => {
                v.retain(|d| d != device);
            }
            Err(e) => {
                error!("Unable to fetch previously connected device list\n{}", e);
            }
        }
    }

    pub fn get_missing_devices(&self) -> HashSet<MissingDevice> {
        match self.missing_devices.lock() {
            Ok(v) => v.clone(),
            Err(e) => {
                error!("Unable to fetch missing device list\n{}", e);
                HashSet::new()
            }
        }
    }

    pub fn add_missing_device(&self, device: &ConnectedDevice) {
        match self.missing_devices.lock() {
            Ok(mut v) => {
                v.insert(MissingDevice {
                    device: device.device.clone(),
                    server_ip: device.server_ip,
                });
            }
            Err(e) => {
                error!("Unable to fetch missing device list\n{}", e);
            }
        }
    }

    pub fn drop_missing_device(&self, device: &MissingDevice) {
        match self.missing_devices.lock() {
            Ok(mut v) => {
                v.remove(device);
            }
            Err(e) => {
                error!("Unable to fetch missing device list\n{}", e);
            }
        }
    }
}
